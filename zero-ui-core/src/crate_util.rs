//! Crate visible macros and utilities.

use rand::Rng;
use rustc_hash::FxHasher;
use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasher, BuildHasherDefault, Hasher},
    num::NonZeroU64,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        Arc, Weak,
    },
};

/// Declare a new unique id type.
macro_rules! unique_id {
    ($(#[$docs:meta])* $vis:vis struct $Type:ident;) => {

        $(#[$docs])*
        ///
        /// # As Hash
        ///
        /// The generated internal number has good statistical distribution and can be used as its own hash,
        /// although it is not cryptographically safe, as it is simply a sequential counter scrambled using `splitmix64`.
        ///
        /// # Non-Zero
        ///
        /// The internal number is non-zero, an `Option` value of this type uses the same 64-bits because
        /// the compiler represents `None` using zero.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis struct $Type(std::num::NonZeroU64);

        impl $Type {
            /// Generates a new unique ID.
            ///
            /// # Panics
            ///
            /// Panics if called more then `u64::MAX` times.
            pub fn new_unique() -> Self {
                use std::sync::atomic::AtomicU64;
                static NEXT: AtomicU64 = AtomicU64::new(1);
                Self($crate::crate_util::next_id(&NEXT))
            }

            /// Retrieve the underlying `u64` value.
            #[allow(dead_code)]
            #[inline]
            pub fn get(self) -> u64 {
                self.0.get()
            }

            /// Un-scramble the underlying value to get the original sequential count number.
            ///
            /// If two ids, `id0` and `id1` where generated by the same thread then `id0.sequential() < id1.sequential()`.
            #[allow(dead_code)]
            pub fn sequential(self) -> u64 {
                $crate::crate_util::un_splitmix64(self.0.get())
            }

            /// Creates an id from a raw value.
            ///
            /// # Safety
            ///
            /// This is only safe if called with a value provided by [`get`](Self::get), the value must not be zero.
            #[allow(dead_code)]
            pub unsafe fn from_raw(raw: u64) -> $Type {
                $Type(std::num::NonZeroU64::new_unchecked(raw))
            }
        }
    };
}
#[doc(hidden)]
pub fn next_id(next: &'static AtomicU64) -> NonZeroU64 {
    loop {
        // the sequential next id is already in the variable.
        let id = next.fetch_add(1, Ordering::Relaxed);

        if id == 0 {
            log::error!("id generator reached `u64::MAX`, will start reusing");
        } else {
            // remove the sequential clustering.
            let id = splitmix64(id);
            if let Some(id) = NonZeroU64::new(id) {
                return id;
            }
        }
    }
}
#[inline]
fn splitmix64(n: u64) -> u64 {
    use std::num::Wrapping as W;

    let mut z = W(n);
    z = (z ^ (z >> 30)) * W(0xBF58476D1CE4E5B9u64);
    z = (z ^ (z >> 27)) * W(0x94D049BB133111EBu64);
    z = z ^ (z >> 31);
    z.0
}
#[doc(hidden)]
pub fn un_splitmix64(z: u64) -> u64 {
    use std::num::Wrapping as W;

    let mut x = W(z);
    x = (x ^ (x >> 31) ^ (x >> 62)) * W(0x319642b2d24d8ec3u64);
    x = (x ^ (x >> 27) ^ (x >> 54)) * W(0x96de1b173f119089u64);
    x = x ^ (x >> 30) ^ (x >> 60);
    x.0
}

/// Ideal map type for key types generated using [`unique_id!`].
pub type IdMap<K, V> = HashMap<K, V, BuildHasherDefault<IdHasher>>;

#[derive(Default)]
pub struct IdHasher(u64);
impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("`only `write_u64` is supported");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

/// Generates a type that can only have a single instance per thread.
macro_rules! thread_singleton {
    ($Singleton:ident) => {
        struct $Singleton {
            _not_send: std::marker::PhantomData<std::rc::Rc<()>>,
        }
        impl $Singleton {
            std::thread_local! {
                static IN_USE: std::cell::Cell<bool> = std::cell::Cell::new(false);
            }

            fn set(in_use: bool) {
                Self::IN_USE.with(|f| f.set(in_use));
            }

            /// If an instance of this type already exists in this thread.
            pub fn in_use() -> bool {
                Self::IN_USE.with(|f| f.get())
            }

            /// Panics if [`Self::in_use`], otherwise creates the single instance of `Self` for the thread.
            pub fn assert_new(type_name: &str) -> Self {
                if Self::in_use() {
                    panic!("only a single instance of `{}` can exist per thread at a time", type_name)
                }
                Self::set(true);

                Self {
                    _not_send: std::marker::PhantomData,
                }
            }
        }
        impl Drop for $Singleton {
            fn drop(&mut self) {
                Self::set(false);
            }
        }
    };
}

/// Runs a cleanup action once on drop.
pub(crate) struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    pub fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}

/// Represents a resource handle.
///
/// The resource stays in memory as long as a handle clone is alive. After the handle
/// is dropped the resource will be removed after an indeterminate time at the discretion
/// of the resource manager.
///
/// You can *forget* a handle by calling [`permanent`](Self::permanent), this releases the handle memory
/// but the resource stays alive for the duration of the app, unlike calling [`std::mem::forget`] no memory is leaked.
///
/// Any handle can also [`force_drop`](Self::force_drop), meaning that even if there are various handles active the
/// resource will be dropped regardless.
///
/// The parameter type `D` is any [`Sync`] data type that will be shared using the handle.
#[must_use = "the resource id dropped if the handle is dropped"]
pub(crate) struct Handle<D: Send + Sync>(Arc<HandleState<D>>);
struct HandleState<D> {
    state: AtomicU8,
    data: D,
}
impl<D: Send + Sync> Handle<D> {
    /// Create a handle with owner pair.
    pub fn new(data: D) -> (HandleOwner<D>, Handle<D>) {
        let handle = Handle(Arc::new(HandleState {
            state: AtomicU8::new(NONE),
            data,
        }));
        (HandleOwner(handle.clone()), handle)
    }

    /// Create a handle to nothing, the handle always in the *dropped* state.
    #[inline]
    pub fn dummy(data: D) -> Self {
        Handle(Arc::new(HandleState {
            state: AtomicU8::new(FORCE_DROP),
            data,
        }))
    }

    /// Reference the attached data.
    #[inline]
    pub fn data(&self) -> &D {
        &self.0.data
    }

    /// Mark the handle as permanent and drops this clone of it. This causes the resource to stay in memory
    /// until the app exits, no need to hold a handle somewhere.
    #[inline]
    pub fn permanent(self) {
        self.0.state.fetch_or(PERMANENT, Ordering::Relaxed);
    }

    /// If [`permanent`](Self::permanent) was called in another clone of this handle.
    ///
    /// If `true` the resource will stay in memory for the duration of the app, unless [`force_drop`](Self::force_drop)
    /// is also called.
    #[inline]
    pub fn is_permanent(&self) -> bool {
        self.0.state.load(Ordering::Relaxed) == PERMANENT
    }

    /// Force drops the handle, meaning the resource will be dropped even if there are other handles active.
    #[inline]
    pub fn force_drop(self) {
        self.0.state.store(FORCE_DROP, Ordering::Relaxed);
    }

    /// If the handle is in *dropped* state.
    ///
    /// The handle is considered dropped when all handle and clones are dropped or when [`force_drop`](Handle::force_drop)
    /// was called in any of the clones.
    ///
    /// Note that in this method it can only be because [`force_drop`](Handle::force_drop) was called.
    #[inline]
    pub fn is_dropped(&self) -> bool {
        self.0.state.load(Ordering::Relaxed) == FORCE_DROP
    }

    /// Create a [`WeakHandle`] to this handle.
    #[inline]
    pub fn downgrade(&self) -> WeakHandle<D> {
        WeakHandle(Arc::downgrade(&self.0))
    }
}
impl<D: Send + Sync> Clone for Handle<D> {
    fn clone(&self) -> Self {
        Handle(Arc::clone(&self.0))
    }
}
impl<D: Send + Sync> Drop for Handle<D> {
    fn drop(&mut self) {
        if !self.is_permanent() && Arc::strong_count(&self.0) == 2 {
            // if we are about to drop the last handle and it is not permanent, force-drop
            // this causes potential weak-handles to not reanimate a dropping resource because
            // of the handle that HandleOwner holds.
            self.0.state.store(FORCE_DROP, Ordering::Relaxed);
        }
    }
}

/// A weak reference to a [`Handle`].
pub(crate) struct WeakHandle<D: Send + Sync>(Weak<HandleState<D>>);
impl<D: Send + Sync> WeakHandle<D> {
    /// Get a live handle if it was not dropped or force-dropped.
    pub fn upgrade(&self) -> Option<Handle<D>> {
        if let Some(arc) = self.0.upgrade() {
            let handle = Handle(arc);
            if handle.is_dropped() {
                None
            } else {
                Some(handle)
            }
        } else {
            None
        }
    }
}
impl<D: Send + Sync> Clone for WeakHandle<D> {
    fn clone(&self) -> Self {
        WeakHandle(self.0.clone())
    }
}

/// A [`Handle`] owner.
///
/// Use [`Handle::new`] to create.
///
/// Dropping the [`HandleOwner`] marks all active handles as *force-drop*.
pub(crate) struct HandleOwner<D: Send + Sync>(Handle<D>);
impl<D: Send + Sync> HandleOwner<D> {
    /// If the handle is in *dropped* state.
    ///
    /// The handle is considered dropped when all handle and clones are dropped or when [`force_drop`](Handle::force_drop)
    /// was called in any of the clones.
    pub fn is_dropped(&self) -> bool {
        let state = self.0 .0.state.load(Ordering::Relaxed);
        state == FORCE_DROP || (state != PERMANENT && Arc::strong_count(&self.0 .0) <= 1)
    }

    /// New handle owner in the dropped state.
    pub fn dropped(data: D) -> HandleOwner<D> {
        HandleOwner(Handle(Arc::new(HandleState {
            state: AtomicU8::new(FORCE_DROP),
            data,
        })))
    }

    /// Gets a new handle and resets the state if it was *force-drop*.
    ///
    /// Note that handles are permanently dropped when the last handle is dropped.
    pub fn reanimate(&self) -> Handle<D> {
        self.0 .0.state.store(NONE, Ordering::Relaxed);
        self.0.clone()
    }

    /// Gets an weak handle that may-not be able to upgrade.
    pub fn weak_handle(&self) -> WeakHandle<D> {
        self.0.downgrade()
    }

    /// Reference the attached data.
    pub fn data(&self) -> &D {
        self.0.data()
    }
}
impl<D: Send + Sync> Drop for HandleOwner<D> {
    fn drop(&mut self) {
        self.0 .0.state.store(FORCE_DROP, Ordering::Relaxed);
    }
}

const NONE: u8 = 0;
const PERMANENT: u8 = 0b01;
const FORCE_DROP: u8 = 0b11;

/// A map of TypeId -> Box<dyn UnsafeAny>.
// Uses IdMap because TypeIds are already hashes by the compiler.
pub type AnyMap = IdMap<std::any::TypeId, Box<dyn unsafe_any::UnsafeAny>>;

/// Converts a [`std::panic::catch_unwind`] payload to a str.
pub fn panic_str<'s>(payload: &'s Box<dyn std::any::Any + Send + 'static>) -> &'s str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s
    } else {
        "<unknown-panic-message-type>"
    }
}

/// Type alias for the *error* of [`PanicResult`].
pub type PanicPayload = Box<dyn std::any::Any + Send + 'static>;

/// The result that is returned by [`std::panic::catch_unwind`].
pub type PanicResult<R> = std::thread::Result<R>;

// this is the FxHashMap with a patch that fixes slow deserialization.
// see https://github.com/rust-lang/rustc-hash/issues/15 for details.
#[derive(Clone)]
pub struct BuildFxHasher(usize);
impl BuildHasher for BuildFxHasher {
    type Hasher = rustc_hash::FxHasher;

    fn build_hasher(&self) -> Self::Hasher {
        let mut hasher = FxHasher::default();
        hasher.write_usize(self.0);
        hasher
    }
}
impl Default for BuildFxHasher {
    fn default() -> Self {
        Self(rand::thread_rng().gen())
    }
}

/// Patched [`rustc_hash::FxHashMap`].
pub type FxHashMap<K, V> = HashMap<K, V, BuildFxHasher>;
/// Patched [`rustc_hash::FxHashSet`].
pub type FxHashSet<V> = HashSet<V, BuildFxHasher>;

#[cfg(test)]
pub mod tests {
    use crate::WidgetId;

    #[test]
    pub fn sequential_id() {
        let id0 = WidgetId::new_unique();
        let id1 = WidgetId::new_unique();

        assert!(id0.sequential() < id1.sequential());
    }
}

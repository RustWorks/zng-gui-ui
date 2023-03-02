//! Crate visible macros and utilities.

use crate::{text::Text, units::Deadline};
use rand::Rng;
use rustc_hash::FxHasher;
use std::{
    collections::{hash_map, HashMap},
    fmt,
    hash::{BuildHasher, Hasher},
    num::{NonZeroU32, NonZeroU64},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering},
        Arc, Weak,
    },
    thread,
    time::{Duration, Instant},
};

/// Asserts the `size_of` a type at compile time.
#[allow(unused)]
macro_rules! assert_size_of {
    ($Type:ty, $n:expr) => {
        const _: () = assert!(std::mem::size_of::<$Type>() == $n);
    };
}

/// Asserts the `size_of` an `Option<T>` is the same as the size of `T` a type at compile time.
#[allow(unused)]
macro_rules! assert_non_null {
    ($Type:ty) => {
        const _: () = assert!(std::mem::size_of::<$Type>() == std::mem::size_of::<Option<$Type>>());
    };
}

/// Declare a new unique id type that is backed by a `NonZeroU32`.
macro_rules! unique_id_32 {
    ($(#[$attrs:meta])* $vis:vis struct $Type:ident $(< $T:ident $(:($($bounds:tt)+))? >)?  $(: $ParentId:path)? ;) => {
       $crate::crate_util::unique_id! {
            request {
                $(#[$attrs])*
                ///
                /// # Memory
                ///
                /// The internal number is a [`NonZeroU32`], so it always uses 32-bits of memory, be it a direct value or in an `Option`.
                ///
                /// # As Hash
                ///
                /// The generated internal number has good statistical distribution and can be used as its own hash,
                /// although it is not cryptographically safe, as it is simply a sequential counter scrambled using a modified
                /// `splitmix64`.
                ///
                /// [`NonZeroU32`]: std::num::NonZeroU32
                ///
                /// # Static
                ///
                /// The unique ID cannot be generated at compile time, but you can use the [`new_static`] constructor to
                /// create a lightweight lazy ID generator that will generate the ID on the first get.
                ///
                /// [`new_static`]: Self::new_static
                $vis struct $Type $(< $T $(:($($bounds)+))? >)?  $(: $ParentId)? ;
            }
            non_zero {
                std::num::NonZeroU32
            }
            atomic {
                std::sync::atomic::AtomicU32
            }
            next_id {
                $crate::crate_util::next_id32
            }
            literal {
                u32
            }
            to_hash {
                crate::crate_util::un_hash32
            }
            to_sequential {
                crate::crate_util::un_hash32
            }
       }
    }
}

/// Declare a new unique id type that is backed by a `NonZeroU64`.
macro_rules! unique_id_64 {
    ($(#[$attrs:meta])* $vis:vis struct $Type:ident $(< $T:ident $(:($($bounds:tt)+))? >)?  $(: $ParentId:path)? ;) => {
        $crate::crate_util::unique_id! {
            request {
                $(#[$attrs])*
                ///
                /// # Memory
                ///
                /// The internal number is a [`NonZeroU64`], so it always uses 64-bits of memory, be it a direct value or in an `Option`.
                ///
                /// # As Hash
                ///
                /// The generated internal number has good statistical distribution and can be used as its own hash,
                /// although it is not cryptographically safe, as it is simply a sequential counter scrambled using `splitmix64`.
                ///
                /// [`NonZeroU64`]: std::num::NonZeroU64
                ///
                /// # Static
                ///
                /// The unique ID cannot be generated at compile time, but you can use the [`new_static`] constructor to
                /// create a lightweight lazy ID generator that will generate the ID on the first get.
                ///
                /// [`new_static`]: Self::new_static
                $vis struct $Type $(< $T $(:($($bounds)+))? >)?  $(: $ParentId)? ;
            }
            non_zero {
                std::num::NonZeroU64
            }
            atomic {
                std::sync::atomic::AtomicU64
            }
            next_id {
                $crate::crate_util::next_id64
            }
            literal {
                u64
            }
            to_hash {
                crate::crate_util::splitmix64
            }
            to_sequential {
                crate::crate_util::un_splitmix64
            }
        }
    };
}

macro_rules! unique_id {
    (
        request {
            $(#[$attrs:meta])* $vis:vis struct $Type:ident $(< $T:ident $(:($($bounds:tt)+))? >)?  $(: $ParentId:path)? ;
        }
        non_zero {
            $non_zero:path
        }
        atomic {
            $atomic:path
        }
        next_id {
            $next_id:path
        }
        literal {
            $lit:ident
        }
        to_hash {
            $to_hash:path
        }
        to_sequential {
            $to_sequential:path
        }
    ) => {

        $(#[$attrs])*
        $vis struct $Type $(<$T $(: $($bounds)+)?>)? ($non_zero $(, std::marker::PhantomData<$T>)?);

        impl$(<$T $(: $($bounds)+)?>)? Clone for $Type $(<$T>)? {
            fn clone(&self) -> Self {
                Self(self.0  $(, std::marker::PhantomData::<$T>)?)
            }
        }
        impl$(<$T $(: $($bounds)+)?>)? Copy for $Type $(<$T>)? {
        }
        impl$(<$T $(: $($bounds)+)?>)? PartialEq for $Type $(<$T>)? {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }
        impl$(<$T $(: $($bounds)+)?>)? Eq for $Type $(<$T>)? {
        }
        impl$(<$T $(: $($bounds)+)?>)? std::hash::Hash for $Type $(<$T>)? {
            fn hash<H>(&self, state: &mut H)
            where
                H: std::hash::Hasher
            {
                std::hash::Hash::hash(&self.0, state)
            }
        }

        #[allow(dead_code)]
        impl$(<$T $(: $($bounds)+)?>)? $Type $(<$T>)? {
            $crate::crate_util::unique_id! {
                new_unique {
                    $($ParentId, )? $(<$T>)?
                }
                atomic {
                    $atomic
                }
                next_id {
                    $next_id
                }
            }

            paste::paste! {
                /// New static ID that will be generated on the first get.
                pub const fn new_static() -> [<Static $Type>] $(<$T>)? {
                    [<Static $Type>] $(::<$T>)? ::new_unique()
                }
            }

            /// Retrieve the underlying value.
            pub fn get(self) -> $lit {
                self.0.get()
            }

            /// Un-scramble the underlying value to get the original sequential count number.
            ///
            /// If two ids, `id0` and `id1` where generated by the same thread then `id0.sequential() < id1.sequential()`.
            pub fn sequential(self) -> $lit {
                $to_sequential(self.0.get())
            }

            /// Creates an id from a raw value.
            ///
            /// The value must not be zero, panics if it is, the value must have been provided by [`get`] otherwise
            /// the ID will not be unique.
            ///
            /// [`get`]: Self::get
            pub fn from_raw(raw: $lit) -> Self {
                use $non_zero as __non_zero;

                Self(__non_zero::new(raw).unwrap() $(, std::marker::PhantomData::<$T>)?)
            }

            /// Creates an id from a [`sequential`] number.
            ///
            /// # Safety
            ///
            /// The value must not be zero, panics if it is, the value must have been provided by [`sequential`] otherwise
            /// the ID will not be unique.
            ///
            /// [`sequential`]: Self::sequential
            pub fn from_sequential(num: $lit) -> Self {
                use $non_zero as __non_zero;

                Self(__non_zero::new($to_hash(num)).unwrap() $(, std::marker::PhantomData::<$T>)?)
            }
        }

        paste::paste! {
            #[doc = "Lazy inited [`" $Type "`]."]
            #[allow(dead_code)]
            $vis struct [<Static $Type>] $(<$T $(: $($bounds)+)?>)? ($atomic $(, std::marker::PhantomData<fn() -> $T>)?);

            #[allow(dead_code)]
            impl $(<$T $(: $($bounds)+)?>)? [<Static $Type>] $(<$T>)? {
                #[doc = "New static [`" $Type "`], an unique ID will be generated on the first get."]
                pub const fn new_unique() -> Self {
                    use $atomic as __atomic;

                    Self(__atomic::new(0) $(, std::marker::PhantomData::<fn() -> $T>)?)
                }

                /// Gets or generates the unique ID.
                pub fn get(&self) -> $Type $(<$T>)? {
                    use std::sync::atomic::Ordering;

                    use $non_zero as __non_zero;

                    let id = self.0.load(Ordering::Relaxed);
                    if let Some(id) = __non_zero::new(id) {
                        $Type(id $(, std::marker::PhantomData::<$T>)?)
                    } else {
                        let id = $Type $(::<$T>)? ::new_unique().get();
                        let id = match self.0.compare_exchange(0, id, Ordering::AcqRel, Ordering::Relaxed) {
                            Ok(_) => id,
                            Err(id) => id,
                        };

                        // SAFETY: already replaced zero.
                        $Type(__non_zero::new(id).unwrap() $(, std::marker::PhantomData::<$T>)?)
                    }
                }
            }

            impl $(<$T $(: $($bounds)+)?>)? From<&'static [<Static $Type>]  $(<$T>)?> for $Type  $(<$T>)? {
                fn from(st: &'static [<Static $Type>]  $(<$T>)?) -> $Type  $(<$T>)? {
                    st.get()
                }
            }
        }
    };

    (
        new_unique {
            $ParentId:path,  $(<$T:ident>)?
        }
        atomic {
            $atomic:path
        }
        next_id {
            $next_id:path
        }
    ) => {
        /// Generates a new unique ID.
        pub fn new_unique() -> Self {
            use $ParentId as __parent;
            let id = __parent $(::<$T>)? ::new_unique().get();
            Self::from_raw(id)
        }
    };

    (
        new_unique {
            $(<$T:ident>)?
        }
        atomic {
            $atomic:path
        }
        next_id {
            $next_id:path
        }
    ) => {
        /// Generates a new unique ID.
        pub fn new_unique() -> Self {
            use $atomic as __atomic;
            static NEXT: __atomic = __atomic::new(1);
            Self($next_id(&NEXT) $(, std::marker::PhantomData::<$T>)?)
        }
    };
}
pub(crate) use unique_id;

#[doc(hidden)]
pub fn next_id32(next: &'static AtomicU32) -> NonZeroU32 {
    loop {
        // the sequential next id is already in the variable.
        let id = next.fetch_add(1, Ordering::Relaxed);

        if id == 0 {
            tracing::error!("id generator reached `u32::MAX`, will start reusing");
        } else {
            let id = hash32(id);
            if let Some(id) = NonZeroU32::new(id) {
                return id;
            }
        }
    }
}
#[doc(hidden)]
pub fn next_id64(next: &'static AtomicU64) -> NonZeroU64 {
    loop {
        // the sequential next id is already in the variable.
        let id = next.fetch_add(1, Ordering::Relaxed);

        if id == 0 {
            tracing::error!("id generator reached `u64::MAX`, will start reusing");
        } else {
            // remove the sequential clustering.
            let id = splitmix64(id);
            if let Some(id) = NonZeroU64::new(id) {
                return id;
            }
        }
    }
}

#[doc(hidden)]
pub fn hash32(n: u32) -> u32 {
    use std::num::Wrapping as W;

    let mut z = W(n);
    z = ((z >> 16) ^ z) * W(0x45d9f3b);
    z = ((z >> 16) ^ z) * W(0x45d9f3b);
    z = (z >> 16) ^ z;
    z.0
}
#[doc(hidden)]

pub fn un_hash32(z: u32) -> u32 {
    use std::num::Wrapping as W;

    let mut n = W(z);
    n = ((n >> 16) ^ n) * W(0x119de1f3);
    n = ((n >> 16) ^ n) * W(0x119de1f3);
    n = (n >> 16) ^ n;
    n.0
}

#[doc(hidden)]
pub fn splitmix64(n: u64) -> u64 {
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

    let mut n = W(z);
    n = (n ^ (n >> 31) ^ (n >> 62)) * W(0x319642b2d24d8ec3u64);
    n = (n ^ (n >> 27) ^ (n >> 54)) * W(0x96de1b173f119089u64);
    n = n ^ (n >> 30) ^ (n >> 60);
    n.0
}

/// Ideal map type for key types generated using `unique_id!`.
///
/// Use [`id_map_new`] to instantiate.
pub type IdMap<K, V> = hashbrown::HashMap<K, V, BuildIdHasher>;
/// Ideal set type for key types generated using `unique_id!`.
///
/// Use [`id_set_new`] to instantiate.
#[allow(unused)]
pub type IdSet<K> = hashbrown::HashSet<K, BuildIdHasher>;

pub const fn id_map_new<K, V>() -> IdMap<K, V> {
    hashbrown::HashMap::with_hasher(BuildIdHasher)
}
#[allow(unused)]
pub const fn id_set_new<K>() -> IdSet<K> {
    hashbrown::HashSet::with_hasher(BuildIdHasher)
}

/// Entry in [`IdMap`].
pub type IdEntry<'a, K, V> = hashbrown::hash_map::Entry<'a, K, V, BuildIdHasher>;

pub type IdOccupiedEntry<'a, K, V> = hashbrown::hash_map::OccupiedEntry<'a, K, V, BuildIdHasher>;

pub type IdVacantEntry<'a, K, V> = hashbrown::hash_map::VacantEntry<'a, K, V, BuildIdHasher>;

#[derive(Default, Clone, Debug, Copy)]
pub struct BuildIdHasher;
impl BuildHasher for BuildIdHasher {
    type Hasher = IdHasher;

    fn build_hasher(&self) -> Self::Hasher {
        IdHasher::default()
    }
}

#[derive(Default)]
pub struct IdHasher(u64);
impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("`only `write_u64` is supported");
    }

    fn write_u32(&mut self, id: u32) {
        self.0 = id as u64;
    }

    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    fn finish(&self) -> u64 {
        self.0
    }
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
/// You can *forget* a handle by calling [`perm`](Self::perm), this releases the handle memory
/// but the resource stays alive for the duration of the app, unlike calling [`std::mem::forget`] no memory is leaked.
///
/// Any handle can also [`force_drop`](Self::force_drop), meaning that even if there are various handles active the
/// resource will be dropped regardless.
///
/// The parameter type `D` is any [`Sync`] data type that will be shared using the handle.
#[must_use = "the resource id dropped if the handle is dropped"]
#[repr(transparent)]
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
    ///
    /// Note that `Option<Handle<D>>` takes up the same space as `Handle<D>` and avoids an allocation.
    pub fn dummy(data: D) -> Self {
        assert_non_null!(Handle<()>);
        Handle(Arc::new(HandleState {
            state: AtomicU8::new(FORCE_DROP),
            data,
        }))
    }

    /// Reference the attached data.
    pub fn data(&self) -> &D {
        &self.0.data
    }

    /// Mark the handle as permanent and drops this clone of it. This causes the resource to stay in memory
    /// until the app exits, no need to hold a handle somewhere.
    pub fn perm(self) {
        self.0.state.fetch_or(PERMANENT, Ordering::Relaxed);
    }

    /// If [`perm`](Self::perm) was called in another clone of this handle.
    ///
    /// If `true` the resource will stay in memory for the duration of the app, unless [`force_drop`](Self::force_drop)
    /// is also called.
    pub fn is_permanent(&self) -> bool {
        self.0.state.load(Ordering::Relaxed) == PERMANENT
    }

    /// Force drops the handle, meaning the resource will be dropped even if there are other handles active.
    pub fn force_drop(self) {
        self.0.state.store(FORCE_DROP, Ordering::Relaxed);
    }

    /// If the handle is in *dropped* state.
    ///
    /// The handle is considered dropped when all handle and clones are dropped or when [`force_drop`](Handle::force_drop)
    /// was called in any of the clones.
    ///
    /// Note that in this method it can only be because [`force_drop`](Handle::force_drop) was called.
    pub fn is_dropped(&self) -> bool {
        self.0.state.load(Ordering::Relaxed) == FORCE_DROP
    }

    /// Create a [`WeakHandle`] to this handle.
    pub fn downgrade(&self) -> WeakHandle<D> {
        WeakHandle(Arc::downgrade(&self.0))
    }
}
impl<D: Send + Sync> Clone for Handle<D> {
    fn clone(&self) -> Self {
        Handle(Arc::clone(&self.0))
    }
}
impl<D: Send + Sync> PartialEq for Handle<D> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<D: Send + Sync> Eq for Handle<D> {}
impl<D: Send + Sync> std::hash::Hash for Handle<D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Arc::as_ptr(&self.0) as usize;
        ptr.hash(state);
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
impl<D: Send + Sync> fmt::Debug for Handle<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_permanent() {
            write!(f, "permanent")
        } else if self.is_dropped() {
            write!(f, "dropped")
        } else {
            write!(f, "holding")
        }
    }
}

/// A weak reference to a [`Handle`].
pub(crate) struct WeakHandle<D: Send + Sync>(Weak<HandleState<D>>);
impl<D: Send + Sync> WeakHandle<D> {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        WeakHandle(Weak::new())
    }

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
impl<D: Send + Sync> Default for WeakHandle<D> {
    fn default() -> Self {
        Self::new()
    }
}
impl<D: Send + Sync> Clone for WeakHandle<D> {
    fn clone(&self) -> Self {
        WeakHandle(self.0.clone())
    }
}
impl<D: Send + Sync> PartialEq for WeakHandle<D> {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}
impl<D: Send + Sync> Eq for WeakHandle<D> {}
impl<D: Send + Sync> std::hash::Hash for WeakHandle<D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = self.0.as_ptr() as usize;
        ptr.hash(state);
    }
}
impl<D: Send + Sync> fmt::Debug for WeakHandle<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.strong_count() > 0 {
            write!(f, "can-upgrade")
        } else {
            write!(f, "dropped")
        }
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

    /*
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
    */

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
pub type PanicResult<R> = thread::Result<R>;

// this is the FxHasher with a patch that fixes slow deserialization.
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

/// Like [`rustc_hash::FxHashMap`] but faster deserialization and access to the raw_entry API.
pub type FxHashMap<K, V> = hashbrown::HashMap<K, V, BuildFxHasher>;
/// Like [`rustc_hash::FxHashSet`] but faster deserialization.
pub type FxHashSet<V> = hashbrown::HashSet<V, BuildFxHasher>;

/// Entry in [`FxHashMap`].
pub type FxEntry<'a, K, V> = hashbrown::hash_map::Entry<'a, K, V, BuildFxHasher>;

/// Bidirectional map between a `Text` and a [`unique_id!`] generated id type.
pub struct NameIdMap<I> {
    name_to_id: HashMap<Text, I>,
    id_to_name: IdMap<I, Text>,
}
impl<I: Copy + PartialEq + Eq + std::hash::Hash + fmt::Debug> NameIdMap<I> {
    pub fn new() -> Self {
        NameIdMap {
            name_to_id: HashMap::default(),
            id_to_name: IdMap::default(),
        }
    }

    pub fn set(&mut self, name: Text, id: I) -> Result<(), IdNameError<I>> {
        if name.is_empty() {
            return Ok(());
        }

        match self.id_to_name.entry(id) {
            IdEntry::Occupied(e) => {
                if *e.get() == name {
                    Ok(())
                } else {
                    Err(IdNameError::AlreadyNamed(e.get().clone()))
                }
            }
            IdEntry::Vacant(e) => match self.name_to_id.entry(name.clone()) {
                hash_map::Entry::Occupied(ne) => Err(IdNameError::NameUsed(*ne.get())),
                hash_map::Entry::Vacant(ne) => {
                    e.insert(name);
                    ne.insert(id);
                    Ok(())
                }
            },
        }
    }

    pub fn get_id_or_insert(&mut self, name: Text, new_unique: impl FnOnce() -> I) -> I {
        if name.is_empty() {
            return new_unique();
        }
        match self.name_to_id.entry(name.clone()) {
            hash_map::Entry::Occupied(e) => *e.get(),
            hash_map::Entry::Vacant(e) => {
                let id = new_unique();
                e.insert(id);
                self.id_to_name.insert(id, name);
                id
            }
        }
    }

    pub fn new_named(&mut self, name: Text, new_unique: impl FnOnce() -> I) -> Result<I, IdNameError<I>> {
        if name.is_empty() {
            Ok(new_unique())
        } else {
            match self.name_to_id.entry(name.clone()) {
                hash_map::Entry::Occupied(e) => Err(IdNameError::NameUsed(*e.get())),
                hash_map::Entry::Vacant(e) => {
                    let id = new_unique();
                    e.insert(id);
                    self.id_to_name.insert(id, name);
                    Ok(id)
                }
            }
        }
    }

    pub fn get_name(&self, id: I) -> Text {
        self.id_to_name.get(&id).cloned().unwrap_or_default()
    }
}

/// Error when trying to associate give a name with an existing id.
#[derive(Clone, Debug)]
pub enum IdNameError<I: Clone + Copy + fmt::Debug> {
    /// The id is already named, id names are permanent.
    ///
    /// The associated value if the id name.
    AlreadyNamed(Text),
    /// The name is already used for another id, names must be unique.
    ///
    /// The associated value if the named id.
    NameUsed(I),
}
impl<I: Clone + Copy + fmt::Debug> fmt::Display for IdNameError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdNameError::AlreadyNamed(name) => write!(f, "cannot name the id, it is already called `{name:?}`"),
            IdNameError::NameUsed(id) => write!(f, "cannot name the id, it is already the name of {id:#?}"),
        }
    }
}
impl<I: Clone + Copy + fmt::Debug> std::error::Error for IdNameError<I> {}

/// Resolves `..` components, without any system request.
///
/// Source: https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

/// Resolves relative paths in the `root` and normalizes then.
///
/// The `base` is only evaluated if the `path` is relative.
///
/// If `allow_escape` is `false`, relative paths with `..` cannot reference outside of `base`.
pub fn absolute_path(path: &Path, base: impl FnOnce() -> PathBuf, allow_escape: bool) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        let mut dir = base();
        if allow_escape {
            dir.push(path);
            normalize_path(&dir)
        } else {
            dir.push(normalize_path(path));
            dir
        }
    }
}

/// A temporary directory for unit tests.
///
/// Directory is "target/tmp/unit_tests/<name>" with fallback to system temporary if the target folder is not found.
///
/// Auto cleanup on drop.
#[cfg(test)]
pub struct TestTempDir {
    path: Option<PathBuf>,
}
#[cfg(test)]
impl Drop for TestTempDir {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = remove_dir_all::remove_dir_all(path);
        }
    }
}
#[cfg(test)]
impl TestTempDir {
    /// Create temporary directory for the unique teste name.
    pub fn new(name: &str) -> Self {
        let path = Self::try_target().unwrap_or_else(Self::fallback).join(name);
        std::fs::create_dir_all(&path).unwrap_or_else(|e| panic!("failed to create temp `{}`, {e:?}", path.display()));
        TestTempDir { path: Some(path) }
    }
    fn try_target() -> Option<PathBuf> {
        let p = std::env::current_exe().ok()?;
        // target/debug/deps/../../..
        let target = p.parent()?.parent()?.parent()?;
        if target.file_name()?.to_str()? != "target" {
            return None;
        }
        Some(target.join("tmp/unit_tests"))
    }
    fn fallback() -> PathBuf {
        tracing::warn!("using fallback temporary directory");
        std::env::temp_dir().join("zero_ui/unit_tests")
    }

    /// Dereferences the temporary directory path.
    pub fn path(&self) -> &Path {
        self.path.as_deref().unwrap()
    }

    /// Drop `self` without removing the temporary files.
    ///
    /// Returns the path to the temporary directory.
    pub fn keep(mut self) -> PathBuf {
        self.path.take().unwrap()
    }
}
#[cfg(test)]
impl std::ops::Deref for TestTempDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}
#[cfg(test)]
impl std::convert::AsRef<Path> for TestTempDir {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}
#[cfg(test)]
impl<'a> From<&'a TestTempDir> for std::path::PathBuf {
    fn from(a: &'a TestTempDir) -> Self {
        a.path.as_ref().unwrap().clone()
    }
}

/// Sets a `tracing` subscriber that writes warnings to stderr and panics on errors.
///
/// Panics if another different subscriber is already set.
#[cfg(any(test, feature = "test_util"))]
pub fn test_log() {
    use std::sync::atomic::*;

    use tracing::*;

    struct TestSubscriber;
    impl Subscriber for TestSubscriber {
        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            metadata.is_event() && metadata.level() < &Level::WARN
        }

        fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
            unimplemented!()
        }

        fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {
            unimplemented!()
        }

        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
            unimplemented!()
        }

        fn event(&self, event: &Event<'_>) {
            struct MsgCollector<'a>(&'a mut String);
            impl<'a> field::Visit for MsgCollector<'a> {
                fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
                    use std::fmt::Write;
                    write!(self.0, "\n  {} = {:?}", field.name(), value).unwrap();
                }
            }

            let meta = event.metadata();
            let file = meta.file().unwrap_or("");
            let line = meta.line().unwrap_or(0);

            let mut msg = format!("[{file}:{line}]");
            event.record(&mut MsgCollector(&mut msg));

            if meta.level() == &Level::ERROR {
                panic!("[LOG-ERROR]{msg}");
            } else {
                eprintln!("[LOG-WARN]{msg}");
            }
        }

        fn enter(&self, _span: &span::Id) {
            unimplemented!()
        }
        fn exit(&self, _span: &span::Id) {
            unimplemented!()
        }
    }

    static IS_SET: AtomicBool = AtomicBool::new(false);

    if !IS_SET.swap(true, Ordering::Relaxed) {
        if let Err(e) = subscriber::set_global_default(TestSubscriber) {
            panic!("failed to set test log subscriber, {e:?}");
        }
    }
}

/// Calls [`fs2::FileExt::unlock`] and ignores "already unlocked" errors.
#[allow(unused)] // http only
pub fn unlock_ok(file: &impl fs2::FileExt) -> std::io::Result<()> {
    if let Err(e) = file.unlock() {
        if let Some(code) = e.raw_os_error() {
            #[cfg(windows)]
            if code == 158 {
                // ERROR_NOT_LOCKED
                return Ok(());
            }

            #[cfg(unix)]
            if code == 22 {
                // EINVAL
                return Ok(());
            }
        }

        Err(e)
    } else {
        Ok(())
    }
}

/// Like [`std::ops::Range<usize>`], but implements [`Copy`].
#[derive(Clone, Copy)]
pub struct IndexRange(pub usize, pub usize);
impl fmt::Debug for IndexRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.0, self.1)
    }
}
impl IntoIterator for IndexRange {
    type Item = usize;

    type IntoIter = std::ops::Range<usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl From<IndexRange> for std::ops::Range<usize> {
    fn from(c: IndexRange) -> Self {
        c.iter()
    }
}
impl From<std::ops::Range<usize>> for IndexRange {
    fn from(r: std::ops::Range<usize>) -> Self {
        IndexRange(r.start, r.end)
    }
}
impl IndexRange {
    /// Into `Range<usize>`.
    pub fn iter(self) -> std::ops::Range<usize> {
        self.0..self.1
    }

    /// `self.0`
    pub fn start(self) -> usize {
        self.0
    }

    /// `self.1`
    pub fn end(self) -> usize {
        self.1
    }

    /// `self.1.saturating_sub(1)`
    pub fn inclusive_end(self) -> usize {
        self.1.saturating_sub(1)
    }

    /// `self.end - self.start`
    pub fn len(self) -> usize {
        self.end() - self.start()
    }
}
impl std::ops::RangeBounds<usize> for IndexRange {
    fn start_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Included(&self.0)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Excluded(&self.1)
    }
}

/// `f32` comparison, panics for `NaN`.
pub fn f32_cmp(a: &f32, b: &f32) -> std::cmp::Ordering {
    a.partial_cmp(b).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget_instance::WidgetId;
    use fs2::FileExt;

    #[test]
    pub fn sequential_id() {
        let id0 = WidgetId::new_unique();
        let id1 = WidgetId::new_unique();

        assert!(id0.sequential() < id1.sequential());
    }

    #[test]
    fn unlock_ok_exclusive_already_unlocked() {
        let dir = TestTempDir::new("unlock_ok_exclusive_already_unlocked");

        let file = std::fs::File::create(dir.join(".lock")).unwrap();
        file.lock_exclusive().unwrap();

        file.unlock().unwrap();

        unlock_ok(&file).unwrap();
    }

    #[test]
    fn unlock_ok_shared_already_unlocked() {
        let dir = TestTempDir::new("unlock_ok_shared_already_unlocked");

        let file = std::fs::File::create(dir.join(".lock")).unwrap();
        file.lock_shared().unwrap();

        file.unlock().unwrap();

        unlock_ok(&file).unwrap();
    }

    #[test]
    fn unlock_ok_exclusive_never_locked() {
        let dir = TestTempDir::new("unlock_ok_exclusive_never_locked");

        let file = std::fs::File::create(dir.join(".lock")).unwrap();

        unlock_ok(&file).unwrap();
    }
}

#[allow(unused)]
macro_rules! print_backtrace {
    () => {{
        let bt = std::backtrace::Backtrace::capture();
        println!("[{}:{}] BACKTRACE\n{bt}\n=====\n", file!(), line!())
    }};
}

/// Extension methods for [`flume::Receiver<T>`].
pub trait ReceiverExt<T> {
    /// Receive or precise timeout.
    fn recv_deadline_sp(&self, deadline: Deadline) -> Result<T, flume::RecvTimeoutError>;
}

const WORST_SLEEP_ERR: Duration = Duration::from_millis(if cfg!(windows) { 20 } else { 10 });
const WORST_SPIN_ERR: Duration = Duration::from_millis(if cfg!(windows) { 2 } else { 1 });

impl<T> ReceiverExt<T> for flume::Receiver<T> {
    fn recv_deadline_sp(&self, deadline: Deadline) -> Result<T, flume::RecvTimeoutError> {
        if let Some(d) = deadline.0.checked_duration_since(Instant::now()) {
            if d > WORST_SLEEP_ERR {
                // probably sleeps here.
                match self.recv_deadline(deadline.0.checked_sub(WORST_SLEEP_ERR).unwrap()) {
                    Err(flume::RecvTimeoutError::Timeout) => self.recv_deadline_sp(deadline),
                    interrupt => interrupt,
                }
            } else if d > WORST_SPIN_ERR {
                let spin_deadline = Deadline(deadline.0.checked_sub(WORST_SPIN_ERR).unwrap());

                // try_recv spin
                while !spin_deadline.has_elapsed() {
                    match self.try_recv() {
                        Err(flume::TryRecvError::Empty) => thread::yield_now(),
                        Err(flume::TryRecvError::Disconnected) => return Err(flume::RecvTimeoutError::Disconnected),
                        Ok(msg) => return Ok(msg),
                    }
                }
                self.recv_deadline_sp(deadline)
            } else {
                // last millis spin
                while !deadline.has_elapsed() {
                    std::thread::yield_now();
                }
                Err(flume::RecvTimeoutError::Timeout)
            }
        } else {
            Err(flume::RecvTimeoutError::Timeout)
        }
    }
}

/// Pre-compile generic variation so that dependent crates don't need to.
#[allow(unused)]
macro_rules! share_generics {
    ($f:path) => {
        #[doc(hidden)]
        #[cfg(debug_assertions)]
        pub const _: *const () = (&$f) as *const _ as _;
    };
}

#[allow(unused)]
#[doc(hidden)]
pub(crate) struct MeasureTime {
    msg: &'static str,
    started: std::time::Instant,
}
impl MeasureTime {
    #[allow(unused)]
    pub(crate) fn start(msg: &'static str) -> Self {
        MeasureTime {
            msg,
            started: std::time::Instant::now(),
        }
    }
}
impl Drop for MeasureTime {
    fn drop(&mut self) {
        println!("{}: {:?}", self.msg, self.started.elapsed());
    }
}

/// Time an operation, time elapsed is printed on drop.
#[allow(unused)]
macro_rules! measure_time {
    ($msg:tt) => {
        $crate::crate_util::MeasureTime::start($msg)
    };
}

pub type BoxedFut<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + Sync>>;

#[allow(unused)]
pub(crate) struct RecursionCheck {
    count: AtomicUsize,
    limit: usize,
}
#[allow(unused)]
impl RecursionCheck {
    pub const fn new(limit: usize) -> Self {
        RecursionCheck {
            count: AtomicUsize::new(0),
            limit,
        }
    }

    pub fn enter(&'static self) -> RecursionCheckExitOnDrop {
        let c = self.count.fetch_add(1, Ordering::Relaxed);
        if c >= self.limit {
            panic!("reached {} limit, probably recursing", self.limit);
        }
        RecursionCheckExitOnDrop { check: self }
    }
}

#[must_use = "must be held while calling inner"]
#[allow(unused)]
pub(crate) struct RecursionCheckExitOnDrop {
    check: &'static RecursionCheck,
}
impl Drop for RecursionCheckExitOnDrop {
    fn drop(&mut self) {
        self.check.count.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Like [`assert!`], but only logs an error.
#[allow(unused)]
macro_rules! trace_assert {
    ($cond:expr $(,)?) => {
        #[allow(clippy::all)]
        if !($cond) {
            tracing::error!("{}", stringify!($cond));
        }
    };
    ($cond:expr, $($arg:tt)+) => {
        #[allow(clippy::all)]
        if !($cond) {
            tracing::error!($($arg)*);
        }
    };
}

//! Crate visible macros and utilities.

use rand::Rng;
use rustc_hash::FxHasher;
use std::{
    collections::{hash_map, HashMap, HashSet},
    fmt,
    hash::{BuildHasher, BuildHasherDefault, Hasher},
    num::{NonZeroU32, NonZeroU64},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering},
        Arc, Weak,
    },
};

/// Declare a new unique id type that is backed by a `NonZeroU32`.
macro_rules! unique_id_32 {
    ($(#[$attrs:meta])* $vis:vis struct $Type:ident;) => {
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
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        $vis struct $Type(std::num::NonZeroU32);

        impl $Type {
            /// Generates a new unique ID.
            pub fn new_unique() -> Self {
                use std::sync::atomic::AtomicU32;
                static NEXT: AtomicU32 = AtomicU32::new(1);
                Self($crate::crate_util::next_id32(&NEXT))
            }

            /// Retrieve the underlying `u32` value.
            #[allow(dead_code)]
            #[inline]
            pub fn get(self) -> u32 {
                self.0.get()
            }

            /// Un-scramble the underlying value to get the original sequential count number.
            ///
            /// If two ids, `id0` and `id1` where generated by the same thread then `id0.sequential() < id1.sequential()`.
            #[allow(dead_code)]
            pub fn sequential(self) -> u32 {
                $crate::crate_util::un_hash32(self.0.get())
            }

            /// Creates an id from a raw value.
            ///
            /// # Safety
            ///
            /// The value must not be zero, panics in debug builds if it is, the value must have been provided by [`get`] otherwise
            /// the ID will not be unique, it may represent a random resource existing or future.
            ///
            /// [`get`]: Self::get
            #[allow(dead_code)]
            pub unsafe fn from_raw(raw: u32) -> $Type {
                debug_assert!(raw != 0);
                $Type(std::num::NonZeroU32::new_unchecked(raw))
            }
        }
    }
}

/// Declare a new unique id type that is backed by a `NonZeroU64`.
macro_rules! unique_id_64 {
    ($(#[$attrs:meta])* $vis:vis struct $Type:ident;) => {

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
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        $vis struct $Type(std::num::NonZeroU64);

        impl $Type {
            /// Generates a new unique ID.
            pub fn new_unique() -> Self {
                use std::sync::atomic::AtomicU64;
                static NEXT: AtomicU64 = AtomicU64::new(1);
                Self($crate::crate_util::next_id64(&NEXT))
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
            /// The value must not be zero, panics in debug builds if it is, the value must have been provided by [`get`] otherwise
            /// the ID will not be unique, it may represent a random resource existing or future.
            ///
            /// [`get`]: Self::get
            #[allow(dead_code)]
            pub unsafe fn from_raw(raw: u64) -> $Type {
                $Type(std::num::NonZeroU64::new_unchecked(raw))
            }
        }
    };
}
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

#[inline]
fn hash32(n: u32) -> u32 {
    use std::num::Wrapping as W;

    let mut z = W(n);
    z = ((z >> 16) ^ z) * W(0x45d9f3b);
    z = ((z >> 16) ^ z) * W(0x45d9f3b);
    z = (z >> 16) ^ z;
    z.0
}
#[doc(hidden)]
#[inline]
pub fn un_hash32(z: u32) -> u32 {
    use std::num::Wrapping as W;

    let mut n = W(z);
    n = ((n >> 16) ^ n) * W(0x119de1f3);
    n = ((n >> 16) ^ n) * W(0x119de1f3);
    n = (n >> 16) ^ n;
    n.0
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
#[inline]
pub fn un_splitmix64(z: u64) -> u64 {
    use std::num::Wrapping as W;

    let mut n = W(z);
    n = (n ^ (n >> 31) ^ (n >> 62)) * W(0x319642b2d24d8ec3u64);
    n = (n ^ (n >> 27) ^ (n >> 54)) * W(0x96de1b173f119089u64);
    n = n ^ (n >> 30) ^ (n >> 60);
    n.0
}

/// Ideal map type for key types generated using [`unique_id!`].
pub type IdMap<K, V> = HashMap<K, V, BuildHasherDefault<IdHasher>>;
/// Ideal set type for key types generated using [`unique_id!`].
pub type IdSet<K> = HashSet<K, BuildHasherDefault<IdHasher>>;

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
                    panic!("only a single instance of `{type_name}` can exist per thread at a time")
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

/// Bidirectional map between a `&'static str` and a [`unique_id!`] generated id type.
pub struct NameIdMap<I> {
    name_to_id: HashMap<&'static str, I>,
    id_to_name: FxHashMap<I, &'static str>,
}
impl<I: Copy + PartialEq + Eq + std::hash::Hash + fmt::Debug> NameIdMap<I> {
    pub fn new() -> Self {
        NameIdMap {
            name_to_id: HashMap::default(),
            id_to_name: FxHashMap::default(),
        }
    }

    pub fn set(&mut self, name: &'static str, id: I) -> Result<(), IdNameError<I>> {
        if name.is_empty() {
            return Ok(());
        }

        match self.id_to_name.entry(id) {
            FxEntry::Occupied(e) => {
                if *e.get() == name {
                    Ok(())
                } else {
                    Err(IdNameError::AlreadyNamed(*e.get()))
                }
            }
            FxEntry::Vacant(e) => match self.name_to_id.entry(name) {
                hash_map::Entry::Occupied(ne) => Err(IdNameError::NameUsed(*ne.get())),
                hash_map::Entry::Vacant(ne) => {
                    e.insert(name);
                    ne.insert(id);
                    Ok(())
                }
            },
        }
    }

    pub fn get_id_or_insert(&mut self, name: &'static str, new_unique: impl FnOnce() -> I) -> I {
        if name.is_empty() {
            return new_unique();
        }
        match self.name_to_id.entry(name) {
            hash_map::Entry::Occupied(e) => *e.get(),
            hash_map::Entry::Vacant(e) => {
                let id = new_unique();
                e.insert(id);
                self.id_to_name.insert(id, name);
                id
            }
        }
    }

    pub fn new_named(&mut self, name: &'static str, new_unique: impl FnOnce() -> I) -> Result<I, IdNameError<I>> {
        if name.is_empty() {
            Ok(new_unique())
        } else {
            match self.name_to_id.entry(name) {
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

    pub fn get_name(&self, id: I) -> &'static str {
        self.id_to_name.get(&id).copied().unwrap_or_default()
    }
}

/// Error when trying to associate give a name with an existing id.
#[derive(Clone, Debug, Copy)]
pub enum IdNameError<I: Clone + Copy + fmt::Debug> {
    /// The id is already named, id names are permanent.
    ///
    /// The associated value if the id name.
    AlreadyNamed(&'static str),
    /// The name is already used for another id, names must be unique.
    ///
    /// The associated value if the named id.
    NameUsed(I),
}
impl<I: Clone + Copy + fmt::Debug> fmt::Display for IdNameError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
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
#[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
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
    #[inline]
    pub fn iter(self) -> std::ops::Range<usize> {
        self.0..self.1
    }

    /// `self.0`
    #[inline]
    pub fn start(self) -> usize {
        self.0
    }

    /// `self.1`
    #[inline]
    pub fn end(self) -> usize {
        self.1
    }

    /// `self.1.saturating_sub(1)`
    #[inline]
    pub fn inclusive_end(self) -> usize {
        self.1.saturating_sub(1)
    }
}

/// `f32` comparison, panics for `NaN`.
pub fn f32_cmp(a: &f32, b: &f32) -> std::cmp::Ordering {
    a.partial_cmp(b).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WidgetId;
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

/*
macro_rules! print_backtrace {
    () => {{
        let bt = backtrace::Backtrace::new();
        println!("[{}:{}] BACKTRACE\n{bt:?}\n=====\n", file!(), line!())
    }}
}
*/

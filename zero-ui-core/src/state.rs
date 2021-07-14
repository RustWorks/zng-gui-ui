use std::{
    any::{type_name, TypeId},
    fmt,
    marker::PhantomData,
};

use unsafe_any::UnsafeAny;

use crate::crate_util::AnyMap;

/// A key to a value in a [`StateMap`].
///
/// The type that implements this trait is the key. You
/// can use the [`state_key!`](crate::context::state_key) macro.
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait StateKey: 'static {
    /// The value type.
    type Type: 'static;
}

/// Declares new [`StateKey`](crate::context::StateKey) types.
///
/// # Example
///
/// ```
/// # use zero_ui_core::context::state_key;
/// state_key! {
///     /// Key docs.
///     pub struct FooKey: u32;
/// }
/// ```
/// # Naming Convention
///
/// It is recommended that the type name ends with the key suffix.
#[macro_export]
macro_rules! state_key {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty;)+) => {$(
        $(#[$outer])*
        /// # StateKey
        /// This `struct` is a [`StateKey`](crate::context::StateKey).
        #[derive(Clone, Copy)]
        $vis struct $ident;

        impl $crate::context::StateKey for $ident {
            type Type = $type;
        }
    )+};
}

/// A map of [state keys](StateKey) to values of their associated types that exists for
/// a stage of the application.
///
/// # No Remove
///
/// Note that there is no way to clear the map, remove a key or replace the map with a new empty one.
/// This is by design, if you want to make a key *removable* make its value `Option<T>`.
pub struct StateMap {
    map: AnyMap,
}
impl fmt::Debug for StateMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateMap[{} entries]", self.map.len())
    }
}
impl StateMap {
    pub(crate) fn new() -> Self {
        StateMap { map: AnyMap::default() }
    }

    /// Set the key `value`.
    ///
    /// # Key
    ///
    /// Use [`state_key!`](crate::context::state_key) to generate a key, any static type can be a key,
    /// the [type id](TypeId) is the actual key.
    pub fn set<S: StateKey>(&mut self, value: S::Type) -> Option<S::Type> {
        self.map.insert(TypeId::of::<S>(), Box::new(value)).map(|any| {
            // SAFETY: The type system asserts this is valid.
            unsafe { *any.downcast_unchecked::<S::Type>() }
        })
    }

    /// Sets a value that is its own [`StateKey`].
    pub fn set_single<S: StateKey<Type = S>>(&mut self, value: S) -> Option<S> {
        self.map.insert(TypeId::of::<S>(), Box::new(value)).map(|any| {
            // SAFETY: The type system asserts this is valid.
            unsafe { *any.downcast_unchecked::<S>() }
        })
    }

    /// Gets if the key is set in this map.
    pub fn contains<S: StateKey>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<S>())
    }

    /// Reference the key value set in this map.
    pub fn get<S: StateKey>(&self) -> Option<&S::Type> {
        self.map.get(&TypeId::of::<S>()).map(|any| {
            // SAFETY: The type system asserts this is valid.
            unsafe { any.downcast_ref_unchecked::<S::Type>() }
        })
    }

    /// Mutable borrow the key value set in this map.
    pub fn get_mut<S: StateKey>(&mut self) -> Option<&mut S::Type> {
        self.map.get_mut(&TypeId::of::<S>()).map(|any| {
            // SAFETY: The type system asserts this is valid.
            unsafe { any.downcast_mut_unchecked::<S::Type>() }
        })
    }

    /// Reference the key value set in this map or panics if the key is not set.
    pub fn req<S: StateKey>(&self) -> &S::Type {
        self.get::<S>()
            .unwrap_or_else(|| panic!("expected `{}` in state map", type_name::<S>()))
    }

    /// Mutable borrow the key value set in this map or panics if the key is not set.
    pub fn req_mut<S: StateKey>(&mut self) -> &mut S::Type {
        self.get_mut::<S>()
            .unwrap_or_else(|| panic!("expected `{}` in state map", type_name::<S>()))
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry<S: StateKey>(&mut self) -> StateMapEntry<S> {
        StateMapEntry {
            _key: PhantomData,
            entry: self.map.entry(TypeId::of::<S>()),
        }
    }

    /// Sets a state key without value.
    ///
    /// Returns if the state key was already flagged.
    pub fn flag<S: StateKey<Type = ()>>(&mut self) -> bool {
        self.set::<S>(()).is_some()
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<S>())
    }

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// A view into a single entry in a state map, which may either be vacant or occupied.
pub struct StateMapEntry<'a, S: StateKey> {
    _key: PhantomData<S>,
    entry: std::collections::hash_map::Entry<'a, TypeId, Box<dyn UnsafeAny>>,
}
impl<'a, S: StateKey> StateMapEntry<'a, S> {
    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns a mutable reference to the value in the entry.
    pub fn or_insert(self, default: S::Type) -> &'a mut S::Type {
        // SAFETY: The type system asserts this is valid.
        unsafe { self.entry.or_insert_with(|| Box::new(default)).downcast_mut_unchecked::<S::Type>() }
    }

    /// Ensures a value is in the entry by inserting the result of the
    /// default function if empty, and returns a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> S::Type>(self, default: F) -> &'a mut S::Type {
        // SAFETY: The type system asserts this is valid.
        unsafe {
            self.entry
                .or_insert_with(|| Box::new(default()))
                .downcast_mut_unchecked::<S::Type>()
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts into the map.
    pub fn and_modify<F: FnOnce(&mut S::Type)>(self, f: F) -> Self {
        let entry = self.entry.and_modify(|a| {
            f({
                // SAFETY: The type system asserts this is valid.
                unsafe { a.downcast_mut_unchecked::<S::Type>() }
            })
        });
        StateMapEntry { _key: PhantomData, entry }
    }
}
impl<'a, S: StateKey> StateMapEntry<'a, S>
where
    S::Type: Default,
{
    /// Ensures a value is in the entry by inserting the default value if empty,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_default(self) -> &'a mut S::Type {
        // SAFETY: The type system asserts this is valid.
        unsafe {
            self.entry
                .or_insert_with(|| Box::new(<S::Type as Default>::default()))
                .downcast_mut_unchecked::<S::Type>()
        }
    }
}

/// Private [`StateMap`].
///
/// The owner of a state map has full access including to the `remove` and `clear` function that is not
/// provided in the [`StateMap`] type.
pub struct OwnedStateMap(pub(crate) StateMap); // TODO deref StateMap?
impl Default for OwnedStateMap {
    fn default() -> Self {
        OwnedStateMap(StateMap::new())
    }
}
impl OwnedStateMap {
    /// New default, empty.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove the key.
    pub fn remove<S: StateKey>(&mut self) -> Option<S::Type> {
        self.0.map.remove(&TypeId::of::<S>()).map(|a| {
            // SAFETY: The type system asserts this is valid.
            unsafe { *a.downcast_unchecked::<S::Type>() }
        })
    }

    /// Removes all entries.
    #[inline]
    pub fn clear(&mut self) {
        self.0.map.clear()
    }

    /// Set the key `value`.
    ///
    /// # Key
    ///
    /// Use [`state_key!`](crate::context::state_key) to generate a key, any static type can be a key,
    /// the [type id](TypeId) is the actual key.
    pub fn set<S: StateKey>(&mut self, value: S::Type) -> Option<S::Type> {
        self.0.set::<S>(value)
    }

    /// Sets a value that is its own [`StateKey`].
    pub fn set_single<S: StateKey<Type = S>>(&mut self, value: S) -> Option<S> {
        self.0.set_single::<S>(value)
    }

    /// Gets if the key is set in this map.
    pub fn contains<S: StateKey>(&self) -> bool {
        self.0.contains::<S>()
    }

    /// Reference the key value set in this map.
    pub fn get<S: StateKey>(&self) -> Option<&S::Type> {
        self.0.get::<S>()
    }

    /// Mutable borrow the key value set in this map.
    pub fn get_mut<S: StateKey>(&mut self) -> Option<&mut S::Type> {
        self.0.get_mut::<S>()
    }

    /// Reference the key value set in this map, or panics if the key is not set.
    pub fn req<S: StateKey>(&self) -> &S::Type {
        self.0.req::<S>()
    }

    /// Mutable borrow the key value set in this map, or panics if the key is not set.
    pub fn req_mut<S: StateKey>(&mut self) -> &mut S::Type {
        self.0.req_mut::<S>()
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry<S: StateKey>(&mut self) -> StateMapEntry<S> {
        self.0.entry::<S>()
    }

    /// Sets a state key without value.
    ///
    /// Returns if the state key was already flagged.
    pub fn flag<S: StateKey<Type = ()>>(&mut self) -> bool {
        self.0.flag::<S>()
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self) -> bool {
        self.0.flagged::<S>()
    }

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A [`StateMap`] with optional fallback map.
pub struct StateMapFb<'a> {
    fallback: Option<&'a mut StateMap>,
    map: &'a mut StateMap,
}
impl<'a> fmt::Debug for StateMapFb<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateMap[{} entries]", self.map.map.len())
    }
}
impl<'a> StateMapFb<'a> {
    /// New with optional fallback.
    pub fn new(fallback: Option<&'a mut StateMap>, map: &'a mut StateMap) -> Self {
        StateMapFb { fallback, map }
    }

    /// Set the key `value`.
    ///
    /// Does not affect the fallback map.
    pub fn set<S: StateKey>(&mut self, value: S::Type) -> Option<S::Type> {
        self.map.set::<S>(value)
    }

    /// Sets a value that is its own [`StateKey`].
    ///
    /// Does not affect the fallback map.
    pub fn set_single<S: StateKey<Type = S>>(&mut self, value: S) -> Option<S> {
        self.map.set_single::<S>(value)
    }

    /// Gets if the key is set in the map or in the fallback map.
    pub fn contains<S: StateKey>(&self) -> bool {
        self.map.contains::<S>() || self.fallback.as_ref().map(|m| m.contains::<S>()).unwrap_or(false)
    }

    /// Reference the key value set in the map or in the fallback map.
    pub fn get<S: StateKey>(&self) -> Option<&S::Type> {
        self.map.get::<S>().or_else(|| self.fallback.as_ref().and_then(|m| m.get::<S>()))
    }

    /// Mutable borrow the key value set in the map.
    ///
    /// If the value was not set in the map but was set in the fallback map, clones the value and sets it in the
    /// map before returning the reference.
    pub fn get_mut<S: StateKey>(&mut self) -> Option<&mut S::Type>
    where
        S::Type: Clone,
    {
        if let Some(fallback) = &self.fallback {
            // don't known how to make this more efficient due to lifetime issues.
            if !self.map.contains::<S>() {
                if let Some(v) = fallback.get::<S>() {
                    let v = v.clone();
                    self.set::<S>(v);
                }
            }
        }
        self.map.get_mut::<S>()
    }

    /// Reference the key value set in this map or panics if the key is not set.
    pub fn req<S: StateKey>(&self) -> &S::Type {
        self.get::<S>()
            .unwrap_or_else(|| panic!("expected `{}` in state map or fallback", type_name::<S>()))
    }

    /// Mutable borrow the key value set in this map or panics if the key is not set.
    ///
    /// If the value was not set in the map but was set in the fallback map, clones the value and sets it in the
    /// map before returning the reference.
    pub fn req_mut<S: StateKey>(&mut self) -> &mut S::Type
    where
        S::Type: Clone,
    {
        self.get_mut::<S>()
            .unwrap_or_else(|| panic!("expected `{}` in state map or fallback", type_name::<S>()))
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    ///
    /// If the key is not present in the map but is present in the fallback map
    /// **it is cloned and inserted** in the map before the entry is returned.
    pub fn entry<S: StateKey>(&mut self) -> StateMapEntry<S>
    where
        S::Type: Clone,
    {
        if let Some(fallback) = &self.fallback {
            // don't known how to make this more efficient due to lifetime issues.
            if !self.map.contains::<S>() {
                if let Some(v) = fallback.get::<S>() {
                    let v = v.clone();
                    self.set::<S>(v);
                }
            }
        }
        self.map.entry::<S>()
    }

    /// Sets a state key without value.
    ///
    /// Returns if the state key was already flagged in the map or the fallback map.
    pub fn flag<S: StateKey<Type = ()>>(&mut self) -> bool {
        self.set::<S>(()).is_some()
    }

    /// Gets if a state key without value is set.
    pub fn flagged<S: StateKey<Type = ()>>(&self) -> bool {
        self.map.flagged::<S>() || self.fallback.as_ref().map(|m| m.flagged::<S>()).unwrap_or(false)
    }

    /// If no state is set.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty() && self.fallback.as_ref().map(|m| m.is_empty()).unwrap_or(true)
    }
}

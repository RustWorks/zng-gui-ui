use std::marker::PhantomData;

use super::*;

/// A [`Var`] wrapper that makes it [`always_read_only`](Var::always_read_only).
pub struct ReadOnlyVar<T: VarValue, V: Var<T>>(V, PhantomData<T>);

impl<T, V> ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    /// Wrap var.
    #[inline]
    pub fn new(var: V) -> Self {
        ReadOnlyVar(var, PhantomData)
    }
}

impl<T, V> Clone for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T> + Clone,
{
    fn clone(&self) -> Self {
        ReadOnlyVar(self.0.clone(), PhantomData)
    }
}

impl<T, V> Var<T> for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type AsReadOnly = Self;

    type AsLocal = ReadOnlyVar<T, V::AsLocal>;

    #[inline]
    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.0.get(vars)
    }

    #[inline]
    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.0.get_new(vars)
    }

    #[inline]
    fn version(&self, vars: &VarsRead) -> u32 {
        self.0.version(vars)
    }

    #[inline]
    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        true
    }

    #[inline]
    fn can_update(&self) -> bool {
        self.0.can_update()
    }

    #[inline]
    fn modify<M>(&self, _: &Vars, _: M) -> Result<(), VarIsReadOnly>
    where
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set<N>(&self, _: &Vars, _: N) -> Result<(), VarIsReadOnly>
    where
        N: Into<T>,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set_ne<N>(&self, _: &Vars, _: N) -> Result<bool, VarIsReadOnly>
    where
        N: Into<T>,
        T: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    #[inline]
    fn into_local(self) -> Self::AsLocal {
        ReadOnlyVar::new(Var::into_local(self.0))
    }
}

impl<T, V> IntoVar<T> for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T, V> VarLocal<T> for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T> + VarLocal<T>,
{
    #[inline]
    fn get_local(&self) -> &T {
        self.0.get_local()
    }

    #[inline]
    fn init_local<'a>(&'a mut self, vars: &'a Vars) -> &'a T {
        self.0.init_local(vars)
    }

    #[inline]
    fn update_local<'a>(&'a mut self, vars: &'a Vars) -> Option<&'a T> {
        self.0.update_local(vars)
    }
}

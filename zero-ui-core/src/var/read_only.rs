use std::marker::PhantomData;

use super::*;

/// A [`WeakVar`] wrapper that upgrades to a [`ReadOnlyVar`].
pub struct ReadOnlyWeakVar<T: VarValue, W: WeakVar<T>>(W, PhantomData<T>);
impl<T: VarValue, W: WeakVar<T>> ReadOnlyWeakVar<T, W> {
    /// New wrapper.
    pub fn new(weak: W) -> Self {
        Self(weak, PhantomData)
    }
}
impl<T: VarValue, W: WeakVar<T>> crate::private::Sealed for ReadOnlyWeakVar<T, W> {}
impl<T: VarValue, W: WeakVar<T>> Clone for ReadOnlyWeakVar<T, W> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<T: VarValue, W: WeakVar<T>> WeakVar<T> for ReadOnlyWeakVar<T, W> {
    type Strong = ReadOnlyVar<T, W::Strong>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(ReadOnlyVar::new)
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr()
    }
}

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
impl<T, V> crate::private::Sealed for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
}
impl<T, V> Var<T> for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type AsReadOnly = Self;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        self.0.get(vars)
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        self.0.get_new(vars)
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.is_new(vars)
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T {
        self.0.into_value(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        self.0.version(vars)
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    #[inline]
    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        self.0.is_animating(vars)
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
    fn is_contextual(&self) -> bool {
        self.0.is_contextual()
    }

    #[inline]
    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    #[inline]
    fn modify<Vw, M>(&self, _: &Vw, _: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(VarModify<T>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set<Vw, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set_ne<Vw, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
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
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.update_mask(vars)
    }

    type Weak = ReadOnlyWeakVar<T, V::Weak>;

    fn is_rc(&self) -> bool {
        self.0.is_rc()
    }

    fn downgrade(&self) -> Option<Self::Weak> {
        self.0.downgrade().map(ReadOnlyWeakVar::new)
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr()
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
impl<T> crate::var::rc::ReadOnlyRcVar<T>
where
    T: VarValue,
{
    /// If both [`ReadOnlyRcVar`] are wrapping the same [`RcVar`].
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

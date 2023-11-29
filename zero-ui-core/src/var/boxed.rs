use super::*;

/// A dynamic [`Var<T>`] in a box.
pub type BoxedVar<T> = Box<dyn VarBoxed<T>>;

/// Represents a weak reference to a [`Var<T>`].
pub type BoxedWeakVar<T> = Box<dyn WeakVarBoxed<T>>;

/// Represents a type erased [`Var<T>`].
pub type BoxedAnyVar = Box<dyn AnyVar>;

/// Represents a type erased weak reference to a [`Var<T>`].
pub type BoxedAnyWeakVar = Box<dyn AnyWeakVar>;

impl<T: VarValue> Clone for BoxedWeakVar<T> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

impl Clone for BoxedAnyVar {
    fn clone(&self) -> Self {
        self.clone_any()
    }
}

impl Clone for BoxedAnyWeakVar {
    fn clone(&self) -> Self {
        self.clone_any()
    }
}

#[doc(hidden)]
pub trait VarBoxed<T: VarValue>: AnyVar {
    fn clone_boxed(&self) -> BoxedVar<T>;
    fn with_boxed(&self, read: &mut dyn FnMut(&T));
    fn modify_boxed(&self, modify: Box<dyn FnOnce(&mut VarModify<T>) + Send>) -> Result<(), VarIsReadOnlyError>;
    fn actual_var_boxed(self: Box<Self>) -> BoxedVar<T>;
    fn downgrade_boxed(&self) -> BoxedWeakVar<T>;
    fn read_only_boxed(&self) -> BoxedVar<T>;
    fn boxed_any_boxed(self: Box<Self>) -> BoxedAnyVar;
}
impl<T: VarValue, V: Var<T>> VarBoxed<T> for V {
    fn clone_boxed(&self) -> BoxedVar<T> {
        self.clone().boxed()
    }

    fn with_boxed(&self, read: &mut dyn FnMut(&T)) {
        self.with(read)
    }

    fn modify_boxed(&self, modify: Box<dyn FnOnce(&mut VarModify<T>) + Send>) -> Result<(), VarIsReadOnlyError> {
        self.modify(modify)
    }

    fn actual_var_boxed(self: Box<Self>) -> BoxedVar<T> {
        (*self).actual_var().boxed()
    }

    fn downgrade_boxed(&self) -> BoxedWeakVar<T> {
        self.downgrade().boxed()
    }

    fn read_only_boxed(&self) -> BoxedVar<T> {
        self.read_only().boxed()
    }

    fn boxed_any_boxed(self: Box<Self>) -> BoxedAnyVar {
        self
    }
}

#[doc(hidden)]
pub trait WeakVarBoxed<T: VarValue>: AnyWeakVar {
    fn clone_boxed(&self) -> BoxedWeakVar<T>;
    fn upgrade_boxed(&self) -> Option<BoxedVar<T>>;
}
impl<T: VarValue, W: WeakVar<T>> WeakVarBoxed<T> for W {
    fn clone_boxed(&self) -> BoxedWeakVar<T> {
        self.clone().boxed()
    }

    fn upgrade_boxed(&self) -> Option<BoxedVar<T>> {
        self.upgrade().map(Var::boxed)
    }
}

impl<T: VarValue> crate::private::Sealed for BoxedWeakVar<T> {}

impl<T: VarValue> AnyWeakVar for BoxedWeakVar<T> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        (**self).clone_any()
    }

    fn strong_count(&self) -> usize {
        (**self).strong_count()
    }

    fn weak_count(&self) -> usize {
        (**self).weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        (**self).upgrade_any()
    }

    fn as_any(&self) -> &dyn Any {
        (**self).as_any()
    }
}
impl<T: VarValue> WeakVar<T> for BoxedWeakVar<T> {
    type Upgrade = BoxedVar<T>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        (**self).upgrade_boxed()
    }
}

impl<T: VarValue> crate::private::Sealed for BoxedVar<T> {}

impl<T: VarValue> Clone for BoxedVar<T> {
    fn clone(&self) -> Self {
        (**self).clone_boxed()
    }
}

impl<T: VarValue> AnyVar for BoxedVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        (**self).clone_any()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn var_type_id(&self) -> TypeId {
        (**self).var_type_id()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        (**self).get_any()
    }

    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        (**self).set_any(value)
    }

    fn last_update(&self) -> VarUpdateId {
        (**self).last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        (**self).capabilities()
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&VarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        (**self).hook(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        (**self).hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        (**self).strong_count()
    }

    fn weak_count(&self) -> usize {
        (**self).weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        (**self).actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        (**self).downgrade_any()
    }

    fn is_animating(&self) -> bool {
        (**self).is_animating()
    }

    fn modify_importance(&self) -> usize {
        (**self).modify_importance()
    }

    fn var_ptr(&self) -> VarPtr {
        (**self).var_ptr()
    }

    fn get_debug(&self) -> crate::text::Txt {
        (**self).get_debug()
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        (**self).update()
    }

    fn map_debug(&self) -> types::ContextualizedVar<crate::text::Txt, ReadOnlyArcVar<crate::text::Txt>> {
        (**self).map_debug()
    }
}

impl<T: VarValue> IntoVar<T> for BoxedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for BoxedVar<T> {
    type ReadOnly = BoxedVar<T>;

    type ActualVar = BoxedVar<T>;

    type Downgrade = BoxedWeakVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        #[cfg(dyn_closure)]
        let read: Box<dyn FnOnce(&T) -> R> = Box::new(read);
        boxed_var_with(self, read)
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        let modify = Box::new(modify);
        (**self).modify_boxed(modify)
    }

    fn boxed(self) -> BoxedVar<T> {
        self
    }

    fn boxed_any(self) -> BoxedAnyVar
    where
        Self: Sized,
    {
        // fix after https://github.com/rust-lang/rust/issues/65991
        self.clone_any()
    }

    fn actual_var(self) -> BoxedVar<T> {
        self.actual_var_boxed()
    }

    fn downgrade(&self) -> BoxedWeakVar<T> {
        (**self).downgrade_boxed()
    }

    fn into_value(self) -> T {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        if self.capabilities().is_always_read_only() {
            self.clone()
        } else {
            (**self).read_only_boxed()
        }
    }
}

fn boxed_var_with<T: VarValue, R, F>(var: &BoxedVar<T>, read: F) -> R
where
    F: FnOnce(&T) -> R,
{
    let mut read = Some(read);
    let mut result = None;
    (**var).with_boxed(&mut |var_value| {
        let read = read.take().unwrap();
        result = Some(read(var_value));
    });
    result.take().unwrap()
}

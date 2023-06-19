use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use parking_lot::{RwLock, RwLockReadGuard};

use super::{types::WeakContextInitHandle, *};

/// Represents a variable that delays initialization until the first usage.
///
/// Usage that initializes the variable are all [`AnyVar`] and [`Var<T>`] methods except `read_only`, `downgrade` and `boxed`.
/// The variable re-initializes when the [`ContextInitHandle::current`] is different on usage.
///
/// This variable is used in the [`Var::map`] and other mapping methods to support mapping from [`ContextVar<T>`].
///
/// ```
/// # macro_rules! fake{($($tt:tt)*) => {}}
/// # fake! {
/// let wgt = MyWgt! {
///     my_property = MY_CTX_VAR.map(|&b| !b);
/// };
/// # }
/// ```
///
/// In the example above the mapping var will bind with the `MY_CTX_VAR` context inside the property node, not
/// the context at the moment the widget is instantiated.
pub struct ContextualizedVar<T, S> {
    _type: PhantomData<T>,
    init: Arc<dyn Fn() -> S + Send + Sync>,
    actual: RwLock<Vec<(WeakContextInitHandle, S)>>,
}
impl<T: VarValue, S: Var<T>> ContextualizedVar<T, S> {
    /// New with initialization function.
    ///
    /// The `init` closure will be called on the first usage of the var, once after the var is cloned and any time
    /// a parent contextualized var is initializing.
    pub fn new(init: Arc<dyn Fn() -> S + Send + Sync>) -> Self {
        Self {
            _type: PhantomData,
            init,
            actual: RwLock::new(Vec::with_capacity(1)),
        }
    }

    /// Borrow/initialize the actual var.
    pub fn borrow_init(&self) -> parking_lot::MappedRwLockReadGuard<S> {
        let current_ctx = ContextInitHandle::current().downgrade();

        let act = self.actual.read_recursive();
        if let Some(i) = act.iter().position(|(h, _)| h == &current_ctx) {
            return RwLockReadGuard::map(act, move |m| &m[i].1);
        }
        drop(act);

        let mut act = self.actual.write();
        act.retain(|(h, _)| h.is_alive());
        let i = act.len();
        act.push((current_ctx, (self.init)()));
        drop(act);

        let act = self.actual.read_recursive();
        RwLockReadGuard::map(act, move |m| &m[i].1)
    }

    /// Unwraps the initialized actual var or initializes it now.
    pub fn into_init(self) -> S {
        let mut act = self.actual.into_inner();
        let current_ctx = ContextInitHandle::current().downgrade();

        if let Some(i) = act.iter().position(|(h, _)| h == &current_ctx) {
            act.swap_remove(i).1
        } else {
            (self.init)()
        }
    }
}

/// Weak var that upgrades to an uninitialized [`ContextualizedVar<T, S>`].
pub struct WeakContextualizedVar<T, S> {
    _type: PhantomData<T>,
    init: Weak<dyn Fn() -> S + Send + Sync>,
}
impl<T: VarValue, S: Var<T>> WeakContextualizedVar<T, S> {
    /// New with weak init function.
    pub fn new(init: Weak<dyn Fn() -> S + Send + Sync>) -> Self {
        Self { _type: PhantomData, init }
    }
}

impl<T: VarValue, S: Var<T>> Clone for ContextualizedVar<T, S> {
    fn clone(&self) -> Self {
        let current_ctx_id = ContextInitHandle::current().downgrade();
        let act = self.actual.read_recursive();
        if let Some(i) = act.iter().position(|(id, _)| *id == current_ctx_id) {
            return Self {
                _type: PhantomData,
                init: self.init.clone(),
                actual: RwLock::new(vec![act[i].clone()]),
            };
        }
        Self::new(self.init.clone())
    }
}
impl<T: VarValue, S: Var<T>> Clone for WeakContextualizedVar<T, S> {
    fn clone(&self) -> Self {
        Self {
            _type: PhantomData,
            init: self.init.clone(),
        }
    }
}

impl<T: VarValue, S: Var<T>> crate::private::Sealed for ContextualizedVar<T, S> {}
impl<T: VarValue, S: Var<T>> crate::private::Sealed for WeakContextualizedVar<T, S> {}

impl<T: VarValue, S: Var<T>> AnyVar for ContextualizedVar<T, S> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<T> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.borrow_init().last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        self.borrow_init().capabilities()
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&VarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        self.borrow_init().hook(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.borrow_init().hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.init)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.init)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.borrow_init().actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }

    fn is_animating(&self) -> bool {
        self.borrow_init().is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.borrow_init().modify_importance()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_arc(&self.init)
    }

    fn get_debug(&self) -> crate::text::Txt {
        self.with(var_debug)
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        Var::modify(self, var_update)
    }

    fn map_debug(&self) -> types::ContextualizedVar<crate::text::Txt, ReadOnlyArcVar<crate::text::Txt>> {
        Var::map(self, var_debug)
    }
}
impl<T: VarValue, S: Var<T>> AnyWeakVar for WeakContextualizedVar<T, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.init.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.init.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.upgrade().map(|c| Box::new(c) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue, S: Var<T>> IntoVar<T> for ContextualizedVar<T, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue, S: Var<T>> Var<T> for ContextualizedVar<T, S> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = S::ActualVar;

    type Downgrade = WeakContextualizedVar<T, S>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.borrow_init().with(read)
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        self.borrow_init().modify(modify)
    }

    fn actual_var(self) -> Self::ActualVar {
        self.into_init().actual_var()
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakContextualizedVar::new(Arc::downgrade(&self.init))
    }

    fn into_value(self) -> T {
        self.into_init().into_value()
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}
impl<T: VarValue, S: Var<T>> WeakVar<T> for WeakContextualizedVar<T, S> {
    type Upgrade = ContextualizedVar<T, S>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.init.upgrade().map(ContextualizedVar::new)
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;

    use super::*;

    #[test]
    fn nested_contextualized_vars() {
        let mut app = App::default().run_headless(false);

        let source = var(0u32);
        let mapped = source.map(|n| n + 1);
        let mapped2 = mapped.map(|n| n - 1); // double contextual here.
        let mapped2_copy = mapped2.clone();

        // init, same effect as subscribe in widgets, the last to init breaks the other.
        assert_eq!(0, mapped2.get());
        assert_eq!(0, mapped2_copy.get());

        source.set(10u32);
        let mut updated = false;
        app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some(10), mapped2.get_new());
                assert_eq!(Some(10), mapped2_copy.get_new());
            },
            false,
        )
        .assert_wait();

        assert!(updated);
    }

    #[test]
    fn nested_contextualized_vars_diff_contexts() {
        let mut app = App::default().run_headless(false);

        let source = var(0u32);
        let mapped = source.map(|n| n + 1);
        let mapped2 = mapped.map(|n| n - 1); // double contextual here.
        let mapped2_copy = mapped2.clone();

        // init, same effect as subscribe in widgets, the last to init breaks the other.
        assert_eq!(0, mapped2.get());
        let other_ctx = ContextInitHandle::new();
        other_ctx.with_context(|| {
            assert_eq!(0, mapped2_copy.get());
        });

        source.set(10u32);
        let mut updated = false;
        app.update_observe(
            || {
                assert!(!updated, "expected one update");
                updated = true;
                assert_eq!(Some(10), mapped2.get_new());
                other_ctx.with_context(|| {
                    assert_eq!(Some(10), mapped2_copy.get_new());
                });
            },
            false,
        )
        .assert_wait();

        assert!(updated);
    }
}

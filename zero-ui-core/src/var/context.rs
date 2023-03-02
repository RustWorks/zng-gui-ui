use crate::context::ContextLocal;

use super::*;

///<span data-del-macro-root></span> Declares new [`ContextVar`] keys.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::var::context_var;
/// # #[derive(Debug, Clone)]
/// # struct NotConst(u8);
/// # fn init_val() -> NotConst { NotConst(10) }
/// #
/// context_var! {
///     /// A public documented context var.
///     pub static FOO_VAR: u8 = 10;
///
///     // A private context var.
///     static BAR_VAR: NotConst = init_val();
///
///     // A var that *inherits* from another.
///     pub static DERIVED_VAR: u8 = FOO_VAR;
/// }
/// ```
///
/// # Default Value
///
/// All context variable have a default fallback value that is used when the variable is not setted in the context.
///
/// The default value is instantiated once per app thread and is the value of the variable when it is not set in the context,
/// any value [`IntoVar<T>`] is allowed, meaning other variables are supported, you can use this to *inherit* from another
/// context variable, when the context fallback to default the other context var is used, it can have a value or fallback to
/// it's default too.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `_VAR` suffix.
///
/// # Context Only
///
/// Note that if you are only interested in sharing a contextual value you can use the [`context_local!`] macro instead.
///
/// [`context_local!`]: crate::context::context_local
#[macro_export]
macro_rules! context_var {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $NAME:ident: $Type:ty = $default:expr;
    )+) => {$(
        $(#[$attr])*
        $vis static $NAME: $crate::var::ContextVar<$Type> = {
            $crate::context::context_local! {
                static VAR: $crate::var::BoxedVar<$Type> = $crate::var::types::context_var_init::<$Type>($default);
            }
            $crate::var::ContextVar::new(&VAR)
        };
    )+}
}
#[doc(inline)]
pub use crate::context_var;

#[doc(hidden)]
pub fn context_var_init<T: VarValue>(init: impl IntoVar<T>) -> BoxedVar<T> {
    init.into_var().boxed()
}

/// Represents another variable in a context.
///
/// Context variables are [`Var<T>`] implementers that represent a contextual value, unlike other variables it does not own
/// the value it represents.
///
/// See [`context_var!`] for more details.
#[derive(Clone)]
pub struct ContextVar<T: VarValue>(&'static ContextLocal<BoxedVar<T>>);
impl<T: VarValue> ContextVar<T> {
    #[doc(hidden)]
    pub const fn new(var: &'static ContextLocal<BoxedVar<T>>) -> Self {
        Self(var)
    }

    /// Runs `action` with this context var representing the other `var` in the current thread.
    ///
    /// Returns the actual var that was used and the result of `action`.
    ///
    /// Note that the `var` must be the same for subsequent calls in the same *context*, otherwise [contextualized]
    /// variables may not update their binding, in widgets you must re-init the descendants if you replace the `var`.
    ///
    /// [contextualized]: types::ContextualizedVar
    pub fn with_context<R>(self, id: ContextInitHandle, var: impl IntoVar<T>, action: impl FnOnce() -> R) -> (BoxedVar<T>, R) {
        let mut var = Some(var.into_var().actual_var().boxed());
        let r = self.0.with_context(&mut var, move || id.with_context(action));
        (var.unwrap(), r)
    }
}
impl<T: VarValue> Copy for ContextVar<T> {}

impl<T: VarValue> crate::private::Sealed for ContextVar<T> {}

impl<T: VarValue> AnyVar for ContextVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(*self)
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
        self.0.read().last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        self.0.read().capabilities() | VarCapabilities::CAPS_CHANGE
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&dyn AnyVarValue) -> bool + Send + Sync>) -> VarHandle {
        self.0.read().hook(pos_modify_action)
    }

    fn subscribe(&self, widget_id: WidgetId) -> VarHandle {
        self.0.read().subscribe(widget_id)
    }

    fn strong_count(&self) -> usize {
        self.0.read().strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.read().weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.0.read().actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        self.0.read().downgrade_any()
    }

    fn is_animating(&self) -> bool {
        self.0.read().is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.0.read().modify_importance()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_ctx_local(self.0)
    }

    fn get_debug(&self) -> crate::text::Text {
        self.with(var_debug)
    }

    fn touch(&self) -> Result<(), VarIsReadOnlyError> {
        Var::modify(self, var_touch)
    }

    fn map_debug(&self) -> types::ContextualizedVar<crate::text::Text, ReadOnlyArcVar<crate::text::Text>> {
        Var::map(self, var_debug)
    }
}

impl<T: VarValue> IntoVar<T> for ContextVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for ContextVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = BoxedVar<T>;

    type Downgrade = BoxedWeakVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.read().with(read)
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut Cow<T>) + Send + 'static,
    {
        self.0.read().modify(modify)
    }

    fn actual_var(self) -> BoxedVar<T> {
        self.0.read().clone().actual_var()
    }

    fn downgrade(&self) -> BoxedWeakVar<T> {
        self.0.read().downgrade()
    }

    fn into_value(self) -> T {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(*self)
    }
}

/// Context var that is always read-only, even if it is representing a read-write var.
pub type ReadOnlyContextVar<T> = types::ReadOnlyVar<T, ContextVar<T>>;

/// Identifies the unique context a [`ContextualizedVar`] is in.
///
/// Each node that sets context-vars have an unique ID, it is different after each (re)init. The [`ContextualizedVar`]
/// records this ID, and rebuilds when it has changed. The contextualized inner vars are retained when the ID has at least one
/// clone.
///
/// [`ContextualizedVar`]: crate::var::types::ContextualizedVar
#[derive(Clone, Default)]
pub struct ContextInitHandle(Arc<()>);
crate::context_local! {
    static CONTEXT_INIT_ID: ContextInitHandle = ContextInitHandle::new();
}
impl ContextInitHandle {
    /// Generates a new unique handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the current context handle.
    pub fn current() -> Self {
        CONTEXT_INIT_ID.get()
    }

    /// Runs `action` with `self` as the current context ID.
    ///
    /// Note that [`ContextVar::with_context`] already calls this method.
    pub fn with_context<R>(&self, action: impl FnOnce() -> R) -> R {
        CONTEXT_INIT_ID.with_context(&mut Some(self.clone()), action)
    }

    /// Create a weak handle that can be used to monitor `self`, but does not hold it.
    pub fn downgrade(&self) -> WeakContextInitHandle {
        WeakContextInitHandle(Arc::downgrade(&self.0))
    }
}
impl fmt::Debug for ContextInitHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ContextInitHandle").field(&Arc::as_ptr(&self.0)).finish()
    }
}
impl PartialEq for ContextInitHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ContextInitHandle {}
impl std::hash::Hash for ContextInitHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let i = Arc::as_ptr(&self.0) as usize;
        std::hash::Hash::hash(&i, state)
    }
}

/// Weak [`ContextInitHandle`].
#[derive(Clone, Default)]
pub struct WeakContextInitHandle(std::sync::Weak<()>);
impl WeakContextInitHandle {
    /// Returns `true` if the strong handle still exists.
    pub fn is_alive(&self) -> bool {
        self.0.strong_count() > 0
    }
}
impl fmt::Debug for WeakContextInitHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakContextInitHandle")
            .field(&std::sync::Weak::as_ptr(&self.0))
            .finish()
    }
}
impl PartialEq for WeakContextInitHandle {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Weak::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WeakContextInitHandle {}
impl std::hash::Hash for WeakContextInitHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let i = std::sync::Weak::as_ptr(&self.0) as usize;
        std::hash::Hash::hash(&i, state)
    }
}
pub use helpers::*;
mod helpers {
    use std::cell::RefCell;

    use crate::{context::*, event::*, render::*, var::*, widget_info::*, widget_instance::*, *};

    /// Helper for declaring properties that sets a context var.
    ///
    /// The method presents the `value` as the [`ContextVar<T>`] in the widget and widget descendants.
    /// The context var [`is_new`] and [`read_only`] status are always equal to the `value` var status. Users
    /// of the context var can also retrieve the `value` var using [`actual_var`].
    ///
    /// The generated [`UiNode`] delegates each method to `child` inside a call to [`ContextVar::with_context`].
    ///
    /// # Examples
    ///
    /// A simple context property declaration:
    ///
    /// ```
    /// # fn main() -> () { }
    /// # use zero_ui_core::{*, widget_instance::*, var::*};
    /// context_var! {
    ///     pub static FOO_VAR: u32 = 0u32;
    /// }
    ///
    /// /// Sets the [`FooVar`] in the widgets and its content.
    /// #[property(CONTEXT, default(FOO_VAR))]
    /// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
    ///     with_context_var(child, FOO_VAR, value)
    /// }
    /// ```
    ///
    /// When set in a widget, the `value` is accessible in all inner nodes of the widget, using `FOO_VAR.get`, and if `value` is set to a
    /// variable the `FOO_VAR` will also reflect its [`is_new`] and [`read_only`]. If the `value` var is not read-only inner nodes
    /// can modify it using `FOO_VAR.set` or `FOO_VAR.modify`.
    ///
    /// Also note that the property [`default`] is set to the same `FOO_VAR`, this causes the property to *pass-through* the outer context
    /// value, as if it was not set.
    ///
    /// **Tip:** You can use a [`merge_var!`] to merge a new value to the previous context value:
    ///
    /// ```
    /// # fn main() -> () { }
    /// # use zero_ui_core::{*, widget_instance::*, var::*};
    ///
    /// #[derive(Debug, Clone, Default)]
    /// pub struct Config {
    ///     pub foo: bool,
    ///     pub bar: bool,
    /// }
    ///
    /// context_var! {
    ///     pub static CONFIG_VAR: Config = Config::default();
    /// }
    ///
    /// /// Sets the *foo* config.
    /// #[property(CONTEXT, default(false))]
    /// pub fn foo(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    ///     with_context_var(child, CONFIG_VAR, merge_var!(CONFIG_VAR, value.into_var(), |c, &v| {
    ///         let mut c = c.clone();
    ///         c.foo = v;
    ///         c
    ///     }))
    /// }
    ///
    /// /// Sets the *bar* config.
    /// #[property(CONTEXT, default(false))]
    /// pub fn bar(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode {
    ///     with_context_var(child, CONFIG_VAR, merge_var!(CONFIG_VAR, value.into_var(), |c, &v| {
    ///         let mut c = c.clone();
    ///         c.bar = v;
    ///         c
    ///     }))
    /// }
    /// ```
    ///
    /// When set in a widget, the [`merge_var!`] will read the context value of the parent properties, modify a clone of the value and
    /// the result will be accessible to the inner properties, the widget user can then set with the composed value in steps and
    /// the final consumer of the composed value only need to monitor to a single context variable.
    ///
    /// [`is_new`]: AnyVar::is_new
    /// [`read_only`]: Var::read_only
    /// [`actual_var`]: Var::actual_var
    /// [`default`]: crate::property#default
    pub fn with_context_var<T: VarValue>(child: impl UiNode, context_var: ContextVar<T>, value: impl IntoVar<T>) -> impl UiNode {
        #[ui_node(struct WithContextVarNode<T: VarValue> {
            child: impl UiNode,
            context_var: ContextVar<T>,
            id: Option<ContextInitHandle>,
            value: RefCell<Option<BoxedVar<T>>>,
        })]
        impl WithContextVarNode {
            fn with<R>(&self, mtd: impl FnOnce(&T_child) -> R) -> R {
                let mut value = self.value.borrow_mut();
                let var = value.take().unwrap();
                let (var, r) = self
                    .context_var
                    .with_context(self.id.clone().expect("node not inited"), var, move || mtd(&self.child));
                *value = Some(var);
                r
            }

            fn with_mut<R>(&mut self, mtd: impl FnOnce(&mut T_child) -> R) -> R {
                let var = self.value.get_mut().take().unwrap();
                let (var, r) = self
                    .context_var
                    .with_context(self.id.get_or_insert_with(ContextInitHandle::new).clone(), var, || {
                        mtd(&mut self.child)
                    });
                *self.value.get_mut() = Some(var);
                r
            }

            #[UiNode]
            fn init(&mut self) {
                self.with_mut(|c| c.init())
            }

            #[UiNode]
            fn deinit(&mut self) {
                self.with_mut(|c| c.deinit());
                self.id = None;
            }

            #[UiNode]
            fn info(&self, info: &mut WidgetInfoBuilder) {
                self.with(|c| c.info(info))
            }

            #[UiNode]
            fn event(&mut self, update: &mut crate::event::EventUpdate) {
                self.with_mut(|c| c.event(update))
            }

            #[UiNode]
            fn update(&mut self, updates: &mut WidgetUpdates) {
                self.with_mut(|c| c.update(updates))
            }

            #[UiNode]
            fn measure(&self, wm: &mut WidgetMeasure) -> units::PxSize {
                self.with(|c| c.measure(wm))
            }

            #[UiNode]
            fn layout(&mut self, wl: &mut WidgetLayout) -> units::PxSize {
                self.with_mut(|c| c.layout(wl))
            }

            #[UiNode]
            fn render(&self, frame: &mut FrameBuilder) {
                self.with(|c| c.render(frame))
            }

            #[UiNode]
            fn render_update(&self, update: &mut FrameUpdate) {
                self.with(|c| c.render_update(update))
            }
        }
        WithContextVarNode {
            child: child.cfg_boxed(),
            context_var,
            id: None,
            value: RefCell::new(Some(value.into_var().boxed())),
        }
    }

    /// Helper for declaring properties that sets a context var to a value generated on init.
    ///
    /// The method calls the `init_value` closure on init to produce a *value* var that is presented as the [`ContextVar<T>`]
    /// in the widget and widget descendants. The closure can be called more than once if the returned node is reinited.
    ///
    /// Apart from the value initialization this behaves just like [`with_context_var`].
    pub fn with_context_var_init<T: VarValue>(
        child: impl UiNode,
        var: ContextVar<T>,
        init_value: impl FnMut() -> BoxedVar<T> + Send + 'static,
    ) -> impl UiNode {
        #[ui_node(struct WithContextVarInitNode<T: VarValue> {
            child: impl UiNode,
            context_var: ContextVar<T>,
            id: Option<ContextInitHandle>,
            init_value: impl FnMut() -> BoxedVar<T> + Send + 'static,
            value: RefCell<Option<BoxedVar<T>>>,
        })]
        impl WithContextVarInitNode {
            fn with<R>(&self, mtd: impl FnOnce(&T_child) -> R) -> R {
                let mut value = self.value.borrow_mut();
                let var = value.take().unwrap();
                let (var, r) = self
                    .context_var
                    .with_context(self.id.clone().expect("node not inited"), var, move || mtd(&self.child));
                *value = Some(var);
                r
            }

            fn with_mut<R>(&mut self, mtd: impl FnOnce(&mut T_child) -> R) -> R {
                let var = self.value.get_mut().take().unwrap();
                let (var, r) = self
                    .context_var
                    .with_context(self.id.get_or_insert_with(ContextInitHandle::new).clone(), var, || {
                        mtd(&mut self.child)
                    });
                *self.value.get_mut() = Some(var);
                r
            }

            #[UiNode]
            fn info(&self, info: &mut WidgetInfoBuilder) {
                self.with(|c| c.info(info));
            }

            #[UiNode]
            fn init(&mut self) {
                *self.value.get_mut() = Some((self.init_value)());
                self.with_mut(|c| c.init());
            }

            #[UiNode]
            fn deinit(&mut self) {
                self.with_mut(|c| c.deinit());
                *self.value.get_mut() = None;
            }

            #[UiNode]
            fn update(&mut self, updates: &mut WidgetUpdates) {
                self.with_mut(|c| c.update(updates));
            }

            #[UiNode]
            fn event(&mut self, update: &mut EventUpdate) {
                self.with_mut(|c| c.event(update));
            }

            #[UiNode]
            fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
                self.with(|c| c.measure(wm))
            }

            #[UiNode]
            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                self.with_mut(|c| c.layout(wl))
            }

            #[UiNode]
            fn render(&self, frame: &mut FrameBuilder) {
                self.with(|c| c.render(frame));
            }

            #[UiNode]
            fn render_update(&self, update: &mut FrameUpdate) {
                self.with(|c| c.render_update(update));
            }
        }
        WithContextVarInitNode {
            child: child.cfg_boxed(),
            context_var: var,
            id: None,
            init_value,
            value: RefCell::new(None),
        }
        .cfg_boxed()
    }

    /// Wraps `child` in a node that provides a unique [`ContextInitHandle`], refreshed every (re)init.
    ///
    /// Note that [`with_context_var`] and [`with_context_var_init`] already provide an unique ID.
    pub fn with_new_context_init_id(child: impl UiNode) -> impl UiNode {
        #[ui_node(struct WithNewContextInitHandleNode {
            child: impl UiNode,
            id: Option<ContextInitHandle>,
        })]
        impl WithNewContextInitHandleNode {
            fn with<R>(&self, mtd: impl FnOnce(&T_child) -> R) -> R {
                self.id.as_ref().expect("node not inited").with_context(|| mtd(&self.child))
            }

            fn with_mut<R>(&mut self, mtd: impl FnOnce(&mut T_child) -> R) -> R {
                self.id
                    .get_or_insert_with(ContextInitHandle::new)
                    .with_context(|| mtd(&mut self.child))
            }

            #[UiNode]
            fn info(&self, info: &mut WidgetInfoBuilder) {
                self.with(|c| c.info(info));
            }

            #[UiNode]
            fn init(&mut self) {
                self.with_mut(|c| c.init());
            }

            #[UiNode]
            fn deinit(&mut self) {
                self.with_mut(|c| c.deinit());
                self.id = None;
            }

            #[UiNode]
            fn update(&mut self, updates: &mut WidgetUpdates) {
                self.with_mut(|c| c.update(updates));
            }

            #[UiNode]
            fn event(&mut self, update: &mut EventUpdate) {
                self.with_mut(|c| c.event(update));
            }

            #[UiNode]
            fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
                self.with(|c| c.measure(wm))
            }

            #[UiNode]
            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                self.with_mut(|c| c.layout(wl))
            }

            #[UiNode]
            fn render(&self, frame: &mut FrameBuilder) {
                self.with(|c| c.render(frame));
            }

            #[UiNode]
            fn render_update(&self, update: &mut FrameUpdate) {
                self.with(|c| c.render_update(update));
            }
        }
        WithNewContextInitHandleNode {
            child: child.cfg_boxed(),
            id: None,
        }
        .cfg_boxed()
    }
}

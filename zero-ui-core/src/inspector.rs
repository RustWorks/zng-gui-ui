#![cfg(any(doc, inspector))]
#![cfg_attr(doc_nightly, doc(cfg(any(debug_assertions, inspector))))]

//! Helper types for debugging an UI tree.

use linear_map::LinearMap;

use crate::{
    context::LayoutContext,
    context::{state_key, InfoContext, WidgetContext},
    impl_ui_node,
    render::{FrameBuilder, FrameUpdate},
    units::*,
    var::{context_var, BoxedVar, ContextVarData, Var, VarVersion},
    widget_base::Visibility,
    widget_info::{WidgetInfo, WidgetInfoTree, WidgetLayout, WidgetSubscriptions},
    UiNode,
};
use crate::{
    context::RenderContext,
    crate_util::IdMap,
    event::EventUpdateArgs,
    formatx,
    text::{Text, ToText},
    widget_info::WidgetInfoBuilder,
    BoxedUiNode,
};

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
    time::{Duration, Instant},
};

/// A location in source-code.
///
/// Use [`source_location!`] to construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// [`file!`]
    pub file: &'static str,
    /// [`line!`]
    pub line: u32,
    /// [`column!`]
    pub column: u32,
}
impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

///<span data-del-macro-root></span> New [`SourceLocation`] that represents the location you call this macro.
#[macro_export]
macro_rules! source_location {
    () => {
        $crate::inspector::SourceLocation {
            file: std::file!(),
            line: std::line!(),
            column: std::column!(),
        }
    };
}
#[doc(inline)]
pub use crate::source_location;

/// Debug information about a property of a widget instance.
#[derive(Debug, Clone)]
pub struct PropertyInstanceInfo {
    /// Property priority in a widget.
    ///
    /// See [the property doc](crate::property#priority) for more details.
    pub priority: PropertyPriority,

    /// Original name of the property.
    pub original_name: &'static str,
    /// Source-code location of the property declaration.
    pub decl_location: SourceLocation,

    /// Name of the property in the widget.
    pub property_name: &'static str,
    /// Source-code location of the widget instantiation or property assign.
    pub instance_location: SourceLocation,

    /// Property arguments, sorted by their index in the property.
    pub args: Box<[PropertyArgInfo]>,

    /// If [`args`](Self::args) values can be inspected.
    ///
    /// Only properties that are `allowed_in_when` are guaranteed to have
    /// variable arguments with values that can print debug. For other properties
    /// the [`value`](PropertyArgInfo::value) is always an empty string and
    /// [`value_version`](PropertyArgInfo::value_version) is always zero.
    pub can_debug_args: bool,

    /// If the user assigned this property.
    pub user_assigned: bool,

    /// Time elapsed in the last call of each property `UiNode` methods.
    pub duration: UiNodeDurations,
    /// Count of calls of each property `UiNode` methods.
    pub count: UiNodeCounts,
}
impl PropertyInstanceInfo {
    /// If `init` and `deinit` count are the same.
    pub fn is_deinited(&self) -> bool {
        self.count.init == self.count.deinit
    }
}

/// A reference to a [`PropertyInstanceInfo`].
pub type PropertyInstance = Rc<RefCell<PropertyInstanceInfo>>;

/// A reference to a [`WidgetInstanceInfo`].
pub type WidgetInstance = Rc<RefCell<WidgetInstanceInfo>>;

/// Debug information about a property argument.
#[derive(Debug, Clone)]
pub struct PropertyArgInfo {
    /// Name of the argument.
    pub name: &'static str,
    /// Value printed in various formats.
    pub value: ValueInfo,
    /// Value version from the source variable.
    pub value_version: VarVersion,
    /// If the arg is a [`can_update` var](crate::var::Var::can_update).
    pub can_update: bool,
}

/// Property priority in a widget.
///
/// See [the property doc](crate::property#priority) for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyPriority {
    /// [Context](crate::property#context) property.
    Context,
    /// [Event](crate::property#event) property.
    Event,
    /// [Layout](crate::property#layout) property.
    Layout,
    /// [Size](crate::property#size) property.
    Size,
    /// [Border](crate::property#border) property.
    Border,
    /// [Fill](crate::property#fill) property.
    Fill,
    /// [Child Context](crate::property#child-context) property.
    ChildContext,
    /// [Child Layout](crate::property#child-layout) property.
    ChildLayout,
    /// [Capture-only](crate::property#capture_only) property.
    CaptureOnly,
}

impl PropertyPriority {
    fn token_str(self) -> &'static str {
        match self {
            PropertyPriority::Context => "context",
            PropertyPriority::Event => "event",
            PropertyPriority::Layout => "layout",
            PropertyPriority::Size => "size",
            PropertyPriority::Border => "border",
            PropertyPriority::Fill => "fill",
            PropertyPriority::ChildContext => "child_context",
            PropertyPriority::ChildLayout => "child_layout",
            PropertyPriority::CaptureOnly => "capture_only",
        }
    }
}

/// Time duration of a [`UiNode`] method in a property branch.
///
/// The durations is the sum of all descendent nodes.
#[derive(Debug, Clone, Default)]
pub struct UiNodeDurations {
    /// Duration of [`UiNode::info`] call.
    pub info: Duration,
    /// Duration of [`UiNode::subscriptions`] call.
    pub subscriptions: Duration,
    /// Duration of [`UiNode::init`] call.
    pub init: Duration,
    /// Duration of [`UiNode::deinit`] call.
    pub deinit: Duration,
    /// Duration of [`UiNode::update`] call.
    pub update: Duration,
    /// Duration of [`UiNode::measure`] call.
    pub measure: Duration,
    /// Duration of [`UiNode::arrange`] call.
    pub arrange: Duration,
    /// Duration of [`UiNode::render`] call.
    pub render: Duration,
    /// Duration of [`UiNode::render_update`] call.
    pub render_update: Duration,
}

/// Number of times a [`UiNode`] method was called in a property branch.
///
/// The counts is only the property node call, not a sum of descendant nodes.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct UiNodeCounts {
    /// Count of calls to [`UiNode::info`].
    pub info: usize,
    /// Count of calls to [`UiNode::subscriptions`].
    pub subscriptions: usize,
    /// Count of calls to [`UiNode::init`].
    pub init: usize,
    /// Count of calls to [`UiNode::deinit`].
    pub deinit: usize,
    /// Count of calls to [`UiNode::update`].
    pub update: usize,
    /// Count of calls to [`UiNode::measure`].
    pub measure: usize,
    /// Count of calls to [`UiNode::arrange`].
    pub arrange: usize,
    /// Count of calls to [`UiNode::render`].
    pub render: usize,
    /// Count of calls to [`UiNode::render_update`].
    pub render_update: usize,
}

/// Represents the widget *new* functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WidgetNewFn {
    /// `new_child`
    NewChild,

    /// `new_child_layout`
    NewChildLayout,
    /// `new_child_context`
    NewChildContext,

    /// `new_fill`
    NewFill,
    /// `new_border`
    NewBorder,
    /// `new_size`
    NewSize,
    /// `new_layout`
    NewLayout,
    /// `new_event`
    NewEvent,
    /// `new_context`
    NewContext,

    /// `new`
    New,
}
impl WidgetNewFn {
    /// All the new functions from `NewChild` to `New`.
    pub fn all() -> &'static [WidgetNewFn] {
        &[
            Self::NewChild,
            Self::NewChildLayout,
            Self::NewChildContext,
            Self::NewFill,
            Self::NewBorder,
            Self::NewSize,
            Self::NewLayout,
            Self::NewEvent,
            Self::NewContext,
            Self::New,
        ]
    }
}
impl fmt::Display for WidgetNewFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NewChild => write!(f, "new_child"),
            Self::NewChildLayout => write!(f, "new_child_layout"),
            Self::NewChildContext => write!(f, "new_child_context"),
            Self::NewFill => write!(f, "new_fill"),
            Self::NewBorder => write!(f, "new_border"),
            Self::NewSize => write!(f, "new_size"),
            Self::NewLayout => write!(f, "new_layout"),
            Self::NewEvent => write!(f, "new_event"),
            Self::NewContext => write!(f, "new_context"),
            Self::New => write!(f, "new"),
        }
    }
}
#[doc(hidden)]
pub type WidgetNewFnV1 = WidgetNewFn;

/// Debug info about a widget instance.
#[derive(Debug, Clone)]
pub struct WidgetInstanceInfo {
    /// Unique ID of the widget instantiation.
    pub instance_id: WidgetInstanceId,

    /// Widget type name.
    pub widget_name: &'static str,

    /// Source-code location of the widget declaration.
    pub decl_location: SourceLocation,

    /// Source-code location of the widget instantiation.
    pub instance_location: SourceLocation,

    /// Properties this widget captured in the new functions.
    pub captures: LinearMap<WidgetNewFn, Box<[CapturedPropertyInfo]>>,

    /// When blocks setup by this widget instance.
    pub whens: Box<[WhenInfo]>,

    /// Name of the parent widget property that introduces this widget.
    ///
    /// Empty string (`""`) when the widget has no parent with debug enabled.
    pub parent_property: &'static str,
}

/// Debug info about a *property* captured by a widget instance.
#[derive(Debug, Clone)]
pub struct CapturedPropertyInfo {
    /// Name of the property in the widget.
    pub property_name: &'static str,

    /// Source-code location of the widget instantiation or property assign.
    pub instance_location: SourceLocation,

    /// Property arguments, sorted by their index in the property.
    pub args: Box<[PropertyArgInfo]>,

    /// If [`args`](Self::args) values can be inspected.
    ///
    /// Only properties that are `allowed_in_when` are guaranteed to have
    /// variable arguments with values that can print debug. For other properties
    /// the [`value`](PropertyArgInfo::value) is always an empty string and
    /// [`value_version`](PropertyArgInfo::value_version) is always zero.
    pub can_debug_args: bool,

    /// If the user assigned this property.
    pub user_assigned: bool,
}

/// When block setup by a widget instance.
#[derive(Debug, Clone)]
pub struct WhenInfo {
    /// When condition expression.
    pub condition_expr: &'static str,
    /// Current when condition result.
    pub condition: bool,
    /// Condition value version.
    pub condition_version: VarVersion,
    /// Properties affected by this when block.
    pub properties: HashSet<&'static str>,

    /// Source-code location of the when block declaration.
    pub decl_location: SourceLocation,

    /// If the user declared the when block in the widget instance.
    pub user_declared: bool,
}

state_key! {
    struct PropertiesInfoKey: Vec<PropertyInstance>;
    struct WidgetInstanceInfoKey: WidgetInstance;
}

unique_id_64! {
    /// Unique ID of a widget instance.
    ///
    /// This is different from the `WidgetId` in that it cannot be manipulated by the user
    /// and identifies the widget *instantiation* event during debug mode.
    #[derive(Debug)]
    pub struct WidgetInstanceId;
}

context_var! {
    struct ParentPropertyName: &'static str = "";
}

type PropertyMembersVars = Box<[BoxedVar<ValueInfo>]>;
type PropertiesVars = Box<[PropertyMembersVars]>;

// Node inserted just before calling the widget new function in debug mode.
// It registers the `WidgetInstanceInfo` metadata.
#[doc(hidden)]
pub struct WidgetInstanceInfoNode {
    child: BoxedUiNode,
    info: WidgetInstance,
    // debug vars, [capture-fn => properties[members[var]]]
    debug_vars: LinearMap<WidgetNewFn, PropertiesVars>,
    // when condition result variables.
    when_vars: Box<[BoxedVar<bool>]>,
}
#[doc(hidden)]
pub struct CapturedPropertyV1 {
    pub property_name: &'static str,
    pub instance_location: SourceLocation,
    pub arg_names: &'static [&'static str],
    pub arg_debug_vars: DebugArgs,
    pub user_assigned: bool,
}
#[doc(hidden)]
pub struct WhenInfoV1 {
    pub condition_expr: &'static str,
    pub condition_var: Option<BoxedVar<bool>>,
    pub properties: Vec<&'static str>,
    pub decl_location: SourceLocation,
    pub user_declared: bool,
}

#[allow(missing_docs)] // this is all hidden
impl WidgetInstanceInfoNode {
    pub fn new_v1(
        node: BoxedUiNode,
        widget_name: &'static str,
        decl_location: SourceLocation,
        instance_location: SourceLocation,
        captures: Vec<(WidgetNewFnV1, Vec<CapturedPropertyV1>)>,
        mut whens: Vec<WhenInfoV1>,
    ) -> Self {
        let mut debug_vars = LinearMap::default();
        debug_vars.reserve(captures.len());
        let mut captures_final = LinearMap::default();
        captures_final.reserve(captures.len());

        for (fn_, properties) in captures {
            let mut infos = Vec::with_capacity(properties.len());
            let mut vars = Vec::with_capacity(properties.len());

            for mut p in properties {
                let dbg_vars: PropertyMembersVars = std::mem::take(&mut p.arg_debug_vars);
                infos.push(CapturedPropertyInfo {
                    property_name: p.property_name,
                    instance_location: p.instance_location,
                    args: p
                        .arg_names
                        .iter()
                        .map(|n| PropertyArgInfo {
                            name: n,
                            // TODO is this right?
                            value: ValueInfo {
                                debug: "".into(),
                                debug_alt: "".into(),
                                type_name: "".into(),
                            },
                            value_version: VarVersion::normal(0),
                            can_update: false,
                        })
                        .collect(),
                    can_debug_args: !dbg_vars.is_empty(),
                    user_assigned: p.user_assigned,
                });
                vars.push(dbg_vars);
            }

            let vars: PropertiesVars = vars.into_boxed_slice();
            debug_vars.insert(fn_, vars);
            captures_final.insert(fn_, infos.into_boxed_slice());
        }

        let when_vars = whens
            .iter_mut()
            .map(|w| w.condition_var.take().unwrap())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let whens = whens
            .into_iter()
            .map(|w| WhenInfo {
                condition_expr: w.condition_expr,
                condition: false,
                condition_version: VarVersion::normal(0),
                properties: w.properties.into_iter().collect(),
                decl_location: w.decl_location,
                user_declared: w.user_declared,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        WidgetInstanceInfoNode {
            child: node,
            info: Rc::new(RefCell::new(WidgetInstanceInfo {
                instance_id: WidgetInstanceId::new_unique(),
                widget_name,
                decl_location,
                instance_location,
                captures: captures_final,
                whens,
                parent_property: "",
            })),
            debug_vars,
            when_vars,
        }
    }
}
impl UiNode for WidgetInstanceInfoNode {
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        let _scope = tracing::trace_span!("widget.info", name = self.info.borrow().widget_name, id = ?ctx.path.widget_id()).entered();

        info.meta().set(WidgetInstanceInfoKey, Rc::clone(&self.info));
        self.child.info(ctx, info);
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        let _scope = tracing::trace_span!("widget.subscriptions", name = self.info.borrow().widget_name).entered();

        self.child.subscriptions(ctx, subscriptions);
    }

    fn init(&mut self, ctx: &mut WidgetContext) {
        let _scope = tracing::trace_span!("widget.init", name = self.info.borrow().widget_name).entered();

        {
            ctx.vars
                .with_context_var(ParentPropertyName, ContextVarData::fixed(&"new(..)"), || {
                    self.child.init(ctx);
                });
        }

        let mut info_borrow = self.info.borrow_mut();
        let info = &mut *info_borrow;

        for (fn_, p) in &mut info.captures {
            for (p, vars) in p.iter_mut().zip(self.debug_vars.get(fn_).unwrap().iter()) {
                for (arg, var) in p.args.iter_mut().zip(vars.iter()) {
                    arg.value = var.get_clone(ctx);
                    arg.value_version = var.version(ctx);
                    arg.can_update = var.can_update();
                }
            }
        }

        for (when, var) in info.whens.iter_mut().zip(self.when_vars.iter()) {
            when.condition = var.copy(ctx);
            when.condition_version = var.version(ctx);
        }

        info.parent_property = ParentPropertyName::get(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let _scope = tracing::trace_span!("widget.deinit", name = self.info.borrow().widget_name).entered();

        self.child.deinit(ctx);
    }

    fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
        let _scope = tracing::trace_span!("widget.event", name = self.info.borrow().widget_name).entered();

        self.child.event(ctx, args);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let _scope = tracing::trace_span!("widget.update", name = self.info.borrow().widget_name).entered();

        self.child.update(ctx);

        let mut info_borrow = self.info.borrow_mut();
        let info = &mut *info_borrow;

        for (fn_, p) in &mut info.captures {
            for (p, vars) in p.iter_mut().zip(self.debug_vars.get(fn_).unwrap().iter()) {
                for (arg, var) in p.args.iter_mut().zip(vars.iter()) {
                    if let Some(update) = var.clone_new(ctx) {
                        arg.value = update;
                        arg.value_version = var.version(ctx);
                    }
                }
            }
        }
        for (when, var) in info.whens.iter_mut().zip(self.when_vars.iter()) {
            if let Some(update) = var.get_new(ctx) {
                when.condition = *update;
                when.condition_version = var.version(ctx);
            }
        }
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        let _scope = tracing::trace_span!("widget.measure", name = self.info.borrow().widget_name).entered();
        self.child.measure(ctx, available_size)
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        let _scope = tracing::trace_span!("widget.arrange", name = self.info.borrow().widget_name).entered();
        self.child.arrange(ctx, widget_layout, final_size)
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let _scope = tracing::trace_span!("widget.render", name = self.info.borrow().widget_name).entered();

        self.child.render(ctx, frame);
    }

    fn render_update(&self, ctx: &mut RenderContext, updates: &mut FrameUpdate) {
        let _scope = tracing::trace_span!("widget.render_update", name = self.info.borrow().widget_name).entered();

        self.child.render_update(ctx, updates);
    }
}

// Node inserted around each widget property in debug mode.
//
// It collects information about the UiNode methods, tracks property variable values
// and registers the property in the widget metadata in a frame.
#[doc(hidden)]
pub struct PropertyInfoNode {
    child: BoxedUiNode,
    arg_debug_vars: Box<[BoxedVar<ValueInfo>]>,
    info: PropertyInstance,
}
#[allow(missing_docs)] // this is all hidden
impl PropertyInfoNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new_v1(
        node: BoxedUiNode,

        priority: PropertyPriority,
        original_name: &'static str,
        decl_location: SourceLocation,

        property_name: &'static str,
        instance_location: SourceLocation,

        arg_names: &[&'static str],
        arg_debug_vars: Box<[BoxedVar<ValueInfo>]>,

        user_assigned: bool,
    ) -> Self {
        assert!(!arg_names.is_empty() && (arg_debug_vars.is_empty() || arg_names.len() == arg_debug_vars.len()));
        let can_debug_args = !arg_debug_vars.is_empty();
        PropertyInfoNode {
            child: node,
            arg_debug_vars,
            info: Rc::new(RefCell::new(PropertyInstanceInfo {
                priority,
                original_name,
                decl_location,
                property_name,
                instance_location,
                args: arg_names
                    .iter()
                    .map(|n| PropertyArgInfo {
                        name: n,
                        value: ValueInfo {
                            debug: "".into(),
                            debug_alt: "".into(),
                            type_name: "".into(),
                        },
                        value_version: VarVersion::normal(0),
                        can_update: false,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                can_debug_args,
                user_assigned,
                duration: UiNodeDurations::default(),
                count: UiNodeCounts::default(),
            })),
        }
    }
}
impl UiNode for PropertyInfoNode {
    fn info(&self, ctx: &mut InfoContext, wgt_info: &mut WidgetInfoBuilder) {
        let _scope = tracing::trace_span!("property.info", name = self.info.borrow().property_name).entered();

        wgt_info.meta().entry(PropertiesInfoKey).or_default().push(Rc::clone(&self.info));

        let t = Instant::now();
        self.child.info(ctx, wgt_info);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.info = d;
        info.count.info += 1;
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        let _scope = tracing::trace_span!("property.subscriptions", name = self.info.borrow().property_name).entered();

        let t = Instant::now();
        self.child.subscriptions(ctx, subscriptions);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.subscriptions = d;
        info.count.subscriptions += 1;
    }

    fn init(&mut self, ctx: &mut WidgetContext) {
        let _scope = tracing::trace_span!("property.init", name = self.info.borrow().property_name).entered();

        let mut info = self.info.borrow_mut();
        let property_name = info.property_name;

        ctx.vars
            .with_context_var(ParentPropertyName, ContextVarData::fixed(&property_name), || {
                let t = Instant::now();
                self.child.init(ctx);
                let d = t.elapsed();
                info.duration.init = d;
                info.count.init += 1;
            });

        for (var, arg) in self.arg_debug_vars.iter().zip(info.args.iter_mut()) {
            arg.value = var.get_clone(ctx);
            arg.value_version = var.version(ctx);
            arg.can_update = var.can_update();
        }
    }
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let _scope = tracing::trace_span!("property.deinit", name = self.info.borrow().property_name).entered();

        let t = Instant::now();
        self.child.deinit(ctx);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.deinit = d;
        info.count.deinit += 1;
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let _scope = tracing::trace_span!("property.update", name = self.info.borrow().property_name).entered();

        let t = Instant::now();
        self.child.update(ctx);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.update = d;
        info.count.update += 1;

        for (var, arg) in self.arg_debug_vars.iter_mut().zip(info.args.iter_mut()) {
            if let Some(new) = var.get_new(ctx) {
                arg.value = new.clone();
                arg.value_version = var.version(ctx);
            }
        }
    }

    fn event<U: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &U)
    where
        Self: Sized,
    {
        let _scope = tracing::trace_span!("property.event", name = self.info.borrow().property_name).entered();

        // TODO measure time and count
        self.child.event(ctx, args);
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        let _scope = tracing::trace_span!("property.measure", name = self.info.borrow().property_name).entered();

        let t = Instant::now();
        let r = self.child.measure(ctx, available_size);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.measure = d;
        info.count.measure += 1;
        r
    }
    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        let _scope = tracing::trace_span!("property.arrange", name = self.info.borrow().property_name).entered();

        let t = Instant::now();
        self.child.arrange(ctx, widget_layout, final_size);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.arrange = d;
        info.count.arrange += 1;
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let _scope = tracing::trace_span!("property.render", name = self.info.borrow().property_name).entered();

        let t = Instant::now();
        self.child.render(ctx, frame);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.render = d;
        info.count.render += 1;
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let t = Instant::now();
        self.child.render_update(ctx, update);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.render_update = d;
        info.count.render_update += 1;
    }
}

#[doc(hidden)]
pub struct NewChildMarkerNode {
    child: BoxedUiNode,
}
#[allow(missing_docs)] // this is hidden
impl NewChildMarkerNode {
    pub fn new_v1(child: BoxedUiNode) -> Self {
        NewChildMarkerNode { child }
    }
}
#[impl_ui_node(child)]
impl UiNode for NewChildMarkerNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        ctx.vars
            .with_context_var(ParentPropertyName, ContextVarData::fixed(&"new_child(..)"), || {
                self.child.init(ctx);
            });
    }
}

/// Formatted data for the inspector.
#[derive(Debug, Clone)]
pub struct ValueInfo {
    /// Data formatted using `{:?}`.
    pub debug: Text,
    /// Data formatted using the `{:#?}`.
    pub debug_alt: Text,
    /// Data type name, acquired using [`std::any::type_name`].
    pub type_name: Text,
}
impl ValueInfo {
    /// New [`ValueInfo`] from a value type that is only known to implement [`Debug`](fmt::Debug).
    pub fn new<T: fmt::Debug>(value: &T) -> Self {
        Self {
            debug: formatx!("{value:?}"),
            debug_alt: formatx!("{value:#?}"),
            type_name: std::any::type_name::<T>().into(),
        }
    }

    /// New [`ValueInfo`] from a value type that is not known to implement any format trait.
    pub fn new_type_name_only<T>(_: &T) -> Self {
        let name = std::any::type_name::<T>();

        let debug = if name.starts_with("zero_ui_core::widget_base::implicit_base::new::WidgetNode") {
            "<widget!>".to_text()
        } else if name == "zero_ui_core::ui_list::WidgetVec" || name.starts_with("zero_ui_core::ui_list::WidgetList") {
            "<[widgets!]>".to_text()
        } else if name == "zero_ui_core::ui_list::UiNodeVec" || name.starts_with("zero_ui_core::ui_list::UiNodeList") {
            "<[nodes!]>".to_text()
        } else if name.ends_with("{{closure}}") {
            "<{{closure}}>".to_text()
        } else if name.contains("::FnMutWidgetHandler<") {
            "hn!({{closure}})".to_text()
        } else if name.contains("::FnOnceWidgetHandler<") {
            "hn_once!({{closure}})".to_text()
        } else if name.contains("::AsyncFnMutWidgetHandler<") {
            "async_hn!({{closure}})".to_text()
        } else if name.contains("::AsyncFnOnceWidgetHandler<") {
            "async_hn_once!({{closure}})".to_text()
        } else if name.contains("::FnMutAppHandler<") {
            "app_hn!({{closure}})".to_text()
        } else if name.contains("::FnOnceAppHandler<") {
            "app_hn_once!({{closure}})".to_text()
        } else if name.contains("::AsyncFnMutAppHandler<") {
            "async_app_hn!({{closure}})".to_text()
        } else if name.contains("::AsyncFnOnceAppHandler<") {
            "async_app_hn_once!({{closure}})".to_text()
        } else {
            // TODO short name
            formatx!("<{name}>")
        };

        Self {
            debug,
            debug_alt: formatx!("<{name}>"),
            type_name: name.into(),
        }
    }
}
impl PartialEq for ValueInfo {
    fn eq(&self, other: &Self) -> bool {
        self.type_name == other.type_name && self.debug == other.debug
    }
}

#[doc(hidden)]
pub mod debug_var_util {
    use std::fmt::Debug;

    use crate::var::{BoxedVar, IntoValue, IntoVar, OwnedVar, Var, VarValue};

    use super::ValueInfo;

    pub struct Wrap<T>(pub T);
    //
    // `Wrap` - type_name only
    //
    pub trait FromTypeNameOnly {
        fn debug_var(&self) -> crate::var::BoxedVar<ValueInfo>;
    }
    impl<T> FromTypeNameOnly for Wrap<&T> {
        fn debug_var(&self) -> BoxedVar<ValueInfo> {
            OwnedVar(ValueInfo::new_type_name_only(self.0)).boxed()
        }
    }

    //
    // `&Wrap` - Into<Debug>
    //
    pub trait FromIntoValueDebug<T> {
        fn debug_var(&self) -> crate::var::BoxedVar<ValueInfo>;
    }
    impl<T: Debug, V: IntoValue<T>> FromIntoValueDebug<T> for &Wrap<&V> {
        fn debug_var(&self) -> BoxedVar<ValueInfo> {
            OwnedVar(ValueInfo::new(&self.0.clone().into())).boxed()
        }
    }

    //
    // `&&Wrap` - IntoVar<Debug>
    //
    pub trait FromIntoVar<T> {
        fn debug_var(&self) -> crate::var::BoxedVar<ValueInfo>;
    }
    impl<T: VarValue, V: IntoVar<T>> FromIntoVar<T> for &&Wrap<&V> {
        fn debug_var(&self) -> BoxedVar<ValueInfo> {
            self.0.clone().into_var().map(ValueInfo::new).boxed()
        }
    }

    //
    // `&&&Wrap` - Debug only
    //
    pub trait FromDebug {
        fn debug_var(&self) -> crate::var::BoxedVar<ValueInfo>;
    }
    impl<T: Debug> FromDebug for &&&Wrap<&T> {
        fn debug_var(&self) -> BoxedVar<ValueInfo> {
            OwnedVar(ValueInfo::new(self.0)).boxed()
        }
    }

    //
    // `&&&&Wrap` - Var<Debug>
    //
    pub trait FromVarDebugOnly<T> {
        fn debug_var(&self) -> crate::var::BoxedVar<ValueInfo>;
    }
    impl<T: VarValue, V: Var<T>> FromVarDebugOnly<T> for &&&&Wrap<&V> {
        fn debug_var(&self) -> BoxedVar<ValueInfo> {
            self.0.map(ValueInfo::new).boxed()
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::var::{IntoValue, Var};

        macro_rules! debug_var_util_trick {
            ($value:expr) => {{
                use $crate::inspector::debug_var_util::*;
                (&&&&Wrap($value)).debug_var()
            }};
        }

        use crate::context::TestWidgetContext;

        #[test]
        fn from_into_var() {
            use crate::var::IntoVar;
            fn value() -> impl IntoVar<&'static str> {
                #[derive(Clone, Copy)]
                struct Test;
                impl IntoVar<&'static str> for Test {
                    type Var = crate::var::OwnedVar<&'static str>;

                    fn into_var(self) -> Self::Var {
                        crate::var::OwnedVar("called into_var")
                    }
                }
                Test
            }
            let value = value();

            let r = debug_var_util_trick!(&value);

            let ctx = TestWidgetContext::new();

            assert_eq!("\"called into_var\"", r.get(&ctx.vars).debug)
        }

        #[test]
        pub fn from_into_value() {
            fn value() -> impl IntoValue<bool> {
                true
            }
            let value = value();

            let r = debug_var_util_trick!(&value);

            let ctx = TestWidgetContext::new();

            assert_eq!("true", r.get(&ctx.vars).debug)
        }

        #[test]
        fn from_var() {
            use crate::var::var;

            let value = var(true);

            let r = debug_var_util_trick!(&value);

            let mut ctx = TestWidgetContext::new();

            assert_eq!("true", r.get(&ctx.vars).debug);

            value.set(&ctx.vars, false);

            ctx.apply_updates();

            assert_eq!("false", r.get(&ctx.vars).debug);
        }

        #[test]
        fn from_debug() {
            let value = true;

            let r = debug_var_util_trick!(&value);

            let ctx = TestWidgetContext::new();

            assert_eq!("true", r.get(&ctx.vars).debug)
        }

        #[test]
        fn from_any() {
            struct Foo;
            let value = Foo;

            let r = debug_var_util_trick!(&value);

            let ctx = TestWidgetContext::new();

            assert!(r.get(&ctx.vars).debug.contains("Foo"));
        }

        #[test]
        pub fn from_into_problem() {
            use crate::inspector::ValueInfo;
            use crate::{NilUiNode, RcNode, UiNode};

            // `RcNode` is not Debug but matches `<T: Debug, V: Into<V> + Clone>` anyway.
            fn value() -> RcNode<impl UiNode> {
                RcNode::new(NilUiNode)
            }
            let value = value();
            let expected = ValueInfo::new_type_name_only(&value).debug;

            let r = debug_var_util_trick!(&value);

            let ctx = TestWidgetContext::new();

            assert_eq!(expected, r.get(&ctx.vars).debug)
        }
    }
}

#[doc(hidden)]
pub type DebugArgs = Box<[BoxedVar<ValueInfo>]>;

/// Adds debug information to [`WidgetInfo`].
pub trait WidgetInspectorInfo<'a> {
    /// If the widget was instantiated with inspector instrumentation.
    #[allow(clippy::wrong_self_convention)] // `self` is a reference.
    fn is_inspected(self) -> bool;

    /// Gets the widget instance info if the widget is [`is_inspected`](Self::is_inspected).
    fn instance(self) -> Option<&'a WidgetInstance>;

    /// Gets the widget properties info.
    ///
    /// Returns empty if [`is_inspected`](Self::is_inspected) is `false`.
    fn properties(self) -> &'a [PropertyInstance];
}
impl<'a> WidgetInspectorInfo<'a> for WidgetInfo<'a> {
    #[inline]
    fn is_inspected(self) -> bool {
        self.meta().contains(WidgetInstanceInfoKey)
    }

    #[inline]
    fn instance(self) -> Option<&'a WidgetInstance> {
        self.meta().get(WidgetInstanceInfoKey)
    }

    #[inline]
    fn properties(self) -> &'a [PropertyInstance] {
        self.meta().get(PropertiesInfoKey).map(|v| &v[..]).unwrap_or(&[])
    }
}

/// State for tracking updates in [`write_tree`].
pub struct WriteTreeState {
    #[allow(clippy::type_complexity)]
    widgets: IdMap<WidgetInstanceId, WriteWidgetState>,
}
struct WriteWidgetState {
    outer_bounds: PxRect,
    inner_bounds: PxRect,
    visibility: Visibility,
    /// [(property_name, arg_name) => (value_version, value)]
    properties: HashMap<(&'static str, &'static str), (VarVersion, ValueInfo)>,
}
impl WriteTreeState {
    /// No property update.
    pub fn none() -> Self {
        WriteTreeState {
            widgets: Default::default(),
        }
    }

    /// State represents no property update.
    pub fn is_none(&self) -> bool {
        self.widgets.is_empty()
    }

    /// State from `tree` that can be compared to future trees.
    pub fn new(tree: &WidgetInfoTree) -> Self {
        let mut widgets = IdMap::default();

        for w in tree.all_widgets() {
            if let Some(info) = w.instance() {
                let info = info.borrow();
                let mut properties = HashMap::new();
                for p in info.captures.values().flat_map(|c| c.iter()) {
                    for arg in p.args.iter() {
                        properties.insert((p.property_name, arg.name), (arg.value_version, arg.value.clone()));
                    }
                }
                for p in w.properties() {
                    let p = p.borrow();
                    for arg in p.args.iter() {
                        properties.insert((p.property_name, arg.name), (arg.value_version, arg.value.clone()));
                    }
                }
                widgets.insert(
                    info.instance_id,
                    WriteWidgetState {
                        outer_bounds: w.outer_bounds(),
                        inner_bounds: w.inner_bounds(),
                        visibility: w.visibility(),
                        properties,
                    },
                );
            }
        }

        WriteTreeState { widgets }
    }

    /// Gets the change in a property argument.
    pub fn arg_diff(&self, widget_id: WidgetInstanceId, property_name: &'static str, arg: &PropertyArgInfo) -> Option<WriteArgDiff> {
        if !self.is_none() {
            if let Some(wgt_state) = self.widgets.get(&widget_id) {
                if let Some((value_version, value)) = wgt_state.properties.get(&(property_name, arg.name)) {
                    if *value_version != arg.value_version {
                        return Some(if value != &arg.value {
                            WriteArgDiff::NewValue
                        } else {
                            WriteArgDiff::NewVersion
                        });
                    }
                }
            }
        }
        None
    }

    /// Gets the change in the widget inner-bounds.
    pub fn inner_bounds_diff(&self, widget_id: WidgetInstanceId, inner_bounds: PxRect) -> Option<WriteArgDiff> {
        if !self.is_none() {
            if let Some(wgt_state) = self.widgets.get(&widget_id) {
                if wgt_state.inner_bounds != inner_bounds {
                    return Some(WriteArgDiff::NewValue);
                }
            }
        }
        None
    }

    /// Gets the change in the widget outer-bounds.
    pub fn outer_bounds_diff(&self, widget_id: WidgetInstanceId, outer_bounds: PxRect) -> Option<WriteArgDiff> {
        if !self.is_none() {
            if let Some(wgt_state) = self.widgets.get(&widget_id) {
                if wgt_state.outer_bounds != outer_bounds {
                    return Some(WriteArgDiff::NewValue);
                }
            }
        }
        None
    }

    /// Gets the change in the widget visibility.
    pub fn visibility_diff(&self, widget_id: WidgetInstanceId, visibility: Visibility) -> Option<WriteArgDiff> {
        if !self.is_none() {
            if let Some(wgt_state) = self.widgets.get(&widget_id) {
                if wgt_state.visibility != visibility {
                    return Some(WriteArgDiff::NewValue);
                }
            }
        }
        None
    }
}

/// Represents the change in a property argument calculated by [`WriteTreeState`].
pub enum WriteArgDiff {
    /// The argument is equal the previous one, but the variable version changed.
    NewVersion,
    /// The argument is not equal the previous one.
    NewValue,
}

/// Writes the widget `tree` to `out`.
///
/// When writing to a terminal the text is color coded and a legend is printed. The coloring
/// can be configured using environment variables, see [colored](https://github.com/mackwic/colored#features)
/// for details.
#[inline]
pub fn write_tree<W: std::io::Write>(tree: &WidgetInfoTree, updates_from: &WriteTreeState, out: &mut W) {
    let mut fmt = print_fmt::Fmt::new(out);
    write_impl(updates_from, tree.root(), "", &mut fmt);
    fmt.write_legend();
}
fn write_impl<W: std::io::Write>(updates_from: &WriteTreeState, widget: WidgetInfo, parent_name: &str, fmt: &mut print_fmt::Fmt<W>) {
    if let Some(info) = widget.instance() {
        let wgt = info.borrow();

        fmt.open_widget(wgt.widget_name, parent_name, wgt.parent_property);

        macro_rules! write_property {
            ($p:ident, $group:tt) => {
                if $p.can_debug_args {
                    if $p.args.len() == 1 {
                        fmt.write_property(
                            $group,
                            $p.property_name,
                            &$p.args[0].value,
                            $p.user_assigned,
                            $p.args[0].can_update,
                            updates_from.arg_diff(wgt.instance_id, $p.property_name, &$p.args[0]),
                        );
                    } else {
                        fmt.open_property($group, $p.property_name, $p.user_assigned);
                        for arg in $p.args.iter() {
                            fmt.write_property_arg(
                                arg.name,
                                &arg.value,
                                $p.user_assigned,
                                arg.can_update,
                                updates_from.arg_diff(wgt.instance_id, $p.property_name, &arg),
                            );
                        }
                        fmt.close_property($p.user_assigned);
                    }
                } else {
                    fmt.write_property_no_dbg($group, $p.property_name, $p.args.iter().map(|a| a.name), $p.user_assigned);
                }
            };
        }

        if let Some(p) = wgt.captures.get(&WidgetNewFn::NewChild) {
            for p in p.iter() {
                write_property!(p, "new_child");
            }
        }
        for prop in widget.properties() {
            // TODO other capture functions
            let p = prop.borrow();
            let group = p.priority.token_str();
            write_property!(p, group);
        }
        if let Some(p) = wgt.captures.get(&WidgetNewFn::New) {
            for p in p.iter() {
                write_property!(p, "new");
            }
        }

        fmt.writeln();

        fmt.write_property(
            ".layout",
            ".inner_bounds",
            {
                let bounds = widget.inner_bounds();
                &ValueInfo {
                    debug: formatx!("{bounds:?}"),
                    debug_alt: formatx!("{bounds:#?}"),
                    type_name: std::any::type_name::<PxRect>().into(),
                }
            },
            false,
            true,
            updates_from.inner_bounds_diff(wgt.instance_id, widget.inner_bounds()),
        );
        fmt.write_property(
            ".layout",
            ".outer_bounds",
            {
                let bounds = widget.outer_bounds();
                &ValueInfo {
                    debug: formatx!("{bounds:?}"),
                    debug_alt: formatx!("{bounds:#?}"),
                    type_name: std::any::type_name::<PxRect>().into(),
                }
            },
            false,
            true,
            updates_from.outer_bounds_diff(wgt.instance_id, widget.outer_bounds()),
        );
        fmt.write_property(
            ".render",
            ".visibility",
            {
                let vis = widget.visibility();
                &ValueInfo {
                    debug: formatx!("{vis:?}"),
                    debug_alt: formatx!("{vis:#?}"),
                    type_name: std::any::type_name::<Visibility>().into(),
                }
            },
            false,
            true,
            updates_from.visibility_diff(wgt.instance_id, widget.visibility()),
        );

        for child in widget.children() {
            write_impl(updates_from, child, wgt.widget_name, fmt);
        }

        fmt.close_widget(wgt.widget_name);
    } else {
        fmt.open_widget("<unknown>", "", "");

        fmt.write_property(
            ".layout",
            ".bounds",
            {
                let bounds = widget.inner_bounds();
                &ValueInfo {
                    debug: formatx!(
                        "({}, {}).at({}, {})",
                        bounds.size.width,
                        bounds.size.height,
                        bounds.origin.x,
                        bounds.origin.y
                    ),
                    debug_alt: formatx!(
                        "LayoutRect {{\n    width: {},\n    height: {},\n    x: {},\n    y: {}}}",
                        bounds.size.width,
                        bounds.size.height,
                        bounds.origin.x,
                        bounds.origin.y
                    ),
                    type_name: std::any::type_name::<PxRect>().into(),
                }
            },
            false,
            true,
            None,
        );

        for child in widget.children() {
            write_impl(updates_from, child, "<unknown>", fmt);
        }
        fmt.close_widget("<unknown>");
    }
}
mod print_fmt {
    use crate::formatx;

    use super::{ValueInfo, WriteArgDiff};
    use colored::*;
    use std::fmt::Display;
    use std::io::Write;

    pub struct Fmt<'w, W: Write> {
        depth: u32,
        output: &'w mut W,
        property_group: &'static str,
    }
    impl<'w, W: Write> Fmt<'w, W> {
        pub fn new(output: &'w mut W) -> Self {
            Fmt {
                depth: 0,
                output,
                property_group: "",
            }
        }

        fn write_tabs(&mut self) {
            let _ = write!(&mut self.output, "{:d$}", "", d = self.depth as usize * 3);
        }

        fn write(&mut self, s: impl Display) {
            let _ = write!(&mut self.output, "{s}");
        }

        pub fn writeln(&mut self) {
            let _ = writeln!(&mut self.output);
        }

        pub fn write_comment(&mut self, comment: impl Display) {
            self.write_tabs();
            self.write_comment_after(comment);
        }

        fn write_comment_after(&mut self, comment: impl Display) {
            self.write("// ".truecolor(117, 113, 94));
            self.write(comment.to_string().truecolor(117, 113, 94));
            self.writeln();
        }

        pub fn open_widget(&mut self, name: &str, parent_name: &str, parent_property: &str) {
            if !parent_property.is_empty() {
                self.writeln();
                self.write_comment(format_args!("in {parent_name}::{parent_property}"));
            }
            self.write_tabs();
            self.write(name.yellow());
            self.write("!".yellow());
            self.write(" {".bold());
            self.writeln();
            self.depth += 1;
        }

        fn write_property_header(&mut self, group: &'static str, name: &str, user_assigned: bool) {
            if self.property_group != group {
                self.write_comment(group);
                self.property_group = group;
            }

            self.write_tabs();
            if user_assigned {
                self.write(name.blue().bold());
            } else {
                self.write(name);
            }
            self.write(" = ");
        }

        fn write_property_end(&mut self, user_assigned: bool) {
            if user_assigned {
                self.write(";".blue().bold());
            } else {
                self.write(";");
            }
            self.writeln();
        }

        fn write_property_value(&mut self, value: &ValueInfo, can_update: bool, diff: Option<WriteArgDiff>) {
            let mut l0 = true;
            for line in value.debug.lines() {
                if l0 {
                    l0 = false;
                } else {
                    self.writeln();
                    self.write_tabs();
                }

                if let Some(diff) = &diff {
                    match diff {
                        WriteArgDiff::NewVersion => self.write(line.truecolor(100, 150, 100)),
                        WriteArgDiff::NewValue => self.write(line.truecolor(150, 255, 150).bold()),
                    }
                } else if can_update {
                    self.write(line.truecolor(200, 150, 150));
                } else {
                    self.write(line.truecolor(150, 150, 200));
                }
            }
        }

        pub fn write_property(
            &mut self,
            group: &'static str,
            name: &str,
            value: &ValueInfo,
            user_assigned: bool,
            can_update: bool,
            diff: Option<WriteArgDiff>,
        ) {
            self.write_property_header(group, name, user_assigned);
            self.write_property_value(value, can_update, diff);
            self.write_property_end(user_assigned);
        }

        pub fn write_property_no_dbg(
            &mut self,
            group: &'static str,
            name: &str,
            arg_names: impl Iterator<Item = &'static str>,
            user_assigned: bool,
        ) {
            self.write_property_header(group, name, user_assigned);
            let mut a0 = true;
            for arg in arg_names {
                if a0 {
                    a0 = false;
                } else if user_assigned {
                    self.write(", ".blue().bold());
                } else {
                    self.write(", ");
                }
                self.write_property_value(
                    &ValueInfo {
                        debug: formatx!("<{arg}>"),
                        debug_alt: "".into(),
                        type_name: "".into(),
                    },
                    false,
                    None,
                );
            }
            self.write_property_end(user_assigned);
        }

        pub fn open_property(&mut self, group: &'static str, name: &str, user_assigned: bool) {
            self.write_property_header(group, name, user_assigned);
            if user_assigned {
                self.write("{".blue().bold());
            } else {
                self.write("{");
            }
            self.writeln();
            self.depth += 1;
        }

        pub fn write_property_arg(
            &mut self,
            name: &str,
            value: &ValueInfo,
            user_assigned: bool,
            can_update: bool,
            diff: Option<WriteArgDiff>,
        ) {
            self.write_tabs();
            if user_assigned {
                self.write(name.blue().bold());
                self.write(": ".blue().bold());
            } else {
                self.write(name);
                self.write(": ");
            }
            self.write_property_value(value, can_update, diff);
            if user_assigned {
                self.write(",".blue().bold());
            } else {
                self.write(",");
            }
            self.writeln();
        }

        pub fn close_property(&mut self, user_assigned: bool) {
            self.depth -= 1;
            self.write_tabs();
            if user_assigned {
                self.write("}".blue().bold());
            } else {
                self.write("}");
            }
            self.write_property_end(user_assigned);
        }

        pub fn close_widget(&mut self, name: &str) {
            self.depth -= 1;
            self.property_group = "";
            self.write_tabs();
            self.write("} ".bold());
            self.write_comment_after(format_args!("{name}!"));
        }

        pub fn write_legend(&mut self) {
            if !control::SHOULD_COLORIZE.should_colorize() {
                return;
            }

            self.writeln();
            self.write("▉".yellow());
            self.write("  - widget");
            self.writeln();

            self.write("▉".blue());
            self.write("  - property, set by user");
            self.writeln();

            self.write("▉  - property, set by widget");
            self.writeln();

            self.write("▉".truecolor(200, 150, 150));
            self.write("  - variable");
            self.writeln();

            self.write("▉".truecolor(150, 150, 200));
            self.write("  - static, init value");
            self.writeln();

            self.write("▉".truecolor(150, 255, 150));
            self.write("  - updated, new value");
            self.writeln();

            self.write("▉".truecolor(100, 150, 100));
            self.write("  - updated, same value");
            self.writeln();

            self.writeln();
        }
    }
}

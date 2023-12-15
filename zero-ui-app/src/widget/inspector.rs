//! Helper types for inspecting an UI tree.
//!
//! When compiled with the `"inspector"` feature all widget instances are instrumented with inspection node
//! that shares a clone of the [`WidgetBuilder`] in the [`WidgetInfo`].

#[cfg(inspector)]
mod inspector_only {
    use std::sync::Arc;

    use zero_ui_app_context::LocalContext;

    use crate::widget::node::{match_node, BoxedUiNode, UiNode, UiNodeOp};

    pub(crate) fn insert_widget_builder_info(child: BoxedUiNode, info: super::InspectorInfo) -> impl UiNode {
        let insp_info = Arc::new(info);
        match_node(child, move |_, op| {
            if let UiNodeOp::Info { info } = op {
                let ctx = LocalContext::capture();
                *insp_info.context.ctx.lock() = ctx;

                info.set_meta(&super::INSPECTOR_INFO_ID, insp_info.clone());
            }
        })
    }
}
#[cfg(inspector)]
pub(crate) use inspector_only::*;
use parking_lot::Mutex;
use zero_ui_app_context::LocalContext;
use zero_ui_state_map::StaticStateId;

use std::{any::TypeId, sync::Arc};

use super::{
    builder::{InputKind, NestGroup, PropertyArgs, PropertyId, WidgetBuilder, WidgetType},
    info::WidgetInfo,
    WidgetUpdateMode, WIDGET,
};

pub(super) static INSPECTOR_INFO_ID: StaticStateId<Arc<InspectorInfo>> = StaticStateId::new_unique();

/// Widget instance item.
///
/// See [`InspectorInfo::items`].
#[derive(Debug)]
pub enum InstanceItem {
    /// Property instance.
    Property {
        /// Final property args.
        ///
        /// Unlike the same property in the builder, these args are affected by `when` assigns.
        args: Box<dyn PropertyArgs>,
        /// If the property was captured by the widget.
        ///
        /// If this is `true` the property is not instantiated in the widget, but its args are used in intrinsic nodes.
        captured: bool,
    },
    /// Marks an intrinsic node instance inserted by the widget.
    Intrinsic {
        /// Intrinsic node nest group.
        group: NestGroup,
        /// Name given to this intrinsic by the widget.
        name: &'static str,
    },
}

/// Widget instance inspector info.
///
/// Can be accessed and queried using [`WidgetInfoInspectorExt`].
#[derive(Debug)]
pub struct InspectorInfo {
    /// Builder that was used to instantiate the widget.
    pub builder: WidgetBuilder,

    /// Final instance items.
    pub items: Box<[InstanceItem]>,

    /// Widget context.
    ///
    /// Property variables can only be resolved inside this context.
    pub context: InspectorContext,
}
impl InspectorInfo {
    /// Iterate over property items.
    pub fn properties(&self) -> impl Iterator<Item = (&dyn PropertyArgs, bool)> {
        self.items.iter().filter_map(|it| match it {
            InstanceItem::Property { args, captured } => Some((&**args, *captured)),
            InstanceItem::Intrinsic { .. } => None,
        })
    }
}

/// Latest info [`LocalContext`] that must be used to inspect property variables.
///
/// The context is captured every info update, so may not reflect the exact context the
/// widget properties read the variable.
pub struct InspectorContext {
    ctx: Arc<Mutex<LocalContext>>,
}
impl std::fmt::Debug for InspectorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InspectorContext").finish_non_exhaustive()
    }
}
impl InspectorContext {
    /// New with empty context.
    pub fn new() -> Self {
        Self {
            ctx: Arc::new(Mutex::new(LocalContext::new())),
        }
    }

    /// Clone the latest context.
    pub fn latest_capture(&self) -> LocalContext {
        self.ctx.lock().clone()
    }
}
impl Default for InspectorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Extensions methods for [`WidgetInfo`].
pub trait WidgetInfoInspectorExt {
    /// Reference the builder that was used to generate the widget, the builder generated items and the widget info context.
    ///
    /// Returns `None` if not build with the `"inspector"` feature, or if the widget instance was not created using
    /// the standard builder.
    fn inspector_info(&self) -> Option<Arc<InspectorInfo>>;

    /// If a [`inspector_info`] is defined for the widget.
    ///
    /// [`inspector_info`]: Self::inspector_info
    fn can_inspect(&self) -> bool;

    /// Returns the first child that matches.
    fn inspect_child<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo>;

    /// Returns the first descendant that matches.
    ///
    /// # Examples
    ///
    /// Example searches for a "button" descendant, using a string search that matches the end of the [`WidgetType::path`] and
    /// an exact widget mod that matches the [`WidgetType::type_id`].
    ///
    /// ```
    /// # use zero_ui_app::widget::{inspector::*, info::*, builder::*};
    /// # fn main() { }
    /// mod widgets {
    ///     use zero_ui_app::widget::*;
    ///     
    ///     #[widget($crate::widgets::Button)]
    ///     pub struct Button(base::WidgetBase);
    /// }
    /// fn demo(info: WidgetInfo) {
    /// let fuzzy = info.inspect_descendant("button");
    /// let exact = info.inspect_descendant(std::any::TypeId::of::<crate::widgets::Button>());
    /// }
    /// ```
    fn inspect_descendant<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo>;

    /// Returns the first ancestor that matches.
    fn inspect_ancestor<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo>;

    /// Search for a property set on the widget.
    ///
    /// # Examples
    ///
    /// Search for a property by name, and then downcast its value.
    ///
    /// ```
    /// # use zero_ui_app::widget::{info::*, inspector::*};
    /// fn inspect_foo(info: WidgetInfo) -> Option<bool> {
    ///     info.inspect_property("foo")?.value(0).as_any().downcast_ref().copied()
    /// }
    /// ```
    fn inspect_property<P: InspectPropertyPattern>(&self, pattern: P) -> Option<&dyn PropertyArgs>;

    /// Gets the parent property that has this widget as an input.
    ///
    /// Returns `Some((PropertyId, member_index))`.
    fn parent_property(&self) -> Option<(PropertyId, usize)>;
}
impl WidgetInfoInspectorExt for WidgetInfo {
    fn inspector_info(&self) -> Option<Arc<InspectorInfo>> {
        self.meta().get_clone(&INSPECTOR_INFO_ID)
    }

    fn can_inspect(&self) -> bool {
        self.meta().contains(&INSPECTOR_INFO_ID)
    }

    fn inspect_child<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo> {
        self.children().find(|c| match c.meta().get(&INSPECTOR_INFO_ID) {
            Some(wgt) => pattern.matches(wgt),
            None => false,
        })
    }

    fn inspect_descendant<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo> {
        self.descendants().find(|c| match c.meta().get(&INSPECTOR_INFO_ID) {
            Some(info) => pattern.matches(info),
            None => false,
        })
    }

    fn inspect_ancestor<P: InspectWidgetPattern>(&self, pattern: P) -> Option<WidgetInfo> {
        self.ancestors().find(|c| match c.meta().get(&INSPECTOR_INFO_ID) {
            Some(info) => pattern.matches(info),
            None => false,
        })
    }

    fn inspect_property<P: InspectPropertyPattern>(&self, pattern: P) -> Option<&dyn PropertyArgs> {
        self.meta()
            .get(&INSPECTOR_INFO_ID)?
            .properties()
            .find_map(|(args, cap)| if pattern.matches(args, cap) { Some(args) } else { None })
    }

    fn parent_property(&self) -> Option<(PropertyId, usize)> {
        self.parent()?.meta().get(&INSPECTOR_INFO_ID)?.properties().find_map(|(args, _)| {
            let id = self.id();
            let info = args.property();
            for (i, input) in info.inputs.iter().enumerate() {
                match input.kind {
                    InputKind::UiNode => {
                        let node = args.ui_node(i);
                        if let Some(true) = node.try_context(WidgetUpdateMode::Ignore, || WIDGET.id() == id) {
                            return Some((args.id(), i));
                        }
                    }
                    InputKind::UiNodeList => {
                        let list = args.ui_node_list(i);
                        let mut found = false;
                        list.for_each_ctx(WidgetUpdateMode::Ignore, |_| {
                            if !found {
                                found = WIDGET.id() == id;
                            }
                        });
                        if found {
                            return Some((args.id(), i));
                        }
                    }
                    _ => continue,
                }
            }
            None
        })
    }
}

/// Query pattern for the [`WidgetInfoInspectorExt`] inspect methods.
pub trait InspectWidgetPattern {
    /// Returns `true` if the pattern includes the widget.
    fn matches(&self, info: &InspectorInfo) -> bool;
}
/// Matches if the [`WidgetType::path`] ends with the string.
impl<'s> InspectWidgetPattern for &'s str {
    fn matches(&self, info: &InspectorInfo) -> bool {
        info.builder.widget_type().path.ends_with(self)
    }
}
impl InspectWidgetPattern for TypeId {
    fn matches(&self, info: &InspectorInfo) -> bool {
        info.builder.widget_type().type_id == *self
    }
}
impl InspectWidgetPattern for WidgetType {
    fn matches(&self, info: &InspectorInfo) -> bool {
        info.builder.widget_type().type_id == self.type_id
    }
}

/// Query pattern for the [`WidgetInfoInspectorExt`] inspect methods.
pub trait InspectPropertyPattern {
    /// Returns `true` if the pattern includes the property.
    fn matches(&self, args: &dyn PropertyArgs, captured: bool) -> bool;
}
/// Matches if the [`PropertyInfo::name`] exactly.
///
/// [`PropertyInfo::name`]: crate::widget::builder::PropertyInfo::name
impl<'s> InspectPropertyPattern for &'s str {
    fn matches(&self, args: &dyn PropertyArgs, _: bool) -> bool {
        args.property().name == *self
    }
}
impl InspectPropertyPattern for PropertyId {
    fn matches(&self, args: &dyn PropertyArgs, _: bool) -> bool {
        args.id() == *self
    }
}

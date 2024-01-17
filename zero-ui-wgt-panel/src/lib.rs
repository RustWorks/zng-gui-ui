#![doc = include_str!("../../zero-ui-app/README.md")]
//!
//! Panel widget and properties.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zero_ui_wgt::enable_widget_macros!();

use zero_ui_wgt::prelude::*;
use zero_ui_wgt_wrap::Wrap;

/// Represents a styleable widget list container.
///
/// This widget can swap between actual layout
#[widget($crate::Panel)]
pub struct Panel(WidgetBase);

impl Panel {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            if let Some(p) = wgt.capture_property(property_id!(Self::children)) {
                let list = p.args.ui_node_list(0).clone();
                wgt.set_child(node(list, PANEL_FN_VAR));
            } else {
                wgt.set_child(node(ArcNodeList::new(ui_vec![].boxed()), PANEL_FN_VAR));
            }
        });
    }
}

/// Panel items.
#[property(CHILD, capture, default(ui_vec![]), widget_impl(Panel))]
pub fn children(children: impl UiNodeList) {}

context_var! {
    /// Defines the layout widget for [`Panel!`].
    ///
    /// Is a [`Wrap!`] panel by default.
    ///
    /// [`Panel!`]: struct@Panel
    /// [`Wrap!`]: struct@Wrap
    pub static PANEL_FN_VAR: WidgetFn<PanelArgs> = wgt_fn!(|a: PanelArgs| {
        Wrap! {
            children = a.children;
        }
    });
}

/// Widget function that generates the panel layout widget.
///
/// This property can be set in any widget to affect all [`Panel!`] descendants.
///
/// This property sets [`PANEL_FN_VAR`].
///
/// [`Panel!`]: struct@Panel
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(Panel))]
pub fn panel_fn(child: impl UiNode, panel: impl IntoVar<WidgetFn<PanelArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_FN_VAR, panel)
}

/// Arguments for [`panel_fn`].
///
/// [`panel_fn`]: fn@panel_fn
pub struct PanelArgs {
    /// The panel children.
    ///
    /// Note that this is probably an [`ArcNodeList`] take-on-init list so it may be empty until inited.
    pub children: BoxedUiNodeList,
}

/// Panel widget child node.
pub fn node(children: ArcNodeList<BoxedUiNodeList>, panel_fn: impl IntoVar<WidgetFn<PanelArgs>>) -> impl UiNode {
    let mut child = NilUiNode.boxed();
    let panel_fn = panel_fn.into_var();
    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&panel_fn);
            child = panel_fn.get().call(PanelArgs {
                children: children.take_on_init().boxed(),
            });
            child.init();
        }
        UiNodeOp::Deinit => {
            child.deinit();
            child = NilUiNode.boxed();
        }
        UiNodeOp::Update { updates } => {
            if let Some(f) = panel_fn.get_new() {
                child.deinit();
                child = f(PanelArgs {
                    children: children.take_on_init().boxed(),
                });
                child.init();
            } else {
                child.update(updates);
            }
        }
        op => child.op(op),
    })
}

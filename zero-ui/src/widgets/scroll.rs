//! Scroll widgets, properties and nodes..

use crate::prelude::new_widget::*;

pub mod commands;
pub mod nodes;
pub mod scrollbar;
pub mod thumb;

mod scroll_properties;
pub use scroll_properties::*;

mod types;
pub use types::*;

pub use scrollbar::Scrollbar;
pub use thumb::Thumb;

/// A single content container that can be larger on the inside.
#[widget($crate::widgets::Scroll)]
pub struct Scroll(ScrollUnitsMix<ScrollbarFnMix<Container>>);

/// Scroll mode.
///
/// Is [`ScrollMode::ZOOM`] by default.
#[property(CONTEXT, capture, default(ScrollMode::ZOOM), widget_impl(Scroll))]
pub fn mode(mode: impl IntoVar<ScrollMode>) {}

impl Scroll {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            child_align = Align::CENTER;
            clip_to_bounds = true;
            focusable = true;
            focus_scope = true;
            focus_scope_behavior = crate::core::focus::FocusScopeOnFocus::FirstDescendant;
        }
        self.widget_builder().push_build_action(on_build);
    }

    widget_impl! {
        /// Content alignment when it is smaller then the viewport.
        pub child_align(align: impl IntoVar<Align>);

        /// Clip content to only be visible within the scroll bounds, including under scrollbars.
        ///
        /// Enabled by default.
        pub clip_to_bounds(clip: impl IntoVar<bool>);

        /// Enables keyboard controls.
        pub focusable(focusable: impl IntoVar<bool>);
    }
}

/// Clip content to only be visible within the viewport, not under scrollbars.
///
/// Disabled by default.
#[property(CONTEXT, capture, default(false), widget_impl(Scroll))]
pub fn clip_to_viewport(clip: impl IntoVar<bool>) {}

/// Properties that define scroll units.
#[widget_mixin]
pub struct ScrollUnitsMix<P>(P);

/// Properties that defines the scrollbar widget used in scrolls.
#[widget_mixin]
pub struct ScrollbarFnMix<P>(P);

fn on_build(wgt: &mut WidgetBuilding) {
    let mode = wgt.capture_var_or_else(property_id!(mode), || ScrollMode::ZOOM);

    let child_align = wgt.capture_var_or_else(property_id!(child_align), || Align::CENTER);
    let clip_to_viewport = wgt.capture_var_or_default(property_id!(clip_to_viewport));

    wgt.push_intrinsic(
        NestGroup::CHILD_CONTEXT,
        "scroll_node",
        clmv!(mode, |child| {
            let child = scroll_node(child, mode, child_align, clip_to_viewport);
            nodes::overscroll_node(child)
        }),
    );

    wgt.push_intrinsic(NestGroup::EVENT, "commands", |child| {
        let child = nodes::scroll_to_node(child);
        let child = nodes::scroll_commands_node(child);
        let child = nodes::page_commands_node(child);
        let child = nodes::scroll_to_edge_commands_node(child);
        let child = nodes::scroll_touch_node(child);
        let child = nodes::zoom_commands_node(child);
        nodes::scroll_wheel_node(child)
    });

    wgt.push_intrinsic(NestGroup::CONTEXT, "context", move |child| {
        let child = with_context_var(child, SCROLL_VIEWPORT_SIZE_VAR, var(PxSize::zero()));
        let child = with_context_var(child, SCROLL_CONTENT_SIZE_VAR, var(PxSize::zero()));

        let child = with_context_var(child, SCROLL_VERTICAL_RATIO_VAR, var(0.fct()));
        let child = with_context_var(child, SCROLL_HORIZONTAL_RATIO_VAR, var(0.fct()));

        let child = with_context_var(child, SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR, var(false));
        let child = with_context_var(child, SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR, var(false));

        let child = SCROLL.config_node(child);

        let child = with_context_var(child, SCROLL_VERTICAL_OFFSET_VAR, var(0.fct()));
        let child = with_context_var(child, SCROLL_HORIZONTAL_OFFSET_VAR, var(0.fct()));

        let child = with_context_var(child, OVERSCROLL_VERTICAL_OFFSET_VAR, var(0.fct()));
        let child = with_context_var(child, OVERSCROLL_HORIZONTAL_OFFSET_VAR, var(0.fct()));

        let child = with_context_var(child, SCROLL_SCALE_VAR, var(1.fct()));

        with_context_var(child, SCROLL_MODE_VAR, mode)
    });
}

fn scroll_node(
    child: impl UiNode,
    mode: impl IntoVar<ScrollMode>,
    child_align: impl IntoVar<Align>,
    clip_to_viewport: impl IntoVar<bool>,
) -> impl UiNode {
    // # Layout
    //
    // +-----------------+---+
    // |                 |   |
    // | 0 - viewport    | 1 | - v_scrollbar
    // |                 |   |
    // +-----------------+---+
    // | 2 - h_scrollbar | 3 | - scrollbar_joiner
    // +-----------------+---+
    let children = ui_vec![
        clip_to_bounds(
            nodes::viewport(child, mode.into_var(), child_align).instrument("viewport"),
            clip_to_viewport.into_var()
        ),
        nodes::v_scrollbar_presenter(),
        nodes::h_scrollbar_presenter(),
        nodes::scrollbar_joiner_presenter(),
    ];

    let mut viewport = PxSize::zero();
    let mut joiner = PxSize::zero();
    let spatial_id = SpatialFrameId::new_unique();

    match_node_list(children, move |children, op| match op {
        UiNodeOp::Measure { wm, desired_size } => {
            let constraints = LAYOUT.constraints();
            *desired_size = if constraints.is_fill_max().all() {
                children.delegated();
                constraints.fill_size()
            } else {
                let size = children.with_node(0, |n| n.measure(wm));
                constraints.clamp_size(size)
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            // scrollbars
            let c = LAYOUT.constraints().with_new_min(Px(0), Px(0));
            {
                joiner.width = LAYOUT.with_constraints(c.with_fill(false, true), || {
                    children.with_node(1, |n| n.measure(&mut wl.to_measure(None))).width
                });
                joiner.height = LAYOUT.with_constraints(c.with_fill(true, false), || {
                    children.with_node(2, |n| n.measure(&mut wl.to_measure(None))).height
                });
            }
            joiner.width = LAYOUT.with_constraints(c.with_fill(false, true).with_less_y(joiner.height), || {
                children.with_node(1, |n| n.layout(wl)).width
            });
            joiner.height = LAYOUT.with_constraints(c.with_fill(true, false).with_less_x(joiner.width), || {
                children.with_node(2, |n| n.layout(wl)).height
            });

            // joiner
            let _ = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(joiner), || children.with_node(3, |n| n.layout(wl)));

            // viewport
            let mut vp = LAYOUT.with_constraints(c.with_less_size(joiner), || children.with_node(0, |n| n.layout(wl)));

            // arrange
            let fs = vp + joiner;
            let content_size = SCROLL_CONTENT_SIZE_VAR.get();

            if content_size.height > fs.height {
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set(true).unwrap();
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set(content_size.width > vp.width).unwrap();
            } else if content_size.width > fs.width {
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set(true).unwrap();
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set(content_size.height > vp.height).unwrap();
            } else {
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set(false).unwrap();
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set(false).unwrap();
            }

            // collapse scrollbars if they take more the 1/3 of the total area.
            if vp.width < joiner.width * 3.0.fct() {
                vp.width += joiner.width;
                joiner.width = Px(0);
            }
            if vp.height < joiner.height * 3.0.fct() {
                vp.height += joiner.height;
                joiner.height = Px(0);
            }

            if vp != viewport {
                viewport = vp;
                WIDGET.render();
            }

            *final_size = viewport + joiner;
        }

        UiNodeOp::Render { frame } => {
            children.with_node(0, |n| n.render(frame));

            if joiner.width > Px(0) {
                let transform = PxTransform::from(PxVector::new(viewport.width, Px(0)));
                frame.push_reference_frame((spatial_id, 1).into(), FrameValue::Value(transform), true, false, |frame| {
                    children.with_node(1, |n| n.render(frame));
                });
            }

            if joiner.height > Px(0) {
                let transform = PxTransform::from(PxVector::new(Px(0), viewport.height));
                frame.push_reference_frame((spatial_id, 2).into(), FrameValue::Value(transform), true, false, |frame| {
                    children.with_node(2, |n| n.render(frame));
                });
            }

            if joiner.width > Px(0) && joiner.height > Px(0) {
                let transform = PxTransform::from(viewport.to_vector());
                frame.push_reference_frame((spatial_id, 3).into(), FrameValue::Value(transform), true, false, |frame| {
                    children.with_node(3, |n| n.render(frame));
                });
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            children.with_node(0, |n| n.render_update(update));

            if joiner.width > Px(0) {
                let transform = PxTransform::from(PxVector::new(viewport.width, Px(0)));
                update.with_transform_value(&transform, |update| {
                    children.with_node(1, |n| n.render_update(update));
                });
            }

            if joiner.height > Px(0) {
                let transform = PxTransform::from(PxVector::new(Px(0), viewport.height));
                update.with_transform_value(&transform, |update| {
                    children.with_node(2, |n| n.render_update(update));
                });
            }

            if joiner.width > Px(0) && joiner.height > Px(0) {
                let transform = PxTransform::from(viewport.to_vector());
                update.with_transform_value(&transform, |update| {
                    children.with_node(3, |n| n.render_update(update));
                });
            }
        }
        _ => {}
    })
}

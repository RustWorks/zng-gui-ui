//! UI nodes used for building the scroll widget.
//!

use crate::prelude::new_widget::*;

use crate::core::{
    focus::FOCUS_CHANGED_EVENT,
    gradient::{ExtendMode, RenderGradientStop},
    mouse::{MouseScrollDelta, MOUSE_WHEEL_EVENT},
    touch::{TouchPhase, TOUCH_TRANSFORM_EVENT},
};

use super::commands::*;
use super::scroll_properties::*;
use super::scrollbar::Orientation;
use super::types::*;

/// The actual content presenter.
pub fn viewport(child: impl UiNode, mode: impl IntoVar<ScrollMode>, child_align: impl IntoVar<Align>) -> impl UiNode {
    let mode = mode.into_var();
    let child_align = child_align.into_var();
    let binding_key = FrameValueKey::new_unique();

    let mut viewport_size = PxSize::zero();
    let mut viewport_unit = PxSize::zero();
    let mut content_offset = PxVector::zero();
    let mut content_scale = 1.fct();
    let mut auto_hide_extra = PxSideOffsets::zero();
    let mut last_render_offset = PxVector::zero();
    let mut scroll_info = None;
    let mut scroll_info = move || {
        scroll_info
            .get_or_insert_with(|| WIDGET.info().meta().get_clone(&SCROLL_INFO_ID).unwrap())
            .clone()
    };

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&mode)
                .sub_var_layout(&SCROLL_VERTICAL_OFFSET_VAR)
                .sub_var_layout(&SCROLL_HORIZONTAL_OFFSET_VAR)
                .sub_var_layout(&SCROLL_SCALE_VAR)
                .sub_var_layout(&child_align);
        }

        UiNodeOp::Measure { wm, desired_size } => {
            let constraints = LAYOUT.constraints();
            if constraints.is_fill_max().all() {
                *desired_size = constraints.fill_size();
                child.delegated();
                return;
            }

            let mode = mode.get();
            let child_align = child_align.get();

            let viewport_unit = constraints.fill_size();
            let define_vp_unit = DEFINE_VIEWPORT_UNIT_VAR.get() // requested
                && viewport_unit.width > Px(0) // and has fill-size
                && viewport_unit.height > Px(0)
                && constraints.max_size() == Some(viewport_unit); // that is not just min size.

            let mut content_size = LAYOUT.with_constraints(
                {
                    let mut c = child_align.child_constraints(constraints.with_new_min_size(viewport_unit));
                    if mode.contains(ScrollMode::VERTICAL) {
                        c = c.with_unbounded_y();
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        c = c.with_unbounded_x();
                    }
                    c
                },
                || {
                    if define_vp_unit {
                        LAYOUT.with_viewport(viewport_unit, || child.measure(wm))
                    } else {
                        child.measure(wm)
                    }
                },
            );

            if mode.contains(ScrollMode::ZOOM) {
                let scale = SCROLL_SCALE_VAR.get();
                content_size.width *= scale;
                content_size.height *= scale;
            }

            *desired_size = constraints.fill_size_or(content_size);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let mode = mode.get();
            let child_align = child_align.get();

            let constraints = LAYOUT.constraints();
            let vp_unit = constraints.fill_size();
            let define_vp_unit = DEFINE_VIEWPORT_UNIT_VAR.get() // requested
                && vp_unit.width > Px(0) // and has fill-size
                && vp_unit.height > Px(0)
                && constraints.max_size() == Some(vp_unit); // that is not just min size.
                
            let joiner_size = scroll_info().joiner_size();

            let mut content_size = LAYOUT.with_constraints(
                {
                    let mut c = child_align.child_constraints(constraints.with_new_min_size(vp_unit + joiner_size));
                    if mode.contains(ScrollMode::VERTICAL) {
                        c = c.with_unbounded_y();
                    }
                    if mode.contains(ScrollMode::HORIZONTAL) {
                        c = c.with_unbounded_x();
                    }
                    c
                },
                || {
                    if define_vp_unit {
                        LAYOUT.with_viewport(vp_unit, || {
                            viewport_unit = vp_unit;
                            child.layout(wl)
                        })
                    } else {
                        child.layout(wl)
                    }
                },
            );
            if mode.contains(ScrollMode::ZOOM) {
                content_scale = SCROLL_SCALE_VAR.get();
                content_size.width *= content_scale;
                content_size.height *= content_scale;
            } else {
                content_scale = 1.fct();
            }

            let vp_size = constraints.fill_size_or(content_size);
            if viewport_size != vp_size {
                viewport_size = vp_size;
                SCROLL_VIEWPORT_SIZE_VAR.set(vp_size).unwrap();
                WIDGET.render();
            }

            auto_hide_extra = LAYOUT.with_viewport(vp_size, || {
                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(vp_size), || {
                    AUTO_HIDE_EXTRA_VAR.layout_dft(PxSideOffsets::new(vp_size.height, vp_size.width, vp_size.height, vp_size.width))
                })
            });
            auto_hide_extra.top = auto_hide_extra.top.max(Px(0));
            auto_hide_extra.right = auto_hide_extra.right.max(Px(0));
            auto_hide_extra.bottom = auto_hide_extra.bottom.max(Px(0));
            auto_hide_extra.left = auto_hide_extra.left.max(Px(0));

            scroll_info().set_viewport_size(vp_size);

            let align_offset = child_align.child_offset(content_size, viewport_size, LAYOUT.direction());

            let mut ct_offset = PxVector::zero();

            if mode.contains(ScrollMode::VERTICAL) && content_size.height > vp_size.height {
                let v_offset = SCROLL_VERTICAL_OFFSET_VAR.get();
                ct_offset.y = (viewport_size.height - content_size.height) * v_offset;
            } else {
                ct_offset.y = align_offset.y;
            }
            if mode.contains(ScrollMode::HORIZONTAL) && content_size.width > vp_size.width {
                let h_offset = SCROLL_HORIZONTAL_OFFSET_VAR.get();
                ct_offset.x = (viewport_size.width - content_size.width) * h_offset;
            } else {
                ct_offset.x = align_offset.x;
            }

            if ct_offset != content_offset {
                content_offset = ct_offset;

                // check if scrolled using only `render_update` to the end of the `auto_hide_extra` space.
                let update_only_offset = (last_render_offset - content_offset).abs();
                const OFFSET_EXTRA: Px = Px(20); // give a margin of error for widgets that render outside bounds.
                let mut need_full_render = if update_only_offset.y < Px(0) {
                    update_only_offset.y.abs() + OFFSET_EXTRA > auto_hide_extra.top
                } else {
                    update_only_offset.y + OFFSET_EXTRA > auto_hide_extra.bottom
                };
                if !need_full_render {
                    need_full_render = if update_only_offset.x < Px(0) {
                        update_only_offset.x.abs() + OFFSET_EXTRA > auto_hide_extra.left
                    } else {
                        update_only_offset.x + OFFSET_EXTRA > auto_hide_extra.right
                    };
                }

                if need_full_render {
                    // need to render more widgets, `auto_hide_extra` was reached using only `render_update`
                    WIDGET.render();
                } else {
                    WIDGET.render_update();
                }
            }

            let v_ratio = viewport_size.height.0 as f32 / content_size.height.0 as f32;
            let h_ratio = viewport_size.width.0 as f32 / content_size.width.0 as f32;

            SCROLL_VERTICAL_RATIO_VAR.set(v_ratio.fct()).unwrap();
            SCROLL_HORIZONTAL_RATIO_VAR.set(h_ratio.fct()).unwrap();
            SCROLL_CONTENT_SIZE_VAR.set(content_size).unwrap();

            let full_size = viewport_size + joiner_size;

            if content_size.height > full_size.height {
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set(true).unwrap();
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR
                    .set(content_size.width > viewport_size.width)
                    .unwrap();
            } else if content_size.width > full_size.width {
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set(true).unwrap();
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR
                    .set(content_size.height > viewport_size.height)
                    .unwrap();
            } else {
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set(false).unwrap();
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set(false).unwrap();
            }

            *final_size = viewport_size;
        }
        UiNodeOp::Render { frame } => {
            scroll_info().set_viewport_transform(*frame.transform());
            last_render_offset = content_offset;

            let mut culling_rect = PxBox::from_size(viewport_size);
            culling_rect.min.y -= auto_hide_extra.top;
            culling_rect.max.x += auto_hide_extra.right;
            culling_rect.max.y += auto_hide_extra.bottom;
            culling_rect.min.x -= auto_hide_extra.left;
            let culling_rect = frame.transform().outer_transformed(culling_rect).unwrap_or(culling_rect).to_rect();

            let transform = if content_scale != 1.fct() {
                PxTransform::scale(content_scale.0, content_scale.0).then_translate(content_offset.cast())
            } else {
                content_offset.into()
            };
            frame.push_reference_frame(binding_key.into(), binding_key.bind(transform, true), true, false, |frame| {
                frame.with_auto_hide_rect(culling_rect, |frame| {
                    child.render(frame);
                });
            });
        }
        UiNodeOp::RenderUpdate { update } => {
            scroll_info().set_viewport_transform(*update.transform());

            let transform = if content_scale != 1.fct() {
                PxTransform::scale(content_scale.0, content_scale.0).then_translate(content_offset.cast())
            } else {
                content_offset.into()
            };
            update.with_transform(binding_key.update(transform, true), false, |update| {
                child.render_update(update);
            });
        }
        _ => {}
    })
}

/// Create a node that generates and presents the [vertical scrollbar].
///
/// [vertical scrollbar]: VERTICAL_SCROLLBAR_FN_VAR
pub fn v_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(VERTICAL_SCROLLBAR_FN_VAR, Orientation::Vertical)
}

/// Create a node that generates and presents the [horizontal scrollbar].
///
/// [horizontal scrollbar]: HORIZONTAL_SCROLLBAR_FN_VAR
pub fn h_scrollbar_presenter() -> impl UiNode {
    scrollbar_presenter(HORIZONTAL_SCROLLBAR_FN_VAR, Orientation::Horizontal)
}

fn scrollbar_presenter(var: impl IntoVar<WidgetFn<ScrollBarArgs>>, orientation: Orientation) -> impl UiNode {
    crate::widgets::presenter(ScrollBarArgs::new(orientation), var)
}

/// Create a node that generates and presents the [scrollbar joiner].
///
/// [scrollbar joiner]: SCROLLBAR_JOINER_FN_VAR
pub fn scrollbar_joiner_presenter() -> impl UiNode {
    crate::widgets::presenter((), SCROLLBAR_JOINER_FN_VAR)
}

/// Create a node that implements [`SCROLL_UP_CMD`], [`SCROLL_DOWN_CMD`],
/// [`SCROLL_LEFT_CMD`] and [`SCROLL_RIGHT_CMD`] scoped on the widget.
pub fn scroll_commands_node(child: impl UiNode) -> impl UiNode {
    let mut up = CommandHandle::dummy();
    let mut down = CommandHandle::dummy();
    let mut left = CommandHandle::dummy();
    let mut right = CommandHandle::dummy();

    let mut layout_line = PxVector::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&VERTICAL_LINE_UNIT_VAR)
                .sub_var_layout(&HORIZONTAL_LINE_UNIT_VAR);

            let scope = WIDGET.id();

            up = SCROLL_UP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            down = SCROLL_DOWN_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            left = SCROLL_LEFT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            right = SCROLL_RIGHT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            up = CommandHandle::dummy();
            down = CommandHandle::dummy();
            left = CommandHandle::dummy();
            right = CommandHandle::dummy();
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            if VERTICAL_LINE_UNIT_VAR.is_new() || HORIZONTAL_LINE_UNIT_VAR.is_new() {
                WIDGET.layout();
            }
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = SCROLL_UP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&up, |_| {
                    let mut offset = -layout_line.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&down, |_| {
                    let mut offset = layout_line.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&left, |_| {
                    let mut offset = -layout_line.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = SCROLL_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&right, |_| {
                    let mut offset = layout_line.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            up.set_enabled(SCROLL.can_scroll_up());
            down.set_enabled(SCROLL.can_scroll_down());
            left.set_enabled(SCROLL.can_scroll_left());
            right.set_enabled(SCROLL.can_scroll_right());

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                layout_line = PxVector::new(
                    HORIZONTAL_LINE_UNIT_VAR.layout_dft_x(Px(20)),
                    VERTICAL_LINE_UNIT_VAR.layout_dft_y(Px(20)),
                );
            });
        }
        _ => {}
    })
}

/// Create a node that implements [`PAGE_UP_CMD`], [`PAGE_DOWN_CMD`],
/// [`PAGE_LEFT_CMD`] and [`PAGE_RIGHT_CMD`] scoped on the widget.
pub fn page_commands_node(child: impl UiNode) -> impl UiNode {
    let mut up = CommandHandle::dummy();
    let mut down = CommandHandle::dummy();
    let mut left = CommandHandle::dummy();
    let mut right = CommandHandle::dummy();

    let mut layout_page = PxVector::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&VERTICAL_PAGE_UNIT_VAR)
                .sub_var_layout(&HORIZONTAL_PAGE_UNIT_VAR);

            let scope = WIDGET.id();

            up = PAGE_UP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            down = PAGE_DOWN_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            left = PAGE_LEFT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            right = PAGE_RIGHT_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            up = CommandHandle::dummy();
            down = CommandHandle::dummy();
            left = CommandHandle::dummy();
            right = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = PAGE_UP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&up, |_| {
                    let mut offset = -layout_page.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_DOWN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&down, |_| {
                    let mut offset = layout_page.y;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_vertical_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_LEFT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&left, |_| {
                    let mut offset = -layout_page.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            } else if let Some(args) = PAGE_RIGHT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&right, |_| {
                    let mut offset = layout_page.x;
                    let args = ScrollRequest::from_args(args).unwrap_or_default();
                    if args.alternate {
                        offset *= ALT_FACTOR_VAR.get();
                    }
                    SCROLL.scroll_horizontal_clamp(ScrollFrom::VarTarget(offset), args.clamp.0, args.clamp.1);
                });
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            up.set_enabled(SCROLL.can_scroll_up());
            down.set_enabled(SCROLL.can_scroll_down());
            left.set_enabled(SCROLL.can_scroll_left());
            right.set_enabled(SCROLL.can_scroll_right());

            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                layout_page = PxVector::new(
                    HORIZONTAL_PAGE_UNIT_VAR.layout_dft_x(Px(20)),
                    VERTICAL_PAGE_UNIT_VAR.layout_dft_y(Px(20)),
                );
            });
        }
        _ => {}
    })
}

/// Create a node that implements [`SCROLL_TO_TOP_CMD`], [`SCROLL_TO_BOTTOM_CMD`],
/// [`SCROLL_TO_LEFTMOST_CMD`] and [`SCROLL_TO_RIGHTMOST_CMD`] scoped on the widget.
pub fn scroll_to_edge_commands_node(child: impl UiNode) -> impl UiNode {
    let mut top = CommandHandle::dummy();
    let mut bottom = CommandHandle::dummy();
    let mut leftmost = CommandHandle::dummy();
    let mut rightmost = CommandHandle::dummy();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            let scope = WIDGET.id();

            top = SCROLL_TO_TOP_CMD.scoped(scope).subscribe(SCROLL.can_scroll_up());
            bottom = SCROLL_TO_BOTTOM_CMD.scoped(scope).subscribe(SCROLL.can_scroll_down());
            leftmost = SCROLL_TO_LEFTMOST_CMD.scoped(scope).subscribe(SCROLL.can_scroll_left());
            rightmost = SCROLL_TO_RIGHTMOST_CMD.scoped(scope).subscribe(SCROLL.can_scroll_right());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            top = CommandHandle::dummy();
            bottom = CommandHandle::dummy();
            leftmost = CommandHandle::dummy();
            rightmost = CommandHandle::dummy();
        }
        UiNodeOp::Layout { .. } => {
            top.set_enabled(SCROLL.can_scroll_up());
            bottom.set_enabled(SCROLL.can_scroll_down());
            leftmost.set_enabled(SCROLL.can_scroll_left());
            rightmost.set_enabled(SCROLL.can_scroll_right());
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = SCROLL_TO_TOP_CMD.scoped(scope).on(update) {
                args.handle_enabled(&top, |_| {
                    SCROLL.chase_vertical(|_| 0.fct());
                });
            } else if let Some(args) = SCROLL_TO_BOTTOM_CMD.scoped(scope).on(update) {
                args.handle_enabled(&bottom, |_| {
                    SCROLL.chase_vertical(|_| 1.fct());
                });
            } else if let Some(args) = SCROLL_TO_LEFTMOST_CMD.scoped(scope).on(update) {
                args.handle_enabled(&leftmost, |_| {
                    SCROLL.chase_horizontal(|_| 0.fct());
                });
            } else if let Some(args) = SCROLL_TO_RIGHTMOST_CMD.scoped(scope).on(update) {
                args.handle_enabled(&rightmost, |_| {
                    SCROLL.chase_horizontal(|_| 1.fct());
                });
            }
        }
        _ => {}
    })
}

/// Create a node that implements [`ZOOM_IN_CMD`], [`ZOOM_OUT_CMD`],
/// and [`ZOOM_RESET_CMD`] scoped on the widget.
pub fn zoom_commands_node(child: impl UiNode) -> impl UiNode {
    let mut zoom_in = CommandHandle::dummy();
    let mut zoom_out = CommandHandle::dummy();
    let mut zoom_reset = CommandHandle::dummy();

    let mut scale_delta = 0.fct();
    let mut origin = Point::default();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            let scope = WIDGET.id();

            zoom_in = ZOOM_IN_CMD.scoped(scope).subscribe(SCROLL.can_zoom_in());
            zoom_out = ZOOM_OUT_CMD.scoped(scope).subscribe(SCROLL.can_zoom_out());
            zoom_reset = ZOOM_RESET_CMD.scoped(scope).subscribe(SCROLL.zoom_scale().get() != 1.fct());
        }
        UiNodeOp::Deinit => {
            child.deinit();

            zoom_in = CommandHandle::dummy();
            zoom_out = CommandHandle::dummy();
            zoom_reset = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            let scope = WIDGET.id();

            if let Some(args) = ZOOM_IN_CMD.scoped(scope).on(update) {
                args.handle_enabled(&zoom_in, |args| {
                    origin = args.param::<Point>().cloned().unwrap_or_default();
                    scale_delta += ZOOM_WHEEL_UNIT_VAR.get();

                    WIDGET.layout();
                });
            } else if let Some(args) = ZOOM_OUT_CMD.scoped(scope).on(update) {
                args.handle_enabled(&zoom_out, |_| {
                    origin = args.param::<Point>().cloned().unwrap_or_default();
                    scale_delta -= ZOOM_WHEEL_UNIT_VAR.get();

                    WIDGET.layout();
                });
            } else if let Some(args) = ZOOM_RESET_CMD.scoped(scope).on(update) {
                args.handle_enabled(&zoom_reset, |_| {
                    SCROLL.chase_zoom(|_| 1.fct());
                    scale_delta = 0.fct();
                });
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            zoom_in.set_enabled(SCROLL.can_zoom_in());
            zoom_out.set_enabled(SCROLL.can_zoom_out());
            zoom_reset.set_enabled(SCROLL.zoom_scale().get() != 1.fct());

            if scale_delta != 0.fct() {
                let scroll_info = WIDGET.info().scroll_info().unwrap();
                let viewport_size = scroll_info.viewport_size();

                let default = PxPoint::new(
                    Px(0),
                    match LAYOUT.direction() {
                        LayoutDirection::LTR => Px(0),
                        LayoutDirection::RTL => viewport_size.width,
                    },
                );
                let center_in_viewport =
                    LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport_size), || origin.layout_dft(default));

                SCROLL.zoom(|f| f + scale_delta, center_in_viewport);
                scale_delta = 0.fct();
            }
        }
        _ => {}
    })
}

/// Create a node that implements [`SCROLL_TO_CMD`] scoped on the widget and scroll to focused.
pub fn scroll_to_node(child: impl UiNode) -> impl UiNode {
    let mut _handle = CommandHandle::dummy();
    let mut scroll_to = None;
    let mut scroll_to_from_cmd = false;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            _handle = SCROLL_TO_CMD.scoped(WIDGET.id()).subscribe(true);
        }
        UiNodeOp::Deinit => {
            _handle = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            let self_id = WIDGET.id();
            if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                if let Some(path) = &args.new_focus {
                    if (scroll_to.is_none() || !scroll_to_from_cmd) && path.contains(self_id) && path.widget_id() != self_id {
                        // focus move inside.
                        if let Some(mode) = SCROLL_TO_FOCUSED_MODE_VAR.get() {
                            if SCROLL.can_scroll_vertical() || SCROLL.can_scroll_horizontal() {
                                let tree = WINDOW.info();
                                if let Some(mut target) = tree.get(path.widget_id()) {
                                    for a in target.ancestors() {
                                        if a.is_scroll() {
                                            if a.id() == self_id {
                                                break;
                                            } else {
                                                // actually focus move inside an inner scroll,
                                                // the inner-most scroll scrolls to the target,
                                                // the outer scrolls scroll to the child scroll.
                                                target = a;
                                            }
                                        }
                                    }

                                    scroll_to = Some((target.bounds_info(), mode, None));
                                    WIDGET.layout();
                                }
                            }
                        }
                    }
                }
            } else if let Some(args) = SCROLL_TO_CMD.scoped(self_id).on(update) {
                // event send to us and enabled
                if let Some(request) = ScrollToRequest::from_args(args) {
                    // has unhandled request
                    let tree = WINDOW.info();
                    if let Some(target) = tree.get(request.widget_id) {
                        // target exists
                        if let Some(us) = target.ancestors().find(|w| w.id() == self_id) {
                            // target is descendant
                            if us.is_scroll() {
                                // we are a scroll.

                                let bounds = target.bounds_info();
                                let mode = request.mode;

                                // will scroll on the next arrange.
                                scroll_to = Some((bounds, mode, request.zoom));
                                scroll_to_from_cmd = true;
                                WIDGET.layout();

                                args.propagation().stop();
                            }
                        }
                    }
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            if let Some((bounds, mode, mut zoom)) = scroll_to.take() {
                if let Some(s) = &mut zoom {
                    *s = s.clamp(MIN_ZOOM_VAR.get(), MAX_ZOOM_VAR.get());
                }

                scroll_to_from_cmd = false;
                let tree = WINDOW.info();
                let us = tree.get(WIDGET.id()).unwrap();

                if let Some(scroll_info) = us.scroll_info() {
                    let mut target_bounds = bounds.inner_bounds();
                    // remove viewport transform
                    target_bounds = scroll_info
                        .viewport_transform()
                        .inverse()
                        .and_then(|t| t.outer_transformed(target_bounds.to_box2d()))
                        .map(|b| b.to_rect())
                        .unwrap_or(target_bounds);

                    let target_bounds_in_content = target_bounds;

                    // remove offset
                    let rendered_offset = SCROLL.rendered_content().origin.to_vector();
                    target_bounds.origin -= rendered_offset;

                    // replace scale
                    let rendered_scale = SCROLL.rendered_zoom_scale();
                    if let Some(s) = zoom {
                        let s = s / rendered_scale;
                        target_bounds.origin *= s;
                        target_bounds.size *= s;
                    }
                    // target bounds is in the content space at future scale

                    let viewport_size = scroll_info.viewport_size();

                    let mut offset = PxVector::zero();

                    match mode {
                        ScrollToMode::Minimal { margin } => {
                            // add minimal margin
                            let margin = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(target_bounds.size), || margin.layout());
                            target_bounds.origin.x -= margin.left;
                            target_bounds.origin.y -= margin.top;
                            target_bounds.size.width += margin.horizontal();
                            target_bounds.size.height += margin.vertical();
                            let target_bounds = target_bounds;

                            // vertical scroll
                            if target_bounds.size.height < viewport_size.height {
                                if target_bounds.origin.y < Px(0) {
                                    // scroll up
                                    offset.y = target_bounds.origin.y;
                                } else if target_bounds.origin.y > viewport_size.height {
                                    // scroll down
                                    offset.y = target_bounds.max_y() - viewport_size.height;
                                }
                            } else {
                                // center
                                offset.y = viewport_size.height / Px(2) - target_bounds.center().y;
                            };

                            // horizontal scroll
                            if target_bounds.size.width < viewport_size.width {
                                if target_bounds_in_content.origin.x < Px(0) {
                                    // scroll left
                                    offset.x = target_bounds.origin.x;
                                } else if target_bounds_in_content.origin.x > viewport_size.width {
                                    // scroll right
                                    offset.x = target_bounds.max_x() - viewport_size.width;
                                }
                            } else {
                                // center
                                offset.x = viewport_size.width / Px(2) - target_bounds.center().x;
                            };
                        }
                        ScrollToMode::Center {
                            widget_point,
                            scroll_point,
                        } => {
                            // find the two points
                            let default = (target_bounds.size / Px(2)).to_vector().to_point();
                            let widget_point = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(target_bounds.size), || {
                                widget_point.layout_dft(default)
                            });
                            let default = (viewport_size / Px(2)).to_vector().to_point();
                            let scroll_point =
                                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport_size), || scroll_point.layout_dft(default));

                            offset = (widget_point + target_bounds.origin.to_vector()) - scroll_point;
                        }
                    }

                    // scroll range
                    let mut content_size = SCROLL.content_size().get();
                    if let Some(scale) = zoom {
                        content_size *= scale / rendered_scale;
                    }
                    let max_scroll = content_size - viewport_size;

                    // apply
                    if let Some(scale) = zoom {
                        SCROLL.chase_zoom(|_| scale);
                    }
                    if offset.y != Px(0) && max_scroll.height > Px(0) {
                        let offset_y = offset.y.0 as f32 / max_scroll.height.0 as f32;
                        SCROLL.chase_vertical(|_| offset_y.fct());
                    }
                    if offset.x != Px(0) && max_scroll.width > Px(0) {
                        let offset_x = offset.x.0 as f32 / max_scroll.width.0 as f32;
                        SCROLL.chase_horizontal(|_| offset_x.fct());
                    }
                }
            }
        }
        _ => {}
    })
}

/// Create a node that implements scroll by touch gestures for the widget.
pub fn scroll_touch_node(child: impl UiNode) -> impl UiNode {
    let mut applied_offset = PxVector::zero();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&TOUCH_TRANSFORM_EVENT);
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = TOUCH_TRANSFORM_EVENT.on_unhandled(update) {
                let mut pending_translate = true;

                if SCROLL.mode().get().contains(ScrollMode::ZOOM) {
                    let f = args.scale();
                    if f != 1.fct() {
                        let center = WIDGET
                            .info()
                            .scroll_info()
                            .unwrap()
                            .viewport_transform()
                            .inverse()
                            .and_then(|t| t.transform_point_f32(args.latest_info.center))
                            .unwrap_or(args.latest_info.center);

                        SCROLL.zoom_touch(args.phase, f, center);
                        pending_translate = false;
                    }
                }

                if pending_translate {
                    let new_offset = args.translation().cast::<Px>();
                    let delta = new_offset - applied_offset;
                    applied_offset = new_offset;

                    if delta.y != Px(0) {
                        SCROLL.scroll_vertical_touch(-delta.y);
                    }
                    if delta.x != Px(0) {
                        SCROLL.scroll_horizontal_touch(-delta.x);
                    }
                }

                match args.phase {
                    TouchPhase::Start => {}
                    TouchPhase::Move => {}
                    TouchPhase::End => {
                        // TODO inertia
                        applied_offset = PxVector::zero();

                        SCROLL.clear_vertical_overscroll();
                        SCROLL.clear_horizontal_overscroll();
                    }
                    TouchPhase::Cancel => {
                        applied_offset = PxVector::zero();

                        SCROLL.clear_vertical_overscroll();
                        SCROLL.clear_horizontal_overscroll();
                    }
                }
            }
        }
        _ => {}
    })
}

/// Create a node that implements scroll-wheel handling for the widget.
pub fn scroll_wheel_node(child: impl UiNode) -> impl UiNode {
    let mut offset = Vector::zero();
    let mut scale_delta = 0.fct();
    let mut scale_position = DipPoint::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&MOUSE_WHEEL_EVENT);
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = MOUSE_WHEEL_EVENT.on_unhandled(update) {
                if let Some(delta) = args.scroll_delta(ALT_FACTOR_VAR.get()) {
                    match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            let scroll_x = if x > 0.0 {
                                SCROLL.can_scroll_left()
                            } else if x < 0.0 {
                                SCROLL.can_scroll_right()
                            } else {
                                false
                            };
                            let scroll_y = if y > 0.0 {
                                SCROLL.can_scroll_up()
                            } else if y < 0.0 {
                                SCROLL.can_scroll_down()
                            } else {
                                false
                            };

                            if scroll_x || scroll_y {
                                args.propagation().stop();

                                if scroll_x {
                                    offset.x -= HORIZONTAL_WHEEL_UNIT_VAR.get() * x.fct();
                                }
                                if scroll_y {
                                    offset.y -= VERTICAL_WHEEL_UNIT_VAR.get() * y.fct();
                                }
                            }
                        }
                        MouseScrollDelta::PixelDelta(x, y) => {
                            let scroll_x = if x > 0.0 {
                                SCROLL.can_scroll_left()
                            } else if x < 0.0 {
                                SCROLL.can_scroll_right()
                            } else {
                                false
                            };
                            let scroll_y = if y > 0.0 {
                                SCROLL.can_scroll_up()
                            } else if y < 0.0 {
                                SCROLL.can_scroll_down()
                            } else {
                                false
                            };

                            if scroll_x || scroll_y {
                                args.propagation().stop();

                                if scroll_x {
                                    offset.x -= x.px();
                                }
                                if scroll_y {
                                    offset.y -= y.px();
                                }
                            }
                        }
                    }

                    WIDGET.layout();
                } else if let Some(delta) = args.zoom_delta() {
                    if !SCROLL_MODE_VAR.get().contains(ScrollMode::ZOOM) {
                        return;
                    }

                    let delta = match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            if y.abs() > x.abs() {
                                ZOOM_WHEEL_UNIT_VAR.get() * y.fct()
                            } else {
                                ZOOM_WHEEL_UNIT_VAR.get() * x.fct()
                            }
                        }
                        MouseScrollDelta::PixelDelta(x, y) => {
                            if y.abs() > x.abs() {
                                // 1% per "pixel".
                                0.001.fct() * y.fct()
                            } else {
                                0.001.fct() * x.fct()
                            }
                        }
                    };

                    let apply = if delta > 0.fct() {
                        SCROLL.can_zoom_in()
                    } else if delta < 0.fct() {
                        SCROLL.can_zoom_out()
                    } else {
                        false
                    };

                    if apply {
                        scale_delta += delta;
                        scale_position = args.position;
                        WIDGET.layout();
                    }
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = child.layout(wl);

            if offset != Vector::zero() {
                let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();

                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport), || {
                    let o = offset.layout_dft(viewport.to_vector());
                    offset = Vector::zero();

                    if o.y != Px(0) {
                        SCROLL.scroll_vertical(ScrollFrom::VarTarget(o.y));
                    }
                    if o.x != Px(0) {
                        SCROLL.scroll_horizontal(ScrollFrom::VarTarget(o.x));
                    }
                });
            }

            if scale_delta != 0.fct() {
                let scroll_info = WIDGET.info().scroll_info().unwrap();
                let default = scale_position.to_px(LAYOUT.scale_factor().0);
                let default = scroll_info
                    .viewport_transform()
                    .inverse()
                    .and_then(|t| t.transform_point(default))
                    .unwrap_or(default);

                let viewport_size = scroll_info.viewport_size();
                let center_in_viewport = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(viewport_size), || {
                    ZOOM_WHEEL_ORIGIN_VAR.layout_dft(default)
                });

                SCROLL.zoom(|f| f + scale_delta, center_in_viewport);
                scale_delta = 0.fct();
            }
        }
        _ => {}
    })
}

/// Overscroll visual indicator.
pub fn overscroll_node(child: impl UiNode) -> impl UiNode {
    let mut v_rect = PxRect::zero();
    let mut v_center = PxPoint::zero();
    let mut v_radius_w = Px(0);

    let mut h_rect = PxRect::zero();
    let mut h_center = PxPoint::zero();
    let mut h_radius_h = Px(0);

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&OVERSCROLL_VERTICAL_OFFSET_VAR)
                .sub_var_layout(&OVERSCROLL_HORIZONTAL_OFFSET_VAR);
        }
        UiNodeOp::Layout { final_size, wl } => {
            *final_size = c.layout(wl);

            let mut new_v_rect = PxRect::zero();
            let v = OVERSCROLL_VERTICAL_OFFSET_VAR.get();
            if v < 0.fct() {
                // overscroll top
                new_v_rect.size = *final_size;
                new_v_rect.size.height *= v.abs().min(0.1.fct());
                v_center.y = Px(0);
            } else if v > 0.fct() {
                // overscroll bottom
                new_v_rect.size = *final_size;
                new_v_rect.size.height *= v.abs().min(0.1.fct());
                new_v_rect.origin.y = final_size.height - new_v_rect.size.height;
                v_center.y = new_v_rect.size.height;
            }

            let mut new_h_rect = PxRect::zero();
            let h = OVERSCROLL_HORIZONTAL_OFFSET_VAR.get();
            if h < 0.fct() {
                // overscroll left
                new_h_rect.size = *final_size;
                new_h_rect.size.width *= h.abs().min(0.1.fct());
                h_center.x = Px(0);
            } else if h > 0.fct() {
                // overscroll right
                new_h_rect.size = *final_size;
                new_h_rect.size.width *= h.abs().min(0.1.fct());
                new_h_rect.origin.x = final_size.width - new_h_rect.size.width;
                h_center.x = new_h_rect.size.width;
            }

            if new_v_rect != v_rect {
                v_rect = new_v_rect;
                // 50%
                v_center.x = v_rect.size.width / Px(2);
                // 110%
                let radius = v_center.x;
                v_radius_w = radius + radius * 0.1;

                WIDGET.render();
            }
            if new_h_rect != h_rect {
                h_rect = new_h_rect;
                h_center.y = h_rect.size.height / Px(2);
                let radius = h_center.y;
                h_radius_h = radius + radius * 0.1;
                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let stops = |color| {
                [
                    RenderGradientStop { offset: 0.0, color },
                    RenderGradientStop { offset: 0.99, color },
                    RenderGradientStop {
                        offset: 1.0,
                        color: {
                            let mut c = color;
                            c.a = 0.0;
                            c
                        },
                    },
                ]
            };

            frame.with_auto_hit_test(false, |frame| {
                if !v_rect.size.is_empty() {
                    let mut color: RenderColor = OVERSCROLL_COLOR_VAR.get().into();
                    color.a *= (OVERSCROLL_VERTICAL_OFFSET_VAR.get().abs().0 * 10.0).min(1.0);
                    let stops = stops(color);

                    let mut radius = v_rect.size;
                    radius.width = v_radius_w;
                    frame.push_radial_gradient(
                        v_rect,
                        v_center,
                        radius,
                        &stops,
                        ExtendMode::Clamp.into(),
                        v_rect.size,
                        PxSize::zero(),
                    );
                }
                if !h_rect.size.is_empty() {
                    let mut color: RenderColor = OVERSCROLL_COLOR_VAR.get().into();
                    color.a *= (OVERSCROLL_HORIZONTAL_OFFSET_VAR.get().abs().0 * 10.0).min(1.0);
                    let stops = stops(color);

                    let mut radius = h_rect.size;
                    radius.height = h_radius_h;
                    frame.push_radial_gradient(
                        h_rect,
                        h_center,
                        radius,
                        &stops,
                        ExtendMode::Clamp.into(),
                        h_rect.size,
                        PxSize::zero(),
                    );
                }
            });
        }
        _ => {}
    })
}

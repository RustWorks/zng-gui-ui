use crate::prelude::new_widget::*;

use crate::core::mouse::{ClickMode, MouseClickArgs};

/// Scrollbar widget.
#[widget($crate::widgets::scroll::scrollbar)]
pub mod scrollbar {
    use super::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use super::thumb;

    properties! {
        /// Thumb widget.
        ///
        /// Recommended widget is [`thumb!`], but can be any widget that implements
        /// thumb behavior and tags it-self in the frame.
        ///
        /// [`thumb!`]: mod@thumb
        pub thumb_node(impl UiNode) = thumb!();

        /// Fills the track with [`vis::BACKGROUND_VAR`]
        pub crate::properties::background_color = vis::BACKGROUND_VAR;

        /// Scrollbar orientation.
        ///
        /// This sets the scrollbar alignment to fill its axis and take the cross-length from the thumb.
        pub orientation(impl IntoVar<Orientation>) = Orientation::Vertical;

        // /// Set to repeat.
        pub crate::properties::click_mode = ClickMode::Repeat;

        pub crate::properties::events::mouse::on_mouse_click = hn!(|args: &MouseClickArgs| {
            use crate::widgets::scroll::*;
            use crate::core::window::WINDOW_CTRL;
            use std::cmp::Ordering;

            println!("!!: {:?}", (args.click_count, args.is_repeat, args.target.widget_id(), args.propagation().is_stopped()));

            let offset = SCROLL_VERTICAL_OFFSET_VAR.get();
            let bounds = WIDGET.bounds().inner_bounds();
            let offset = bounds.origin.y + bounds.size.height * offset;

            let scale_factor = WINDOW_CTRL.vars().scale_factor().get();
            let position = args.position.to_px(scale_factor.0);

            match position.y.cmp(&offset) {
                Ordering::Less => commands::PAGE_UP_CMD.scoped(SCROLL.id()).notify(),
                Ordering::Greater => commands::PAGE_DOWN_CMD.scoped(SCROLL.id()).notify(),
                Ordering::Equal => {},
            }

            args.propagation().stop();
        });
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let orientation = wgt.capture_var_or_else(property_id!(self::orientation), || Orientation::Vertical);

            // scrollbar is larger than thumb, align inserts the extra space.
            let thumb = wgt.capture_ui_node_or_else(property_id!(self::thumb_node), || NilUiNode);
            let thumb = align(thumb, Align::FILL);
            wgt.set_child(thumb);

            wgt.push_intrinsic(NestGroup::LAYOUT, "orientation-align", move |child| {
                align(
                    child,
                    orientation.map(|o| match o {
                        Orientation::Vertical => Align::FILL_RIGHT,
                        Orientation::Horizontal => Align::FILL_BOTTOM,
                    }),
                )
            });
        });
    }

    /// Style variables and properties.
    pub mod vis {
        use crate::prelude::new_property::*;

        context_var! {
            /// Scrollbar track background color
            pub static BACKGROUND_VAR: Rgba = rgba(80, 80, 80, 50.pct());
        }
    }

    /// Orientation of a scrollbar.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Orientation {
        /// Bar fills the in the ***x*** dimension and scrolls left-right.
        Horizontal,
        /// Bar fills the in the ***y*** dimension and scrolls top-bottom.
        Vertical,
    }
}

/// Scrollbar thumb widget.
#[widget($crate::widgets::scroll::scrollbar::thumb)]
pub mod thumb {
    use super::*;
    use crate::core::mouse::*;

    inherit!(widget_base::base);

    properties! {
        /// Scrollbar orientation.
        pub orientation(impl IntoVar<scrollbar::Orientation>) = scrollbar::Orientation::Vertical;

        /// Viewport/content ratio.
        ///
        /// This becomes the height for vertical and width for horizontal.
        pub viewport_ratio(impl IntoVar<Factor>);

        /// Content offset.
        pub offset(impl IntoVar<Factor>);

        /// Width if orientation is vertical, otherwise height if orientation is horizontal.
        pub cross_length(impl IntoVar<Length>) = 16;

        /// Fills the thumb with [`vis::BACKGROUND_VAR`].
        pub crate::properties::background_color = vis::BACKGROUND_VAR;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the thumb is pressed.
        capture_mouse = true;

        /// When the pointer device is over this thumb.
        when *#is_hovered {
            background_color = vis::hovered::BACKGROUND_VAR;
        }

        /// When the thumb is pressed.
        when *#is_cap_pressed  {
            background_color = vis::pressed::BACKGROUND_VAR;
        }
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(on_build);
    }
    fn on_build(wgt: &mut WidgetBuilding) {
        let cross_length = wgt.capture_var_or_default::<Length>(property_id!(self::cross_length));
        wgt.push_intrinsic(NestGroup::SIZE, "orientation-size", move |child| {
            size(
                child,
                merge_var!(THUMB_ORIENTATION_VAR, THUMB_VIEWPORT_RATIO_VAR, cross_length, |o, r, l| {
                    match o {
                        scrollbar::Orientation::Vertical => Size::new(l.clone(), *r),
                        scrollbar::Orientation::Horizontal => Size::new(*r, l.clone()),
                    }
                }),
            )
        });

        wgt.push_intrinsic(NestGroup::LAYOUT, "thumb_layout", thumb_layout);

        let orientation = wgt.capture_var_or_else(property_id!(self::orientation), || scrollbar::Orientation::Vertical);
        let viewport_ratio = wgt.capture_var_or_else(property_id!(self::viewport_ratio), || 1.fct());
        let offset = wgt.capture_var_or_else(property_id!(self::offset), || 0.fct());

        wgt.push_intrinsic(NestGroup::CONTEXT, "thumb-context", move |child| {
            let child = with_context_var(child, THUMB_ORIENTATION_VAR, orientation);
            let child = with_context_var(child, THUMB_VIEWPORT_RATIO_VAR, viewport_ratio);
            with_context_var(child, THUMB_OFFSET_VAR, offset)
        });
    }

    fn thumb_layout(child: impl UiNode) -> impl UiNode {
        #[ui_node(struct DragNode {
            child: impl UiNode,
            content_length: Px,
            viewport_length: Px,
            thumb_length: Px,
            scale_factor: Factor,

            mouse_down: Option<(Px, Factor)>,
        })]
        impl UiNode for DragNode {
            fn init(&mut self) {
                WIDGET
                    .sub_event(&MOUSE_MOVE_EVENT)
                    .sub_event(&MOUSE_INPUT_EVENT)
                    .sub_var(&THUMB_OFFSET_VAR);
                self.child.init();
            }

            fn event(&mut self, update: &mut EventUpdate) {
                if let Some((mouse_down, start_offset)) = self.mouse_down {
                    if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                        let bounds = WIDGET.bounds().inner_bounds();
                        let (mut offset, cancel_offset, bounds_min, bounds_max) = match THUMB_ORIENTATION_VAR.get() {
                            scrollbar::Orientation::Vertical => (
                                args.position.y.to_px(self.scale_factor.0),
                                args.position.x.to_px(self.scale_factor.0),
                                bounds.min_x(),
                                bounds.max_x(),
                            ),
                            scrollbar::Orientation::Horizontal => (
                                args.position.x.to_px(self.scale_factor.0),
                                args.position.y.to_px(self.scale_factor.0),
                                bounds.min_y(),
                                bounds.max_y(),
                            ),
                        };

                        let cancel_margin = Dip::new(40).to_px(self.scale_factor.0);
                        let offset = if cancel_offset < bounds_min - cancel_margin || cancel_offset > bounds_max + cancel_margin {
                            // pointer moved outside of the thumb + 40, snap back to initial
                            start_offset
                        } else {
                            offset -= mouse_down;

                            let max_length = self.viewport_length - self.thumb_length;
                            let start_offset = max_length * start_offset.0;

                            let offset = offset + start_offset;
                            let offset = (offset.0 as f32 / max_length.0 as f32).clamp(0.0, 1.0);

                            // snap to pixel
                            let max_length = self.viewport_length - self.content_length;
                            let offset = max_length * offset;
                            let offset = offset.0 as f32 / max_length.0 as f32;
                            offset.fct()
                        };

                        THUMB_OFFSET_VAR.set_ne(offset).expect("THUMB_OFFSET_VAR is read-only");
                        WIDGET.layout();

                        args.propagation().stop();
                    } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                        if args.is_primary() && args.is_mouse_up() {
                            self.mouse_down = None;

                            args.propagation().stop();
                        }
                    }
                } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    if args.is_primary() && args.is_mouse_down() {
                        let a = match THUMB_ORIENTATION_VAR.get() {
                            scrollbar::Orientation::Vertical => args.position.y.to_px(self.scale_factor.0),
                            scrollbar::Orientation::Horizontal => args.position.x.to_px(self.scale_factor.0),
                        };
                        self.mouse_down = Some((a, THUMB_OFFSET_VAR.get()));

                        args.propagation().stop();
                    }
                }
                self.child.event(update);
            }

            fn update(&mut self, updates: &mut WidgetUpdates) {
                if THUMB_OFFSET_VAR.is_new() {
                    WIDGET.layout();
                }

                self.child.update(updates);
            }

            fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
                self.child.measure(wm)
            }
            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                let bar_size = LAYOUT.constrains().fill_size();
                let mut final_offset = PxVector::zero();
                let (bar_length, final_d) = match THUMB_ORIENTATION_VAR.get() {
                    scrollbar::Orientation::Vertical => (bar_size.height, &mut final_offset.y),
                    scrollbar::Orientation::Horizontal => (bar_size.width, &mut final_offset.x),
                };

                let ratio = THUMB_VIEWPORT_RATIO_VAR.get();
                let thumb_length = bar_length * ratio;
                *final_d = (bar_length - thumb_length) * THUMB_OFFSET_VAR.get();

                self.scale_factor = LAYOUT.scale_factor();
                self.content_length = bar_length / ratio;
                self.viewport_length = bar_length;
                self.thumb_length = thumb_length;

                wl.translate(final_offset);

                self.child.layout(wl)
            }
        }
        DragNode {
            child,
            content_length: Px(0),
            viewport_length: Px(0),
            thumb_length: Px(0),
            scale_factor: 1.fct(),

            mouse_down: None,
        }
    }

    context_var! {
        static THUMB_ORIENTATION_VAR: scrollbar::Orientation = scrollbar::Orientation::Vertical;
        static THUMB_VIEWPORT_RATIO_VAR: Factor = 1.fct();
        static THUMB_OFFSET_VAR: Factor = 0.fct();
    }

    /// Style variables.
    pub mod vis {
        use crate::prelude::new_property::*;

        context_var! {
            /// Fill color.
            pub static BACKGROUND_VAR: Rgba = rgba(200, 200, 200, 50.pct());
        }

        /// Variables when the pointer device is over the thumb.
        pub mod hovered {
            use super::*;

            context_var! {
                /// Fill color.
                pub static BACKGROUND_VAR: Rgba = rgba(200, 200, 200, 70.pct());
            }
        }

        /// Variables when the pointer device is pressing the thumb.
        pub mod pressed {
            use super::*;

            context_var! {
                /// Fill color.
                pub static BACKGROUND_VAR: Rgba = rgba(200, 200, 200, 90.pct());
            }
        }
    }
}

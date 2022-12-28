//! Properties that affect the widget render only.

use crate::core::gradient::{GradientStops, LinearGradientAxis};
use crate::prelude::new_property::*;
use crate::widgets::{flood, linear_gradient};

use super::hit_test_mode;

/// Custom background property. Allows using any other widget as a background.
///
/// Backgrounds are not interactive, but are hit-testable, they don't influence the layout being measured and
/// arranged with the widget size, and they are always clipped to the widget bounds.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     background = text! {
///         txt = "CUSTOM BACKGROUND";
///         font_size = 72;
///         txt_color = colors::LIGHT_GRAY;
///         transform = rotate(45.deg());
///         align = Align::CENTER;
///     }
/// }
/// # ;
/// ```
///
/// The example renders a custom text background.
#[property(FILL)]
pub fn background(child: impl UiNode, background: impl UiNode) -> impl UiNode {
    #[ui_node(struct BackgroundNode {
        children: impl UiNodeList,
    })]
    impl UiNode for BackgroundNode {
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.children.with_node(1, |n| n.measure(ctx, wm))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.children.with_node_mut(1, |n| n.layout(ctx, wl));
            ctx.with_constrains(
                |c| PxConstrains2d::new_exact_size(c.fill_size_or(size)),
                |ctx| {
                    self.children.with_node_mut(0, |n| n.layout(ctx, wl));
                },
            );
            size
        }
    }

    let background = interactive_node(background, false);
    let background = fill_node(background);

    BackgroundNode {
        children: ui_vec![background, child],
    }
}

/// Custom background generated using a [`ViewGenerator<()>`].
///
/// This is the equivalent of setting [`background`] to the [`presenter_default`] node.
///
/// [`ViewGenerator<()>`]: ViewGenerator
/// [`background`]: fn@background
/// [`presenter_default`]: ViewGenerator::presenter_default
#[property(FILL, default(ViewGenerator::nil()))]
pub fn background_gen(child: impl UiNode, generator: impl IntoVar<ViewGenerator<()>>) -> impl UiNode {
    background(child, ViewGenerator::presenter_default(generator))
}

/// Single color background property.
///
/// This property applies a [`flood`] as [`background`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     background_color = hex!(#ADF0B0);
/// }
/// # ;
/// ```
///
/// [`background`]: fn@background
#[property(FILL, default(colors::BLACK.transparent()))]
pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    background(child, flood(color))
}

/// Linear gradient background property.
///
/// This property applies a [`linear_gradient`] as [`background`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     background_gradient = {
///         axis: 90.deg(),
///         stops: [colors::BLACK, colors::WHITE]
///     }
/// }
/// # ;
/// ```
///
/// [`background`]: fn@background
#[property(FILL, default(0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn background_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    background(child, linear_gradient(axis, stops))
}

/// Custom foreground fill property. Allows using any other widget as a foreground overlay.
///
/// The foreground is rendered over the widget content and background and under the widget borders.
///
/// Foregrounds are not interactive, not hit-testable and don't influence the widget layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     foreground = text! {
///         txt = "TRIAL";
///         font_size = 72;
///         txt_color = colors::BLACK;
///         opacity = 10.pct();
///         transform = rotate(45.deg());
///         align = Align::CENTER;
///     }
/// }
/// # ;
/// ```
///
/// The example renders a custom see-through text overlay.
#[property(FILL, default(crate::core::widget_instance::NilUiNode))]
pub fn foreground(child: impl UiNode, foreground: impl UiNode) -> impl UiNode {
    #[ui_node(struct ForegroundNode {
        children: impl UiNodeList,
    })]
    impl UiNode for ForegroundNode {
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.children.with_node(0, |n| n.measure(ctx, wm))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.children.with_node_mut(0, |n| n.layout(ctx, wl));
            ctx.with_constrains(
                |c| PxConstrains2d::new_exact_size(c.fill_size_or(size)),
                |ctx| {
                    self.children.with_node_mut(1, |n| n.layout(ctx, wl));
                },
            );
            size
        }
    }

    let foreground = interactive_node(foreground, false);
    let foreground = fill_node(foreground);
    let foreground = hit_test_mode(foreground, HitTestMode::Disabled);

    ForegroundNode {
        children: ui_vec![child, foreground],
    }
}

/// Foreground highlight border overlay.
///
/// This property draws a border contour with extra `offsets` padding as an overlay.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { wgt!() }
/// container! {
///     child = foo();
///     foreground_highlight = {
///         offsets: 3,
///         widths: 1,
///         sides: colors::BLUE,
///     }
/// }
/// # ;
/// ```
///
/// The example renders a solid blue 1 pixel border overlay, the border lines are offset 3 pixels into the container.
#[property(FILL, default(0, 0, BorderStyle::Hidden))]
pub fn foreground_highlight(
    child: impl UiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
) -> impl UiNode {
    #[ui_node(struct ForegroundHighlightNode {
        child: impl UiNode,
        #[var] offsets: impl Var<SideOffsets>,
        #[var] widths: impl Var<SideOffsets>,
        #[var] sides: impl Var<BorderSides>,

        render_bounds: PxRect,
        render_widths: PxSideOffsets,
        render_radius: PxCornerRadius,
    })]
    impl UiNode for ForegroundHighlightNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.offsets.is_new(ctx) || self.widths.is_new(ctx) {
                ctx.updates.layout();
            } else if self.sides.is_new(ctx) {
                ctx.updates.render();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.child.layout(ctx, wl);

            let radius = ContextBorders::inner_radius(ctx);
            let offsets = self.offsets.get().layout(ctx.metrics, |_| PxSideOffsets::zero());
            let radius = radius.deflate(offsets);

            let bounds;

            if let Some(inline) = wl.inline() {
                let mut rect = inline.first_rect();
                if rect.size.is_empty() {
                    rect = inline.middle_rect();
                }
                if rect.size.is_empty() {
                    rect = inline.last_rect();
                }

                rect.origin.x += offsets.left;
                rect.origin.y += offsets.top;
                rect.size.width -= offsets.horizontal();
                rect.size.height -= offsets.vertical();

                bounds = rect;
            } else {
                let border_offsets = ContextBorders::inner_offsets(ctx.path.widget_id());

                bounds = PxRect::new(
                    PxPoint::new(offsets.left + border_offsets.left, offsets.top + border_offsets.top),
                    size - PxSize::new(offsets.horizontal(), offsets.vertical()),
                );
            }

            let widths = ctx.with_constrains(
                |c| PxConstrains2d::new_exact_size(c.fill_size_or(size)),
                |ctx| self.widths.get().layout(ctx.metrics, |_| PxSideOffsets::zero()),
            );

            if self.render_bounds != bounds || self.render_widths != widths || self.render_radius != radius {
                self.render_bounds = bounds;
                self.render_widths = widths;
                self.render_radius = radius;
                ctx.updates.render();
            }

            size
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);
            frame.push_border(self.render_bounds, self.render_widths, self.sides.get(), self.render_radius);
        }
    }
    ForegroundHighlightNode {
        child: child.cfg_boxed(),
        offsets: offsets.into_var(),
        widths: widths.into_var(),
        sides: sides.into_var(),

        render_bounds: PxRect::zero(),
        render_widths: PxSideOffsets::zero(),
        render_radius: PxCornerRadius::zero(),
    }
    .cfg_boxed()
}

/// Fill color overlay property.
///
/// This property applies a [`flood`] as [`foreground`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     foreground_color = rgba(0, 240, 0, 10.pct())
/// }
/// # ;
/// ```
///
/// The example adds a green tint to the container content.
///
/// [`foreground`]: fn@foreground
#[property(FILL, default(colors::BLACK.transparent()))]
pub fn foreground_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    foreground(child, flood(color))
}

/// Linear gradient overlay property.
///
/// This property applies a [`linear_gradient`] as [`foreground`] using the [`Clamp`] extend mode.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # fn foo() -> impl UiNode { wgt!() }
/// #
/// container! {
///     child = foo();
///     foreground_gradient = {
///         axis: (0, 0).to(0, 10),
///         stops: [colors::BLACK, colors::BLACK.transparent()]
///     }
/// }
/// # ;
/// ```
///
/// The example adds a *shadow* gradient to a 10px strip in the top part of the container content.
///
/// [`foreground`]: fn@foreground
/// [`Clamp`]: crate::core::gradient::ExtendMode::Clamp
#[property(FILL, default(0.deg(), {
    let c = colors::BLACK.transparent();
    crate::core::gradient::stops![c, c]
}))]
pub fn foreground_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    foreground(child, linear_gradient(axis, stops))
}

/// Clips the widget child to the area of the widget when set to `true`.
///
/// Any content rendered outside the widget inner bounds is clipped, hit test shapes are also clipped. The clip is
/// rectangular and can have rounded corners if [`corner_radius`] is set. If the widget is inlined during layout the first
/// row advance and last row trail are also clipped.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// #
/// container! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     corner_radius = 5;
///     clip_to_bounds = true;
///     child = container! {
///         background_color = rgb(0, 255, 0);
///         // fixed size ignores the layout available size.
///         size = (1000, 1000);
///         child = text("1000x1000 green clipped to 200x300");
///     };
/// }
/// # ;
/// ```
///
/// [`corner_radius`]: fn@corner_radius
#[property(FILL, default(false))]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct ClipToBoundsNode {
        child: impl UiNode,
        #[var] clip: impl Var<bool>,
        corners: PxCornerRadius,
        inline: (PxPoint, PxPoint),
    })]
    impl UiNode for ClipToBoundsNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.clip.is_new(ctx) {
                ctx.updates.layout_render();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let bounds = self.child.layout(ctx, wl);

            if self.clip.get() {
                let corners = ContextBorders::border_radius(ctx);
                if corners != self.corners {
                    self.corners = corners;
                    ctx.updates.render();
                }

                if let Some(inline) = wl.inline() {
                    self.inline = (inline.first_row, inline.last_row);
                } else {
                    self.inline = (PxPoint::zero(), PxPoint::zero());
                }
            }

            bounds
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if self.clip.get() {
                frame.push_clips(
                    |c| {
                        let bounds = PxRect::from_size(ctx.widget_info.bounds.inner_size());

                        if self.corners != PxCornerRadius::zero() {
                            c.push_clip_rounded_rect(bounds, self.corners, false, true);
                        } else {
                            c.push_clip_rect(bounds, false, true);
                        }

                        if self.inline.1 != PxPoint::zero() {
                            if self.inline.0 != PxPoint::zero() {
                                let first = PxRect::new(bounds.origin, self.inline.0.to_vector().to_size());
                                c.push_clip_rect(first, true, true);
                            }

                            let mut last = bounds.to_box2d();
                            last.min += self.inline.1.to_vector();
                            let last = last.to_rect();
                            c.push_clip_rect(last, true, true);
                        }
                    },
                    |f| self.child.render(ctx, f),
                );
            } else {
                self.child.render(ctx, frame);
            }
        }
    }
    ClipToBoundsNode {
        child,
        clip: clip.into_var(),
        corners: PxCornerRadius::zero(),
        inline: (PxPoint::zero(), PxPoint::zero()),
    }
}

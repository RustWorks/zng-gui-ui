//! Color filter properties, [`opacity`](fn@opacity), [`filter`](fn@filter) and more.

use crate::prelude::new_property::*;

use crate::core::color::filters::{
    self as cf, {Filter, RenderFilter},
};

/// Color filter, or combination of filters.
///
/// This property allows setting multiple filters at once, there is also a property for every
/// filter for easier value updating.
///
/// # Performance
///
/// The performance for setting specific filter properties versus this one is the same, except for [`opacity`]
/// which can be animated using only frame updates instead of generating a new frame every change.
///
/// [`opacity`]: fn@opacity
#[property(context, default(Filter::default()))]
pub fn filter(child: impl UiNode, filter: impl IntoVar<Filter>) -> impl UiNode {
    #[ui_node(struct FilterNode {
        child: impl UiNode,
        #[var] filter: impl Var<Filter>,
        render_filter: Option<RenderFilter>,
    })]
    impl UiNode for FilterNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.init_handles(ctx);
            self.render_filter = self.filter.with(Filter::try_render);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.filter.with_new(ctx.vars, |f| {
                if let Some(f) = f.try_render() {
                    self.render_filter = Some(f);
                    ctx.updates.render();
                } else {
                    self.render_filter = None;
                    ctx.updates.layout();
                }
            });
            self.child.update(ctx, updates)
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            if self.render_filter.is_none() {
                self.render_filter = Some(self.filter.get().layout(ctx.metrics));
                ctx.updates.render();
            }
            self.child.layout(ctx, wl)
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_inner_filter(self.render_filter.clone().unwrap(), |frame| self.child.render(ctx, frame));
        }
    }
    FilterNode {
        child,
        filter: filter.into_var(),
        render_filter: None,
    }
}

/// Color filter, or combination of filters targeting the widget's descendants and not the widget itself.
///
/// This property allows setting multiple filters at once, there is also a property for every
/// filter for easier value updating.
///
/// # Performance
///
/// The performance for setting specific filter properties versus this one is the same, except for [`child_opacity`]
/// which can be animated using only frame updates instead of generating a new frame every change.
///
/// [`child_opacity`]: fn@child_opacity
#[property(child_context, default(Filter::default()))]
pub fn child_filter(child: impl UiNode, filter: impl IntoVar<Filter>) -> impl UiNode {
    #[ui_node(struct ChildFilterNode {
        child: impl UiNode,
        #[var] filter: impl Var<Filter>,
        render_filter: Option<RenderFilter>,
    })]
    impl UiNode for ChildFilterNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.init_handles(ctx);
            self.render_filter = self.filter.with(Filter::try_render);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.filter.with_new(ctx.vars, |f| {
                if let Some(f) = f.try_render() {
                    self.render_filter = Some(f);
                    ctx.updates.render();
                } else {
                    self.render_filter = None;
                    ctx.updates.layout();
                }
            });
            self.child.update(ctx, updates)
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            if self.render_filter.is_none() {
                self.render_filter = Some(self.filter.get().layout(ctx.metrics));
                ctx.updates.render();
            }
            self.child.layout(ctx, wl)
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_filter(MixBlendMode::Normal.into(), self.render_filter.as_ref().unwrap(), |frame| {
                self.child.render(ctx, frame)
            });
        }
    }
    ChildFilterNode {
        child,
        filter: filter.into_var(),
        render_filter: None,
    }
}

/// Inverts the colors of the widget.
///
/// Zero does not invert, one fully inverts.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::invert`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(false))]
pub fn invert_color(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| cf::invert(a)))
}

/// Blur the widget.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::blur`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(0))]
pub fn blur(child: impl UiNode, radius: impl IntoVar<Length>) -> impl UiNode {
    filter(child, radius.into_var().map(|r| cf::blur(r.clone())))
}

/// Sepia tone the widget.
///
/// zero is the original colors, one is the full desaturated brown look.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::sepia`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(false))]
pub fn sepia(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| cf::sepia(a)))
}

/// Grayscale tone the widget.
///
/// Zero is the original colors, one if the full grayscale.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::grayscale`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(false))]
pub fn grayscale(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| cf::grayscale(a)))
}

/// Drop-shadow effect for the widget.
///
/// The shadow is *pixel accurate*.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::drop_shadow`] using variable merging.
///
/// [`filter`]: fn@filter
#[property(context, default((0, 0), 0, colors::BLACK.transparent()))]
pub fn drop_shadow(
    child: impl UiNode,
    offset: impl IntoVar<Point>,
    blur_radius: impl IntoVar<Length>,
    color: impl IntoVar<Rgba>,
) -> impl UiNode {
    filter(
        child,
        merge_var!(offset.into_var(), blur_radius.into_var(), color.into_var(), |o, r, &c| {
            cf::drop_shadow(o.clone(), r.clone(), c)
        }),
    )
}

/// Adjust the widget colors brightness.
///
/// Zero removes all brightness, one is the original brightness.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::brightness`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(1.0))]
pub fn brightness(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| cf::brightness(a)))
}

/// Adjust the widget colors contrast.
///
/// Zero removes all contrast, one is the original contrast.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::brightness`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(1.0))]
pub fn contrast(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| cf::contrast(a)))
}

/// Adjust the widget colors saturation.
///
/// Zero fully desaturates, one is the original saturation.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::saturate`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(1.0))]
pub fn saturate(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| cf::saturate(a)))
}

/// Hue shift the widget colors.
///
/// Adds `angle` to the [`hue`] of the widget colors.
///
/// This property is a shorthand way of setting [`filter`] to [`color::filter::hue_rotate`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`hue`]: Hsla::hue
#[property(context, default(0.deg()))]
pub fn hue_rotate(child: impl UiNode, angle: impl IntoVar<AngleDegree>) -> impl UiNode {
    filter(child, angle.into_var().map(|&a| cf::hue_rotate(a)))
}

/// Custom color filter.
///
/// The color matrix is in the format of SVG color matrix, [0..5] is the first matrix row.
#[property(context, default(cf::ColorMatrix::identity()))]
pub fn color_matrix(child: impl UiNode, matrix: impl IntoVar<cf::ColorMatrix>) -> impl UiNode {
    filter(child, matrix.into_var().map(|&m| cf::color_matrix(m)))
}

/// Opacity/transparency of the widget.
///
/// This property provides the same visual result as setting [`filter`] to [`color::filter::opacity(opacity)`](color::filter::opacity),
/// **but** updating the opacity is faster in this property.
///
/// [`filter`]: fn@filter
#[property(context, default(1.0))]
pub fn opacity(child: impl UiNode, alpha: impl IntoVar<Factor>) -> impl UiNode {
    #[ui_node(struct OpacityNode {
        child: impl UiNode,
        #[var] alpha: impl Var<Factor>,
        frame_key: FrameVarKey<f32>,
    })]
    impl UiNode for OpacityNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.alpha.is_new(ctx) {
                ctx.updates.render_update();
            }
            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let opacity = self.frame_key.bind(&self.alpha, |f| f.0);
            frame.push_inner_opacity(opacity, |frame| self.child.render(ctx, frame));
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            update.update_f32_opt(self.frame_key.update(&self.alpha, |f| f.0));
            self.child.render_update(ctx, update);
        }
    }

    OpacityNode {
        child,
        frame_key: FrameVarKey::new(),
        alpha: alpha.into_var(),
    }
}

/// Opacity/transparency of the widget's child.
///
/// This property provides the same visual result as setting [`child_filter`] to [`color::filter::opacity(opacity)`](color::filter::opacity),
/// **but** updating the opacity is faster in this property.
///
/// [`child_filter`]: fn@child_filter
#[property(child_context, default(1.0))]
pub fn child_opacity(child: impl UiNode, alpha: impl IntoVar<Factor>) -> impl UiNode {
    #[ui_node(struct ChildOpacityNode {
        child: impl UiNode,
        #[var] alpha: impl Var<Factor>,
        frame_key: FrameVarKey<f32>,
    })]
    impl UiNode for ChildOpacityNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.alpha.is_new(ctx) {
                ctx.updates.render_update();
            }
            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let opacity = self.frame_key.bind(&self.alpha, |f| f.0);
            frame.push_opacity(opacity, |frame| self.child.render(ctx, frame));
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            update.update_f32_opt(self.frame_key.update(&self.alpha, |f| f.0));
            self.child.render_update(ctx, update);
        }
    }

    ChildOpacityNode {
        child,
        frame_key: FrameVarKey::new(),
        alpha: alpha.into_var(),
    }
}

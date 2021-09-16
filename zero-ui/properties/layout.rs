//! Properties that affect the widget layout only.

use zero_ui::prelude::new_property::*;

/// Margin space around the widget.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
///
/// button! {
///     margin = 10;
///     content = text("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button has `10` layout pixels of space in all directions around it. You can
/// also control each side in specific:
///
/// ```
/// # use zero_ui::prelude::*;
/// container! {
///     content = button! {
///         margin = (10, 5.pct());
///         content = text("Click Me!")
///     };
///     margin = (1, 2, 3, 4);
/// }
/// # ;
/// ```
///
/// In the example the button has `10` pixels of space above and bellow and `5%` of the container width to the left and right.
/// The container itself has margin of `1` to the top, `2` to the right, `3` to the bottom and `4` to the left.
#[property(outer, default(0))]
pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
    struct MarginNode<T, M> {
        child: T,
        margin: M,
        size_increment: PxSize,
        child_rect: PxRect,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, M: Var<SideOffsets>> UiNode for MarginNode<T, M> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.margin.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let margin = self.margin.get(ctx).to_layout(ctx, available_size);
            self.size_increment = PxSize::new(margin.left + margin.right, margin.top + margin.bottom);

            // the size is set in `arrange`
            self.child_rect.origin = PxPoint::new(margin.left, margin.top);

            self.child.measure(ctx, available_size.sub_px(self.size_increment)) + self.size_increment
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, mut final_size: PxSize) {
            final_size -= self.size_increment;
            self.child_rect.size = final_size;
            self.child.arrange(ctx, final_size);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(ctx, frame));
        }
    }
    MarginNode {
        child,
        margin: margin.into_var(),
        size_increment: PxSize::zero(),
        child_rect: PxRect::zero(),
    }
}

/// Aligns the widget within the available space.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         align = Alignment::TOP;
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is positioned at the top-center of the container. See [`Alignment`] for
/// more details.
#[property(outer, default(Alignment::FILL))]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Alignment>) -> impl UiNode {
    struct AlignNode<T, A> {
        child: T,
        alignment: A,
        child_rect: PxRect,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, A: Var<Alignment>> UiNode for AlignNode<T, A> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.alignment.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let size = self.child.measure(ctx, available_size);
            self.child_rect.size = size;
            size
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            self.child_rect = self.alignment.get(ctx.vars).solve(self.child_rect.size, final_size);

            self.child.arrange(ctx, self.child_rect.size);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(ctx, frame));
        }
    }

    AlignNode {
        child,
        alignment: alignment.into_var(),
        child_rect: PxRect::zero(),
    }
}

/// Widget left-top offset.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         position = (100, 20.pct());
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is manually positioned `100` layout pixels from the left of the container and
/// at `20` percent of the container height from the top of the container.
///
/// # `x` and `y`
///
/// You can use the [`x`](fn@x) and [`y`](fn@y) properties to only set the position in one dimension.
#[property(outer, default((0, 0)))]
pub fn position(child: impl UiNode, position: impl IntoVar<Point>) -> impl UiNode {
    struct PositionNode<T: UiNode, P: Var<Point>> {
        child: T,
        position: P,
        final_position: PxPoint,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, P: Var<Point>> UiNode for PositionNode<T, P> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.position.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            self.child.arrange(ctx, final_size);
            self.final_position = self.position.get(ctx).to_layout(ctx, AvailableSize::finite(final_size));
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_reference_frame(self.final_position, |frame| self.child.render(ctx, frame));
        }
    }
    PositionNode {
        child,
        position: position.into_var(),
        final_position: PxPoint::zero(),
    }
}

/// Left offset.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         x = 20.pct();
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is manually positioned at `20` percent of the container width from the left of the container.
///
/// # `position`
///
/// You can set both `x` and `y` at the same time using the [`position`](fn@position) property.
#[property(outer, default(0))]
pub fn x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    struct XNode<T: UiNode, X: Var<Length>> {
        child: T,
        x: X,
        final_x: Px,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, X: Var<Length>> UiNode for XNode<T, X> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.x.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            self.child.arrange(ctx, final_size);
            self.final_x = self.x.get(ctx).to_layout(ctx, AvailablePx::Finite(final_size.width));
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_reference_frame(PxPoint::new(self.final_x, Px(0)), |frame| self.child.render(ctx, frame));
        }
    }
    XNode {
        child,
        x: x.into_var(),
        final_x: Px(0),
    }
}

/// Top offset.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         y = 20.pct();
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is manually positioned at `20` percent of the container height from the top of the container.
///
/// # `position`
///
/// You can set both `x` and `y` at the same time using the [`position`](fn@position) property.
#[property(outer, default(0))]
pub fn y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    struct YNode<T: UiNode, Y: Var<Length>> {
        child: T,
        y: Y,
        final_y: Px,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, Y: Var<Length>> UiNode for YNode<T, Y> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.y.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            self.child.arrange(ctx, final_size);
            self.final_y = self.y.get(ctx).to_layout(ctx, AvailablePx::Finite(final_size.height));
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_reference_frame(PxPoint::new(Px(0), self.final_y), |frame| self.child.render(ctx, frame));
        }
    }
    YNode {
        child,
        y: y.into_var(),
        final_y: Px(0),
    }
}

/// Minimum size of the widget.
///
/// The widget size can be larger then this but not smaller.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// # let label = formatx!("");
///
/// button! {
///     content = text(label);
///     min_size = (100, 50);
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `label` value but it will
/// always have a minimum width of `100` and a minimum height of `50`.
///
/// # `min_width` and `min_height`
///
/// You can use the [`min_width`](fn@min_width) and [`min_height`](fn@min_height) properties to only
/// set the minimum size of one dimension.
#[property(size, default((0, 0)))]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<Size>) -> impl UiNode {
    struct MinSizeNode<T: UiNode, S: Var<Size>> {
        child: T,
        min_size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: Var<Size>> UiNode for MinSizeNode<T, S> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let min_size = self.min_size.get(ctx).to_layout(ctx, available_size);
            let available_size = available_size.max_px(min_size);

            let desired_size = self.child.measure(ctx, available_size);

            desired_size.max(min_size)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            let min_size = self.min_size.get(ctx).to_layout(ctx, AvailableSize::finite(final_size));
            self.child.arrange(ctx, min_size.max(final_size));
        }
    }
    MinSizeNode {
        child,
        min_size: min_size.into_var(),
    }
}

/// Minimum width of the widget.
///
/// The widget width can be larger then this but not smaller.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// # let label = formatx!("");
///
/// button! {
///     content = text(label);
///     min_width = 100;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `label` value but it will
/// always have a minimum width of `100`.
///
/// # `min_size`
///
/// You can set both `min_width` and `min_height` at the same time using the [`min_size`](fn@min_size) property.
#[property(size, default(0))]
pub fn min_width(child: impl UiNode, min_width: impl IntoVar<Length>) -> impl UiNode {
    struct MinWidthNode<T: UiNode, W: Var<Length>> {
        child: T,
        min_width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: Var<Length>> UiNode for MinWidthNode<T, W> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
            let min_width = self.min_width.get(ctx).to_layout(ctx, available_size.width);
            available_size.width = available_size.width.max_px(min_width);

            let mut desired_size = self.child.measure(ctx, available_size);
            desired_size.width = min_width.max(desired_size.width);

            desired_size
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, mut final_size: PxSize) {
            let min_width = self.min_width.get(ctx).to_layout(ctx, AvailablePx::Finite(final_size.width));
            final_size.width = min_width.max(final_size.width);

            self.child.arrange(ctx, final_size);
        }
    }
    MinWidthNode {
        child,
        min_width: min_width.into_var(),
    }
}

/// Minimum height of the widget.
///
/// The widget height can be larger then this but not smaller.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// # let btn_content = text("");
///
/// button! {
///     content = btn_content;
///     min_height = 50;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a minimum height of `50`.
///
/// # `min_size`
///
/// You can set both `min_width` and `min_height` at the same time using the [`min_size`](fn@min_size) property.
#[property(size, default(0))]
pub fn min_height(child: impl UiNode, min_height: impl IntoVar<Length>) -> impl UiNode {
    struct MinHeightNode<T: UiNode, H: Var<Length>> {
        child: T,
        min_height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: Var<Length>> UiNode for MinHeightNode<T, H> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
            let min_height = self.min_height.get(ctx).to_layout(ctx, available_size.height);
            available_size.height = available_size.height.max_px(min_height);

            let mut desired_size = self.child.measure(ctx, available_size);
            desired_size.height = min_height.max(desired_size.height);

            desired_size
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, mut final_size: PxSize) {
            let min_height = self.min_height.get(ctx).to_layout(ctx, AvailablePx::Finite(final_size.height));
            final_size.height = min_height.max(final_size.height);

            self.child.arrange(ctx, final_size);
        }
    }
    MinHeightNode {
        child,
        min_height: min_height.into_var(),
    }
}

/// Maximum size of the widget.
///
/// The widget size can be smaller then this but not larger.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// # let btn_content = text("");
///
/// button! {
///     content = btn_content;
///     max_size = (200, 100);
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum width of `200` and a maximum height of `100`.
///
/// # `max_width` and `max_height`
///
/// You can use the [`max_width`](fn@max_width) and [`max_height`](fn@max_height) properties to only
/// set the maximum size of one dimension.
#[property(size)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<Size>) -> impl UiNode {
    struct MaxSizeNode<T: UiNode, S: Var<Size>> {
        child: T,
        max_size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: Var<Size>> UiNode for MaxSizeNode<T, S> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let max_size = self.max_size.get(ctx).to_layout(ctx, available_size);
            self.child.measure(ctx, available_size.min_px(max_size)).min(max_size)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            let final_size = self
                .max_size
                .get(ctx)
                .to_layout(ctx, AvailableSize::finite(final_size))
                .min(final_size);

            self.child.arrange(ctx, final_size);
        }
    }
    MaxSizeNode {
        child,
        max_size: max_size.into_var(),
    }
}

/// Maximum width of the widget.
///
/// The widget width can be smaller then this but not larger.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// # let btn_content = text("");
///
/// button! {
///     content = btn_content;
///     max_width = 200;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum width of `200`.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@max_size) property.
#[property(size)]
pub fn max_width(child: impl UiNode, max_width: impl IntoVar<Length>) -> impl UiNode {
    struct MaxWidthNode<T: UiNode, W: Var<Length>> {
        child: T,
        max_width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: Var<Length>> UiNode for MaxWidthNode<T, W> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
            let max_width = self.max_width.get(ctx).to_layout(ctx, available_size.width);

            available_size.width = available_size.width.min_px(max_width);

            let mut desired_size = self.child.measure(ctx, available_size);
            desired_size.width = desired_size.width.min(max_width);
            desired_size
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, mut final_size: PxSize) {
            final_size.width = self
                .max_width
                .get(ctx)
                .to_layout(ctx, AvailablePx::Finite(final_size.width))
                .min(final_size.width);

            self.child.arrange(ctx, final_size);
        }
    }
    MaxWidthNode {
        child,
        max_width: max_width.into_var(),
    }
}

/// Maximum height of the widget.
///
/// The widget height can be smaller then this but not larger.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// # let btn_content = text("");
///
/// button! {
///     content = btn_content;
///     max_height = 100;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum height of `100`.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@max_size) property.
#[property(size)]
pub fn max_height(child: impl UiNode, max_height: impl IntoVar<Length>) -> impl UiNode {
    struct MaxHeightNode<T: UiNode, H: Var<Length>> {
        child: T,
        max_height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: Var<Length>> UiNode for MaxHeightNode<T, H> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
            let max_height = self.max_height.get(ctx).to_layout(ctx, available_size.height);

            available_size.height = available_size.height.min_px(max_height);

            let mut desired_size = self.child.measure(ctx, available_size);
            desired_size.height = desired_size.height.min(max_height);
            desired_size
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, mut final_size: PxSize) {
            final_size.height = self
                .max_height
                .get(ctx)
                .to_layout(ctx, AvailablePx::Finite(final_size.height))
                .min(final_size.height);

            self.child.arrange(ctx, final_size);
        }
    }
    MaxHeightNode {
        child,
        max_height: max_height.into_var(),
    }
}

/// Manually sets the size of the widget.
///
/// When set the widget is sized with the given value, independent of the parent available size.
///
/// If the width or height is set to [positive infinity](is_layout_any_size) then the normal layout measuring happens.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// button! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     content = text("200x300 red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed size of `200` width and `300` height.
///
/// # `width` and `height`
///
/// You can use the [`width`](fn@width) and [`height`](fn@height) properties to only set the size of one dimension.
#[property(size)]
pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
    struct SizeNode<T: UiNode, S: Var<Size>> {
        child: T,
        size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: Var<Size>> UiNode for SizeNode<T, S> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let size = self.size.get(ctx).to_layout(ctx, available_size);
            let _ = self.child.measure(ctx, AvailableSize::finite(size));
            size
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            let size = self.size.get(ctx).to_layout(ctx, AvailableSize::finite(final_size));
            self.child.arrange(ctx, size);
        }
    }
    SizeNode {
        child,
        size: size.into_var(),
    }
}

/// Exact width of the widget.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// button! {
///     background_color = rgb(255, 0, 0);
///     width = 200;
///     content = text("200x? red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed width of `200`.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
#[property(size)]
pub fn width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    struct WidthNode<T: UiNode, W: Var<Length>> {
        child: T,
        width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: Var<Length>> UiNode for WidthNode<T, W> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
            let width = self.width.get(ctx).to_layout(ctx, available_size.width);

            available_size.width = AvailablePx::Finite(width);

            self.child.measure(ctx, available_size)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, mut final_size: PxSize) {
            let width = self.width.get(ctx).to_layout(ctx, AvailablePx::Finite(final_size.width));
            final_size.width = width;
            self.child.arrange(ctx, final_size);
        }
    }
    WidthNode {
        child,
        width: width.into_var(),
    }
}

/// Exact height of the widget.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
/// button! {
///     background_color = rgb(255, 0, 0);
///     height = 300;
///     content = text("?x300 red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed size of `300` height.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
#[property(size)]
pub fn height(child: impl UiNode, height: impl IntoVar<Length>) -> impl UiNode {
    struct HeightNode<T: UiNode, H: Var<Length>> {
        child: T,
        height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: Var<Length>> UiNode for HeightNode<T, H> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, mut available_size: AvailableSize) -> PxSize {
            let height = self.height.get(ctx).to_layout(ctx, available_size.height);

            available_size.height = AvailablePx::Finite(height);

            self.child.measure(ctx, available_size)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, mut final_size: PxSize) {
            let height = self.height.get(ctx).to_layout(ctx, AvailablePx::Finite(final_size.height));
            final_size.height = height;
            self.child.arrange(ctx, final_size)
        }
    }
    HeightNode {
        child,
        height: height.into_var(),
    }
}

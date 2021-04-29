use crate::context::*;
use crate::impl_ui_node;
use crate::render::{FrameBuilder, FrameUpdate};
use crate::units::*;

unique_id! {
    /// Unique id of a widget.
    ///
    /// # Details
    /// Underlying value is a `NonZeroU64` generated using a relaxed global atomic `fetch_add`,
    /// so IDs are unique for the process duration, but order is not guaranteed.
    ///
    /// Panics if you somehow reach `u64::max_value()` calls to `new`.
    pub struct WidgetId;
}

/// An Ui tree node.
pub trait UiNode: 'static {
    /// Called every time the node is plugged in an Ui tree.
    fn init(&mut self, ctx: &mut WidgetContext);

    /// Called every time the node is unplugged from an Ui tree.
    fn deinit(&mut self, ctx: &mut WidgetContext);

    /// Called every time a low pressure event update happens.
    ///
    /// # Event Pressure
    /// See [`update_hp`](UiNode::update_hp) for more information about event pressure rate.
    fn update(&mut self, ctx: &mut WidgetContext);

    /// Called every time a high pressure event update happens.
    ///
    /// # Event Pressure
    /// Some events occur a lot more times then others, for performance reasons this
    /// event source may choose to be propagated in this high-pressure lane.
    ///
    /// Event sources that are high pressure mention this in their documentation.
    fn update_hp(&mut self, ctx: &mut WidgetContext);

    /// Called every time a layout update is needed.
    ///
    /// # Arguments
    /// * `available_size`: The total available size for the node. Can contain positive infinity to
    /// indicate the parent will accommodate [any size](crate::is_layout_any_size). Finite values are pixel aligned.
    /// * `ctx`: Measure context.
    ///
    /// # Return
    /// Return the nodes desired size. Must not contain infinity or NaN. Must be pixel aligned.
    fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize;

    /// Called every time a layout update is needed, after [`measure`](UiNode::measure).
    ///
    /// # Arguments
    /// * `final_size`: The size the parent node reserved for the node. Must reposition its contents
    /// to fit this size. The value does not contain infinity or NaNs and is pixel aligned.
    /// TODO args docs.
    fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize);

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Called every time a frame can be updated without fully rebuilding.
    ///
    /// # Arguments
    /// * `update`: Contains the frame value updates.
    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    /// Box this node, unless it is already `Box<dyn UiNode>`.
    fn boxed(self) -> Box<dyn UiNode>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}
#[impl_ui_node(delegate = self.as_ref(), delegate_mut = self.as_mut())]
impl UiNode for Box<dyn UiNode> {
    fn boxed(self) -> Box<dyn UiNode> {
        self
    }
}

macro_rules! declare_widget_test_calls {
    ($(
        $method:ident
    ),+) => {$(paste::paste! {
        #[doc = "<span class='stab portability' title='This is supported on `any(test, doc, feature=\"pub_test\")` only'><code>any(test, doc, feature=\"pub_test\")</code></span>"]
        #[doc = "Run [`UiNode::" $method "`] using the [`TestWidgetContext`]."]
        #[cfg(any(test, doc, feature = "pub_test"))]
        fn [<test_ $method>](&mut self, ctx: &mut TestWidgetContext) {
            // `self` already creates an `widget_context`, we assume, so this
            // call is for a dummy parent of `self`.
            ctx.widget_context(|ctx| {
                self.$method(ctx);
            });
        }
    })+};
}

/// Represents an widget [`UiNode`].
pub trait Widget: UiNode {
    /// Id of the widget.
    fn id(&self) -> WidgetId;

    /// Reference the widget lazy state.
    fn state(&self) -> &LazyStateMap;
    /// Exclusive borrow the widget lazy state.
    fn state_mut(&mut self) -> &mut LazyStateMap;

    /// Last arranged size.
    fn size(&self) -> LayoutSize;

    /// Box this widget node, unless it is already `Box<dyn Widget>`.
    fn boxed_widget(self) -> Box<dyn Widget>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    declare_widget_test_calls! {
        init, deinit, update, update_hp
    }

    /// <span class='stab portability' title='This is supported on `any(test, doc, feature="pub_test")` only'><code>any(test, doc, feature="pub_test")</code></span>
    /// Run [`UiNode::measure`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "pub_test"))]
    fn test_measure(&mut self, ctx: &mut TestWidgetContext, available_size: LayoutSize) -> LayoutSize {
        ctx.layout_context(14.0, 14.0, self.size(), PixelGrid::new(1.0), |ctx| {
            self.measure(ctx, available_size)
        })
    }
    /// <span class='stab portability' title='This is supported on `any(test, doc, feature="pub_test")` only'><code>any(test, doc, feature="pub_test")</code></span>
    /// Run [`UiNode::arrange`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "pub_test"))]
    fn test_arrange(&mut self, ctx: &mut TestWidgetContext, final_size: LayoutSize) {
        ctx.layout_context(14.0, 14.0, self.size(), PixelGrid::new(1.0), |ctx| self.arrange(ctx, final_size))
    }

    // TODO don't require user to init frame?

    /// <span class='stab portability' title='This is supported on `any(test, doc, feature="pub_test")` only'><code>any(test, doc, feature="pub_test")</code></span>
    /// Run [`UiNode::render`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "pub_test"))]
    fn test_render(&self, ctx: &mut TestWidgetContext, frame: &mut FrameBuilder) {
        ctx.render_context(|ctx| self.render(ctx, frame));
    }

    /// <span class='stab portability' title='This is supported on `any(test, doc, feature="pub_test")` only'><code>any(test, doc, feature="pub_test")</code></span>
    /// Run [`UiNode::render_update`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "pub_test"))]
    fn test_render_update(&self, ctx: &mut TestWidgetContext, update: &mut FrameUpdate) {
        ctx.render_context(|ctx| self.render_update(ctx, update));
    }
}

#[impl_ui_node(delegate = self.as_ref(), delegate_mut = self.as_mut())]
impl UiNode for Box<dyn Widget> {}
impl Widget for Box<dyn Widget> {
    #[inline]
    fn id(&self) -> WidgetId {
        self.as_ref().id()
    }
    #[inline]
    fn state(&self) -> &LazyStateMap {
        self.as_ref().state()
    }
    #[inline]
    fn state_mut(&mut self) -> &mut LazyStateMap {
        self.as_mut().state_mut()
    }
    #[inline]
    fn size(&self) -> LayoutSize {
        self.as_ref().size()
    }
    #[inline]
    fn boxed_widget(self) -> Box<dyn Widget> {
        self
    }
}

/// A UI node that does not contain any other node, does not take any space and renders nothing.
pub struct NilUiNode;
#[impl_ui_node(none)]
impl UiNode for NilUiNode {
    fn measure(&mut self, _: &mut LayoutContext, _: LayoutSize) -> LayoutSize {
        LayoutSize::zero()
    }
}

/// A UI node that does not contain any other node, fills the available space, but renders nothing.
pub struct FillUiNode;
#[impl_ui_node(none)]
impl UiNode for FillUiNode {}

// Used by #[impl_ui_node] to validate custom delegation.
#[doc(hidden)]
pub mod impl_ui_node_util {
    use crate::{
        context::{LayoutContext, RenderContext, WidgetContext},
        render::{FrameBuilder, FrameUpdate},
        units::LayoutSize,
        UiNode, UiNodeList,
    };

    #[inline]
    pub fn delegate(d: &(impl UiNode + ?Sized)) -> &(impl UiNode + ?Sized) {
        d
    }
    #[inline]
    pub fn delegate_mut(d: &mut (impl UiNode + ?Sized)) -> &mut (impl UiNode + ?Sized) {
        d
    }

    #[inline]
    pub fn delegate_list(d: &(impl UiNodeList + ?Sized)) -> &(impl UiNodeList + ?Sized) {
        d
    }
    #[inline]
    pub fn delegate_list_mut(d: &mut (impl UiNodeList + ?Sized)) -> &mut (impl UiNodeList + ?Sized) {
        d
    }

    #[inline]
    pub fn delegate_iter<'a>(d: impl IntoIterator<Item = &'a impl UiNode>) -> impl IterImpl {
        d
    }
    #[inline]
    pub fn delegate_iter_mut<'a>(d: impl IntoIterator<Item = &'a mut impl UiNode>) -> impl IterMutImpl {
        d
    }

    pub trait IterMutImpl {
        fn init_all(self, ctx: &mut WidgetContext);
        fn deinit_all(self, ctx: &mut WidgetContext);
        fn update_all(self, ctx: &mut WidgetContext);
        fn update_hp_all(self, ctx: &mut WidgetContext);
        fn measure_all(self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize;
        fn arrange_all(self, ctx: &mut LayoutContext, final_size: LayoutSize);
    }
    pub trait IterImpl {
        fn render_all(self, ctx: &mut RenderContext, frame: &mut FrameBuilder);
        fn render_update_all(self, ctx: &mut RenderContext, update: &mut FrameUpdate);
    }

    impl<'u, U: UiNode, I: IntoIterator<Item = &'u mut U>> IterMutImpl for I {
        fn init_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.init(ctx);
            }
        }

        fn deinit_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.deinit(ctx);
            }
        }

        fn update_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.update(ctx);
            }
        }

        fn update_hp_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.update_hp(ctx);
            }
        }

        fn measure_all(self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
            let mut size = LayoutSize::zero();
            for child in self {
                size = child.measure(ctx, available_size).max(size);
            }
            size
        }

        fn arrange_all(self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            for child in self {
                child.arrange(ctx, final_size);
            }
        }
    }

    impl<'u, U: UiNode, I: IntoIterator<Item = &'u U>> IterImpl for I {
        fn render_all(self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            for child in self {
                child.render(ctx, frame);
            }
        }

        fn render_update_all(self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            for child in self {
                child.render_update(ctx, update);
            }
        }
    }
}

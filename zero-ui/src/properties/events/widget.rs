//! Widget events, [`on_init`](fn@on_init), [`on_update`](fn@on_update), [`on_render`](fn@on_render) and more.
//!
//! These events map very close to the [`UiNode`] methods. The event handler have non-standard signatures
//! and the event does not consider the widget's [`interactivity`](crate::core::widget_info::WidgetInfo::interactivity).

use zero_ui_core::widget_info::UpdateSlot;

use crate::core::handler::*;
use crate::core::render::FrameBuilder;
use crate::core::units::*;
use crate::core::widget_info::{WidgetLayout, WidgetSubscriptions};
use crate::core::*;
use crate::core::{
    context::{InfoContext, LayoutContext, MeasureContext, RenderContext, WidgetContext},
    render::FrameUpdate,
};

/// Arguments for the [`on_init`](fn@on_init) event.
#[derive(Clone, Debug, Copy)]
pub struct OnInitArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`init`](UiNode::init) event.
///
/// This property calls `handler` when the widget and its content initializes. Note that widgets
/// can be [deinitialized](fn@on_deinit) and reinitialized, so the `handler` can be called more then once,
/// you can use one of the *once* handlers to only be called once or use the arguments [`count`](OnInitArgs::count).
/// to determinate if you are in the first init.
///
/// Note that the widget is not in the [`WidgetInfoTree`] when this event happens, you can use [`on_info_init`] for initialization
/// that depends on the widget info.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
///
/// [`on_info_init`]: fn@on_info_init
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    struct OnInitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C, H> UiNode for OnInitNode<C, H>
    where
        C: UiNode,
        H: WidgetHandler<OnInitArgs>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnInitArgs { count: self.count });
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.handler(&self.handler);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            self.handler.update(ctx);
        }
    }

    OnInitNode { child, handler, count: 0 }
}

/// Preview [`on_init`] event.
///
/// This property calls `handler` when the widget initializes, before the widget content initializes. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
///
/// [`on_init`]: fn@on_init
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_pre_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    struct OnPreviewInitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnInitArgs>> UiNode for OnPreviewInitNode<C, H> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnInitArgs { count: self.count });

            self.child.init(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.handler(&self.handler);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);
            self.child.update(ctx);
        }
    }

    OnPreviewInitNode { child, handler, count: 0 }
}

/// Widget inited and info collected event.
///
/// This event fires after the first [`UiNode::info`] construction after [`UiNode::init`]. This event can be used when
/// some widget initialization needs to happen, but the widget must be in the [`WidgetInfoTree`] for it to work.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_info_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    struct OnInfoInitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
        pending: Option<UpdateSlot>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnInitArgs>> UiNode for OnInfoInitNode<C, H> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            let slot = UpdateSlot::next();
            ctx.updates.update(slot.mask());
            self.pending = Some(slot);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            if let Some(s) = self.pending {
                subs.update(s);
            }
            subs.handler(&self.handler);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if self.pending.take().is_some() {
                ctx.updates.subscriptions();
                self.count = self.count.wrapping_add(1);
                self.handler.event(ctx, &OnInitArgs { count: self.count });
            }

            self.handler.update(ctx);
        }
    }
    OnInfoInitNode {
        child,
        handler,
        count: 0,
        pending: None,
    }
}

/// Arguments for the [`on_update`](fn@on_update) event.
#[derive(Clone, Debug, Copy)]
pub struct OnUpdateArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`update`](UiNode::update) event.
///
/// This property calls `handler` every UI update, after the widget content updates. Updates happen in
/// high volume in between idle moments, so the handler code should be considered a *hot-path*.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers are not permitted here because of the high volume of calls and because async tasks cause an
/// UI update every time they awake, so it is very easy to lock the app in a constant sequence of updates.
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_update(child: impl UiNode, handler: impl WidgetHandler<OnUpdateArgs> + marker::NotAsyncHn) -> impl UiNode {
    struct OnUpdateNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnUpdateArgs>> UiNode for OnUpdateNode<C, H> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            self.child.subscriptions(ctx, subs);
            subs.handler(&self.handler);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnUpdateArgs { count: self.count });
        }
    }

    OnUpdateNode { child, handler, count: 0 }
}

/// Preview [`on_update`] event.
///
/// This property calls `handler` every time the UI updates, before the widget content updates. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers are not permitted here because of the high volume of calls and because async tasks cause an
/// UI update every time they awake, so it is very easy to lock the app in a constant sequence of updates.
///
/// [`on_update`]: fn@on_update
/// [`on_init`]: fn@on_init
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_pre_update(child: impl UiNode, handler: impl WidgetHandler<OnUpdateArgs> + marker::NotAsyncHn) -> impl UiNode {
    struct OnPreviewUpdateNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnUpdateArgs>> UiNode for OnPreviewUpdateNode<C, H> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.handler(&self.handler);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnUpdateArgs { count: self.count });

            self.child.update(ctx);
        }
    }

    OnPreviewUpdateNode { child, handler, count: 0 }
}

/// Arguments for the [`on_deinit`](fn@on_deinit) event.
#[derive(Clone, Debug, Copy)]
pub struct OnDeinitArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`deinit`](UiNode::deinit) event.
///
/// This property calls `handler` when the widget deinits, after the widget content deinits. Note that
/// widgets can be [reinitialized](fn@on_init) so the `handler` can be called more then once,
/// you can use one of the *once* handlers to only be called once or use the arguments [`count`](OnDeinitArgs::count)
/// to determinate if you are in the first deinit.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers are not permitted here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but we are deiniting the widget, probably about to drop it. You can start an UI bound
/// async task in the app context using [`WidgetContext::async_task`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_deinit(child: impl UiNode, handler: impl WidgetHandler<OnDeinitArgs> + marker::NotAsyncHn) -> impl UiNode {
    struct OnDeinitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnDeinitArgs>> UiNode for OnDeinitNode<C, H> {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnDeinitArgs { count: self.count });
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            self.child.subscriptions(ctx, subs);
            subs.handler(&self.handler);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            self.handler.update(ctx);
        }
    }

    OnDeinitNode { child, handler, count: 0 }
}

/// Preview [`on_update`] event.
///
/// This property calls `handler` every time the UI updates, before the widget content updates. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers are not permitted here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but we are deiniting the widget, probably about to drop it. You can start an UI bound
/// async task in the app context using [`WidgetContext::async_task`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
///
/// [`on_update`]: fn@on_update
/// [`on_init`]: fn@on_init
#[property(event,  default( hn!(|_, _|{}) ))]
pub fn on_pre_deinit(child: impl UiNode, handler: impl WidgetHandler<OnDeinitArgs> + marker::NotAsyncHn) -> impl UiNode {
    struct OnPreviewDeinitNode<C, H> {
        child: C,
        handler: H,
        count: usize,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, H: WidgetHandler<OnDeinitArgs>> UiNode for OnPreviewDeinitNode<C, H> {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnDeinitArgs { count: self.count });

            self.child.deinit(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            self.child.subscriptions(ctx, subs);
            subs.handler(&self.handler);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);
            self.child.update(ctx);
        }
    }

    OnPreviewDeinitNode { child, handler, count: 0 }
}

/// Arguments of the [`on_measure`](fn@on_measure) event.
pub struct OnMeasureArgs {
    /// The measured size for the widget's outer bounds.
    pub size: PxSize,
}

/// Event fired during the widget [`measure`](UiNode::measure).
///
/// The `handler` is called after the [preview event](fn@on_pre_measure) and after the widget children.
/// The inputs are the measure context and [`OnMeasureArgs`].
///
/// The `handler` is called even if the widget has [`Interactivity::BLOCK`].
#[property(event, default(|_, _|{}))]
pub fn on_measure(child: impl UiNode, handler: impl Fn(&mut MeasureContext, OnMeasureArgs) + 'static) -> impl UiNode {
    struct OnMeasureNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut MeasureContext, OnMeasureArgs) + 'static> UiNode for OnMeasureNode<C, F> {
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let size = self.child.measure(ctx);
            (self.handler)(ctx, OnMeasureArgs { size });
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            self.child.layout(ctx, wl)
        }
    }
    OnMeasureNode { child, handler }
}

/// Preview [`on_measure`] event.
///
/// The `handler` is called before the main event and before the widget children. The input is the measure context.
///
/// The `handler` is called every time, independent of the widget's interactivity.
///
/// [disabled]: crate::core::widget_base::IsEnabled
/// [`on_layout`]: fn@on_layout
#[property(event, default(|_|{}))]
pub fn on_pre_measure(child: impl UiNode, handler: impl Fn(&mut MeasureContext) + 'static) -> impl UiNode {
    struct OnMeasureNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut MeasureContext) + 'static> UiNode for OnMeasureNode<C, F> {
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            (self.handler)(ctx);
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            self.child.layout(ctx, wl)
        }
    }
    OnMeasureNode { child, handler }
}

/// Arguments of the [`on_layout`](fn@on_layout) event.
pub struct OnLayoutArgs<'a> {
    /// The layout builder.
    pub wl: &'a mut WidgetLayout,

    /// The calculated size for the widget outer bounds.
    pub size: PxSize,
}

/// Event fired during the widget [`layout`](UiNode::layout).
///
/// The `handler` is called after the [preview event](fn@on_pre_layout) and after the widget children.
/// The inputs are layout context and [`OnLayoutArgs`].
///
/// The `handler` is called even if the widget has [`Interactivity::BLOCK`].
#[property(event, default(|_, _|{}))]
pub fn on_layout(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, OnLayoutArgs) + 'static) -> impl UiNode {
    struct OnLayoutNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, OnLayoutArgs) + 'static> UiNode for OnLayoutNode<C, F> {
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.child.layout(ctx, wl);
            (self.handler)(ctx, OnLayoutArgs { wl, size });
            size
        }
    }
    OnLayoutNode { child, handler }
}

/// Preview [`on_layout`] event.
///
/// The `handler` is called before the main event and before the widget children. The inputs are the layout context
/// and layout builder.
///
/// The `handler` is called every time, independent of the widget's interactivity.
///
/// [disabled]: crate::core::widget_base::IsEnabled
/// [`on_layout`]: fn@on_layout
#[property(event, default(|_, _|{}))]
pub fn on_pre_layout(child: impl UiNode, handler: impl FnMut(&mut LayoutContext, &mut WidgetLayout) + 'static) -> impl UiNode {
    struct OnPreviewLayoutNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: FnMut(&mut LayoutContext, &mut WidgetLayout) + 'static> UiNode for OnPreviewLayoutNode<C, F> {
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            (self.handler)(ctx, wl);
            self.child.layout(ctx, wl)
        }
    }
    OnPreviewLayoutNode { child, handler }
}

/// Event fired during the widget [`render`](UiNode::render).
///
/// The `handler` is called after the [preview event](fn@on_pre_render) and after the widget children. That means that
/// display items added by the `handler` are rendered on top of the widget visual.
///
/// The `handler` is called every time, independent of the widget's interactivity.
///
/// [disabled]: crate::core::widget_base::IsEnabled
#[property(event, default(|_, _|{}))]
pub fn on_render(child: impl UiNode, handler: impl Fn(&mut RenderContext, &mut FrameBuilder) + 'static) -> impl UiNode {
    struct OnRenderNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut RenderContext, &mut FrameBuilder) + 'static> UiNode for OnRenderNode<C, F> {
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);
            (self.handler)(ctx, frame);
        }
    }
    OnRenderNode { child, handler }
}
/// Preview [`on_render`] event.
///
/// The `handler` is called before the main event and before the widget children. That means that
/// display items added by the `handler` are rendered as background of the widget visual.
///
/// The `handler` is called every time, independent of the widget's interactivity.
///
/// [disabled]: crate::core::widget_base::IsEnabled
/// [`on_render`]: fn@on_render
#[property(event, default(|_, _|{}))]
pub fn on_pre_render(child: impl UiNode, handler: impl Fn(&mut RenderContext, &mut FrameBuilder) + 'static) -> impl UiNode {
    struct OnPreviewRenderNode<C: UiNode, F: Fn(&mut RenderContext, &mut FrameBuilder)> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut RenderContext, &mut FrameBuilder) + 'static> UiNode for OnPreviewRenderNode<C, F> {
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            (self.handler)(ctx, frame);
            self.child.render(ctx, frame);
        }
    }
    OnPreviewRenderNode { child, handler }
}

/// Event fired during the widget [`render_update`](UiNode::render_update).
///
/// The `handler` is called after the [preview event](fn@on_pre_render_update) and after the widget children.
///
/// The `handler` is called every time, independent of the widget's interactivity.
///
/// [disabled]: crate::core::widget_base::IsEnabled
#[property(event, default(|_, _|{}))]
pub fn on_render_update(child: impl UiNode, handler: impl Fn(&mut RenderContext, &mut FrameUpdate) + 'static) -> impl UiNode {
    struct OnRenderUpdateNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut RenderContext, &mut FrameUpdate) + 'static> UiNode for OnRenderUpdateNode<C, F> {
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.child.render_update(ctx, update);
            (self.handler)(ctx, update);
        }
    }
    OnRenderUpdateNode { child, handler }
}
/// Preview [`on_render_update`] event.
///
/// The `handler` is called before the main event and before the widget children.
///
/// The `handler` is called event when widget is [disabled].
///
/// [disabled]: crate::core::widget_base::IsEnabled
/// [`on_render_update`]: fn@on_render_update
#[property(event, default(|_, _|{}))]
pub fn on_pre_render_update(child: impl UiNode, handler: impl Fn(&mut RenderContext, &mut FrameUpdate) + 'static) -> impl UiNode {
    struct OnPreviewRenderUpdateNode<C, F> {
        child: C,
        handler: F,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Fn(&mut RenderContext, &mut FrameUpdate) + 'static> UiNode for OnPreviewRenderUpdateNode<C, F> {
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            (self.handler)(ctx, update);
            self.child.render_update(ctx, update);
        }
    }
    OnPreviewRenderUpdateNode { child, handler }
}

use crate::core::app::{AppEvent, AppExtension};
use crate::core::context::*;
use crate::core::event::*;
use crate::core::frame::{FrameBuilder, FrameHitInfo};
use crate::core::types::*;
use crate::core::var::*;
use crate::core::UiNode;
use gleam::gl;
use glutin::dpi::LogicalSize;
use glutin::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest};
use glutin::{NotCurrent, WindowedContext};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::borrow::Cow;
use std::sync::Arc;
use std::time::Instant;
use webrender::api::*;

event_args! {
    /// [WindowOpen], [WindowClose] event args.
    pub struct WindowEventArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        fn concerns_widget(&self, ctx: &mut WidgetContext) {
            //! If the widget is in the same window.
            ctx.window_id == self.window_id
        }
    }

    /// [WindowResize] event args.
    pub struct WindowResizeArgs {
        pub window_id: WindowId,
        pub new_size: LayoutSize,

        fn concerns_widget(&self, ctx: &mut WidgetContext) {
            //! If the widget is in the same window.
            ctx.window_id == self.window_id
        }
    }

    /// [WindowMove] event args.
    pub struct WindowMoveArgs {
        pub window_id: WindowId,
        pub new_position: LayoutPoint,

        fn concerns_widget(&self, ctx: &mut WidgetContext) {
            //! If the widget is in the same window.
            ctx.window_id == self.window_id
        }
    }

    /// [WindowScaleChanged] event args.
    pub struct WindowScaleChangedArgs {
        pub window_id: WindowId,
        pub new_scale_factor: f32,
        pub new_size: LayoutSize,

        fn concerns_widget(&self, ctx: &mut WidgetContext) {
            //! If the widget is in the same window.
            ctx.window_id == self.window_id
        }
    }
}
cancelable_event_args! {
    /// [WindowClosing] event args.
    pub struct WindowClosingArgs {
        pub window_id: WindowId,

        fn concerns_widget(&self, ctx: &mut WidgetContext) {
            //! If the widget is in the same window.
            ctx.window_id == self.window_id
        }
    }
}

/// New window event.
pub struct WindowOpen;
impl Event for WindowOpen {
    type Args = WindowEventArgs;
}

/// Window resized event.
pub struct WindowResize;
impl Event for WindowResize {
    type Args = WindowResizeArgs;

    const IS_HIGH_PRESSURE: bool = true;
}

/// Window moved event.
pub struct WindowMove;
impl Event for WindowMove {
    type Args = WindowMoveArgs;

    const IS_HIGH_PRESSURE: bool = true;
}

/// Window scale factor changed.
pub struct WindowScaleChanged;
impl Event for WindowScaleChanged {
    type Args = WindowScaleChangedArgs;
}

/// Closing window event.
pub struct WindowClosing;
impl Event for WindowClosing {
    type Args = WindowClosingArgs;
}
impl CancelableEvent for WindowClosing {
    type Args = WindowClosingArgs;
}

/// Closed window event.
pub struct WindowClose;
impl Event for WindowClose {
    type Args = WindowEventArgs;
}

/// Windows management [AppExtension].
pub struct AppWindows {
    event_loop_proxy: Option<EventLoopProxy<AppEvent>>,
    ui_threads: Arc<ThreadPool>,
    windows: Vec<GlWindow>,
    window_open: EventEmitter<WindowEventArgs>,
    window_resize: EventEmitter<WindowResizeArgs>,
    window_move: EventEmitter<WindowMoveArgs>,
    window_scale_changed: EventEmitter<WindowScaleChangedArgs>,
    window_closing: EventEmitter<WindowClosingArgs>,
    window_close: EventEmitter<WindowEventArgs>,
}

impl Default for AppWindows {
    fn default() -> Self {
        let ui_threads = Arc::new(
            ThreadPoolBuilder::new()
                .thread_name(|idx| format!("UI#{}", idx))
                .start_handler(|_| {
                    #[cfg(feature = "app_profiler")]
                    crate::core::profiler::register_thread_with_profiler();
                })
                .build()
                .unwrap(),
        );

        AppWindows {
            event_loop_proxy: None,
            ui_threads,
            windows: Vec::with_capacity(1),
            window_open: EventEmitter::new(false),
            window_resize: EventEmitter::new(true),
            window_move: EventEmitter::new(true),
            window_scale_changed: EventEmitter::new(false),
            window_closing: EventEmitter::new(false),
            window_close: EventEmitter::new(false),
        }
    }
}

impl AppWindows {
    fn window_index(&self, window_id: WindowId) -> Option<usize> {
        self.windows.iter().position(|w| w.id() == window_id)
    }

    fn contains_window(&self, window_id: WindowId) -> bool {
        self.windows.iter().any(|w| w.id() == window_id)
    }

    fn window_mut(&mut self, window_id: WindowId) -> Option<&mut GlWindow> {
        self.windows.iter_mut().find(|w| w.id() == window_id)
    }
}

impl AppExtension for AppWindows {
    fn init(&mut self, r: &mut AppInitContext) {
        self.event_loop_proxy = Some(r.event_loop.clone());
        r.services.register(Windows::new(r.updates.notifier().clone()));
        r.events.register::<WindowOpen>(self.window_open.listener());
        r.events.register::<WindowResize>(self.window_resize.listener());
        r.events.register::<WindowMove>(self.window_move.listener());
        r.events.register::<WindowScaleChanged>(self.window_scale_changed.listener());
        r.events.register::<WindowClosing>(self.window_closing.listener());
        r.events.register::<WindowClose>(self.window_close.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match event {
            WindowEvent::Resized(new_size) => {
                let new_size = LayoutSize::new(new_size.width as f32, new_size.height as f32);

                // raise window_resize
                ctx.updates
                    .push_notify(self.window_resize.clone(), WindowResizeArgs::now(window_id, new_size));

                // set the window size variable if it is not read-only.
                if let Some(window) = self.window_mut(window_id) {
                    let _ = ctx.updates.push_set(&window.root.size, new_size);
                }
            }
            WindowEvent::Moved(new_position) => {
                let new_position = LayoutPoint::new(new_position.x as f32, new_position.y as f32);
                ctx.updates
                    .push_notify(self.window_move.clone(), WindowMoveArgs::now(window_id, new_position))
            }
            WindowEvent::CloseRequested => {
                if self.contains_window(window_id) {
                    ctx.updates
                        .push_notify(self.window_closing.clone(), WindowClosingArgs::now(window_id))
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                if self.contains_window(window_id) {
                    ctx.updates.push_notify(
                        self.window_scale_changed.clone(),
                        WindowScaleChangedArgs::now(
                            window_id,
                            *scale_factor as f32,
                            LayoutSize::new(new_inner_size.width as f32, new_inner_size.height as f32),
                        ),
                    )
                }
            }
            _ => {}
        }
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, _: &mut AppContext) {
        if let Some(window) = self.window_mut(window_id) {
            window.request_redraw();
        }
    }

    fn on_redraw_requested(&mut self, window_id: WindowId, _: &mut AppContext) {
        if let Some(window) = self.window_mut(window_id) {
            window.redraw();
        }
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        // respond to service requests
        let requests = ctx.services.req::<Windows>().take_requests();
        for request in requests {
            let mut w = GlWindow::new(
                request.new,
                ctx,
                ctx.event_loop,
                self.event_loop_proxy.as_ref().unwrap().clone(),
                Arc::clone(&self.ui_threads),
            );

            let args = WindowEventArgs {
                timestamp: Instant::now(),
                window_id: w.id(),
            };

            w.init(ctx);
            self.windows.push(w);

            // notify the window requester
            ctx.updates.push_notify(request.notifier, args.clone());

            // notify everyone
            ctx.updates.push_notify(self.window_open.clone(), args.clone());
        }

        // notify and respond to updates
        if update.update_hp {
            for window in self.windows.iter_mut() {
                window.update_hp(ctx);
            }
        }
        if update.update {
            for window in self.windows.iter_mut() {
                window.update(ctx);
            }

            // respond to window_closing events
            for closing in self.window_closing.updates(ctx.events) {
                if !closing.cancel_requested() && self.contains_window(closing.window_id) {
                    // not canceled and we can close the window.
                    // notify close, the window will be deinited on
                    // the next update.
                    ctx.updates
                        .push_notify(self.window_close.clone(), WindowEventArgs::now(closing.window_id));
                }
            }

            // respond to window_close events
            for close in self.window_close.updates(ctx.events) {
                if let Some(i) = self.window_index(close.window_id) {
                    self.windows.remove(i).deinit(ctx);
                }
            }
        }
    }

    fn update_display(&mut self, _: UpdateDisplayRequest) {
        // Pump layout and render in all windows.
        // The windows don't do an update unless they recorded
        // an update request for layout or render.
        for window in self.windows.iter_mut() {
            window.layout();
            window.render();
        }
    }
}

struct NewWindowRequest {
    new: Box<dyn FnOnce(&AppContext) -> UiRoot>,
    notifier: EventEmitter<WindowEventArgs>,
}

/// Windows service.
pub struct Windows {
    requests: Vec<NewWindowRequest>,
    update_notifier: UpdateNotifier,
}

impl AppService for Windows {}

impl Windows {
    fn new(update_notifier: UpdateNotifier) -> Self {
        Windows {
            requests: Vec::default(),
            update_notifier,
        }
    }

    fn take_requests(&mut self) -> Vec<NewWindowRequest> {
        std::mem::replace(&mut self.requests, Vec::default())
    }

    /// Requests a new window. Returns a notice that gets updated once
    /// when the window is launched.
    pub fn new_window(&mut self, new_window: impl FnOnce(&AppContext) -> UiRoot + 'static) -> EventListener<WindowEventArgs> {
        let request = NewWindowRequest {
            new: Box::new(new_window),
            notifier: EventEmitter::new(false),
        };
        let notice = request.notifier.listener();
        self.requests.push(request);

        self.update_notifier.push_update();

        notice
    }

    pub fn hit_test(&mut self, window_id: WindowId, point: LayoutPoint) -> FrameHitInfo {
        todo!()
    }
}

#[derive(Clone)]
struct Notifier {
    window_id: WindowId,
    event_loop: EventLoopProxy<AppEvent>,
}
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Clone::clone(self))
    }

    fn wake_up(&self) {}

    fn new_frame_ready(&self, _: DocumentId, _scrolled: bool, _composite_needed: bool, _: Option<u64>) {
        let _ = self.event_loop.send_event(AppEvent::NewFrameReady(self.window_id));
    }
}

struct GlWindow {
    context: Option<WindowedContext<NotCurrent>>,
    renderer: webrender::Renderer,
    api: Arc<RenderApi>,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    state: WindowState,
    services: WindowServices,

    root: UiRoot,
    update: UpdateDisplayRequest,
    first_draw: bool,

    latest_frame_id: Epoch,
}

impl GlWindow {
    pub fn new(
        new_window: Box<dyn FnOnce(&AppContext) -> UiRoot>,
        ctx: &mut AppContext,
        event_loop: &EventLoopWindowTarget<AppEvent>,
        event_loop_proxy: EventLoopProxy<AppEvent>,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        let root = new_window(ctx);
        let inner_size = *root.size.get(ctx.vars);
        let clear_color = *root.background_color.get(ctx.vars);

        let window_builder = WindowBuilder::new()
            .with_visible(false) // not visible until first render, to flickering
            .with_inner_size(LogicalSize::<f64>::new(inner_size.width.into(), inner_size.height.into()));

        let context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(window_builder, &event_loop)
            .unwrap();

        // SAFETY: This is the recomended way of doing it.
        let context = unsafe { context.make_current().unwrap() };

        let gl = match context.get_api() {
            Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::WebGl => panic!("WebGl is not supported"),
        };

        let dpi_factor = context.window().scale_factor() as f32;

        let opts = webrender::RendererOptions {
            device_pixel_ratio: dpi_factor,
            clear_color: Some(clear_color),
            workers: Some(ui_threads),
            ..webrender::RendererOptions::default()
        };

        let notifier = Box::new(Notifier {
            window_id: context.window().id(),
            event_loop: event_loop_proxy,
        });

        let start_size = inner_size * units::LayoutToDeviceScale::new(dpi_factor);
        let start_size = units::DeviceIntSize::new(start_size.width as i32, start_size.height as i32);
        let (renderer, sender) = webrender::Renderer::new(gl.clone(), notifier, opts, None, start_size).unwrap();
        let api = Arc::new(sender.create_api());
        let document_id = api.add_document(start_size, 0);

        let id = context.window().id();
        let (state, services) = ctx.new_window(id, &api);

        GlWindow {
            context: Some(unsafe { context.make_not_current().unwrap() }),
            renderer,
            api,
            pipeline_id: PipelineId(1, 0),
            document_id,
            state,
            services,

            root,
            update: UpdateDisplayRequest::Layout,
            first_draw: true,

            latest_frame_id: Epoch(0),
        }
    }

    pub fn id(&self) -> WindowId {
        self.context.as_ref().unwrap().window().id()
    }

    fn root_context(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut Box<dyn UiNode>, &mut WidgetContext)) -> UpdateDisplayRequest {
        let id = self.id();
        let root = &mut self.root;

        ctx.window_context(id, &mut self.state, &mut self.services, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.widget_context(root.id, &mut root.state, |ctx| {
                f(child, ctx);
            });
        })
    }

    pub fn init(&mut self, ctx: &mut AppContext) {
        let update = self.root_context(ctx, |root, ctx| {
            ctx.updates.push_layout();

            root.init(ctx);
        });
        self.update |= update;
    }

    pub fn update_hp(&mut self, ctx: &mut AppContext) {
        let update = self.root_context(ctx, |root, ctx| root.update_hp(ctx));
        self.update |= update;
    }

    pub fn update(&mut self, ctx: &mut AppContext) {
        // do winit window updates
        let window = self.context.as_ref().unwrap().window();
        if let Some(title) = self.root.title.update(ctx.vars) {
            window.set_title(title);
        }

        // do UiNode updates
        let update = self.root_context(ctx, |root, ctx| root.update_hp(ctx));
        self.update |= update;
    }

    pub fn layout(&mut self) {
        if self.update == UpdateDisplayRequest::Layout {
            self.update = UpdateDisplayRequest::Render;

            let available_size = self.context.as_ref().unwrap().window().inner_size();
            let available_size = LayoutSize::new(available_size.width as f32, available_size.height as f32);

            let desired_size = self.root.child.measure(available_size);

            let final_size = desired_size.min(available_size);

            self.root.child.arrange(final_size);
        }
    }

    pub fn render(&mut self) {
        if self.update == UpdateDisplayRequest::Render {
            self.update = UpdateDisplayRequest::None;

            let size = self.context.as_ref().unwrap().window().inner_size();
            let size = LayoutSize::new(size.width as f32, size.height as f32);

            let mut frame = FrameBuilder::new(self.root.id, size, self.pipeline_id);
            self.root.child.render(&mut frame);

            let (display_list_data, frame_info) = frame.finalize();
            //TODO - Use frame_info

            self.latest_frame_id = Epoch({
                let mut next = self.latest_frame_id.0.wrapping_add(1);
                if next == Epoch::invalid().0 {
                    next = next.wrapping_add(1);
                }
                next
            });

            let mut txn = Transaction::new();
            txn.set_display_list(self.latest_frame_id, None, size, display_list_data, true);
            txn.set_root_pipeline(self.pipeline_id);
            txn.generate_frame();
            self.api.send_transaction(self.document_id, txn);
        }
    }

    /// Notifies the OS to redraw the window, will receive WindowEvent::RedrawRequested
    /// from the OS after calling this.
    pub fn request_redraw(&mut self) {
        let context = self.context.as_ref().unwrap();
        if self.first_draw {
            context.window().set_visible(true); // OS generates a RequestRedraw here
            self.first_draw = false;
        } else {
            context.window().request_redraw();
        }
    }

    /// Redraws the last ready frame and swaps buffers.
    ///
    /// **`swap_buffers` Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    pub fn redraw(&mut self) {
        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };

        self.renderer.update();

        let size = context.window().inner_size();
        let size = LayoutSize::new(size.width as f32, size.height as f32);
        let dpi_factor = context.window().scale_factor() as f32;

        let device_size = size * units::LayoutToDeviceScale::new(dpi_factor);
        let device_size = units::DeviceIntSize::new(device_size.width as i32, device_size.height as i32);

        self.renderer.render(device_size).unwrap();
        let _ = self.renderer.flush_pipeline_info();

        context.swap_buffers().ok();

        self.context = Some(unsafe { context.make_not_current().unwrap() });
    }

    pub fn deinit(mut self, ctx: &mut AppContext) {
        self.root_context(ctx, |root, ctx| root.deinit(ctx));

        let context = unsafe { self.context.take().unwrap().make_current().unwrap() };
        self.renderer.deinit();
        unsafe { context.make_not_current().unwrap() };
    }
}

pub struct UiRoot {
    id: WidgetId,
    state: LazyStateMap,
    title: BoxVar<Cow<'static, str>>,
    size: SharedVar<LayoutSize>,
    background_color: BoxVar<ColorF>,
    child: Box<dyn UiNode>,
}

// TODO widget like window! macro
//fn window(
//    child: impl UiNode,
//    title: impl IntoVar<Cow<'static, str>>,
//    size: impl Into<SharedVar<LayoutSize>>,
//    background_color: impl IntoVar<ColorF>,
//) -> UiRoot {
//    UiRoot {
//        title: Box::new(title.into_var()),
//        size: size.into(),
//        background_color: Box::new(background_color.into_var()),
//        child: Box::new(child),
//    }
//}

//! App windows manager.

use super::event::{cancelable_event_args, event_args, CancelableEvent, CancelableEventArgs, Event, EventEmitter, EventListener};
use super::profiler::profile_scope;
use super::{
    app::{self, EventLoopProxy, EventLoopWindowTarget, ShutdownRequestedArgs},
    context::{
        AppContext, AppInitContext, AppService, LazyStateMap, UpdateDisplayRequest, UpdateNotifier, UpdateRequest, Updates, Vars,
        WidgetContext, WindowServices, WindowState,
    },
    render::{FrameBuilder, FrameHitInfo, FrameInfo},
    types::{ColorF, FrameId, LayoutPoint, LayoutRect, LayoutSize, PixelGrid, Text, WidgetId, WindowEvent, WindowId},
    var::{BoxLocalVar, BoxVar, IntoVar, ObjVar},
    UiNode,
};
use app::{AppExtended, AppExtension, AppProcess};
use fnv::FnvHashMap;
use gleam::gl;
use glutin::{
    dpi::LogicalSize as WLogicalSize,
    window::{Window as GlutinWindow, WindowBuilder},
    Api, ContextBuilder, GlRequest, NotCurrent, PossiblyCurrent, WindowedContext,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::cell::RefCell;
use std::num::NonZeroU16;
use std::{mem, rc::Rc, sync::Arc};
use webrender::api::{euclid, units, DocumentId, Epoch, HitTestFlags, PipelineId, RenderApi, RenderNotifier, Transaction};

type HeadedEventLoopWindowTarget = glutin::event_loop::EventLoopWindowTarget<app::AppEvent>;
type CloseTogetherGroup = Option<NonZeroU16>;

/// Extension trait, adds [`run_window`](AppRunWindow::run_window) to [`AppExtended`](AppExtended)
pub trait AppRunWindow {
    /// Runs the application event loop and requests a new window.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use zero_ui::prelude::*;
    ///
    /// App::default().run_window(|_| {
    ///     window! {
    ///         title: "Window 1";
    ///         content: text("Window 1");
    ///     }
    /// })   
    /// ```
    ///
    /// Which is a shortcut for:
    /// ```no_run
    /// use zero_ui::prelude::*;
    ///
    /// App::default().run(|ctx| {
    ///     ctx.services.req::<Windows>().open(|_| {
    ///         window! {
    ///             title: "Window 1";
    ///             content: text("Window 1");
    ///         }
    ///     });
    /// })   
    /// ```
    fn run_window(self, new_window: impl FnOnce(&AppContext) -> Window + 'static);
}

impl<E: AppExtension> AppRunWindow for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&AppContext) -> Window + 'static) {
        self.run(|ctx| {
            ctx.services.req::<Windows>().open(new_window);
        })
    }
}

event_args! {
    /// [`WindowOpen`](WindowOpen), [`WindowClose`](WindowClose) event args.
    pub struct WindowEventArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }

    /// [`WindowIsActiveChanged`](WindowIsActiveChanged), [`WindowActivated`](WindowActivated), [`WindowDeactivated`](WindowDeactivated) event args.
    pub struct WindowIsActiveArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        /// If the window was activated in this event.
        pub activated: bool,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }

    /// [`WindowResize`](WindowResize) event args.
    pub struct WindowResizeArgs {
        pub window_id: WindowId,
        pub new_size: LayoutSize,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }

    /// [`WindowMove`](WindowMove) event args.
    pub struct WindowMoveArgs {
        pub window_id: WindowId,
        pub new_position: LayoutPoint,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }

    /// [`WindowScaleChanged`](WindowScaleChanged) event args.
    pub struct WindowScaleChangedArgs {
        pub window_id: WindowId,
        pub new_scale_factor: f32,
        pub new_size: LayoutSize,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }
}
cancelable_event_args! {
    /// [`WindowCloseRequested`](WindowCloseRequested) event args.
    pub struct WindowCloseRequestedArgs {
        pub window_id: WindowId,
        group: CloseTogetherGroup,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.window_id == self.window_id
        }
    }
}

/// New window event.
pub struct WindowOpenEvent;
impl Event for WindowOpenEvent {
    type Args = WindowEventArgs;
}

/// Window activated/deactivated event.
pub struct WindowIsActiveChangedEvent;
impl Event for WindowIsActiveChangedEvent {
    type Args = WindowIsActiveArgs;
}

/// Window activated event.
pub struct WindowActivatedEvent;
impl Event for WindowActivatedEvent {
    type Args = WindowIsActiveArgs;
}

/// Window deactivated event.
pub struct WindowDeactivatedEvent;
impl Event for WindowDeactivatedEvent {
    type Args = WindowIsActiveArgs;
}

/// Window resized event.
pub struct WindowResizeEvent;
impl Event for WindowResizeEvent {
    type Args = WindowResizeArgs;

    const IS_HIGH_PRESSURE: bool = true;
}

/// Window moved event.
pub struct WindowMoveEvent;
impl Event for WindowMoveEvent {
    type Args = WindowMoveArgs;

    const IS_HIGH_PRESSURE: bool = true;
}

/// Window scale factor changed.
pub struct WindowScaleChangedEvent;
impl Event for WindowScaleChangedEvent {
    type Args = WindowScaleChangedArgs;
}

/// Closing window event.
pub struct WindowCloseRequestedEvent;
impl Event for WindowCloseRequestedEvent {
    type Args = WindowCloseRequestedArgs;
}
impl CancelableEvent for WindowCloseRequestedEvent {
    type Args = WindowCloseRequestedArgs;
}

/// Close window event.
pub struct WindowCloseEvent;
impl Event for WindowCloseEvent {
    type Args = WindowEventArgs;
}

/// Application extension that manages windows.
///
/// # Events
///
/// Events this extension provides:
///
/// * [WindowOpenEvent]
/// * [WindowIsActiveChangedEvent]
/// * [WindowActivatedEvent]
/// * [WindowDeactivatedEvent]
/// * [WindowResizeEvent]
/// * [WindowMoveEvent]
/// * [WindowScaleChangedEvent]
/// * [WindowCloseRequestedEvent]
/// * [WindowCloseEvent]
///
/// # Services
///
/// Services this extension provides:
///
/// * [Windows]
pub struct WindowManager {
    event_loop_proxy: Option<EventLoopProxy>,
    ui_threads: Arc<ThreadPool>,
    window_open: EventEmitter<WindowEventArgs>,
    window_is_active_changed: EventEmitter<WindowIsActiveArgs>,
    window_activated: EventEmitter<WindowIsActiveArgs>,
    window_deactivated: EventEmitter<WindowIsActiveArgs>,
    window_resize: EventEmitter<WindowResizeArgs>,
    window_move: EventEmitter<WindowMoveArgs>,
    window_scale_changed: EventEmitter<WindowScaleChangedArgs>,
    window_closing: EventEmitter<WindowCloseRequestedArgs>,
    window_close: EventEmitter<WindowEventArgs>,
}

impl Default for WindowManager {
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

        WindowManager {
            event_loop_proxy: None,
            ui_threads,
            window_open: EventEmitter::new(false),
            window_is_active_changed: EventEmitter::new(false),
            window_activated: EventEmitter::new(false),
            window_deactivated: EventEmitter::new(false),
            window_resize: EventEmitter::new(true),
            window_move: EventEmitter::new(true),
            window_scale_changed: EventEmitter::new(false),
            window_closing: EventEmitter::new(false),
            window_close: EventEmitter::new(false),
        }
    }
}

impl AppExtension for WindowManager {
    fn init(&mut self, r: &mut AppInitContext) {
        self.event_loop_proxy = Some(r.event_loop.clone());
        r.services.register(Windows::new(r.updates.notifier().clone()));
        r.events.register::<WindowOpenEvent>(self.window_open.listener());
        r.events
            .register::<WindowIsActiveChangedEvent>(self.window_is_active_changed.listener());
        r.events.register::<WindowActivatedEvent>(self.window_activated.listener());
        r.events.register::<WindowDeactivatedEvent>(self.window_deactivated.listener());
        r.events.register::<WindowResizeEvent>(self.window_resize.listener());
        r.events.register::<WindowMoveEvent>(self.window_move.listener());
        r.events.register::<WindowScaleChangedEvent>(self.window_scale_changed.listener());
        r.events.register::<WindowCloseRequestedEvent>(self.window_closing.listener());
        r.events.register::<WindowCloseEvent>(self.window_close.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match event {
            WindowEvent::Focused(focused) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    window.is_active = *focused;

                    let args = WindowIsActiveArgs::now(window_id, window.is_active);
                    ctx.updates.push_notify(self.window_is_active_changed.clone(), args.clone());
                    if window.is_active {
                        ctx.updates.push_notify(self.window_activated.clone(), args);
                    } else {
                        ctx.updates.push_notify(self.window_deactivated.clone(), args);
                    }
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    let new_size = window.size();

                    ctx.updates.push_layout();
                    window.expect_layout_update();
                    window.resize_next_render();

                    // raise window_resize
                    ctx.updates
                        .push_notify(self.window_resize.clone(), WindowResizeArgs::now(window_id, new_size));

                    // set the window size variable if it is not read-only.
                    let wn_ctx = window.wn_ctx.borrow();
                    if !wn_ctx.root.size.read_only(ctx.vars) {
                        let current_size = *wn_ctx.root.size.get(ctx.vars);
                        // the var can already be set if the user modified it to resize the window.
                        if current_size != new_size {
                            ctx.updates.push_set(&wn_ctx.root.size, new_size, ctx.vars).unwrap();
                        }
                    }
                }
            }
            WindowEvent::Moved(_) => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    let new_position = window.position();

                    // set the window position variable if it is not read-only.
                    let wn_ctx = window.wn_ctx.borrow();
                    if !wn_ctx.root.position.read_only(ctx.vars) {
                        let current_pos = *wn_ctx.root.position.get(ctx.vars);
                        // the var can already be set if the user modified it to resize the window.
                        if current_pos != new_position {
                            ctx.updates.push_set(&wn_ctx.root.position, new_position, ctx.vars).unwrap();
                        }
                    }

                    ctx.updates
                        .push_notify(self.window_move.clone(), WindowMoveArgs::now(window_id, new_position))
                }
            }
            WindowEvent::CloseRequested => {
                let wins = ctx.services.req::<Windows>();
                if wins.windows.contains_key(&window_id) {
                    wins.close_requests.insert(window_id, None);
                    ctx.updates.push_update();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
                    ctx.updates.push_layout();
                    window.expect_layout_update();

                    ctx.updates.push_notify(
                        self.window_scale_changed.clone(),
                        WindowScaleChangedArgs::now(window_id, *scale_factor as f32, window.size()),
                    );
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        self.update_open_close(ctx);
        self.update_pump(update, ctx);
        self.update_closing(update, ctx);
        self.update_close(update, ctx);
    }

    fn update_display(&mut self, _: UpdateDisplayRequest, ctx: &mut AppContext) {
        // Pump layout and render in all windows.
        // The windows don't do an update unless they recorded
        // an update request for layout or render.
        for (_, window) in ctx.services.req::<Windows>().windows.iter_mut() {
            window.layout();
            window.render();
        }
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
            window.request_redraw();
        }
    }

    fn on_redraw_requested(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        if let Some(window) = ctx.services.req::<Windows>().windows.get_mut(&window_id) {
            window.redraw();
        }
    }

    fn on_shutdown_requested(&mut self, args: &ShutdownRequestedArgs, ctx: &mut AppContext) {
        if !args.cancel_requested() {
            let service = ctx.services.req::<Windows>();
            if service.shutdown_on_last_close {
                let windows: Vec<WindowId> = service.windows.keys().copied().collect();
                if !windows.is_empty() {
                    args.cancel();
                    service.close_together(windows).unwrap();
                }
            }
        }
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        let windows = mem::take(&mut ctx.services.req::<Windows>().windows);
        for (id, window) in windows {
            {
                let mut w_ctx = window.wn_ctx.borrow_mut();
                error_println!("dropping `{:?} ({})` without closing events", id, w_ctx.root.title.get_local());
                w_ctx.deinit(ctx);
            }
            window.deinit();
        }
    }
}

impl WindowManager {
    /// Respond to open/close requests.
    fn update_open_close(&mut self, ctx: &mut AppContext) {
        // respond to service requests
        let (open, close) = ctx.services.req::<Windows>().take_requests();

        for request in open {
            let w = OpenWindow::new(
                request.new,
                ctx,
                ctx.event_loop,
                self.event_loop_proxy.as_ref().unwrap().clone(),
                Arc::clone(&self.ui_threads),
            );

            let args = WindowEventArgs::now(w.id());

            let wn_ctx = w.wn_ctx.clone();
            let mut wn_ctx = wn_ctx.borrow_mut();
            ctx.services.req::<Windows>().windows.insert(args.window_id, w);
            wn_ctx.init(ctx);

            // notify the window requester
            ctx.updates.push_notify(request.notifier, args.clone());

            // notify everyone
            ctx.updates.push_notify(self.window_open.clone(), args.clone());
        }

        for (window_id, group) in close {
            ctx.updates
                .push_notify(self.window_closing.clone(), WindowCloseRequestedArgs::now(window_id, group))
        }
    }

    /// Pump the requested update methods.
    fn update_pump(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update_hp || update.update {
            // detach context part so we can let a window content access its own window.
            let mut wn_ctxs: Vec<_> = ctx
                .services
                .req::<Windows>()
                .windows
                .iter_mut()
                .map(|(_, w)| w.wn_ctx.clone())
                .collect();

            // high-pressure pump.
            if update.update_hp {
                for wn_ctx in wn_ctxs.iter_mut() {
                    wn_ctx.borrow_mut().update_hp(ctx);
                }
            }

            // low-pressure pump.
            if update.update {
                for wn_ctx in wn_ctxs.iter_mut() {
                    wn_ctx.borrow_mut().update(ctx);
                }
            }

            // do window vars update.
            if update.update {
                for (_, window) in ctx.services.req::<Windows>().windows.iter_mut() {
                    window.update_window_vars(ctx.vars, ctx.updates);
                }
            }
        }
    }

    /// Respond to window_closing events.
    fn update_closing(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if !update.update {
            return;
        }

        // close_together are canceled together
        let canceled_groups: Vec<_> = self
            .window_closing
            .updates(ctx.events)
            .iter()
            .filter_map(|c| {
                if c.cancel_requested() && c.group.is_some() {
                    Some(c.group)
                } else {
                    None
                }
            })
            .collect();

        let service = ctx.services.req::<Windows>();

        for closing in self.window_closing.updates(ctx.events) {
            if !closing.cancel_requested() && !canceled_groups.contains(&closing.group) {
                // not canceled and we can close the window.
                // notify close, the window will be deinit on
                // the next update.
                ctx.updates
                    .push_notify(self.window_close.clone(), WindowEventArgs::now(closing.window_id));

                for listener in service.close_listeners.remove(&closing.window_id).unwrap_or_default() {
                    ctx.updates.push_notify(listener, CloseWindowResult::Close);
                }
            } else {
                // canceled notify operation listeners.

                for listener in service.close_listeners.remove(&closing.window_id).unwrap_or_default() {
                    ctx.updates.push_notify(listener, CloseWindowResult::Cancel);
                }
            }
        }
    }

    /// Respond to window_close events.
    fn update_close(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if !update.update {
            return;
        }

        for close in self.window_close.updates(ctx.events) {
            if let Some(w) = ctx.services.req::<Windows>().windows.remove(&close.window_id) {
                w.wn_ctx.clone().borrow_mut().deinit(ctx);
                w.deinit();
            }
        }

        let service = ctx.services.req::<Windows>();
        if service.shutdown_on_last_close && service.windows.is_empty() {
            ctx.services.req::<AppProcess>().shutdown();
        }
    }
}

/// Windows service.
pub struct Windows {
    /// If shutdown is requested when there are no more windows open, `true` by default.
    pub shutdown_on_last_close: bool,

    windows: FnvHashMap<WindowId, OpenWindow>,

    open_requests: Vec<OpenWindowRequest>,
    close_requests: FnvHashMap<WindowId, CloseTogetherGroup>,
    next_group: u16,
    close_listeners: FnvHashMap<WindowId, Vec<EventEmitter<CloseWindowResult>>>,
    update_notifier: UpdateNotifier,
}

impl AppService for Windows {}

impl Windows {
    fn new(update_notifier: UpdateNotifier) -> Self {
        Windows {
            shutdown_on_last_close: true,
            open_requests: Vec::with_capacity(1),
            close_requests: FnvHashMap::default(),
            close_listeners: FnvHashMap::default(),
            next_group: 1,
            windows: FnvHashMap::default(),
            update_notifier,
        }
    }

    /// Requests a new window. Returns a listener that will update once when the window is opened.
    pub fn open(&mut self, new_window: impl FnOnce(&AppContext) -> Window + 'static) -> EventListener<WindowEventArgs> {
        let request = OpenWindowRequest {
            new: Box::new(new_window),
            notifier: EventEmitter::new(false),
        };
        let notice = request.notifier.listener();
        self.open_requests.push(request);

        self.update_notifier.push_update();

        notice
    }

    /// Starts closing a window, the operation can be canceled by listeners of the 
    /// [close requested event](WindowCloseRequestedEvent).
    ///
    /// Returns a listener that will update once with the result of the operation.
    pub fn close(&mut self, window_id: WindowId) -> Result<EventListener<CloseWindowResult>, WindowNotFound> {
        if self.windows.contains_key(&window_id) {
            let notifier = EventEmitter::new(false);
            let notice = notifier.listener();
            self.insert_close(window_id, None, notifier);
            self.update_notifier.push_update();
            Ok(notice)
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Requests closing multiple windows together, the operation can be canceled by listeners of the
    /// [close requested event](WindowCloseRequestedEvent). If canceled none of the windows are closed.
    ///
    /// Returns a listener that will update once with the result of the operation.
    pub fn close_together(
        &mut self,
        windows: impl IntoIterator<Item = WindowId>,
    ) -> Result<EventListener<CloseWindowResult>, WindowNotFound> {
        let windows = windows.into_iter();
        let mut buffer = Vec::with_capacity(windows.size_hint().0);
        {
            for id in windows {
                if !self.windows.contains_key(&id) {
                    return Err(WindowNotFound(id));
                }
                buffer.push(id);
            }
        }

        let set_id = NonZeroU16::new(self.next_group).unwrap();
        self.next_group += 1;

        let notifier = EventEmitter::new(false);

        for id in buffer {
            self.insert_close(id, Some(set_id), notifier.clone());
        }

        self.update_notifier.push_update();

        Ok(notifier.into_listener())
    }

    fn insert_close(&mut self, window_id: WindowId, set: CloseTogetherGroup, notifier: EventEmitter<CloseWindowResult>) {
        self.close_requests.insert(window_id, set);
        let listeners = self.close_listeners.entry(window_id).or_insert_with(Vec::new);
        listeners.push(notifier)
    }

    /// Reference an open window.
    #[inline]
    pub fn window(&self, window_id: WindowId) -> Result<&OpenWindow, WindowNotFound> {
        self.windows.get(&window_id).ok_or(WindowNotFound(window_id))
    }

    /// Iterate over all open windows.
    #[inline]
    pub fn windows(&self) -> impl Iterator<Item = &OpenWindow> {
        self.windows.values()
    }

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, FnvHashMap<WindowId, CloseTogetherGroup>) {
        (mem::take(&mut self.open_requests), mem::take(&mut self.close_requests))
    }
}

struct OpenWindowRequest {
    new: Box<dyn FnOnce(&AppContext) -> Window>,
    notifier: EventEmitter<WindowEventArgs>,
}

/// Response message of [`close`](Windows::close) and [`close_together`](Windows::close_together).
#[derive(Debug, Eq, PartialEq)]
pub enum CloseWindowResult {
    /// Operation completed, all requested windows closed.
    Close,

    /// Operation canceled, no window closed.
    Cancel,
}

/// Window not found error.
#[derive(Debug)]
pub struct WindowNotFound(pub WindowId);
impl std::fmt::Display for WindowNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "window `{:?}` is not opened in `Windows` service", self.0)
    }
}
impl std::error::Error for WindowNotFound {}

/// Window configuration.
pub struct Window {
    meta: LazyStateMap,
    id: WidgetId,
    title: BoxLocalVar<Text>,
    position: BoxVar<LayoutPoint>,
    size: BoxVar<LayoutSize>,
    background_color: BoxLocalVar<ColorF>,
    child: Box<dyn UiNode>,
}

impl Window {
    pub fn new(
        root_id: WidgetId,
        title: impl IntoVar<Text>,
        position: impl IntoVar<LayoutPoint>,
        size: impl IntoVar<LayoutSize>,
        background_color: impl IntoVar<ColorF>,
        child: impl UiNode,
    ) -> Self {
        Window {
            meta: LazyStateMap::default(),
            id: root_id,
            title: Box::new(title.into_local()),
            position: position.into_var().boxed(),
            size: size.into_var().boxed(),
            background_color: Box::new(background_color.into_local()),
            child: child.boxed(),
        }
    }
}

/// An open window.
pub struct OpenWindow {
    gl_ctx: RefCell<GlContext>,
    wn_ctx: Rc<RefCell<OwnedWindowContext>>,

    renderer: RendererState,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    clear_color: ColorF,

    first_draw: bool,
    frame_info: FrameInfo,

    // document area visible in this window.
    doc_view: units::DeviceIntRect,
    // if [doc_view] changed and no render was called yet.
    doc_view_changed: bool,

    is_active: bool,
}

impl OpenWindow {
    fn new(
        new_window: Box<dyn FnOnce(&AppContext) -> Window>,
        ctx: &mut AppContext,
        event_loop: EventLoopWindowTarget,
        event_loop_proxy: EventLoopProxy,
        ui_threads: Arc<ThreadPool>,
    ) -> Self {
        let root = new_window(ctx);
        let inner_size = *root.size.get(ctx.vars);
        let clear_color = *root.background_color.get(ctx.vars);

        let window_builder = WindowBuilder::new()
            .with_visible(false) // not visible until first render, to flickering
            .with_title(root.title.get(ctx.vars).to_owned())
            .with_inner_size(WLogicalSize::<f64>::new(inner_size.width.into(), inner_size.height.into()));

        let mut gl_ctx = GlContext::new(window_builder, event_loop.headed_target().expect("headless window not implemented"));

        let dpi_factor = gl_ctx.window().scale_factor() as f32;

        // set the user initial position.
        let pos = gl_ctx.window().outer_position().expect("only desktop windows are supported");
        let position = *root.position.get(ctx.vars);
        let user_pos = glutin::dpi::PhysicalPosition::new(
            if position.x.is_finite() {
                (position.x * dpi_factor) as i32
            } else {
                pos.x
            },
            if position.y.is_finite() {
                (position.y * dpi_factor) as i32
            } else {
                pos.y
            },
        );
        let mut set_position_var = false;
        if user_pos != pos {
            gl_ctx.window().set_outer_position(user_pos);
        } else {
            set_position_var = !root.position.read_only(ctx.vars);
        }

        let opts = webrender::RendererOptions {
            device_pixel_ratio: dpi_factor,
            clear_color: Some(clear_color),
            workers: Some(ui_threads),
            ..webrender::RendererOptions::default()
        };

        let notifier = Box::new(Notifier {
            window_id: gl_ctx.window().id(),
            event_loop: event_loop_proxy,
        });

        let start_size = inner_size * units::LayoutToDeviceScale::new(dpi_factor);
        let start_size = units::DeviceIntSize::new(start_size.width as i32, start_size.height as i32);
        let (renderer, sender) = webrender::Renderer::new(gl_ctx.gl.clone(), notifier, opts, None, start_size).unwrap();
        let api = Arc::new(sender.create_api());
        let document_id = api.add_document(start_size, 0);

        let window_id = gl_ctx.window().id();
        let (state, services) = ctx.new_window(window_id, &api);

        gl_ctx.make_not_current();

        let frame_info = FrameInfo::blank(window_id, root.id);

        let w = OpenWindow {
            gl_ctx: RefCell::new(gl_ctx),
            wn_ctx: Rc::new(RefCell::new(OwnedWindowContext {
                api,
                root,
                state,
                services,
                window_id,
                update: UpdateDisplayRequest::Layout,
            })),
            renderer: RendererState::Running(renderer),
            document_id,
            pipeline_id: PipelineId(1, 0),
            clear_color,

            first_draw: true,
            frame_info,

            doc_view: units::DeviceIntRect::from_size(start_size),
            doc_view_changed: false,
            is_active: true, // just opened it?
        };

        if set_position_var {
            // use did not set position, but variable is read-write,
            // so we update with the OS provided initial position.
            let pos = w.position();
            ctx.updates.push_set(&w.wn_ctx.borrow().root.position, pos, ctx.vars).unwrap();
        }

        w
    }

    #[inline]
    pub fn id(&self) -> WindowId {
        self.gl_ctx.borrow().window().id()
    }

    /// If the window is the foreground window.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Position of the window.
    #[inline]
    pub fn position(&self) -> LayoutPoint {
        let gl_ctx = self.gl_ctx.borrow();
        let wn = gl_ctx.window();
        let s = wn.scale_factor() as f32;
        let pos = wn.outer_position().expect("only desktop windows are supported");
        LayoutPoint::new(pos.x as f32 / s, pos.y as f32 / s)
    }

    /// Size of the window content.
    #[inline]
    pub fn size(&self) -> LayoutSize {
        let gl_ctx = self.gl_ctx.borrow();
        let wn = gl_ctx.window();
        let s = wn.scale_factor() as f32;
        let p_size = wn.inner_size();
        LayoutSize::new(p_size.width as f32 / s, p_size.height as f32 / s)
    }

    /// Scale factor used by this window, all `Layout*` values are scaled by this value by the renderer.
    #[inline]
    pub fn scale_factor(&self) -> f32 {
        self.gl_ctx.borrow().window().scale_factor() as f32
    }

    /// Pixel grid of this window, all `Layout*` values are aligned with this grid during layout.
    #[inline]
    pub fn pixel_grid(&self) -> PixelGrid {
        PixelGrid::new(self.scale_factor())
    }

    /// Hit-test the latest frame.
    #[inline]
    pub fn hit_test(&self, point: LayoutPoint) -> FrameHitInfo {
        let r = self.wn_ctx.borrow().api.hit_test(
            self.document_id,
            Some(self.pipeline_id),
            units::WorldPoint::new(point.x, point.y),
            HitTestFlags::all(),
        );

        FrameHitInfo::new(self.id(), self.frame_info.frame_id(), point, r)
    }

    /// Latest frame info.
    pub fn frame_info(&self) -> &FrameInfo {
        &self.frame_info
    }

    /// Take a screenshot of the full window area.
    pub fn screenshot(&self) -> ScreenshotData {
        self.screenshot_rect(LayoutRect::from_size(self.size()))
    }

    /// Take a screenshot of a window area.
    pub fn screenshot_rect(&self, rect: LayoutRect) -> ScreenshotData {
        let mut gl_ctx = self.gl_ctx.borrow_mut();

        // calculate intersection with window in physical pixels.
        let (x, y, width, height, dpi) = {
            let window = gl_ctx.window();
            let dpi = window.scale_factor() as f32;
            let max_size = window.inner_size();
            let max_rect = LayoutRect::new(LayoutPoint::zero(), LayoutSize::new(max_size.width as f32, max_size.height as f32));
            let rect = rect * euclid::Scale::new(dpi);
            let rect = rect.intersection(&max_rect).unwrap_or_default();
            (
                rect.origin.x as u32,
                // read_pixels (0, 0) is the lower left corner.
                (max_rect.size.height - rect.origin.y - rect.size.height).max(0.0) as u32,
                rect.size.width as u32,
                rect.size.height as u32,
                dpi,
            )
        };

        if width == 0 || height == 0 {
            return ScreenshotData {
                pixels: vec![],
                width,
                height,
                dpi,
            };
        }

        gl_ctx.make_current();
        gl_ctx.swap_buffers();

        let pixels = gl_ctx
            .gl
            .read_pixels(x as _, y as _, width as _, height as _, gl::RGB, gl::UNSIGNED_BYTE);

        gl_ctx.swap_buffers();

        let error = gl_ctx.gl.get_error();
        if error != gl::NO_ERROR {
            panic!("read_pixels error: {:#x}", error)
        }

        gl_ctx.make_not_current();

        let mut pixels_flipped = Vec::with_capacity(pixels.len());
        for v in (0..height as _).rev() {
            let s = 3 * v as usize * width as usize;
            let o = 3 * width as usize;
            pixels_flipped.extend_from_slice(&pixels[s..(s + o)]);
        }

        ScreenshotData {
            pixels: pixels_flipped,
            width,
            height,
            dpi,
        }
    }

    /// Manually flags layout to actually update on the next call.
    ///
    /// This is required for updates generated outside of this window but that affect this window.
    fn expect_layout_update(&mut self) {
        self.wn_ctx.borrow_mut().update |= UpdateDisplayRequest::Layout;
    }

    fn update_window_vars(&mut self, vars: &Vars, updates: &mut Updates) {
        profile_scope!("window::update_window_vars");

        let gl_ctx = self.gl_ctx.borrow();
        let window = gl_ctx.window();
        let mut wn_ctx = self.wn_ctx.borrow_mut();

        // title
        if let Some(title) = wn_ctx.root.title.update_local(vars) {
            window.set_title(title);
        }

        // position
        if let Some(&new_pos) = wn_ctx.root.position.update(vars) {
            let pos = window.outer_position().expect("only desktop windows are supported");
            let s = window.scale_factor() as f32;

            let new_pos = glutin::dpi::PhysicalPosition::new(
                if new_pos.x.is_finite() { (new_pos.x * s) as i32 } else { pos.x },
                if new_pos.y.is_finite() { (new_pos.y * s) as i32 } else { pos.y },
            );

            if new_pos != pos {
                // the position variable was changed to set the position size.
                window.set_outer_position(new_pos);
            }
        }

        // size
        if let Some(&new_size) = wn_ctx.root.size.update(vars) {
            let s = window.scale_factor() as f32;
            let new_size = glutin::dpi::PhysicalSize::new((new_size.width * s) as u32, (new_size.height * s) as u32);
            if new_size != window.inner_size() {
                // the size variable was changed to set the window size.
                window.set_inner_size(new_size);
            }
        }

        // background_color
        if wn_ctx.root.background_color.update_local(vars).is_some() {
            wn_ctx.update |= UpdateDisplayRequest::Render;
            updates.push_render();
        }
    }

    /// Re-flow layout if a layout pass was required. If yes will
    /// flag a render required.
    fn layout(&mut self) {
        let mut ctx = self.wn_ctx.borrow_mut();

        if ctx.update == UpdateDisplayRequest::Layout {
            profile_scope!("window::layout");

            ctx.update = UpdateDisplayRequest::Render;

            let available_size = self.size();
            let pixels = self.pixel_grid();

            ctx.root.child.measure(available_size, pixels);
            ctx.root.child.arrange(available_size, pixels);
        }
    }

    fn resize_next_render(&mut self) {
        let inner_size = self.gl_ctx.borrow().window().inner_size();
        let device_size = units::DeviceIntSize::new(inner_size.width as i32, inner_size.height as i32);
        self.doc_view = units::DeviceIntRect::from_size(device_size);
        self.doc_view_changed = true;
    }

    /// Render a frame if one was required.
    fn render(&mut self) {
        let mut ctx = self.wn_ctx.borrow_mut();

        if ctx.update == UpdateDisplayRequest::Render {
            profile_scope!("window::render");

            ctx.update = UpdateDisplayRequest::None;

            let frame_id = Epoch({
                let mut next = self.frame_info.frame_id().0.wrapping_add(1);
                if next == FrameId::invalid().0 {
                    next = next.wrapping_add(1);
                }
                next
            });

            let size = self.size();
            let mut frame = FrameBuilder::new(frame_id, ctx.window_id, self.pipeline_id, ctx.root.id, size, self.scale_factor());
            let clear_color = *ctx.root.background_color.get_local();

            if clear_color != self.clear_color {
                frame.push_color(LayoutRect::from_size(size), clear_color);
            }
            ctx.root.child.render(&mut frame);

            let (display_list_data, frame_info) = frame.finalize();

            self.frame_info = frame_info;

            let mut txn = Transaction::new();
            txn.set_display_list(frame_id, Some(clear_color), size, display_list_data, true);
            txn.set_root_pipeline(self.pipeline_id);

            if self.doc_view_changed {
                self.doc_view_changed = false;
                txn.set_document_view(self.doc_view, self.scale_factor());
            }

            txn.generate_frame();
            ctx.api.send_transaction(self.document_id, txn);
        }
    }

    /// Notifies the OS to redraw the window, will receive WindowEvent::RedrawRequested
    /// from the OS after calling this.
    fn request_redraw(&mut self) {
        if self.first_draw {
            self.gl_ctx.borrow().window().set_visible(true); // OS generates a RequestRedraw here
            self.first_draw = false;
        } else {
            self.gl_ctx.borrow().window().request_redraw();
        }
    }

    /// Redraws the last ready frame and swaps buffers.
    ///
    /// **`swap_buffers` Warning**: if you enabled vsync, this function will block until the
    /// next time the screen is refreshed. However drivers can choose to
    /// override your vsync settings, which means that you can't know in
    /// advance whether `swap_buffers` will block or not.
    fn redraw(&mut self) {
        profile_scope!("window::redraw");

        let mut context = self.gl_ctx.borrow_mut();
        context.make_current();

        let renderer = self.renderer.borrow_mut();
        renderer.update();

        let size = context.window().inner_size();
        let device_size = units::DeviceIntSize::new(size.width as i32, size.height as i32);

        renderer.render(device_size).unwrap();
        let _ = renderer.flush_pipeline_info();

        context.swap_buffers();
        context.make_not_current();
    }

    fn deinited(&self) -> bool {
        self.renderer.deinited()
    }

    fn deinit(mut self) {
        self.gl_ctx.borrow_mut().make_current();
        self.renderer.deinit();
        self.gl_ctx.borrow_mut().make_not_current();
    }
}

impl Drop for OpenWindow {
    fn drop(&mut self) {
        if !self.deinited() {
            error_println!("dropping window without calling `OpenWindow::deinit`");
        }
    }
}

/// Window screenshot image data.
pub struct ScreenshotData {
    /// RGB8
    pub pixels: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Dpi scale when the screenshot was taken.
    pub dpi: f32,
}
impl ScreenshotData {
    /// Encode and save the screenshot image.
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> image::ImageResult<()> {
        image::save_buffer(path, &self.pixels, self.width, self.height, image::ColorType::Rgb8)
    }
}

struct OwnedWindowContext {
    window_id: WindowId,
    state: WindowState,
    services: WindowServices,
    root: Window,
    api: Arc<RenderApi>,
    update: UpdateDisplayRequest,
}

impl OwnedWindowContext {
    fn root_context(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut Box<dyn UiNode>, &mut WidgetContext)) -> UpdateDisplayRequest {
        let root = &mut self.root;

        ctx.window_context(self.window_id, &mut self.state, &mut self.services, &self.api, |ctx| {
            let child = &mut root.child;
            ctx.widget_context(root.id, &mut root.meta, |ctx| {
                f(child, ctx);
            });
        })
    }

    /// Call [`UiNode::init`](UiNode::init) in all nodes.
    pub fn init(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::init");

        self.root.title.init_local(ctx.vars);
        self.root.background_color.init_local(ctx.vars);

        let update = self.root_context(ctx, |root, ctx| {
            ctx.updates.push_layout();

            root.init(ctx);
        });
        self.update |= update;
    }

    /// Call [`UiNode::update_hp`](UiNode::update_hp) in all nodes.
    pub fn update_hp(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::update_hp");

        let update = self.root_context(ctx, |root, ctx| root.update_hp(ctx));
        self.update |= update;
    }

    /// Call [`UiNode::update`](UiNode::update) in all nodes.
    pub fn update(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::update");

        // do UiNode updates
        let update = self.root_context(ctx, |root, ctx| root.update(ctx));
        self.update |= update;
    }

    /// Call [`UiNode::deinit`](UiNode::deinit) in all nodes.
    pub fn deinit(&mut self, ctx: &mut AppContext) {
        profile_scope!("window::deinit");
        self.root_context(ctx, |root, ctx| root.deinit(ctx));
    }
}

#[derive(Clone)]
struct Notifier {
    window_id: WindowId,
    event_loop: EventLoopProxy,
}
impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Clone::clone(self))
    }

    fn wake_up(&self) {}

    fn new_frame_ready(&self, _: DocumentId, _scrolled: bool, _composite_needed: bool, _: Option<u64>) {
        self.event_loop.send_event(app::AppEvent::NewFrameReady(self.window_id));
    }
}

#[allow(clippy::large_enum_variant)]
enum RendererState {
    Running(webrender::Renderer),
    Deinited,
}

impl RendererState {
    fn deinit(&mut self) {
        match mem::replace(self, RendererState::Deinited) {
            RendererState::Running(r) => r.deinit(),
            RendererState::Deinited => panic!("renderer already deinited"),
        }
    }

    fn borrow_mut(&mut self) -> &mut webrender::Renderer {
        match self {
            RendererState::Running(wr) => wr,
            RendererState::Deinited => panic!("cannot borrow deinited renderer"),
        }
    }

    fn deinited(&self) -> bool {
        match self {
            RendererState::Deinited => true,
            _ => false,
        }
    }
}

enum GlContextState {
    Current(WindowedContext<PossiblyCurrent>),
    NotCurrent(WindowedContext<NotCurrent>),
    Changing,
}

struct GlContext {
    ctx: GlContextState,
    gl: Rc<dyn gl::Gl>,
}

impl GlContext {
    fn new(window_builder: WindowBuilder, event_loop: &HeadedEventLoopWindowTarget) -> Self {
        let context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(window_builder, &event_loop)
            .unwrap();

        let context = unsafe { context.make_current().expect("couldn't make `GlContext` current") };

        let gl = match context.get_api() {
            Api::OpenGl => unsafe { gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::OpenGlEs => unsafe { gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _) },
            Api::WebGl => panic!("WebGl is not supported"),
        };

        GlContext {
            ctx: GlContextState::Current(context),
            gl,
        }
    }

    fn window(&self) -> &GlutinWindow {
        match &self.ctx {
            GlContextState::Current(c) => c.window(),
            GlContextState::NotCurrent(c) => c.window(),
            GlContextState::Changing => unreachable!(),
        }
    }

    fn make_current(&mut self) {
        self.ctx = match std::mem::replace(&mut self.ctx, GlContextState::Changing) {
            GlContextState::Current(_) => {
                panic!("`GlContext` already is current");
            }
            GlContextState::NotCurrent(c) => {
                let c = unsafe { c.make_current().expect("couldn't make `GlContext` current") };
                GlContextState::Current(c)
            }
            GlContextState::Changing => unreachable!(),
        }
    }

    fn make_not_current(&mut self) {
        self.ctx = match mem::replace(&mut self.ctx, GlContextState::Changing) {
            GlContextState::Current(c) => {
                let c = unsafe { c.make_not_current().expect("couldn't make `GlContext` not current") };
                GlContextState::NotCurrent(c)
            }
            GlContextState::NotCurrent(_) => {
                panic!("`GlContext` already is not current");
            }
            GlContextState::Changing => unreachable!(),
        }
    }

    fn swap_buffers(&self) {
        match &self.ctx {
            GlContextState::Current(c) => c.swap_buffers().expect("failed to swap buffers"),
            GlContextState::NotCurrent(_) => {
                panic!("can only swap buffers of current contexts");
            }
            GlContextState::Changing => unreachable!(),
        };
    }
}

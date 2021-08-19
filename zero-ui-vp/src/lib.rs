//! Zero-Ui View Process.
//!
//! Zero-Ui isolates all OpenGL and windowing related code to a different process to be able to recover from driver errors.
//! This crate contains the `glutin` and `webrender` code that interacts with the actual system. Communication
//! with the app process is done using `ipmpsc`.

#![allow(unused_parens)]

use config::system_text_aa;
use glutin::{
    event::*,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
    monitor::MonitorHandle,
    window::WindowId,
};
use ipmpsc::{Receiver, Sender, SharedRingBuffer};
use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{ErrorKind, Read},
    panic,
    path::{Path, PathBuf},
    process::{self, Child, Command, Stdio},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};
use window::ViewWindow;

mod config;
mod types;
mod window;

const CHANNEL_VAR: &str = "ZERO_UI_WR_CHANNELS";
const MODE_VAR: &str = "ZERO_UI_WR_MODE";

const MAX_RESPONSE_SIZE: u32 = 1024u32.pow(2) * 20;
const MAX_REQUEST_SIZE: u32 = 1024u32.pow(2) * 20;
const MAX_EVENT_SIZE: u32 = 1024u32.pow(2) * 20;

/// Version 0.1.
///
/// The *App Process* and *View Process* must be build using the same exact version of `zero-ui-vp` and this is
/// validated during run-time, causing a panic if the versions don't match. Usually the same executable is used
/// for both processes so this is not a problem.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Call this function before anything else in the app `main` function.
///
/// If the process is started with the right environment configuration this function
/// high-jacks the process and turns it into a *View Process*, never returning.
///
/// This function does nothing if the *View Process* environment is not set, you can safely call it more then once.
///
/// # Examples
///
/// ```no_run
/// # use zero_ui_vp::init_view_process;
/// fn main() {
///     init_view_process();
///
///     println!("Only Prints if is not View Process");
///
///     // .. init app here.
/// }
/// ```
pub fn init_view_process() {
    if let Ok(channel_dir) = env::var(CHANNEL_VAR) {
        let mode = env::var(MODE_VAR).unwrap_or_else(|_| "headed".to_owned());
        let headless = match mode.as_str() {
            "headed" => false,
            "headless" => true,
            _ => panic!("unknown mode"),
        };
        run(PathBuf::from(channel_dir), headless);
    }
}

struct SameProcessConfig {
    waiter: Arc<Condvar>,
    channel_dir: PathBuf,
    headless: bool,
}
static SAME_PROCESS_CONFIG: Mutex<Option<SameProcessConfig>> = parking_lot::const_mutex(None);

/// Run both View and App in the same process.
///
/// This function must be called in the main thread, it initializes the View and calls `run_app`
/// in a new thread to initialize the App.

/// The primary use of this function is debugging the view process code
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) -> ! {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("can only init view in the main thread")
    }

    let mut config = SAME_PROCESS_CONFIG.lock();

    thread::spawn(run_app);

    let waiter = Arc::new(Condvar::new());
    *config = Some(SameProcessConfig {
        waiter: waiter.clone(),
        channel_dir: PathBuf::new(),
        headless: false,
    });

    let result = waiter.wait_for(&mut config, Duration::from_secs(10));
    if result.timed_out() {
        panic!("Controller::start was not called in 10 seconds");
    }

    let config = config.take().unwrap();
    run(config.channel_dir, config.headless)
}

pub use types::{
    AxisId, ButtonId, CursorIcon, DevId, ElementState, Error, Ev, FramePixels, FrameRequest, Icon, ModifiersState, MonId, MonitorInfo,
    MouseButton, MouseScrollDelta, Result, ScanCode, TextAntiAliasing, VideoMode, VirtualKeyCode, WinId, WindowConfig, WindowTheme,
};

use webrender::api::{
    units::{LayoutPoint, LayoutRect, LayoutSize},
    DynamicProperties, FontInstanceKey, FontKey, HitTestResult, IdNamespace, ImageKey, PipelineId, ResourceUpdate,
};

/// Start the app event loop in the View Process.
fn run(channel_dir: PathBuf, headless: bool) -> ! {
    if !is_main_thread::is_main_thread().unwrap_or(true) {
        panic!("can only init view-process in the main thread")
    }

    let request_receiver = Receiver::new(
        SharedRingBuffer::open(&channel_dir.join("request").display().to_string()).expect("request channel connection failed"),
    );
    let response_sender = Sender::new(
        SharedRingBuffer::open(&channel_dir.join("response").display().to_string()).expect("response channel connection failed"),
    );
    let event_sender =
        Sender::new(SharedRingBuffer::open(&channel_dir.join("event").display().to_string()).expect("event channel connection failed"));

    let event_loop = EventLoop::<AppEvent>::with_user_event();

    let request_sender = event_loop.create_proxy();
    thread::spawn(move || {
        loop {
            match request_receiver.recv() {
                Ok(req) => {
                    if request_sender.send_event(AppEvent::Request(req)).is_err() {
                        // event-loop shutdown
                        return;
                    }
                }
                Err(e) => {
                    eprintln!("request channel error:\n{:#?}", e);
                    process::exit(i32::from_ne_bytes(*b"requ"));
                }
            }
        }
    });

    let mut app = ViewApp::new(response_sender, event_sender, headless);

    let el = event_loop.create_proxy();
    #[cfg(windows)]
    let config_listener = config::config_listener(&Context {
        event_loop: &el,
        window_target: &event_loop,
    });

    event_loop.run(move |event, window_target, control| {
        *control = ControlFlow::Wait; // will wait after current event sequence.

        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent { window_id, event } => {
                #[cfg(windows)]
                if window_id == config_listener.id() {
                    return; // ignore events for this window.
                }

                app.on_window_event(window_id, event)
            }
            Event::DeviceEvent { device_id, event } => app.on_device_event(device_id, event),
            Event::UserEvent(ev) => match ev {
                AppEvent::Request(req) => app.on_request(
                    &Context {
                        event_loop: &el,
                        window_target,
                    },
                    req,
                ),
                AppEvent::FrameReady(window_id) => app.on_frame_ready(window_id),
                AppEvent::SystemFontsChanged => app.notify(Ev::FontsChanged),
                AppEvent::SystemTextAaChanged(aa) => app.notify(Ev::TextAaChanged(aa)),
                AppEvent::KeyboardInput(w_id, d_id, k) => app.notify(Ev::KeyboardInput(w_id, d_id, k)),
            },
            Event::Suspended => {}
            Event::Resumed => {}
            Event::MainEventsCleared => app.on_events_cleared(),
            Event::RedrawRequested(w) => app.on_redraw(w),
            Event::RedrawEventsCleared => {}
            Event::LoopDestroyed => panic!("unexpected event loop shutdown, we use `process::exit` to shutdown"),
        }
    })
}

pub(crate) struct Context<'a> {
    pub event_loop: &'a EventLoopProxy<AppEvent>,
    pub window_target: &'a EventLoopWindowTarget<AppEvent>,
}

/// Custom event loop event.
pub(crate) enum AppEvent {
    Request(Request),
    FrameReady(WindowId),
    SystemFontsChanged,
    SystemTextAaChanged(TextAntiAliasing),
    KeyboardInput(WinId, DevId, KeyboardInput),
}

/// Declares the `Request` and `Response` enums, and two methods in `Controller` and `ViewApp`, in the
/// controller it packs and sends the request and receives and unpacks the response. In the view it implements
/// the method.
macro_rules! declare_ipc {
    (
        $(
            $(#[$doc:meta])*
            $vis:vis fn $method:ident(&mut $self:ident, $ctx:ident: &Context $(, $input:ident : $RequestType:ty)* $(,)?) -> Result<$ResponseType:ty> {
                $($impl:tt)*
            }
        )*
    ) => {
        #[derive(Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        enum Request {
            $(
                $method { $($input: $RequestType),* },
            )*
        }

        #[derive(Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        enum Response {
            $(
                $method(Result<$ResponseType>),
            )*
        }

        #[allow(unused_parens)]
        impl Controller {
            $(
                $(#[$doc])*
                #[allow(clippy::too_many_arguments)]
                $vis fn $method(&mut self $(, $input: $RequestType)*) -> Result<$ResponseType> {
                    match self.talk(Request::$method { $($input),* })? {
                        Response::$method(r) => r,
                        _ => panic!("view-process did not respond correctly")
                    }
                }
            )*
        }

        #[allow(unused_parens)]
        impl ViewApp {
            pub fn on_request(&mut self, ctx: &Context, request: Request) {
                match request {
                    $(
                        Request::$method { $($input),* } => {
                            let r = self.$method(ctx, $($input),*);
                            self.respond(Response::$method(r));
                        }
                    )*
                }
            }

            $(
                #[allow(clippy::too_many_arguments)]
                fn $method(&mut $self, $ctx: &Context $(, $input: $RequestType)*) -> Result<$ResponseType> {
                    $($impl)*
                }
            )*
        }
    };
}

/// The listener returns the closure on join for reuse in respawn.
type EventListenerJoin = JoinHandle<Box<dyn FnMut(Ev) + Send>>;

/// View Process controller, used in the App Process.
pub struct Controller {
    process: Option<Child>,
    view_process_exe: PathBuf,
    channel_dir: PathBuf,
    request_chan: Sender,
    response_chan: Receiver,
    event_listener: Option<EventListenerJoin>,
    headless: bool,
}
impl Controller {
    /// Start with a custom view process.
    ///
    /// The `view_process_exe` must be an executable that calls [`init_view_process`], if not set
    /// the [`current_exe`] is used. Note that the [`VERSION`] of this crate must match in both executables.
    ///
    /// The `on_event` closure is called in another thread every time the app receives an event.
    ///
    /// [`current_exe`]: std::env::current_exe
    /// [`init_view_process`]: crate::init_view_process
    /// [`VERSION`]: crate::VERSION
    pub fn start<F>(view_process_exe: Option<PathBuf>, device_events: bool, headless: bool, mut on_event: F) -> Self
    where
        F: FnMut(Ev) + Send + 'static,
    {
        let view_process_exe = view_process_exe.unwrap_or_else(|| std::env::current_exe().unwrap());

        let (channel_dir, process, request_chan, response_chan, event_chan) = Self::spawn_view_process(&view_process_exe, headless);

        let ev = thread::spawn(move || {
            while let Ok(ev) = event_chan.recv() {
                on_event(ev);
            }
            // return to use in respawn.
            let t: Box<dyn FnMut(Ev) + Send> = Box::new(on_event);
            t
        });

        let mut c = Controller {
            process,
            view_process_exe,
            channel_dir,
            request_chan,
            response_chan,
            event_listener: Some(ev),
            headless,
        };
        if crate::VERSION != c.version().unwrap() {
            panic!("app-process and view-process must be build using the same exact version of zero-ui-vp");
        }

        assert!(c.startup(device_events, headless).unwrap());

        c
    }

    /// If is running in headless mode.
    #[inline]
    pub fn headless(&self) -> bool {
        self.headless
    }

    /// If is running both view and app in the same process.
    #[inline]
    pub fn same_process(&self) -> bool {
        self.process.is_none()
    }

    fn try_talk(&mut self, req: &Request) -> ipmpsc::Result<Response> {
        self.request_chan.send_when_empty(req)?;
        let r = self.response_chan.recv_timeout(Duration::from_secs(5))?;
        r.ok_or_else(|| {
            ipmpsc::Error::Io(std::io::Error::new(
                ErrorKind::TimedOut,
                "view-process did not respond in 5 seconds",
            ))
        })
    }

    fn talk(&mut self, req: Request) -> Result<Response> {
        match self.try_talk(&req) {
            Ok(r) => return Ok(r),
            Err(e) => match e {
                ipmpsc::Error::AlreadyReceived => unreachable!("we don't use ZeroCopyContext, yet? TODO"),
                ipmpsc::Error::ZeroSizedMessage => panic!("implementation error, ZeroSizedMessage"),
                ipmpsc::Error::MessageTooLarge => panic!("implementation error, MessageTooLarge"),
                ipmpsc::Error::TooManySenders => panic!("expected one sender per view-process"),
                ipmpsc::Error::IncompatibleRingBuffer => {
                    unreachable!("app-process and view-process must be build with the same version of zero-ui-vp")
                }
                ipmpsc::Error::Runtime(e) => {
                    log::error!(target: "vp_recover", "will retry after ipmpsc runtime error, {}", e);
                    self.try_recover();
                }
                ipmpsc::Error::Io(e) => {
                    log::error!(target: "vp_recover", "will retry after ipmpsc IO error, {}", e);
                    self.try_recover();
                }
                ipmpsc::Error::Bincode(e) => {
                    log::error!(target: "vp_recover", "will retry after ipmpsc bincode error, {}", e);
                    self.try_recover();
                }
            },
        }
        Err(Error::Respawn)
    }

    fn spawn_view_process(view_process_exe: &Path, headless: bool) -> (PathBuf, Option<Child>, Sender, Receiver, Receiver) {
        let channel_dir = loop {
            let temp_dir = env::temp_dir().join(uuid::Uuid::new_v4().to_simple().to_string());
            match std::fs::create_dir(&temp_dir) {
                Ok(_) => break temp_dir,
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("failed to create channel directory: {}", e),
            }
        };

        let rsp = Receiver::new(
            SharedRingBuffer::create(channel_dir.join("response").display().to_string().as_str(), MAX_RESPONSE_SIZE)
                .expect("response channel creation failed"),
        );
        let ev = Receiver::new(
            SharedRingBuffer::create(channel_dir.join("event").display().to_string().as_str(), MAX_EVENT_SIZE)
                .expect("event channel creation failed"),
        );
        let req = Sender::new(
            SharedRingBuffer::create(channel_dir.join("request").display().to_string().as_str(), MAX_REQUEST_SIZE)
                .expect("request channel creation failed"),
        );

        // create process and spawn it, unless is running in same process mode.
        if let Some(config) = &mut *SAME_PROCESS_CONFIG.lock() {
            config.channel_dir = channel_dir.clone();
            config.headless = headless;
            config.waiter.notify_one();
            (channel_dir, None, req, rsp, ev)
        } else {
            let process = Command::new(&view_process_exe)
                .env(CHANNEL_VAR, &channel_dir)
                .env(MODE_VAR, if headless { "headless" } else { "headed" })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("view-process failed to spawn");
            (channel_dir, Some(process), req, rsp, ev)
        }
    }

    fn try_recover(&mut self) {
        let process = self.process.as_mut().expect("cannot recover in same_process mode");

        log::info!(target: "vp_recover", "trying to recover view-process");

        // try exit
        let exit_code = match process.try_wait() {
            Ok(Some(code)) => Some(code),
            Ok(None) => {
                log::warn!(target: "vp_recover", "view-process still running");
                match process.kill() {
                    Ok(_) => {
                        log::info!(target: "vp_recover", "killed view-process");
                        match process.try_wait() {
                            Ok(Some(s)) => Some(s),
                            Ok(None) => unreachable!(),
                            Err(e) => {
                                log::error!(target: "vp_recover", "try_wait after kill error, {:?}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        log::error!(target: "vp_recover", "view process kill error, {:?}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::error!(target: "vp_recover", "try_wait error, {:?}", e);
                None
            }
        };

        // try print stdout/err and exit code.
        if let Some(code) = exit_code {
            log::info!(target: "vp_recover", "view-process reaped");
            log::error!(target: "vp_recover", "view-process exit_code: {:x}", code.code().unwrap_or(0));

            if let Some(mut err) = process.stderr.take() {
                let mut s = String::new();
                match err.read_to_string(&mut s) {
                    Ok(l) => log::error!(target: "vp_recover", "view-process stderr ({} bytes):\n{}\n=====", l, s),
                    Err(e) => log::error!(target: "vp_recover", "failed to read view-process stderr: {}", e),
                }
            }
            if let Some(mut out) = process.stdout.take() {
                let mut s = String::new();
                match out.read_to_string(&mut s) {
                    Ok(l) => log::info!(target: "vp_recover", "view-process stdout ({} bytes):\n{}\n=====", l, s),
                    Err(e) => log::error!(target: "vp_recover", "failed to read view-process stdout: {}", e),
                }
            }
        } else {
            log::error!(target: "vp_recover", "failed to reap view-process, will abandon it running");
        }

        let mut on_event = match self.event_listener.take().unwrap().join() {
            Ok(fn_) => fn_,
            Err(p) => panic::resume_unwind(p),
        };

        let (channel_dir, new_process, request, response, event) = Self::spawn_view_process(&self.view_process_exe, self.headless);

        on_event(Ev::Respawned);

        self.channel_dir = channel_dir;
        *process = new_process.unwrap();
        self.request_chan = request;
        self.response_chan = response;

        let ev = thread::spawn(move || {
            while let Ok(ev) = event.recv() {
                on_event(ev);
            }
            on_event
        });

        self.event_listener = Some(ev);

        todo!("limit retries")
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
        let _ = fs::remove_dir_all(&self.channel_dir);
    }
}

/// The View Process.
pub(crate) struct ViewApp {
    response_chan: Sender,
    event_chan: Sender,
    started: bool,
    device_events: bool,
    headless: bool,

    window_id_count: WinId,
    windows: Vec<ViewWindow>,

    monitor_id_count: MonId,
    monitors: Vec<(MonId, MonitorHandle)>,

    device_id_count: DevId,
    devices: Vec<(DevId, DeviceId)>,

    // if one or more events where send after the last on_events_cleared.
    pending_clear: bool,
}
impl ViewApp {
    pub fn new(response_chan: Sender, event_chan: Sender, headless: bool) -> ViewApp {
        ViewApp {
            response_chan,
            event_chan,
            started: false,
            device_events: false,
            headless,
            window_id_count: u32::from_ne_bytes(*b"zwvp"),
            windows: vec![],
            monitor_id_count: u32::from_ne_bytes(*b"zsvp"),
            monitors: vec![],
            device_id_count: u32::from_ne_bytes(*b"zdvp"),
            devices: vec![],
            pending_clear: false,
        }
    }

    fn respond(&self, response: Response) {
        self.response_chan.send(&response).expect("TODO")
    }
    fn notify(&mut self, event: Ev) {
        self.pending_clear = true;
        self.event_chan.send(&event).expect("TODO")
    }

    fn monitor_id(&mut self, handle: &MonitorHandle) -> MonId {
        if let Some((id, _)) = self.monitors.iter().find(|(_, h)| h == handle) {
            *id
        } else {
            let mut id = self.monitor_id_count.wrapping_add(1);
            if id == 0 {
                id = 1;
            }
            self.monitor_id_count = id;
            self.monitors.push((id, handle.clone()));
            id
        }
    }

    fn device_id(&mut self, device_id: DeviceId) -> DevId {
        if let Some((id, _)) = self.devices.iter().find(|(_, id)| *id == device_id) {
            *id
        } else {
            let mut id = self.device_id_count.wrapping_add(1);
            if id == 0 {
                id = 1;
            }
            self.device_id_count = id;
            self.devices.push((id, device_id));
            id
        }
    }

    fn with_window<R>(&mut self, id: WinId, f: impl FnOnce(&mut ViewWindow) -> R) -> Result<R> {
        assert!(self.started);

        if let Some(w) = self.windows.iter_mut().find(|w| w.id() == id) {
            Ok(f(w))
        } else {
            Err(Error::WindowNotFound(id))
        }
    }
}

declare_ipc! {
    fn version(&mut self, _ctx: &Context) -> Result<String> {
        Ok(crate::VERSION.to_string())
    }

    fn startup(&mut self, _ctx: &Context, device_events: bool, headless: bool) -> Result<bool> {
        assert!(!self.started, "view-process already started");

        self.device_events = device_events;

        assert!(self.headless == headless, "view-process environemt and startup do not agree");

        self.started = true;
        Ok(true)
    }

    /// Returns the primary monitor if there is any or the first available monitor or none if no monitor was found.
    pub fn primary_monitor(&mut self, ctx: &Context) -> Result<Option<(MonId, MonitorInfo)>> {
        Ok(
            ctx.window_target
            .primary_monitor()
            .or_else(|| ctx.window_target.available_monitors().next())
            .map(|m| {
                let id = self.monitor_id(&m);
                let info = m.into();
                (id, info)
            })
        )
    }

    /// Returns information about the specific monitor, if it exists.
    pub fn monitor_info(&mut self, _ctx: &Context, id: MonId) -> Result<Option<MonitorInfo>> {
        Ok(self.monitors.iter().find(|(i, _)| *i == id).map(|(_, h)| MonitorInfo::from(h)))
    }

    /// Returns all available monitors.
    pub fn available_monitors(&mut self, ctx: &Context) -> Result<Vec<(MonId, MonitorInfo)>> {
        Ok(
            ctx.window_target
            .available_monitors()
            .map(|m| {
                let id = self.monitor_id(&m);
                let info = m.into();
                (id, info)
            })
            .collect()
        )
    }

    /// Open a window.
    ///
    /// Returns the window id.
    pub fn open_window(
        &mut self,
        ctx: &Context,
        config: WindowConfig,
    ) -> Result<WinId> {
        assert!(self.started);

        let mut id = self.window_id_count.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.window_id_count = id;

        let window = ViewWindow::new(ctx, id, config);
        self.windows.push(window);

        Ok(id)
    }

    /// Close the window.
    pub fn close_window(&mut self, _ctx: &Context, id: WinId) -> Result<()> {
        assert!(self.started);

        if let Some(i) = self.windows.iter().position(|w|w.id() == id) {
            self.windows.remove(i);
            Ok(())
        } else {
            Err(Error::WindowNotFound(id))
        }
    }

    /// Reads the default text anti-aliasing.
    pub fn system_text_aa(&mut self, _ctx: &Context) -> Result<TextAntiAliasing> {
        Ok(system_text_aa())
    }

    /// Set window title.
    pub fn set_title(&mut self, _ctx: &Context, id: WinId, title: String) -> Result<()> {
        self.with_window(id, |w| w.set_title(title))
    }

    /// Set window visible.
    pub fn set_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> Result<()> {
        self.with_window(id, |w| w.set_visible(visible))
    }

    /// Set if the window is "top-most".
    pub fn set_always_on_top(&mut self, _ctx: &Context, id: WinId, always_on_top: bool) -> Result<()> {
        self.with_window(id, |w| w.set_always_on_top(always_on_top))
    }

    /// Set if the user can drag-move the window.
    pub fn set_movable(&mut self, _ctx: &Context, id: WinId, movable: bool) -> Result<()> {
        self.with_window(id, |w| w.set_movable(movable))
    }

    /// Set if the user can resize the window.
    pub fn set_resizable(&mut self, _ctx: &Context, id: WinId, resizable: bool) -> Result<()> {
        self.with_window(id, |w| w.set_resizable(resizable))
    }

    /// Set the window taskbar icon visibility.
    pub fn set_taskbar_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> Result<()> {
        self.with_window(id, |w| w.set_taskbar_visible(visible))
    }

    /// Set the window parent and if `self` blocks the parent events while open (`modal`).
    pub fn set_parent(&mut self, _ctx: &Context, id: WinId, parent: Option<WinId>, modal: bool) -> Result<()> {
        if let Some(parent_id) = parent {
            if let Some(parent_id) = self.windows.iter().find(|w|w.id() == parent_id).map(|w|w.actual_id()) {
                self.with_window(id, |w|w.set_parent(Some(parent_id), modal))
            } else {
                self.with_window(id, |w| w.set_parent(None, modal))?;
                Err(Error::WindowNotFound(parent_id))
            }
        } else {
            self.with_window(id, |w| w.set_parent(None, modal))
        }
    }

    /// Set if the window is see-through.
    pub fn set_transparent(&mut self, _ctx: &Context, id: WinId, transparent: bool) -> Result<()> {
        self.with_window(id, |w| w.set_transparent(transparent))
    }

   /// Set the window system border and title visibility.
    pub fn set_chrome_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> Result<()> {
        self.with_window(id, |w|w.set_chrome_visible(visible))
    }

     /// Set the window top-left offset, includes the window chrome (outer-position).
    pub fn set_position(&mut self, _ctx: &Context, id: WinId, pos: LayoutPoint) -> Result<()> {
        self.with_window(id, |w|w.set_outer_pos(pos))
    }

     /// Set the window content area size (inner-size).
    pub fn set_size(&mut self, _ctx: &Context, id: WinId, size: LayoutSize) -> Result<()> {
        self.with_window(id, |w|w.resize_inner(size))
    }

    /// Set the window icon.
    pub fn set_icon(&mut self, _ctx: &Context, id: WinId, icon: Option<Icon>) -> Result<()> {
        self.with_window(id, |w|w.set_icon(icon))
    }

    /// Gets the root pipeline ID.
    pub fn pipeline_id(&mut self, _ctx: &Context, id: WinId) -> Result<PipelineId> {
        self.with_window(id, |w|w.pipeline_id())
    }

    /// Gets the resources namespace.
    pub fn namespace_id(&mut self, _ctx: &Context, id: WinId) -> Result<IdNamespace> {
        self.with_window(id, |w|w.namespace_id())
    }

    /// New image resource key.
    pub fn generate_image_key(&mut self, _ctx: &Context, id: WinId) -> Result<ImageKey> {
        self.with_window(id, |w|w.generate_image_key())
    }

    /// New font resource key.
    pub fn generate_font_key(&mut self, _ctx: &Context, id: WinId) -> Result<FontKey> {
        self.with_window(id, |w|w.generate_font_key())
    }

    /// New font instance key.
    pub fn generate_font_instance_key(&mut self, _ctx: &Context, id: WinId) -> Result<FontInstanceKey> {
        self.with_window(id, |w|w.generate_font_instance_key())
    }

    /// Gets the window content are size.
    pub fn size(&mut self, _ctx: &Context, id: WinId) -> Result<LayoutSize> {
        self.with_window(id, |w|w.inner_size())
    }

    /// Gets the window content are size.
    pub fn scale_factor(&mut self, _ctx: &Context, id: WinId) -> Result<f32> {
        self.with_window(id, |w|w.scale_factor())
    }

    /// In Windows, set if the `Alt+F4` should not cause a window close request and instead generate a key-press event.
    pub fn set_allow_alt_f4(&mut self, _ctx: &Context, id: WinId, allow: bool) -> Result<()> {
        self.with_window(id, |w|w.set_allow_alt_f4(allow))
    }

    /// Read all pixels of the current frame.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels(&mut self, _ctx: &Context, id: WinId) -> Result<FramePixels> {
        self.with_window(id, |w|w.read_pixels())
    }

    /// `glReadPixels` a new buffer.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels_rect(&mut self, _ctx: &Context, id: WinId, rect: LayoutRect) -> Result<FramePixels> {
        self.with_window(id, |w|w.read_pixels_rect(rect))
    }

    /// Get display items of the last rendered frame that intercept the `point`.
    ///
    /// Returns all hits from front-to-back.
    pub fn hit_test(&mut self, _ctx: &Context, id: WinId, point: LayoutPoint) -> Result<HitTestResult> {
        self.with_window(id, |w|w.hit_test(point))
    }

    /// Set the text anti-aliasing used in the window renderer.
    pub fn set_text_aa(&mut self, _ctx: &Context, id: WinId, aa: TextAntiAliasing) -> Result<()> {
        self.with_window(id, |w|w.set_text_aa(aa))
    }

    /// Render a new frame.
    pub fn render(&mut self, _ctx: &Context, id: WinId, frame: FrameRequest) -> Result<()> {
        self.with_window(id, |w|w.render(frame))
    }

    /// Update the current frame and re-render it.
    pub fn render_update(&mut self, _ctx: &Context, id: WinId, updates: DynamicProperties) -> Result<()> {
        self.with_window(id, |w|w.render_update(updates))
    }

    /// Add/remove/update resources such as images and fonts.
    pub fn update_resources(&mut self, _ctx: &Context, id: WinId, updates: Vec<ResourceUpdate>) -> Result<()> {
        self.with_window(id, |w|w.update_resources(updates))
    }
}

impl ViewApp {
    fn on_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        let (i, w) = if let Some(r) = self.windows.iter_mut().enumerate().find(|(_, w)| w.is_window(window_id)) {
            r
        } else {
            return;
        };
        let id = w.id();
        let scale_factor = w.scale_factor();
        match event {
            WindowEvent::Resized(size) => {
                let s = w.scale_factor();
                let size = LayoutSize::new(size.width as f32 / s, size.height as f32 / s);
                self.notify(Ev::WindowResized(id, size));
            }
            WindowEvent::Moved(p) => {
                let p = LayoutPoint::new(p.x as f32 / scale_factor, p.y as f32 / scale_factor);
                self.notify(Ev::WindowMoved(id, p));
            }
            WindowEvent::CloseRequested => self.notify(Ev::WindowCloseRequested(id)),
            WindowEvent::Destroyed => {
                self.windows.remove(i);
                self.notify(Ev::WindowClosed(id));
            }
            WindowEvent::DroppedFile(file) => self.notify(Ev::DroppedFile(id, file)),
            WindowEvent::HoveredFile(file) => self.notify(Ev::HoveredFile(id, file)),
            WindowEvent::HoveredFileCancelled => self.notify(Ev::HoveredFileCancelled(id)),
            WindowEvent::ReceivedCharacter(c) => self.notify(Ev::ReceivedCharacter(id, c)),
            WindowEvent::Focused(focused) => self.notify(Ev::Focused(id, focused)),
            WindowEvent::KeyboardInput { device_id, input, .. } => {
                let d_id = self.device_id(device_id);
                self.notify(Ev::KeyboardInput(id, d_id, input))
            }
            WindowEvent::ModifiersChanged(m) => self.notify(Ev::ModifiersChanged(id, m)),
            WindowEvent::CursorMoved { device_id, position, .. } => {
                let d_id = self.device_id(device_id);
                let p = LayoutPoint::new(position.x as f32 / scale_factor, position.y as f32 / scale_factor);
                self.notify(Ev::CursorMoved(id, d_id, p));
            }
            WindowEvent::CursorEntered { device_id } => {
                let d_id = self.device_id(device_id);
                self.notify(Ev::CursorEntered(id, d_id));
            }
            WindowEvent::CursorLeft { device_id } => {
                let d_id = self.device_id(device_id);
                self.notify(Ev::CursorLeft(id, d_id));
            }
            WindowEvent::MouseWheel {
                device_id, delta, phase, ..
            } => {
                let d_id = self.device_id(device_id);
                self.notify(Ev::MouseWheel(id, d_id, delta, phase));
            }
            WindowEvent::MouseInput {
                device_id, state, button, ..
            } => {
                let d_id = self.device_id(device_id);
                self.notify(Ev::MouseInput(id, d_id, state, button));
            }
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => {
                let d_id = self.device_id(device_id);
                self.notify(Ev::TouchpadPressure(id, d_id, pressure, stage));
            }
            WindowEvent::AxisMotion { device_id, axis, value } => {
                let d_id = self.device_id(device_id);
                self.notify(Ev::AxisMotion(id, d_id, axis, value));
            }
            WindowEvent::Touch(t) => {
                let d_id = self.device_id(t.device_id);
                let location = LayoutPoint::new(t.location.x as f32 / scale_factor, t.location.y as f32 / scale_factor);
                self.notify(Ev::Touch(id, d_id, t.phase, location, t.force.map(Into::into), t.id));
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => self.notify(Ev::ScaleFactorChanged(id, scale_factor as f32)),
            WindowEvent::ThemeChanged(t) => self.notify(Ev::ThemeChanged(id, t.into())),
        }
    }

    fn on_device_event(&mut self, device_id: DeviceId, event: DeviceEvent) {
        if self.device_events {
            let d_id = self.device_id(device_id);
            match event {
                DeviceEvent::Added => self.notify(Ev::DeviceAdded(d_id)),
                DeviceEvent::Removed => self.notify(Ev::DeviceRemoved(d_id)),
                DeviceEvent::MouseMotion { delta } => self.notify(Ev::DeviceMouseMotion(d_id, delta)),
                DeviceEvent::MouseWheel { delta } => self.notify(Ev::DeviceMouseWheel(d_id, delta)),
                DeviceEvent::Motion { axis, value } => self.notify(Ev::DeviceMotion(d_id, axis, value)),
                DeviceEvent::Button { button, state } => self.notify(Ev::DeviceButton(d_id, button, state)),
                DeviceEvent::Key(k) => self.notify(Ev::DeviceKey(d_id, k)),
                DeviceEvent::Text { codepoint } => self.notify(Ev::DeviceText(d_id, codepoint)),
            }
        }
    }

    fn on_frame_ready(&mut self, window_id: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.is_window(window_id)) {
            w.request_redraw();
        }
    }

    fn on_redraw(&mut self, window_id: WindowId) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.is_window(window_id)) {
            w.redraw();
        }
    }

    fn on_events_cleared(&mut self) {
        if self.pending_clear {
            self.notify(Ev::EventsCleared);
            self.pending_clear = false;
        }
    }
}

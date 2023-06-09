use std::{collections::VecDeque, fmt, mem};

use tracing::span::EnteredSpan;
use webrender::{
    api::{
        ColorF, DocumentId, DynamicProperties, FontInstanceKey, FontInstanceOptions, FontInstancePlatformOptions, FontKey, FontVariation,
        IdNamespace, ImageKey, PipelineId,
    },
    RenderApi, Renderer, Transaction, UploadMethod, VertexUsageHint,
};
use winit::{
    event_loop::EventLoopWindowTarget,
    monitor::{MonitorHandle, VideoMode as GVideoMode},
    window::{Fullscreen, Icon, Window as GWindow, WindowBuilder},
};
use zero_ui_view_api::{
    units::*, ApiExtensionId, ApiExtensionPayload, ColorScheme, CursorIcon, DeviceId, DisplayListCache, FocusIndicator, FrameId,
    FrameRequest, FrameUpdateRequest, ImageId, ImageLoadedData, RenderMode, VideoMode, ViewProcessGen, WindowId, WindowRequest,
    WindowState, WindowStateAll,
};

#[cfg(windows)]
use zero_ui_view_api::{Event, Key, KeyState, ScanCode};

use crate::{
    extensions::{
        BlobExtensionsImgHandler, DisplayListExtAdapter, RendererCommandArgs, RendererConfigArgs, RendererCreatedArgs, RendererExtension,
    },
    gl::{GlContext, GlContextManager},
    image_cache::{Image, ImageCache, ImageUseMap, WrImageCache},
    util::{CursorToWinit, DipToWinit, WinitToDip, WinitToPx},
    AppEvent, AppEventSender, FrameReadyMsg, WrNotifier,
};

/// A headed window.
pub(crate) struct Window {
    id: WindowId,
    pipeline_id: PipelineId,
    document_id: DocumentId,

    api: RenderApi,
    image_use: ImageUseMap,

    display_list_cache: DisplayListCache,
    clear_color: Option<ColorF>,

    context: GlContext, // context must be dropped before window.
    window: GWindow,
    renderer: Option<Renderer>,
    renderer_exts: Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
    capture_mode: bool,

    pending_frames: VecDeque<(FrameId, bool, Option<EnteredSpan>)>,
    rendered_frame_id: FrameId,
    kiosk: bool,

    resized: bool,

    video_mode: VideoMode,

    state: WindowStateAll,

    prev_pos: PxPoint,
    prev_size: PxSize,

    prev_monitor: Option<MonitorHandle>,

    visible: bool,
    is_always_on_top: bool,
    waiting_first_frame: bool,
    steal_init_focus: bool,
    init_focus_request: Option<FocusIndicator>,

    taskbar_visible: bool,

    movable: bool,

    cursor_pos: DipPoint,
    cursor_device: DeviceId,
    cursor_over: bool,

    focused: Option<bool>,

    render_mode: RenderMode,
}
impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .field("document_id", &self.document_id)
            .finish_non_exhaustive()
    }
}
impl Window {
    pub fn open(
        gen: ViewProcessGen,
        icon: Option<Icon>,
        mut cfg: WindowRequest,
        window_target: &EventLoopWindowTarget<AppEvent>,
        gl_manager: &mut GlContextManager,
        mut renderer_exts: Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
        event_sender: AppEventSender,
    ) -> Self {
        let id = cfg.id;

        let window_scope = tracing::trace_span!("glutin").entered();

        // create window and OpenGL context
        let mut winit = WindowBuilder::new()
            .with_title(cfg.title)
            .with_resizable(cfg.resizable)
            .with_transparent(cfg.transparent)
            .with_window_icon(icon);

        let mut s = cfg.state;
        s.clamp_size();

        if let WindowState::Normal = s.state {
            winit = winit
                .with_min_inner_size(s.min_size.to_winit())
                .with_max_inner_size(s.max_size.to_winit())
                .with_inner_size(s.restore_rect.size.to_winit());

            #[cfg(target_os = "linux")]
            if cfg.default_position {
                // default X11 position is outer zero.
                winit = winit.with_position(DipPoint::new(Dip::new(120), Dip::new(80)).to_winit());
            }
        } else if cfg.default_position {
            if let Some(screen) = window_target.primary_monitor() {
                // fallback to center.
                let screen_size = screen.size().to_px().to_dip(screen.scale_factor() as f32);
                s.restore_rect.origin.x = (screen_size.width - s.restore_rect.size.width) / 2.0;
                s.restore_rect.origin.y = (screen_size.height - s.restore_rect.size.height) / 2.0;
            }
        }

        winit = winit
            .with_decorations(s.chrome_visible)
            // we wait for the first frame to show the window,
            // so that there is no white frame when it's opening.
            //
            // unless its "kiosk" mode.
            .with_visible(cfg.kiosk);

        winit = match s.state {
            WindowState::Normal | WindowState::Minimized => winit,
            WindowState::Maximized => winit.with_maximized(true),
            WindowState::Fullscreen | WindowState::Exclusive => winit.with_fullscreen(Some(Fullscreen::Borderless(None))),
        };

        let mut render_mode = cfg.render_mode;
        if !cfg!(software) && render_mode == RenderMode::Software {
            tracing::warn!("ignoring `RenderMode::Software` because did not build with \"software\" feature");
            render_mode = RenderMode::Integrated;
        }

        let (winit_window, context) = gl_manager.create_headed(id, winit, window_target, render_mode);
        render_mode = context.render_mode();

        // * Extend the winit Windows window to not block the Alt+F4 key press.
        // * Check if the window is actually keyboard focused until first focus.
        #[cfg(windows)]
        {
            let event_sender = event_sender.clone();
            use winit::platform::windows::WindowExtWindows;

            let mut first_focus = false;

            let window_id = winit_window.id();
            let hwnd = winit_window.hwnd() as _;
            crate::util::set_raw_windows_event_handler(hwnd, u32::from_ne_bytes(*b"alf4") as _, move |_, msg, wparam, _| {
                if !first_focus && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() } == hwnd {
                    // Windows sends a `WM_SETFOCUS` when the window open, even if the user changed focus to something
                    // else before the process opens the window so that the window title bar shows the unfocused visual and
                    // we are not actually keyboard focused. We block this in `focused_changed` but then become out-of-sync
                    // with the native window state, to recover from this we check the system wide foreground window at every
                    // opportunity until we actually become the keyboard focus, at that point we can stop checking because we are in sync with
                    // the native window state and the native window state is in sync with the system wide state.
                    first_focus = true;
                    let _ = event_sender.send(AppEvent::WinitFocused(window_id, true));
                }

                if msg == windows_sys::Win32::UI::WindowsAndMessaging::WM_SYSKEYDOWN
                    && wparam as windows_sys::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY
                        == windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F4
                {
                    // winit always blocks ALT+F4 we want to allow it so that the shortcut is handled in the same way as other commands.

                    let _ = event_sender.send(AppEvent::Notify(Event::KeyboardInput {
                        window: id,
                        device: DeviceId::INVALID, // same as winit
                        scan_code: ScanCode(wparam as _),
                        state: KeyState::Pressed,
                        key: Some(Key::F4),
                    }));
                    return Some(0);
                }
                None
            });
        }

        drop(window_scope);
        let wr_scope = tracing::trace_span!("webrender").entered();

        // create renderer and start the first frame.

        let device_size = winit_window.inner_size().to_px().to_wr_device();

        let mut opts = webrender::WebRenderOptions {
            // text-aa config from Firefox.
            enable_aa: true,
            enable_subpixel_aa: cfg!(not(target_os = "android")),

            renderer_id: Some((gen.get() as u64) << 32 | id.get() as u64),

            // this clear color paints over the one set using `Renderer::set_clear_color`.
            clear_color: ColorF::new(0.0, 0.0, 0.0, 0.0),

            allow_advanced_blend_equation: context.is_software(),
            clear_caches_with_quads: !context.is_software(),
            enable_gpu_markers: !context.is_software(),

            // best for GL
            upload_method: UploadMethod::PixelBuffer(VertexUsageHint::Dynamic),

            // extensions expect this to be set.
            workers: Some(crate::util::wr_workers()),

            //panic_on_gl_error: true,
            ..Default::default()
        };
        let mut blobs = BlobExtensionsImgHandler(vec![]);
        for (id, ext) in &mut renderer_exts {
            let cfg = cfg
                .extensions
                .iter()
                .position(|(k, _)| k == id)
                .map(|i| cfg.extensions.swap_remove(i).1);

            ext.configure(&mut RendererConfigArgs {
                config: cfg,
                options: &mut opts,
                blobs: &mut blobs.0,
            });
        }
        if !opts.enable_multithreading {
            for b in &mut blobs.0 {
                b.enable_multithreading(false);
            }
        }
        opts.blob_image_handler = Some(Box::new(blobs));

        let (mut renderer, sender) =
            webrender::create_webrender_instance(context.gl().clone(), WrNotifier::create(id, event_sender), opts, None).unwrap();
        renderer.set_external_image_handler(WrImageCache::new_boxed());

        let mut api = sender.create_api();
        let document_id = api.add_document(device_size);
        let pipeline_id = webrender::api::PipelineId(gen.get(), id.get());

        renderer_exts.retain_mut(|(_, ext)| {
            ext.renderer_created(&mut RendererCreatedArgs {
                renderer: &mut renderer,
                api_sender: &sender,
                api: &mut api,
                document_id,
                pipeline_id,
            });
            !ext.is_config_only()
        });

        drop(wr_scope);

        let mut win = Self {
            id,
            image_use: ImageUseMap::default(),
            prev_pos: winit_window.inner_position().unwrap_or_default().to_px(),
            prev_size: winit_window.inner_size().to_px(),
            prev_monitor: winit_window.current_monitor(),
            state: s,
            kiosk: cfg.kiosk,
            window: winit_window,
            context,
            capture_mode: cfg.capture_mode,
            renderer: Some(renderer),
            renderer_exts,
            video_mode: cfg.video_mode,
            api,
            document_id,
            pipeline_id,
            resized: true,
            display_list_cache: DisplayListCache::new(pipeline_id),
            waiting_first_frame: true,
            steal_init_focus: cfg.focus,
            init_focus_request: cfg.focus_indicator,
            visible: cfg.visible,
            is_always_on_top: false,
            taskbar_visible: true,
            movable: cfg.movable,
            pending_frames: VecDeque::new(),
            rendered_frame_id: FrameId::INVALID,
            cursor_pos: DipPoint::zero(),
            cursor_device: DeviceId::INVALID,
            cursor_over: false,
            clear_color: None,
            focused: None,
            render_mode,
        };

        if !cfg.default_position && win.state.state == WindowState::Normal {
            win.set_inner_position(win.state.restore_rect.origin);
        }

        if cfg.always_on_top {
            win.set_always_on_top(true);
        }

        if win.state.state == WindowState::Normal && cfg.default_position {
            // system position.
            win.state.restore_rect.origin = win.window.inner_position().unwrap_or_default().to_px().to_dip(win.scale_factor());
        }

        #[cfg(windows)]
        if win.state.state != WindowState::Normal {
            win.windows_set_restore();
        }

        win.set_cursor(cfg.cursor);
        win.set_taskbar_visible(cfg.taskbar_visible);
        win
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        self.window.current_monitor()
    }

    pub fn window_id(&self) -> winit::window::WindowId {
        self.window.id()
    }

    pub fn id_namespace(&self) -> IdNamespace {
        self.api.get_namespace_id()
    }

    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Latest rendered frame.
    pub fn frame_id(&self) -> FrameId {
        self.rendered_frame_id
    }

    pub fn set_title(&self, title: String) {
        self.window.set_title(&title);
    }

    /// Returns `true` if the cursor actually moved.
    pub fn cursor_moved(&mut self, pos: DipPoint, device: DeviceId) -> bool {
        let moved = self.cursor_pos != pos || self.cursor_device != device;

        if moved {
            self.cursor_pos = pos;
            self.cursor_device = device;
        }

        moved && self.cursor_over
    }

    #[cfg(windows)]
    fn windows_is_foreground(&self) -> bool {
        use winit::platform::windows::WindowExtWindows;

        let foreground = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
        foreground == self.window.hwnd()
    }

    pub fn is_focused(&self) -> bool {
        self.focused.unwrap_or(false)
    }

    /// Returns `true` if the previous focused status is different from `focused`.
    ///
    /// Sets the `focused` to if the window is actually the foreground keyboard focused window.
    pub fn focused_changed(&mut self, focused: &mut bool) -> bool {
        #[cfg(windows)]
        if self.focused.is_none() {
            *focused = self.windows_is_foreground();
        }

        let focused = Some(*focused);

        let changed = self.focused != focused;
        if changed {
            self.focused = focused;
        }
        changed
    }

    /// Returns the last cursor moved data.
    pub fn last_cursor_pos(&self) -> (DipPoint, DeviceId) {
        (self.cursor_pos, self.cursor_device)
    }

    /// Returns `true` if the cursor was not over the window.
    pub fn cursor_entered(&mut self) -> bool {
        let changed = !self.cursor_over;
        self.cursor_over = true;
        changed
    }

    /// Returns `true` if the cursor was over the window.
    pub fn cursor_left(&mut self) -> bool {
        let changed = self.cursor_over;
        self.cursor_over = false;
        changed
    }

    pub fn set_visible(&mut self, visible: bool) {
        if self.kiosk && !self.visible {
            tracing::error!("window in `kiosk` mode cannot be hidden");
        }

        if !self.waiting_first_frame {
            let _s = tracing::trace_span!("set_visible", %visible).entered();

            self.visible = visible;

            if visible {
                if self.state.state != WindowState::Minimized {
                    self.window.set_minimized(false);
                }

                self.window.set_visible(true);
                self.apply_state(self.state.clone(), true);
            } else {
                if self.state.state != WindowState::Minimized {
                    // if the state is maximized or fullscreen the window is not hidden, a white
                    // "restored" window is shown instead.
                    self.window.set_minimized(true);
                }

                self.window.set_visible(false);
            }
        }
    }

    pub fn set_always_on_top(&mut self, always_on_top: bool) {
        self.window.set_window_level(if always_on_top {
            winit::window::WindowLevel::AlwaysOnTop
        } else {
            winit::window::WindowLevel::Normal
        });
        self.is_always_on_top = always_on_top;
    }

    pub fn set_movable(&mut self, movable: bool) {
        self.movable = movable;
    }

    pub fn set_resizable(&mut self, resizable: bool) {
        self.window.set_resizable(resizable)
    }

    #[cfg(windows)]
    pub fn bring_to_top(&mut self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        use winit::platform::windows::WindowExtWindows;

        if !self.is_always_on_top {
            let hwnd = self.window.hwnd();

            unsafe {
                let _ = SetWindowPos(
                    hwnd as _,
                    HWND_TOP,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );
            }
        }
    }

    #[cfg(not(windows))]
    pub fn bring_to_top(&mut self) {
        if !self.is_always_on_top {
            self.set_always_on_top(true);
            self.set_always_on_top(false);
        }
    }

    /// Returns `Some(new_pos)` if the window position is different from the previous call to this function.
    pub fn moved(&mut self) -> Option<DipPoint> {
        if !self.visible {
            return None;
        }

        let new_pos = self.window.inner_position().unwrap().to_px();
        if self.prev_pos != new_pos {
            self.prev_pos = new_pos;

            Some(new_pos.to_dip(self.scale_factor()))
        } else {
            None
        }
    }

    /// Returns `Some(new_size)` if the window size is different from the previous call to this function.
    pub fn resized(&mut self) -> Option<DipSize> {
        if !self.visible {
            return None;
        }

        let new_size = self.window.inner_size().to_px();
        if self.prev_size != new_size {
            self.prev_size = new_size;
            self.resized = true;

            Some(new_size.to_dip(self.scale_factor()))
        } else {
            None
        }
    }

    /// Returns `Some(new_monitor)` if the parent monitor changed from the previous call to this function.
    pub fn monitor_change(&mut self) -> Option<MonitorHandle> {
        let handle = self.window.current_monitor();
        if self.prev_monitor != handle {
            self.prev_monitor = handle.clone();
            handle
        } else {
            None
        }
    }

    #[cfg(windows)]
    fn windows_set_restore(&self) {
        use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO, MONITORINFOEXW};
        use windows_sys::Win32::{
            Foundation::{POINT, RECT},
            UI::WindowsAndMessaging::*,
        };
        use winit::platform::windows::{MonitorHandleExtWindows, WindowExtWindows};

        if let Some(monitor) = self.window.current_monitor() {
            let hwnd = self.window.hwnd() as _;
            let mut placement = WINDOWPLACEMENT {
                length: mem::size_of::<WINDOWPLACEMENT>() as _,
                flags: 0,
                showCmd: 0,
                ptMinPosition: POINT { x: 0, y: 0 },
                ptMaxPosition: POINT { x: 0, y: 0 },
                rcNormalPosition: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
            };
            if unsafe { GetWindowPlacement(hwnd, &mut placement) } != 0 {
                let scale_factor = self.scale_factor();
                let mut left_top = self.state.restore_rect.origin.to_px(scale_factor);

                // placement is in "workspace", window is in "virtual screen space".
                let hmonitor = monitor.hmonitor() as _;
                let mut monitor_info = MONITORINFOEXW {
                    monitorInfo: MONITORINFO {
                        cbSize: mem::size_of::<MONITORINFOEXW>() as _,
                        rcMonitor: RECT {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        },
                        rcWork: RECT {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        },
                        dwFlags: 0,
                    },
                    szDevice: [0; 32],
                };
                if unsafe { GetMonitorInfoW(hmonitor, &mut monitor_info as *mut MONITORINFOEXW as *mut MONITORINFO) } != 0 {
                    left_top.x.0 -= monitor_info.monitorInfo.rcWork.left;
                    left_top.y.0 -= monitor_info.monitorInfo.rcWork.top;
                }

                // placement includes the non-client area.
                let outer_offset =
                    self.window.outer_position().unwrap_or_default().to_px() - self.window.inner_position().unwrap_or_default().to_px();
                let size_offset = self.window.outer_size().to_px() - self.window.inner_size().to_px();

                left_top += outer_offset;
                let bottom_right = left_top + self.state.restore_rect.size.to_px(scale_factor) + size_offset;

                placement.rcNormalPosition.top = left_top.y.0;
                placement.rcNormalPosition.left = left_top.x.0;
                placement.rcNormalPosition.bottom = bottom_right.y.0;
                placement.rcNormalPosition.right = bottom_right.x.0;

                let _ = unsafe { SetWindowPlacement(hwnd, &placement) };
            }
        }
    }

    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.window.set_window_icon(icon);
    }

    /// Set cursor icon and visibility.
    pub fn set_cursor(&mut self, icon: Option<CursorIcon>) {
        if let Some(icon) = icon {
            self.window.set_cursor_icon(icon.to_winit());
            self.window.set_cursor_visible(true);
        } else {
            self.window.set_cursor_visible(false);
        }
    }

    /// Sets the focus request indicator.
    pub fn set_focus_request(&mut self, request: Option<FocusIndicator>) {
        if self.waiting_first_frame {
            self.init_focus_request = request;
        } else {
            self.window.request_user_attention(request.map(|r| match r {
                FocusIndicator::Critical => winit::window::UserAttentionType::Critical,
                FocusIndicator::Info => winit::window::UserAttentionType::Informational,
            }));
        }
    }

    /// Steal input focus.
    #[cfg(not(windows))]
    pub fn focus(&mut self) {
        if self.waiting_first_frame {
            self.steal_init_focus = true;
        } else {
            self.window.focus_window();
        }
    }

    /// Steal input focus.
    ///
    /// Returns if the next `RAlt` press and release key inputs must be ignored.
    #[cfg(windows)]
    #[must_use]
    pub fn focus(&mut self) -> bool {
        if self.waiting_first_frame {
            self.steal_init_focus = true;
            false
        } else if !self.windows_is_foreground() {
            // winit uses a hack to steal focus that causes a `RAlt` key press.
            self.window.focus_window();
            self.windows_is_foreground()
        } else {
            false
        }
    }

    /// Gets the current Maximized status as early as possible.
    fn is_maximized(&self) -> bool {
        #[cfg(windows)]
        {
            let hwnd = winit::platform::windows::WindowExtWindows::hwnd(&self.window);
            // SAFETY: function does not fail.
            return unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsZoomed(hwnd as _) } != 0;
        }

        #[allow(unreachable_code)]
        {
            // this changes only after the Resized event, we want state change detection before the Moved also.
            self.window.is_maximized()
        }
    }

    /// Gets the current Maximized status.
    fn is_minimized(&self) -> bool {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return true;
        }

        #[cfg(windows)]
        {
            let hwnd = winit::platform::windows::WindowExtWindows::hwnd(&self.window);
            // SAFETY: function does not fail.
            return unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd as _) } != 0;
        }

        #[allow(unreachable_code)]
        false
    }

    fn probe_state(&self) -> WindowStateAll {
        let mut state = self.state.clone();

        if self.is_minimized() {
            state.state = WindowState::Minimized;
        } else if let Some(h) = self.window.fullscreen() {
            state.state = match h {
                Fullscreen::Exclusive(_) => WindowState::Exclusive,
                Fullscreen::Borderless(_) => WindowState::Fullscreen,
            };
        } else if self.is_maximized() {
            state.state = WindowState::Maximized;
        } else {
            state.state = WindowState::Normal;

            let scale_factor = self.scale_factor();

            state.restore_rect = DipRect::new(
                self.window.inner_position().unwrap().to_px().to_dip(scale_factor),
                self.window.inner_size().to_px().to_dip(scale_factor),
            );
        }

        state
    }

    /// Probe state, returns `Some(new_state)`
    pub fn state_change(&mut self) -> Option<WindowStateAll> {
        if !self.visible {
            return None;
        }

        let mut new_state = self.probe_state();

        if self.state.state == WindowState::Minimized && self.state.restore_state == WindowState::Fullscreen {
            self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else if new_state.state == WindowState::Normal && self.state.state != WindowState::Normal {
            new_state.restore_rect = self.state.restore_rect;

            self.set_inner_position(new_state.restore_rect.origin);
            self.window.set_inner_size(new_state.restore_rect.size.to_winit());

            self.window.set_min_inner_size(Some(new_state.min_size.to_winit()));
            self.window.set_max_inner_size(Some(new_state.max_size.to_winit()));
        }

        new_state.set_restore_state_from(self.state.state);

        if new_state != self.state {
            self.state = new_state.clone();
            Some(new_state)
        } else {
            None
        }
    }

    fn video_mode(&self) -> Option<GVideoMode> {
        let mode = &self.video_mode;
        self.window.current_monitor().and_then(|m| {
            let mut candidate: Option<GVideoMode> = None;
            for m in m.video_modes() {
                // filter out video modes larger than requested
                if m.size().width <= mode.size.width.0 as u32
                    && m.size().height <= mode.size.height.0 as u32
                    && m.bit_depth() <= mode.bit_depth
                    && m.refresh_rate_millihertz() <= mode.refresh_rate
                {
                    // select closest match to the requested video mode
                    if let Some(c) = &candidate {
                        if m.size().width >= c.size().width
                            && m.size().height >= c.size().height
                            && m.bit_depth() >= c.bit_depth()
                            && m.refresh_rate_millihertz() >= c.refresh_rate_millihertz()
                        {
                            candidate = Some(m);
                        }
                    } else {
                        candidate = Some(m);
                    }
                }
            }
            candidate
        })
    }

    pub fn set_video_mode(&mut self, mode: VideoMode) {
        self.video_mode = mode;
        if let WindowState::Exclusive = self.state.state {
            self.window.set_fullscreen(None);

            if let Some(mode) = self.video_mode() {
                self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
            } else {
                self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }
    }

    #[cfg(not(windows))]
    pub fn set_taskbar_visible(&mut self, visible: bool) {
        if visible != self.taskbar_visible {
            return;
        }
        self.taskbar_visible = visible;
        tracing::error!("`set_taskbar_visible` not implemented for this OS");
    }

    #[cfg(windows)]
    pub fn set_taskbar_visible(&mut self, visible: bool) {
        if visible == self.taskbar_visible {
            return;
        }
        self.taskbar_visible = visible;

        use windows_sys::Win32::System::Com::*;
        use winit::platform::windows::WindowExtWindows;

        use crate::util::taskbar_com;

        // winit already initializes COM

        unsafe {
            let mut taskbar_list2: *mut taskbar_com::ITaskbarList2 = std::ptr::null_mut();
            match CoCreateInstance(
                &taskbar_com::CLSID_TaskbarList,
                std::ptr::null_mut(),
                CLSCTX_ALL,
                &taskbar_com::IID_ITaskbarList2,
                &mut taskbar_list2 as *mut _ as *mut _,
            ) {
                0 => {
                    let taskbar_list2 = taskbar_list2 as *mut taskbar_com::ITaskbarList2;

                    let result = if visible {
                        let add_tab = (*(*taskbar_list2).lpVtbl).parent.AddTab;
                        add_tab(taskbar_list2.cast(), self.window.hwnd() as _)
                    } else {
                        let delete_tab = (*(*taskbar_list2).lpVtbl).parent.DeleteTab;
                        delete_tab(taskbar_list2.cast(), self.window.hwnd() as _)
                    };
                    if result != 0 {
                        let mtd_name = if visible { "AddTab" } else { "DeleteTab" };
                        tracing::error!(
                            target: "window",
                            "cannot set `taskbar_visible`, `ITaskbarList::{mtd_name}` failed, error: 0x{result:x}",
                        )
                    }

                    let release = (*(*taskbar_list2).lpVtbl).parent.parent.Release;
                    let result = release(taskbar_list2.cast());
                    if result != 0 {
                        tracing::error!(
                            target: "window",
                            "failed to release `taskbar_list`, error: 0x{result:x}"
                        )
                    }
                }
                error => {
                    tracing::error!(
                        target: "window",
                        "cannot set `taskbar_visible`, failed to create instance of `ITaskbarList`, error: 0x{error:x}",
                    )
                }
            }
        }
    }

    /// Returns of the last update state.
    pub fn state(&self) -> WindowStateAll {
        self.state.clone()
    }

    #[cfg(windows)]
    /// Returns the preferred color scheme for the window.
    pub fn color_scheme(&self) -> ColorScheme {
        match self.window.theme().unwrap_or(winit::window::Theme::Light) {
            winit::window::Theme::Light => ColorScheme::Light,
            winit::window::Theme::Dark => ColorScheme::Dark,
        }
    }

    #[cfg(not(windows))]
    /// Returns the preferred color scheme for the window.
    pub fn color_scheme(&self) -> ColorScheme {
        tracing::error!("`color_scheme` not implemented for this OS");
        ColorScheme::default()
    }

    fn set_inner_position(&self, pos: DipPoint) {
        let outer_pos = self.window.outer_position().unwrap_or_default();
        let inner_pos = self.window.inner_position().unwrap_or_default();
        let inner_offset = PxVector::new(Px(outer_pos.x - inner_pos.x), Px(outer_pos.y - inner_pos.y)).to_dip(self.scale_factor());
        let pos = pos + inner_offset;
        self.window.set_outer_position(pos.to_winit());
    }

    /// Reset all window state.
    ///
    /// Returns `true` if the state changed.
    pub fn set_state(&mut self, new_state: WindowStateAll) -> bool {
        if self.state == new_state {
            return false;
        }

        if !self.visible {
            // will force apply when set to visible again.
            self.state = new_state;
            return true;
        }

        self.apply_state(new_state, false);

        true
    }

    fn apply_state(&mut self, new_state: WindowStateAll, force: bool) {
        if self.state.chrome_visible != new_state.chrome_visible {
            self.window.set_decorations(new_state.chrome_visible);
        }

        if self.state.state != new_state.state || force {
            // unset previous state.
            match self.state.state {
                WindowState::Normal => {}
                WindowState::Minimized => self.window.set_minimized(false),
                WindowState::Maximized => {
                    if !new_state.state.is_fullscreen() {
                        self.window.set_maximized(false);
                    }
                }
                WindowState::Fullscreen | WindowState::Exclusive => self.window.set_fullscreen(None),
            }

            // set new state.
            match new_state.state {
                WindowState::Normal => {}
                WindowState::Minimized => self.window.set_minimized(true),
                WindowState::Maximized => self.window.set_maximized(true),
                WindowState::Fullscreen => {
                    self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                }
                WindowState::Exclusive => {
                    if let Some(mode) = self.video_mode() {
                        self.window.set_fullscreen(Some(Fullscreen::Exclusive(mode)));
                    } else {
                        self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }
                }
            }
        }

        self.state = new_state;

        if self.state.state == WindowState::Normal {
            self.set_inner_position(self.state.restore_rect.origin);
            self.window.set_inner_size(self.state.restore_rect.size.to_winit());

            self.window.set_min_inner_size(Some(self.state.min_size.to_winit()));
            self.window.set_max_inner_size(Some(self.state.max_size.to_winit()));

            // this can happen if minimized from "Task Manager"
            //
            // - Set to Fullscreen.
            // - Minimize from Windows Task Manager.
            // - Restore from Taskbar.
            // - Set the state to Normal.
            //
            // Without this hack the window stays minimized and then restores
            // Normal but at the fullscreen size.
            #[cfg(windows)]
            if self.is_minimized() {
                self.windows_set_restore();

                self.window.set_minimized(true);
                self.window.set_minimized(false);
            }
        }

        // Update restore placement for Windows to avoid rendering incorrect frame when the OS restores the window.
        //
        // Windows changes the size if it considers the window "restored", that is the case for `Normal` and `Borderless` fullscreen.
        #[cfg(windows)]
        if !matches!(self.state.state, WindowState::Normal | WindowState::Fullscreen) {
            self.windows_set_restore();
        }
    }

    pub fn use_image(&mut self, image: &Image) -> ImageKey {
        self.image_use.new_use(image, self.document_id, &mut self.api)
    }

    pub fn update_image(&mut self, key: ImageKey, image: &Image) {
        self.image_use.update_use(key, image, self.document_id, &mut self.api);
    }

    pub fn delete_image(&mut self, key: ImageKey) {
        self.image_use.delete(key, self.document_id, &mut self.api);
    }

    pub fn add_font(&mut self, font: Vec<u8>, index: u32) -> FontKey {
        let key = self.api.generate_font_key();
        let mut txn = webrender::Transaction::new();
        txn.add_raw_font(key, font, index);
        self.api.send_transaction(self.document_id, txn);
        key
    }

    pub fn delete_font(&mut self, key: FontKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font(key);
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn add_font_instance(
        &mut self,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<FontInstanceOptions>,
        plataform_options: Option<FontInstancePlatformOptions>,
        variations: Vec<FontVariation>,
    ) -> FontInstanceKey {
        let key = self.api.generate_font_instance_key();
        let mut txn = webrender::Transaction::new();
        txn.add_font_instance(key, font_key, glyph_size.to_wr().get(), options, plataform_options, variations);
        self.api.send_transaction(self.document_id, txn);
        key
    }

    pub fn delete_font_instance(&mut self, instance_key: FontInstanceKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font_instance(instance_key);
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn set_capture_mode(&mut self, enabled: bool) {
        self.capture_mode = enabled;
    }

    /// Start rendering a new frame.
    ///
    /// The [callback](#callback) will be called when the frame is ready to be [presented](Self::present).
    pub fn render(&mut self, frame: FrameRequest) {
        let _scope = tracing::trace_span!("render", ?frame.id).entered();

        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color);

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        self.push_resize(&mut txn);
        txn.generate_frame(frame.id.get(), frame.render_reasons());

        let display_list = frame.display_list.to_webrender(
            &mut DisplayListExtAdapter {
                extensions: &mut self.renderer_exts,
                transaction: &mut txn,
                renderer: self.renderer.as_mut().unwrap(),
                api: &mut self.api,
            },
            &mut self.display_list_cache,
        );

        txn.reset_dynamic_properties();
        txn.append_dynamic_properties(DynamicProperties {
            transforms: vec![],
            floats: vec![],
            colors: vec![],
        });

        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color);
        self.clear_color = Some(frame.clear_color);

        txn.set_display_list(frame.id.epoch(), (frame.pipeline_id, display_list));

        let frame_scope =
            tracing::trace_span!("<frame>", ?frame.id, capture_image = ?frame.capture_image, from_update = false, thread = "<webrender>")
                .entered();

        self.pending_frames.push_back((frame.id, frame.capture_image, Some(frame_scope)));

        self.api.send_transaction(self.document_id, txn);
    }

    /// Start rendering a new frame based on the data of the last frame.
    pub fn render_update(&mut self, frame: FrameUpdateRequest) {
        let _scope = tracing::trace_span!("render_update", ?frame.id).entered();

        let render_reasons = frame.render_reasons();

        if let Some(color) = frame.clear_color {
            self.clear_color = Some(color);
            self.renderer.as_mut().unwrap().set_clear_color(color);
        }

        let resized = self.resized;

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        self.push_resize(&mut txn);
        txn.generate_frame(self.frame_id().get(), render_reasons);

        let frame_scope = match self.display_list_cache.update(
            &mut DisplayListExtAdapter {
                extensions: &mut self.renderer_exts,
                transaction: &mut txn,
                renderer: self.renderer.as_mut().unwrap(),
                api: &mut self.api,
            },
            frame.transforms,
            frame.floats,
            frame.colors,
            frame.extensions,
            resized,
        ) {
            Ok(p) => {
                if let Some(p) = p {
                    txn.append_dynamic_properties(p);
                }

                tracing::trace_span!("<frame-update>", ?frame.id, capture_image = ?frame.capture_image, thread = "<webrender>")
            }
            Err(d) => {
                txn.reset_dynamic_properties();
                txn.append_dynamic_properties(DynamicProperties {
                    transforms: vec![],
                    floats: vec![],
                    colors: vec![],
                });

                txn.set_display_list(frame.id.epoch(), (self.pipeline_id, d));

                tracing::trace_span!("<frame>", ?frame.id, capture_image = ?frame.capture_image, from_update = true, thread = "<webrender>")
            }
        };

        self.pending_frames.push_back((frame.id, false, Some(frame_scope.entered())));

        self.api.send_transaction(self.document_id, txn);
    }

    /// Returns info for `FrameRendered` and if this is the first frame.
    #[must_use = "events must be generated from the result"]
    pub fn on_frame_ready(&mut self, msg: FrameReadyMsg, images: &mut ImageCache) -> FrameReadyResult {
        let (frame_id, capture, _) = self.pending_frames.pop_front().unwrap_or((self.rendered_frame_id, false, None));
        self.rendered_frame_id = frame_id;

        let first_frame = self.waiting_first_frame;

        if self.waiting_first_frame {
            let _s = tracing::trace_span!("first-draw").entered();
            debug_assert!(msg.composite_needed);

            self.waiting_first_frame = false;
            let s = self.window.inner_size();
            self.context.make_current();
            self.context.resize(s);
            self.redraw();
            if self.kiosk {
                self.window.request_redraw();
            } else if self.visible {
                self.set_visible(true);

                if mem::take(&mut self.steal_init_focus) {
                    self.window.focus_window();
                }
                if let Some(r) = self.init_focus_request.take() {
                    self.set_focus_request(Some(r));
                }
            }
        } else if msg.composite_needed {
            self.window.request_redraw();
        }

        let scale_factor = self.scale_factor();

        let image = if capture {
            let _s = tracing::trace_span!("capture_image").entered();
            if msg.composite_needed {
                self.redraw();
            }
            let renderer = self.renderer.as_mut().unwrap();
            Some(images.frame_image_data(renderer, PxRect::from_size(self.window.inner_size().to_px()), true, scale_factor))
        } else {
            None
        };

        FrameReadyResult {
            frame_id,
            image,
            first_frame,
        }
    }

    pub fn redraw(&mut self) {
        let span = tracing::trace_span!("redraw", stats = tracing::field::Empty).entered();

        self.context.make_current();

        let renderer = self.renderer.as_mut().unwrap();
        renderer.update();
        let s = self.window.inner_size();

        let r = renderer.render(s.to_px().to_wr_device(), 0).unwrap();
        span.record("stats", &tracing::field::debug(&r.stats));

        let _ = renderer.flush_pipeline_info();
        self.context.swap_buffers();
    }

    pub fn is_rendering_frame(&self) -> bool {
        !self.pending_frames.is_empty()
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;

            self.context.make_current();
            let size = self.window.inner_size();
            self.context.resize(size);

            let size = self.window.inner_size();
            txn.set_document_view(PxRect::from_size(size.to_px()).to_wr_device());
        }
    }

    pub fn frame_image(&mut self, images: &mut ImageCache) -> ImageId {
        let scale_factor = self.scale_factor();
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            PxRect::from_size(self.window.inner_size().to_px()),
            self.capture_mode,
            self.id,
            self.rendered_frame_id,
            scale_factor,
        )
    }

    pub fn frame_image_rect(&mut self, images: &mut ImageCache, rect: PxRect) -> ImageId {
        let scale_factor = self.scale_factor();
        let rect = PxRect::from_size(self.window.inner_size().to_px())
            .intersection(&rect)
            .unwrap_or_default();
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            rect,
            self.capture_mode,
            self.id,
            self.rendered_frame_id,
            scale_factor,
        )
    }

    pub fn inner_position(&self) -> DipPoint {
        self.window
            .inner_position()
            .unwrap_or_default()
            .to_logical(self.window.scale_factor())
            .to_dip()
    }

    pub fn size(&self) -> DipSize {
        self.window.inner_size().to_logical(self.window.scale_factor()).to_dip()
    }

    pub fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    /// Window actual render mode.
    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    /// Calls the render extension command.
    pub fn render_extension(&mut self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> ApiExtensionPayload {
        for (key, ext) in &mut self.renderer_exts {
            if *key == extension_id {
                return ext.command(&mut RendererCommandArgs {
                    renderer: self.renderer.as_mut().unwrap(),
                    api: &mut self.api,
                    request,
                });
            }
        }
        ApiExtensionPayload::unknown_extension(extension_id)
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        self.api.stop_render_backend();
        self.api.shut_down(true);

        // webrender deinit panics if the context is not current.
        self.context.make_current();
        self.renderer.take().unwrap().deinit();
    }
}

pub(crate) struct FrameReadyResult {
    pub frame_id: FrameId,
    pub image: Option<ImageLoadedData>,
    pub first_frame: bool,
}

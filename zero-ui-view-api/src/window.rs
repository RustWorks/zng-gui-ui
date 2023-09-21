//! Window, surface and frame types.

use std::fmt;

use serde::{Deserialize, Serialize};
use webrender_api::{ColorF, Epoch, PipelineId, RenderReasons};

use crate::{
    access::AccessTree,
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    config::ColorScheme,
    display_list::{DisplayList, FrameValueUpdate},
    image::{ImageId, ImageLoadedData, ImageMaskMode},
    units::{Dip, DipPoint, DipRect, DipSize, Px, PxPoint, PxSize, PxToDip, PxTransform},
};

crate::declare_id! {
    /// Window ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is an unique id that survives View crashes.
    ///
    /// The App Process defines the ID.
    pub struct WindowId(_);

        /// Monitor screen ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is mapped to an unique id, but does not survived View crashes.
    ///
    /// The View Process defines the ID.
    pub struct MonitorId(_);

    /// Identifies a frame request for collaborative resize in [`WindowChanged`].
    ///
    /// The View Process defines the ID.
    pub struct FrameWaitId(_);
}

/// Render backend preference.
///
/// This is mostly a trade-off between performance and power consumption, but the cold startup time can also be a
/// concern, both `Dedicated` and `Integrated` load the system OpenGL driver, depending on the installed
/// drivers and hardware this can take up to 500ms in rare cases, in most systems this delay stays around 100ms
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RenderMode {
    /// Prefer the *best* dedicated GPU, probably the best performance after initialization, but also the
    /// most power consumption.
    ///
    /// Falls-back to `Integrated`, then `Software`.
    Dedicated,

    /// Prefer the integrated *GPU*, probably the best power consumption and good performance for most GUI applications,
    /// this is the default value.
    ///
    /// Falls-back to `Dedicated`, then `Software`.
    Integrated,

    /// Use a software render fallback, this has the best compatibility and best initialization time. This is probably the
    /// best pick for one frame render tasks and small windows where the initialization time of a GPU context may not offset
    /// the render time gains.
    ///
    /// If the view-process implementation has no software fallback it may use one of the GPUs.
    Software,
}
impl Default for RenderMode {
    /// [`RenderMode::Integrated`].
    fn default() -> Self {
        RenderMode::Integrated
    }
}
impl RenderMode {
    /// Returns fallbacks that view-process implementers will try if `self` is not available.
    pub fn fallbacks(self) -> [RenderMode; 2] {
        use RenderMode::*;
        match self {
            Dedicated => [Integrated, Software],
            Integrated => [Dedicated, Software],
            Software => [Integrated, Dedicated],
        }
    }

    /// Returns `self` plus [`fallbacks`].
    ///
    /// [`fallbacks`]: Self::fallbacks
    pub fn with_fallbacks(self) -> [RenderMode; 3] {
        let [f0, f1] = self.fallbacks();
        [self, f0, f1]
    }
}

/// Configuration of a new headless surface.
///
/// Headless surfaces are always [`capture_mode`] enabled.
///
/// [`capture_mode`]: WindowRequest::capture_mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessRequest {
    /// ID that will identify the new headless surface.
    ///
    /// The surface is identified by a [`WindowId`] so that some API methods
    /// can apply to both windows or surfaces, no actual window is created.
    pub id: WindowId,

    /// Scale for the layout units in this config.
    pub scale_factor: f32,

    /// Surface area (viewport size).
    pub size: DipSize,

    /// Render mode preference for this headless surface.
    pub render_mode: RenderMode,

    /// Config for renderer extensions.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
}

/// Information about a monitor screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Readable name of the monitor.
    pub name: String,
    /// Top-left offset of the monitor region in the virtual screen, in pixels.
    pub position: PxPoint,
    /// Width/height of the monitor region in the virtual screen, in pixels.
    pub size: PxSize,
    /// The monitor scale factor.
    pub scale_factor: f32,
    /// Exclusive fullscreen video modes.
    pub video_modes: Vec<VideoMode>,

    /// If could determine this monitor is the primary.
    pub is_primary: bool,
}
impl MonitorInfo {
    /// Returns the `size` descaled using the `scale_factor`.
    pub fn dip_size(&self) -> DipSize {
        self.size.to_dip(self.scale_factor)
    }
}

/// Exclusive video mode info.
///
/// You can get this values from [`MonitorInfo::video_modes`]. Note that when setting the
/// video mode the actual system mode is selected by approximation, closest `size`, then `bit_depth` then `refresh_rate`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoMode {
    /// Resolution of this video mode.
    pub size: PxSize,
    /// The bit depth of this video mode, as in how many bits you have available per color.
    /// This is generally 24 bits or 32 bits on modern systems, depending on whether the alpha channel is counted or not.
    pub bit_depth: u16,
    /// The refresh rate of this video mode, in millihertz.
    pub refresh_rate: u32,
}
impl Default for VideoMode {
    fn default() -> Self {
        Self::MAX
    }
}
impl VideoMode {
    /// Default value, matches with the largest size, greatest bit-depth and refresh rate.
    pub const MAX: VideoMode = VideoMode {
        size: PxSize::new(Px::MAX, Px::MAX),
        bit_depth: u16::MAX,
        refresh_rate: u32::MAX,
    };
}
impl fmt::Display for VideoMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::MAX {
            write!(f, "MAX")
        } else {
            write!(
                f,
                "{}x{}, {}, {}hz",
                self.size.width.0,
                self.size.height.0,
                self.bit_depth,
                (self.refresh_rate as f32 * 0.001).round()
            )
        }
    }
}

/// Information about a successfully opened window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowOpenData {
    /// Window renderer ID namespace.
    pub id_namespace: webrender_api::IdNamespace,
    /// Window renderer pipeline.
    pub pipeline_id: webrender_api::PipelineId,

    /// Window complete state.
    pub state: WindowStateAll,

    /// Monitor that contains the window, if any.
    pub monitor: Option<MonitorId>,

    /// Final top-left offset of the window (excluding outer chrome).
    ///
    /// The values are the global position and the position in the monitor.
    pub position: (PxPoint, DipPoint),
    /// Final dimensions of the client area of the window (excluding outer chrome).
    pub size: DipSize,

    /// Final scale factor.
    pub scale_factor: f32,

    /// Actual render mode, can be different from the requested mode if it is not available.
    pub render_mode: RenderMode,

    /// Preferred color scheme.
    pub color_scheme: ColorScheme,
}

/// Information about a successfully opened headless surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessOpenData {
    /// Window renderer ID namespace.
    pub id_namespace: webrender_api::IdNamespace,
    /// Window renderer pipeline.
    pub pipeline_id: webrender_api::PipelineId,

    /// Actual render mode, can be different from the requested mode if it is not available.
    pub render_mode: RenderMode,
}
impl HeadlessOpenData {
    /// Create an *invalid* result, for when the surface can not be opened.
    pub fn invalid() -> Self {
        HeadlessOpenData {
            id_namespace: webrender_api::IdNamespace(0),
            pipeline_id: webrender_api::PipelineId::dummy(),
            render_mode: RenderMode::Software,
        }
    }

    /// If any of the data is invalid.
    pub fn is_invalid(&self) -> bool {
        let invalid = Self::invalid();
        self.pipeline_id == invalid.pipeline_id || self.id_namespace == invalid.id_namespace
    }
}

/// Represents a focus request indicator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FocusIndicator {
    /// Activate critical focus request.
    Critical,
    /// Activate informational focus request.
    Info,
}

/// Frame image capture request.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameCapture {
    /// Don't capture the frame.
    #[default]
    None,
    /// Captures a full BGRA8 image.
    Full,
    /// Captures an A8 mask image.
    Mask(ImageMaskMode),
}

/// Data for rendering a new frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameRequest {
    /// ID of the new frame.
    pub id: FrameId,
    /// Pipeline Tag.
    pub pipeline_id: PipelineId,

    /// Frame clear color.
    pub clear_color: ColorF,

    /// Display list.
    pub display_list: DisplayList,

    /// Create an image or mask from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is sent with the image.
    ///
    /// [`Event::FrameImageReady`]: crate::Event::FrameImageReady
    pub capture: FrameCapture,

    /// Identifies this frame as the response to the [`WindowChanged`] resized frame request.
    pub wait_id: Option<FrameWaitId>,
}
impl FrameRequest {
    /// Compute webrender analysis info.
    pub fn render_reasons(&self) -> RenderReasons {
        let mut reasons = RenderReasons::SCENE;

        if self.capture != FrameCapture::None {
            reasons |= RenderReasons::SNAPSHOT;
        }

        reasons
    }
}

/// Data for rendering a new frame that is derived from the current frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct FrameUpdateRequest {
    /// ID of the new frame.
    pub id: FrameId,

    /// Bound transforms.
    pub transforms: Vec<FrameValueUpdate<PxTransform>>,
    /// Bound floats.
    pub floats: Vec<FrameValueUpdate<f32>>,
    /// Bound colors.
    pub colors: Vec<FrameValueUpdate<ColorF>>,

    /// Render update extension key and payload.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,

    /// New clear color.
    pub clear_color: Option<ColorF>,

    /// Create an image or mask from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is send with the image.
    ///
    /// [`Event::FrameImageReady`]: crate::Event::FrameImageReady
    pub capture: FrameCapture,

    /// Identifies this frame as the response to the [`WindowChanged`] resized frame request.
    pub wait_id: Option<FrameWaitId>,
}
impl FrameUpdateRequest {
    /// A request that does nothing, apart from re-rendering the frame.
    pub fn empty(id: FrameId) -> FrameUpdateRequest {
        FrameUpdateRequest {
            id,
            transforms: vec![],
            floats: vec![],
            colors: vec![],
            extensions: vec![],
            clear_color: None,
            capture: FrameCapture::None,
            wait_id: None,
        }
    }

    /// If some property updates are requested.
    pub fn has_bounds(&self) -> bool {
        !(self.transforms.is_empty() && self.floats.is_empty() && self.colors.is_empty())
    }

    /// If this request does not do anything, apart from notifying
    /// a new frame if send to the renderer.
    pub fn is_empty(&self) -> bool {
        !self.has_bounds() && self.extensions.is_empty() && self.clear_color.is_none() && self.capture != FrameCapture::None
    }

    /// Compute webrender analysis info.
    pub fn render_reasons(&self) -> RenderReasons {
        let mut reasons = RenderReasons::empty();

        if self.has_bounds() {
            reasons |= RenderReasons::ANIMATED_PROPERTY;
        }

        if self.capture != FrameCapture::None {
            reasons |= RenderReasons::SNAPSHOT;
        }

        if self.clear_color.is_some() {
            reasons |= RenderReasons::CONFIG_CHANGE;
        }

        reasons
    }
}
impl fmt::Debug for FrameUpdateRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameUpdateRequest")
            .field("id", &self.id)
            .field("transforms", &self.transforms)
            .field("floats", &self.floats)
            .field("colors", &self.colors)
            .field("clear_color", &self.clear_color)
            .field("capture", &self.capture)
            .finish()
    }
}

/// Configuration of a new window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRequest {
    /// ID that will identify the new window.
    pub id: WindowId,
    /// Title text.
    pub title: String,

    /// Window state, position, size and restore rectangle.
    pub state: WindowStateAll,

    /// Lock-in kiosk mode.
    ///
    /// If `true` the app-process will only set fullscreen states, never hide or minimize the window, never
    /// make the window chrome visible and only request an opaque window. The view-process implementer is expected
    /// to also never exit the fullscreen state, even temporally.
    ///
    /// The app-process does not expect the view-process to configure the operating system to run in kiosk mode, but
    /// if possible to detect the view-process can assert that it is running in kiosk mode, logging an error if the assert fails.
    pub kiosk: bool,

    /// If the initial position should be provided the operating system,
    /// if this is not possible the `state.restore_rect.origin` is used.
    pub default_position: bool,

    /// Video mode used when the window is in exclusive state.
    pub video_mode: VideoMode,

    /// Window visibility.
    pub visible: bool,
    /// Window taskbar icon visibility.
    pub taskbar_visible: bool,
    /// If the window is "top-most".
    pub always_on_top: bool,
    /// If the user can move the window.
    pub movable: bool,
    /// If the user can resize the window.
    pub resizable: bool,
    /// Window icon.
    pub icon: Option<ImageId>,
    /// Window cursor icon and visibility.
    pub cursor: Option<CursorIcon>,
    /// If the window is see-through in pixels that are not fully opaque.
    pub transparent: bool,

    /// If all or most frames will be *screenshotted*.
    ///
    /// If `false` all resources for capturing frame images
    /// are discarded after each screenshot request.
    pub capture_mode: bool,

    /// Render mode preference for this window.
    pub render_mode: RenderMode,

    /// Focus request indicator on init.
    pub focus_indicator: Option<FocusIndicator>,

    /// Ensures the window is focused after open, if not set the initial focus is decided by
    /// the windows manager, usually focusing the new window only if the process that causes the window has focus.
    pub focus: bool,

    /// Config for renderer extensions.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,

    /// Initial accessibility info tree.
    ///
    /// This can be just the root node empty to lazy init, the [`Event::AccessInit`] event is send to the window
    /// if any accessibility service requests the tree, after the init the window should assume that accessibility
    /// info is required for the lifetime of the window and updates must be send.
    ///
    /// [`Event::AccessInit`]: crate::Event::AccessInit
    pub access_tree: AccessTree,
}
impl WindowRequest {
    /// Corrects invalid values if [`kiosk`] is `true`.
    ///
    /// An error is logged for each invalid value.
    ///
    /// [`kiosk`]: Self::kiosk
    pub fn enforce_kiosk(&mut self) {
        if self.kiosk {
            if !self.state.state.is_fullscreen() {
                tracing::error!("window in `kiosk` mode did not request fullscreen");
                self.state.state = WindowState::Exclusive;
            }
            if self.state.chrome_visible {
                tracing::error!("window in `kiosk` mode request chrome");
                self.state.chrome_visible = false;
            }
            if !self.visible {
                tracing::error!("window in `kiosk` mode can only be visible");
                self.visible = true;
            }
        }
    }
}

/// Represents the properties of a window that affect its position, size and state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowStateAll {
    /// The window state.
    pub state: WindowState,

    /// Position across monitors.
    ///
    /// This is mostly used to find a monitor to resolve the `restore_rect` in.
    pub global_position: PxPoint,

    /// Position and size of the window in the `Normal` state.
    ///
    /// The position is relative to the monitor.
    pub restore_rect: DipRect,

    /// What state the window goes too when "restored".
    ///
    /// The *restore* state that the window must be set to be restored, if the [current state] is [`Maximized`], [`Fullscreen`] or [`Exclusive`]
    /// the restore state is [`Normal`], if the [current state] is [`Minimized`] the restore state is the previous state.
    ///
    /// When the restore state is [`Normal`] the [`restore_rect`] defines the window position and size.
    ///
    ///
    /// [current state]: Self::state
    /// [`Maximized`]: WindowState::Maximized
    /// [`Fullscreen`]: WindowState::Fullscreen
    /// [`Exclusive`]: WindowState::Exclusive
    /// [`Normal`]: WindowState::Normal
    /// [`Minimized`]: WindowState::Minimized
    /// [`restore_rect`]: Self::restore_rect
    pub restore_state: WindowState,

    /// Minimal `Normal` size allowed.
    pub min_size: DipSize,
    /// Maximum `Normal` size allowed.
    pub max_size: DipSize,

    /// If the system provided outer-border and title-bar is visible.
    ///
    /// This is also called the "decoration" or "chrome" of the window.
    pub chrome_visible: bool,
}
impl WindowStateAll {
    /// Clamp the `restore_rect.size` to `min_size` and `max_size`.
    pub fn clamp_size(&mut self) {
        self.restore_rect.size = self.restore_rect.size.min(self.max_size).max(self.min_size)
    }

    /// Compute a value for [`restore_state`] given the previous [`state`] in `self` and the `new_state` and update the [`state`].
    ///
    /// [`restore_state`]: Self::restore_state
    /// [`state`]: Self::state
    pub fn set_state(&mut self, new_state: WindowState) {
        self.restore_state = Self::compute_restore_state(self.restore_state, self.state, new_state);
        self.state = new_state;
    }

    /// Compute a value for [`restore_state`] given the previous `prev_state` and the new [`state`] in `self`.
    ///
    /// [`restore_state`]: Self::restore_state
    /// [`state`]: Self::state
    pub fn set_restore_state_from(&mut self, prev_state: WindowState) {
        self.restore_state = Self::compute_restore_state(self.restore_state, prev_state, self.state);
    }

    fn compute_restore_state(restore_state: WindowState, prev_state: WindowState, new_state: WindowState) -> WindowState {
        if new_state == WindowState::Minimized {
            // restore to previous state from minimized.
            if prev_state != WindowState::Minimized {
                prev_state
            } else {
                WindowState::Normal
            }
        } else if new_state.is_fullscreen() && !prev_state.is_fullscreen() {
            // restore to maximized or normal from fullscreen.
            if prev_state == WindowState::Maximized {
                WindowState::Maximized
            } else {
                WindowState::Normal
            }
        } else if new_state == WindowState::Maximized {
            WindowState::Normal
        } else {
            // Fullscreen to/from Exclusive keeps the previous restore_state.
            restore_state
        }
    }
}

/// Describes the appearance of the mouse cursor.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CursorIcon {
    /// The platform-dependent default cursor.
    #[default]
    Default,
    /// A simple crosshair.
    Crosshair,
    /// A hand (often used to indicate links in web browsers).
    Hand,
    /// Self explanatory.
    Arrow,
    /// Indicates something is to be moved.
    Move,
    /// Indicates horizontal text that may be selected or edited.
    Text,
    /// Program busy indicator.
    Wait,
    /// Help indicator (often rendered as a "?")
    Help,
    /// Progress indicator. Shows that processing is being done. But in contrast
    /// with "Wait" the user may still interact with the program. Often rendered
    /// as a spinning beach ball, or an arrow with a watch or hourglass.
    Progress,

    /// Cursor showing that something cannot be done.
    NotAllowed,
    /// Indicates that a context menu is available.
    ContextMenu,
    /// Indicates a table cell or set of cells can be selected.
    Cell,
    /// Indicates vertical text that may be selected or edited.
    VerticalText,
    /// Indicates an alias or shortcut is to be created.
    Alias,
    /// Indicates something is to be copied.
    Copy,
    /// An item may not be dropped at the current location.
    NoDrop,
    /// Indicates something can be grabbed.
    Grab,
    /// Indicates something is grabbed.
    Grabbing,
    /// Something can be scrolled in any direction (panned).
    AllScroll,
    /// Something can be zoomed (magnified) in.
    ZoomIn,
    /// Something can be zoomed (magnified) out.
    ZoomOut,

    /// Indicate that the right vertical edge is to be moved left/right.
    EResize,
    /// Indicates that the top horizontal edge is to be moved up/down.
    NResize,
    /// Indicates that top-right corner is to be moved.
    NeResize,
    /// Indicates that the top-left corner is to be moved.
    NwResize,
    /// Indicates that the bottom vertical edge is to be moved up/down.
    SResize,
    /// Indicates that the bottom-right corner is to be moved.
    SeResize,
    /// Indicates that the bottom-left corner is to be moved.
    SwResize,
    /// Indicates that the left vertical edge is to be moved left/right.
    WResize,
    /// Indicates that the any of the vertical edges is to be moved left/right.
    EwResize,
    /// Indicates that the any of the horizontal edges is to be moved up/down.
    NsResize,
    /// Indicates that the top-right or bottom-left corners are to be moved.
    NeswResize,
    /// Indicates that the top-left or bottom-right corners are to be moved.
    NwseResize,
    /// Indicates that the item/column can be resized horizontally.
    ColResize,
    /// Indicates that the item/row can be resized vertically.
    RowResize,
}

impl CursorIcon {
    /// All cursor icons.
    pub const ALL: &'static [CursorIcon] = &[
        CursorIcon::Default,
        CursorIcon::Crosshair,
        CursorIcon::Hand,
        CursorIcon::Arrow,
        CursorIcon::Move,
        CursorIcon::Text,
        CursorIcon::Wait,
        CursorIcon::Help,
        CursorIcon::Progress,
        CursorIcon::NotAllowed,
        CursorIcon::ContextMenu,
        CursorIcon::Cell,
        CursorIcon::VerticalText,
        CursorIcon::Alias,
        CursorIcon::Copy,
        CursorIcon::NoDrop,
        CursorIcon::Grab,
        CursorIcon::Grabbing,
        CursorIcon::AllScroll,
        CursorIcon::ZoomIn,
        CursorIcon::ZoomOut,
        CursorIcon::EResize,
        CursorIcon::NResize,
        CursorIcon::NeResize,
        CursorIcon::NwResize,
        CursorIcon::SResize,
        CursorIcon::SeResize,
        CursorIcon::SwResize,
        CursorIcon::WResize,
        CursorIcon::EwResize,
        CursorIcon::NsResize,
        CursorIcon::NeswResize,
        CursorIcon::NwseResize,
        CursorIcon::ColResize,
        CursorIcon::RowResize,
    ];

    /// Estimated icon size and click spot in that size.
    pub fn size_and_spot(&self) -> (DipSize, DipPoint) {
        fn splat(s: f32, rel_pt: f32) -> (DipSize, DipPoint) {
            size(s, s, rel_pt, rel_pt)
        }
        fn size(w: f32, h: f32, rel_x: f32, rel_y: f32) -> (DipSize, DipPoint) {
            (
                DipSize::new(Dip::new_f32(w), Dip::new_f32(h)),
                DipPoint::new(Dip::new_f32(w * rel_x), Dip::new_f32(h * rel_y)),
            )
        }

        match self {
            CursorIcon::Crosshair
            | CursorIcon::Move
            | CursorIcon::Wait
            | CursorIcon::NotAllowed
            | CursorIcon::NoDrop
            | CursorIcon::Cell
            | CursorIcon::Grab
            | CursorIcon::Grabbing
            | CursorIcon::AllScroll => splat(20.0, 0.5),
            CursorIcon::Text | CursorIcon::NResize | CursorIcon::SResize | CursorIcon::NsResize => size(8.0, 20.0, 0.5, 0.5),
            CursorIcon::VerticalText | CursorIcon::EResize | CursorIcon::WResize | CursorIcon::EwResize => size(20.0, 8.0, 0.5, 0.5),
            _ => splat(20.0, 0.0),
        }
    }
}

/// Window state after a resize.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum WindowState {
    /// Window is visible but does not fill the screen.
    #[default]
    Normal,
    /// Window is only visible as an icon in the taskbar.
    Minimized,
    /// Window fills the screen, but not the parts reserved by the system, like the taskbar.
    Maximized,
    /// Window is chromeless and completely fills the screen, including over parts reserved by the system.
    Fullscreen,
    /// Window has exclusive access to the video output, so only the window content is visible.
    Exclusive,
}
impl WindowState {
    /// Returns `true` if `self` matches [`Fullscreen`] or [`Exclusive`].
    ///
    /// [`Fullscreen`]: WindowState::Fullscreen
    /// [`Exclusive`]: WindowState::Exclusive
    pub fn is_fullscreen(self) -> bool {
        matches!(self, Self::Fullscreen | Self::Exclusive)
    }
}

/// [`Event::FrameRendered`] payload.
///
/// [`Event::FrameRendered`]: crate::Event::FrameRendered
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventFrameRendered {
    /// Window that was rendered.
    pub window: WindowId,
    /// Frame that was rendered.
    pub frame: FrameId,
    /// Frame image, if one was requested with the frame request.
    pub frame_image: Option<ImageLoadedData>,
}

/// [`Event::WindowChanged`] payload.
///
/// [`Event::WindowChanged`]: crate::Event::WindowChanged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowChanged {
    // note that this payload is handled by `Event::coalesce`, add new fields there too.
    //
    /// Window that has changed state.
    pub window: WindowId,

    /// Window new state, is `None` if the window state did not change.
    pub state: Option<WindowStateAll>,

    /// Window new global position, is `None` if the window position did not change.
    ///
    /// The values are the global position and the position in the monitor.
    pub position: Option<(PxPoint, DipPoint)>,

    /// Window new monitor.
    ///
    /// The window's monitor change when it is moved enough so that most of the
    /// client area is in the new monitor screen.
    pub monitor: Option<MonitorId>,

    /// The window new size, is `None` if the window size did not change.
    pub size: Option<DipSize>,

    /// If the view-process is blocking the event loop for a time waiting for a frame for the new `size` this
    /// ID must be send with the frame to signal that it is the frame for the new size.
    ///
    /// Event loop implementations can use this to resize without visible artifacts
    /// like the clear color flashing on the window corners, there is a timeout to this delay but it
    /// can be a noticeable stutter, a [`render`] or [`render_update`] request for the window unblocks the loop early
    /// to continue the resize operation.
    ///
    /// [`render`]: crate::Api::render
    /// [`render_update`]: crate::Api::render_update
    pub frame_wait_id: Option<FrameWaitId>,

    /// What caused the change, end-user/OS modifying the window or the app.
    pub cause: EventCause,
}
impl WindowChanged {
    /// Create an event that represents window move.
    pub fn moved(window: WindowId, global_position: PxPoint, position: DipPoint, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: None,
            position: Some((global_position, position)),
            monitor: None,
            size: None,
            frame_wait_id: None,
            cause,
        }
    }

    /// Create an event that represents window parent monitor change.
    pub fn monitor_changed(window: WindowId, monitor: MonitorId, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: None,
            position: None,
            monitor: Some(monitor),
            size: None,
            frame_wait_id: None,
            cause,
        }
    }

    /// Create an event that represents window resized.
    pub fn resized(window: WindowId, size: DipSize, cause: EventCause, frame_wait_id: Option<FrameWaitId>) -> Self {
        WindowChanged {
            window,
            state: None,
            position: None,
            monitor: None,
            size: Some(size),
            frame_wait_id,
            cause,
        }
    }

    /// Create an event that represents [`WindowStateAll`] change.
    pub fn state_changed(window: WindowId, state: WindowStateAll, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: Some(state),
            position: None,
            monitor: None,
            size: None,
            frame_wait_id: None,
            cause,
        }
    }
}

/// Identifier of a frame or frame update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, bytemuck::NoUninit)]
#[repr(C)]
pub struct FrameId(u32, u32);
impl FrameId {
    /// Dummy frame ID.
    pub const INVALID: FrameId = FrameId(u32::MAX, u32::MAX);

    /// Create first frame id of a window.
    pub fn first() -> FrameId {
        FrameId(0, 0)
    }

    /// Create the next full frame ID after the current one.
    pub fn next(self) -> FrameId {
        let mut id = self.0.wrapping_add(1);
        if id == u32::MAX {
            id = 0;
        }
        FrameId(id, 0)
    }

    /// Create the next update frame ID after the current one.
    pub fn next_update(self) -> FrameId {
        let mut id = self.1.wrapping_add(1);
        if id == u32::MAX {
            id = 0;
        }
        FrameId(self.0, id)
    }

    /// Get the raw ID.
    pub fn get(self) -> u64 {
        (self.0 as u64) << 32 | (self.1 as u64)
    }

    /// Get the full frame ID.
    pub fn epoch(self) -> Epoch {
        Epoch(self.0)
    }

    /// Get the frame update ID.
    pub fn update(self) -> u32 {
        self.1
    }
}

/// Cause of a window state change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventCause {
    /// Operating system or end-user affected the window.
    System,
    /// App affected the window.
    App,
}

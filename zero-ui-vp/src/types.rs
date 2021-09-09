pub use glutin::event::{
    AxisId, ButtonId, ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
};
pub use glutin::window::CursorIcon;
use serde::{Deserialize, Serialize};
pub use serde_bytes::ByteBuf;
use std::time::Duration;
use std::{fmt, path::PathBuf};
use webrender::api::units::{LayoutPoint, LayoutSize};
use webrender::api::{BuiltDisplayListDescriptor, ColorF, Epoch, HitTestResult, PipelineId};

/// Window ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id that survives View crashes.
///
/// Zero is never an ID.
pub type WinId = u32;

/// Device ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id, but does not survived View crashes.
///
/// Zero is never an ID.
pub type DevId = u32;

/// Monitor screen ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id, but does not survived View crashes.
///
/// Zero is never an ID.
pub type MonId = u32;

/// View-process generation, starts at one and changes every respawn, it is never zero.
pub type ViewProcessGen = u32;

/// System/User events sent from the View Process.
#[repr(u32)]
#[derive(Debug, Serialize, Deserialize)]
pub enum Ev {
    /// The view-process crashed and respawned, all resources must be rebuild.
    ///
    /// The [`ViewProcessGen`] is the new generation, after the respawn.
    Respawned(ViewProcessGen),
    /// The event channel disconnected, probably because the view-process crashed.
    ///
    /// The [`ViewProcessGen`] is the generation of the view-process that was lost, it must be passed to
    /// [`Controller::handle_disconnect`].
    ///
    /// [`Controller::handle_disconnect`]: crate::Controller::handle_disconnect
    Disconnected(ViewProcessGen),
    /// A sequence of events that happened at the *same time* finished sending.
    ///
    /// The same device action can generate multiple events, this event is send after
    /// each such sequence of window and device events, even if it only one event.
    EventsCleared,

    /// A frame finished rendering.
    ///
    /// `EventsCleared` is not send after this event.
    FrameRendered(WinId, Epoch),

    // Window events
    WindowResized(WinId, LayoutSize, EventCause),
    WindowMoved(WinId, LayoutPoint, EventCause),
    DroppedFile(WinId, PathBuf),
    HoveredFile(WinId, PathBuf),
    HoveredFileCancelled(WinId),
    ReceivedCharacter(WinId, char),
    Focused(WinId, bool),
    KeyboardInput(WinId, DevId, KeyboardInput),
    ModifiersChanged(WinId, ModifiersState),
    CursorMoved(WinId, DevId, LayoutPoint, HitTestResult, Epoch),
    CursorEntered(WinId, DevId),
    CursorLeft(WinId, DevId),
    MouseWheel(WinId, DevId, MouseScrollDelta, TouchPhase),
    MouseInput(WinId, DevId, ElementState, MouseButton),
    TouchpadPressure(WinId, DevId, f32, i64),
    AxisMotion(WinId, DevId, AxisId, f64),
    Touch(WinId, DevId, TouchPhase, LayoutPoint, Option<Force>, u64),
    ScaleFactorChanged(WinId, f32),
    MonitorsChanged(Vec<(MonId, MonitorInfo)>),
    ThemeChanged(WinId, WindowTheme),
    WindowCloseRequested(WinId),
    WindowClosed(WinId),

    // Config events
    FontsChanged,
    TextAaChanged(TextAntiAliasing),
    MultiClickConfigChanged(MultiClickConfig),
    AnimationEnabledChanged(bool),
    KeyRepeatDelayChanged(Duration),

    // Raw device events
    DeviceAdded(DevId),
    DeviceRemoved(DevId),
    DeviceMouseMotion(DevId, (f64, f64)),
    DeviceMouseWheel(DevId, MouseScrollDelta),
    DeviceMotion(DevId, AxisId, f64),
    DeviceButton(DevId, ButtonId, ElementState),
    DeviceKey(DevId, KeyboardInput),
    DeviceText(DevId, char),
}

/// Cause of a window move or resize event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EventCause {
    /// Operating system or end-user moved or resized the window.
    System,
    /// App move resized the window.
    App,
}

/// Describes the force of a touch event
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Force {
    /// On iOS, the force is calibrated so that the same number corresponds to
    /// roughly the same amount of pressure on the screen regardless of the
    /// device.
    Calibrated {
        /// The force of the touch, where a value of 1.0 represents the force of
        /// an average touch (predetermined by the system, not user-specific).
        ///
        /// The force reported by Apple Pencil is measured along the axis of the
        /// pencil. If you want a force perpendicular to the device, you need to
        /// calculate this value using the `altitude_angle` value.
        force: f64,
        /// The maximum possible force for a touch.
        ///
        /// The value of this field is sufficiently high to provide a wide
        /// dynamic range for values of the `force` field.
        max_possible_force: f64,
        /// The altitude (in radians) of the stylus.
        ///
        /// A value of 0 radians indicates that the stylus is parallel to the
        /// surface. The value of this property is Pi/2 when the stylus is
        /// perpendicular to the surface.
        altitude_angle: Option<f64>,
    },
    /// If the platform reports the force as normalized, we have no way of
    /// knowing how much pressure 1.0 corresponds to – we know it's the maximum
    /// amount of force, but as to how much force, you might either have to
    /// press really really hard, or not hard at all, depending on the device.
    Normalized(f64),
}
impl From<glutin::event::Force> for Force {
    fn from(f: glutin::event::Force) -> Self {
        match f {
            glutin::event::Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            } => Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            },
            glutin::event::Force::Normalized(f) => Force::Normalized(f),
        }
    }
}

/// OS theme.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum WindowTheme {
    /// Dark text on light background.
    Light,

    /// Light text on dark background.
    Dark,
}
impl From<glutin::window::Theme> for WindowTheme {
    fn from(t: glutin::window::Theme) -> Self {
        match t {
            glutin::window::Theme::Light => WindowTheme::Light,
            glutin::window::Theme::Dark => WindowTheme::Dark,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Icon {
    pub rgba: ByteBuf,
    pub width: u32,
    pub height: u32,
}
impl fmt::Debug for Icon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Icon")
            .field("rgba", &format_args!("<{} bytes>", self.rgba.len()))
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

/// Text anti-aliasing.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAntiAliasing {
    /// Uses the operating system configuration.
    Default,
    /// Sub-pixel anti-aliasing if a fast implementation is available, otherwise uses `Alpha`.
    Subpixel,
    /// Alpha blending anti-aliasing.
    Alpha,
    /// Disable anti-aliasing.
    Mono,
}
impl Default for TextAntiAliasing {
    fn default() -> Self {
        Self::Default
    }
}
impl fmt::Debug for TextAntiAliasing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "TextAntiAliasing::")?;
        }
        match self {
            TextAntiAliasing::Default => write!(f, "Default"),
            TextAntiAliasing::Subpixel => write!(f, "Subpixel"),
            TextAntiAliasing::Alpha => write!(f, "Alpha"),
            TextAntiAliasing::Mono => write!(f, "Mono"),
        }
    }
}

/// The View-Process crashed and respawned, all resources must be recreated.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Respawned;
impl fmt::Display for Respawned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "view-process crashed and respawned, all resources must be rebuild")
    }
}
impl std::error::Error for Respawned {}

/// View Process IPC result.
pub type Result<T> = std::result::Result<T, Respawned>;

/// Data for rendering a new frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct FrameRequest {
    /// Frame Tag.
    pub id: Epoch,
    /// Pipeline Tag.
    pub pipeline_id: PipelineId,

    /// Window inner size in layout pixels.
    ///
    /// This is both the viewport_size and document_size for webrender
    /// as we don't do root level scrolling.
    pub size: LayoutSize,

    /// Display list, split in serializable parts.
    pub display_list: (ByteBuf, BuiltDisplayListDescriptor),
}
impl fmt::Debug for FrameRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameRequest")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Configuration of a window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Title text.
    pub title: String,
    /// Top-left offset, including the chrome (outer-position).
    ///
    /// If *x* or *y* are not-finite the initial position is not set and the OS defines it.
    pub pos: LayoutPoint,
    /// Content size (inner-size).
    pub size: LayoutSize,

    /// Minimal size allowed.
    pub min_size: LayoutSize,
    /// Maximum size allowed.
    pub max_size: LayoutSize,

    /// Window visibility.
    pub visible: bool,
    /// Window taskbar icon visibility.
    pub taskbar_visible: bool,
    /// Window chrome visibility (decoration-visibility).
    pub chrome_visible: bool,
    /// In Windows, if `Alt+F4` does **not** causes a close request and instead causes a key-press event.
    pub allow_alt_f4: bool,
    /// If the window is "top-most".
    pub always_on_top: bool,
    /// If the user can move the window.
    pub movable: bool,
    /// If the user can resize the window.
    pub resizable: bool,
    /// Window icon.
    pub icon: Option<Icon>,
    /// If the window is see-through.
    pub transparent: bool,

    /// OpenGL clear color.
    pub clear_color: Option<ColorF>,
    /// Text anti-aliasing.
    pub text_aa: TextAntiAliasing,
}

/// Configuration of a headless surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessConfig {
    /// Scale for the layout units in this config.
    pub scale_factor: f32,

    /// Surface area (viewport size).
    pub size: LayoutSize,

    /// OpenGL clear color.
    pub clear_color: Option<ColorF>,

    /// Text anti-aliasing.
    pub text_aa: TextAntiAliasing,
}

/// BGRA8 pixel data copied from a rendered frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct FramePixels {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,

    /// BGRA8 data, bottom-to-top.
    pub bgra: ByteBuf,

    /// Scale factor when the frame was rendered.
    pub scale_factor: f32,

    /// If all alpha values are `255`.
    pub opaque: bool,
}
impl Default for FramePixels {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            bgra: ByteBuf::default(),
            scale_factor: 1.0,
            opaque: true,
        }
    }
}
impl fmt::Debug for FramePixels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FramePixels")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bgra", &format_args!("<{} bytes>", self.bgra.len()))
            .field("scale_factor", &self.scale_factor)
            .field("opaque", &self.opaque)
            .finish()
    }
}

/// Information about a monitor screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Readable name of the monitor.
    pub name: String,
    /// Top-left offset of the monitor region in the virtual screen, in pixels.
    pub position: (i32, i32),
    /// Width/height of the monitor region in the virtual screen, in pixels.
    pub size: (u32, u32),
    /// The monitor scale factor.
    pub scale_factor: f32,
    /// Exclusive fullscreen video modes.
    pub video_modes: Vec<VideoMode>,

    /// If could determine this monitor is the primary.
    pub is_primary: bool,
}
impl MonitorInfo {
    /// Returns the `size` descaled using the `scale_factor`.
    #[inline]
    pub fn layout_size(&self) -> LayoutSize {
        LayoutSize::new(self.size.0 as f32 / self.scale_factor, self.size.1 as f32 / self.scale_factor)
    }
}
impl<'a> From<&'a glutin::monitor::MonitorHandle> for MonitorInfo {
    fn from(m: &'a glutin::monitor::MonitorHandle) -> Self {
        let pos = m.position();
        let size = m.size();
        Self {
            name: m.name().unwrap_or_default(),
            position: (pos.x, pos.y),
            size: (size.width, size.height),
            scale_factor: m.scale_factor() as f32,
            video_modes: m.video_modes().map(Into::into).collect(),
            is_primary: false,
        }
    }
}
impl From<glutin::monitor::MonitorHandle> for MonitorInfo {
    fn from(m: glutin::monitor::MonitorHandle) -> Self {
        (&m).into()
    }
}

/// Exclusive video mode info.
///
/// You can get this values from [`MonitorInfo::video_modes`]. Note that when setting the
/// video mode the actual system mode is selected by approximation, closest `size`, then `bit_depth` then `refresh_rate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMode {
    /// Resolution of this video mode.
    pub size: (u32, u32),
    /// the bit depth of this video mode, as in how many bits you have available per color.
    /// This is generally 24 bits or 32 bits on modern systems, depending on whether the alpha channel is counted or not.
    pub bit_depth: u16,
    /// The refresh rate of this video mode.
    ///
    /// Note: the returned refresh rate is an integer approximation, and you shouldn’t rely on this value to be exact.
    pub refresh_rate: u16,
}
impl From<glutin::monitor::VideoMode> for VideoMode {
    fn from(v: glutin::monitor::VideoMode) -> Self {
        let size = v.size();
        Self {
            size: (size.width, size.height),
            bit_depth: v.bit_depth(),
            refresh_rate: v.refresh_rate(),
        }
    }
}

/// System settings needed for implementing double/triple clicks.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
pub struct MultiClickConfig {
    /// Maximum time interval between clicks.
    ///
    /// Only repeated clicks within this time interval can count as double-clicks.
    pub time: Duration,

    /// Maximum (x, y) distance in pixels.
    ///
    /// Only repeated clicks that are within this distance of the first click can count as double-clicks.
    pub area: (u32, u32),
}
impl Default for MultiClickConfig {
    /// `500ms` and `4, 4`.
    fn default() -> Self {
        Self {
            time: Duration::from_millis(500),
            area: (4, 4),
        }
    }
}

pub(crate) struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    pub fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}

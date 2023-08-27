//! Mouse and touch capture.

use std::fmt;

use crate::{
    app_local,
    context::{UPDATES, WIDGET, WINDOW},
    event::{event, event_args},
    var::*,
    widget_info::WidgetPath,
    widget_instance::WidgetId,
};

/// Mouse and touch capture service.
///
/// Mouse and touch events are redirected to the widget that holds capture when the widget is pressed.
#[allow(non_camel_case_types)]
pub struct POINTER_CAPTURE;
impl POINTER_CAPTURE {
    /// Variable that gets the current capture target and mode.
    pub fn current_capture(&self) -> ReadOnlyArcVar<Option<(WidgetPath, CaptureMode)>> {
        POINTER_CAPTURE_SV.read().capture.read_only()
    }

    /// Set a widget to redirect all mouse and touch events to.
    ///
    /// The capture will be set only if the widget is pressed.
    pub fn capture_widget(&self, widget_id: WidgetId) {
        let mut m = POINTER_CAPTURE_SV.write();
        m.capture_request = Some((widget_id, CaptureMode::Widget));
        UPDATES.update(None);
    }

    /// Set a widget to be the root of a capture subtree.
    ///
    /// Mouse and touch events targeting inside the subtree go to target normally. Mouse and touch events outside
    /// the capture root are redirected to the capture root.
    ///
    /// The capture will be set only if the widget is pressed.
    pub fn capture_subtree(&self, widget_id: WidgetId) {
        let mut m = POINTER_CAPTURE_SV.write();
        m.capture_request = Some((widget_id, CaptureMode::Subtree));
        UPDATES.update(None);
    }

    /// Release the current mouse and touch capture back to window.
    ///
    /// **Note:** The capture is released automatically when the mouse buttons or touch are released
    /// or when the window loses focus.
    pub fn release_capture(&self) {
        let mut m = POINTER_CAPTURE_SV.write();
        m.release_requested = true;
        UPDATES.update(None);
    }
}

/// Mouse and touch capture mode.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CaptureMode {
    /// Mouse and touch captured by the window only.
    ///
    /// Default behavior.
    Window,
    /// Mouse and touch events inside the widget sub-tree permitted. Mouse events
    /// outside of the widget redirected to the widget.
    Subtree,

    /// Mouse and touch events redirected to the widget.
    Widget,
}
impl fmt::Debug for CaptureMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "CaptureMode::")?;
        }
        match self {
            CaptureMode::Window => write!(f, "Window"),
            CaptureMode::Subtree => write!(f, "Subtree"),
            CaptureMode::Widget => write!(f, "Widget"),
        }
    }
}
impl Default for CaptureMode {
    /// [`CaptureMode::Window`]
    fn default() -> Self {
        CaptureMode::Window
    }
}
impl_from_and_into_var! {
    /// Convert `true` to [`CaptureMode::Widget`] and `false` to [`CaptureMode::Window`].
    fn from(widget: bool) -> CaptureMode {
        if widget {
            CaptureMode::Widget
        } else {
            CaptureMode::Window
        }
    }
}

/// Information about mouse and touch capture in a mouse or touch event argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureInfo {
    /// Widget that is capturing all mouse and touch events. The widget and all ancestors are [`ENABLED`].
    ///
    /// This is the window root widget for capture mode `Window`.
    ///
    /// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
    pub target: WidgetPath,
    /// Capture mode, see [`allows`](Self::allows) for more details.
    pub mode: CaptureMode,
}
impl CaptureInfo {
    /// If the widget is allowed by the current capture.
    ///
    /// This method uses [`WINDOW`] and [`WIDGET`] to identify the widget context.
    ///
    /// | Mode           | Allows                                             |
    /// |----------------|----------------------------------------------------|
    /// | `Window`       | All widgets in the same window.                    |
    /// | `Subtree`      | All widgets that have the `target` in their path.  |
    /// | `Widget`       | Only the `target` widget.                          |
    pub fn allows(&self) -> bool {
        match self.mode {
            CaptureMode::Window => self.target.window_id() == WINDOW.id(),
            CaptureMode::Widget => self.target.widget_id() == WIDGET.id(),
            CaptureMode::Subtree => {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(WIDGET.id()) {
                    for wgt in wgt.self_and_ancestors() {
                        if wgt.id() == self.target.widget_id() {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
}

app_local! {
    static POINTER_CAPTURE_SV: PointerCaptureService = PointerCaptureService {
        capture: var(None),
        capture_request: None,
        release_requested: false,
    };
}

struct PointerCaptureService {
    capture: ArcVar<Option<(WidgetPath, CaptureMode)>>,
    capture_request: Option<(WidgetId, CaptureMode)>,
    release_requested: bool,
}

event! {
    /// Pointer capture changed event.
    pub static POINTER_CAPTURE_EVENT: MouseCaptureArgs;
}

event_args! {

    /// [`MOUSE_CAPTURE_EVENT`] arguments.
    pub struct MouseCaptureArgs {
        /// Previous mouse capture target and mode.
        pub prev_capture: Option<(WidgetPath, CaptureMode)>,
        /// new mouse capture target and mode.
        pub new_capture: Option<(WidgetPath, CaptureMode)>,

        ..

        /// The [`prev_capture`] and [`new_capture`] paths start with the current path.
        ///
        /// [`prev_capture`]: Self::prev_capture
        /// [`new_capture`]: Self::new_capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            if let Some((p, _)) = &self.prev_capture {
                list.insert_path(p);
            }
            if let Some((p, _)) = &self.new_capture {
                list.insert_path(p);
            }
        }
    }
}

impl MouseCaptureArgs {
    /// If the same widget has mouse capture, but the widget path changed.
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (Some(prev), Some(new)) => prev.0.widget_id() == new.0.widget_id() && prev.0 != new.0,
            _ => false,
        }
    }

    /// If the same widget has mouse capture, but the capture mode changed.
    pub fn is_mode_change(&self) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (Some(prev), Some(new)) => prev.0.widget_id() == new.0.widget_id() && prev.1 != new.1,
            _ => false,
        }
    }

    /// If the `widget_id` lost mouse capture with this update.
    pub fn is_lost(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (None, _) => false,
            (Some((path, _)), None) => path.widget_id() == widget_id,
            (Some((prev_path, _)), Some((new_path, _))) => prev_path.widget_id() == widget_id && new_path.widget_id() != widget_id,
        }
    }

    /// If the `widget_id` got mouse capture with this update.
    pub fn is_got(&self, widget_id: WidgetId) -> bool {
        match (&self.prev_capture, &self.new_capture) {
            (_, None) => false,
            (None, Some((path, _))) => path.widget_id() == widget_id,
            (Some((prev_path, _)), Some((new_path, _))) => prev_path.widget_id() != widget_id && new_path.widget_id() == widget_id,
        }
    }
}

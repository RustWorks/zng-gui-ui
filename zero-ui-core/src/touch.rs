//! Touch events and service.
//!
//! The app extension [`TouchManager`] provides the events and service. It is included in the default application.

use std::{mem, time::Instant};

pub use zero_ui_view_api::{TouchConfig, TouchForce, TouchId, TouchPhase, TouchUpdate};

use crate::{
    app::{raw_events::*, *},
    context::*,
    event::*,
    keyboard::{ModifiersState, MODIFIERS_CHANGED_EVENT},
    pointer_capture::{CaptureInfo, POINTER_CAPTURE},
    units::*,
    var::*,
    widget_info::{HitTestInfo, InteractionPath},
    widget_instance::WidgetId,
    window::{WindowId, WINDOWS},
};

/// Application extension that provides touch events and service.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`TOUCH_MOVE_EVENT`]
/// * [`TOUCH_INPUT_EVENT`]
///
/// # Services
///
/// Services this extension provides.
///
/// * [`TOUCH`]
///
/// # Default
///
/// This extension is included in the [default app], events provided by it
/// are required by multiple other extensions.
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct TouchManager {
    tap_start: Option<TapStart>,
    modifiers: ModifiersState,
}

/// Touch service.
///
/// # Touch Capture
///
/// Touch capture is integrated with mouse capture in the [`POINTER_CAPTURE`] service.
///
/// # Provider
///
/// This service is provided by the [`TouchManager`] extension.
///
/// [`POINTER_CAPTURE`]: crate::pointer_capture::POINTER_CAPTURE
pub struct TOUCH;

impl TOUCH {
    /// Read-only variable that tracks the system touch config.
    ///
    /// # Value Source
    ///
    /// The value comes from the operating system settings, the variable
    /// updates with a new value if the system setting is changed.
    ///
    /// In headless apps the default is [`TouchConfig::default`] and does not change.
    ///
    /// Internally the [`RAW_TOUCH_CONFIG_CHANGED_EVENT`] is listened to update this variable, so you can notify
    /// this event to set this variable, if you really must.
    pub fn touch_config(&self) -> ReadOnlyArcVar<TouchConfig> {
        TOUCH_SV.read().touch_config.read_only()
    }
}

app_local! {
    static TOUCH_SV: TouchService = TouchService {
        touch_config: var(TouchConfig::default())
    };
}
struct TouchService {
    touch_config: ArcVar<TouchConfig>,
}

event_args! {
    /// Arguments for [`TOUCH_MOVE_EVENT`].
    pub struct TouchMoveArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Positions and force of touch moves in between the previous event and this one.
        ///
        /// Touch move events can be coalesced, i.e. multiple moves packed into a single event.
        pub coalesced: Vec<(TouchId, DipPoint, Option<TouchForce>)>,

        /// Identify a the touch contact or *finger*.
        ///
        /// Multiple points of contact can happen in the same device at the same time,
        /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
        /// on the same device, after a touch is ended an ID may be reused.
        pub touch: TouchId,

        /// Center of the touch in the window's content area.
        pub position: DipPoint,

        /// Touch pressure force and angle.
        pub force: Option<TouchForce>,

        /// Hit-test result for the touch point in the window.
        pub hits: HitTestInfo,

        /// Full path to the top-most hit in [`hits`](TouchMoveArgs::hits).
        pub target: InteractionPath,

        /// Current touch capture.
        pub capture: Option<CaptureInfo>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        ..

        /// The [`target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }

    /// Arguments for [`TOUCH_INPUT_EVENT`].
    pub struct TouchInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Identify a the touch contact or *finger*.
        ///
        /// Multiple points of contact can happen in the same device at the same time,
        /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
        /// on the same device, after a touch is ended an ID may be reused.
        pub touch: TouchId,

        /// Center of the touch in the window's content area.
        pub position: DipPoint,

        /// Touch pressure force and angle.
        pub force: Option<TouchForce>,

        /// Touch phase.
        ///
        /// Does not include `Moved`.
        pub phase: TouchPhase,

        /// Hit-test result for the touch point in the window.
        pub hits: HitTestInfo,

        /// Full path to the top-most hit in [`hits`](TouchInputArgs::hits).
        pub target: InteractionPath,

        /// Current touch capture.
        pub capture: Option<CaptureInfo>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        ..

        /// The [`target`] and [`capture`].
        ///
        /// [`target`]: Self::target
        /// [`capture`]: Self::capture
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target);
            if let Some(c) = &self.capture {
                list.insert_path(&c.target);
            }
        }
    }

        /// Arguments for [`TOUCH_TAP_EVENT`].
        pub struct TouchTapArgs {
            /// Id of window that received the event.
            pub window_id: WindowId,

            /// Id of device that generated the event.
            pub device_id: DeviceId,

            /// Identify a the touch contact or *finger*.
            ///
            /// Multiple points of contact can happen in the same device at the same time,
            /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
            /// on the same device, after a touch is ended an ID may be reused.
            pub touch: TouchId,

            /// Center of the touch in the window's content area.
            pub position: DipPoint,

            /// Hit-test result for the touch point in the window.
            pub hits: HitTestInfo,

            /// Full path to the top-most hit in [`hits`](TouchInputArgs::hits).
            pub target: InteractionPath,

            /// Current touch capture.
            pub capture: Option<CaptureInfo>,

            /// What modifier keys where pressed when this event happened.
            pub modifiers: ModifiersState,

            ..

            /// The [`target`] and [`capture`].
            ///
            /// [`target`]: Self::target
            /// [`capture`]: Self::capture
            fn delivery_list(&self, list: &mut UpdateDeliveryList) {
                list.insert_path(&self.target);
                if let Some(c) = &self.capture {
                    list.insert_path(&c.target);
                }
            }
        }
}

impl TouchMoveArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }
}

impl TouchInputArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }

    /// If the `widget_id` is in the [`target`] is enabled.
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// If the `widget_id` is in the [`target`] is disabled.
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }

    /// If the [`phase`] is start.
    ///
    /// [`phase`]: Self::phase
    pub fn is_touch_start(&self) -> bool {
        matches!(self.phase, TouchPhase::Start)
    }

    /// If the [`phase`] is start.
    ///
    /// [`phase`]: Self::phase
    pub fn is_touch_end(&self) -> bool {
        matches!(self.phase, TouchPhase::End)
    }

    /// If the [`phase`] is start.
    ///
    /// [`phase`]: Self::phase
    pub fn is_touch_cancel(&self) -> bool {
        matches!(self.phase, TouchPhase::Cancel)
    }
}

impl TouchTapArgs {
    /// If [`capture`] is `None` or [`allows`] the [`WIDGET`] to receive this event.
    ///
    /// [`capture`]: Self::capture
    /// [`allows`]: CaptureInfo::allows
    pub fn capture_allows(&self) -> bool {
        self.capture.as_ref().map(|c| c.allows()).unwrap_or(true)
    }

    /// If the `widget_id` is in the [`target`] is enabled.
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// If the `widget_id` is in the [`target`] is disabled.
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }
}

event! {
    /// Touch contact moved.
    pub static TOUCH_MOVE_EVENT: TouchMoveArgs;

    /// Touch contact started or ended.
    pub static TOUCH_INPUT_EVENT: TouchInputArgs;

    /// Touch tap.
    pub static TOUCH_TAP_EVENT: TouchTapArgs;
}

impl AppExtension for TouchManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_TOUCH_EVENT.on(update) {
            let mut pending_move = vec![];

            for u in &args.touches {
                if let TouchPhase::Move = u.phase {
                    pending_move.push((u.touch, u.position, u.force));
                } else {
                    self.on_move(args, mem::take(&mut pending_move));
                    self.on_input(args, u);
                }
            }

            self.on_move(args, pending_move);
        } else if let Some(args) = MODIFIERS_CHANGED_EVENT.on(update) {
            self.modifiers = args.modifiers;
        } else if let Some(args) = RAW_TOUCH_CONFIG_CHANGED_EVENT.on(update) {
            TOUCH_SV.read().touch_config.set(args.config);
        } else if let Some(args) = view_process::VIEW_PROCESS_INITED_EVENT.on(update) {
            TOUCH_SV.read().touch_config.set(args.touch_config);

            if args.is_respawn {
                self.tap_start = None;
            }
        }
    }
}
impl TouchManager {
    fn on_input(&mut self, args: &RawTouchArgs, update: &TouchUpdate) {
        if let Ok(w) = WINDOWS.widget_tree(args.window_id) {
            let hits = w.root().hit_test(update.position.to_px(w.scale_factor().0));
            let target = hits
                .target()
                .and_then(|t| w.get(t.widget_id))
                .map(|t| t.interaction_path())
                .unwrap_or_else(|| w.root().interaction_path());

            let capture_info = POINTER_CAPTURE.current_capture_value();

            let args = TouchInputArgs::now(
                args.window_id,
                args.device_id,
                update.touch,
                update.position,
                update.force,
                update.phase,
                hits,
                target,
                capture_info,
                self.modifiers,
            );

            if let Some(s) = self.tap_start.take() {
                s.try_complete(&args, update);
            } else {
                self.tap_start = TapStart::try_start(&args, update);
            }

            TOUCH_INPUT_EVENT.notify(args);
        }
    }

    fn on_move(&mut self, args: &RawTouchArgs, mut moves: Vec<(TouchId, DipPoint, Option<TouchForce>)>) {
        if let Some((touch, position, force)) = moves.pop() {
            if let Ok(w) = WINDOWS.widget_tree(args.window_id) {
                let hits = w.root().hit_test(position.to_px(w.scale_factor().0));
                let target = hits
                    .target()
                    .and_then(|t| w.get(t.widget_id))
                    .map(|t| t.interaction_path())
                    .unwrap_or_else(|| w.root().interaction_path());

                let capture_info = POINTER_CAPTURE.current_capture_value();

                let args = TouchMoveArgs::now(
                    args.window_id,
                    args.device_id,
                    moves,
                    touch,
                    position,
                    force,
                    hits,
                    target,
                    capture_info,
                    self.modifiers,
                );

                if let Some(s) = &self.tap_start {
                    if !s.retain(args.timestamp, args.window_id, args.device_id, touch, position) {
                        self.tap_start = None;
                    }
                }

                TOUCH_MOVE_EVENT.notify(args);
            }
        }
    }
}

struct TapStart {
    window_id: WindowId,
    device_id: DeviceId,
    touch: TouchId,

    timestamp: Instant,
    pos: DipPoint,

    propagation: EventPropagationHandle,
}
impl TapStart {
    /// Returns `Some(_)` if args could be the start of a tap event.
    fn try_start(args: &TouchInputArgs, update: &TouchUpdate) -> Option<Self> {
        if let TouchPhase::Start = update.phase {
            Some(Self {
                window_id: args.window_id,
                device_id: args.device_id,
                touch: update.touch,
                timestamp: args.timestamp,
                pos: update.position,
                propagation: args.propagation().clone(),
            })
        } else {
            None
        }
    }

    /// Check if the tap is still possible after a touch move..
    ///
    /// Returns `true` if it is.
    fn retain(&self, timestamp: Instant, window_id: WindowId, device_id: DeviceId, touch: TouchId, position: DipPoint) -> bool {
        if self.propagation.is_stopped() {
            // cancel, TOUCH_INPUT_EVENT handled.
            return false;
        }

        let cfg = TOUCH_SV.read().touch_config.get();

        if timestamp.duration_since(self.timestamp) > cfg.max_tap_time {
            // cancel, timeout.
            return false;
        }

        if window_id != self.window_id || device_id != self.device_id {
            // cancel, not same source or target.
            return false;
        }

        if touch != self.touch {
            // cancel, multi-touch.
            return false;
        }

        let dist = (position - self.pos).abs();
        if dist.x > cfg.tap_area.width || dist.y > cfg.tap_area.height {
            // cancel, moved too far
            return false;
        }

        // retain
        true
    }

    /// Complete or cancel the tap.
    fn try_complete(self, args: &TouchInputArgs, update: &TouchUpdate) {
        if !self.retain(args.timestamp, args.window_id, args.device_id, update.touch, update.position) {
            return;
        }

        if let TouchPhase::End = update.phase {
            TOUCH_TAP_EVENT.notify(TouchTapArgs::new(
                args.timestamp,
                args.propagation().clone(),
                self.window_id,
                self.device_id,
                self.touch,
                update.position,
                args.hits.clone(),
                args.target.clone(),
                args.capture.clone(),
                args.modifiers,
            ));
        }
    }
}

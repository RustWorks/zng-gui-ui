//! Mouse events.

use crate::core::app::*;
use crate::core::context::*;
use crate::core::event::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::window::Windows;
use std::num::NonZeroU8;
use std::time::*;

event_args! {
    /// [MouseMove] event args.
    pub struct MouseMoveArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Position of the mouse in the coordinates of [target](MouseMoveArgs::target).
        pub position: LayoutPoint,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Full path to the top-most hit in [hits](MouseMoveArgs::hits).
        pub target: WidgetPath,

        ..

        /// If the widget is in [target](MouseMoveArgs::target).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.widget_id)
         }
    }

    /// [MouseInput], [MouseDown], [MouseUp] event args.
    pub struct MouseInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Which mouse button generated the event.
        pub button: MouseButton,

        /// Position of the mouse in the coordinates of [target](MouseInputArgs::target).
        pub position: LayoutPoint,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// The state the [button](MouseInputArgs::button) was changed to.
        pub state: ElementState,

        /// Hit-test result for the mouse point in the window.
        pub hits: FrameHitInfo,

        /// Full path to the top-most hit in [hits](MouseInputArgs::hits).
        pub target: WidgetPath,

        ..

        /// If the widget is in [target](MouseInputArgs::target).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.widget_id)
        }
    }

    /// [MouseClick] event args.
    pub struct MouseClickArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Which mouse button generated the event.
        pub button: MouseButton,

        /// Position of the mouse in the coordinates of [target](MouseClickArgs::target).
        pub position: LayoutPoint,

         /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// Sequential click count . Number `1` is single click, `2` is double click, etc.
        pub click_count: NonZeroU8,

        /// Hit-test result for the mouse point in the window, at the moment the click event
        /// was generated.
        pub hits: FrameHitInfo,

        /// Full path to the widget that got clicked.
        ///
        /// A widget is clicked if the [MouseDown] and [MouseUp] happen
        /// in sequence in the same widget. Subsequent clicks (double, triple)
        /// happen on [MouseDown].
        ///
        /// If a [MouseDown] happen in a child widget and the pointer is dragged
        /// to a larger parent widget and then let go ([MouseUp]), the click target
        /// is the parent widget.
        ///
        /// Multi-clicks (`[click_count](MouseClickArgs::click_count) > 1`) only happen to
        /// the same target.
        pub target: WidgetPath,

        ..

        /// If the widget is in [target](MouseClickArgs::target).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.hits.contains(ctx.widget_id)
        }
    }
}

/// Mouse move event.
pub struct MouseMove;
impl Event for MouseMove {
    type Args = MouseMoveArgs;
    const IS_HIGH_PRESSURE: bool = true;
}

/// Mouse down or up event.
pub struct MouseInput;
impl Event for MouseInput {
    type Args = MouseInputArgs;
}

/// Mouse down event.
pub struct MouseDown;
impl Event for MouseDown {
    type Args = MouseInputArgs;
}

/// Mouse up event.
pub struct MouseUp;
impl Event for MouseUp {
    type Args = MouseInputArgs;
}

/// Mouse click event, any [click_count](MouseClickArgs::click_count).
pub struct MouseClick;
impl Event for MouseClick {
    type Args = MouseClickArgs;
}

/// Mouse single-click event ([click_count](MouseClickArgs::click_count) = `1`).
pub struct MouseSingleClick;
impl Event for MouseSingleClick {
    type Args = MouseClickArgs;
}

/// Mouse double-click event ([click_count](MouseClickArgs::click_count) = `2`).
pub struct MouseDoubleClick;
impl Event for MouseDoubleClick {
    type Args = MouseClickArgs;
}

/// Mouse triple-click event ([click_count](MouseClickArgs::click_count) = `3`).
pub struct MouseTripleClick;
impl Event for MouseTripleClick {
    type Args = MouseClickArgs;
}

/// Application extension that provides mouse events.
///
/// # Events
///
/// Events this extension provides.
///
/// * [MouseMove]
/// * [MouseInput]
/// * [MouseDown]
/// * [MouseUp]
/// * [MouseClick]
/// * [MouseSingleClick]
/// * [MouseDoubleClick]
/// * [MouseTripleClick]
pub struct MouseEvents {
    /// last cursor move position.
    pos: LayoutPoint,
    /// last cursor move window.
    pos_window: Option<WindowId>,

    /// last modifiers.
    modifiers: ModifiersState,

    /// when the last mouse_down event happened.
    last_pressed: Instant,
    click_target: Option<WidgetPath>,
    click_count: u8,

    mouse_move: EventEmitter<MouseMoveArgs>,

    mouse_input: EventEmitter<MouseInputArgs>,
    mouse_down: EventEmitter<MouseInputArgs>,
    mouse_up: EventEmitter<MouseInputArgs>,

    mouse_click: EventEmitter<MouseClickArgs>,
    mouse_single_click: EventEmitter<MouseClickArgs>,
    mouse_double_click: EventEmitter<MouseClickArgs>,
    mouse_triple_click: EventEmitter<MouseClickArgs>,
}

impl Default for MouseEvents {
    fn default() -> Self {
        MouseEvents {
            pos: LayoutPoint::default(),
            pos_window: None,

            modifiers: ModifiersState::default(),

            last_pressed: Instant::now() - Duration::from_secs(60),
            click_target: None,
            click_count: 0,

            mouse_move: EventEmitter::new(true),

            mouse_input: EventEmitter::new(false),
            mouse_down: EventEmitter::new(false),
            mouse_up: EventEmitter::new(false),

            mouse_click: EventEmitter::new(false),
            mouse_single_click: EventEmitter::new(false),
            mouse_double_click: EventEmitter::new(false),
            mouse_triple_click: EventEmitter::new(false),
        }
    }
}

impl MouseEvents {
    fn on_mouse_input(&mut self, window_id: WindowId, device_id: DeviceId, state: ElementState, button: MouseButton, ctx: &mut AppContext) {
        let position = if self.pos_window == Some(window_id) {
            self.pos
        } else {
            LayoutPoint::default()
        };

        let windows = ctx.services.req::<Windows>();
        let hits = windows.hit_test(window_id, position).unwrap();
        let frame_info = windows.frame_info(window_id).unwrap();

        let (target, position) = if let Some(t) = hits.target() {
            (frame_info.find(t.widget_id).unwrap().path(), t.point)
        } else {
            (frame_info.root().path(), position)
        };

        let args = MouseInputArgs::now(
            window_id,
            device_id,
            button,
            position,
            self.modifiers,
            state,
            hits.clone(),
            target.clone(),
        );

        // on_mouse_input
        ctx.updates.push_notify(self.mouse_input.clone(), args.clone());

        match state {
            ElementState::Pressed => {
                // on_mouse_down
                ctx.updates.push_notify(self.mouse_down.clone(), args);

                self.click_count = self.click_count.saturating_add(1);
                let now = Instant::now();

                if self.click_count == 1 {
                    // first mouse press, could be a click if a Released happen on the same target.
                    self.click_target = Some(target);
                } else if self.click_count >= 2
                    && (now - self.last_pressed) < multi_click_time_ms()
                    && self.click_target.as_ref().unwrap() == &target
                {
                    // if click_count >= 2 AND the time is in multi-click range, AND is the same exact target.

                    let args = MouseClickArgs::new(
                        now,
                        window_id,
                        device_id,
                        button,
                        position,
                        self.modifiers,
                        NonZeroU8::new(self.click_count).unwrap(),
                        hits,
                        target,
                    );

                    // on_mouse_click (click_count > 1)

                    if self.click_count == 2 {
                        if self.mouse_double_click.has_listeners() {
                            ctx.updates.push_notify(self.mouse_double_click.clone(), args.clone());
                        }
                    } else if self.click_count == 3 && self.mouse_triple_click.has_listeners() {
                        ctx.updates.push_notify(self.mouse_triple_click.clone(), args.clone());
                    }

                    ctx.updates.push_notify(self.mouse_click.clone(), args);
                } else {
                    self.click_count = 0;
                    self.click_target = None;
                }
                self.last_pressed = now;
            }
            ElementState::Released => {
                // on_mouse_up
                ctx.updates.push_notify(self.mouse_up.clone(), args);

                if let Some(click_count) = NonZeroU8::new(self.click_count) {
                    if click_count.get() == 1 {
                        if let Some(target) = self.click_target.as_ref().unwrap().shared_ancestor(&target) {
                            //if MouseDown and MouseUp happened in the same target.

                            let args = MouseClickArgs::now(
                                window_id,
                                device_id,
                                button,
                                position,
                                self.modifiers,
                                click_count,
                                hits,
                                target.clone(),
                            );

                            self.click_target = Some(target);

                            if self.mouse_single_click.has_listeners() {
                                ctx.updates.push_notify(self.mouse_single_click.clone(), args.clone());
                            }

                            // on_mouse_click
                            ctx.updates.push_notify(self.mouse_click.clone(), args);
                        } else {
                            self.click_count = 0;
                            self.click_target = None;
                        }
                    }
                }
            }
        }
    }

    fn on_cursor_moved(&mut self, window_id: WindowId, device_id: DeviceId, position: LayoutPoint, ctx: &mut AppContext) {
        if position != self.pos || Some(window_id) != self.pos_window {
            self.pos = position;
            self.pos_window = Some(window_id);
            let windows = ctx.services.req::<Windows>();
            let hits = windows.hit_test(window_id, position).unwrap();
            let frame_info = windows.frame_info(window_id).unwrap();

            let (target, position) = if let Some(t) = hits.target() {
                (frame_info.find(t.widget_id).unwrap().path(), t.point)
            } else {
                (frame_info.root().path(), position)
            };

            let args = MouseMoveArgs::now(window_id, device_id, self.modifiers, position, hits, target);

            ctx.updates.push_notify(self.mouse_move.clone(), args);
        }
    }
}

impl AppExtension for MouseEvents {
    fn init(&mut self, r: &mut AppInitContext) {
        r.events.register::<MouseMove>(self.mouse_move.listener());

        r.events.register::<MouseInput>(self.mouse_input.listener());
        r.events.register::<MouseDown>(self.mouse_down.listener());
        r.events.register::<MouseUp>(self.mouse_up.listener());

        r.events.register::<MouseClick>(self.mouse_click.listener());
        r.events.register::<MouseClick>(self.mouse_click.listener());
        r.events.register::<MouseDoubleClick>(self.mouse_double_click.listener());
        r.events.register::<MouseTripleClick>(self.mouse_triple_click.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match *event {
            WindowEvent::CursorMoved { device_id, position, .. } => {
                self.on_cursor_moved(window_id, device_id, LayoutPoint::new(position.x as f32, position.y as f32), ctx)
            }
            WindowEvent::MouseInput {
                state, device_id, button, ..
            } => self.on_mouse_input(window_id, device_id, state, button, ctx),
            WindowEvent::ModifiersChanged(m) => self.modifiers = m,
            _ => {}
        }
    }
}

#[cfg(target_os = "windows")]
fn multi_click_time_ms() -> Duration {
    Duration::from_millis(u64::from(unsafe { winapi::um::winuser::GetDoubleClickTime() }))
}

#[cfg(not(target_os = "windows"))]
fn multi_click_time_ms() -> u32 {
    // https://stackoverflow.com/questions/50868129/how-to-get-double-click-time-interval-value-programmatically-on-linux
    // https://developer.apple.com/documentation/appkit/nsevent/1532495-mouseevent
    Duration::from_millis(500)
}

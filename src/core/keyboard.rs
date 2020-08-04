//! Keyboard events.

use crate::core::app::*;
use crate::core::context::*;
use crate::core::event::*;
use crate::core::focus::Focus;
use crate::core::render::WidgetPath;
use crate::core::types::*;
use crate::core::window::Windows;
use std::time::Instant;

event_args! {
    /// [`KeyInput`](KeyInput), [`KeyDown`](KeyDown), [`KeyUp`](KeyUp) event args.
    pub struct KeyInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        pub device_id: DeviceId,

        /// Raw code of key.
        pub scancode: ScanCode,

        /// If the key was pressed or released.
        pub state: ElementState,

        /// Symbolic name of [`scancode`](KeyInputArgs::scancode).
        pub key: Option<VirtualKeyCode>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// If the key-down event was generated by holding the key pressed.
        pub repeat: bool,

        /// The focused element at the time of the key input.
        pub target: WidgetPath,

        ..

        /// If the widget is focused or contains the focused widget.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.widget_id)
         }
    }
}

/// Key pressed or released event.
pub struct KeyInputEvent;
impl Event for KeyInputEvent {
    type Args = KeyInputArgs;
}

/// Key pressed or repeat event.
pub struct KeyDownEvent;
impl Event for KeyDownEvent {
    type Args = KeyInputArgs;
}

/// Key released event.
pub struct KeyUpEvent;
impl Event for KeyUpEvent {
    type Args = KeyInputArgs;
}

/// Application extension that provides keyboard events.
///
/// # Events
///
/// Events this extension provides.
///
/// * [KeyInputEvent]
/// * [KeyDownEvent]
/// * [KeyUpEvent]
pub struct KeyboardEvents {
    last_key_down: Option<ScanCode>,
    modifiers: ModifiersState,
    key_input: EventEmitter<KeyInputArgs>,
    key_down: EventEmitter<KeyInputArgs>,
    key_up: EventEmitter<KeyInputArgs>,
}

impl Default for KeyboardEvents {
    fn default() -> Self {
        KeyboardEvents {
            last_key_down: None,
            modifiers: ModifiersState::default(),
            key_input: EventEmitter::new(false),
            key_down: EventEmitter::new(false),
            key_up: EventEmitter::new(false),
        }
    }
}

impl AppExtension for KeyboardEvents {
    fn init(&mut self, r: &mut AppInitContext) {
        r.events.register::<KeyInputEvent>(self.key_input.listener());
        r.events.register::<KeyDownEvent>(self.key_down.listener());
        r.events.register::<KeyUpEvent>(self.key_up.listener());
    }

    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent, ctx: &mut AppContext) {
        match *event {
            WindowEvent::KeyboardInput {
                device_id,
                input:
                    KeyboardInput {
                        scancode,
                        state,
                        virtual_keycode: key,
                        ..
                    },
                ..
            } => {
                let mut repeat = false;
                if state == ElementState::Pressed {
                    repeat = self.last_key_down == Some(scancode);
                    if !repeat {
                        self.last_key_down = Some(scancode);
                    }
                } else {
                    self.last_key_down = None;
                }

                let focused = ctx.services.get::<Focus>().and_then(|f| f.focused().cloned());
                let target = if let Some(focused) = focused {
                    focused
                } else {
                    ctx.services.req::<Windows>().window(window_id).unwrap().frame_info().root().path()
                };

                let args = KeyInputArgs {
                    timestamp: Instant::now(),
                    window_id,
                    device_id,
                    scancode,
                    key,
                    modifiers: self.modifiers,
                    state,
                    repeat,
                    target,
                };

                ctx.updates.push_notify(self.key_input.clone(), args.clone());

                match state {
                    ElementState::Pressed => {
                        ctx.updates.push_notify(self.key_down.clone(), args);
                    }
                    ElementState::Released => {
                        ctx.updates.push_notify(self.key_up.clone(), args);
                    }
                }
            }
            // Cache modifiers
            WindowEvent::ModifiersChanged(m) => self.modifiers = m,
            _ => {}
        }
    }
}

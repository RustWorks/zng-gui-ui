//! Keyboard manager.
//!
//! The [`KeyboardManager`] struct is an [app extension](crate::app::AppExtension). It
//! is included in the [default app](crate::app::App::default) and provides the [`Keyboard`] service
//! and keyboard input events.

use std::time::{Duration, Instant};

use crate::app::view_process::ViewProcessInitedEvent;
use crate::app::{raw_events::*, *};
use crate::context::*;
use crate::event::*;
use crate::focus::FocusExt;
use crate::service::*;
use crate::units::TimeUnits;
use crate::var::{var, RcVar, ReadOnlyRcVar, Var, Vars};
use crate::widget_info::WidgetPath;
use crate::window::WindowId;

pub use zero_ui_view_api::{Key, KeyState, ModifiersState, ScanCode};

event_args! {
    /// Arguments for [`KeyInputEvent`].
    pub struct KeyInputArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Raw code of key.
        pub scan_code: ScanCode,

        /// If the key was pressed or released.
        pub state: KeyState,

        /// Symbolic name of [`scan_code`](KeyInputArgs::scan_code).
        pub key: Option<Key>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// If the key-down event was generated by holding the key pressed.
        pub is_repeat: bool,

        /// The focused element at the time of the key input.
        pub target: WidgetPath,

        ..

        /// Returns `true` if the widget is the [`target`](Self::target) or is a parent of the target.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }

    /// Arguments for [`CharInputEvent`].
    pub struct CharInputArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Unicode character.
        pub character: char,

        /// The focused element at the time of the key input.
        pub target: WidgetPath,

        ..

        /// Returns `true` if the widget is the [`target`](Self::target) or is a parent of the target.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }

    /// Arguments for [`ModifiersChangedEvent`].
    pub struct ModifiersChangedArgs {
        /// Previous modifiers state.
        pub prev_modifiers: ModifiersState,

        /// Current modifiers state.
        pub modifiers: ModifiersState,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }
}

event! {
    /// Key pressed, repeat pressed or released event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub KeyInputEvent: KeyInputArgs;

    /// Modifiers key state changed event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub ModifiersChangedEvent: ModifiersChangedArgs;

    /// Character received event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub CharInputEvent: CharInputArgs;
}

/// Application extension that provides keyboard events targeting the focused widget.
///
/// This [extension] processes the raw keyboard events retargeting then to the focused widget, generating derived events and variables.
///
/// # Events
///
/// Events this extension provides.
///
/// * [KeyInputEvent]
/// * [ModifiersChangedEvent]
/// * [CharInputEvent]
///
/// # Services
///
/// Services this extension provides.
///
/// * [Keyboard]
///
/// # Default
///
/// This extension is included in the [default app], events provided by it
/// are required by multiple other extensions.
///
/// # Dependencies
///
/// This extension requires the [`Focus`] and [`Windows`] services before the first raw key input event. It does not
/// require anything for initialization.
///
/// [extension]: AppExtension
/// [default app]: crate::app::App::default
/// [`Focus`]: crate::focus::Focus
/// [`Windows`]: crate::window::Windows
#[derive(Default)]
pub struct KeyboardManager;
impl AppExtension for KeyboardManager {
    fn init(&mut self, r: &mut AppContext) {
        let kb = Keyboard::new();
        r.services.register(kb);
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = RawKeyInputEvent.update(args) {
            let focused = ctx.services.focus().focused().get_clone(ctx);
            let keyboard = ctx.services.keyboard();
            keyboard.key_input(ctx.events, ctx.vars, args, focused);
        } else if let Some(args) = RawModifiersChangedEvent.update(args) {
            let keyboard = ctx.services.keyboard();
            keyboard.set_modifiers(ctx.events, ctx.vars, args.modifiers);
        } else if let Some(args) = RawCharInputEvent.update(args) {
            let focused = ctx.services.focus().focused().get_clone(ctx);
            if let Some(target) = focused {
                if target.window_id() == args.window_id {
                    CharInputEvent.notify(ctx, CharInputArgs::now(args.window_id, args.character, target));
                }
            }
        } else if let Some(args) = RawKeyRepeatDelayChangedEvent.update(args) {
            let kb = ctx.services.keyboard();
            kb.repeat_delay.set_ne(ctx.vars, args.delay);
            kb.last_key_down = None;
        } else if let Some(args) = ViewProcessInitedEvent.update(args) {
            let kb = ctx.services.keyboard();
            kb.repeat_delay.set_ne(ctx.vars, args.key_repeat_delay);

            if args.is_respawn {
                kb.modifiers.set_ne(ctx.vars, ModifiersState::empty());
                kb.codes.set_ne(ctx.vars, vec![]);
                kb.keys.set_ne(ctx.vars, vec![]);

                kb.last_key_down = None;
            }
        }
    }
}

/// Keyboard service.
///
/// # Provider
///
/// This service is provided by the [`KeyboardManager`] extension.
#[derive(Service)]
pub struct Keyboard {
    // the `modifiers` variable only updates after a burst of raw events
    // we need the most current modifiers immediately.
    current_modifiers: ModifiersState,
    modifiers: RcVar<ModifiersState>,
    codes: RcVar<Vec<ScanCode>>,
    keys: RcVar<Vec<Key>>,
    repeat_delay: RcVar<Duration>,

    last_key_down: Option<(DeviceId, ScanCode, Instant)>,
}
impl Keyboard {
    fn new() -> Self {
        Keyboard {
            current_modifiers: ModifiersState::empty(),
            modifiers: var(ModifiersState::empty()),
            codes: var(vec![]),
            keys: var(vec![]),
            repeat_delay: var(600.ms()),
            last_key_down: None,
        }
    }

    fn set_modifiers(&mut self, events: &mut Events, vars: &Vars, modifiers: ModifiersState) {
        if self.current_modifiers != modifiers {
            self.modifiers.set(vars, modifiers);

            let prev_modifiers = self.current_modifiers;
            self.current_modifiers = modifiers;

            ModifiersChangedEvent.notify(events, ModifiersChangedArgs::now(prev_modifiers, modifiers));
        }
    }

    fn key_input(&mut self, events: &mut Events, vars: &Vars, args: &RawKeyInputArgs, focused: Option<WidgetPath>) {
        let mut repeat = false;

        // update state and vars
        match args.state {
            KeyState::Pressed => {
                if let Some((d_id, code, time)) = &mut self.last_key_down {
                    let max_t = self.repeat_delay.copy(vars) * 2;
                    if args.scan_code == *code && args.device_id == *d_id && (args.timestamp - *time) < max_t {
                        repeat = true;
                    } else {
                        *d_id = args.device_id;
                        *code = args.scan_code;
                    }
                    *time = args.timestamp;
                } else {
                    self.last_key_down = Some((args.device_id, args.scan_code, args.timestamp));
                }

                let scan_code = args.scan_code;
                if !self.codes.get(vars).contains(&scan_code) {
                    self.codes.modify(vars, move |mut cs| {
                        cs.push(scan_code);
                    });
                }

                if let Some(key) = args.key {
                    if !self.keys.get(vars).contains(&key) {
                        self.keys.modify(vars, move |mut ks| {
                            ks.push(key);
                        });
                    }
                }
            }
            KeyState::Released => {
                self.last_key_down = None;

                let key = args.scan_code;
                if self.codes.get(vars).contains(&key) {
                    self.codes.modify(vars, move |mut cs| {
                        if let Some(i) = cs.iter().position(|c| *c == key) {
                            cs.swap_remove(i);
                        }
                    });
                }

                if let Some(key) = args.key {
                    if self.keys.get(vars).contains(&key) {
                        self.keys.modify(vars, move |mut ks| {
                            if let Some(i) = ks.iter().position(|k| *k == key) {
                                ks.swap_remove(i);
                            }
                        });
                    }
                }
            }
        }

        // notify events
        if let Some(target) = focused {
            if target.window_id() == args.window_id {
                let args = KeyInputArgs::now(
                    args.window_id,
                    args.device_id,
                    args.scan_code,
                    args.state,
                    args.key,
                    self.current_modifiers,
                    repeat,
                    target,
                );
                KeyInputEvent.notify(events, args);
            }
        }
    }

    /// Returns a read-only variable  that tracks the currently pressed modifier keys.
    #[inline]
    pub fn modifiers(&self) -> ReadOnlyRcVar<ModifiersState> {
        self.modifiers.clone().into_read_only()
    }

    /// Returns a read-only variable that tracks the [`ScanCode`] of the keys currently pressed.
    #[inline]
    pub fn codes(&self) -> ReadOnlyRcVar<Vec<ScanCode>> {
        self.codes.clone().into_read_only()
    }

    /// Returns a read-only variable that tracks the [`Key`] identifier of the keys currently pressed.
    #[inline]
    pub fn keys(&self) -> ReadOnlyRcVar<Vec<Key>> {
        self.keys.clone().into_read_only()
    }

    /// Returns a read-only variable that tracks the operating system key press repeat delay.
    ///
    /// This delay is roughly the time the user must hold a key pressed to generate a new key
    /// press event. When a second key press happens without any other keyboard event and within twice this
    /// value if is marked [`is_repeat`] by the [`KeyboardManager`].
    ///
    /// [`is_repeat`]: KeyInputArgs::is_repeat
    #[inline]
    pub fn repeat_delay(&self) -> ReadOnlyRcVar<Duration> {
        self.repeat_delay.clone().into_read_only()
    }
}

// TODO refactor this.

/// Extension trait that adds keyboard simulation methods to [`HeadlessApp`].
pub trait HeadlessAppKeyboardExt {
    /// Does a keyboard input event.
    fn on_keyboard_input(&mut self, window_id: WindowId, key: Key, state: KeyState);

    /// Does a keyboard modifiers changed event.
    fn on_modifiers_changed(&mut self, window_id: WindowId, modifiers: ModifiersState);

    /// Does a key-down, key-up and updates.
    fn press_key(&mut self, window_id: WindowId, key: Key);

    /// Does a modifiers changed, key-down, key-up, reset modifiers and updates.
    fn press_modified_key(&mut self, window_id: WindowId, modifiers: ModifiersState, key: Key);
}
impl HeadlessAppKeyboardExt for HeadlessApp {
    fn on_keyboard_input(&mut self, window_id: WindowId, key: Key, state: KeyState) {
        use crate::app::raw_events::*;

        let args = RawKeyInputArgs::now(window_id, DeviceId::virtual_keyboard(), key as u32, state, Some(key));
        RawKeyInputEvent.notify(self.ctx().events, args);
    }

    fn on_modifiers_changed(&mut self, window_id: WindowId, modifiers: ModifiersState) {
        use crate::app::raw_events::*;

        let args = RawModifiersChangedArgs::now(window_id, modifiers);
        RawModifiersChangedEvent.notify(self.ctx().events, args);
    }

    fn press_key(&mut self, window_id: WindowId, key: Key) {
        self.on_keyboard_input(window_id, key, KeyState::Pressed);
        self.on_keyboard_input(window_id, key, KeyState::Released);
        let _ = self.update(false);
    }

    fn press_modified_key(&mut self, window_id: WindowId, modifiers: ModifiersState, key: Key) {
        if modifiers.is_empty() {
            self.press_key(window_id, key);
        } else {
            if modifiers.logo() {
                self.on_keyboard_input(window_id, Key::LLogo, KeyState::Pressed);
            }
            if modifiers.ctrl() {
                self.on_keyboard_input(window_id, Key::LCtrl, KeyState::Pressed);
            }
            if modifiers.shift() {
                self.on_keyboard_input(window_id, Key::LShift, KeyState::Pressed);
            }
            if modifiers.alt() {
                self.on_keyboard_input(window_id, Key::LAlt, KeyState::Pressed);
            }
            self.on_modifiers_changed(window_id, modifiers);

            // pressed the modifiers.
            let _ = self.update(false);

            self.on_keyboard_input(window_id, key, KeyState::Pressed);
            self.on_keyboard_input(window_id, key, KeyState::Released);

            // pressed the key.
            let _ = self.update(false);

            self.on_modifiers_changed(window_id, ModifiersState::default());
            if modifiers.logo() {
                self.on_keyboard_input(window_id, Key::LLogo, KeyState::Released);
            }
            if modifiers.ctrl() {
                self.on_keyboard_input(window_id, Key::LCtrl, KeyState::Released);
            }
            if modifiers.shift() {
                self.on_keyboard_input(window_id, Key::LShift, KeyState::Released);
            }
            if modifiers.alt() {
                self.on_keyboard_input(window_id, Key::LAlt, KeyState::Released);
            }

            // released the modifiers.
            let _ = self.update(false);
        }
    }
}

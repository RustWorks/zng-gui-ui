//! Keyboard manager.
//!
//! The [`KeyboardManager`] struct is an [app extension](crate::app::AppExtension), it
//! is included in the [default app](crate::app::App::default) and provides the [`KEYBOARD`] service
//! and keyboard input events.

use std::time::{Duration, Instant};

use crate::app::view_process::{AnimationsConfig, VIEW_PROCESS_INITED_EVENT};
use crate::app::{raw_events::*, *};
use crate::crate_util::FxHashSet;
use crate::event::*;
use crate::focus::FOCUS;
use crate::units::*;
use crate::var::{var, var_default, ArcVar, ReadOnlyArcVar, Var};
use crate::widget_info::InteractionPath;
use crate::window::WindowId;
use crate::{context::*, widget_instance::WidgetId};

pub use zero_ui_view_api::{Key, KeyRepeatConfig, KeyState, ScanCode};

event_args! {
    /// Arguments for [`KEY_INPUT_EVENT`].
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

        /// Number of repeats generated by holding the key pressed.
        ///
        /// This is zero for the first key press, increments by one for each event while the key is held pressed.
        pub repeat_count: u32,

        /// The focused element at the time of the key input.
        pub target: InteractionPath,

        ..

        /// The [`target`](Self::target).
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target)
        }
    }

    /// Arguments for [`CHAR_INPUT_EVENT`].
    pub struct CharInputArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Unicode character.
        pub character: char,

        /// The focused element at the time of the key input.
        pub target: InteractionPath,

        ..

        /// The [`target`](Self::target).
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.insert_path(&self.target)
        }
    }

    /// Arguments for [`MODIFIERS_CHANGED_EVENT`].
    pub struct ModifiersChangedArgs {
        /// Previous modifiers state.
        pub prev_modifiers: ModifiersState,

        /// Current modifiers state.
        pub modifiers: ModifiersState,

        ..

        /// Broadcast to all.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}
impl KeyInputArgs {
    /// Returns `true` if the widget is enabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// Returns `true` if the widget is disabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }
}
impl CharInputArgs {
    /// Returns `true` if the widget is enabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_enabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_enabled()).unwrap_or(false)
    }

    /// Returns `true` if the widget is disabled in [`target`].
    ///
    /// [`target`]: Self::target
    pub fn is_disabled(&self, widget_id: WidgetId) -> bool {
        self.target.interactivity_of(widget_id).map(|i| i.is_disabled()).unwrap_or(false)
    }

    /// Returns `true` if the character is the backspace.
    pub fn is_backspace(&self) -> bool {
        self.character == '\u{8}'
    }

    /// Returns `true` if the character is delete.
    pub fn is_delete(&self) -> bool {
        self.character == '\u{7F}'
    }

    /// Returns `true` if the character is the tab space.
    pub fn is_tab(&self) -> bool {
        "\t\u{B}\u{1F}".contains(self.character)
    }

    /// Returns `true` if the character is a line-break.
    pub fn is_line_break(&self) -> bool {
        "\r\n\u{85}".contains(self.character)
    }

    /// Gets the character to insert in a text string.
    ///
    /// Replaces all [`is_tab`] with `\t` and all [`is_line_break`] with `\n`.
    /// Returns `None` if the character must not be inserted.
    ///
    /// [`is_tab`]: Self::is_tab
    /// [`is_line_break`]: Self::is_line_break
    pub fn insert_char(&self) -> Option<char> {
        if self.is_tab() {
            Some('\t')
        } else if self.is_line_break() {
            Some('\n')
        } else if self.character.is_ascii_control() {
            None
        } else {
            Some(self.character)
        }
    }
}

event! {
    /// Key pressed, repeat pressed or released event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub static KEY_INPUT_EVENT: KeyInputArgs;

    /// Modifiers key state changed event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub static MODIFIERS_CHANGED_EVENT: ModifiersChangedArgs;

    /// Character received event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub static CHAR_INPUT_EVENT: CharInputArgs;
}

/// Application extension that provides keyboard events targeting the focused widget.
///
/// This [extension] processes the raw keyboard events retargeting then to the focused widget, generating derived events and variables.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`KEY_INPUT_EVENT`]
/// * [`MODIFIERS_CHANGED_EVENT`]
/// * [`CHAR_INPUT_EVENT`]
///
/// # Services
///
/// Services this extension provides.
///
/// * [`KEYBOARD`]
///
/// # Default
///
/// This extension is included in the [default app], events provided by it
/// are required by multiple other extensions.
///
/// # Dependencies
///
/// This extension requires the [`FOCUS`] and [`WINDOWS`] services before the first raw key input event. It does not
/// require anything for initialization.
///
/// [extension]: AppExtension
/// [default app]: crate::app::App::default
/// [`FOCUS`]: crate::focus::FOCUS
/// [`WINDOWS`]: crate::window::WINDOWS
#[derive(Default)]
pub struct KeyboardManager;
impl AppExtension for KeyboardManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_KEY_INPUT_EVENT.on(update) {
            let focused = FOCUS.focused().get();
            KEYBOARD_SV.write().key_input(args, focused);
        } else if let Some(args) = RAW_CHAR_INPUT_EVENT.on(update) {
            let focused = FOCUS.focused().get();
            if let Some(target) = focused {
                if target.window_id() == args.window_id {
                    CHAR_INPUT_EVENT.notify(CharInputArgs::now(args.window_id, args.character, target));
                }
            }
        } else if let Some(args) = RAW_KEY_REPEAT_CONFIG_CHANGED_EVENT.on(update) {
            let mut kb = KEYBOARD_SV.write();
            kb.repeat_config.set(args.config);
            kb.last_key_down = None;
        } else if let Some(args) = RAW_ANIMATIONS_CONFIG_CHANGED_EVENT.on(update) {
            let kb = KEYBOARD_SV.read();
            kb.caret_animation_config
                .set((args.config.caret_blink_interval, args.config.caret_blink_timeout));
        } else if let Some(args) = RAW_WINDOW_FOCUS_EVENT.on(update) {
            if args.new_focus.is_none() {
                let mut kb = KEYBOARD_SV.write();
                kb.clear_modifiers();
                kb.codes.set(vec![]);
                kb.keys.set(vec![]);

                kb.last_key_down = None;
            }
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            let mut kb = KEYBOARD_SV.write();
            kb.repeat_config.set(args.key_repeat_config);
            kb.caret_animation_config.set((
                args.animations_config.caret_blink_interval,
                args.animations_config.caret_blink_timeout,
            ));

            if args.is_respawn {
                kb.clear_modifiers();
                kb.codes.set(vec![]);
                kb.keys.set(vec![]);

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
pub struct KEYBOARD;
impl KEYBOARD {
    /// Returns a read-only variable that tracks the currently pressed modifier keys.
    pub fn modifiers(&self) -> ReadOnlyArcVar<ModifiersState> {
        KEYBOARD_SV.read().modifiers.read_only()
    }

    /// Returns a read-only variable that tracks the [`ScanCode`] of the keys currently pressed.
    pub fn codes(&self) -> ReadOnlyArcVar<Vec<ScanCode>> {
        KEYBOARD_SV.read().codes.read_only()
    }

    /// Returns a read-only variable that tracks the [`Key`] identifier of the keys currently pressed.
    pub fn keys(&self) -> ReadOnlyArcVar<Vec<Key>> {
        KEYBOARD_SV.read().keys.read_only()
    }

    /// Returns a read-only variable that tracks the operating system key press repeat start delay and repeat speed.
    ///
    /// This delay is roughly the time the user must hold a key pressed to start repeating. When a second key press
    /// happens without any other keyboard event and within twice this value it increments the [`repeat_count`] by the [`KeyboardManager`].
    ///
    /// [`repeat_count`]: KeyInputArgs::repeat_count
    /// [`repeat_speed`]: Self::repeat_speed
    pub fn repeat_config(&self) -> ReadOnlyArcVar<KeyRepeatConfig> {
        KEYBOARD_SV.read().repeat_config.read_only()
    }

    /// Returns a read-only variable that defines the system config for the caret blink speed and timeout.
    ///
    /// The first value defines the blink speed interval, the caret is visible for the duration, then not visible for the duration. The
    /// second value defines the animation total duration, the caret stops animating and sticks to visible after this timeout is reached.
    ///
    /// You can use the [`caret_animation`] method to generate a new animation.
    ///
    /// [`caret_animation`]: Self::caret_animation
    pub fn caret_animation_config(&self) -> ReadOnlyArcVar<(Duration, Duration)> {
        KEYBOARD_SV.read().caret_animation_config.read_only()
    }

    /// Returns a new read-only variable that animates the caret opacity.
    ///
    /// A new animation must be started after each key press. The value is always 1 or 0, no easing is used by default,
    /// it can be added using the [`Var::easing`] method.
    pub fn caret_animation(&self) -> ReadOnlyArcVar<Factor> {
        KEYBOARD_SV.read().caret_animation()
    }
}

app_local! {
    static KEYBOARD_SV: KeyboardService = KeyboardService {
        current_modifiers: FxHashSet::default(),
        modifiers: var(ModifiersState::empty()),
        codes: var(vec![]),
        keys: var(vec![]),
        repeat_config: var_default(),
        caret_animation_config: {
            let cfg = AnimationsConfig::default();
            var((cfg.caret_blink_interval, cfg.caret_blink_timeout))
        },
        last_key_down: None,
    };
}

struct KeyboardService {
    current_modifiers: FxHashSet<Key>,

    modifiers: ArcVar<ModifiersState>,
    codes: ArcVar<Vec<ScanCode>>,
    keys: ArcVar<Vec<Key>>,
    repeat_config: ArcVar<KeyRepeatConfig>,
    caret_animation_config: ArcVar<(Duration, Duration)>,

    last_key_down: Option<(DeviceId, ScanCode, Instant, u32)>,
}
impl KeyboardService {
    fn key_input(&mut self, args: &RawKeyInputArgs, focused: Option<InteractionPath>) {
        let mut repeat = 0;

        // update state and vars
        match args.state {
            KeyState::Pressed => {
                if let Some((d_id, code, time, count)) = &mut self.last_key_down {
                    let max_t = self.repeat_config.get().start_delay * 2;
                    if args.scan_code == *code && args.device_id == *d_id && (args.timestamp - *time) < max_t {
                        *count = (*count).saturating_add(1);
                        repeat = *count;
                    } else {
                        *d_id = args.device_id;
                        *code = args.scan_code;
                        *count = 0;
                    }
                    *time = args.timestamp;
                } else {
                    self.last_key_down = Some((args.device_id, args.scan_code, args.timestamp, 0));
                }

                let scan_code = args.scan_code;
                if !self.codes.with(|c| c.contains(&scan_code)) {
                    self.codes.modify(move |cs| {
                        cs.to_mut().push(scan_code);
                    });
                }

                if let Some(key) = args.key {
                    if !self.keys.with(|c| c.contains(&key)) {
                        self.keys.modify(move |ks| {
                            ks.to_mut().push(key);
                        });
                    }

                    if key.is_modifier() {
                        self.set_modifiers(key, true);
                    }
                }
            }
            KeyState::Released => {
                self.last_key_down = None;

                let key = args.scan_code;
                if self.codes.with(|c| c.contains(&key)) {
                    self.codes.modify(move |cs| {
                        if let Some(i) = cs.as_ref().iter().position(|c| *c == key) {
                            cs.to_mut().swap_remove(i);
                        }
                    });
                }

                if let Some(key) = args.key {
                    if self.keys.with(|c| c.contains(&key)) {
                        self.keys.modify(move |ks| {
                            if let Some(i) = ks.as_ref().iter().position(|k| *k == key) {
                                ks.to_mut().swap_remove(i);
                            }
                        });
                    }

                    if key.is_modifier() {
                        self.set_modifiers(key, false);
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
                    self.current_modifiers(),
                    repeat,
                    target,
                );
                KEY_INPUT_EVENT.notify(args);
            }
        }
    }
    fn set_modifiers(&mut self, key: Key, pressed: bool) {
        let prev_modifiers = self.current_modifiers();

        if pressed {
            self.current_modifiers.insert(key);
        } else {
            self.current_modifiers.remove(&key);
        }

        let new_modifiers = self.current_modifiers();

        if prev_modifiers != new_modifiers {
            self.modifiers.set(new_modifiers);
            MODIFIERS_CHANGED_EVENT.notify(ModifiersChangedArgs::now(prev_modifiers, new_modifiers));
        }
    }

    fn clear_modifiers(&mut self) {
        let prev_modifiers = self.current_modifiers();
        self.current_modifiers.clear();
        let new_modifiers = self.current_modifiers();

        if prev_modifiers != new_modifiers {
            self.modifiers.set(new_modifiers);
            MODIFIERS_CHANGED_EVENT.notify(ModifiersChangedArgs::now(prev_modifiers, new_modifiers));
        }
    }

    fn current_modifiers(&self) -> ModifiersState {
        let mut state = ModifiersState::empty();
        for key in &self.current_modifiers {
            state |= ModifiersState::from_key(*key);
        }
        state
    }

    fn caret_animation(&self) -> ReadOnlyArcVar<Factor> {
        let var = var(0.fct());
        let cfg = self.caret_animation_config.clone();

        let zero = 0.fct();
        let one = 1.fct();

        var.animate(move |anim, vm| {
            let (interval, timeout) = cfg.get();
            if anim.start_time().elapsed() >= timeout {
                if **vm != one {
                    vm.set(one);
                }
                anim.stop();
            } else {
                if **vm == one {
                    vm.set(zero);
                } else {
                    vm.set(one);
                }
                anim.sleep(interval);
            }
        })
        .perm();

        var.read_only()
    }
}

/// Extension trait that adds keyboard simulation methods to [`HeadlessApp`].
pub trait HeadlessAppKeyboardExt {
    /// Notifies keyboard input event.
    ///
    /// Note that the app is not updated so the event is pending after this call.
    fn on_keyboard_input(&mut self, window_id: WindowId, key: Key, state: KeyState);

    /// Does a key-down, key-up and updates.
    fn press_key(&mut self, window_id: WindowId, key: Key);

    /// Does a modifiers changed, key-down, key-up, reset modifiers and updates.
    fn press_modified_key(&mut self, window_id: WindowId, modifiers: ModifiersState, key: Key);
}
impl HeadlessAppKeyboardExt for HeadlessApp {
    fn on_keyboard_input(&mut self, window_id: WindowId, key: Key, state: KeyState) {
        use crate::app::raw_events::*;

        let args = RawKeyInputArgs::now(window_id, DeviceId::virtual_keyboard(), ScanCode(key as _), state, Some(key));
        RAW_KEY_INPUT_EVENT.notify(args);
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
            let modifiers = modifiers.keys();
            for &key in &modifiers {
                self.on_keyboard_input(window_id, key, KeyState::Pressed);
            }

            // pressed the modifiers.
            let _ = self.update(false);

            self.on_keyboard_input(window_id, key, KeyState::Pressed);
            self.on_keyboard_input(window_id, key, KeyState::Released);

            // pressed the key.
            let _ = self.update(false);

            for key in modifiers {
                self.on_keyboard_input(window_id, key, KeyState::Released);
            }

            // released the modifiers.
            let _ = self.update(false);
        }
    }
}

bitflags! {
    /// Represents the current state of the keyboard modifiers.
    ///
    /// Each flag represents a modifier and is set if this modifier is active.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct ModifiersState: u8 {
        /// The left "shift" key.
        const L_SHIFT = 0b0000_0001;
        /// The right "shift" key.
        const R_SHIFT = 0b0000_0010;
        /// Any "shift" key.
        const SHIFT   = 0b0000_0011;

        /// The left "control" key.
        const L_CTRL = 0b0000_0100;
        /// The right "control" key.
        const R_CTRL = 0b0000_1000;
        /// Any "control" key.
        const CTRL   = 0b0000_1100;

        /// The left "alt" key.
        const L_ALT = 0b0001_0000;
        /// The right "alt" key.
        const R_ALT = 0b0010_0000;
        /// Any "alt" key.
        const ALT   = 0b0011_0000;

        /// The left "logo" key.
        const L_LOGO = 0b0100_0000;
        /// The right "logo" key.
        const R_LOGO = 0b1000_0000;
        /// Any "logo" key.
        ///
        /// This is the "windows" key on PC and "command" key on Mac.
        const LOGO   = 0b1100_0000;
    }
}
impl ModifiersState {
    /// Returns `true` if any shift key is pressed.
    pub fn has_shift(self) -> bool {
        self.intersects(Self::SHIFT)
    }
    /// Returns `true` if any control key is pressed.
    pub fn has_ctrl(self) -> bool {
        self.intersects(Self::CTRL)
    }
    /// Returns `true` if any alt key is pressed.
    pub fn has_alt(self) -> bool {
        self.intersects(Self::ALT)
    }
    /// Returns `true` if any logo key is pressed.
    pub fn has_logo(self) -> bool {
        self.intersects(Self::LOGO)
    }

    /// Returns `true` if only any flag in `part` is pressed.
    pub fn is_only(self, part: ModifiersState) -> bool {
        !self.is_empty() && (self - part).is_empty()
    }

    /// Returns `true` if only any shift key is pressed.
    pub fn is_only_shift(self) -> bool {
        self.is_only(ModifiersState::SHIFT)
    }
    /// Returns `true` if only any control key is pressed.
    pub fn is_only_ctrl(self) -> bool {
        self.is_only(ModifiersState::CTRL)
    }
    /// Returns `true` if only any alt key is pressed.
    pub fn is_only_alt(self) -> bool {
        self.is_only(ModifiersState::ALT)
    }
    /// Returns `true` if only any logo key is pressed.
    pub fn is_only_logo(self) -> bool {
        self.is_only(ModifiersState::LOGO)
    }

    /// Removes `part` and returns if it was removed.
    pub fn take(&mut self, part: ModifiersState) -> bool {
        let r = self.intersects(part);
        if r {
            self.remove(part);
        }
        r
    }

    /// Removes `SHIFT` and returns if it was removed.
    pub fn take_shift(&mut self) -> bool {
        self.take(ModifiersState::SHIFT)
    }

    /// Removes `CTRL` and returns if it was removed.
    pub fn take_ctrl(&mut self) -> bool {
        self.take(ModifiersState::CTRL)
    }

    /// Removes `ALT` and returns if it was removed.
    pub fn take_alt(&mut self) -> bool {
        self.take(ModifiersState::ALT)
    }

    /// Removes `LOGO` and returns if it was removed.
    pub fn take_logo(&mut self) -> bool {
        self.take(ModifiersState::LOGO)
    }

    /// Returns modifiers that set both left and right flags if any side is set in `self`.
    pub fn ambit(self) -> Self {
        let mut r = Self::empty();
        if self.has_alt() {
            r |= Self::ALT;
        }
        if self.has_ctrl() {
            r |= Self::CTRL;
        }
        if self.has_shift() {
            r |= Self::SHIFT;
        }
        if self.has_logo() {
            r |= Self::LOGO;
        }
        r
    }

    /// Returns only the alt flags in `self`.
    pub fn into_alt(self) -> Self {
        self & Self::ALT
    }

    /// Returns only the control flags in `self`.
    pub fn into_ctrl(self) -> Self {
        self & Self::CTRL
    }

    /// Returns only the shift flags in `self`.
    pub fn into_shift(self) -> Self {
        self & Self::SHIFT
    }

    /// Returns only the logo flags in `self`.
    pub fn into_logo(self) -> Self {
        self & Self::LOGO
    }

    /// Modifier from `key`, returns empty if the key is not a modifier.
    pub fn from_key(key: Key) -> ModifiersState {
        match key {
            Key::LAlt => Self::L_ALT,
            Key::RAlt => Self::R_ALT,
            Key::LCtrl => Self::L_CTRL,
            Key::RCtrl => Self::R_CTRL,
            Key::LShift => Self::L_SHIFT,
            Key::RShift => Self::R_SHIFT,
            Key::LLogo => Self::L_LOGO,
            Key::RLogo => Self::R_LOGO,
            _ => Self::empty(),
        }
    }

    /// All keys that when pressed form the modifiers state.
    ///
    /// In case of multiple keys the order is `LOGO`, `CTRL`, `SHIFT`, `ALT`.
    ///
    /// In case both left and right keys are flagged for a modifier, the left key is used.
    pub fn keys(self) -> Vec<Key> {
        let mut r = vec![];

        if self.contains(Self::L_LOGO) {
            r.push(Key::LLogo);
        } else if self.contains(Self::R_LOGO) {
            r.push(Key::RLogo);
        }

        if self.contains(Self::L_CTRL) {
            r.push(Key::LCtrl);
        } else if self.contains(Self::R_CTRL) {
            r.push(Key::RCtrl);
        }

        if self.contains(Self::L_SHIFT) {
            r.push(Key::LShift);
        } else if self.contains(Self::R_SHIFT) {
            r.push(Key::RShift);
        }

        if self.contains(Self::L_ALT) {
            r.push(Key::LAlt);
        } else if self.contains(Self::R_ALT) {
            r.push(Key::RAlt);
        }

        r
    }
}

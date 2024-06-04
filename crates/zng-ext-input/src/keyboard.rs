//! Keyboard manager.

use std::{collections::HashSet, time::Duration};

use zng_app::{
    event::{event, event_args},
    shortcut::ModifiersState,
    update::EventUpdate,
    view_process::{
        raw_device_events::DeviceId,
        raw_events::{
            RawKeyInputArgs, RAW_ANIMATIONS_CONFIG_CHANGED_EVENT, RAW_KEY_INPUT_EVENT, RAW_KEY_REPEAT_CONFIG_CHANGED_EVENT,
            RAW_WINDOW_FOCUS_EVENT,
        },
        VIEW_PROCESS_INITED_EVENT,
    },
    widget::{info::InteractionPath, WidgetId},
    window::WindowId,
    AppExtension, DInstant, HeadlessApp,
};
use zng_app_context::app_local;
use zng_clone_move::clmv;
use zng_layout::unit::{Factor, FactorUnits};
use zng_txt::Txt;
use zng_var::{types::ArcCowVar, var, var_default, ArcVar, ReadOnlyArcVar, Var};
use zng_view_api::config::AnimationsConfig;
pub use zng_view_api::{
    config::KeyRepeatConfig,
    keyboard::{Key, KeyCode, KeyLocation, KeyState, NativeKeyCode},
};

use crate::focus::FOCUS;

event_args! {
    /// Arguments for [`KEY_INPUT_EVENT`].
    pub struct KeyInputArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Physical key.
        pub key_code: KeyCode,

        /// The location of the key on the keyboard.
        pub key_location: KeyLocation,

        /// If the key was pressed or released.
        pub state: KeyState,

        /// Semantic key.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('a')` in QWERTY keyboards, the modifiers are not applied.
        pub key: Key,
        /// Semantic key modified by the current active modifiers.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('A')` in QWERTY keyboards, the modifiers are applied.
        pub key_modified: Key,

        /// Text typed.
        ///
        /// This is only set during [`KeyState::Pressed`] of a key that generates text.
        ///
        /// This is usually the `key_modified` char, but is also `'\r'` for `Key::Enter`. On Windows when a dead key was
        /// pressed earlier but cannot be combined with the character from this key press, the produced text
        /// will consist of two characters: the dead-key-character followed by the character resulting from this key press.
        pub text: Txt,

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
            list.insert_wgt(&self.target)
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

    /// Gets the modified key for Numpad keys and the unmodified key for the rest.
    pub fn shortcut_key(&self) -> &Key {
        if matches!(self.key_location, KeyLocation::Numpad) {
            &self.key_modified
        } else {
            &self.key
        }
    }
}

/// Text input methods.
///
/// The [`text`] field contains the raw text associated with the key-press by the operating system,
/// these methods normalize and filter this text.
///
/// [`text`]: KeyInputArgs::text
impl KeyInputArgs {
    /// Returns `true` if the character is the backspace and `CTRL` is not pressed.
    pub fn is_backspace(&self) -> bool {
        !self.modifiers.contains(ModifiersState::CTRL) && self.text.contains('\u{8}')
    }

    /// Returns `true` if the character is delete and `CTRL` is not pressed.
    pub fn is_delete(&self) -> bool {
        !self.modifiers.contains(ModifiersState::CTRL) && self.text.contains('\u{7F}')
    }

    /// Returns `true` if the character is a tab space and `CTRL` is not pressed.
    pub fn is_tab(&self) -> bool {
        !self.modifiers.contains(ModifiersState::CTRL) && self.text.chars().any(|c| "\t\u{B}\u{1F}".contains(c))
    }

    /// Returns `true` if the character is a line-break and `CTRL` is not pressed.
    pub fn is_line_break(&self) -> bool {
        !self.modifiers.contains(ModifiersState::CTRL) && self.text.chars().any(|c| "\r\n\u{85}".contains(c))
    }

    /// Gets the characters to insert in a typed text.
    ///
    /// Replaces all [`is_tab`] with `\t` and all [`is_line_break`] with `\n`.
    /// Returns `""` if there is no text or it contains ASCII control characters or `CTRL` is pressed.
    ///
    /// [`is_tab`]: Self::is_tab
    /// [`is_line_break`]: Self::is_line_break
    pub fn insert_str(&self) -> &str {
        if self.modifiers.contains(ModifiersState::CTRL) {
            // ignore legacy ASCII control combinators like `ctrl+i` generated `\t`.
            ""
        } else if self.is_tab() {
            "\t"
        } else if self.is_line_break() {
            "\n"
        } else if self.text.chars().any(|c| c.is_ascii_control()) {
            ""
        } else {
            &self.text
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
///
/// # Services
///
/// Services this extension provides.
///
/// * [`KEYBOARD`]
///
/// # Dependencies
///
/// This extension requires the [`FOCUS`] and [`WINDOWS`] services before the first raw key input event. It does not
/// require anything for initialization.
///
/// [extension]: AppExtension
/// [default app]: zng_app::APP::default
/// [`FOCUS`]: crate::focus::FOCUS
/// [`WINDOWS`]: zng_ext_window::WINDOWS
#[derive(Default)]
pub struct KeyboardManager {}
impl AppExtension for KeyboardManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = RAW_KEY_INPUT_EVENT.on(update) {
            let focused = FOCUS.focused().get();
            KEYBOARD_SV.write().key_input(args, focused);
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

    /// Returns a read-only variable that tracks the [`KeyCode`] of the keys currently pressed.
    pub fn codes(&self) -> ReadOnlyArcVar<Vec<KeyCode>> {
        KEYBOARD_SV.read().codes.read_only()
    }

    /// Returns a read-only variable that tracks the [`Key`] identifier of the keys currently pressed.
    pub fn keys(&self) -> ReadOnlyArcVar<Vec<Key>> {
        KEYBOARD_SV.read().keys.read_only()
    }

    /// Returns a variable that defines key press repeat start delay and repeat speed on the app.
    ///
    /// This delay is roughly the time the user must hold a key pressed to start repeating. When a second key press
    /// happens without any other keyboard event and within twice this value it increments the [`repeat_count`] by the [`KeyboardManager`].
    ///
    /// The value is the same as [`sys_repeat_config`], if set the variable disconnects from system config.
    ///
    /// [`sys_repeat_config`]: KEYBOARD::sys_repeat_config
    /// [`repeat_count`]: KeyInputArgs::repeat_count
    /// [`repeat_speed`]: Self::repeat_speed
    pub fn repeat_config(&self) -> ArcCowVar<KeyRepeatConfig, ArcVar<KeyRepeatConfig>> {
        KEYBOARD_SV.read().repeat_config.clone()
    }

    /// Returns a read-only variable that tracks the operating system key press repeat start delay and repeat speed.
    ///
    /// The variable updates every time the system config changes and on view-process (re)init.
    pub fn sys_repeat_config(&self) -> ReadOnlyArcVar<KeyRepeatConfig> {
        KEYBOARD_SV.read().sys_repeat_config.read_only()
    }

    /// Returns a variable that defines the system config for the caret blink speed and timeout for the app.
    ///
    /// The first value defines the blink speed interval, the caret is visible for the duration, then not visible for the duration. The
    /// second value defines the animation total duration, the caret stops animating and sticks to visible after this timeout is reached.
    ///
    /// You can use the [`caret_animation`] method to generate a new animation.
    ///
    /// The value is the same as [`sys_repeat_config`], if set the variable disconnects from system config.
    ///
    /// [`caret_animation`]: Self::caret_animation
    /// [`sys_repeat_config`]: Self::sys_repeat_config
    pub fn caret_animation_config(&self) -> ArcCowVar<(Duration, Duration), ArcVar<(Duration, Duration)>> {
        KEYBOARD_SV.read().caret_animation_config.clone()
    }

    /// Returns a read-only variable that tracks the operating system caret blink speed and timeout.
    ///
    /// The variable updates every time the system config changes and on view-process (re)init.
    pub fn sys_caret_animation_config(&self) -> ReadOnlyArcVar<(Duration, Duration)> {
        KEYBOARD_SV.read().sys_caret_animation_config.read_only()
    }

    /// Returns a new read-only variable that animates the caret opacity.
    ///
    /// A new animation must be started after each key press. The value is always 1 or 0, no easing is used by default,
    /// it can be added using the [`Var::easing`] method.
    ///
    /// [`Var::easing`]: zng_var::Var::easing
    pub fn caret_animation(&self) -> ReadOnlyArcVar<Factor> {
        KEYBOARD_SV.read().caret_animation()
    }
}

app_local! {
    static KEYBOARD_SV: KeyboardService = {
        let sys_repeat_config = var_default();
        let cfg = AnimationsConfig::default();
        let sys_caret_animation_config = var((cfg.caret_blink_interval, cfg.caret_blink_timeout));
        KeyboardService {
            current_modifiers: HashSet::default(),
            modifiers: var(ModifiersState::empty()),
            codes: var(vec![]),
            keys: var(vec![]),
            repeat_config: sys_repeat_config.cow(),
            sys_repeat_config,
            caret_animation_config: sys_caret_animation_config.cow(),
            sys_caret_animation_config,
            last_key_down: None,
        }
    };
}

struct KeyboardService {
    current_modifiers: HashSet<Key>,

    modifiers: ArcVar<ModifiersState>,
    codes: ArcVar<Vec<KeyCode>>,
    keys: ArcVar<Vec<Key>>,
    repeat_config: ArcCowVar<KeyRepeatConfig, ArcVar<KeyRepeatConfig>>,
    sys_repeat_config: ArcVar<KeyRepeatConfig>,
    caret_animation_config: ArcCowVar<(Duration, Duration), ArcVar<(Duration, Duration)>>,
    sys_caret_animation_config: ArcVar<(Duration, Duration)>,

    last_key_down: Option<(DeviceId, KeyCode, DInstant, u32)>,
}
impl KeyboardService {
    fn key_input(&mut self, args: &RawKeyInputArgs, focused: Option<InteractionPath>) {
        let mut repeat = 0;

        // update state and vars
        match args.state {
            KeyState::Pressed => {
                if let Some((d_id, code, time, count)) = &mut self.last_key_down {
                    let max_t = self.repeat_config.get().start_delay * 2;
                    if args.key_code == *code && args.device_id == *d_id && (args.timestamp - *time) < max_t {
                        *count = (*count).saturating_add(1);
                        repeat = *count;
                    } else {
                        *d_id = args.device_id;
                        *code = args.key_code;
                        *count = 0;
                    }
                    *time = args.timestamp;
                } else {
                    self.last_key_down = Some((args.device_id, args.key_code, args.timestamp, 0));
                }

                let key_code = args.key_code;
                if !self.codes.with(|c| c.contains(&key_code)) {
                    self.codes.modify(move |cs| {
                        cs.to_mut().push(key_code);
                    });
                }

                let key = &args.key;
                if !matches!(&key, Key::Unidentified) {
                    if !self.keys.with(|c| c.contains(key)) {
                        self.keys.modify(clmv!(key, |ks| {
                            ks.to_mut().push(key);
                        }));
                    }

                    if key.is_modifier() {
                        self.set_modifiers(key.clone(), true);
                    }
                }
            }
            KeyState::Released => {
                self.last_key_down = None;

                let key = args.key_code;
                if self.codes.with(|c| c.contains(&key)) {
                    self.codes.modify(move |cs| {
                        if let Some(i) = cs.as_ref().iter().position(|c| *c == key) {
                            cs.to_mut().swap_remove(i);
                        }
                    });
                }

                let key = &args.key;
                if !matches!(&key, Key::Unidentified) {
                    if self.keys.with(|c| c.contains(key)) {
                        self.keys.modify(clmv!(key, |ks| {
                            if let Some(i) = ks.as_ref().iter().position(|k| k == &key) {
                                ks.to_mut().swap_remove(i);
                            }
                        }));
                    }

                    if key.is_modifier() {
                        self.set_modifiers(key.clone(), false);
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
                    args.key_code,
                    args.key_location,
                    args.state,
                    args.key.clone(),
                    args.key_modified.clone(),
                    args.text.clone(),
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
            state |= ModifiersState::from_key(key.clone());
        }
        state
    }

    fn caret_animation(&self) -> ReadOnlyArcVar<Factor> {
        let var = var(1.fct());
        let cfg = self.caret_animation_config.clone();

        let zero = 0.fct();
        let one = 1.fct();
        let mut init = true;

        var.animate(move |anim, vm| {
            let (interval, timeout) = cfg.get();
            if anim.start_time().elapsed() >= timeout {
                if **vm != one {
                    vm.set(one);
                }
                anim.stop();
            } else {
                if **vm == one {
                    if !std::mem::take(&mut init) {
                        vm.set(zero);
                    }
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
///
/// [`HeadlessApp`]: zng_app::HeadlessApp
pub trait HeadlessAppKeyboardExt {
    /// Notifies keyboard input event.
    ///
    /// Note that the app is not updated so the event is pending after this call.
    fn on_keyboard_input(&mut self, window_id: WindowId, code: KeyCode, location: KeyLocation, key: Key, state: KeyState);

    /// Does a key-down, key-up and updates.
    fn press_key(&mut self, window_id: WindowId, code: KeyCode, location: KeyLocation, key: Key);

    /// Does a modifiers changed, key-down, key-up, reset modifiers and updates.
    fn press_modified_key(&mut self, window_id: WindowId, modifiers: ModifiersState, code: KeyCode, location: KeyLocation, key: Key);
}
impl HeadlessAppKeyboardExt for HeadlessApp {
    fn on_keyboard_input(&mut self, window_id: WindowId, code: KeyCode, location: KeyLocation, key: Key, state: KeyState) {
        use zng_app::view_process::raw_events::*;

        let args = RawKeyInputArgs::now(window_id, DeviceId::virtual_keyboard(), code, location, state, key.clone(), key, "");
        RAW_KEY_INPUT_EVENT.notify(args);
    }

    fn press_key(&mut self, window_id: WindowId, code: KeyCode, location: KeyLocation, key: Key) {
        self.on_keyboard_input(window_id, code, location, key.clone(), KeyState::Pressed);
        self.on_keyboard_input(window_id, code, location, key, KeyState::Released);
        let _ = self.update(false);
    }

    fn press_modified_key(&mut self, window_id: WindowId, modifiers: ModifiersState, code: KeyCode, location: KeyLocation, key: Key) {
        if modifiers.is_empty() {
            self.press_key(window_id, code, location, key);
        } else {
            let modifiers = modifiers.keys();
            for key in &modifiers {
                self.on_keyboard_input(window_id, code, location, key.clone(), KeyState::Pressed);
            }

            // pressed the modifiers.
            let _ = self.update(false);

            self.on_keyboard_input(window_id, code, location, key.clone(), KeyState::Pressed);
            self.on_keyboard_input(window_id, code, location, key.clone(), KeyState::Released);

            // pressed the key.
            let _ = self.update(false);

            for key in modifiers {
                self.on_keyboard_input(window_id, code, location, key, KeyState::Released);
            }

            // released the modifiers.
            let _ = self.update(false);
        }
    }
}

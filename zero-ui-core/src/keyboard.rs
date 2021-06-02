//! Keyboard manager.
//!
//! The [`KeyboardManager`] struct is an [app extension](crate::app::AppExtension). It
//! is included in the [default app](crate::app::App::default) and provides the [`Keyboard`] service
//! and keyboard input events.

use crate::app::*;
use crate::context::*;
use crate::event::*;
use crate::focus::Focus;
use crate::render::WidgetPath;
use crate::service::*;
use crate::window::{WindowEvent, WindowId, Windows};

pub use glutin::event::{KeyboardInput, ModifiersState, ScanCode};

event_args! {
    /// Keyboard event args.
    pub struct KeyInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        ///
        /// Is `None` if the event was generated programmatically.
        pub device_id: Option<DeviceId>,

        /// Raw code of key.
        pub scan_code: ScanCode,

        /// If the key was pressed or released.
        pub state: ElementState,

        /// Symbolic name of [`scan_code`](KeyInputArgs::scan_code).
        pub key: Option<Key>,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// If the key-down event was generated by holding the key pressed.
        pub repeat: bool,

        /// The focused element at the time of the key input.
        pub target: WidgetPath,

        ..

        /// If the widget is focused or contains the focused widget.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }

    /// Character received event args.
    pub struct CharInputArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// The character.
        pub character: char,

        /// The focused element at the time of the key input.
        pub target: WidgetPath,

        ..

        /// If the widget is focused or contains the focused widget.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }

    /// Keyboard modifiers changed event args.
    pub struct ModifiersChangedArgs {
        /// Previous modifiers state.
        pub prev_modifiers: ModifiersState,

        /// Current modifiers state.
        pub modifiers: ModifiersState,

        /// The focused element at the time of the update.
        pub target: WidgetPath,

        ..

        /// If the widget is focused or contains the focused widget.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }
}

event! {
    /// Key pressed or released event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub KeyInputEvent: KeyInputArgs;

    /// Key pressed or repeat event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub KeyDownEvent: KeyInputArgs;

    /// Key released event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`KeyboardManager`] extension.
    pub KeyUpEvent: KeyInputArgs;

    /// Modifiers state changed event.
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

/// Application extension that provides keyboard events.
///
/// # Events
///
/// Events this extension provides.
///
/// * [KeyInputEvent]
/// * [KeyDownEvent]
/// * [KeyUpEvent]
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
/// This extension is included in the [default app](crate::app::App::default), events provided by it
/// are required by multiple other extensions.
///
/// # Dependencies
///
/// This extension requires the [`Focus`] and [`Windows`] services before the first window event. It does not
/// require anything for initialization.
#[derive(Default)]
pub struct KeyboardManager;
impl KeyboardManager {
    fn target(window_id: WindowId, services: &mut Services) -> Option<WidgetPath> {
        if let Some(focused) = services.get::<Focus>().and_then(|f| f.focused().cloned()) {
            Some(focused)
        } else {
            services
                .get::<Windows>()
                .and_then(|f| f.window(window_id).ok())
                .map(|w| w.frame_info().root().path())
        }
    }
}
impl AppExtension for KeyboardManager {
    fn init(&mut self, r: &mut AppInitContext) {
        r.services.register(Keyboard::default());
    }

    fn window_event(&mut self, ctx: &mut AppContext, window_id: WindowId, event: &WindowEvent) {
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
                if let Some(target) = Self::target(window_id, ctx.services) {
                    ctx.services
                        .req::<Keyboard>()
                        .device_input(device_id, scancode, key.map(Into::into), state, target, ctx.events);
                }
            }

            WindowEvent::ModifiersChanged(m) => {
                if let Some(target) = Self::target(window_id, ctx.services) {
                    ctx.services.req::<Keyboard>().set_modifiers(m, target, ctx.events);
                }
            }

            WindowEvent::ReceivedCharacter(c) => {
                if let Some(target) = Self::target(window_id, ctx.services) {
                    ctx.services.req::<Keyboard>().char_input(c, target, ctx.events);
                }
            }

            _ => {}
        }
    }
}

/// Keyboard service.
///
/// # Provider
///
/// This service is provided by the [`KeyboardManager`] extension.
#[derive(Service, Default)]
pub struct Keyboard {
    modifiers: ModifiersState,
    last_key_down: Option<(Option<DeviceId>, ScanCode)>,
}
impl Keyboard {
    /// Process a software keyboard input.
    #[inline]
    pub fn input(&mut self, key: Key, state: ElementState, target: WidgetPath, events: &Events) {
        self.do_input(None, key as ScanCode, Some(key), state, target, events);
    }

    /// Process a external keyboard input.
    #[inline]
    pub fn device_input(
        &mut self,
        device_id: DeviceId,
        scan_code: ScanCode,
        key: Option<Key>,
        state: ElementState,
        target: WidgetPath,
        events: &Events,
    ) {
        self.do_input(Some(device_id), scan_code, key, state, target, events);
    }

    /// Set the keyboard modifiers state.
    pub fn set_modifiers(&mut self, modifiers: ModifiersState, target: WidgetPath, events: &Events) {
        if self.modifiers != modifiers {
            let prev_modifiers = std::mem::replace(&mut self.modifiers, modifiers);
            let args = ModifiersChangedArgs::now(prev_modifiers, modifiers, target);
            ModifiersChangedEvent::notify(events, args);
        }
    }

    /// Character input.
    pub fn char_input(&mut self, character: char, target: WidgetPath, events: &Events) {
        let args = CharInputArgs::now(target.window_id(), character, target);
        CharInputEvent::notify(events, args);
    }

    /// Current modifiers pressed.
    #[inline]
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    fn do_input(
        &mut self,
        device_id: Option<DeviceId>,
        scan_code: ScanCode,
        key: Option<Key>,
        state: ElementState,
        target: WidgetPath,
        events: &Events,
    ) {
        let mut repeat = false;
        if state == ElementState::Pressed {
            repeat = self.last_key_down == Some((device_id, scan_code));
            if !repeat {
                self.last_key_down = Some((device_id, scan_code));
            }
        } else {
            self.last_key_down = None;
        }

        let args = KeyInputArgs::now(target.window_id(), device_id, scan_code, state, key, self.modifiers, repeat, target);

        KeyInputEvent::notify(events, args.clone());

        match args.state {
            ElementState::Pressed => KeyDownEvent::notify(events, args),
            ElementState::Released => KeyUpEvent::notify(events, args),
        }
    }
}

// Symbolic name for a keyboard key.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
#[allow(missing_docs)] // some of these are self-explanatory.
pub enum Key {
    /// The '1' key over the letters.
    Key1,
    /// The '2' key over the letters.
    Key2,
    /// The '3' key over the letters.
    Key3,
    /// The '4' key over the letters.
    Key4,
    /// The '5' key over the letters.
    Key5,
    /// The '6' key over the letters.
    Key6,
    /// The '7' key over the letters.
    Key7,
    /// The '8' key over the letters.
    Key8,
    /// The '9' key over the letters.
    Key9,
    /// The '0' key over the 'O' and 'P' keys.
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// The Escape key, next to F1.
    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    /// Print Screen/SysRq.
    PrtScr,
    ScrollLock,
    /// Pause/Break key, next to Scroll lock.
    Pause,

    /// `Insert`, next to Backspace.
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    /// The Backspace key, right over Enter.
    Backspace,
    /// The Return key.
    Enter,
    /// The space bar.
    Space,

    /// The "Compose" key on Linux.
    Compose,

    Caret,

    NumLock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,

    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
    Calculator,
    CapsLock,
    Colon,
    Comma,
    Convert,
    Equals,
    Grave,
    Kana,
    Kanji,
    /// Left Alt
    LAlt,
    LBracket,
    /// Left Control
    LCtrl,
    /// Left Shift
    LShift,
    LLogo,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Mute,
    MyComputer,
    // also called "Next"
    NavigateForward,
    // also called "Prior"
    NavigateBackward,
    NextTrack,
    NoConvert,
    Oem102,
    /// The '.' key, also called a dot.
    Period,
    PlayPause,
    Plus,
    Power,
    PrevTrack,
    /// Right Alt.
    RAlt,
    RBracket,
    RControl,
    RShift,
    RLogo,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
}
impl Key {
    /// If the key is a modifier key.
    pub fn is_modifier(self) -> bool {
        matches!(
            self,
            Key::LAlt | Key::LCtrl | Key::LShift | Key::LLogo | Key::RAlt | Key::RControl | Key::RShift | Key::RLogo
        )
    }

    /// If the key is left alt or right alt.
    pub fn is_alt(self) -> bool {
        matches!(self, Key::LAlt | Key::RAlt)
    }

    /// If the key is left ctrl or right ctrl.
    pub fn is_ctrl(self) -> bool {
        matches!(self, Key::LCtrl | Key::RControl)
    }

    /// If the key is left shift or right shift.
    pub fn is_shift(self) -> bool {
        matches!(self, Key::LShift | Key::RShift)
    }

    /// If the key is left logo or right logo.
    pub fn is_logo(self) -> bool {
        matches!(self, Key::LLogo | Key::RLogo)
    }

    /// If the key is a numpad key, includes numlock.
    pub fn is_numpad(self) -> bool {
        let key = self as u32;
        key >= Key::NumLock as u32 && key <= Key::NumpadSubtract as u32
    }
}
use glutin::event::VirtualKeyCode as VKey;
impl From<VKey> for Key {
    fn from(v_key: VKey) -> Self {
        #[cfg(debug_assertions)]
        match v_key {
            VKey::Key1 => Key::Key1,
            VKey::Key2 => Key::Key2,
            VKey::Key3 => Key::Key3,
            VKey::Key4 => Key::Key4,
            VKey::Key5 => Key::Key5,
            VKey::Key6 => Key::Key6,
            VKey::Key7 => Key::Key7,
            VKey::Key8 => Key::Key8,
            VKey::Key9 => Key::Key9,
            VKey::Key0 => Key::Key0,
            VKey::A => Key::A,
            VKey::B => Key::B,
            VKey::C => Key::C,
            VKey::D => Key::D,
            VKey::E => Key::E,
            VKey::F => Key::F,
            VKey::G => Key::G,
            VKey::H => Key::H,
            VKey::I => Key::I,
            VKey::J => Key::J,
            VKey::K => Key::K,
            VKey::L => Key::L,
            VKey::M => Key::M,
            VKey::N => Key::N,
            VKey::O => Key::O,
            VKey::P => Key::P,
            VKey::Q => Key::Q,
            VKey::R => Key::R,
            VKey::S => Key::S,
            VKey::T => Key::T,
            VKey::U => Key::U,
            VKey::V => Key::V,
            VKey::W => Key::W,
            VKey::X => Key::X,
            VKey::Y => Key::Y,
            VKey::Z => Key::Z,
            VKey::Escape => Key::Escape,
            VKey::F1 => Key::F1,
            VKey::F2 => Key::F2,
            VKey::F3 => Key::F3,
            VKey::F4 => Key::F4,
            VKey::F5 => Key::F5,
            VKey::F6 => Key::F6,
            VKey::F7 => Key::F7,
            VKey::F8 => Key::F8,
            VKey::F9 => Key::F9,
            VKey::F10 => Key::F10,
            VKey::F11 => Key::F11,
            VKey::F12 => Key::F12,
            VKey::F13 => Key::F13,
            VKey::F14 => Key::F14,
            VKey::F15 => Key::F15,
            VKey::F16 => Key::F16,
            VKey::F17 => Key::F17,
            VKey::F18 => Key::F18,
            VKey::F19 => Key::F19,
            VKey::F20 => Key::F20,
            VKey::F21 => Key::F21,
            VKey::F22 => Key::F22,
            VKey::F23 => Key::F23,
            VKey::F24 => Key::F24,
            VKey::Snapshot => Key::PrtScr,
            VKey::Scroll => Key::ScrollLock,
            VKey::Pause => Key::Pause,
            VKey::Insert => Key::Insert,
            VKey::Home => Key::Home,
            VKey::Delete => Key::Delete,
            VKey::End => Key::End,
            VKey::PageDown => Key::PageDown,
            VKey::PageUp => Key::PageUp,
            VKey::Left => Key::Left,
            VKey::Up => Key::Up,
            VKey::Right => Key::Right,
            VKey::Down => Key::Down,
            VKey::Back => Key::Backspace,
            VKey::Return => Key::Enter,
            VKey::Space => Key::Space,
            VKey::Compose => Key::Compose,
            VKey::Caret => Key::Caret,
            VKey::Numlock => Key::NumLock,
            VKey::Numpad0 => Key::Numpad0,
            VKey::Numpad1 => Key::Numpad1,
            VKey::Numpad2 => Key::Numpad2,
            VKey::Numpad3 => Key::Numpad3,
            VKey::Numpad4 => Key::Numpad4,
            VKey::Numpad5 => Key::Numpad5,
            VKey::Numpad6 => Key::Numpad6,
            VKey::Numpad7 => Key::Numpad7,
            VKey::Numpad8 => Key::Numpad8,
            VKey::Numpad9 => Key::Numpad9,
            VKey::NumpadAdd => Key::NumpadAdd,
            VKey::NumpadDivide => Key::NumpadDivide,
            VKey::NumpadDecimal => Key::NumpadDecimal,
            VKey::NumpadComma => Key::NumpadComma,
            VKey::NumpadEnter => Key::NumpadEnter,
            VKey::NumpadEquals => Key::NumpadEquals,
            VKey::NumpadMultiply => Key::NumpadMultiply,
            VKey::NumpadSubtract => Key::NumpadSubtract,
            VKey::AbntC1 => Key::AbntC1,
            VKey::AbntC2 => Key::AbntC2,
            VKey::Apostrophe => Key::Apostrophe,
            VKey::Apps => Key::Apps,
            VKey::Asterisk => Key::Asterisk,
            VKey::At => Key::At,
            VKey::Ax => Key::Ax,
            VKey::Backslash => Key::Backslash,
            VKey::Calculator => Key::Calculator,
            VKey::Capital => Key::CapsLock,
            VKey::Colon => Key::Colon,
            VKey::Comma => Key::Comma,
            VKey::Convert => Key::Convert,
            VKey::Equals => Key::Equals,
            VKey::Grave => Key::Grave,
            VKey::Kana => Key::Kana,
            VKey::Kanji => Key::Kanji,
            VKey::LAlt => Key::LAlt,
            VKey::LBracket => Key::LBracket,
            VKey::LControl => Key::LCtrl,
            VKey::LShift => Key::LShift,
            VKey::LWin => Key::LLogo,
            VKey::Mail => Key::Mail,
            VKey::MediaSelect => Key::MediaSelect,
            VKey::MediaStop => Key::MediaStop,
            VKey::Minus => Key::Minus,
            VKey::Mute => Key::Mute,
            VKey::MyComputer => Key::MyComputer,
            VKey::NavigateForward => Key::NavigateForward,
            VKey::NavigateBackward => Key::NavigateBackward,
            VKey::NextTrack => Key::NextTrack,
            VKey::NoConvert => Key::NoConvert,
            VKey::OEM102 => Key::Oem102,
            VKey::Period => Key::Period,
            VKey::PlayPause => Key::PlayPause,
            VKey::Plus => Key::Plus,
            VKey::Power => Key::Power,
            VKey::PrevTrack => Key::PrevTrack,
            VKey::RAlt => Key::RAlt,
            VKey::RBracket => Key::RBracket,
            VKey::RControl => Key::RControl,
            VKey::RShift => Key::RShift,
            VKey::RWin => Key::RLogo,
            VKey::Semicolon => Key::Semicolon,
            VKey::Slash => Key::Slash,
            VKey::Sleep => Key::Sleep,
            VKey::Stop => Key::Stop,
            VKey::Sysrq => Key::Sysrq,
            VKey::Tab => Key::Tab,
            VKey::Underline => Key::Underline,
            VKey::Unlabeled => Key::Unlabeled,
            VKey::VolumeDown => Key::VolumeDown,
            VKey::VolumeUp => Key::VolumeUp,
            VKey::Wake => Key::Wake,
            VKey::WebBack => Key::WebBack,
            VKey::WebFavorites => Key::WebFavorites,
            VKey::WebForward => Key::WebForward,
            VKey::WebHome => Key::WebHome,
            VKey::WebRefresh => Key::WebRefresh,
            VKey::WebSearch => Key::WebSearch,
            VKey::WebStop => Key::WebStop,
            VKey::Yen => Key::Yen,
            VKey::Copy => Key::Copy,
            VKey::Paste => Key::Paste,
            VKey::Cut => Key::Cut,
        }
        #[cfg(not(debug_assertions))]
        unsafe {
            std::mem::transmute(v_key)
        }
    }
}
impl From<Key> for VKey {
    fn from(key: Key) -> Self {
        // SAFETY: This is safe because we use the same repr(u32) and use a match to convert
        // from VKey in debug builds to ensure the types are synced if winit changes their enum.
        unsafe { std::mem::transmute(key) }
    }
}

/// Extension trait that adds keyboard simulation methods to [`HeadlessApp`].
pub trait HeadlessAppKeyboardExt {
    /// Does a keyboard input event.
    fn on_keyboard_input(&mut self, window_id: WindowId, key: Key, state: ElementState);

    /// Does a keyboard modifiers changed event.
    fn on_modifiers_changed(&mut self, window_id: WindowId, modifiers: ModifiersState);

    /// Does a key-down, key-up and updates.
    fn press_key(&mut self, window_id: WindowId, key: Key);

    /// Does a modifiers changed, key-down, key-up, reset modifiers and updates.
    fn press_modified_key(&mut self, window_id: WindowId, modifiers: ModifiersState, key: Key);
}
impl HeadlessAppKeyboardExt for HeadlessApp {
    fn on_keyboard_input(&mut self, window_id: WindowId, key: Key, state: ElementState) {
        #[allow(deprecated)]
        let raw_event = WindowEvent::KeyboardInput {
            device_id: unsafe { DeviceId::dummy() },
            input: KeyboardInput {
                scancode: 0,
                state,
                virtual_keycode: Some(key.into()),

                // what can we
                modifiers: ModifiersState::empty(),
            },
            is_synthetic: false,
        };

        self.window_event(window_id, &raw_event);
    }

    fn on_modifiers_changed(&mut self, window_id: WindowId, modifiers: ModifiersState) {
        let raw_event = WindowEvent::ModifiersChanged(modifiers);
        self.window_event(window_id, &raw_event);
    }

    fn press_key(&mut self, window_id: WindowId, key: Key) {
        self.on_keyboard_input(window_id, key, ElementState::Pressed);
        self.on_keyboard_input(window_id, key, ElementState::Released);
        self.update(false);
    }

    fn press_modified_key(&mut self, window_id: WindowId, modifiers: ModifiersState, key: Key) {
        if modifiers.is_empty() {
            self.press_key(window_id, key);
        } else {
            if modifiers.logo() {
                self.on_keyboard_input(window_id, Key::LLogo, ElementState::Pressed);
            }
            if modifiers.ctrl() {
                self.on_keyboard_input(window_id, Key::LCtrl, ElementState::Pressed);
            }
            if modifiers.shift() {
                self.on_keyboard_input(window_id, Key::LShift, ElementState::Pressed);
            }
            if modifiers.alt() {
                self.on_keyboard_input(window_id, Key::LAlt, ElementState::Pressed);
            }
            self.on_modifiers_changed(window_id, modifiers);

            // pressed the modifiers.
            self.update(false);

            self.on_keyboard_input(window_id, key, ElementState::Pressed);
            self.on_keyboard_input(window_id, key, ElementState::Released);

            // pressed the key.
            self.update(false);

            self.on_modifiers_changed(window_id, ModifiersState::default());
            if modifiers.logo() {
                self.on_keyboard_input(window_id, Key::LLogo, ElementState::Released);
            }
            if modifiers.ctrl() {
                self.on_keyboard_input(window_id, Key::LCtrl, ElementState::Released);
            }
            if modifiers.shift() {
                self.on_keyboard_input(window_id, Key::LShift, ElementState::Released);
            }
            if modifiers.alt() {
                self.on_keyboard_input(window_id, Key::LAlt, ElementState::Released);
            }

            // released the modifiers.
            self.update(false);
        }
    }
}

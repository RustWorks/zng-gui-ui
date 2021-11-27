//! Aggregate events.

use crate::{
    app::{raw_events::RawWindowFocusEvent, *},
    command::{Command, CommandMetaVar},
    context::*,
    event::*,
    focus::Focus,
    keyboard::*,
    mouse::*,
    render::*,
    service::Service,
    units::DipPoint,
    var::impl_from_and_into_var,
    widget_info::WidgetPath,
    window::{WindowId, Windows},
    WidgetId,
};
use std::{
    convert::{TryFrom, TryInto},
    fmt::{self, Display},
    mem,
    num::NonZeroU32,
    time::Duration,
};

/// Specific information from the source of a [`ClickArgs`].
#[derive(Debug, Clone)]
pub enum ClickArgsSource {
    /// Click event was generated by the [mouse click event](MouseClickEvent).
    Mouse {
        /// Which mouse button generated the event.
        button: MouseButton,

        /// Position of the mouse in the coordinates of [`target`](ClickArgs::target).
        position: DipPoint,

        /// Hit-test result for the mouse point in the window, at the moment the click event
        /// was generated.
        hits: FrameHitInfo,
    },

    /// Click event was generated by the [shortcut event](ShortcutEvent).
    Shortcut {
        /// The shortcut.
        shortcut: Shortcut,
        /// If the shortcut event was generated by holding a key pressed.
        is_repeat: bool,
        /// Kind of click represented by the `shortcut`.
        kind: ShortcutClick,
    },
}

/// What kind of click a shortcut represents in a [`ClickArgsSource::Shortcut`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutClick {
    /// The shortcut represents a primary click on the focused widget.
    Primary,
    /// The shortcut represents a context click on the focused widget.
    Context,
}

event_args! {
    /// [`ClickEvent`] arguments.
    pub struct ClickArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        ///
        /// Is `None` if the event was generated programmatically.
        pub device_id: Option<DeviceId>,

        /// Specific info from the source of this event.
        pub source: ClickArgsSource,

        /// Sequential click count. Number `1` is single click, `2` is double click, etc.
        ///
        /// This is always `1` for clicks initiated by the keyboard.
        pub click_count: NonZeroU32,

        /// What modifier keys where pressed when this event happened.
        pub modifiers: ModifiersState,

        /// The mouse input top-most hit or the focused element at the time of the key input.
        pub target: WidgetPath,

        ..

        /// If the widget is in [`target`](Self::target).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }

    /// [`ShortcutEvent`] arguments.
    pub struct ShortcutArgs {
        /// Id of window that received the event.
        pub window_id: WindowId,

        /// Id of device that generated the event.
        ///
        /// Is `None` if the event was generated programmatically.
        pub device_id: Option<DeviceId>,

        /// The shortcut.
        pub shortcut: Shortcut,

        /// If the event was generated by holding the key pressed.
        pub is_repeat: bool,

        /// The focused element at the time of the shortcut input.
        pub target: WidgetPath,

        ..

        /// If the widget is in [`target`](Self::target).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.contains(ctx.path.widget_id())
        }
    }
}

impl From<MouseClickArgs> for ClickArgs {
    #[inline]
    fn from(args: MouseClickArgs) -> Self {
        ClickArgs::new(
            args.timestamp,
            args.window_id,
            args.device_id,
            ClickArgsSource::Mouse {
                button: args.button,
                position: args.position,
                hits: args.hits,
            },
            args.click_count,
            args.modifiers,
            args.target,
        )
    }
}
impl ClickArgs {
    /// If the event counts as *primary* click.
    ///
    /// A primary click causes the default widget function interaction.
    ///
    /// Returns `true` if the click source is a left mouse button click or a
    /// [primary click shortcut](Gestures::click_focused).
    #[inline]
    pub fn is_primary(&self) -> bool {
        match &self.source {
            ClickArgsSource::Mouse { button, .. } => *button == MouseButton::Left,
            ClickArgsSource::Shortcut { kind, .. } => *kind == ShortcutClick::Primary,
        }
    }

    /// If the event counts as a *context menu* request.
    ///
    /// Returns `true` if the [`click_count`](Self::click_count) is `1` and the
    /// click source is a right mouse button click or a [context click shortcut](Gestures::context_click_focused).
    #[inline]
    pub fn is_context(&self) -> bool {
        self.click_count.get() == 1
            && match &self.source {
                ClickArgsSource::Mouse { button, .. } => *button == MouseButton::Right,
                ClickArgsSource::Shortcut { kind, is_repeat, .. } => *kind == ShortcutClick::Context && !is_repeat,
            }
    }

    /// If the event was caused by a press of `mouse_button`.
    #[inline]
    pub fn is_mouse_btn(&self, mouse_button: MouseButton) -> bool {
        match &self.source {
            ClickArgsSource::Mouse { button, .. } => *button == mouse_button,
            ClickArgsSource::Shortcut { .. } => false,
        }
    }

    /// If the event was generated by holding a keyboard key pressed.
    #[inline]
    pub fn is_repeat(&self) -> bool {
        match &self.source {
            ClickArgsSource::Shortcut { is_repeat, .. } => *is_repeat,
            ClickArgsSource::Mouse { .. } => false,
        }
    }

    /// The shortcut the generated this event.
    #[inline]
    pub fn shortcut(&self) -> Option<Shortcut> {
        match &self.source {
            ClickArgsSource::Shortcut { shortcut, .. } => Some(*shortcut),
            ClickArgsSource::Mouse { .. } => None,
        }
    }

    /// If the [`click_count`](Self::click_count) is `1`.
    #[inline]
    pub fn is_single(&self) -> bool {
        self.click_count.get() == 1
    }

    /// If the [`click_count`](Self::click_count) is `2`.
    #[inline]
    pub fn is_double(&self) -> bool {
        self.click_count.get() == 2
    }

    /// If the [`click_count`](Self::click_count) is `3`.
    #[inline]
    pub fn is_triple(&self) -> bool {
        self.click_count.get() == 3
    }

    /// If this event was generated by a mouse device.
    #[inline]
    pub fn is_from_mouse(&self) -> bool {
        matches!(&self.source, ClickArgsSource::Mouse { .. })
    }

    /// If this event was generated by a keyboard device.
    #[inline]
    pub fn is_from_keyboard(&self) -> bool {
        matches!(&self.source, ClickArgsSource::Shortcut { .. })
    }
}

/// A keyboard combination.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyGesture {
    /// The key modifiers.
    pub modifiers: ModifiersState,
    /// The key.
    pub key: GestureKey,
}
impl KeyGesture {
    #[inline]
    #[allow(missing_docs)]
    pub fn new(modifiers: ModifiersState, key: GestureKey) -> Self {
        KeyGesture { modifiers, key }
    }

    /// New key gesture without modifiers.
    #[inline]
    pub fn new_key(key: GestureKey) -> Self {
        KeyGesture {
            modifiers: ModifiersState::empty(),
            key,
        }
    }
}
impl fmt::Debug for KeyGesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("KeyGesture")
                .field("modifiers", &self.modifiers)
                .field("key", &self.key)
                .finish()
        } else {
            write!(f, "{}", self)
        }
    }
}
impl Display for KeyGesture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.modifiers.logo() {
            write!(f, "Logo+")?
        }
        if self.modifiers.ctrl() {
            write!(f, "Ctrl+")?
        }
        if self.modifiers.shift() {
            write!(f, "Shift+")?
        }
        if self.modifiers.alt() {
            write!(f, "Alt+")?
        }

        write!(f, "{}", self.key)
    }
}

/// A modifier key press and release without any other key press in between.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum ModifierGesture {
    /// Any of the Windows/Apple keys.
    Logo,
    /// Any of the CTRL keys.
    Ctrl,
    /// Any of the SHIFT keys.
    Shift,
    /// Any of the ALT keys.
    Alt,
}
impl fmt::Debug for ModifierGesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ModifierGesture::")?;
        }
        write!(f, "{}", self)
    }
}
impl Display for ModifierGesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModifierGesture::Logo => write!(f, "Logo"),
            ModifierGesture::Ctrl => write!(f, "Ctrl"),
            ModifierGesture::Shift => write!(f, "Shift"),
            ModifierGesture::Alt => write!(f, "Alt"),
        }
    }
}
impl TryFrom<Key> for ModifierGesture {
    type Error = Key;
    fn try_from(value: Key) -> Result<Self, Self::Error> {
        match value {
            Key::LAlt | Key::RAlt => Ok(ModifierGesture::Alt),
            Key::LCtrl | Key::RControl => Ok(ModifierGesture::Ctrl),
            Key::LShift | Key::RShift => Ok(ModifierGesture::Shift),
            Key::LLogo | Key::RLogo => Ok(ModifierGesture::Logo),
            key => Err(key),
        }
    }
}
impl ModifierGesture {
    fn left_key(&self) -> Key {
        match self {
            ModifierGesture::Logo => Key::LLogo,
            ModifierGesture::Ctrl => Key::LCtrl,
            ModifierGesture::Shift => Key::LShift,
            ModifierGesture::Alt => Key::LAlt,
        }
    }
    fn modifiers_state(&self) -> ModifiersState {
        match self {
            ModifierGesture::Logo => ModifiersState::LOGO,
            ModifierGesture::Ctrl => ModifiersState::CTRL,
            ModifierGesture::Shift => ModifiersState::SHIFT,
            ModifierGesture::Alt => ModifiersState::ALT,
        }
    }
}

/// A sequence of two keyboard combinations.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    /// The first key gesture.
    pub starter: KeyGesture,

    /// The second key gesture.
    pub complement: KeyGesture,
}
impl fmt::Debug for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("KeyChord")
                .field("starter", &self.starter)
                .field("complement", &self.complement)
                .finish()
        } else {
            write!(f, "{}", self)
        }
    }
}
impl Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.starter, self.complement)
    }
}

/// Keyboard gesture or chord associated with a command.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shortcut {
    /// Key-press plus modifiers.
    Gesture(KeyGesture),
    /// Sequence of two key gestures.
    Chord(KeyChord),
    /// Modifier press and release.
    Modifier(ModifierGesture),
}
impl fmt::Debug for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                Shortcut::Gesture(g) => f.debug_tuple("Shortcut::Gesture").field(g).finish(),
                Shortcut::Chord(c) => f.debug_tuple("Shortcut::Chord").field(c).finish(),
                Shortcut::Modifier(m) => f.debug_tuple("Shortcut::Modifier").field(m).finish(),
            }
        } else {
            write!(f, "{}", self)
        }
    }
}
impl Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Shortcut::Gesture(g) => Display::fmt(g, f),
            Shortcut::Chord(c) => Display::fmt(c, f),
            Shortcut::Modifier(m) => Display::fmt(m, f),
        }
    }
}
impl Shortcut {
    fn modifiers_state(&self) -> ModifiersState {
        match self {
            Shortcut::Gesture(g) => g.modifiers,
            Shortcut::Chord(c) => c.complement.modifiers,
            Shortcut::Modifier(m) => m.modifiers_state(),
        }
    }
}
impl_from_and_into_var! {
    fn from(shortcut: Shortcut) -> Shortcuts {
        Shortcuts(vec![shortcut])
    }

    fn from(key_gesture: KeyGesture) -> Shortcut {
        Shortcut::Gesture(key_gesture)
    }

    fn from(key_chord: KeyChord) -> Shortcut {
        Shortcut::Chord(key_chord)
    }

    fn from(modifier: ModifierGesture) -> Shortcut {
        Shortcut::Modifier(modifier)
    }

    fn from(gesture_key: GestureKey) -> Shortcut {
        KeyGesture::new_key(gesture_key).into()
    }

    fn from(gesture_key: GestureKey) -> Shortcuts {
        Shortcuts(vec![gesture_key.into()])
    }

    fn from(key_gesture: KeyGesture) -> Shortcuts {
        Shortcuts(vec![key_gesture.into()])
    }

    fn from(key_chord: KeyChord) -> Shortcuts {
        Shortcuts(vec![key_chord.into()])
    }

    fn from(modifier: ModifierGesture) -> Shortcuts {
        Shortcuts(vec![modifier.into()])
    }

    fn from(shortcuts: Vec<Shortcut>) -> Shortcuts {
        Shortcuts(shortcuts)
    }
}
impl<const N: usize> From<[Shortcut; N]> for Shortcuts {
    fn from(a: [Shortcut; N]) -> Self {
        Shortcuts(a.into())
    }
}
impl<const N: usize> crate::var::IntoVar<Shortcuts> for [Shortcut; N] {
    type Var = crate::var::OwnedVar<Shortcuts>;

    fn into_var(self) -> Self::Var {
        crate::var::OwnedVar(self.into())
    }
}

/// Multiple shortcuts.
#[derive(Default, Clone)]
pub struct Shortcuts(pub Vec<Shortcut>);
impl Shortcuts {
    /// New default (empty).
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to generate shortcuts that produce the `character`.
    ///
    /// Returns at least one shortcut or error the char back if it cannot
    /// be generated by a single shortcut.
    ///
    /// Note chords are not generated. Caps lock is assumed to be off.
    pub fn from_char(character: char) -> Result<Self, char> {
        let char_range_to_key = |char0: char, key0: GestureKey| {
            // SAFETY: this is safe if the char range matches the key range,
            // which we verified for all cases.
            let key: GestureKey = unsafe { mem::transmute(key0 as u32 + (character as u8 - char0 as u8) as u32) };
            key
        };

        match character {
            'a'..='z' => {
                let key = char_range_to_key('a', GestureKey::A);
                Ok(key.into())
            }
            'A'..='Z' => {
                let key = char_range_to_key('A', GestureKey::A);
                let gesture = KeyGesture::new(ModifiersState::SHIFT, key);
                Ok(gesture.into())
            }
            '1'..='9' => {
                let key = char_range_to_key('1', GestureKey::Key1);
                let num_key = char_range_to_key('1', GestureKey::Numpad1);
                Ok(Shortcuts(vec![key.into(), num_key.into()]))
            }
            '0' => Ok(Shortcuts(vec![GestureKey::Key0.into(), GestureKey::Numpad0.into()])),
            '+' => Ok(Shortcuts(vec![GestureKey::Plus.into(), GestureKey::NumpadAdd.into()])),
            '-' => Ok(Shortcuts(vec![GestureKey::Minus.into(), GestureKey::NumpadSubtract.into()])),
            '*' => Ok(Shortcuts(vec![GestureKey::Asterisk.into(), GestureKey::NumpadMultiply.into()])),
            '/' => Ok(Shortcuts(vec![GestureKey::Slash.into(), GestureKey::NumpadDivide.into()])),
            '.' => Ok(GestureKey::Period.into()),
            _ => Err(character),
        }
    }

    /// If the `shortcut` is present in the shortcuts.
    pub fn contains(&self, shortcut: Shortcut) -> bool {
        self.0.contains(&shortcut)
    }
}
impl TryFrom<char> for Shortcuts {
    type Error = char;

    /// See [`from_char`](Self::from_char).
    fn try_from(value: char) -> Result<Self, Self::Error> {
        Shortcuts::from_char(value)
    }
}
impl fmt::Debug for Shortcuts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("Shortcuts").field(&self.0).finish()
        } else {
            write!(f, "[")?;
            if !self.0.is_empty() {
                if let Shortcut::Chord(c) = self.0[0] {
                    write!(f, "({:?})", c)?;
                } else {
                    write!(f, "{:?}", self.0[0])?;
                }
                for shortcut in &self.0[1..] {
                    if let Shortcut::Chord(c) = shortcut {
                        write!(f, ", ({:?})", c)?;
                    } else {
                        write!(f, ", {:?}", shortcut)?;
                    }
                }
            }
            write!(f, "]")
        }
    }
}
impl fmt::Display for Shortcuts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.0.is_empty() {
            write!(f, "{}", self.0[0])?;
            for shortcut in &self.0[1..] {
                write!(f, " | {}", shortcut)?;
            }
        }
        Ok(())
    }
}
impl std::ops::Deref for Shortcuts {
    type Target = Vec<Shortcut>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Shortcuts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl KeyInputArgs {
    /// Key gesture this key press triggers.
    ///
    /// See also [`ShortcutArgs`].
    #[inline]
    pub fn gesture(&self) -> Option<KeyGesture> {
        if self.state == KeyState::Released {
            return None;
        }

        self.key.and_then(|k| k.try_into().ok()).map(|key| KeyGesture {
            key,
            modifiers: self.modifiers,
        })
    }

    /// Gets [`gesture`](Self::gesture) as a shortcut.
    ///
    /// See also [`ShortcutArgs`].
    #[inline]
    pub fn shortcut(&self) -> Option<Shortcut> {
        self.gesture().map(Shortcut::Gesture)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __shortcut {
    (-> + $Key:ident) => {
        $crate::gesture::KeyGesture {
            key: $crate::gesture::GestureKey::$Key,
            modifiers: $crate::keyboard::ModifiersState::empty(),
        }
    };

    (-> $($MODIFIER:ident)|+ + $Key:ident) => {
        $crate::gesture::KeyGesture {
            key: $crate::gesture::GestureKey::$Key,
            modifiers: $($crate::keyboard::ModifiersState::$MODIFIER)|+,
        }
    };

    (=> $($STARTER_MODIFIER:ident)|* + $StarterKey:ident, $($COMPLEMENT_MODIFIER:ident)|* + $ComplementKey:ident) => {
        $crate::gesture::KeyChord {
            starter: $crate::__shortcut!(-> $($STARTER_MODIFIER)|* + $StarterKey),
            complement: $crate::__shortcut!(-> $($COMPLEMENT_MODIFIER)|* + $ComplementKey)
        }
    };
}

///<span data-inline></span> Creates a [`Shortcut`](crate::gesture::Shortcut).
///
/// # Examples
///
/// ```
/// use zero_ui_core::gesture::{Shortcut, shortcut};
///
/// fn single_key() -> Shortcut {
///     shortcut!(Enter)
/// }
///
/// fn modified_key() -> Shortcut {
///     shortcut!(CTRL+C)
/// }
///
/// fn multi_modified_key() -> Shortcut {
///     shortcut!(CTRL|SHIFT+C)
/// }
///
/// fn chord() -> Shortcut {
///     shortcut!(CTRL+E, A)
/// }
///
/// fn modifier_release() -> Shortcut {
///     shortcut!(Alt)
/// }
/// ```
#[macro_export]
macro_rules! shortcut {
    (Logo) => {
        $crate::gesture::Shortcut::Modifier($crate::gesture::ModifierGesture::Logo)
    };
    (Shift) => {
        $crate::gesture::Shortcut::Modifier($crate::gesture::ModifierGesture::Shift)
    };
    (Ctrl) => {
        $crate::gesture::Shortcut::Modifier($crate::gesture::ModifierGesture::Ctrl)
    };
    (Alt) => {
        $crate::gesture::Shortcut::Modifier($crate::gesture::ModifierGesture::Alt)
    };

    ($Key:ident) => {
        $crate::gesture::Shortcut::Gesture($crate::__shortcut!(-> + $Key))
    };
    ($($MODIFIER:ident)|+ + $Key:ident) => {
        $crate::gesture::Shortcut::Gesture($crate::__shortcut!(-> $($MODIFIER)|+ + $Key))
    };

    ($StarterKey:ident, $ComplementKey:ident) => {
        $crate::gesture::Shortcut::Chord($crate::__shortcut!(=>
            + $StarterKey,
            + $ComplementKey
        ))
    };

    ($StarterKey:ident, $($COMPLEMENT_MODIFIER:ident)|+ + $ComplementKey:ident) => {
        $crate::gesture::Shortcut::Chord($crate::__shortcut!(=>
            + $StarterKey,
            $(COMPLEMENT_MODIFIER)|* + $ComplementKey
        ))
    };

    ($($STARTER_MODIFIER:ident)|+ + $StarterKey:ident, $ComplementKey:ident) => {
        $crate::gesture::Shortcut::Chord($crate::__shortcut!(=>
            $($STARTER_MODIFIER)|* + $StarterKey,
            + $ComplementKey
        ))
    };

    ($($STARTER_MODIFIER:ident)|+ + $StarterKey:ident, $($COMPLEMENT_MODIFIER:ident)|+ + $ComplementKey:ident) => {
        $crate::gesture::Shortcut::Chord($crate::__shortcut!(=>
            $($STARTER_MODIFIER)|* + $StarterKey,
            $($COMPLEMENT_MODIFIER)|* + $ComplementKey
        ))
    };
}
#[doc(inline)]
pub use crate::shortcut;

event! {
    /// Aggregate click event.
    ///
    /// Can be a mouse click, a shortcut press or a touch tap.
    pub ClickEvent: ClickArgs;

    /// Shortcut input event.
    ///
    /// Event happens every time a full [`Shortcut`] is completed.
    pub ShortcutEvent: ShortcutArgs;
}

/// Application extension that provides aggregate events.
///
/// Events this extension provides.
///
/// * [ClickEvent]
/// * [ShortcutEvent]
///
/// Services this extension provides.
///
/// * [Gestures]
pub struct GestureManager {
    pressed_modifier: Option<ModifierGesture>,
}
impl Default for GestureManager {
    fn default() -> Self {
        GestureManager { pressed_modifier: None }
    }
}
impl AppExtension for GestureManager {
    fn init(&mut self, r: &mut AppContext) {
        r.services.register(Gestures::new(r.updates.sender()));
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, _: &mut AppContext, args: &EV) {
        if let Some(args) = RawWindowFocusEvent.update(args) {
            if !args.focused {
                self.pressed_modifier = None;
            }
        }
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = MouseClickEvent.update(args) {
            // Generate click events from mouse clicks.
            if !args.stop_propagation_requested() {
                ClickEvent.notify(ctx.events, args.clone().into());
            }
        } else if let Some(args) = KeyInputEvent.update(args) {
            // Generate shortcut events from keyboard input.
            if !args.stop_propagation_requested() {
                if let Some(key) = args.key {
                    match args.state {
                        KeyState::Pressed => {
                            if let Ok(gesture_key) = GestureKey::try_from(key) {
                                let s_args = ShortcutArgs::new(
                                    args.timestamp,
                                    args.window_id,
                                    args.device_id,
                                    Shortcut::Gesture(KeyGesture::new(args.modifiers, gesture_key)),
                                    args.is_repeat,
                                    args.target.clone(),
                                );
                                ShortcutEvent.notify(ctx.events, s_args);
                                self.pressed_modifier = None;
                            } else if let Ok(mod_gesture) = ModifierGesture::try_from(key) {
                                if !args.is_repeat {
                                    self.pressed_modifier = Some(mod_gesture);
                                }
                            } else {
                                self.pressed_modifier = None;
                            }
                        }
                        KeyState::Released => {
                            if let Ok(mod_gesture) = ModifierGesture::try_from(key) {
                                if Some(mod_gesture) == self.pressed_modifier.take() && args.modifiers.is_empty() {
                                    let s_args = ShortcutArgs::new(
                                        args.timestamp,
                                        args.window_id,
                                        args.device_id,
                                        Shortcut::Modifier(mod_gesture),
                                        false,
                                        args.target.clone(),
                                    );
                                    ShortcutEvent.notify(ctx.events, s_args);
                                }
                            }
                        }
                    }
                } else {
                    // Scancode only
                    self.pressed_modifier = None;
                }
            }
        } else if let Some(args) = ShortcutEvent.update(args) {
            // Generate click events from shortcuts.
            if !args.stop_propagation_requested() {
                let gestures = ctx.services.gestures();
                let click = if gestures.click_focused.contains(args.shortcut) {
                    Some(ShortcutClick::Primary)
                } else if gestures.context_click_focused.contains(args.shortcut) {
                    Some(ShortcutClick::Context)
                } else {
                    None
                };
                if let Some(kind) = click {
                    ClickEvent.notify(
                        ctx.events,
                        ClickArgs::new(
                            args.timestamp,
                            args.window_id,
                            args.device_id,
                            ClickArgsSource::Shortcut {
                                shortcut: args.shortcut,
                                kind,
                                is_repeat: args.is_repeat,
                            },
                            NonZeroU32::new(1).unwrap(),
                            args.shortcut.modifiers_state(),
                            args.target.clone(),
                        ),
                    );
                } else {
                    let command = ctx
                        .events
                        .commands()
                        .find(|c| c.enabled_value() && c.shortcut().get(ctx.vars).contains(args.shortcut));
                    if let Some(command) = command {
                        command.notify(ctx.events, None);
                        args.stop_propagation()
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let (gestures, windows, focus) = ctx.services.req_multi::<(Gestures, Windows, Focus)>();
        // Generate click event for special shortcut.
        for (window_id, widget_id, kind, args) in gestures.click_shortcut.drain(..) {
            if let Ok(true) = windows.is_focused(window_id) {
                if let Some(widget) = windows.frame_info(window_id).unwrap().find(widget_id) {
                    // click target exists, in focused window.
                    ClickEvent.notify(
                        ctx.events,
                        ClickArgs::now(
                            window_id,
                            args.device_id,
                            ClickArgsSource::Shortcut {
                                shortcut: args.shortcut,
                                kind,
                                is_repeat: args.is_repeat,
                            },
                            NonZeroU32::new(1).unwrap(),
                            args.shortcut.modifiers_state(),
                            widget.path(),
                        ),
                    );
                    focus.focus_widget(widget_id, true);
                    args.stop_propagation();
                }
            }
        }
    }
}

/// Gesture events configuration.
#[derive(Service)]
pub struct Gestures {
    /// Shortcuts that generate a primary [`ClickEvent`] for the focused widget.
    /// The shortcut only works if no widget handles the [`ShortcutEvent`].
    ///
    /// Clicks generated by this shortcut count as [primary](ClickArgs::is_primary).
    ///
    /// Initial shortcuts are [`Enter`](Key::Enter) and [`Space`](Key::Space).
    pub click_focused: Shortcuts,

    /// Shortcuts that generate a context [`ClickEvent`] for the focused widget.
    /// The shortcut only works if no widget handles the [`ShortcutEvent`].
    ///
    /// Clicks generated by this shortcut count as [context](ClickArgs::is_context).
    ///
    /// Initial shortcut is [`Apps`](Key::Apps).
    pub context_click_focused: Shortcuts,

    /// When a shortcut primary click happens, targeted widgets can indicate that
    /// they are pressed for this duration.
    ///
    /// Initial value is `50ms`, set to to `0` to deactivate this type of indication.
    pub shortcut_pressed_duration: Duration,

    click_shortcut: Vec<(WindowId, WidgetId, ShortcutClick, ShortcutArgs)>,
    app_event_sender: AppEventSender,
}
impl Gestures {
    fn new(app_event_sender: AppEventSender) -> Self {
        Gestures {
            click_focused: [shortcut!(Enter), shortcut!(Space)].into(),
            context_click_focused: [shortcut!(Apps)].into(),
            shortcut_pressed_duration: Duration::from_millis(50),
            click_shortcut: vec![],
            app_event_sender,
        }
    }

    /// Schedules a click event for the next update. The click arguments will say that the
    /// click origin is the shortcut press.
    pub fn click_shortcut(&mut self, window_id: WindowId, widget_id: WidgetId, click_kind: ShortcutClick, args: ShortcutArgs) {
        self.click_shortcut.push((window_id, widget_id, click_kind, args));
        let _ = self.app_event_sender.send_update();
    }
}

impl std::str::FromStr for ModifierGesture {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "Ctrl" => Ok(ModifierGesture::Ctrl),
            "Shift" => Ok(ModifierGesture::Shift),
            "Alt" => Ok(ModifierGesture::Alt),
            "Logo" => Ok(ModifierGesture::Logo),
            s => Err(ParseError::new(format!("`{}` is not a modifier", s))),
        }
    }
}

impl std::str::FromStr for KeyGesture {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut modifiers = ModifiersState::empty();
        let mut parts = s.split('+');

        while let Some(part) = parts.next() {
            if let Ok(mod_) = part.parse::<ModifierGesture>() {
                match mod_ {
                    ModifierGesture::Logo => modifiers |= ModifiersState::LOGO,
                    ModifierGesture::Ctrl => modifiers |= ModifiersState::CTRL,
                    ModifierGesture::Shift => modifiers |= ModifiersState::SHIFT,
                    ModifierGesture::Alt => modifiers |= ModifiersState::ALT,
                }
            } else if let Ok(key) = part.parse::<GestureKey>() {
                if let Some(extra) = parts.next() {
                    return Err(ParseError::new(format!("`{}` is not a key gesture, unexpected `+{}`", s, extra)));
                }

                return Ok(KeyGesture { modifiers, key });
            } else {
                return Err(ParseError::new(format!("`{}` is not a key gesture, unexpected `{}`", s, part)));
            }
        }

        Err(ParseError::new(format!("`{}` is not a key gesture, missing key", s)))
    }
}

impl std::str::FromStr for KeyChord {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(',');

        let starter = if let Some(starter) = parts.next() {
            starter.parse()?
        } else {
            return Err(ParseError::new("`` is not a key chord, empty"));
        };

        let complement = if let Some(complement) = parts.next() {
            complement.parse()?
        } else {
            return Err(ParseError::new(format!("`{}` is not a key chord, expected `, <complement>`", s)));
        };

        if let Some(extra) = parts.next() {
            return Err(ParseError::new(format!("`{}` is not a key chord, unexpected `,{}`", s, extra)));
        }

        Ok(KeyChord { starter, complement })
    }
}

impl std::str::FromStr for Shortcut {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(',') {
            s.parse().map(Shortcut::Chord)
        } else if s.contains('+') {
            s.parse().map(Shortcut::Gesture)
        } else {
            s.parse()
                .map(Shortcut::Modifier)
                .map_err(|_| ParseError::new(format!("`{}` is not a shortcut", s)))
        }
    }
}

macro_rules! gesture_key_name {
    ($key:ident = $name:expr) => {
        $name
    };
    ($key:ident) => {
        stringify!($key)
    };
}

macro_rules! gesture_keys {
    ($($(#[$docs:meta])* $key:ident $(= $name:expr)?),+ $(,)?) => {
        /// The set of keys that can be used in a [`KeyGesture`].
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u32)]
        #[allow(missing_docs)] // they are mostly self-explanatory.
        pub enum GestureKey {
            $(
                $(#[$docs])*
                $key
            ),+
        }
        impl fmt::Debug for GestureKey {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                if f.alternate() {
                    write!(f, "GestureKey::")?;
                }
                match self {
                    $(
                        GestureKey::$key => write!(f, "{}", stringify!($key)),
                    )+
                }
            }
        }
        impl TryFrom<Key> for GestureKey {
            type Error = Key;

            fn try_from(key: Key) -> Result<Self, Key> {
                match key {
                    $(Key::$key => Ok(GestureKey::$key),)+
                    _ => Err(key)
                }
            }
        }
        impl From<GestureKey> for Key {
            fn from(key: GestureKey) -> Key {
                match key {
                    $(GestureKey::$key => Key::$key,)+
                }
            }
        }
        impl Display for GestureKey {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    $(GestureKey::$key => gesture_key_name!($key $(=$name)?).fmt(f),)+
                }
            }
        }
        impl std::str::FromStr for GestureKey {
            type Err = ParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.trim() {
                    $(stringify!($key) $(| $name)? => Ok(Self::$key),)+
                    s => Err(ParseError::new(format!("`{}` is not a gesture key", s)))
                }
            }
        }
    };
}

gesture_keys! {
    /// The '1' key over the letters.
    Key1 = "1",
    /// The '2' key over the letters.
    Key2 = "2",
    /// The '3' key over the letters.
    Key3 = "3",
    /// The '4' key over the letters.
    Key4 = "4",
    /// The '5' key over the letters.
    Key5 = "5",
    /// The '6' key over the letters.
    Key6 = "6",
    /// The '7' key over the letters.
    Key7 = "7",
    /// The '8' key over the letters.
    Key8 = "8",
    /// The '9' key over the letters.
    Key9 = "9",
    /// The '0' key over the 'O' and 'P' keys.
    Key0 = "0",
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
    Escape = "Esc",
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
    Pause,
    Insert,
    Home,
    Delete,
    End,
    PageDown = "Page Down",
    PageUp = "Page Up",
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
    Plus = "+",
    Asterisk = "*",
    Apostrophe = "'",
    Backslash = "\\",
    Comma = ",",
    Slash = "/",
    Equals = "=",
    Minus = "-",
    Period = ".",
    Numpad1 = "Numpad 1",
    Numpad2 = "Numpad 2",
    Numpad3 = "Numpad 3",
    Numpad4 = "Numpad 4",
    Numpad5 = "Numpad 5",
    Numpad6 = "Numpad 6",
    Numpad7 = "Numpad 7",
    Numpad8 = "Numpad 8",
    Numpad9 = "Numpad 9",
    Numpad0 = "Numpad 0",
    NumpadComma = "Numpad ,",
    NumpadAdd = "Numpad +",
    NumpadSubtract = "Numpad -",
    NumpadMultiply = "Numpad *",
    NumpadDivide = "Numpad /",
    NumpadEnter = "Numpad Enter",
    Tab,
    Apps,
}

/// Shortcut, gesture parsing error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Error message, usually in the pattern "`{invalid-input}` is not a {shortcut/modifier}".
    pub error: String,
}
impl ParseError {
    #[allow(missing_docs)]
    pub fn new(error: impl ToString) -> Self {
        ParseError { error: error.to_string() }
    }
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for ParseError {}

/// Extension trait that adds gesture simulation methods to [`HeadlessApp`].
pub trait HeadlessAppGestureExt {
    /// Does key presses to mimic the shortcut and updates.
    fn press_shortcut(&mut self, window_id: WindowId, shortcut: impl Into<Shortcut>);
}
impl HeadlessAppGestureExt for HeadlessApp {
    fn press_shortcut(&mut self, window_id: WindowId, shortcut: impl Into<Shortcut>) {
        let shortcut = shortcut.into();
        match shortcut {
            Shortcut::Modifier(m) => {
                self.press_key(window_id, m.left_key());
            }
            Shortcut::Gesture(g) => {
                self.press_modified_key(window_id, g.modifiers, g.key.into());
            }
            Shortcut::Chord(c) => {
                self.press_shortcut(window_id, c.starter);
                self.press_shortcut(window_id, c.complement);
            }
        }
    }
}

/// Adds the [`shortcut`](Self::shortcut) metadata.
///
/// If a command has a shortcut the [`GestureManager`] will invoke the command when the shortcut is pressed
/// and the command is enabled.
pub trait CommandShortcutExt {
    /// Gets a read-write variable that is zero-or-more shortcuts that invoke the command.
    fn shortcut(self) -> CommandMetaVar<Shortcuts>;

    /// Sets the initial shortcuts.
    fn init_shortcut(self, shortcut: impl Into<Shortcuts>) -> Self;
}
impl<C: Command> CommandShortcutExt for C {
    fn shortcut(self) -> CommandMetaVar<Shortcuts> {
        self.with_meta(|m| m.get_var_or_default(CommandShortcutKey))
    }

    fn init_shortcut(self, shortcut: impl Into<Shortcuts>) -> Self {
        self.with_meta(|m| m.init_var(CommandShortcutKey, shortcut.into()));
        self
    }
}
state_key! {
    struct CommandShortcutKey: Shortcuts;
}

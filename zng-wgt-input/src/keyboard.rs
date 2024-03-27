//! Keyboard events, [`on_key_down`](fn@on_key_down), [`on_key_up`](fn@on_key_up) and more.
//!
//! These events are low level and directly tied to a keyboard device.
//! Before using them review the [`gesture`](super::gesture) properties, in particular
//! the [`click_shortcut`](fn@super::gesture::click_shortcut) property.

use zng_ext_input::keyboard::{KeyInputArgs, KeyState, KEY_INPUT_EVENT};
use zng_wgt::prelude::*;

event_property! {
    /// Event fired when a keyboard key is pressed or released and the widget is enabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is disabled or blocked the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key press/release generates a key input event, including keys codes that don't map
    /// to any virtual key, see [`KeyInputArgs`] for more details.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// [`on_shortcut`]: fn@on_shortcut
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KEY_INPUT_EVENT`] that is included in the default app.
    pub fn key_input {
        event: KEY_INPUT_EVENT,
        args: KeyInputArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Event fired when a keyboard key is pressed or released and the widget is disabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is enabled or blocked the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key press/release generates a key input event, including keys that don't map
    /// to any virtual key, see [`KeyInputArgs`] for more details.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// [`on_shortcut`]: fn@on_shortcut
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KEY_INPUT_EVENT`] that is included in the default app.
    pub fn disabled_key_input {
        event: KEY_INPUT_EVENT,
        args: KeyInputArgs,
        filter: |args| args.is_disabled(WIDGET.id()),
    }

    /// Event fired when a keyboard key is pressed and the widget is enabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is disabled or blocked the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key press generates a key down event, including keys that don't map to any virtual key, see [`KeyInputArgs`]
    /// for more details.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// [`on_shortcut`]: fn@on_shortcut
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KEY_INPUT_EVENT`] that is included in the default app.
    pub fn key_down {
        event: KEY_INPUT_EVENT,
        args: KeyInputArgs,
        filter: |args| args.state == KeyState::Pressed && args.is_enabled(WIDGET.id()),
    }

    /// Event fired when a keyboard key is released and the widget is enabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is disabled or blocked the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key release generates a key up event, including keys that don't map to any virtual key, see [`KeyInputArgs`]
    /// for more details.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// [`on_shortcut`]: fn@on_shortcut
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KEY_INPUT_EVENT`] that is included in the default app.
    pub fn key_up {
        event: KEY_INPUT_EVENT,
        args: KeyInputArgs,
        filter: |args| args.state == KeyState::Released && args.is_enabled(WIDGET.id()),
    }
}

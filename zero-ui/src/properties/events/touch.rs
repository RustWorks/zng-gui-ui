//! Touch events, [`on_touch_move`](fn@on_touch_move), [`on_touch_tap`](fn@on_touch_tap),
//! [`on_touch_start`](fn@on_touch_start) and more.
//!
//! There events are low level and directly tied to touch inputs.
//! Before using them review the [`gesture`](super::gesture) events, in particular the
//! [`on_click`](fn@super::gesture::on_click) event.

use super::event_property;
use crate::core::{context::WIDGET, touch::*};

event_property! {
    /// Touch contact moved over the widget.
    pub fn touch_move {
        event: TOUCH_MOVE_EVENT,
        args: TouchMoveArgs,
    }

    /// Touch contact started or ended over the widget and the widget is enabled.
    pub fn touch_input {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Touch contact started or ended over the widget and the widget is disabled.
    pub fn disabled_touch_input {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.is_disabled(WIDGET.id()),
    }

    /// Touch contact started over the widget and the widget is enabled.
    pub fn touch_start {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.is_touch_start() && args.is_enabled(WIDGET.id()),
    }

    /// Touch contact ended over the widget and the widget is enabled.
    pub fn touch_end {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.is_touch_end() && args.is_enabled(WIDGET.id()),
    }

    /// Touch contact canceled over the widget and the widget is enabled.
    pub fn touch_cancel {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.is_touch_cancel() && args.is_enabled(WIDGET.id()),
    }

    /// Touch tap on the widget and the widget is enabled.
    pub fn touch_tap {
        event: TOUCH_TAP_EVENT,
        args: TouchTapArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Touch tap on the widget and the widget is disabled.
    pub fn disabled_touch_tap {
        event: TOUCH_TAP_EVENT,
        args: TouchTapArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Touch contact is now over the widget or a descendant widget and the widget is enabled.
    pub fn touch_enter {
        event: TOUCHED_EVENT,
        args: TouchedArgs,
        filter: |args| args.is_touch_enter_enabled(),
    }

    /// Touch contact is no longer over the widget or any descendant widget and the widget is enabled.
    pub fn touch_leave {
        event: TOUCHED_EVENT,
        args: TouchedArgs,
        filter: |args| args.is_touch_leave_enabled(),
    }

    /// Touch contact entered or left the widget and descendant widgets area and the widget is enabled.
    ///
    /// You can use the [`is_touch_enter`] and [`is_touch_leave`] methods to determinate the state change.
    ///
    /// [`is_touch_enter`]: TouchedArgs::is_touch_enter
    /// [`is_touch_leave`]: TouchedArgs::is_touch_leave
    pub fn touched {
        event: TOUCHED_EVENT,
        args: TouchedArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Touch gesture to translate, scale or rotate happened over this widget.
    pub fn touch_transform {
        event: TOUCH_TRANSFORM_EVENT,
        args: TouchTransformArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }
}

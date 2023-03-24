//! Commands that control focus.

use crate::{event::*, gesture::*};

use super::*;

command! {
    /// Represents the **focus next** action.
    pub static FOCUS_NEXT_CMD = {
        name: "Focus Next",
        info: "Focus next focusable.",
        shortcut: shortcut!(Tab),
    };

    /// Represents the **focus previous** action.
    pub static FOCUS_PREV_CMD = {
        name: "Focus Previous",
        info: "Focus previous focusable.",
        shortcut: shortcut!(SHIFT+Tab),
    };

    /// Represents the **focus/escape alt** action.
    pub static FOCUS_ALT_CMD = {
        name: "Focus Alt",
        info: "Focus alt scope.",
        shortcut: shortcut!(Alt),
    };

    /// Represents the **focus enter** action.
    pub static FOCUS_ENTER_CMD = {
        name: "Focus Enter",
        info: "Focus child focusable.",
        shortcut: [shortcut!(Enter), shortcut!(ALT+Enter)],
    };

    /// Represents the **focus exit** action.
    pub static FOCUS_EXIT_CMD = {
        name: "Focus Exit",
        info: "Focus parent focusable, or return focus.",
        shortcut: [shortcut!(Escape), shortcut!(ALT+Escape)],
    };

    /// Represents the **focus up** action.
    pub static FOCUS_UP_CMD = {
        name: "Focus Up",
        info: "Focus closest focusable up.",
        shortcut: [shortcut!(Up), shortcut!(ALT+Up)],
    };

    /// Represents the **focus down** action.
    pub static FOCUS_DOWN_CMD = {
        name: "Focus Down",
        info: "Focus closest focusable down.",
        shortcut: [shortcut!(Down), shortcut!(ALT+Down)],
    };

    /// Represents the **focus left** action.
    pub static FOCUS_LEFT_CMD = {
        name: "Focus Left",
        info: "Focus closest focusable left.",
        shortcut: [shortcut!(Left), shortcut!(ALT+Left)],
    };

    /// Represents the **focus right** action.
    pub static FOCUS_RIGHT_CMD = {
        name: "Focus Right",
        info: "Focus closest focusable right.",
        shortcut: [shortcut!(Right), shortcut!(ALT+Right)],
    };

    /// Represents a [`FocusRequest`] action.
    ///
    /// If this command parameter is a [`FocusRequest`] the request is made.
    pub static FOCUS_CMD;
}

pub(super) struct FocusCommands {
    next_handle: CommandHandle,
    prev_handle: CommandHandle,

    alt_handle: CommandHandle,

    up_handle: CommandHandle,
    down_handle: CommandHandle,
    left_handle: CommandHandle,
    right_handle: CommandHandle,

    exit_handle: CommandHandle,
    enter_handle: CommandHandle,

    #[allow(dead_code)]
    focus_handle: CommandHandle,
}
impl FocusCommands {
    pub fn new() -> Self {
        Self {
            next_handle: FOCUS_NEXT_CMD.subscribe(false),
            prev_handle: FOCUS_PREV_CMD.subscribe(false),

            alt_handle: FOCUS_ALT_CMD.subscribe(false),

            up_handle: FOCUS_UP_CMD.subscribe(false),
            down_handle: FOCUS_DOWN_CMD.subscribe(false),
            left_handle: FOCUS_LEFT_CMD.subscribe(false),
            right_handle: FOCUS_RIGHT_CMD.subscribe(false),

            exit_handle: FOCUS_EXIT_CMD.subscribe(false),
            enter_handle: FOCUS_ENTER_CMD.subscribe(false),

            focus_handle: FOCUS_CMD.subscribe(true),
        }
    }

    pub fn update_enabled(&mut self, nav: FocusNavAction) {
        self.next_handle.set_enabled(nav.contains(FocusNavAction::NEXT));
        self.prev_handle.set_enabled(nav.contains(FocusNavAction::PREV));

        self.alt_handle.set_enabled(nav.contains(FocusNavAction::ALT));

        self.up_handle.set_enabled(nav.contains(FocusNavAction::UP));
        self.down_handle.set_enabled(nav.contains(FocusNavAction::DOWN));
        self.left_handle.set_enabled(nav.contains(FocusNavAction::LEFT));
        self.right_handle.set_enabled(nav.contains(FocusNavAction::RIGHT));

        self.exit_handle.set_enabled(nav.contains(FocusNavAction::EXIT));
        self.enter_handle.set_enabled(nav.contains(FocusNavAction::ENTER));
    }

    pub fn event_preview(&mut self, update: &EventUpdate) {
        macro_rules! handle {
            ($($CMD:ident($handle:ident) => $method:ident,)+) => {$(
                if let Some(args) = $CMD.on(update) {
                    args.handle(|args| {
                        if args.enabled && self.$handle.is_enabled() {
                            FOCUS.$method();
                        } else {
                            FOCUS.on_disabled_cmd();
                        }
                    });
                    return;
                }
            )+};
        }
        handle! {
            FOCUS_NEXT_CMD(next_handle) => focus_next,
            FOCUS_PREV_CMD(prev_handle) => focus_prev,
            FOCUS_ALT_CMD(alt_handle) => focus_alt,
            FOCUS_UP_CMD(up_handle) => focus_up,
            FOCUS_DOWN_CMD(down_handle) => focus_down,
            FOCUS_LEFT_CMD(left_handle) => focus_left,
            FOCUS_RIGHT_CMD(right_handle) => focus_right,
            FOCUS_ENTER_CMD(enter_handle) => focus_enter,
            FOCUS_EXIT_CMD(exit_handle) => focus_exit,
        }

        if let Some(args) = FOCUS_CMD.on(update) {
            if let Some(req) = args.param::<FocusRequest>() {
                args.handle_enabled(&self.focus_handle, |_| {
                    FOCUS.focus(*req);
                });
            }
        }
    }
}

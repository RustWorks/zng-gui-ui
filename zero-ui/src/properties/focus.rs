//! Keyboard focus properties, [`tab_index`](fn@tab_index), [`focusable`](fn@focusable),
//! [`on_focus`](fn@on_focus), [`is_focused`](fn@is_focused) and more.

use crate::core::focus::*;
use crate::prelude::new_property::*;

/// Enables a widget to receive focus.
#[property(CONTEXT, default(false))]
pub fn focusable(child: impl UiNode, focusable: impl IntoVar<bool>) -> impl UiNode {
    let focusable = focusable.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&focusable);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).focusable(focusable.get());
        }
        _ => {}
    })
}

/// Customizes the widget order during TAB navigation.
#[property(CONTEXT, default(TabIndex::default()))]
pub fn tab_index(child: impl UiNode, tab_index: impl IntoVar<TabIndex>) -> impl UiNode {
    let tab_index = tab_index.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&tab_index);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).tab_index(tab_index.get());
        }
        _ => {}
    })
}

/// Widget is a focus scope.
#[property(CONTEXT, default(false))]
pub fn focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    focus_scope_impl(child, is_scope, false)
}
/// Widget is the ALT focus scope.
///
/// ALT focus scopes are also, `TabIndex::SKIP`, `skip_directional_nav`, `TabNav::Cycle` and `DirectionalNav::Cycle` by default.
///
/// Also see [`focus_click_behavior`] that can be used to return focus automatically when any widget inside the ALT scope
/// handles a click.
///
/// [`focus_click_behavior`]: fn@focus_click_behavior
#[property(CONTEXT, default(false))]
pub fn alt_focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    focus_scope_impl(child, is_scope, true)
}

fn focus_scope_impl(child: impl UiNode, is_scope: impl IntoVar<bool>, is_alt: bool) -> impl UiNode {
    let is_scope = is_scope.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&is_scope);
        }
        UiNodeOp::Info { info } => {
            let mut info = FocusInfoBuilder::new(info);
            if is_alt {
                info.alt_scope(is_scope.get());
            } else {
                info.scope(is_scope.get());
            }
        }
        _ => {}
    })
}

/// Behavior of a focus scope when it receives direct focus.
#[property(CONTEXT, default(FocusScopeOnFocus::default()))]
pub fn focus_scope_behavior(child: impl UiNode, behavior: impl IntoVar<FocusScopeOnFocus>) -> impl UiNode {
    let behavior = behavior.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&behavior);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).on_focus(behavior.get());
        }
        _ => {}
    })
}

/// Tab navigation within this focus scope.
#[property(CONTEXT, default(TabNav::Continue))]
pub fn tab_nav(child: impl UiNode, tab_nav: impl IntoVar<TabNav>) -> impl UiNode {
    let tab_nav = tab_nav.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&tab_nav);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).tab_nav(tab_nav.get());
        }
        _ => {}
    })
}

/// Arrows navigation within this focus scope.
#[property(CONTEXT, default(DirectionalNav::Continue))]
pub fn directional_nav(child: impl UiNode, directional_nav: impl IntoVar<DirectionalNav>) -> impl UiNode {
    let directional_nav = directional_nav.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&directional_nav);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).directional_nav(directional_nav.get());
        }
        _ => {}
    })
}

/// Keyboard shortcuts that focus this widget or its first focusable descendant or its first focusable parent.
#[property(CONTEXT, default(Shortcuts::default()))]
pub fn focus_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    let shortcuts = shortcuts.into_var();
    let mut _handle = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&shortcuts);
            let s = shortcuts.get();
            _handle = Some(GESTURES.focus_shortcut(s, WIDGET.id()));
        }
        UiNodeOp::Update { .. } => {
            if let Some(s) = shortcuts.get_new() {
                _handle = Some(GESTURES.focus_shortcut(s, WIDGET.id()));
            }
        }
        _ => {}
    })
}

/// If directional navigation from outside this widget skips over it and its descendants.
///
/// Setting this to `true` is the directional navigation equivalent of setting `tab_index` to `SKIP`.
#[property(CONTEXT, default(false))]
pub fn skip_directional(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&enabled);
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).skip_directional(enabled.get());
        }
        _ => {}
    })
}

/// Behavior of an widget when a click event is send to it or a descendant.
///
/// See [`focus_click_behavior`] for more details.
///
/// [`focus_click_behavior`]: fn@focus_click_behavior
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusClickBehavior {
    /// Click event always ignored.
    Ignore,
    /// Exit focus if a click event was send to the widget or descendant.
    Exit,
    /// Exit focus if a click event was send to the enabled widget or enabled descendant.
    ExitEnabled,
    /// Exit focus if the click event was received by the widget or descendant and event propagation was stopped.
    ExitHandled,
}

impl std::fmt::Debug for FocusClickBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "FocusClickBehavior::")?;
        }
        match self {
            Self::Ignore => write!(f, "Ignore"),
            Self::Exit => write!(f, "Exit"),
            Self::ExitEnabled => write!(f, "ExitEnabled"),
            Self::ExitHandled => write!(f, "ExitHandled"),
        }
    }
}

/// Behavior of an widget when a click event is send to it or a descendant.
///
/// When a click event targets the widget or descendant the `behavior` is applied.
///
/// Note that this property does not subscribe to any event, it only observes events flowing trough.
#[property(CONTEXT, default(FocusClickBehavior::Ignore))]
pub fn focus_click_behavior(child: impl UiNode, behavior: impl IntoVar<FocusClickBehavior>) -> impl UiNode {
    let behavior = behavior.into_var();
    match_node(child, move |c, op| {
        if let UiNodeOp::Event { update } = op {
            c.event(update);

            if let Some(args) = CLICK_EVENT.on(update) {
                let exit = match behavior.get() {
                    FocusClickBehavior::Ignore => false,
                    FocusClickBehavior::Exit => true,
                    FocusClickBehavior::ExitEnabled => args.target.interactivity().is_enabled(),
                    FocusClickBehavior::ExitHandled => args.propagation().is_stopped(),
                };
                if exit {
                    FOCUS.focus_exit();
                }
            } else if let Some(args) = crate::core::mouse::MOUSE_INPUT_EVENT.on(update) {
                if args.propagation().is_stopped() {
                    // CLICK_EVENT not send if source mouse-input is already handled.

                    let exit = match behavior.get() {
                        FocusClickBehavior::Ignore => false,
                        FocusClickBehavior::Exit => true,
                        FocusClickBehavior::ExitEnabled => args.target.interactivity().is_enabled(),
                        FocusClickBehavior::ExitHandled => true,
                    };
                    if exit {
                        FOCUS.focus_exit();
                    }
                }
            }
        }
    })
}

event_property! {
    /// Focus changed in the widget or its descendants.
    pub fn focus_changed {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
    }

    /// Widget got direct keyboard focus.
    pub fn focus {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |args| args.is_focus(WIDGET.id()),
    }

    /// Widget lost direct keyboard focus.
    pub fn blur {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |args| args.is_blur(WIDGET.id()),
    }

    /// Widget or one of its descendants got focus.
    pub fn focus_enter {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |args| args.is_focus_enter(WIDGET.id())
    }

    /// Widget or one of its descendants lost focus.
    pub fn focus_leave {
        event: FOCUS_CHANGED_EVENT,
        args: FocusChangedArgs,
        filter: |args| args.is_focus_leave(WIDGET.id())
    }
}

/// If the widget has keyboard focus.
///
/// This is only `true` if the widget itself is focused.
/// You can use [`is_focus_within`] to include focused widgets inside this one.
///
/// # Highlighting
///
/// This property is always `true` when the widget has focus, ignoring what device was used to move the focus,
/// usually when the keyboard is used a special visual indicator is rendered, a dotted line border is common,
/// this state is called *highlighting* and is tracked by the focus manager. To implement such a visual you can use the
/// [`is_focused_hgl`] property.
///
/// # Return Focus
///
/// Usually widgets that have a visual state for this property also have one for [`is_return_focus`], a common example is the
/// *text-input* or *text-box* widget that shows an emphasized border and blinking cursor when focused and still shows the
/// emphasized border without cursor when a menu is open and it is only the return focus.
///
/// [`is_focus_within`]: fn@zero_ui::properties::focus::is_focus_within
/// [`is_focused_hgl`]: fn@zero_ui::properties::focus::is_focused_hgl
/// [`is_return_focus`]: fn@zero_ui::properties::focus::is_return_focus
#[property(CONTEXT)]
pub fn is_focused(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state(child, state, false, FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_focus(id) {
            Some(true)
        } else if args.is_blur(id) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants has keyboard focus.
///
/// To check if only the widget has keyboard focus use [`is_focused`].
///
/// To track *highlighted* focus within use [`is_focus_within_hgl`] property.
///
/// [`is_focused`]: fn@zero_ui::properties::focus::is_focused
/// [`is_focus_within_hgl`]: fn@zero_ui::properties::focus::is_focus_within_hgl
#[property(CONTEXT)]
pub fn is_focus_within(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state(child, state, false, FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_focus_enter(id) {
            Some(true)
        } else if args.is_focus_leave(id) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget has keyboard focus and the user is using the keyboard to navigate.
///
/// This is only `true` if the widget itself is focused and the focus was acquired by keyboard navigation.
/// You can use [`is_focus_within_hgl`] to include widgets inside this one.
///
/// # Highlighting
///
/// Usually when the keyboard is used to move the focus a special visual indicator is rendered, a dotted line border is common,
/// this state is called *highlighting* and is tracked by the focus manager, this property is only `true`.
///
/// [`is_focus_within_hgl`]: fn@zero_ui::properties::focus::is_focus_within_hgl
/// [`is_focused`]: fn@zero_ui::properties::focus::is_focused
#[property(CONTEXT)]
pub fn is_focused_hgl(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state(child, state, false, FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_focus(id) {
            Some(args.highlight)
        } else if args.is_blur(id) {
            Some(false)
        } else if args.is_hightlight_changed() && args.new_focus.as_ref().map(|p| p.widget_id() == id).unwrap_or(false) {
            Some(args.highlight)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants has keyboard focus and the user is using the keyboard to navigate.
///
/// To check if only the widget has keyboard focus use [`is_focused_hgl`].
///
/// Also see [`is_focus_within`] to check if the widget has focus within regardless of highlighting.
///
/// [`is_focused_hgl`]: fn@zero_ui::properties::focus::is_focused_hgl
/// [`is_focus_within`]: fn@zero_ui::properties::focus::is_focus_within
#[property(CONTEXT)]
pub fn is_focus_within_hgl(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state(child, state, false, FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_focus_enter(id) {
            Some(args.highlight)
        } else if args.is_focus_leave(id) {
            Some(false)
        } else if args.is_hightlight_changed() && args.new_focus.as_ref().map(|p| p.contains(id)).unwrap_or(false) {
            Some(args.highlight)
        } else {
            None
        }
    })
}

/// If the widget will be focused when a parent scope is focused.
///
/// Focus scopes can be configured to remember the last focused widget inside then, the focus than *returns* to
/// this widget when the scope receives focus. Alt scopes also remember the widget from which the *alt* focus happened
/// and can also return focus back to that widget.
///
/// Usually input widgets that have a visual state for [`is_focused`] also have a visual for this, a common example is the
/// *text-input* or *text-box* widget that shows an emphasized border and blinking cursor when focused and still shows the
/// emphasized border without cursor when a menu is open and it is only the return focus.
///
/// Note that a widget can be [`is_focused`] and `is_return_focus`, this property is `true` if any focus scope considers the
/// widget its return focus, you probably want to declare the widget visual states in such a order that [`is_focused`] overrides
/// the state of this property.
///
/// [`is_focused`]: fn@zero_ui::properties::focus::is_focused_hgl
/// [`is_focused_hgl`]: fn@zero_ui::properties::focus::is_focused_hgl
#[property(CONTEXT)]
pub fn is_return_focus(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state(child, state, false, RETURN_FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_return_focus(id) {
            Some(true)
        } else if args.was_return_focus(id) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget or one of its descendants will be focused when a focus scope is focused.
///
/// To check if only the widget is the return focus use [`is_return_focus`].
///
/// [`is_return_focus`]: fn@zero_ui::properties::focus::is_return_focus
#[property(CONTEXT)]
pub fn is_return_focus_within(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    event_is_state(child, state, false, RETURN_FOCUS_CHANGED_EVENT, |args| {
        let id = WIDGET.id();
        if args.is_return_focus_enter(id) {
            Some(true)
        } else if args.is_return_focus_leave(id) {
            Some(false)
        } else {
            None
        }
    })
}

/// If the widget is focused on info init.
///
/// When the widget is inited and present in the info tree a [`FOCUS.focus_widget_or_related`] request is made for the widget.
#[property(CONTEXT, default(false))]
pub fn focus_on_init(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();

    enum State {
        WaitInfo,
        InfoInited,
        Done,
    }
    let mut state = State::WaitInfo;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            if enabled.get() {
                state = State::WaitInfo;
            } else {
                state = State::Done;
            }
        }
        UiNodeOp::Info { .. } => {
            if let State::WaitInfo = &state {
                state = State::InfoInited;
                // next update will be after the info is in tree.
                WIDGET.update();
            }
        }
        UiNodeOp::Update { .. } => {
            if let State::InfoInited = &state {
                state = State::Done;
                FOCUS.focus_widget_or_related(WIDGET.id(), false);
            }
        }
        _ => {}
    })
}

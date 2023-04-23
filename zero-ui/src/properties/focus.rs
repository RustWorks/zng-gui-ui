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
            WIDGET.sub_var(&focusable);
        }
        UiNodeOp::Update { .. } => {
            if focusable.is_new() {
                WIDGET.update_info();
            }
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
            WIDGET.sub_var(&tab_index);
        }
        UiNodeOp::Update { .. } => {
            if tab_index.is_new() {
                WIDGET.update_info();
            }
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
#[property(CONTEXT, default(false))]
pub fn alt_focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    focus_scope_impl(child, is_scope, true)
}

fn focus_scope_impl(child: impl UiNode, is_scope: impl IntoVar<bool>, is_alt: bool) -> impl UiNode {
    let is_scope = is_scope.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&is_scope);
        }
        UiNodeOp::Update { .. } => {
            if is_scope.is_new() {
                WIDGET.update_info();
            }
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
            WIDGET.sub_var(&behavior);
        }
        UiNodeOp::Update { .. } => {
            if behavior.is_new() {
                WIDGET.update_info();
            }
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
            if tab_nav.is_new() {
                WIDGET.update_info();
            }
        }
        UiNodeOp::Update { .. } => {
            if tab_nav.is_new() {
                WIDGET.update_info();
            }
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
            WIDGET.sub_var(&directional_nav);
        }
        UiNodeOp::Update { .. } => {
            if directional_nav.is_new() {
                WIDGET.update_info();
            }
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
            WIDGET.sub_var(&enabled);
        }
        UiNodeOp::Update { .. } => {
            if enabled.is_new() {
                WIDGET.update_info();
            }
        }
        UiNodeOp::Info { info } => {
            FocusInfoBuilder::new(info).skip_directional(enabled.get());
        }
        _ => {}
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

/// If the widget is focused on init.
///
/// When the widget is inited a [`FOCUS.focus_widget_or_related`] request is made for the widget.
#[property(CONTEXT, default(false))]
pub fn focus_on_init(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();
    match_node(child, move |child, op| {
        if let UiNodeOp::Init = op {
            child.init();

            if enabled.get() {
                FOCUS.focus_widget_or_related(WIDGET.id(), false);
            }
        }
    })
}

//! Widget state properties, [`is_hovered`], [`is_pressed`], [`is_focused`] and more.

use crate::core::focus::*;
use crate::core::mouse::*;
use crate::prelude::new_property::*;

struct IsHoveredNode<C: UiNode> {
    child: C,
    state: StateVar,
    mouse_enter: EventListener<MouseHoverArgs>,
    mouse_leave: EventListener<MouseHoverArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsHoveredNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.mouse_enter = ctx.events.listen::<MouseEnterEvent>();
        self.mouse_leave = ctx.events.listen::<MouseLeaveEvent>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let mut state = *self.state.get(ctx.vars);

        if IsEnabled::get(ctx.vars) {
            if self.mouse_leave.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                state = false;
            }
            if self.mouse_enter.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                state = true;
            }
        } else {
            state = false;
        }

        if state != *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, state);
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the mouse pointer is over the widget.
///
/// This is always `false` when the widget is [disabled](IsEnabled).
#[property(context)]
pub fn is_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsHoveredNode {
        child,
        state,
        mouse_enter: MouseEnterEvent::never(),
        mouse_leave: MouseLeaveEvent::never(),
    }
}

struct IsPressedNode<C: UiNode> {
    child: C,
    state: StateVar,
    mouse_down: EventListener<MouseInputArgs>,
    mouse_up: EventListener<MouseInputArgs>,
    mouse_leave: EventListener<MouseHoverArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsPressedNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.mouse_down = ctx.events.listen::<MouseDownEvent>();
        self.mouse_up = ctx.events.listen::<MouseUpEvent>();
        self.mouse_leave = ctx.events.listen::<MouseLeaveEvent>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let mut state = *self.state.get(ctx.vars);

        if IsEnabled::get(ctx.vars) {
            if self.mouse_up.has_updates(ctx.events) || self.mouse_leave.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                state = false;
            }
            if self.mouse_down.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                state = true;
            }
        } else {
            state = false;
        }

        if state != *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, state);
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the mouse pointer is pressed in the widget.
///
/// This is always `false` when the widget is [disabled](IsEnabled).
#[property(context)]
pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsPressedNode {
        child,
        state,
        mouse_down: MouseDownEvent::never(),
        mouse_up: MouseUpEvent::never(),
        mouse_leave: MouseLeaveEvent::never(),
    }
}

struct IsFocusedNode<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusedNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused = *self.state.get(ctx.vars);
            let is_focused = u
                .new_focus
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or_default();
            if was_focused != is_focused {
                self.state.set(ctx.vars, is_focused);
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the widget has keyboard focus.
///
/// This is only `true` if the widget itself is focused.
/// You can use [`is_focus_within`] to check if the focused widget is within this one.
///
/// # Highlighting
///
/// TODO
#[property(context)]
pub fn is_focused(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusedNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

struct IsFocusWithinNode<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusWithinNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused = *self.state.get(ctx.vars);
            let is_focused = u.new_focus.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or_default();

            if was_focused != is_focused {
                self.state.set(ctx.vars, is_focused);
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the widget or one of its descendants has keyboard focus.
///
/// To check if only the widget has keyboard focus use [`is_focused`].
#[property(context)]
pub fn is_focus_within(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusWithinNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

struct IsFocusedHglNode<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusedHglNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused_hgl = *self.state.get(ctx.vars);
            let is_focused_hgl = u.highlight
                && u.new_focus
                    .as_ref()
                    .map(|p| p.widget_id() == ctx.path.widget_id())
                    .unwrap_or_default();
            if was_focused_hgl != is_focused_hgl {
                self.state.set(ctx.vars, is_focused_hgl);
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the widget has keyboard focus and focus highlighting is enabled.
///
/// This is only `true` if the widget itself is focused and focus highlighting is enabled.
/// You can use [`is_focus_within_hgl`] to check if the focused widget is within this one.
///
/// Also see [`is_focused`] to check if the widget is focused regardless of highlighting.
#[property(context)]
pub fn is_focused_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusedHglNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

struct IsFocusWithinHglNode<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusWithinHglNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused_hgl = *self.state.get(ctx.vars);
            let is_focused_hgl = u.highlight && u.new_focus.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or_default();

            if was_focused_hgl != is_focused_hgl {
                self.state.set(ctx.vars, is_focused_hgl);
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the widget or one of its descendants has keyboard focus and focus highlighting is enabled.
///
/// To check if only the widget has keyboard focus use [`is_focused_hgl`].
///
/// Also see [`is_focus_within`] to check if the widget has focus within regardless of highlighting.
#[property(context)]
pub fn is_focus_within_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusWithinHglNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

struct IsReturnFocusNode<C: UiNode> {
    child: C,
    state: StateVar,
    return_focus_changed: EventListener<ReturnFocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsReturnFocusNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.return_focus_changed = ctx.events.listen::<ReturnFocusChangedEvent>();
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let state = *self.state.get(ctx.vars);
        let mut new_state = state;
        for args in self.return_focus_changed.updates(ctx.events) {
            if args
                .prev_return
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or_default()
            {
                new_state = false;
            }
            if args
                .new_return
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or_default()
            {
                new_state = true;
            }
        }

        if new_state != state {
            self.state.set(ctx.vars, new_state);
        }
    }
}

/// If the widget is focused when a parent scope is focused.
#[property(context)]
pub fn is_return_focus(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsReturnFocusNode {
        child,
        state,
        return_focus_changed: ReturnFocusChangedEvent::never(),
    }
}

struct IsHitTestableNode<C: UiNode> {
    child: C,
    state: StateVar,
    //expected: bool,
}
impl<C: UiNode> IsHitTestableNode<C> {
    fn update_state(&self, ctx: &mut WidgetContext) {
        let hit_testable = IsHitTestable::get(ctx.vars) && ctx.widget_state.hit_testable();
        let is_state = hit_testable; // == self.expected;
        if is_state != *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, is_state);
        }
    }
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsHitTestableNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.update_state(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        self.update_state(ctx);
    }
}

/// If the widget is hit-test visible.
///
/// This property is used only for probing the state. You can set the state using the
/// [`hit_testable`](crate::properties::hit_testable) property.
#[property(context)]
pub fn is_hit_testable(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsHitTestableNode {
        child,
        state,
        //expected: true
    }
}

struct IsEnabledNode<C: UiNode> {
    child: C,
    state: StateVar,
    expected: bool,
}
impl<C: UiNode> IsEnabledNode<C> {
    fn update_state(&self, ctx: &mut WidgetContext) {
        let enabled = IsEnabled::get(ctx.vars) && ctx.widget_state.enabled();
        let is_state = enabled == self.expected;
        if is_state != *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, is_state);
        }
    }
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsEnabledNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.update_state(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        self.update_state(ctx);
    }
}

/// If the widget is enabled for receiving events.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`](crate::properties::enabled) property.
#[property(context)]
pub fn is_enabled(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsEnabledNode {
        child,
        state,
        expected: true,
    }
}

/// If the widget is disabled for receiving events.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`](crate::properties::enabled) property.
///
/// This is the same as `!self.is_enabled`.
#[property(context)]
pub fn is_disabled(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsEnabledNode {
        child,
        state,
        expected: false,
    }
}

use crate::properties::{Visibility, VisibilityContext, WidgetVisibilityExt};

use super::{IsHitTestable, WidgetHitTestableExt};

struct IsVisibilityNode<C: UiNode> {
    child: C,
    state: StateVar,
    expected: Visibility,
}
impl<C: UiNode> IsVisibilityNode<C> {
    fn update_state(&self, ctx: &mut WidgetContext) {
        let vis = VisibilityContext::get(ctx.vars) | ctx.widget_state.visibility();
        let is_state = vis == self.expected;
        if is_state != *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, is_state);
        }
    }
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsVisibilityNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.update_state(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        self.update_state(ctx);
    }
}

/// If the widget [`visibility`](super::visibility) is [`Visible`](super::Visibility::Visible).
#[property(context)]
pub fn is_visible(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Visible,
    }
}

/// If the widget [`visibility`](super::visibility) is [`Hidden`](super::Visibility::Hidden).
#[property(context)]
pub fn is_hidden(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Hidden,
    }
}

/// If the widget [`visibility`](super::visibility) is [`Collapsed`](super::Visibility::Collapsed).
#[property(context)]
pub fn is_collapsed(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Collapsed,
    }
}

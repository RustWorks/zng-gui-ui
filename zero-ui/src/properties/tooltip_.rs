use std::mem;
use std::time::{Duration, Instant};

use crate::core::{mouse::MOUSE_HOVERED_EVENT, timer::DeadlineVar};

use crate::prelude::{
    new_property::*,
    window::{AnchorOffset, AnchorSize, AnchorTransform},
    *,
};

context_var! {
    /// Position of the tip widget in relation to the anchor widget.
    ///
    /// By default the tip widget is shown below the cursor.
    pub static TOOLTIP_TRANSFORM_VAR: AnchorTransform = AnchorTransform::CursorOnce(AnchorOffset::out_bottom_in_left());

    /// Duration the cursor must be over the anchor widget before the tip widget is opened.
    pub static TOOLTIP_DELAY_VAR: Duration = 500.ms();

    /// Maximum duration from the last time a tooltip was shown that a new tooltip opens instantly.
    pub static TOOLTIP_INTERVAL_VAR: Duration = 100.ms();

    /// Maximum time a tooltip stays open.
    pub static TOOLTIP_DURATION_VAR: Duration = 5.secs();
}

app_local! {
    /// Tracks the instant the last tooltip was closed on the widget.
    ///
    /// This value is used to implement the [`TOOLTIP_INTERVAL_VAR`], custom tooltip implementers must set it
    /// to integrate with the [`tooltip`] implementation.
    ///
    /// [`tooltip`]: fn@tooltip
    pub static TOOLTIP_LAST_CLOSED: Instant = Instant::now() - 1.hours();
}

/// Set the position of the tip widgets opened for the widget or its descendants.
///
/// Tips are inserted as [`LayerIndex::TOP_MOST`] when shown, this property defines how the tip layer
/// is aligned with the *anchor* widget, or the cursor.
///
/// By default tips are aligned below the cursor position at the time they are opened.
///
/// This property sets the [`TOOLTIP_TRANSFORM_VAR`].
#[property(CONTEXT, default(TOOLTIP_TRANSFORM_VAR))]
pub fn tooltip_transform(child: impl UiNode, transform: impl IntoVar<AnchorTransform>) -> impl UiNode {
    with_context_var(child, TOOLTIP_TRANSFORM_VAR, transform)
}

/// Set the duration the cursor must be over the widget or its descendants before the tip widget is opened.
///
/// This delay applies when no other tooltip was opened within the [`tooltip_interval`] duration, otherwise the
/// tooltip opens instantly.
///
/// This property sets the [`TOOLTIP_DELAY_VAR`].
///
/// [`tooltip_interval`]: fn@tooltip_interval
#[property(CONTEXT, default(TOOLTIP_DELAY_VAR))]
pub fn tooltip_delay(child: impl UiNode, delay: impl IntoVar<Duration>) -> impl UiNode {
    with_context_var(child, TOOLTIP_DELAY_VAR, delay)
}

/// Sets the maximum interval a second tooltip is opened instantly if a previous tip was just closed. The config
/// applies for tooltips opening on the widget or descendants, but considers previous tooltips opened on any widget.
///
/// This property sets the [`TOOLTIP_INTERVAL_VAR`].
#[property(CONTEXT, default(TOOLTIP_INTERVAL_VAR))]
pub fn tooltip_interval(child: impl UiNode, interval: impl IntoVar<Duration>) -> impl UiNode {
    with_context_var(child, TOOLTIP_INTERVAL_VAR, interval)
}

/// Sets the maximum duration a tooltip stays open on the widget or descendants.
///
/// Note that the tooltip closes at the moment the cursor leaves the widget, this duration defines the
/// time the tooltip is closed even if the cursor is still hovering the widget.
///
/// This property sets the [`TOOLTIP_DURATION_VAR`].
#[property(CONTEXT, default(TOOLTIP_DURATION_VAR))]
pub fn tooltip_duration(child: impl UiNode, duration: impl IntoVar<Duration>) -> impl UiNode {
    with_context_var(child, TOOLTIP_DURATION_VAR, duration)
}

/// Widget tooltip.
///
/// Any other widget can be used as tooltip, the recommended widget is the [`tip!`] container, it provides the tooltip style.
///
/// # Context
///
/// This property can be configured by [`tooltip_transform`], [`tooltip_delay`], [`tooltip_interval`] and [`tooltip_duration`].
///
/// # Disabled
///
/// This tooltip only opens if the widget is enabled, see [`disabled_tooltip`] for a tooltip that only shows when the widget is disabled.
///
/// [`tip!`]: mod@crate::widgets::tip
/// [`tooltip_transform`]: fn@tooltip_transform
/// [`tooltip_delay`]: fn@tooltip_delay
/// [`tooltip_interval`]: fn@tooltip_interval
#[property(EVENT)]
pub fn tooltip(child: impl UiNode, tip: impl UiNode) -> impl UiNode {
    tooltip_fn(child, WidgetFn::singleton(tip))
}

/// Widget tooltip set as an widget function that is called every time the tooltip must be shown.
///
/// The `tip` widget function is used to instantiate a new tip widget when one needs to be shown, any widget
/// can be used as tooltip, the recommended widget is the [`tip!`] container, it provides the tooltip style.
///
/// # Context
///
/// This property can be configured by [`tooltip_transform`], [`tooltip_delay`], [`tooltip_interval`] and [`tooltip_duration`].
///
/// # Disabled
///
/// This tooltip only opens if the widget is enabled, see [`disabled_tooltip`] for a tooltip that only shows when the widget is disabled.
///
/// [`tip!`]: mod@crate::widgets::tip
/// [`tooltip_transform`]: fn@tooltip_transform
/// [`tooltip_delay`]: fn@tooltip_delay
/// [`tooltip_interval`]: fn@tooltip_interval
#[property(EVENT, default(WidgetFn::nil()))]
pub fn tooltip_fn(child: impl UiNode, tip: impl IntoVar<WidgetFn<TooltipArgs>>) -> impl UiNode {
    TooltipNode {
        child,
        tip: tip.into_var(),
        disabled_only: false,
        state: TooltipState::Closed,
    }
}

/// Disabled widget tooltip.
///
/// This property behaves like [`tooltip`], but the tooltip only opens if the widget is disabled.
///
/// Note that the `tip` widget will be opened in a disabled context, so the tip style can use [`is_disabled`]
/// to provide an alternative look.
///
/// [`tooltip`]: fn@tooltip
/// [`is_disabled`]: fn@is_disabled
#[property(EVENT)]
pub fn disabled_tooltip(child: impl UiNode, tip: impl UiNode) -> impl UiNode {
    disabled_tooltip_fn(child, WidgetFn::singleton(tip))
}

/// Disabled widget tooltip.
///
/// This property behaves like [`tooltip_fn`], but the tooltip only opens if the widget is disabled.
///
/// Note that the `tip` widget will be opened in a disabled context, so the tip style can use [`is_disabled`]
/// to provide an alternative look.
///
/// [`tooltip_fn`]: fn@tooltip
/// [`is_disabled`]: fn@is_disabled
#[property(EVENT, default(WidgetFn::nil()))]
pub fn disabled_tooltip_fn(child: impl UiNode, tip: impl IntoVar<WidgetFn<TooltipArgs>>) -> impl UiNode {
    TooltipNode {
        child,
        tip: tip.into_var(),
        disabled_only: true,
        state: TooltipState::Closed,
    }
}

/// Arguments for tooltip widget functions.
///
/// [`tooltip_fn`]: fn@tooltip_fn
/// [`disabled_tooltip_fn`]: fn@disabled_tooltip_fn
pub struct TooltipArgs {
    /// If the tooltip is for [`disabled_tooltip_fn`], if `false` is for [`tooltip_fn`].
    ///
    /// [`tooltip_fn`]: fn@tooltip_fn
    /// [`disabled_tooltip_fn`]: fn@disabled_tooltip_fn
    pub disabled: bool,
}

#[derive(Default)]
enum TooltipState {
    #[default]
    Closed,
    Delay(DeadlineVar),
    /// Tip layer ID and duration deadline.
    Open(WidgetId, Option<DeadlineVar>),
}

#[ui_node(struct TooltipNode {
    child: impl UiNode,
    tip: impl Var<WidgetFn<TooltipArgs>>,
    disabled_only: bool,
    state: TooltipState,
})]
impl UiNode for TooltipNode {
    fn init(&mut self) {
        WIDGET.sub_var(&self.tip).sub_event(&MOUSE_HOVERED_EVENT);
        self.child.init()
    }

    fn deinit(&mut self) {
        self.child.deinit();
        if let TooltipState::Open(id, _) = mem::take(&mut self.state) {
            LAYERS.remove(id);
            TOOLTIP_LAST_CLOSED.set(Instant::now());
        }
    }

    fn event(&mut self, update: &EventUpdate) {
        self.child.event(update);

        if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
            if let TooltipState::Open(t, _) = &self.state {
                if !WINDOW.widget_tree().contains(*t) {
                    // already closed (from the layer probably)
                    self.state = TooltipState::Closed;
                }
            }
            match &self.state {
                TooltipState::Open(tooltip_id, _) => {
                    if !args
                        .target
                        .as_ref()
                        .map(|t| t.contains(*tooltip_id) || t.contains(WIDGET.id()))
                        .unwrap_or(true)
                    {
                        LAYERS.remove(*tooltip_id);
                        TOOLTIP_LAST_CLOSED.set(Instant::now());
                        self.state = TooltipState::Closed;
                    }
                }
                TooltipState::Delay(_) => {
                    if !args.target.as_ref().map(|t| t.contains(WIDGET.id())).unwrap_or(true) {
                        // cancel
                        self.state = TooltipState::Closed;
                    }
                }
                TooltipState::Closed => {
                    if args.is_mouse_enter() && args.is_enabled(WIDGET.id()) != self.disabled_only {
                        let delay = if TOOLTIP_LAST_CLOSED.get().elapsed() > TOOLTIP_INTERVAL_VAR.get() {
                            TOOLTIP_DELAY_VAR.get()
                        } else {
                            Duration::ZERO
                        };

                        self.state = if delay == Duration::ZERO {
                            TooltipState::Open(open_tooltip(self.tip.get(), self.disabled_only), duration_timer())
                        } else {
                            let delay = TIMERS.deadline(delay);
                            delay.subscribe(WIDGET.id()).perm();
                            TooltipState::Delay(delay)
                        };
                    }
                }
            }
        }
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.child.update(updates);

        match &mut self.state {
            TooltipState::Open(tooltip_id, timer) => {
                if let Some(t) = &timer {
                    if let Some(t) = t.get_new() {
                        if t.has_elapsed() {
                            LAYERS.remove(*tooltip_id);
                            TOOLTIP_LAST_CLOSED.set(Instant::now());
                            self.state = TooltipState::Closed;
                        }
                    }
                } else if let Some(func) = self.tip.get_new() {
                    LAYERS.remove(*tooltip_id);
                    *tooltip_id = open_tooltip(func, self.disabled_only);
                }
            }
            TooltipState::Delay(delay) => {
                if let Some(t) = delay.get_new() {
                    if t.has_elapsed() {
                        self.state = TooltipState::Open(open_tooltip(self.tip.get(), self.disabled_only), duration_timer());
                    }
                }
            }
            TooltipState::Closed => {}
        }
    }
}
fn open_tooltip(func: WidgetFn<TooltipArgs>, disabled: bool) -> WidgetId {
    let mut child = func(TooltipArgs { disabled }).boxed();

    if !child.is_widget() {
        let node = widget_base::nodes::widget_inner(child);

        // set hit test mode so that it's only hit-testable if the child is hit-testable
        let node = hit_test_mode(node, HitTestMode::Visual);
        let node = widget_base::enabled(node, !disabled);

        child = widget_base::nodes::widget(node, WidgetId::new_unique()).boxed();
    }

    let tooltip = TooltipLayerNode {
        child,
        anchor_id: WIDGET.id(),
    };

    let id = tooltip.with_context(|| WIDGET.id()).unwrap();

    let mode = AnchorMode {
        transform: AnchorTransform::CursorOnce(AnchorOffset::out_bottom_in_left()),
        size: AnchorSize::Unbounded,
        viewport_bound: true,
        visibility: true,
        interactivity: false,
        corner_radius: false,
    };

    LAYERS.insert_anchored(LayerIndex::TOP_MOST, tooltip.anchor_id, mode, tooltip);

    id
}

fn duration_timer() -> Option<DeadlineVar> {
    let duration = TOOLTIP_DURATION_VAR.get();
    if duration > Duration::ZERO {
        let dur = TIMERS.deadline(duration);
        dur.subscribe(WIDGET.id()).perm();
        Some(dur)
    } else {
        None
    }
}

#[ui_node(struct TooltipLayerNode {
    child: impl UiNode,
    anchor_id: WidgetId,
})]
impl UiNode for TooltipLayerNode {
    fn with_context<R, F: FnOnce() -> R>(&self, f: F) -> Option<R> {
        self.child.with_context(f)
    }

    fn init(&mut self) {
        // if the tooltip is hit-testable and the mouse hovers it, the anchor widget
        // will not receive mouse-leave, because it is not the logical parent of the tooltip,
        // so we need to duplicate cleanup logic here.
        self.with_context(|| {
            WIDGET.sub_event(&MOUSE_HOVERED_EVENT);
        });
        self.child.init()
    }

    fn event(&mut self, update: &EventUpdate) {
        self.child.event(update);

        if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
            let tooltip_id = self.with_context(|| WIDGET.id()).unwrap();
            let keep_open = if let Some(t) = &args.target {
                t.contains(self.anchor_id) || t.contains(tooltip_id)
            } else {
                false
            };
            if !keep_open {
                LAYERS.remove(tooltip_id);
                TOOLTIP_LAST_CLOSED.set(Instant::now());
            }
        }
    }
}

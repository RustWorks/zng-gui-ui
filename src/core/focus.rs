//! Keyboard focus manager.
//!
//! The [`FocusManager`] struct is an [app extension](crate::core::app::AppExtension). It
//! is included in the [default app](crate::core::app::App::default) and provides the [`Focus`] service
//! and the [`FocusChangedEvent`] event.
//!
//! # Keyboard Focus
//!
//! In a given program only a single widget can receive keyboard input at a time, this widget has the *keyboard focus*.
//!
//! # Navigation
//!
//! The keyboard focus can be moved from one widget to the next using the keyboard or the [`Focus`] service methods.
//! There are two styles of movement: [tabbing](#tab-navigation) that follows the logical order and [directional](#directional-navigation)
//! that follows the visual order.
//!
//! Keyboard navigation behaves different depending on what region of the screen the current focused widget is in, these regions
//! are called [focus scopes](#-focus-scopes). Every window is a focus scope that can be subdivided further.
//!
//! ## Tab Navigation
//!
//! Tab navigation follows a logical order, the position of the widget in the [widget tree](FrameFocusInfo),
//! optionally overridden with a [custom index](TabIndex).
//!
//! Focus is moved forward by pressing `TAB` or calling [`focus_next`](Focus::focus_next) and backward by pressing `SHIFT+TAB` or
//! calling [`focus_prev`](Focus::focus_prev).
//!
//! ## Directional Navigation
//!
//! Directional navigation follows the visual position of the widget on the screen.
//!
//! Focus is moved by pressing the **arrow keys** or calling the focus direction methods in the [`Focus`](Focus::focus_up) service.
//!
//! ## Focus Scopes
//!
//! TODO

use crate::core::app::AppExtension;
use crate::core::context::*;
use crate::core::event::*;
use crate::core::keyboard::*;
use crate::core::mouse::*;
use crate::core::render::{FrameInfo, WidgetInfo, WidgetPath};
use crate::core::types::*;
use crate::core::window::{WindowIsActiveArgs, WindowIsActiveChangedEvent, Windows};

event_args! {
    /// [`FocusChangedEvent`] arguments.
    pub struct FocusChangedArgs {
        /// Previously focused widget.
        pub prev_focus: Option<WidgetPath>,

        /// Newly focused widget.
        pub new_focus: Option<WidgetPath>,

        /// If the focused widget should visually indicate that it is focused.
        ///
        /// This is `true` when the focus change is caused by a key press, `false` when it is caused by a mouse click.
        ///
        /// Some widgets, like *text input*, may ignore this field and always indicate that they are focused.
        pub highlight: bool,

        ..

        /// If the widget is [prev_focus](FocusChangedArgs::prev_focus) or
        /// [`new_focus`](FocusChangedArgs::new_focus).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            if let Some(prev) = &self.prev_focus {
                if prev.widget_id() == ctx.widget_id {
                    return true
                }
            }

            if let Some(new) = &self.new_focus {
                if new.widget_id() == ctx.widget_id {
                    return true
                }
            }

            false
        }
    }
}

impl FocusChangedArgs {
    /// If the focus is still in the same widget but the widget path changed.
    #[inline]
    pub fn is_widget_move(&self) -> bool {
        match (&self.prev_focus, &self.new_focus) {
            (Some(prev), Some(new)) => prev.widget_id() == new.widget_id() && prev != new,
            _ => false,
        }
    }

    /// If the focus is still in the same widget but [`highlight`](FocusChangedArgs::highlight) changed.
    #[inline]
    pub fn is_hightlight_changed(&self) -> bool {
        self.prev_focus == self.new_focus
    }
}

state_key! {
    pub(crate) struct IsFocusableKey: bool;
    pub(crate) struct TabIndexKey: TabIndex;
    pub(crate) struct IsFocusScopeKey: bool;
    pub(crate) struct TabNavKey: TabNav;
    pub(crate) struct DirectionalNavKey: DirectionalNav;
}

/// Widget tab navigation position within a focus scope.
///
/// The index is zero based, zero first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TabIndex(pub u32);

impl TabIndex {
    /// Widget is skipped during tab navigation.
    ///
    /// The integer value is `u32::MAX`.
    pub const SKIP: TabIndex = TabIndex(u32::MAX);

    /// Default focusable widget index.
    ///
    /// Tab navigation uses the widget position in the widget tree when multiple widgets have the same index
    /// so if no widget index is explicitly set they get auto-sorted by their position.
    ///
    /// The integer value is `u32::MAX / 2`.
    pub const AUTO: TabIndex = TabIndex(u32::MAX / 2);

    /// If is [`SKIP`](TabIndex::SKIP).
    #[inline]
    pub fn is_skip(self) -> bool {
        self == Self::SKIP
    }

    /// If is [`AUTO`](TabIndex::AUTO).
    #[inline]
    pub fn is_auto(self) -> bool {
        self == Self::AUTO
    }

    /// Create a new tab index that is guaranteed to not be [`SKIP`](Self::SKIP).
    ///
    /// Returns `SKIP - 1` if `index` is `SKIP`.
    #[inline]
    pub fn not_skip(index: u32) -> Self {
        TabIndex(if index == Self::SKIP.0 { Self::SKIP.0 - 1 } else { index })
    }

    /// Create a new tab index that is guaranteed to be before [`AUTO`](Self::AUTO).
    ///
    /// Returns `AUTO - 1` if `index` is equal to or greater then `AUTO`.
    #[inline]
    pub fn before_auto(index: u32) -> Self {
        TabIndex(if index >= Self::AUTO.0 { Self::AUTO.0 - 1 } else { index })
    }

    /// Create a new tab index that is guaranteed to be after [`AUTO`](Self::AUTO) and not [`SKIP`](Self::SKIP).
    ///
    /// The `index` argument is zero based here.
    ///
    /// Returns `not_skip(AUTO + 1 + index)`.
    #[inline]
    pub fn after_auto(index: u32) -> Self {
        Self::not_skip((Self::AUTO.0 + 1).saturating_add(index))
    }
}

/// Tab navigation configuration of a focus scope.
///
/// See the [module level](zero_ui::core::focus#tab-navigation) for an overview of tab navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabNav {
    /// Tab does not move the focus inside the scope.
    None,
    /// Tab moves the focus through the scope continuing out after the last item.
    Continue,
    /// Tab is contained in the scope, does not move after the last item.
    Contained,
    /// Tab is contained in the scope, after the last item moves to the first item in the scope.
    Cycle,
    /// Tab moves into the scope once but then moves out of the scope.
    Once,
}

/// Directional navigation configuration of a focus scope.
///
/// See the [module level](zero_ui::core::focus#directional-navigation) for an overview of directional navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionalNav {
    /// Arrows does not move the focus inside the scope.
    None,
    /// Arrows move the focus through the scope continuing out of the edges.
    Continue,
    ///
    Contained,
    Cycle,
}

event! {
    /// Keyboard focused widget changed event.
    ///
    /// # Provider
    ///
    /// This event is provided by the [`FocusManager`] extension.
    pub FocusChangedEvent: FocusChangedArgs;
}

/// Application extension that manages keyboard focus.
///
/// # Events
///
/// Events this extension provides.
///
/// * [FocusChangedEvent]
///
/// # Services
///
/// Services this extension provides.
///
/// * [Focus]
///
/// # Requirements
///
/// This extension requires the [`MouseDownEvent`],
/// [`KeyDownEvent`] and [`WindowIsActiveChangedEvent`]
///  events to function.
///
/// # About Focus
///
/// See the [module level](zero_ui::core::focus) documentation for an overview of the keyboard
/// focus concepts implemented by this app extension.
pub struct FocusManager {
    focus_changed: EventEmitter<FocusChangedArgs>,
    windows_activation: EventListener<WindowIsActiveArgs>,
    mouse_down: EventListener<MouseInputArgs>,
    key_down: EventListener<KeyInputArgs>,
    focused: Option<WidgetPath>,
}
impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focus_changed: FocusChangedEvent::emitter(),
            windows_activation: WindowIsActiveChangedEvent::never(),
            mouse_down: MouseDownEvent::never(),
            key_down: KeyDownEvent::never(),
            focused: None,
        }
    }
}
impl AppExtension for FocusManager {
    fn init(&mut self, ctx: &mut AppInitContext) {
        self.windows_activation = ctx.events.listen::<WindowIsActiveChangedEvent>();
        self.mouse_down = ctx.events.listen::<MouseDownEvent>();
        self.key_down = ctx.events.listen::<KeyDownEvent>();

        ctx.services.register(Focus::new(ctx.updates.notifier().clone()));

        ctx.events.register::<FocusChangedEvent>(self.focus_changed.listener());
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update_hp {
            return;
        }

        let mut request = None;

        if let Some(req) = ctx.services.req::<Focus>().request.take() {
            // custom
            request = Some(req);
        } else if let Some(args) = self.mouse_down.updates(ctx.events).last() {
            // click
            request = Some(FocusRequest::direct_or_parent(args.target.widget_id(), false));
        } else if let Some(args) = self.key_down.updates(ctx.events).last() {
            // keyboard
            match &args.key {
                Some(VirtualKeyCode::Tab) => {
                    request = Some(if args.modifiers.shift() {
                        FocusRequest::prev(true)
                    } else {
                        FocusRequest::next(true)
                    })
                }
                Some(VirtualKeyCode::Up) => request = Some(FocusRequest::up(true)),
                Some(VirtualKeyCode::Right) => request = Some(FocusRequest::right(true)),
                Some(VirtualKeyCode::Down) => request = Some(FocusRequest::down(true)),
                Some(VirtualKeyCode::Left) => request = Some(FocusRequest::left(true)),
                _ => {}
            }
        }

        if let Some(request) = request {
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();
            self.notify(focus.fulfill_request(request, windows), ctx);
        } else if self.windows_activation.has_updates(ctx.events) {
            // foreground window maybe changed
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();
            self.notify(focus.continue_focus(windows), ctx);
        }
    }

    fn on_new_frame_ready(&mut self, window_id: WindowId, ctx: &mut AppContext) {
        if self.focused.as_ref().map(|f| f.window_id() == window_id).unwrap_or_default() {
            let (focus, windows) = ctx.services.req_multi::<(Focus, Windows)>();
            // new window frame, check if focus is still valid
            self.notify(focus.continue_focus(windows), ctx);
        }
    }
}
impl FocusManager {
    fn notify(&mut self, args: Option<FocusChangedArgs>, ctx: &mut AppContext) {
        if let Some(args) = args {
            self.focused = args.new_focus.clone();
            ctx.updates.push_notify(self.focus_changed.clone(), args);
        }
    }
}

/// Keyboard focus service.
///
/// # Provider
///
/// This service is provided by the [`FocusManager`] extension.
pub struct Focus {
    request: Option<FocusRequest>,
    update_notifier: UpdateNotifier,
    focused: Option<WidgetPath>,
    is_highlighting: bool,
}

impl Focus {
    #[inline]
    pub fn new(update_notifier: UpdateNotifier) -> Self {
        Focus {
            request: None,
            update_notifier,
            focused: None,
            is_highlighting: false,
        }
    }

    /// Current focused widget.
    #[inline]
    pub fn focused(&self) -> Option<&WidgetPath> {
        self.focused.as_ref()
    }

    /// If the current focused widget is visually indicated.
    #[inline]
    pub fn is_highlighting(&self) -> bool {
        self.is_highlighting
    }

    /// Request a focus update.
    #[inline]
    pub fn focus(&mut self, request: FocusRequest) {
        self.request = Some(request);
        self.update_notifier.push_update();
    }

    /// Focus the widget if it is focusable.
    #[inline]
    pub fn focus_widget(&mut self, widget_id: WidgetId, highlight: bool) {
        self.focus(FocusRequest::direct(widget_id, highlight))
    }

    /// Focus the widget if it is focusable, else focus the first focusable parent.
    #[inline]
    pub fn focus_widget_or_parent(&mut self, widget_id: WidgetId, highlight: bool) {
        self.focus(FocusRequest::direct_or_parent(widget_id, highlight))
    }

    #[inline]
    pub fn focus_next(&mut self) {
        self.focus(FocusRequest::next(self.is_highlighting));
    }

    #[inline]
    pub fn focus_prev(&mut self) {
        self.focus(FocusRequest::prev(self.is_highlighting));
    }

    #[inline]
    pub fn focus_up(&mut self) {
        self.focus(FocusRequest::up(self.is_highlighting));
    }

    #[inline]
    pub fn focus_right(&mut self) {
        self.focus(FocusRequest::right(self.is_highlighting));
    }

    #[inline]
    pub fn focus_down(&mut self) {
        self.focus(FocusRequest::down(self.is_highlighting));
    }

    #[inline]
    pub fn focus_left(&mut self) {
        self.focus(FocusRequest::left(self.is_highlighting));
    }

    #[must_use]
    fn fulfill_request(&mut self, request: FocusRequest, windows: &Windows) -> Option<FocusChangedArgs> {
        match (&self.focused, request.target) {
            (_, FocusTarget::Direct(widget_id)) => self.focus_direct(widget_id, request.highlight, false, windows),
            (_, FocusTarget::DirectOrParent(widget_id)) => self.focus_direct(widget_id, request.highlight, true, windows),
            (Some(prev), move_) => {
                if let Ok(w) = windows.window(prev.window_id()) {
                    let frame = FrameFocusInfo::new(w.frame_info());
                    if let Some(w) = frame.find(prev.widget_id()) {
                        if let Some(new_focus) = match move_ {
                            FocusTarget::Next => w.next_tab(),
                            FocusTarget::Prev => w.prev_tab(),
                            FocusTarget::Up => w.next_up(),
                            FocusTarget::Right => w.next_right(),
                            FocusTarget::Down => w.next_down(),
                            FocusTarget::Left => w.next_left(),
                            FocusTarget::Direct { .. } | FocusTarget::DirectOrParent { .. } => unreachable!(),
                        } {
                            self.move_focus(Some(new_focus.info.path()), request.highlight)
                        } else {
                            // widget may have moved inside the same window.
                            self.continue_focus_highlight(windows, request.highlight)
                        }
                    } else {
                        // widget not found.
                        self.continue_focus_highlight(windows, request.highlight)
                    }
                } else {
                    // window not found
                    self.continue_focus_highlight(windows, request.highlight)
                }
            }
            _ => None,
        }
    }

    /// Checks if `focused()` is still valid, if not moves focus to nearest valid.
    #[must_use]
    fn continue_focus(&mut self, windows: &Windows) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused {
            if let Ok(window) = windows.window(focused.window_id()) {
                if window.is_active() {
                    if let Some(widget) = window.frame_info().find(focused.widget_id()).map(|w| w.as_focus_info()) {
                        if widget.is_focusable() {
                            // :-) probably in the same place, maybe moved inside same window.
                            self.move_focus(Some(widget.info.path()), self.is_highlighting)
                        } else {
                            // widget no longer focusable
                            if let Some(parent) = widget.parent() {
                                // move to focusable parent
                                self.move_focus(Some(parent.info.path()), self.is_highlighting)
                            } else {
                                // no focusable parent, is this an error?
                                self.move_focus(None, false)
                            }
                        }
                    } else {
                        // widget not found
                        self.continue_focus_moved_widget(windows)
                    }
                } else {
                    // window not active anymore
                    self.continue_focus_moved_widget(windows)
                }
            } else {
                // window not found
                self.continue_focus_moved_widget(windows)
            }
        } else {
            // no previous focus
            self.focus_active_window(windows, false)
        }
    }

    #[must_use]
    fn continue_focus_moved_widget(&mut self, windows: &Windows) -> Option<FocusChangedArgs> {
        let focused = self.focused.as_ref().unwrap();
        for window in windows.windows() {
            if let Some(widget) = window.frame_info().find(focused.widget_id()).map(|w| w.as_focus_info()) {
                // found the widget in another window
                if window.is_active() {
                    return if widget.is_focusable() {
                        // same widget, moved to another window
                        self.move_focus(Some(widget.info.path()), self.is_highlighting)
                    } else {
                        // widget no longer focusable
                        if let Some(parent) = widget.parent() {
                            // move to focusable parent
                            self.move_focus(Some(parent.info.path()), self.is_highlighting)
                        } else {
                            // no focusable parent, is this an error?
                            self.move_focus(None, false)
                        }
                    };
                }
                break;
            }
        }
        // did not find the widget in a focusable context, was removed or is inside an inactive window.
        self.focus_active_window(windows, self.is_highlighting)
    }

    #[must_use]
    fn continue_focus_highlight(&mut self, windows: &Windows, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(mut args) = self.continue_focus(windows) {
            args.highlight = highlight;
            self.is_highlighting = highlight;
            Some(args)
        } else if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            Some(FocusChangedArgs::now(self.focused.clone(), self.focused.clone(), highlight))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_direct(
        &mut self,
        widget_id: WidgetId,
        highlight: bool,
        fallback_to_parents: bool,
        windows: &Windows,
    ) -> Option<FocusChangedArgs> {
        for w in windows.windows() {
            let frame = w.frame_info();
            if let Some(w) = frame.find(widget_id).map(|w| w.as_focus_info()) {
                if w.is_focusable() {
                    return self.move_focus(Some(w.info.path()), highlight);
                } else if fallback_to_parents {
                    if let Some(w) = w.parent() {
                        return self.move_focus(Some(w.info.path()), highlight);
                    } else {
                        // no focusable parent, just activate window?
                        //TODO
                    }
                }
                break;
            }
        }

        self.change_highlight(highlight)
    }

    #[must_use]
    fn change_highlight(&mut self, highlight: bool) -> Option<FocusChangedArgs> {
        if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            Some(FocusChangedArgs::now(self.focused.clone(), self.focused.clone(), highlight))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_active_window(&mut self, windows: &Windows, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(active) = windows.windows().find(|w| w.is_active()) {
            let frame = FrameFocusInfo::new(active.frame_info());
            let root = frame.root();
            if root.is_focusable() {
                // found active window and it is focusable.
                self.move_focus(Some(root.info.path()), highlight)
            } else {
                // has active window but it is not focusable
                self.move_focus(None, false)
            }
        } else {
            // no active window
            self.move_focus(None, false)
        }
    }

    #[must_use]
    fn move_focus(&mut self, new_focus: Option<WidgetPath>, highlight: bool) -> Option<FocusChangedArgs> {
        let prev_highlight = std::mem::replace(&mut self.is_highlighting, highlight);

        if self.focused != new_focus {
            let args = FocusChangedArgs::now(self.focused.take(), new_focus.clone(), self.is_highlighting);
            self.focused = new_focus;
            Some(args)
        } else if prev_highlight != highlight {
            Some(FocusChangedArgs::now(new_focus.clone(), new_focus, highlight))
        } else {
            None
        }
    }
}

impl AppService for Focus {}

#[derive(Clone, Copy, Debug)]
/// Focus change request.
pub struct FocusRequest {
    /// Where to move the focus.
    pub target: FocusTarget,
    /// If the widget should visually indicate that it is focused.
    pub highlight: bool,
}

impl FocusRequest {
    #[inline]
    pub fn new(target: FocusTarget, highlight: bool) -> Self {
        Self { target, highlight }
    }

    #[inline]
    pub fn direct(widget_id: WidgetId, highlight: bool) -> Self {
        Self::new(FocusTarget::Direct(widget_id), highlight)
    }

    #[inline]
    pub fn direct_or_parent(widget_id: WidgetId, highlight: bool) -> Self {
        Self::new(FocusTarget::DirectOrParent(widget_id), highlight)
    }

    #[inline]
    pub fn next(highlight: bool) -> Self {
        Self::new(FocusTarget::Next, highlight)
    }

    #[inline]
    pub fn prev(highlight: bool) -> Self {
        Self::new(FocusTarget::Prev, highlight)
    }

    #[inline]
    pub fn up(highlight: bool) -> Self {
        Self::new(FocusTarget::Up, highlight)
    }

    #[inline]
    pub fn right(highlight: bool) -> Self {
        Self::new(FocusTarget::Right, highlight)
    }

    #[inline]
    pub fn down(highlight: bool) -> Self {
        Self::new(FocusTarget::Down, highlight)
    }

    #[inline]
    pub fn left(highlight: bool) -> Self {
        Self::new(FocusTarget::Left, highlight)
    }
}

/// Focus request target.
#[derive(Clone, Copy, Debug)]
pub enum FocusTarget {
    /// Move focus to widget.
    Direct(WidgetId),
    /// Move focus to the widget if it is focusable or to a focusable parent.
    DirectOrParent(WidgetId),

    /// Move focus to next from current in screen, or to first in screen.
    Next,
    /// Move focus to previous from current in screen, or to last in screen.
    Prev,

    /// Move focus above current.
    Up,
    /// Move focus to the right of current.
    Right,
    /// Move focus bellow current.
    Down,
    /// Move focus to the left of current.
    Left,
}

/// A [`FrameInfo`] wrapper for querying focus info out of the widget tree.
#[derive(Copy, Clone)]
pub struct FrameFocusInfo<'a> {
    /// Full frame info.
    pub info: &'a FrameInfo,
}
impl<'a> FrameFocusInfo<'a> {
    #[inline]
    pub fn new(frame_info: &'a FrameInfo) -> Self {
        FrameFocusInfo { info: frame_info }
    }

    /// Reference to the root widget in the frame.
    ///
    /// The root is usually a focusable focus scope but it may not be. This
    /// is the only method that returns a [`WidgetFocusInfo`] that may not be focusable.
    #[inline]
    pub fn root(&self) -> WidgetFocusInfo {
        WidgetFocusInfo::new(self.info.root())
    }

    /// Reference to the widget in the frame, if it is present and is focusable.
    #[inline]
    pub fn find(&self, widget_id: WidgetId) -> Option<WidgetFocusInfo> {
        self.info.find(widget_id).and_then(|i| i.as_focusable())
    }

    /// If the frame info contains the widget and it is focusable.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.find(widget_id).is_some()
    }
}

/// [`WidgetInfo`] extensions that build a [`WidgetFocusInfo`].
pub trait WidgetInfoFocusExt<'a> {
    /// Wraps the [`WidgetInfo`] in a [`WidgetFocusInfo`] even if it is not focusable.
    fn as_focus_info(self) -> WidgetFocusInfo<'a>;

    /// Returns a wrapped [`WidgetFocusInfo`] if the [`WidgetInfo`] is focusable.
    fn as_focusable(self) -> Option<WidgetFocusInfo<'a>>;
}
impl<'a> WidgetInfoFocusExt<'a> for WidgetInfo<'a> {
    fn as_focus_info(self) -> WidgetFocusInfo<'a> {
        WidgetFocusInfo::new(self)
    }
    fn as_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        let r = self.as_focus_info();
        if r.is_focusable() {
            Some(r)
        } else {
            None
        }
    }
}

/// [`WidgetInfo`] wrapper that adds focus information for each widget.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct WidgetFocusInfo<'a> {
    /// Full widget info.
    pub info: WidgetInfo<'a>,
}
macro_rules! DirectionFn {
    (impl) => { impl Fn(LayoutPoint, LayoutPoint) -> (f32, f32, f32, f32) };
    (up) => { |from_pt, cand_c| (cand_c.y, from_pt.y, cand_c.x, from_pt.x) };
    (down) => { |from_pt, cand_c| (from_pt.y, cand_c.y, cand_c.x, from_pt.x) };
    (left) => { |from_pt, cand_c| (cand_c.x, from_pt.x, cand_c.y, from_pt.y) };
    (right) => { |from_pt, cand_c| (from_pt.x, cand_c.x, cand_c.y, from_pt.y) };
}
impl<'a> WidgetFocusInfo<'a> {
    #[inline]
    pub fn new(widget_info: WidgetInfo<'a>) -> Self {
        WidgetFocusInfo { info: widget_info }
    }

    /// Root focusable.
    #[inline]
    pub fn root(self) -> Self {
        self.ancestors().last().unwrap_or(self)
    }

    /// If the widget is focusable.
    ///
    /// ## Note
    ///
    /// This is probably `true`, the only way to get a [`WidgetFocusInfo`] for a non-focusable widget is by
    /// calling [`as_focus_info`](WidgetInfoFocusExt::as_focus_info) or explicitly constructing one.
    ///
    /// Focus scopes are also focusable.
    #[inline]
    pub fn is_focusable(self) -> bool {
        self.focus_info().is_focusable()
    }

    /// Is focus scope.
    #[inline]
    pub fn is_scope(self) -> bool {
        self.focus_info().is_scope()
    }

    /// Widget focus metadata.
    #[inline]
    pub fn focus_info(self) -> FocusInfo {
        let m = self.info.meta();
        match (
            m.get(IsFocusableKey).copied(),
            m.get(IsFocusScopeKey).copied(),
            m.get(TabIndexKey).copied(),
            m.get(TabNavKey).copied(),
            m.get(DirectionalNavKey).copied(),
        ) {
            // Set as not focusable.
            (Some(false), _, _, _, _) => FocusInfo::NotFocusable,

            // Set as focus scope and not set as not focusable
            // or set tab navigation and did not set as not focus scope
            // or set directional navigation and did not set as not focus scope.
            (_, Some(true), idx, tab, dir) | (_, None, idx, tab @ Some(_), dir) | (_, None, idx, tab, dir @ Some(_)) => {
                FocusInfo::FocusScope(
                    idx.unwrap_or(TabIndex::AUTO),
                    tab.unwrap_or(TabNav::Continue),
                    dir.unwrap_or(DirectionalNav::None),
                )
            }

            // Set as focusable and was not focus scope
            // or set tab index and was not focus scope and did not set as not focusable.
            (Some(true), _, idx, _, _) | (_, _, idx @ Some(_), _, _) => FocusInfo::Focusable(idx.unwrap_or(TabIndex::AUTO)),

            _ => FocusInfo::NotFocusable,
        }
    }

    /// Iterator over focusable parent -> grandparent -> .. -> root.
    #[inline]
    pub fn ancestors(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.ancestors().focusable()
    }

    /// Iterator over focus scopes parent -> grandparent -> .. -> root.
    #[inline]
    pub fn scopes(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.ancestors().filter_map(|i| {
            let i = i.as_focus_info();
            if i.is_scope() {
                Some(i)
            } else {
                None
            }
        })
    }

    /// Reference to the focusable parent that contains this widget.
    #[inline]
    pub fn parent(self) -> Option<WidgetFocusInfo<'a>> {
        self.ancestors().next()
    }

    /// Reference the focus scope parent that contains the widget.
    #[inline]
    pub fn scope(self) -> Option<WidgetFocusInfo<'a>> {
        self.scopes().next()
    }

    /// Iterator over the focusable widgets contained by this widget.
    #[inline]
    pub fn descendants(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.descendants().focusable()
    }

    /// Descendants sorted by TAB index.
    #[inline]
    pub fn descendants_sorted(self) -> Vec<WidgetFocusInfo<'a>> {
        let mut vec: Vec<_> = self.descendants().collect();
        vec.sort_by_key(|f| f.focus_info().tab_index());
        vec
    }

    /// Iterator over all focusable widgets in the same scope after this widget.
    #[inline]
    pub fn next_focusables(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();
        self.scope()
            .into_iter()
            .flat_map(|s| s.descendants())
            .skip_while(move |f| f.info.widget_id() != self_id)
            .skip(1)
    }

    /// Next focusable in the same scope after this widget.
    #[inline]
    pub fn next_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_focusables().next()
    }

    /// Next focusable in the same scope after this widget respecting the TAB index.
    ///
    /// If `self` is `TabIndex::SKIP` returns the next focusable in the same scope after this widget.
    ///
    /// If `self` is the last item in scope returns the sorted descendants of the parent scope.
    pub fn next_focusable_sorted(self) -> Result<WidgetFocusInfo<'a>, Vec<WidgetFocusInfo<'a>>> {
        let self_index = self.focus_info().tab_index();
        let mut siblings = self.scope().map(|s| s.descendants_sorted()).unwrap_or_default();

        if self_index == TabIndex::SKIP {
            // TAB from skip, goes to next in widget tree.
            return self.next_focusable().ok_or(siblings);
        }

        // binary search the same tab index gets any of the items with the same tab index.
        let i_same = siblings.binary_search_by_key(&self_index, |f| f.focus_info().tab_index()).unwrap();
        // so we do a linear search before and after to find `self`.
        let mut i = i_same;
        // before
        loop {
            if siblings[i] == self {
                return if i == siblings.len() - 1 {
                    // we are the last item.
                    Err(siblings)
                } else {
                    let r = siblings.swap_remove(i + 1);
                    if r.focus_info().tab_index() == TabIndex::SKIP {
                        // `i_same` was `self` and we are the last non-skip item.
                        Err(siblings)
                    } else {
                        Ok(r)
                    }
                };
            } else if i == 0 || siblings[i].focus_info().tab_index() != self_index {
                // did not find `self` before `i_same`
                break;
            } else {
                i -= 1;
            }
        }
        // after
        i = i_same + 1;
        while i < siblings.len() {
            if siblings[i] == self {
                return if i == siblings.len() - 1 {
                    // we are the last item.
                    Err(siblings)
                } else {
                    let r = siblings.swap_remove(i + 1);
                    if r.focus_info().tab_index() == TabIndex::SKIP {
                        // we are the last non-skip item.
                        Err(siblings)
                    } else {
                        Ok(r)
                    }
                };
            } else {
                debug_assert_eq!(
                    siblings[i].focus_info().tab_index(),
                    self_index,
                    "`self must be in sorted `siblings` and we did not find before `i_same``"
                );
                i += 1;
            }
        }

        Err(siblings)
    }

    /// Iterator over all focusable widgets in the same scope before this widget in reverse.
    #[inline]
    pub fn prev_focusables(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();

        let mut prev: Vec<_> = self
            .scope()
            .into_iter()
            .flat_map(|s| s.descendants())
            .take_while(move |f| f.info.widget_id() != self_id)
            .collect();

        prev.reverse();

        prev.into_iter()
    }

    /// Previous focusable in the same scope before this widget.
    #[inline]
    pub fn prev_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();

        self.scope()
            .and_then(move |s| s.descendants().take_while(move |f| f.info.widget_id() != self_id).last())
    }

    /// Previous focusable in the same scope before this widget respecting the TAB index.
    ///
    /// If `self` is `TabIndex::SKIP` returns the previous focusable in the same scope before this widget.
    ///
    /// If `self` is the first item in scope returns the sorted descendants of the parent scope.
    pub fn prev_focusable_sorted(self) -> Result<WidgetFocusInfo<'a>, Vec<WidgetFocusInfo<'a>>> {
        let self_index = self.focus_info().tab_index();
        let mut siblings = self.scope().map(|s| s.descendants_sorted()).unwrap_or_default();

        if self_index == TabIndex::SKIP {
            // TAB from skip, goes prev in widget tree.
            return self.prev_focusable().ok_or(siblings);
        }

        // binary search the same tab index gets any of the items with the same tab index.
        let i_same = siblings.binary_search_by_key(&self_index, |f| f.focus_info().tab_index()).unwrap();
        // so we do a linear search before and after to find `self`.
        let mut i = i_same;
        // before
        loop {
            if siblings[i] == self {
                return if i == 0 { Err(siblings) } else { Ok(siblings.swap_remove(i - 1)) };
            } else if i == 0 || siblings[i].focus_info().tab_index() != self_index {
                // did not find `self` before `i_same`
                break;
            } else {
                i -= 1;
            }
        }
        // after
        i = i_same + 1;
        while i < siblings.len() {
            if siblings[i] == self {
                return Ok(siblings.swap_remove(i - 1));
            } else {
                debug_assert_eq!(
                    siblings[i].focus_info().tab_index(),
                    self_index,
                    "`self must be in sorted `siblings` and we did not find before `i_same``"
                );
                i += 1;
            }
        }

        Err(siblings)
    }

    /// Widget to focus when pressing TAB from this widget.
    ///
    /// Returns `None` if the focus does not move to another widget.
    #[inline]
    pub fn next_tab(self) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => None,
                TabNav::Continue => self.next_focusable_sorted().ok().or_else(|| scope.next_tab()),
                TabNav::Contained => self.next_focusable_sorted().ok(),
                TabNav::Cycle => self
                    .next_focusable_sorted()
                    .or_else(|sorted_siblings| {
                        if let Some(first) = sorted_siblings.into_iter().find(|f| f.focus_info().tab_index() != TabIndex::SKIP) {
                            if first == self {
                                Err(())
                            } else {
                                Ok(first)
                            }
                        } else {
                            Err(())
                        }
                    })
                    .ok(),
                TabNav::Once => scope.next_tab(),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing SHIFT+TAB from this widget.
    ///
    /// Returns `None` if the focus does not move to another widget.
    #[inline]
    pub fn prev_tab(self) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => None,
                TabNav::Continue => self.prev_focusable_sorted().ok().or_else(|| scope.prev_tab()),
                TabNav::Contained => self.prev_focusable_sorted().ok(),
                TabNav::Cycle => self
                    .prev_focusable_sorted()
                    .or_else(|sorted_siblings| {
                        if let Some(last) = sorted_siblings.into_iter().rfind(|f| f.focus_info().tab_index() != TabIndex::SKIP) {
                            if last == self {
                                Err(())
                            } else {
                                Ok(last)
                            }
                        } else {
                            Err(())
                        }
                    })
                    .ok(),
                TabNav::Once => scope.prev_tab(),
            }
        } else {
            None
        }
    }

    fn directional_from_pt(
        scope: WidgetFocusInfo<'a>,
        from_pt: LayoutPoint,
        skip_id: WidgetId,
        direction: DirectionFn![impl],
    ) -> Option<WidgetFocusInfo<'a>> {
        let distance = move |other_pt: LayoutPoint| {
            let a = (other_pt.x - from_pt.x).powf(2.);
            let b = (other_pt.y - from_pt.y).powf(2.);
            a + b
        };

        let mut candidate_dist = f32::MAX;
        let mut candidate = None;

        for w in scope.descendants() {
            if w.info.widget_id() != skip_id {
                let candidate_center = w.info.center();

                let (a, b, c, d) = direction(from_pt, candidate_center);
                let mut is_in_direction = false;

                // for 'above' this is:
                // is above line?
                if a <= b {
                    // is to the right?
                    if c >= d {
                        // is in the 45º 'frustum'
                        // │?╱
                        // │╱__
                        is_in_direction = c <= d + (b - a);
                    } else {
                        //  ╲?│
                        // __╲│
                        is_in_direction = c >= d - (b - a);
                    }
                }

                if is_in_direction {
                    let dist = distance(candidate_center);
                    if dist < candidate_dist {
                        candidate = Some(w);
                        candidate_dist = dist;
                    }
                }
            }
        }

        candidate
    }

    fn directional_next(self, direction_vals: DirectionFn![impl]) -> Option<WidgetFocusInfo<'a>> {
        self.scope()
            .and_then(|s| Self::directional_from_pt(s, self.info.center(), s.info.widget_id(), direction_vals))
    }

    /// Closest focusable in the same scope above this widget.
    #[inline]
    pub fn focusable_above(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![up])
    }

    /// Closest focusable in the same scope below this widget.
    #[inline]
    pub fn focusable_below(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![down])
    }

    /// Closest focusable in the same scope to the left of this widget.
    #[inline]
    pub fn focusable_left(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![left])
    }

    /// Closest focusable in the same scope to the right of this widget.
    #[inline]
    pub fn focusable_right(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![right])
    }

    /// Widget to focus when pressing the arrow up key from this widget.
    #[inline]
    pub fn next_up(self) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_above().or_else(|| scope.focusable_above()),
                DirectionalNav::Contained => self.focusable_above(),
                DirectionalNav::Cycle => self.focusable_above().or_else(|| {
                    // next up from the same X but from the bottom segment of scope.
                    let mut from_pt = self.info.center();
                    from_pt.y = scope.info.bounds().max_y();
                    Self::directional_from_pt(scope, from_pt, self.info.widget_id(), DirectionFn![up])
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow right key from this widget.
    #[inline]
    pub fn next_right(self) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_right().or_else(|| scope.focusable_right()),
                DirectionalNav::Contained => self.focusable_right(),
                DirectionalNav::Cycle => self.focusable_right().or_else(|| {
                    // next right from the same Y but from the left segment of scope.
                    let mut from_pt = self.info.center();
                    from_pt.x = scope.info.bounds().min_x();
                    Self::directional_from_pt(scope, from_pt, self.info.widget_id(), DirectionFn![right])
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow down key from this widget.
    #[inline]
    pub fn next_down(self) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_below().or_else(|| scope.focusable_below()),
                DirectionalNav::Contained => self.focusable_below(),
                DirectionalNav::Cycle => self.focusable_below().or_else(|| {
                    // next down from the same X but from the top segment of scope.
                    let mut from_pt = self.info.center();
                    from_pt.y = scope.info.bounds().min_y();
                    Self::directional_from_pt(scope, from_pt, self.info.widget_id(), DirectionFn![down])
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow left key from this widget.
    #[inline]
    pub fn next_left(self) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_left().or_else(|| scope.focusable_left()),
                DirectionalNav::Contained => self.focusable_left(),
                DirectionalNav::Cycle => self.focusable_left().or_else(|| {
                    // next left from the same Y but from the right segment of scope.
                    let mut from_pt = self.info.center();
                    from_pt.x = scope.info.bounds().max_x();
                    Self::directional_from_pt(scope, from_pt, self.info.widget_id(), DirectionFn![left])
                }),
            }
        } else {
            None
        }
    }
}

/// Filter-maps an iterator of [`WidgetInfo`] to [`WidgetFocusInfo`].
pub trait IterFocusable<'a, I: Iterator<Item = WidgetInfo<'a>>> {
    fn focusable(self) -> std::iter::FilterMap<I, fn(WidgetInfo<'a>) -> Option<WidgetFocusInfo<'a>>>;
}
impl<'a, I: Iterator<Item = WidgetInfo<'a>>> IterFocusable<'a, I> for I {
    fn focusable(self) -> std::iter::FilterMap<I, fn(WidgetInfo<'a>) -> Option<WidgetFocusInfo<'a>>> {
        self.filter_map(|i| i.as_focusable())
    }
}

/// Focus metadata associated with a widget in a frame.
#[derive(Debug, Clone, Copy)]
pub enum FocusInfo {
    NotFocusable,
    Focusable(TabIndex),
    FocusScope(TabIndex, TabNav, DirectionalNav),
}

impl FocusInfo {
    /// If is focusable or a focus scope.
    #[inline]
    pub fn is_focusable(self) -> bool {
        match self {
            FocusInfo::NotFocusable => false,
            _ => true,
        }
    }

    /// If is a focus scope.
    #[inline]
    pub fn is_scope(self) -> bool {
        match self {
            FocusInfo::FocusScope(..) => true,
            _ => false,
        }
    }

    /// Tab navigation mode.
    ///
    /// | Variant                   | Returns                                 |
    /// |---------------------------|-----------------------------------------|
    /// | Focus scope               | Associated value, default is `Continue` |
    /// | Focusable                 | `TabNav::Continue`                      |
    /// | Not-Focusable             | `TabNav::None`                          |
    #[inline]
    pub fn tab_nav(self) -> TabNav {
        match self {
            FocusInfo::FocusScope(_, tab_nav, _) => tab_nav,
            FocusInfo::Focusable(_) => TabNav::Continue,
            FocusInfo::NotFocusable => TabNav::None,
        }
    }

    /// Directional navigation mode.
    ///
    /// | Variant                   | Returns                             |
    /// |---------------------------|-------------------------------------|
    /// | Focus scope               | Associated value, default is `None` |
    /// | Focusable                 | `DirectionalNav::Continue`          |
    /// | Not-Focusable             | `DirectionalNav::None`              |
    #[inline]
    pub fn directional_nav(self) -> DirectionalNav {
        match self {
            FocusInfo::FocusScope(_, _, dir_nav) => dir_nav,
            FocusInfo::Focusable(_) => DirectionalNav::Continue,
            FocusInfo::NotFocusable => DirectionalNav::None,
        }
    }

    /// Tab navigation index.
    ///
    /// | Variant           | Returns                                       |
    /// |-------------------|-----------------------------------------------|
    /// | Focusable & Scope | Associated value, default is `TabIndex::AUTO` |
    /// | Not-Focusable     | `TabIndex::SKIP`                              |
    #[inline]
    pub fn tab_index(self) -> TabIndex {
        match self {
            FocusInfo::Focusable(i) => i,
            FocusInfo::FocusScope(i, _, _) => i,
            FocusInfo::NotFocusable => TabIndex::SKIP,
        }
    }
}

use crate::core::*;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub static STOP_CLICK: ChildValueKeyRef<()> = ChildValueKey::new_lazy();
pub static STOP_MOUSE_DOWN: ChildValueKeyRef<()> = ChildValueKey::new_lazy();
pub static STOP_MOUSE_UP: ChildValueKeyRef<()> = ChildValueKey::new_lazy();
pub static STOP_MOUSE_MOVE: ChildValueKeyRef<()> = ChildValueKey::new_lazy();
pub static STOP_KEY_DOWN: ChildValueKeyRef<()> = ChildValueKey::new_lazy();
pub static STOP_KEY_UP: ChildValueKeyRef<()> = ChildValueKey::new_lazy();

#[derive(new)]
pub struct OnKeyDown<T: Ui, F: FnMut(KeyDown, &mut NextUpdate)> {
    child: T,
    handler: F,
}

#[impl_ui_crate(child)]
impl<T: Ui, F: FnMut(KeyDown, &mut NextUpdate)> OnKeyDown<T, F> {
    #[Ui]
    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.keyboard_input(input, values, update);

        if values.child(*STOP_KEY_DOWN).is_some() {
            return;
        }

        if let (ElementState::Pressed, Some(key), Some(focus)) =
            (input.state, input.virtual_keycode, self.child.focus_status())
        {
            let stop = Rc::default();
            let input = KeyDown {
                key,
                modifiers: input.modifiers,
                repeat: input.repeat,
                focus,
                stop_propagation: Rc::clone(&stop),
            };
            (self.handler)(input, update);
            if stop.get() {
                values.set_child_value(*STOP_KEY_DOWN, ());
            }
        }
    }
}

#[derive(new)]
pub struct OnKeyUp<T: Ui, F: FnMut(KeyUp, &mut NextUpdate)> {
    child: T,
    handler: F,
}

#[impl_ui_crate(child)]
impl<T: Ui, F: FnMut(KeyUp, &mut NextUpdate)> OnKeyUp<T, F> {
    #[Ui]
    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.keyboard_input(input, values, update);

        if values.child(*STOP_KEY_UP).is_some() {
            return;
        }

        if let (ElementState::Released, Some(key), Some(focus)) =
            (input.state, input.virtual_keycode, self.child.focus_status())
        {
            let stop = Rc::default();
            let input = KeyUp {
                key,
                modifiers: input.modifiers,
                focus,
                stop_propagation: Rc::clone(&stop),
            };
            (self.handler)(input, update);
            if stop.get() {
                values.set_child_value(*STOP_KEY_UP, ());
            }
        }
    }
}

pub trait KeyboardEvents: Ui + Sized {
    fn on_key_down<F: FnMut(KeyDown, &mut NextUpdate)>(self, handler: F) -> OnKeyDown<Self, F> {
        OnKeyDown::new(self, handler)
    }

    fn on_key_up<F: FnMut(KeyUp, &mut NextUpdate)>(self, handler: F) -> OnKeyUp<Self, F> {
        OnKeyUp::new(self, handler)
    }
}
impl<T: Ui + Sized> KeyboardEvents for T {}

macro_rules! on_mouse {
    ($state: ident, $name: ident, $stop_tag: expr) => {
        #[derive(Clone, new)]
        pub struct $name<T: Ui, F: FnMut(MouseButtonInput, &mut NextUpdate)> {
            child: T,
            handler: F,
        }

        #[impl_ui_crate(child)]
        impl<T: Ui + 'static, F: FnMut(MouseButtonInput, &mut NextUpdate)> $name<T, F> {
            #[Ui]
            fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
                self.child.mouse_input(input, hits, values, update);

                //child didn't stop propagation
                if values.child(*$stop_tag).is_none() {
                    if let (ElementState::$state, Some(position)) = (input.state, self.child.point_over(hits)) {
                        let stop = Rc::default();

                        let input = MouseButtonInput {
                            button: input.button,
                            modifiers: input.modifiers,
                            position,
                            stop_propagation: Rc::clone(&stop),
                        };
                        (self.handler)(input, update);

                        if stop.get() {
                            values.set_child_value(*$stop_tag, ());
                        }
                    }
                }
            }
        }
    };
}

on_mouse!(Pressed, OnMouseDown, STOP_MOUSE_DOWN);
on_mouse!(Released, OnMouseUp, STOP_MOUSE_UP);

#[derive(new)]
pub struct OnClick<T: Ui, F: FnMut(ClickInput, &mut NextUpdate)> {
    child: T,
    handler: F,
    #[new(default)]
    click_count: u8,
    #[new(value = "Instant::now() - Duration::from_secs(30)")]
    last_pressed: Instant,
}

#[impl_ui_crate(child)]
impl<T: Ui, F: FnMut(ClickInput, &mut NextUpdate)> OnClick<T, F> {
    fn call_handler(
        &mut self,
        input: &MouseInput,
        position: LayoutPoint,
        stop_propagation: Rc<Cell<bool>>,
        update: &mut NextUpdate,
    ) {
        let input = ClickInput {
            button: input.button,
            modifiers: input.modifiers,
            position,
            click_count: self.click_count,
            stop_propagation,
        };
        (self.handler)(input, update);
    }

    fn interaction_outside(&mut self) {
        self.click_count = 0;
        self.last_pressed -= Duration::from_secs(30);
    }

    #[Ui]
    fn window_focused(&mut self, _: bool, _: &mut UiValues, _: &mut NextUpdate) {
        self.interaction_outside();
    }

    #[Ui]
    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.mouse_input(input, hits, values, update);
        if values.child(*STOP_CLICK).is_some() {
            self.click_count = 0;
            return;
        }

        let stop = Rc::default();

        match input.state {
            ElementState::Pressed => {
                if let Some(position) = self.child.point_over(hits) {
                    self.click_count = self.click_count.saturating_add(1);

                    let now = Instant::now();

                    if self.click_count > 1 {
                        if (now - self.last_pressed) < multi_click_time_ms() {
                            self.call_handler(input, position, Rc::clone(&stop), update);
                        } else {
                            self.click_count = 1;
                        }
                    }
                    self.last_pressed = now;
                } else {
                    self.interaction_outside();
                }
            }
            ElementState::Released => {
                if self.click_count > 0 {
                    if let Some(position) = self.child.point_over(hits) {
                        if self.click_count == 1 {
                            self.call_handler(input, position, Rc::clone(&stop), update);
                        }
                    } else {
                        self.interaction_outside();
                    }
                }
            }
        }
        if stop.get() {
            values.set_child_value(*STOP_CLICK, ());
        }
    }
}

#[cfg(target_os = "windows")]
fn multi_click_time_ms() -> Duration {
    Duration::from_millis(u64::from(unsafe { winapi::um::winuser::GetDoubleClickTime() }))
}

#[cfg(not(target_os = "windows"))]
fn multi_click_time_ms() -> u32 {
    // https://stackoverflow.com/questions/50868129/how-to-get-double-click-time-interval-value-programmatically-on-linux
    // https://developer.apple.com/documentation/appkit/nsevent/1532495-mouseevent
    Duration::from_millis(500)
}

#[derive(Clone, new)]
pub struct OnMouseMove<T: Ui, F: FnMut(MouseMove, &mut NextUpdate)> {
    child: T,
    handler: F,
}

#[impl_ui_crate(child)]
impl<T: Ui + 'static, F: FnMut(MouseMove, &mut NextUpdate)> OnMouseMove<T, F> {
    #[Ui]
    fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.mouse_move(input, hits, values, update);

        if values.child(*STOP_MOUSE_MOVE).is_some() {
            return;
        }

        if let Some(position) = self.child.point_over(hits) {
            let stop = Rc::default();
            (self.handler)(
                MouseMove {
                    position,
                    modifiers: input.modifiers,
                    stop_propagation: Rc::clone(&stop),
                },
                update,
            );

            if stop.get() {
                values.set_child_value(*STOP_MOUSE_MOVE, ());
            }
        }
    }
}

macro_rules! on_mouse_enter_leave {
    ($Type: ident, $mouse_over: ident, $if_mouse_over: expr) => {
        #[derive(new)]
        pub struct $Type<T: Ui, F: FnMut(&mut NextUpdate)> {
            child: T,
            handler: F,
            #[new(default)]
            mouse_over: bool,
        }

        #[impl_ui_crate(child)]
        impl<T: Ui, F: FnMut(&mut NextUpdate)> $Type<T, F> {
            fn set_mouse_over(&mut self, $mouse_over: bool, update: &mut NextUpdate) {
                if self.mouse_over != $mouse_over {
                    self.mouse_over = $mouse_over;
                    if $if_mouse_over {
                        (self.handler)(update);
                    }
                }
            }

            #[Ui]
            fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
                self.child.mouse_move(input, hits, values, update);
                self.set_mouse_over(self.child.point_over(hits).is_some(), update);
            }

            #[Ui]
            fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
                self.child.mouse_left(values, update);
                self.set_mouse_over(false, update);
            }
        }
    };
}
on_mouse_enter_leave!(OnMouseEnter, mouse_over, mouse_over);
on_mouse_enter_leave!(OnMouseLeave, mouse_over, !mouse_over);

pub trait MouseEvents: Ui + Sized {
    fn on_mouse_down<F: FnMut(MouseButtonInput, &mut NextUpdate)>(self, handler: F) -> OnMouseDown<Self, F> {
        OnMouseDown::new(self, handler)
    }

    fn on_mouse_up<F: FnMut(MouseButtonInput, &mut NextUpdate)>(self, handler: F) -> OnMouseUp<Self, F> {
        OnMouseUp::new(self, handler)
    }

    fn on_click<F: FnMut(ClickInput, &mut NextUpdate)>(self, handler: F) -> OnClick<Self, F> {
        OnClick::new(self, handler)
    }

    fn on_mouse_move<F: FnMut(MouseMove, &mut NextUpdate)>(self, handler: F) -> OnMouseMove<Self, F> {
        OnMouseMove::new(self, handler)
    }

    fn on_mouse_enter<F: FnMut(&mut NextUpdate)>(self, handler: F) -> OnMouseEnter<Self, F> {
        OnMouseEnter::new(self, handler)
    }

    fn on_mouse_leave<F: FnMut(&mut NextUpdate)>(self, handler: F) -> OnMouseLeave<Self, F> {
        OnMouseLeave::new(self, handler)
    }
}
impl<T: Ui + Sized> MouseEvents for T {}

macro_rules! on_focus_events {
    ($Type:ident, $focused: ident, $if_focused: expr) => {
        #[derive(new)]
        pub struct $Type<T: Ui, F: FnMut(&mut NextUpdate)> {
            child: T,
            handler: F,
            #[new(default)]
            was_focused: bool,
        }

        #[impl_ui_crate(child)]
        impl<T: Ui + 'static, F: FnMut(&mut NextUpdate)> Ui for $Type<T, F> {
            fn focus_changed(&mut self, change: &FocusChange, values: &mut UiValues, update: &mut NextUpdate) {
                self.child.focus_changed(change, values, update);

                let $focused = self.child.focus_status() == Some(FocusStatus::Focused);

                if $if_focused && self.was_focused != $focused {
                    (self.handler)(update);
                }

                self.was_focused = $focused;
            }
        }
    };
}

macro_rules! on_focus_within_events {
    ($Type:ident, $is_in: ident, $if_is_in: expr) => {
        #[derive(new)]
        pub struct $Type<T: Ui, F: FnMut(&mut NextUpdate)> {
            child: T,
            handler: F,
            #[new(default)]
            was_in: bool,
        }

        #[impl_ui_crate(child)]
        impl<T: Ui + 'static, F: FnMut(&mut NextUpdate)> Ui for $Type<T, F> {
            fn focus_changed(&mut self, change: &FocusChange, values: &mut UiValues, update: &mut NextUpdate) {
                self.child.focus_changed(change, values, update);

                let $is_in = self.child.focus_status().is_some();

                if $if_is_in && self.was_in != $is_in {
                    (self.handler)(update);
                }

                self.was_in = $is_in;
            }
        }
    };
}

on_focus_events!(OnFocus, focused, focused);
on_focus_events!(OnBlur, focused, !focused);

on_focus_within_events!(OnFocusEnter, focused, focused);
on_focus_within_events!(OnFocusLeave, focused, !focused);

pub trait FocusEvents: Ui + Sized {
    fn on_focus<F: FnMut(&mut NextUpdate)>(self, handler: F) -> OnFocus<Self, F> {
        OnFocus::new(self, handler)
    }

    fn on_blur<F: FnMut(&mut NextUpdate)>(self, handler: F) -> OnBlur<Self, F> {
        OnBlur::new(self, handler)
    }

    fn on_focus_enter<F: FnMut(&mut NextUpdate)>(self, handler: F) -> OnFocusEnter<Self, F> {
        OnFocusEnter::new(self, handler)
    }

    fn on_focus_leave<F: FnMut(&mut NextUpdate)>(self, handler: F) -> OnFocusLeave<Self, F> {
        OnFocusLeave::new(self, handler)
    }
}
impl<T: Ui + Sized> FocusEvents for T {}

#[derive(Debug)]
pub struct KeyDown {
    pub key: VirtualKeyCode,
    pub modifiers: ModifiersState,
    pub repeat: bool,
    pub focus: FocusStatus,
    stop_propagation: Rc<Cell<bool>>,
}

impl KeyDown {
    pub fn stop_propagation(&self) {
        self.stop_propagation.set(true);
    }
}

#[derive(Debug)]
pub struct KeyUp {
    pub key: VirtualKeyCode,
    pub modifiers: ModifiersState,
    pub focus: FocusStatus,
    stop_propagation: Rc<Cell<bool>>,
}

impl KeyUp {
    pub fn stop_propagation(&self) {
        self.stop_propagation.set(true);
    }
}

#[derive(Debug, Clone)]
pub struct MouseMove {
    pub position: LayoutPoint,
    pub modifiers: ModifiersState,
    stop_propagation: Rc<Cell<bool>>,
}
impl MouseMove {
    pub fn stop_propagation(&self) {
        self.stop_propagation.set(true);
    }
}

fn display_modifiers(m: ModifiersState, f: &mut fmt::Formatter) -> fmt::Result {
    if m.ctrl {
        write!(f, "Ctrl + ")?;
    }
    if m.alt {
        write!(f, "Alt + ")?;
    }
    if m.shift {
        write!(f, "Shift + ")?;
    }
    if m.logo {
        write!(f, "Logo + ")?;
    }

    Ok(())
}

impl fmt::Display for KeyDown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.key {
            VirtualKeyCode::LControl
            | VirtualKeyCode::RControl
            | VirtualKeyCode::LShift
            | VirtualKeyCode::RShift
            | VirtualKeyCode::LAlt
            | VirtualKeyCode::RAlt => write!(f, "{:?}", self.key)?,
            _ => {
                display_modifiers(self.modifiers, f)?;
                write!(f, "{:?}", self.key)?;
            }
        }

        if self.repeat && self.focus == FocusStatus::FocusWithin {
            write!(f, " (repeat, focus-within)")?;
        } else if self.repeat {
            write!(f, " (repeat)")?;
        } else if self.focus == FocusStatus::FocusWithin {
            write!(f, " (focus-within)")?;
        }

        Ok(())
    }
}

impl fmt::Display for KeyUp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.key {
            VirtualKeyCode::LControl
            | VirtualKeyCode::RControl
            | VirtualKeyCode::LShift
            | VirtualKeyCode::RShift
            | VirtualKeyCode::LAlt
            | VirtualKeyCode::RAlt => write!(f, "{:?}", self.key)?,
            _ => {
                display_modifiers(self.modifiers, f)?;
                write!(f, "{:?}", self.key)?;
            }
        }

        if self.focus == FocusStatus::FocusWithin {
            write!(f, " (focus-within)")?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub modifiers: ModifiersState,
    pub position: LayoutPoint,
    stop_propagation: Rc<Cell<bool>>,
}

impl MouseButtonInput {
    pub fn stop_propagation(&self) {
        self.stop_propagation.set(true);
    }
}

#[derive(Debug)]
pub struct ClickInput {
    pub button: MouseButton,
    pub modifiers: ModifiersState,
    pub position: LayoutPoint,
    pub click_count: u8,
    stop_propagation: Rc<Cell<bool>>,
}
impl ClickInput {
    pub fn stop_propagation(&self) {
        self.stop_propagation.set(true);
    }
}

impl fmt::Display for MouseButtonInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_modifiers(self.modifiers, f)?;
        write!(f, "{:?} {}", self.button, self.position)
    }
}

impl fmt::Display for ClickInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_modifiers(self.modifiers, f)?;
        write!(f, "{:?} {}", self.button, self.position)?;
        match self.click_count {
            0..=1 => {}
            2 => write!(f, " double-click")?,
            3 => write!(f, " triple-click")?,
            n => write!(f, " click_count={}", n)?,
        }
        Ok(())
    }
}

//! Sub-menu popup widget and properties.

use zero_ui_core::focus::WidgetInfoFocusExt;

use crate::{
    core::{
        focus::{FOCUS, FOCUS_CHANGED_EVENT},
        gesture::{CommandShortcutExt, Shortcuts},
        keyboard::{Key, KeyState, KEY_INPUT_EVENT},
        timer::TIMERS,
        widget_instance::ArcNodeList,
    },
    prelude::popup::{PopupCloseMode, POPUP},
};

use crate::prelude::{
    button,
    new_widget::*,
    popup::{POPUP_CLOSE_CMD, POPUP_CLOSE_REQUESTED_EVENT},
    scroll,
};

use super::sub::{SubMenuWidgetInfoExt, HOVER_OPEN_DELAY_VAR};

/// Sub-menu popup.
#[widget($crate::widgets::menu::popup::SubMenuPopup)]
pub struct SubMenuPopup(crate::widgets::popup::Popup);
impl SubMenuPopup {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;

            // Supports press-and-drag to click gesture:
            //
            // - Sub-menu is `capture_pointer = true`.
            // - Menu items set`click_mode = release`.
            //
            // So the user can press to open the menu, then drag over an item and release to click it.
            capture_pointer_on_init = crate::core::pointer_capture::CaptureMode::Subtree;
        }

        self.widget_builder().push_build_action(|wgt| {
            let id = wgt.capture_value::<WidgetId>(property_id!(Self::parent_id));
            let children = wgt
                .capture_property(property_id!(Self::children))
                .map(|p| p.args.ui_node_list(0).clone())
                .unwrap_or_else(|| ArcNodeList::new(ui_vec![].boxed()));

            wgt.set_child(sub_menu_popup_node(children, id));
        });
    }

    widget_impl! {
        /// Sub-menu items.
        pub crate::core::widget_base::children(children: impl UiNodeList);
    }
}

/// Parent sub-menu ID.
#[property(CONTEXT, capture, widget_impl(SubMenuPopup))]
pub fn parent_id(submenu_id: impl IntoValue<WidgetId>) {}

context_var! {
    /// Defines the layout widget for [`SubMenuPopup!`].
    ///
    /// Is [`default_panel_fn`] by default.
    ///
    /// [`SubMenuPopup!`]: struct@SubMenuPopup
    pub static PANEL_FN_VAR: WidgetFn<panel::PanelArgs> = WidgetFn::new(default_panel_fn);

    /// Sub-menu popup style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());
}

/// Widget function that generates the sub-menu popup layout.
///
/// This property sets [`PANEL_FN_VAR`].
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(SubMenuPopup))]
pub fn panel_fn(child: impl UiNode, panel: impl IntoVar<WidgetFn<panel::PanelArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_FN_VAR, panel)
}

/// Sets the sub-menu popup style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the sub-menu popup style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Sub-menu popup default style.
#[widget($crate::widgets::menu::popup::DefaultStyle)]
pub struct DefaultStyle(crate::widgets::popup::DefaultStyle);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            super::sub::replace_style = super::sub::SubMenuStyle!();

            background_color = color_scheme_pair(button::BASE_COLORS_VAR);
            border = {
                widths: 1,
                sides: button::color_scheme_hovered(button::BASE_COLORS_VAR).map_into(),
            };
        }
    }
}

/// Default sub-menu popup panel view.
///
/// See [`PANEL_FN_VAR`] for more details.
pub fn default_panel_fn(args: panel::PanelArgs) -> impl UiNode {
    // remove arrow key shortcuts, they are used to nav. focus.
    let scroll_id = WidgetId::new_unique();
    let _ = scroll::commands::SCROLL_UP_CMD.scoped(scroll_id).shortcut().set(Shortcuts::new());
    let _ = scroll::commands::SCROLL_DOWN_CMD.scoped(scroll_id).shortcut().set(Shortcuts::new());

    crate::widgets::Scroll! {
        id = scroll_id;
        focusable = false;
        child = crate::widgets::layouts::Stack! {
            children = args.children;
            direction = crate::widgets::layouts::stack::StackDirection::top_to_bottom();
        };
        mode = scroll::ScrollMode::VERTICAL;
    }
}

/// Sub-menu popup implementation.
pub fn sub_menu_popup_node(children: ArcNodeList<BoxedUiNodeList>, parent: Option<WidgetId>) -> impl UiNode {
    let child = crate::widgets::layouts::panel::node(
        children,
        if parent.is_none() {
            super::context::PANEL_FN_VAR
        } else {
            PANEL_FN_VAR
        },
    );
    let mut close_timer = None;
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_event(&KEY_INPUT_EVENT)
                .sub_event(&POPUP_CLOSE_REQUESTED_EVENT)
                .sub_event(&FOCUS_CHANGED_EVENT);
        }
        UiNodeOp::Deinit => {
            close_timer = None;
        }
        UiNodeOp::Info { info } => {
            // sub-menus set the popup as parent in context menu.
            super::sub::SUB_MENU_PARENT_CTX.with_context_value(Some(parent.unwrap_or_else(|| WIDGET.id())), || c.info(info));
            info.set_meta(&super::sub::SUB_MENU_POPUP_ID, super::sub::SubMenuPopupInfo { parent });
        }
        UiNodeOp::Event { update } => {
            c.event(update);

            if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                if let KeyState::Pressed = args.state {
                    match &args.key {
                        Key::Escape => {
                            let info = WIDGET.info();
                            if let Some(m) = info.submenu_parent() {
                                args.propagation().stop();

                                FOCUS.focus_widget(m.id(), true);
                                POPUP.force_close(info.id());
                            }
                        }
                        Key::ArrowLeft | Key::ArrowRight => {
                            if let Some(info) = WINDOW.info().get(args.target.widget_id()) {
                                let info = info.into_focus_info(true, true);
                                if info.focusable_left().is_none() && info.focusable_right().is_none() {
                                    // escape to parent or change root.
                                    if let Some(m) = info.info().submenu_parent() {
                                        let mut escape = false;
                                        if m.submenu_parent().is_some() {
                                            if let Some(o) = m.orientation_from(info.info().center()) {
                                                escape = match o {
                                                    Orientation2D::Left => args.key == Key::ArrowLeft,
                                                    Orientation2D::Right => args.key == Key::ArrowRight,
                                                    Orientation2D::Below | Orientation2D::Above => false,
                                                };
                                            }
                                        }

                                        if escape {
                                            args.propagation().stop();
                                            // escape

                                            FOCUS.focus_widget(m.id(), true);
                                            POPUP.force_close(WIDGET.id());
                                        } else if let Some(m) = info.info().submenu_root() {
                                            args.propagation().stop();
                                            // change root

                                            let m = m.into_focus_info(true, true);
                                            let next_root = match &args.key {
                                                Key::ArrowLeft => m.next_left(),
                                                Key::ArrowRight => m.next_right(),
                                                _ => unreachable!(),
                                            };
                                            if let Some(n) = next_root {
                                                FOCUS.focus_widget(n.info().id(), true);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            } else if let Some(args) = POPUP_CLOSE_REQUESTED_EVENT.on_unhandled(update) {
                let sub_self = if parent.is_some() {
                    WIDGET.info().submenu_parent()
                } else {
                    // is context menu
                    Some(WIDGET.info())
                };
                if let Some(sub_self) = sub_self {
                    let mut close_ancestors = Some(None);

                    if let Some(focused) = FOCUS.focused().get() {
                        if let Some(focused) = sub_self.tree().get(focused.widget_id()) {
                            if let Some(sub_focused) = focused.submenu_parent() {
                                if sub_focused.submenu_ancestors().any(|a| a.id() == sub_self.id()) {
                                    // keep open, focused child.
                                    args.propagation().stop();
                                    close_ancestors = None;
                                } else if sub_self.submenu_ancestors().any(|a| a.id() == sub_focused.id()) {
                                    if Some(sub_focused.id()) == sub_self.submenu_parent().map(|s| s.id()) {
                                        // keep open, focused parent.
                                        args.propagation().stop();
                                        close_ancestors = None;
                                    } else {
                                        close_ancestors = Some(Some(sub_focused.id()));
                                    }
                                }
                            }
                        }
                    }

                    if let Some(sub_parent_focused) = close_ancestors {
                        // close any parent sub-menu that is not focused.
                        for a in sub_self.submenu_ancestors() {
                            if Some(a.id()) == sub_parent_focused {
                                break;
                            }

                            if let Some(v) = a.is_submenu_open() {
                                if v.get() {
                                    // request ancestor close the popup.
                                    POPUP_CLOSE_CMD.scoped(a.id()).notify();
                                }
                            } else if a.menu().is_none() {
                                // request context menu popup close
                                POPUP_CLOSE_CMD.scoped(a.id()).notify_param(PopupCloseMode::Force);
                            }
                        }
                    }
                }
            } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                if args.is_focus_leave(WIDGET.id()) {
                    if let Some(f) = &args.new_focus {
                        let info = WIDGET.info();
                        let sub_self = if parent.is_some() {
                            info.submenu_parent()
                        } else {
                            // is context menu
                            Some(info.clone())
                        };
                        if let (Some(sub_menu), Some(f)) = (sub_self, info.tree().get(f.widget_id())) {
                            if !f.submenu_self_and_ancestors().any(|s| s.id() == sub_menu.id()) {
                                // Focus did not move to child sub-menu nor parent,
                                // close after delay.
                                //
                                // This covers the case of focus moving to an widget that is not
                                // a child sub-menu and is not the parent sub-menu,
                                // `sub_menu_node` covers the case of focus moving to the parent sub-menu and out.
                                let t = TIMERS.deadline(HOVER_OPEN_DELAY_VAR.get());
                                t.subscribe(UpdateOp::Update, info.id()).perm();
                                close_timer = Some(t);
                            }
                        }
                    }
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(t) = &close_timer {
                if t.get().has_elapsed() {
                    close_timer = None;
                    POPUP.force_close(WIDGET.id());
                }
            }
        }
        _ => {}
    })
}

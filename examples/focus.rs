#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::focus::{FocusChangedEvent, FocusRequest, FocusTarget};
use zero_ui::prelude::*;
use zero_ui::widgets::window::{LayerIndex, WindowLayers};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-focus.json.gz", &[("example", &"focus")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        ctx.window_id.set_name("main").unwrap();

        trace_focus(ctx.events);
        let window_enabled = var(true);
        window! {
            title = "Focus Example";
            enabled = window_enabled.clone();
            content = v_stack! {
                items = widgets![
                    alt_scope(),
                    h_stack! {
                        margin = (50, 0, 0, 0);
                        align = Align::CENTER;
                        spacing = 10;
                        items = widgets![
                            tab_index(),
                            functions(window_enabled),
                            delayed_focus(),
                        ]
                    }
                ];
            };
            // zero_ui::widgets::inspector::show_center_points = true;
            // zero_ui::widgets::inspector::show_bounds = true;
        }
    })
}

fn alt_scope() -> impl Widget {
    h_stack! {
        alt_focus_scope = true;
        button::theme::border = 0, BorderStyle::Solid;
        button::theme::corner_radius = 0;
        items = widgets![
            button("alt", TabIndex::AUTO),
            button("scope", TabIndex::AUTO),
        ];
    }
}

fn tab_index() -> impl Widget {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(T);
        items = widgets![
            title("TabIndex (T)"),
            button("Button A5", 5),
            button("Button A4", 3),
            button("Button A3", 2),
            button("Button A1", 0),
            button("Button A2", 0),
        ];
    }
}

fn functions(window_enabled: RcVar<bool>) -> impl Widget {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(F);
        items = widgets![
            title("Functions (F)"),
            // New Window
            button! {
                content = text("New Window");
                on_click = hn!(|ctx, _| {
                    ctx.services.windows().open(|ctx| {
                        let _ = ctx.window_id.set_name("other");
                        window! {
                            title = "Other Window";
                            focus_shortcut = shortcut!(W);
                            content = v_stack! {
                                align = Align::CENTER;
                                spacing = 5;
                                items = widgets![
                                    title("Other Window (W)"),
                                    button("Button B5", 5),
                                    button("Button B4", 3),
                                    button("Button B3", 2),
                                    button("Button B1", 0),
                                    button("Button B2", 0),
                                ]
                            };
                        }
                    });
                });
            },
            // Detach Button
            {
                let detach_focused = RcNode::new_cyclic(|wk| {
                    let btn = button! {
                        content = text("Detach Button");
                        on_click = hn!(|ctx, _| {
                            let wwk = wk.clone();
                            ctx.services.windows().open(move |_| {
                                window! {
                                    title = "Detached Button";
                                    content_align = Align::CENTER;
                                    content = slot(wwk.upgrade().unwrap(), slot::take_on_init());
                                }
                            });
                        });
                    };
                    btn.boxed()
                });
                slot(detach_focused, slot::take_on_init())
            },
            // Disable Window
            disable_window(window_enabled.clone()),
            // Overlay Scope
            button! {
                content = text("Overlay Scope");
                on_click = hn!(|ctx, _| {
                    WindowLayers::insert(ctx, LayerIndex::TOP_MOST, overlay(window_enabled.clone()));
                });
            },
        ]
    }
}
fn disable_window(window_enabled: RcVar<bool>) -> impl Widget {
    button! {
        content = text(window_enabled.map(|&e| if e { "Disable Window" } else { "Enabling in 1s..." }.into()));
        width = 140;
        on_click = async_hn!(window_enabled, |ctx, _| {
            window_enabled.set(&ctx, false);
            task::timeout(1.secs()).await;
            window_enabled.set(&ctx, true);
        });
    }
}
fn overlay(window_enabled: RcVar<bool>) -> impl Widget {
    container! {
        id = "overlay";
        modal = true;
        content_align = Align::CENTER;
        content = container! {
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            background_color = rgb(0.05, 0.05, 0.05);
            drop_shadow = (0, 0), 4, colors::BLACK;
            padding = 2;
            content = v_stack! {
                items_align = Align::RIGHT;
                items = widgets![
                    text! {
                        text = "Window scope is disabled so the overlay scope is the root scope.";
                        margin = 15;
                    },
                    h_stack! {
                        spacing = 2;
                        items = widgets![
                        disable_window(window_enabled),
                        button! {
                                content = text("Close");
                                on_click = hn!(|ctx, _| {
                                    WindowLayers::remove(ctx, "overlay");
                                })
                            }
                        ]
                    }
                ]
            }
        }
    }
}

fn delayed_focus() -> impl Widget {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(D);
        items = widgets![
            title("Delayed 4s (D)"),

            delayed_btn("Force Focus", |ctx| {
                ctx.services.focus().focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: true,
                    window_indicator: None,
                });
            }),
            delayed_btn("Info Indicator", |ctx| {
                ctx.services.focus().focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: false,
                    window_indicator: Some(FocusIndicator::Info),
                });
            }),
            delayed_btn("Critical Indicator", |ctx| {
                ctx.services.focus().focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: false,
                    window_indicator: Some(FocusIndicator::Critical),
                });
            }),

            text! {
                id = "target";
                padding = zero_ui::widgets::button::theme::PaddingVar;
                corner_radius = zero_ui::widgets::button::theme::CornerRadiusVar;
                text = "delayed target";
                font_style = FontStyle::Italic;
                text_align = TextAlign::CENTER_MIDDLE;
                background_color = rgb(30, 30, 30);

                focusable = true;
                when self.is_focused {
                    text = "focused";
                    background_color = colors::DARK_GREEN;
                }
            },
        ]
    }
}
fn delayed_btn(content: impl Into<Text>, on_timeout: impl FnMut(&mut WidgetContext) + 'static) -> impl Widget {
    let on_timeout = std::rc::Rc::new(std::cell::RefCell::new(Box::new(on_timeout)));
    let enabled = var(true);
    button! {
        content = text(content.into());
        on_click = async_hn!(enabled, on_timeout, |ctx, _| {
            enabled.set(&ctx, false);
            task::timeout(4.secs()).await;
            ctx.with(|ctx| {
                let mut on_timeout = on_timeout.borrow_mut();
                on_timeout(ctx);
            });
            enabled.set(&ctx, true);
        });
        enabled;
    }
}

fn title(text: impl IntoVar<Text>) -> impl Widget {
    text! { text; font_weight = FontWeight::BOLD; align = Align::CENTER; }
}

fn button(content: impl Into<Text>, tab_index: impl Into<TabIndex>) -> impl Widget {
    let content = content.into();
    let tab_index = tab_index.into();
    button! {
        content = text(content.clone());
        tab_index;
        on_click = hn!(|_, _| {
            println!("Clicked {content} {tab_index:?}")
        });
    }
}

fn trace_focus(events: &mut Events) {
    events
        .on_pre_event(
            FocusChangedEvent,
            app_hn!(|ctx, args: &FocusChangedArgs, _| {
                if args.is_hightlight_changed() {
                    println!("highlight: {}", args.highlight);
                } else if args.is_widget_move() {
                    println!("focused {:?} moved", args.new_focus.as_ref().unwrap());
                } else if args.is_enabled_change() {
                    println!("focused {:?} enabled/disabled", args.new_focus.as_ref().unwrap());
                } else {
                    println!(
                        "{} -> {}",
                        inspect::focus(&args.prev_focus, ctx.services),
                        inspect::focus(&args.new_focus, ctx.services)
                    );
                }
            }),
        )
        .perm();
}

#[cfg(debug_assertions)]
mod inspect {
    use super::*;
    use zero_ui::core::focus::WidgetInfoFocusExt;
    use zero_ui::core::inspector::WidgetInspectorInfo;

    pub fn focus(path: &Option<InteractionPath>, services: &mut Services) -> String {
        path.as_ref()
            .map(|p| {
                let frame = if let Ok(w) = services.windows().widget_tree(p.window_id()) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let widget = if let Some(w) = frame.get(p) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let info = widget.instance().expect("expected debug info").borrow();

                if info.widget_name == "button" {
                    format!(
                        "button({})",
                        widget
                            .descendant_instance("text")
                            .expect("expected text in button")
                            .property("text")
                            .expect("expected text property")
                            .borrow()
                            .arg(0)
                            .value
                            .debug
                    )
                } else if info.widget_name == "window" {
                    let title = widget
                        .properties()
                        .iter()
                        .find(|p| p.borrow().property_name == "title")
                        .map(|p| p.borrow().args[0].value.debug.clone())
                        .unwrap_or_default();
                    format!("window({title})")
                } else {
                    let focus_info = widget.as_focus_info(true);
                    if focus_info.is_alt_scope() {
                        format!("{}(is_alt_scope)", info.widget_name)
                    } else if focus_info.is_scope() {
                        format!("{}(is_scope)", info.widget_name)
                    } else {
                        info.widget_name.to_owned()
                    }
                }
            })
            .unwrap_or_else(|| "<none>".to_owned())
    }
}

#[cfg(not(debug_assertions))]
mod inspect {
    use super::*;

    pub fn focus(path: &Option<WidgetPath>, _: &mut Services) -> String {
        path.as_ref()
            .map(|p| format!("{:?}", p.widget_id()))
            .unwrap_or_else(|| "<none>".to_owned())
    }
}

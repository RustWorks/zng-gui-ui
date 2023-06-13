#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::text::{UnderlinePosition, UnderlineSkip, FONTS};
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("text");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        let fs = var(Length::Pt(11.0));

        Window! {
            zero_ui::core::widget_base::parallel = false;
            title = fs.map(|s| formatx!("Text Example - font_size: {s}"));
            child = z_stack(ui_vec![
                Stack! {
                    font_size = fs.easing(150.ms(), easing::linear);
                    direction = StackDirection::left_to_right();
                    align = Align::CENTER;
                    spacing = 40;
                    children = ui_vec![
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![
                                basic(),
                                defaults(),
                            ];
                        },
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![
                                line_height(),
                                line_spacing(),
                                word_spacing(),
                                letter_spacing(),
                            ];
                        },
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![
                                decoration_lines(),
                            ]
                        }
                    ];
                },
                Container! {
                    align = Align::TOP;
                    margin = 10;
                    child = font_size(fs);
                },
                Container! {
                    align = Align::BOTTOM_RIGHT;
                    margin = 20;
                    child = text_editor();
                }
            ])
        }
    })
}

fn font_size(font_size: ArcVar<Length>) -> impl UiNode {
    fn change_size(font_size: &ArcVar<Length>, change: f32) {
        font_size.modify(move |s| {
            *s.to_mut() += Length::Pt(change);
        });
    }
    Stack! {
        button::extend_style = Style! { padding = (0, 5) };
        direction = StackDirection::left_to_right();
        spacing = 5;
        corner_radius = 4;
        background_color = color_scheme_map(rgba(0, 0, 0, 40.pct()), rgba(1., 1., 1., 40.pct()));
        padding = 4;
        children = ui_vec![
            Button! {
                child = Text!("-");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!(Minus), shortcut!(NumpadSubtract)];
                on_click = hn!(font_size, |_| {
                    change_size(&font_size, -1.0)
                });
            },
            Text! {
                txt = font_size.map(|s| formatx!("{s}"));
            },
            Button! {
                child = Text!("+");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!(Plus), shortcut!(NumpadAdd)];
                on_click = hn!(font_size, |_| {
                    change_size(&font_size, 1.0)
                });
            },
        ]
    }
}

fn basic() -> impl UiNode {
    section(
        "basic",
        ui_vec![
            Text!("Basic Text"),
            Strong!("Strong Text"),
            Em!("Emphasis Text"),
            Text! {
                txt_color = color_scheme_map(colors::LIGHT_GREEN, colors::DARK_GREEN);
                txt = "Colored Text";

                when *#is_hovered {
                    txt_color = color_scheme_map(colors::YELLOW, colors::BROWN);
                }
            },
        ],
    )
}

fn line_height() -> impl UiNode {
    section(
        "line_height",
        ui_vec![
            Text! {
                txt = "Default: 'Émp Giga Ç'";
                background_color = colors::LIGHT_BLUE;
                txt_color = colors::BLACK;
            },
            Text! {
                txt = "150%: 'Émp Giga Ç'";
                background_color = colors::LIGHT_BLUE;
                txt_color = colors::BLACK;
                line_height = 150.pct();
            },
        ],
    )
}

fn line_spacing() -> impl UiNode {
    section(
        "line_spacing",
        ui_vec![Container! {
            child = Text! {
                txt = "Hello line 1!\nHello line 2!\nHover to change `line_spacing`";
                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                txt_wrap = false;

                when *#is_hovered {
                    #[easing(150.ms())]
                    line_spacing = 30.pct();
                }
            };
            child_align = Align::TOP;
            min_height = 1.7.em() * 3.fct();
        }],
    )
}

fn word_spacing() -> impl UiNode {
    section(
        "word_spacing",
        ui_vec![Text! {
            txt = "Word spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when *#is_hovered {
                #[easing(150.ms())]
                word_spacing = 100.pct();
            }
        }],
    )
}

fn letter_spacing() -> impl UiNode {
    section(
        "letter_spacing",
        ui_vec![Text! {
            txt = "Letter spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when *#is_hovered {
                #[easing(150.ms())]
                letter_spacing = 30.pct();
            }
        }],
    )
}

fn decoration_lines() -> impl UiNode {
    section(
        "Decorations",
        ui_vec![
            Text! {
                txt = "Overline, 1, Dotted,\ndefault color";
                overline = 1, LineStyle::Dotted;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Strikethrough, 1, Solid,\ndefault color";
                strikethrough = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Strikethrough, 4, Double,\ndifferent color";
                strikethrough = 4, LineStyle::Double;
                strikethrough_color = colors::RED;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color";
                underline = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color, skip spaces";
                underline = 1, LineStyle::Solid;
                underline_skip = UnderlineSkip::SPACES;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color, descent";
                underline = 1, LineStyle::Solid;
                underline_position = UnderlinePosition::Descent;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 3, wavy,\ndifferent color, no skip";
                underline = 3, LineStyle::Wavy(1.0);
                underline_color = colors::GREEN;
                underline_skip = UnderlineSkip::NONE;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
            }
        ],
    )
}

fn defaults() -> impl UiNode {
    fn demo(title: &str, font_family: impl Into<FontNames>) -> impl UiNode {
        let font_family = font_family.into();

        let font_name = FONTS
            .list(
                &font_family,
                FontStyle::Normal,
                FontWeight::NORMAL,
                FontStretch::NORMAL,
                &lang!(und),
            )
            .map(|f| match f.done() {
                Some(f) => f.best().family_name().to_text(),
                None => Txt::from_str(""),
            });

        Stack! {
            direction = StackDirection::left_to_right();
            children_align = Align::BASELINE_LEFT;
            children = ui_vec![
                Text!(if title.is_empty() {
                    formatx!("{font_family}: ")
                } else {
                    formatx!("{title}: ")
                }),
                Text! {
                    txt = font_name;
                    font_family;
                }
            ];
        }
    }

    section(
        "defaults",
        ui_vec![
            // Generic
            demo("", FontName::serif()),
            demo("", FontName::sans_serif()),
            demo("", FontName::monospace()),
            demo("", FontName::cursive()),
            demo("", FontName::fantasy()),
            demo("Fallback", "not-a-font-get-fallback"),
            demo("UI", FontNames::default())
        ],
    )
}

fn section(header: &'static str, items: impl UiNodeList) -> impl UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![Text! {
            txt = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}

fn text_editor() -> impl UiNode {
    let is_open = var(false);

    Button! {
        child = Text!(is_open.map(|&i| if i { "show text editor" } else { "open text editor" }.into()));
        style_fn = button::LinkStyle!();
        on_click = hn!(|_| {
            let editor_id = WindowId::named("text-editor");
            if is_open.get() {
                if WINDOWS.focus(editor_id).is_err() {
                    is_open.set_ne(false);
                }
            } else {
                WINDOWS.open_id(editor_id, async_clmv!(is_open, {
                    let txt_status = var(text::CaretStatus::none());
                    Window! {
                        title = "Text Example - Editor";
                        on_open = hn!(is_open, |_| {
                            is_open.set_ne(true);
                        });
                        on_close = hn!(is_open, |_| {
                            is_open.set_ne(false);
                        });
                        child = Grid! {
                            columns = ui_vec![grid::Column!(1.lft())];
                            rows = ui_vec![
                                grid::Row!(1.lft()),
                                grid::Row!(),
                            ];
                            cells = ui_vec![
                                TextInput! {
                                    txt = var(Txt::from_static("Hello, this is a multi-line text input"));
                                    get_caret_status = txt_status.clone();
                                },
                                Text! {
                                    grid::cell::row = 1;
                                    txt = txt_status.map_to_text();
                                }
                            ];
                        }
                    }
                }));
            }
        });
    }
}
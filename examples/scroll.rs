#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::widgets::scroll::commands::ScrollToMode;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("scroll");
    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        Window! {
            title = "Scroll Example";
            child = z_stack(ui_vec![
                scroll! {
                    id = "scroll";
                    padding = 20;
                    background_color = color_scheme_map(
                        hex!(#245E81),
                        colors::WHITE.with_alpha(80.pct()).mix_normal(hex!(#245E81))
                    );
                    // smooth_scrolling = false;
                    child = stack!{
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::LEFT;
                        children = ui_vec![
                            Text! {
                                id = "Lorem 1";
                                txt = "Lorem 1";
                                font_size = 20;
                            },
                            Text!(ipsum()),
                            Text! {
                                id = "Lorem 2";
                                txt = "Lorem 2";
                                font_size = 20;
                            },
                            Text!(ipsum())
                        ];
                    }
                },
                commands()
            ]);
        }
    })
}

fn commands() -> impl UiNode {
    use zero_ui::widgets::scroll::commands::*;

    let show = var(false);

    stack! {
        direction = StackDirection::top_to_bottom();
        align = Align::TOP;
        padding = 5;
        background_color = color_scheme_map(colors::BLACK.with_alpha(90.pct()), colors::WHITE.with_alpha(90.pct()));
        corner_radius = (0, 0, 8, 8);
        alt_focus_scope = true;

        children = ui_vec![
            stack! {
                direction = StackDirection::top_to_bottom();
                visibility = show.map_into();
                spacing = 3;

                children = ui_vec![
                    cmd_btn(SCROLL_UP_CMD),
                    cmd_btn(SCROLL_DOWN_CMD),
                    cmd_btn(SCROLL_LEFT_CMD),
                    cmd_btn(SCROLL_RIGHT_CMD),
                    separator(),
                    cmd_btn(PAGE_UP_CMD),
                    cmd_btn(PAGE_DOWN_CMD),
                    cmd_btn(PAGE_LEFT_CMD),
                    cmd_btn(PAGE_RIGHT_CMD),
                    separator(),
                    cmd_btn(SCROLL_TO_TOP_CMD),
                    cmd_btn(SCROLL_TO_BOTTOM_CMD),
                    cmd_btn(SCROLL_TO_LEFTMOST_CMD),
                    cmd_btn(SCROLL_TO_RIGHTMOST_CMD),
                    separator(),
                    scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::minimal(10)),
                    scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::center()),
                    separator(),
                ]
            },
            Button! {
                child = Text!(show.map(|s| if !s { "Commands" } else { "Close" }.to_text()));
                margin = show.map(|s| if !s { 0.into() } else { (3, 0, 0, 0).into() });
                on_click = hn!(|_| {
                    show.modify(|s| *s.to_mut() = !**s);
                });

                corner_radius = (0, 0, 4, 4);
                padding = 4;
            }
        ];
    }
}
fn cmd_btn(cmd: Command) -> impl UiNode {
    let cmd = cmd.scoped(WidgetId::named("scroll"));
    Button! {
        child = Text!(cmd.name_with_shortcut());
        enabled = cmd.is_enabled();
        // visibility = cmd.has_handlers().map_into();
        on_click = hn!(|_| {
            cmd.notify();
        });

        corner_radius = 0;
        padding = 4;
    }
}
fn scroll_to_btn(target: WidgetId, mode: ScrollToMode) -> impl UiNode {
    use zero_ui::widgets::scroll::commands;

    let scroll = WidgetId::named("scroll");
    let cmd = commands::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} {}", target, if let ScrollToMode::Minimal{..} = &mode { "(minimal)" } else { "(center)" });
        enabled = cmd.is_enabled();
        on_click = hn!(|_| {
            commands::scroll_to(scroll, target, mode.clone());
        });

        corner_radius = 0;
        padding = 4;
    }
}
fn separator() -> impl UiNode {
    wgt! {
        size = (8, 8);
    }
}

fn ipsum() -> Txt {
    let mut s = String::new();
    for _ in 0..10 {
        for _ in 0..10 {
            s.push_str(&lipsum::lipsum_words(25));
            s.push('\n');
        }
        s.push('\n');
    }

    s.into()
}

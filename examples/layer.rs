#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::{
    button,
    color::filter::opacity,
    container::padding,
    focus::{directional_nav, focus_scope, tab_nav, DirectionalNav, TabNav},
    layer::AnchorOffset,
    layout::{align, margin, offset, rotate, scale, y},
    mouse::{on_mouse_enter, on_mouse_leave},
    prelude::*,
    widget::{background_color, border, corner_radius, hit_test_mode, modal, visibility, HitTestMode},
};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("layer");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    APP.defaults().run_window(async {
        // layer will init with the window, when it opens.
        LAYERS.insert(
            LayerIndex::TOP_MOST - 100,
            Text! {
                hit_test_mode = HitTestMode::Disabled;
                txt = "on_pre_init";
                font_size = 72;
                font_family = "monospace";
                opacity = 3.pct();
                // rotate = 45.deg();
                align = Align::CENTER;
            },
        );

        Window! {
            title = "Layer Example";

            // zero_ui::properties::inspector::show_bounds = true;
            // zero_ui::properties::inspector::show_hit_test = true;

            child_align = Align::CENTER;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                spacing = 5;
                children = ui_vec![
                    overlay_example(),
                    layer_index_example(),
                    anchor_example(),
                    transform_anchor_example(),
                ];
            };
        }
    })
}

fn overlay_example() -> impl UiNode {
    Button! {
        child = Text!("TOP_MOST");
        on_click = hn!(|_| {
            LAYERS.insert(LayerIndex::TOP_MOST, overlay("overlay", 0));
        });
    }
}
fn overlay(id: impl Into<WidgetId>, offset: i32) -> impl UiNode {
    let id = id.into();
    Container! {
        id;
        modal = true;
        background_color = color_scheme_map(colors::WHITE.with_alpha(10.pct()), colors::BLACK.with_alpha(10.pct()));
        child_align = Align::CENTER;
        child = Container! {
            offset = (offset, offset);
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            background_color = color_scheme_map(
                colors::GREEN.darken(80.pct()),
                colors::WHITE.with_alpha(80.pct()).mix_normal(colors::GREEN)
            );
            button::extend_style = Style! { corner_radius = unset! };
            padding = 2;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                children_align = Align::RIGHT;
                children = ui_vec![
                    Text! {
                        txt = "Overlay inserted in the TOP_MOST layer.";
                        margin = 15;
                    },
                    Stack! {
                        direction = StackDirection::left_to_right();
                        spacing = 2;
                        children = ui_vec![
                            Button! {
                                visibility = offset < 50;
                                child = Text!("Open Another");
                                on_click = hn!(|_| {
                                    LAYERS.insert(LayerIndex::TOP_MOST, overlay(WidgetId::new_unique(), offset + 10));
                                })
                            },
                            Button! {
                                child = Text!("Remove");
                                on_click = hn!(|_| {
                                    LAYERS.remove(id);
                                })
                            },
                        ]
                    }
                ]
            }
        }
    }
}

fn layer_index_example() -> impl UiNode {
    // demonstrates that the z-order is not affected by the order of insertion.
    Stack! {
        direction = StackDirection::left_to_right();
        spacing = 5;
        children = ui_vec![
            layer_n_btn(7, web_colors::DARK_GREEN),
            layer_n_btn(8, web_colors::DARK_BLUE),
            layer_n_btn(9, web_colors::DARK_RED),
        ]
    }
}
fn layer_n_btn(n: u32, color: Rgba) -> impl UiNode {
    let label = formatx!("Layer {n}");
    Button! {
        child = Text!(label.clone());
        on_click = async_hn!(label, |_| {
            let id = WidgetId::new_unique();
            LAYERS.insert(n, Container! {
                id;
                child = Text! {
                    txt = label.clone();
                    font_color = rgb(0.92, 0.92, 0.92);
                    font_size = 16;
                    font_weight = FontWeight::BOLD;
                };
                background_color = color.with_alpha(80.pct());
                padding = 10;
                margin = {
                    let inc = n as i32 * 10;
                    (20 + inc, 10, 0, inc - 40)
                };
                align = Align::TOP;
                hit_test_mode = HitTestMode::Disabled;
            });

            task::deadline(2.secs()).await;

            LAYERS.remove(id);
        });
    }
}

fn anchor_example() -> impl UiNode {
    let offsets = [
        AnchorOffset::out_top_left(),
        AnchorOffset::out_top(),
        AnchorOffset::out_top_right(),
        AnchorOffset::out_right(),
        AnchorOffset::out_bottom_right(),
        AnchorOffset::out_bottom(),
        AnchorOffset::out_bottom_left(),
        AnchorOffset::out_left(),
    ];
    let len = offsets.len();
    let idx = var(0);
    let anchor_mode = idx.map(move |&i| AnchorMode {
        transform: offsets[i].clone().into(),
        min_size: layer::AnchorSize::Unbounded,
        max_size: layer::AnchorSize::Unbounded,
        visibility: true,
        interactivity: false,
        corner_radius: false,
        viewport_bound: false,
    });

    let next_offset = hn!(|_| {
        idx.modify(move |i| {
            let next = **i + 1;
            *i.to_mut() = if next == len { 0 } else { next };
        })
    });

    Button! {
        id = "anchor";
        child = Text!("Anchored");

        margin = (60, 0);
        align = Align::CENTER;

        on_mouse_enter = hn!(|_| {
            LAYERS.insert_anchored(LayerIndex::ADORNER, "anchor", anchor_mode.clone(), Text! {
                id = "anchored";
                txt = "Example";
                font_color = rgb(0.92, 0.92, 0.92);
                padding = 4;
                font_weight = FontWeight::BOLD;
                background_color = web_colors::DARK_GREEN.with_alpha(80.pct());
                border = 1, web_colors::GREEN.darken(20.pct());
                margin = 2;
                hit_test_mode = HitTestMode::Disabled;
            })
        });
        on_mouse_leave = hn!(|_| {
            LAYERS.remove("anchored");
        });

        on_click = next_offset;
    }
}

fn transform_anchor_example() -> impl UiNode {
    let mut insert = true;
    Button! {
        id = "t-anchor";
        child = Text!("Transform Anchored");

        rotate = 20.deg();
        scale = 110.pct();

        on_click = hn!(|_| {
            if insert {
                LAYERS.insert_anchored(LayerIndex::ADORNER, "t-anchor", AnchorMode::foreground(), Container! {
                    id = "t-anchored";
                    child_align = Align::TOP_LEFT;
                    border = 1, colors::GREEN.lighten(30.pct());
                    hit_test_mode = HitTestMode::Disabled;
                    child = Text! {
                        y = -(2.dip() + 100.pct());
                        txt = "example";
                        font_weight = FontWeight::BOLD;
                    }
                })
            } else {
                LAYERS.remove("t-anchored");
            }
            insert = !insert;
        })
    }
}

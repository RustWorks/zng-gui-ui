#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-animation.json.gz", &[("example", &"animation")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        let x = var_from(0);

        use easing::EasingModifierFn::*;
        let easing_mod = var(EaseIn);

        window! {
            title = "Animation Example";

            min_size = (800, 550);
            height = 550;

            content_align = unset!;
            padding = 10;
            content = h_stack(widgets![
                v_stack! {
                    id = "side-menu";
                    margin = (34, 0, 0, 0);
                    spacing = 2;
                    button::theme::padding = 3;
                    items = widgets![
                        ease_btn(&x, "linear", easing::linear, &easing_mod),
                        ease_btn(&x, "quad", easing::quad, &easing_mod),
                        ease_btn(&x, "cubic", easing::cubic, &easing_mod),
                        ease_btn(&x, "quart", easing::quart, &easing_mod),
                        ease_btn(&x, "quint", easing::quint, &easing_mod),
                        ease_btn(&x, "sine", easing::sine, &easing_mod),
                        ease_btn(&x, "expo", easing::expo, &easing_mod),
                        ease_btn(&x, "circ", easing::circ, &easing_mod),
                        ease_btn(&x, "back", easing::back, &easing_mod),
                        ease_btn(&x, "elastic", easing::elastic, &easing_mod),
                        ease_btn(&x, "bounce", easing::bounce, &easing_mod),
                        ease_btn(&x, "step_ceil", |t| easing::step_ceil(5, t), &easing_mod),
                        ease_btn(&x, "step_floor", |t| easing::step_floor(5, t), &easing_mod),
                        ease_btn(&x, "none", easing::none, &easing_mod),
                        button! {
                            content = text("reset");
                            foreground_highlight = {
                                offsets: -2,
                                widths: 1,
                                sides: colors::DARK_RED,
                            };
                            margin = (10, 0, 0, 0);
                            on_click = hn!(x, |ctx, _| {
                                x.set(ctx, 0);
                            });
                        },
                    ]
                },
                v_stack(widgets![
                    h_stack! {
                        id = "top-menu";
                        margin = (0, 0, 0, 5);
                        spacing = 2;
                        button::theme::padding = 3;
                        items = widgets![
                            easing_mod_btn(&easing_mod, EaseIn),
                            easing_mod_btn(&easing_mod, EaseOut),
                            easing_mod_btn(&easing_mod, EaseInOut),
                        ]
                    },
                    container! {
                        id = "demo-area";
                        min_width = 500;
                        content_align = Align::LEFT;
                        margin = (200, 150);
                        content = blank! {
                            id = "ball";
                            size = (40, 40);
                            corner_radius = 20;
                            background_color = colors::RED;

                            x;
                        };
                        background = z_stack!{
                            items_align = Align::LEFT;
                            items = widgets![
                                marker("0", 0),
                                marker("50", 50),
                                marker("100", 100),
                                marker("150", 150),
                                marker("200", 200),
                                marker("250", 250),
                                marker("300", 300),
                            ]
                        }
                    }
                ])
            ]);
        }
    })
}

fn ease_btn(
    l: &RcVar<Length>,
    name: impl Into<Text>,
    easing: impl Fn(EasingTime) -> EasingStep + Clone + 'static,
    easing_mod: &RcVar<easing::EasingModifierFn>,
) -> impl Widget {
    button! {
        content = text(name.into());
        on_click = hn!(l, easing_mod, |ctx, _| {
            let easing = easing_mod.get(ctx).modify_fn(easing.clone());
            l.set_ease(ctx, 0, 300, 1.secs(), easing);
        });
    }
}

fn easing_mod_btn(easing_mod: &RcVar<easing::EasingModifierFn>, value: easing::EasingModifierFn) -> impl Widget {
    button! {
        content = text(value.to_text());
        on_click = hn!(easing_mod, |ctx, _| {
            easing_mod.set_ne(ctx, value);
        });

        when *#{easing_mod.clone()} == value {
            background_color = rgb(40, 40, 60);
        }
    }
}

fn marker(c: impl Into<Text>, x: impl Into<Length>) -> impl Widget {
    text! {
        text = c.into();
        color = colors::WHITE.with_alpha(30.pct());
        font_size = 20;
        font_weight = FontWeight::BOLD;
        x = x.into();
    }
}

fn plot(easing: impl Fn(EasingTime) -> EasingStep, size: impl IntoVar<Size>) -> impl Widget {
    let mut dots = widget_vec![];
    for i in 0..100 {
        let x = (i as f32 / 100.0).fct();
        let y = easing(EasingTime::new(x));
        dots.push(blank! {
            position = (x, y);
            size = (10, 10);
            corner_radius = 5;
            translate = -5, -5;
            background_color = colors::WHITE;
        })
    }
    z_stack! {
        items_align = Align::TOP_LEFT;
        items = dots;
        size;
    }
}

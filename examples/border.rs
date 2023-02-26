#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("border);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|| {
        window! {
            title = "Border Example";

            height = 800;

            background_color = colors::BLUE.darken(70.pct());

            color_scheme = ColorScheme::Dark;

            child = stack! {
                direction = StackDirection::top_to_bottom();
                align = Align::CENTER;
                spacing = 20;
                children = ui_vec![
                    widgets::mr_borders! {
                        border_align = 0.pct();
                        child = text!("border_align = 0.pct();");
                    },
                    widgets::mr_borders! {
                        border_align = (1.0 / 3.0).fct();
                        child = text!("border_align = (1.0 / 3.0).fct();");
                    },
                    widgets::mr_borders! {
                        border_align = 50.pct();
                        child = text!("border_align = 50.pct();");
                    },
                    widgets::mr_borders! {
                        border_align = 100.pct();
                        child = text!("border_align = 100.pct();");
                    },
                    clip_to_bounds_demo(),
                    widgets::mr_borders! {
                        border_align = 100.pct();
                        child = widgets::mr_borders! {
                            border_align = 100.pct();
                            child = widgets::mr_borders! {
                                border_align = 100.pct();
                                child = text!("Nested");
                            },
                        },
                    },
                ]
            };
        }
    })
}

fn clip_to_bounds_demo() -> impl UiNode {
    let clip = var(true);
    container! {
        child_align = Align::FILL;
        corner_radius = 10;
        border = 0.5, colors::RED.darken(20.pct());
        clip_to_bounds = clip.clone();
        on_click = hn!(clip, |_| {
            clip.modify(|c| *c.to_mut() = !**c)
        });
        child = text! {
            corner_radius = 0;
            background_color = colors::GREEN.darken(40.pct());
            padding = 3;
            rotate = -(5.deg());
            txt_align = Align::CENTER;
            txt = clip.map(|c| formatx!("clip_to_bounds = {c}"));
        };
    }
}

mod widgets {
    use zero_ui::prelude::new_widget::*;

    #[widget($crate::widgets::mr_borders)]
    pub mod mr_borders {
        use super::*;

        inherit!(container);

        properties! {
            padding = 20;

            child_align = Align::CENTER;

            background_color = colors::GREEN.darken(40.pct());

            border as border0 = 4, colors::WHITE.with_alpha(20.pct());
            border as border1 = 4, colors::BLACK.with_alpha(20.pct());
            border as border2 = 4, colors::WHITE.with_alpha(20.pct());

            foreground_highlight = 3, 1, colors::ORANGE;

            corner_radius = 20;
        }
    }
}

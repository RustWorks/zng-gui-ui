#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui::core::config::*;

use zero_ui::prelude::new_widget::{Dip, DipPoint, DipVector};
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("button");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|| {
        CONFIG.load(ConfigFile::new("target/tmp/example.config.json"));
        // CONFIG.remove("old.key");

        let checked = CONFIG.var("main.checked", || false);
        let count = CONFIG.var("main.count", || 0);
        let txt = CONFIG.var("main.txt", || "Save this".to_text());
        let status = CONFIG.status();

        trace_status(&status);

        window! {
            title = "Config Example";
            background = text! {
                txt = status.map_to_text();
                margin = 10;
                font_family = "monospace";
                align = Align::TOP_LEFT;

                font_weight = FontWeight::BOLD;

                when *#{status.map(|s| s.has_errors())} {
                    txt_color = colors::RED;
                }
            };
            child = stack! {
                direction = StackDirection::top_to_bottom();
                align = Align::CENTER;
                spacing = 5;
                children = ui_vec![
                    toggle! {
                        child = text!(checked.map(|c| formatx!("Checked: {c:?}")));
                        checked = checked.clone();
                    },
                    button! {
                        child = text!(count.map(|c| formatx!("Count: {c:?}")));
                        on_click = hn!(count, |_, _| {
                            count.modify(|c| *c.to_mut() += 1).unwrap();
                        })
                    },
                    separator(),
                    text_input! {
                        txt = txt.clone();
                        min_width = 100;
                    },
                    separator(),
                    button! {
                        child = text!("Reset");
                        on_click = hn!(|_, _| {
                            checked.set_ne(false).unwrap();
                            count.set_ne(0).unwrap();
                            txt.set_ne("Save this").unwrap();
                        })
                    },
                    button! {
                        child = text!("Open Another Instance");
                        on_click = hn!(|ctx, _| {
                            let offset = Dip::new(30);
                            let pos = WindowVars::req().actual_position().get() + DipVector::new(offset, offset);
                            let pos = pos.to_i32();
                            let r: Result<(), Box<dyn std::error::Error>> = (|| {
                                let exe = std::env::current_exe()?;
                                std::process::Command::new(exe).env("MOVE-TO", format!("{},{}", pos.x, pos.y)).spawn()?;
                                Ok(())
                            })();
                            match r {
                                Ok(_) => println!("Opened another instance"),
                                Err(e) => eprintln!("Error opening another instance, {e:?}"),
                            }
                        })
                    }
                ];
            };
            on_load = hn_once!(|ctx, _| {
                if let Ok(pos) = std::env::var("MOVE-TO") {
                    if let Some((x, y)) = pos.split_once(',') {
                        if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                            let pos = DipPoint::new(Dip::new(x), Dip::new(y));
                            WindowVars::req().position().set(pos);
                            WINDOWS.focus(ctx.path.window_id()).unwrap();
                        }
                    }
                }
            });
        }
    })
}

fn separator() -> impl UiNode {
    hr! {
        color = rgba(1.0, 1.0, 1.0, 0.2);
        margin = (0, 8);
        line_style = LineStyle::Dashed;
    }
}

fn trace_status(status: &impl Var<ConfigStatus>) {
    let mut read_errs = 0;
    let mut write_errs = 0;
    let mut internal_errs = 0;
    status
        .trace_value(move |s: &ConfigStatus| {
            if let Some(e) = &s.internal_error {
                if s.internal_errors != internal_errs {
                    internal_errs = s.internal_errors;
                    tracing::error!("internal error: {e}");
                }
            }

            if let Some(e) = &s.read_error {
                if s.read_errors != read_errs {
                    read_errs = s.read_errors;
                    tracing::error!("read error: {e}");
                }
            }

            if let Some(e) = &s.write_error {
                if s.write_errors != write_errs {
                    write_errs = s.write_errors;
                    tracing::error!("write error: {e}");
                }
            }
        })
        .perm();
}

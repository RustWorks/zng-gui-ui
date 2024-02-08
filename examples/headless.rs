use std::{io::Write, path::PathBuf};

use zero_ui::{
    color::{
        self,
        gradient::{linear_gradient, GradientStops},
    },
    image::Img,
    prelude::*,
    stack::stack_nodes,
    window::{FrameCaptureMode, FrameImageReadyArgs, HeadlessAppWindowExt},
};

use zero_ui::view_process::prebuilt as view_process;

fn main() {
    examples_util::print_info();
    view_process::init();

    // view_process::run_same_process(headless_example);

    images_render();
    // headless_example();
    // headless_example_video();
}

#[allow(unused)]
fn headless_example() {
    println!("-=Headless Example=-\n");
    // This example uses a headless window to render an image.

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = APP.defaults().run_headless(true);

    app.run_window(async {
        Window! {
            // the window content is the image.
            child = image();
            auto_size = true;

            // use the CPU only backend if available, by default the
            // same GPU used for headed windows is used.
            render_mode = window::RenderMode::Software;

            // capture the first frame.
            frame_capture_mode = FrameCaptureMode::Next;

            // this event will fire every time a frame is rendered (just once in this case).
            on_frame_image_ready = async_hn_once!(|args: FrameImageReadyArgs| {
                // in this case a `frame_image` was already captured.
                let img = args.frame_image.unwrap();

                // we save it...
                print!("saving ./screenshot.png ... ");
                flush_stdout();

                img.save("screenshot.png").await.unwrap();

                println!("done");
                APP.exit();
            });
        }
    });
}

// A 800x600 "Hello World!" with a fancy background.
fn image() -> impl UiNode {
    Container! {
        layout::size = (800, 600);

        widget::background = stack_nodes({
            fn gradient(angle: i32, mut color: color::Rgba) -> impl UiNode {
                color.alpha = 0.3;
                let stops = GradientStops::from_stripes(&[color, color.transparent()], 0.0);
                linear_gradient(angle.deg(), stops)
            }

            ui_vec![
                color::flood(colors::WHITE),
                gradient(0, colors::RED),
                gradient(20, colors::RED),
                gradient(40, colors::RED),
                gradient(120, colors::GREEN),
                gradient(140, colors::GREEN),
                gradient(160, colors::GREEN),
                gradient(240, colors::BLUE),
                gradient(260, colors::BLUE),
                gradient(280, colors::BLUE),
            ]
        });

        child = Text! {
            layout::align = Align::CENTER;
            txt = "Hello World!";
            font_size = 72;
            font_family = ["Consolas", "monospace"];
            font_color = colors::WHITE;
        };
    }
}

fn flush_stdout() {
    std::io::stdout().lock().flush().ok();
}

/// You can also use the `Images` service to render to an image.
#[allow(unused)]
fn images_render() {
    println!("-=Images::render Example=-\n");

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = APP.defaults().run_headless(true);

    // request an image rendered from a node, the `Images` service will render the node and update the image
    // variable every time the node (re)renders.
    let img = zero_ui::image::IMAGES.render_node(window::RenderMode::Software, 1.fct(), None, image);

    app.run_task(async move {
        while img.with(Img::is_loading) {
            img.wait_update().await;
        }
        let img = img.get();

        if img.is_loaded() {
            // we save it...
            print!("saving ./screenshot.Images.png ... ");
            flush_stdout();

            img.save("screenshot.Images.png").await.unwrap();

            println!("done");
        } else if let Some(err) = img.error() {
            eprintln!("[error]: {err}");
        }
    });

    // Internally the `Images` service uses a headless window for rendering too, but this method is more easy
    // to use, with the trade-off that you have less control over the headless window.
}

fn video(finished: zero_ui::var::ArcVar<bool>) -> impl UiNode {
    let bkg_rotate = var(0.turn());
    let txt_fade = var(0.fct());
    let txt_size = var(32.dip());
    let fade_out = var(0.fct());
    Container! {
        layout::size = (800, 600);

        widget::on_init = async_hn!(txt_fade, txt_size, bkg_rotate, fade_out, finished, |_| {
            task::deadline(300.ms()).await;
            txt_fade.ease(1.fct(), 800.ms(), easing::linear).perm();
            txt_size.ease(72, 800.ms(), easing::linear).perm();

            task::deadline(100.ms()).await;
            bkg_rotate.ease(5.turn(), 10.secs(), easing::circ).perm();

            task::deadline(8.secs()).await;
            txt_size.ease(120, 2.secs(), easing::linear).perm();
            txt_fade.ease(0.fct(), 2.secs(), easing::linear).perm();

            task::deadline(1.secs()).await;
            fade_out.ease(1.fct(), 1.secs(), easing::linear).perm();

            bkg_rotate.wait_animation().await;
            finished.set(true);
        });

        widget::background = Stack! {
            children = {
                let gradient = clmv!(bkg_rotate, |angle: i32, mut color: color::Rgba| {
                    color.alpha = 0.3;
                    let stops = color::gradient::GradientStops::from_stripes(&[color, color.transparent()], 0.0);
                    let angle = angle.deg();
                    color::gradient::linear_gradient(
                        bkg_rotate.map(move |r| (angle + layout::AngleDegree::from(*r)).into()),
                        stops
                    )
                });

                ui_vec![
                    color::flood(colors::WHITE),
                    gradient(0, colors::RED),
                    gradient(20, colors::RED),
                    gradient(40, colors::RED),
                    gradient(120, colors::GREEN),
                    gradient(140, colors::GREEN),
                    gradient(160, colors::GREEN),
                    gradient(240, colors::BLUE),
                    gradient(260, colors::BLUE),
                    gradient(280, colors::BLUE),
                ]
            };
        };
        widget::foreground_color = fade_out.map(|&o| colors::BLACK.with_alpha(o));

        child = Text! {
            layout::align = Align::CENTER;
            txt = "Hello World!";
            font_family = ["Consolas", "monospace"];
            font_color = colors::WHITE;
            font_size = txt_size;
            color::filter::opacity = txt_fade;
        };
    }
}

#[allow(unused)]
fn headless_example_video() {
    println!("-=Headless Example (video)=-\n");
    // This example uses a headless window to render frames for FFmpeg.

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = APP.defaults().run_headless(true);
    // saving frame can be slow, so we will manually control the app time to not miss any frame.
    APP.start_manual_time();

    const FPS: f32 = 60.0;
    zero_ui::var::VARS.frame_duration().set((1.0 / FPS).secs());

    app.run_window(async {
        // will save frames as "{temp}/{frame}.png"
        let temp = PathBuf::from("target/tmp/headless_example_video");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        let frame = var(0u32);
        let recorded = var(false);
        print_status("recording", &recorded);

        Window! {
            // the window content is the "video".
            child = video(recorded.clone());
            auto_size = true;

            // use the CPU only backend if available, by default the
            // same GPU used for headed windows is used.
            render_mode = window::RenderMode::Software;

            // capture all frames.
            frame_capture_mode = FrameCaptureMode::All;

            // this event will fire every time a frame is rendered.
            on_frame_image_ready = async_hn!(temp, frame, |args: FrameImageReadyArgs| {
                let img = args.frame_image.unwrap();

                let frame_i = frame.get();
                frame.set(frame_i + 1);

                img.save(temp.join(format!("{frame_i:05}.png"))).await.unwrap();

                // advance time at a perfect framerate.
                APP.advance_manual_time((1.0 / FPS).secs());
                // ensure a frame image is actually generated (for video).
                //
                // also, retained rendering only renders when needed, so without this
                // line the app never even updates, and the initial delay timer waits forever.
                WIDGET.render_update();
            });

            on_load = async_hn!(recorded, temp, |_| {
                recorded.wait_value(|&f| f).await;

                let encoded = var(false);
                print_status("encoding", &encoded);

                task::spawn_wait(clmv!(encoded, || {
                    // https://www.ffmpeg.org/download.html
                    let ffmpeg = std::process::Command::new("ffmpeg")
                    .arg("-framerate")
                    .arg(FPS.to_string())
                    .arg("-y")
                    .arg("-i")
                    .arg(temp.join("%05d.png"))
                    .arg("-c:v")
                    .arg("libx264")
                    .arg("-pix_fmt")
                    .arg("yuv420p")
                    .arg("screencast.mp4")
                    .arg("-hide_banner")
                    .arg("-loglevel")
                    .arg("error")
                    .status();
                    let _ = std::fs::remove_dir_all(temp);
                    encoded.set(true);

                    assert!(ffmpeg.unwrap().success());
                }));
                encoded.wait_value(|&f| f).await;
                println!("\rfinished.");

                APP.exit();
            });
        }
    });
    while !matches!(app.update(true), zero_ui::app::AppControlFlow::Exit) {}
}

fn print_status(task: &'static str, done: &zero_ui::var::ArcVar<bool>) {
    task::spawn(async_clmv!(done, {
        let mut dots = 0;
        while !done.get() {
            dots += 1;
            if dots > 3 {
                dots = 0;
            }
            print!("\r                      ");
            print!("\r{task}{}", String::from_utf8(vec![b'.'; dots]).unwrap());
            flush_stdout();
            task::deadline(500.ms()).await;
        }
    }));
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use enclose::enclose;
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        window! {
            title: "Button Example";
            content: v_stack! {
                spacing: 5.0;
                items: ui_vec![example(), example()];
            };
        }
    })
}

fn example() -> impl Widget {
    let t = var("Click Me!");
    let mut count = 0;

    button! {
        on_click: enclose!{ (t) move |a| {
            let ctx = a.ctx();
            count += 1;
            let new_txt = formatx!("Clicked {} time{}!", count, if count > 1 {"s"} else {""});
            println!("{}", new_txt);
            ctx.updates.push_set(&t, new_txt, ctx.vars).unwrap();
        }};
        transform: rotate(30.0); // for testing the future mouse hover implementation

        content: text(t);
    }
}

use zero_ui::prelude::*;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _err = toggle! {
        child = Text!("");
        value = 0;
    };

    let _ok = toggle! {
        child = Text!("");
        value::<i32> = 0;
    };
}

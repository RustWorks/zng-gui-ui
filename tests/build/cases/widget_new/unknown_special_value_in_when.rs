use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        margin = 0;
        when *#is_pressed {
            margin = foo!;
        }
    };
}

use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        margin = 0
        // we expect this properties to be used.
        enabled = true;
        cursor = CursorIcon::Hand;
    };
}

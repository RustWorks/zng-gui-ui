use zero_ui::prelude::*;

fn main() {
    let _scope = App::minimal();
    let _ = Wgt! {
        unknown = {
            value: 0,
        };
    };
}

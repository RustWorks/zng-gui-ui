use zero_ui::properties::{cursor, margin};
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = Wgt! {
        // we expected an error here.
        cursor = ;
        // we expect margin to be used here.
        margin = 0;
    };
}

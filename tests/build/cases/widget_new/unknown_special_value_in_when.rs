use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = Wgt! {
        margin = 0;
        when *#is_pressed {
            margin = foo!;
        }
    };
}

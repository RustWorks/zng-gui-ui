use zero_ui::properties::{margin, states::is_pressed};
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = wgt! {
        margin = 0 // missing ; here
        when *#is_pressed {
            margin = 20;
        }
    };
}

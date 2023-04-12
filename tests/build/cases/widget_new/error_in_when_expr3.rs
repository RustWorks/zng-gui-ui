use zero_ui::properties::margin;
use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = Wgt! {
        margin = 0;
        when *#margin.0. {
            margin = 10;
        }
    };
}

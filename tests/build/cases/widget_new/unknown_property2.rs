use zero_ui::widgets::blank;

fn main() {
    let _scope = zero_ui::core::app::App::blank();
    let _ = blank! {
        unknown = {
            value: 0,
        };
    };
}

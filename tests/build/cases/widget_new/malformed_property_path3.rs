use zero_ui::widgets::wgt;

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = Wgt! {
        zero_ui::properties:margin = 0;
    };
}

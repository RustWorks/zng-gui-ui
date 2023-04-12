use zero_ui::core::{property, var::IntoVar, widget_instance::UiNode};
use zero_ui::widgets::wgt;

#[property(CONTEXT)]
pub fn simple_type(child: impl UiNode, simple_a: impl IntoVar<u32>, simple_b: impl IntoVar<u32>) -> impl UiNode {
    child
}

fn main() {
    let _scope = zero_ui::core::app::App::minimal();
    let _ = Wgt! {
        simple_type = {
            simple_a: 42,
            simple_b: true,
        }
    };
}

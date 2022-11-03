use zero_ui::core::{property, var::IntoVar, UiNode};
use zero_ui::widgets::blank;

#[property(context)]
pub fn my_property(child: impl UiNode, a: impl IntoVar<u32>) -> impl UiNode {
    let _ = a;
    child
}

fn main() {
    let _ = blank! {
        when *#my_property == 1 { }
    };
}

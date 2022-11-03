use zero_ui::core::{property, widget, UiNode};
use zero_ui::properties::{margin, states::is_pressed};

struct NotVarValue;
impl NotVarValue {
    fn is(&self) -> bool {
        true
    }
}

#[property(context)]
pub fn foo(child: impl UiNode, value: NotVarValue) -> impl UiNode {
    let _ = value;
    child
}

#[widget($crate::bar)]
pub mod bar {
    use super::*;

    properties! {
        foo = NotVarValue;
        margin = 0;
        when self.foo.is() && self.is_pressed {
            margin = 1;
        }
    }
}

fn main() {}

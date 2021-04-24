use zero_ui::core::widget;

#[widget($crate::test_widget)]
pub mod test_widget {
    use zero_ui::core::NilUiNode;

    properties! {
        #[allowed_in_when = false]
        foo { bool };
    }

    fn new_child(foo: bool) -> NilUiNode {
        println!("{}", foo);
        NilUiNode
    }
}

fn main() {
    let _ = test_widget! {
        foo = unset!;
    };
}

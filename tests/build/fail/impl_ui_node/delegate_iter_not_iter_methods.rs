use zero_ui::core::{impl_ui_node, UiNode};

struct NoIterMethods;

struct MyNode {
    inner: NoIterMethods,
}

#[impl_ui_node(delegate_iter = self.inner.iter(), delegate_iter_mut = self.inner.iter_mut())]
impl UiNode for MyNode {}

fn main() {}

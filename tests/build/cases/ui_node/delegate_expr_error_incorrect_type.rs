use zero_ui::core::{ui_node, widget_instance::UiNode};

struct NotANode;

struct MyNode {
    inner: NotANode,
}

#[ui_node(delegate = &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}

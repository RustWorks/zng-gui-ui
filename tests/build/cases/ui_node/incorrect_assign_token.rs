use zero_ui::wgt_prelude::{ui_node, NilUiNode, UiNode};

struct Node(NilUiNode);
#[ui_node(
    delegate: &self.0,
    delegate_mut: &mut self.0,
)]
impl UiNode for Node {}

fn main() {}

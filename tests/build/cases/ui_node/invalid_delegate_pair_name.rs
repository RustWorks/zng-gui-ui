use zero_ui::wgt_prelude::{ui_node, NilUiNode, UiNode};

struct Node1 {
    inner: NilUiNode,
}
#[ui_node(delegate2 = &mut self.inner)]
impl UiNode for Node1 {}

fn main() {}

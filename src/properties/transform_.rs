use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::{FrameBinding, FrameBindingKey, FrameBuilder, FrameUpdate},
    units::{LayoutSize, LayoutTransform, Transform},
    var::{IntoVar, LocalVar, ObjVar},
};
use crate::core::{impl_ui_node, property, UiNode};

struct TransformNode<C: UiNode, T: LocalVar<Transform>> {
    child: C,
    transform: T,
    layout_transform: LayoutTransform,
    frame_key: Option<FrameBindingKey<LayoutTransform>>,
}

#[impl_ui_node(child)]
impl<C: UiNode, T: LocalVar<Transform>> UiNode for TransformNode<C, T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.transform.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        if self.transform.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
            // ctx.updates.push_render_update(); TODO?
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.layout_transform = self.transform.get_local().to_layout(final_size, ctx);
        self.child.arrange(final_size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let transform = if let Some(frame_key) = self.frame_key {
            frame_key.bind(self.layout_transform)
        } else {
            FrameBinding::Value(self.layout_transform)
        };
        frame.push_transform(transform, |frame| self.child.render(frame));
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        if let Some(frame_key) = self.frame_key {
            update.update_transform(frame_key.update(self.layout_transform));
        }
        self.child.render_update(update);
    }
}

#[property(outer)]
pub fn transform(child: impl UiNode, transform: impl IntoVar<Transform>) -> impl UiNode {
    let transform = transform.into_local();
    let frame_key = if transform.can_update() {
        Some(FrameBindingKey::new_unique())
    } else {
        None
    };
    TransformNode {
        child,
        transform,
        frame_key,
        layout_transform: LayoutTransform::identity(),
    }
}

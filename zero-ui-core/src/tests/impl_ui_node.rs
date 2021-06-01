//! Tests for `#[impl_ui_node(..)]` macro.
//!
//! Note: Compile error tests are in the integration tests folder: `tests/build/impl_ui_node`

use util::{assert_did_not_trace, assert_only_traced, TraceNode};

use crate::{
    context::{TestWidgetContext, UpdateDisplayRequest, UpdateRequest, WidgetContext},
    impl_ui_node, node_vec, nodes,
    render::{FrameBuilder, FrameId, FrameUpdate, WidgetTransformKey},
    units::LayoutSize,
    widget_base::implicit_base,
    window::WindowId,
    UiNode, UiNodeList, UiNodeVec, Widget, WidgetId, LAYOUT_ANY_SIZE,
};

#[test]
pub fn default_child() {
    struct Node<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for Node<C> {}

    test_trace(Node {
        child: TraceNode::default(),
    });
}
#[test]
pub fn default_delegate() {
    struct Node<C> {
        inner: C,
    }
    #[impl_ui_node(delegate = &self.inner, delegate_mut = &mut self.inner)]
    impl<C: UiNode> UiNode for Node<C> {}

    test_trace(Node {
        inner: TraceNode::default(),
    });
}
#[test]
pub fn default_children() {
    struct Node<C> {
        children: C,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList> UiNode for Node<C> {}

    test_trace(Node {
        children: nodes![TraceNode::default(), TraceNode::default()],
    });
}
#[test]
pub fn default_delegate_list() {
    struct Node<C> {
        inner: C,
    }
    #[impl_ui_node(delegate_list = &self.inner, delegate_list_mut = &mut self.inner)]
    impl<C: UiNodeList> UiNode for Node<C> {}

    test_trace(Node {
        inner: nodes![TraceNode::default(), TraceNode::default()],
    });
}
#[test]
pub fn default_children_iter() {
    struct Node {
        children: UiNodeVec,
    }
    #[impl_ui_node(children_iter)]
    impl UiNode for Node {}

    test_trace(Node {
        children: node_vec![TraceNode::default(), TraceNode::default()],
    })
}
#[test]
pub fn default_delegate_iter() {
    struct Node {
        inner: UiNodeVec,
    }
    #[impl_ui_node(delegate_iter = self.inner.iter(), delegate_iter_mut = self.inner.iter_mut())]
    impl UiNode for Node {}

    test_trace(Node {
        inner: node_vec![TraceNode::default(), TraceNode::default()],
    })
}
fn test_trace(node: impl UiNode) {
    let mut wgt = implicit_base::new(node, WidgetId::new_unique());
    let mut ctx = TestWidgetContext::new();

    wgt.test_init(&mut ctx);
    assert_only_traced!(wgt.state(), "init");

    wgt.test_update(&mut ctx);
    assert_only_traced!(wgt.state(), "update");

    let l_size = LayoutSize::new(1000.0, 800.0);

    wgt.test_measure(&mut ctx, l_size);
    assert_only_traced!(wgt.state(), "measure");

    wgt.test_arrange(&mut ctx, l_size);
    assert_only_traced!(wgt.state(), "arrange");

    let window_id = WindowId::new_unique();
    let root_transform_key = WidgetTransformKey::new_unique();
    let mut frame = FrameBuilder::new_renderless(FrameId::invalid(), window_id, wgt.id(), root_transform_key, l_size, 1.0);
    wgt.test_render(&mut ctx, &mut frame);
    assert_only_traced!(wgt.state(), "render");

    let mut update = FrameUpdate::new(window_id, wgt.id(), root_transform_key, FrameId::invalid());
    wgt.test_render_update(&mut ctx, &mut update);
    assert_only_traced!(wgt.state(), "render_update");

    wgt.test_deinit(&mut ctx);
    assert_only_traced!(wgt.state(), "deinit");
}

#[test]
pub fn allow_missing_delegate() {
    struct Node1<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for Node1<C> {
        #[allow_(zero_ui::missing_delegate)]
        fn update(&mut self, _: &mut WidgetContext) {
            // self.child.update(ctx);
        }
    }
    struct Node2<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    #[allow_(zero_ui::missing_delegate)]
    impl<C: UiNode> UiNode for Node2<C> {
        fn update(&mut self, _: &mut WidgetContext) {
            // self.child.update(ctx);
        }
    }

    fn test(node: impl UiNode) {
        let mut wgt = implicit_base::new(node, WidgetId::new_unique());
        let mut ctx = TestWidgetContext::new();

        wgt.test_init(&mut ctx);
        assert_only_traced!(wgt.state(), "init");

        wgt.test_update(&mut ctx);
        assert_did_not_trace!(wgt.state());
    }

    test(Node1 {
        child: TraceNode::default(),
    });
    test(Node2 {
        child: TraceNode::default(),
    });
}

#[test]
pub fn default_no_child() {
    struct Node;
    #[impl_ui_node(none)]
    impl UiNode for Node {}

    let mut wgt = implicit_base::new(Node, WidgetId::new_unique());
    let mut ctx = TestWidgetContext::new();

    // we expect defaults to do nothing with the WidgetContext.
    wgt.test_init(&mut ctx);
    wgt.test_update(&mut ctx);
    wgt.test_deinit(&mut ctx);
    let u = ctx.apply_updates();
    assert!(u.events.is_empty());
    assert_eq!(u.update, UpdateRequest::default());
    assert_eq!(u.display_update, UpdateDisplayRequest::None);
    assert!(u.wake_time.is_none());

    wgt.test_init(&mut ctx);

    let available_size = LayoutSize::new(1000.0, 800.0);

    // we expect default to fill available space and collapse in infinite spaces.
    let desired_size = wgt.test_measure(&mut ctx, available_size);
    assert_eq!(desired_size, available_size);

    let available_size = LayoutSize::new(LAYOUT_ANY_SIZE, LAYOUT_ANY_SIZE);
    let desired_size = wgt.test_measure(&mut ctx, available_size);
    assert_eq!(desired_size, LayoutSize::zero());

    // arrange does nothing, not really anything to test.
    wgt.test_arrange(&mut ctx, desired_size);

    // we expect default to not render anything.
    let window_id = WindowId::new_unique();
    let root_transform_key = WidgetTransformKey::new_unique();
    let mut frame = FrameBuilder::new_renderless(FrameId::invalid(), window_id, wgt.id(), root_transform_key, desired_size, 1.0);
    wgt.test_render(&mut ctx, &mut frame);
    let (_, frame_info) = frame.finalize();
    let wgt_info = frame_info.find(wgt.id()).unwrap();
    assert!(wgt_info.descendants().next().is_none());
    assert!(wgt_info.meta().is_empty());

    // and not update render..
    let mut update = FrameUpdate::new(window_id, wgt.id(), root_transform_key, FrameId::invalid());
    wgt.test_render_update(&mut ctx, &mut update);
    let update = update.finalize();
    assert!(update.transforms.is_empty());
    assert!(update.floats.is_empty());
}

mod util {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        context::{LayoutContext, RenderContext, WidgetContext},
        event::EventUpdateArgs,
        render::{FrameBuilder, FrameUpdate},
        state_key,
        units::LayoutSize,
        UiNode,
    };

    state_key! {
        pub struct TraceKey: Vec<TraceRef>;
    }

    type TraceRef = Rc<RefCell<Vec<&'static str>>>;

    /// Asserts that only `method` was traced and clears the trace.
    #[macro_export]
    macro_rules! __impl_ui_node_util_assert_only_traced {
        ($state:expr, $method:expr) => {{
            let state = $state;
            let method = $method;
            if let Some(db) = state.get::<util::TraceKey>() {
                for (i, trace_ref) in db.iter().enumerate() {
                    let mut any = false;
                    for trace_entry in trace_ref.borrow_mut().drain(..) {
                        assert_eq!(trace_entry, method, "tracer_0 traced `{}`, expected only `{}`", trace_entry, method);
                        any = true;
                    }
                    assert!(any, "tracer_{} did not trace anything", i);
                }
            } else {
                panic!("no trace initialized");
            }
        }};
    }
    pub use __impl_ui_node_util_assert_only_traced as assert_only_traced;

    /// Asserts that no trace entry was pushed.
    #[macro_export]
    macro_rules! __impl_ui_node_util_assert_did_not_trace {
        ($state:expr) => {{
            let state = $state;
            if let Some(db) = state.get::<util::TraceKey>() {
                for (i, trace_ref) in db.iter().enumerate() {
                    let mut any = false;
                    for trace_entry in trace_ref.borrow().iter() {
                        assert!(any, "tracer_{} traced `{}`, expected nothing", i, trace_entry);
                        any = true;
                    }
                }
            } else {
                panic!("no trace initialized");
            }
        }};
    }
    pub use __impl_ui_node_util_assert_did_not_trace as assert_did_not_trace;

    #[derive(Default)]
    pub struct TraceNode {
        trace: TraceRef,
    }
    impl TraceNode {
        fn trace(&self, method: &'static str) {
            self.trace.borrow_mut().push(method);
        }
    }
    impl UiNode for TraceNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let db = ctx.widget_state.entry::<TraceKey>().or_default();
            assert!(db.iter().all(|t| !Rc::ptr_eq(t, &self.trace)), "TraceNode::init called twice");
            db.push(Rc::clone(&self.trace));

            self.trace("init");
        }

        fn deinit(&mut self, _: &mut WidgetContext) {
            self.trace("deinit");
        }

        fn update(&mut self, _: &mut WidgetContext) {
            self.trace("update");
        }

        fn event<U: EventUpdateArgs>(&mut self, _: &mut WidgetContext, _: &U) {
            self.trace("event");
        }

        fn measure(&mut self, _: &mut LayoutContext, _: LayoutSize) -> LayoutSize {
            self.trace("measure");
            LayoutSize::zero()
        }

        fn arrange(&mut self, _: &mut LayoutContext, _: LayoutSize) {
            self.trace("arrange");
        }

        fn render(&self, _: &mut RenderContext, _: &mut FrameBuilder) {
            self.trace("render");
        }

        fn render_update(&self, _: &mut RenderContext, _: &mut FrameUpdate) {
            self.trace("render_update");
        }
    }
}

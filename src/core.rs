//! Core infrastructure required for creating components and running an app.

pub mod animation;
pub mod app;
pub mod color;
pub mod context;
pub mod debug;
pub mod event;
pub mod focus;
pub mod gesture;
pub mod keyboard;
pub mod mouse;
pub mod profiler;
pub mod render;
pub mod service;
pub mod sync;
pub mod text;
pub mod types;
pub mod units;
pub mod var;
pub mod window;

pub use zero_ui_macros::{impl_ui_node, property, ui_vec, widget, widget_mixin};

use context::{LayoutContext, LazyStateMap, WidgetContext};
use render::{FrameBuilder, FrameUpdate, WidgetTransformKey};
use units::LayoutSize;

use self::units::PixelGridExt;

unique_id! {
    /// Unique id of a widget.
    ///
    /// # Details
    /// Underlying value is a `NonZeroU64` generated using a relaxed global atomic `fetch_add`,
    /// so IDs are unique for the process duration, but order is not guaranteed.
    ///
    /// Panics if you somehow reach `u64::max_value()` calls to `new`.
    pub struct WidgetId;
}

/// An Ui tree node.
pub trait UiNode: 'static {
    /// Called every time the node is plugged in an Ui tree.
    fn init(&mut self, ctx: &mut WidgetContext);

    /// Called every time the node is unplugged from an Ui tree.
    fn deinit(&mut self, ctx: &mut WidgetContext);

    /// Called every time a low pressure event update happens.
    ///
    /// # Event Pressure
    /// See [`update_hp`](UiNode::update_hp) for more information about event pressure rate.
    fn update(&mut self, ctx: &mut WidgetContext);

    /// Called every time a high pressure event update happens.
    ///
    /// # Event Pressure
    /// Some events occur a lot more times then others, for performance reasons this
    /// event source may choose to be propagated in this high-pressure lane.
    ///
    /// Event sources that are high pressure mention this in their documentation.
    fn update_hp(&mut self, ctx: &mut WidgetContext);

    /// Called every time a layout update is needed.
    ///
    /// # Arguments
    /// * `available_size`: The total available size for the node. Can contain positive infinity to
    /// indicate the parent will accommodate [any size](is_layout_any_size). Finite values are pixel aligned.
    /// * `ctx`: Measure context.
    ///
    /// # Return
    /// Return the nodes desired size. Must not contain infinity or NaN. Must be pixel aligned.
    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize;

    /// Called every time a layout update is needed, after [`measure`](UiNode::measure).
    ///
    /// # Arguments
    /// * `final_size`: The size the parent node reserved for the node. Must reposition its contents
    /// to fit this size. The value does not contain infinity or NaNs and is pixel aligned.
    /// TODO args docs.
    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext);

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&self, frame: &mut FrameBuilder);

    /// Called every time a frame can be updated without fully rebuilding.
    ///
    /// # Arguments
    /// * `update`: Contains the frame value updates.
    fn render_update(&self, update: &mut FrameUpdate);

    /// Box this node, unless it is already `Box<dyn UiNode>`.
    fn boxed(self) -> Box<dyn UiNode>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

#[impl_ui_node(delegate: self.as_ref(), delegate_mut: self.as_mut())]
impl UiNode for Box<dyn UiNode> {
    fn boxed(self) -> Box<dyn UiNode> {
        self
    }
}

struct WidgetNode<T: UiNode> {
    id: WidgetId,
    transform_key: WidgetTransformKey,
    state: LazyStateMap,
    child: T,
    size: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode> UiNode for WidgetNode<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        ctx.widget_context(self.id, &mut self.state, |ctx| child.update_hp(ctx));
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        #[cfg(debug_assertions)]
        {
            fn valid_measure(f: f32) -> bool {
                f.is_finite() || is_layout_any_size(f)
            }

            if !valid_measure(available_size.width) || !valid_measure(available_size.height) {
                error_println!(
                    "{:?} `UiNode::measure` called with invalid `available_size: {:?}`, must be finite or `LAYOUT_ANY_SIZE`",
                    self.id,
                    available_size
                );
            }
        }

        let child_size = self.child.measure(available_size, ctx);

        #[cfg(debug_assertions)]
        {
            if !child_size.width.is_finite() || !child_size.height.is_finite() {
                error_println!("{:?} `UiNode::measure` result is not finite: `{:?}`", self.id, child_size);
            } else if !child_size.is_aligned_to(ctx.pixel_grid()) {
                let snapped = child_size.snap_to(ctx.pixel_grid());
                error_println!(
                    "{:?} `UiNode::measure` result not aligned, was: `{:?}`, expected: `{:?}`",
                    self.id,
                    child_size,
                    snapped
                );
                return snapped;
            }
        }
        child_size
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.size = final_size;

        #[cfg(debug_assertions)]
        {
            if !final_size.width.is_finite() || !final_size.height.is_finite() {
                error_println!(
                    "{:?} `UiNode::arrange` called with invalid `final_size: {:?}`, must be finite",
                    self.id,
                    final_size
                );
            } else if !final_size.is_aligned_to(ctx.pixel_grid()) {
                self.size = final_size.snap_to(ctx.pixel_grid());
                error_println!(
                    "{:?} `UiNode::arrange` called with not aligned value, was: `{:?}`, expected: `{:?}`",
                    self.id,
                    final_size,
                    self.size
                );
            }
        }

        self.child.arrange(self.size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_widget(self.id, self.transform_key, self.size, &self.child);
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        update.update_widget(self.id, self.transform_key, &self.child);
    }
}

/// Represents an widget [`UiNode`].
pub trait Widget: UiNode {
    fn id(&self) -> WidgetId;

    fn state(&self) -> &LazyStateMap;
    fn state_mut(&mut self) -> &mut LazyStateMap;

    /// Last arranged size.
    fn size(&self) -> LayoutSize;

    /// Box this widget node, unless it is already `Box<dyn Widget>`.
    fn boxed_widget(self) -> Box<dyn Widget>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

impl<T: UiNode> Widget for WidgetNode<T> {
    #[inline]
    fn id(&self) -> WidgetId {
        self.id
    }
    #[inline]
    fn state(&self) -> &LazyStateMap {
        &self.state
    }
    #[inline]
    fn state_mut(&mut self) -> &mut LazyStateMap {
        &mut self.state
    }
    #[inline]
    fn size(&self) -> LayoutSize {
        self.size
    }
}

#[impl_ui_node(delegate: self.as_ref(), delegate_mut: self.as_mut())]
impl UiNode for Box<dyn Widget> {}

impl Widget for Box<dyn Widget> {
    #[inline]
    fn id(&self) -> WidgetId {
        self.as_ref().id()
    }
    #[inline]
    fn state(&self) -> &LazyStateMap {
        self.as_ref().state()
    }
    #[inline]
    fn state_mut(&mut self) -> &mut LazyStateMap {
        self.as_mut().state_mut()
    }
    #[inline]
    fn size(&self) -> LayoutSize {
        self.as_ref().size()
    }
    #[inline]
    fn boxed_widget(self) -> Box<dyn Widget> {
        self
    }
}

/// A UI node that does not contain any other node, does not take any space and renders nothing.
pub struct NilUiNode;
#[impl_ui_node(none)]
impl UiNode for NilUiNode {
    fn measure(&mut self, _: LayoutSize, _: &mut LayoutContext) -> LayoutSize {
        LayoutSize::zero()
    }
}

/// A UI node that does not contain any other node, fills the available space, but renders nothing.
pub struct FillUiNode;
#[impl_ui_node(none)]
impl UiNode for FillUiNode {}

/// This is called by the default widgets `new_child` function.
///
/// See [widgets](crate::widgets) for more details.
///
/// Returns a [`NilUiNode`].
#[inline]
pub fn default_widget_new_child() -> impl UiNode {
    NilUiNode
}

/// This is called by the default widgets `new` function.
///
/// See [widgets](crate::widgets) for more details.
///
/// A new widget context is introduced by this function. `child` is wrapped in a node that calls
/// [`WidgetContext::widget_context`](WidgetContext::widget_context) and [`FrameBuilder::push_widget`] to define the widget.
#[inline]
pub fn default_widget_new(child: impl UiNode, id_args: impl zero_ui::properties::capture_only::widget_id::Args) -> impl Widget {
    WidgetNode {
        id: id_args.unwrap(),
        transform_key: WidgetTransformKey::new_unique(),
        state: LazyStateMap::default(),
        child,
        size: LayoutSize::zero(),
    }
}

/// Gets if the value indicates that any size is available during layout (positive infinity)
#[inline]
pub fn is_layout_any_size(f: f32) -> bool {
    f.is_infinite() && f.is_sign_positive()
}

/// Value that indicates that any size is available during layout.
pub const LAYOUT_ANY_SIZE: f32 = f32::INFINITY;

/// A mixed vector of [`Widget`] types.
pub type UiVec = Vec<Box<dyn Widget>>;

/// A map of TypeId -> Box<dyn Any>.
type AnyMap = fnv::FnvHashMap<std::any::TypeId, Box<dyn std::any::Any>>;

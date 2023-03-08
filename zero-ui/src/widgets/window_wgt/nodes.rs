//! UI nodes used for building a window widget.

use crate::core::window::{WIDGET_INFO_CHANGED_EVENT, WINDOW_CTRL};
use crate::prelude::new_property::*;

use std::ops;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use crate::crate_util::RunOnDrop;

struct LayersCtx {
    items: EditableUiNodeListRef,
}

/// Windows layers.
///
/// The window layers is z-order stacking panel that fills the window content area, widgets can be inserted
/// with a *z-index* that is the [`LayerIndex`]. The inserted widgets parent is the window root widget and
/// it is affected by the context properties set on the window only.
///
/// # Layout & Render
///
/// Layered widgets are measured and arranged using the same constrains as the window root widget, the desired
/// size is discarded, only the root widget desired size can affect the window size. Layered widgets are all layout
/// and rendered after the window content and from the bottom layer up to the top-most, this means that the [`WidgetBoundsInfo`]
/// of normal widgets are always up-to-date when the layered widget is arranged and rendered, so if you
/// implement custom layouts that align the layered widget with a normal widget using the info values it will always be in sync with
/// a single layout pass, see [`insert_anchored`] for more details.
///
/// [`insert_anchored`]: Self::insert_anchored
pub struct LAYERS;
impl LAYERS {
    /// Insert the `widget` in the layer identified by a [`LayerIndex`].
    ///
    /// If the `layer` variable updates the widget is moved to the new layer, if multiple widgets
    /// are inserted in the same layer the later inserts are on top of the previous.
    pub fn insert(&self, layer: impl IntoVar<LayerIndex>, widget: impl UiNode) {
        struct LayeredWidget<L, W> {
            layer: L,
            widget: W,
        }
        #[ui_node(
                delegate = &self.widget,
                delegate_mut = &mut self.widget,
            )]
        impl<L: Var<LayerIndex>, W: UiNode> UiNode for LayeredWidget<L, W> {
            fn init(&mut self) {
                self.widget.with_context(|| {
                    WIDGET.set_state(&LAYER_INDEX_ID, self.layer.get());
                });
                self.widget.init();
            }

            fn update(&mut self, updates: &mut WidgetUpdates) {
                if let Some(index) = self.layer.get_new() {
                    self.widget.with_context(|| {
                        WIDGET.set_state(&LAYER_INDEX_ID, index);
                    });
                    SortingListParent::invalidate_sort();
                }
                self.widget.update(updates);
            }

            fn with_context<R, F>(&self, f: F) -> Option<R>
            where
                F: FnOnce() -> R,
            {
                self.widget.with_context(f)
            }
        }

        WINDOW.with_state(|s| {
            s.req(&WINDOW_LAYERS_ID).items.push(LayeredWidget {
                layer: layer.into_var(),
                widget: widget.cfg_boxed(),
            });
        });
    }

    /// Insert the `widget` in the layer and *anchor* it to the offset/transform of another widget.
    ///
    /// The `anchor` is the ID of another widget, the inserted `widget` will be offset/transform so that it aligns
    /// with the `anchor` widget top-left. The `mode` is a value of [`AnchorMode`] that defines if the `widget` will
    /// receive the full transform or just the offset.
    ///
    /// If the `anchor` widget is not found the `widget` is not rendered (visibility `Collapsed`).
    pub fn insert_anchored(
        &self,
        layer: impl IntoVar<LayerIndex>,
        anchor: impl IntoVar<WidgetId>,
        mode: impl IntoVar<AnchorMode>,

        widget: impl UiNode,
    ) {
        struct AnchoredWidget<A, M, W> {
            anchor: A,
            mode: M,
            widget: W,
            info_changed_handle: Option<EventHandle>,

            anchor_info: Option<(WidgetBoundsInfo, WidgetBorderInfo)>,
            offset_point: PxPoint,
            interaction: bool,

            transform_key: FrameValueKey<PxTransform>,
            corner_radius_ctx_handle: Option<ContextInitHandle>,
        }
        #[ui_node(
                delegate = &self.widget,
                delegate_mut = &mut self.widget,
            )]
        impl<A, M, W> UiNode for AnchoredWidget<A, M, W>
        where
            A: Var<WidgetId>,
            M: Var<AnchorMode>,
            W: UiNode,
        {
            fn info(&self, info: &mut WidgetInfoBuilder) {
                if self.interaction {
                    if let Some(widget) = self.widget.with_context(|| WIDGET.id()) {
                        let anchor = self.anchor.get();
                        let querying = AtomicBool::new(false);
                        info.push_interactivity_filter(move |args| {
                            if args.info.widget_id() == widget {
                                if querying.swap(true, Ordering::Relaxed) {
                                    return Interactivity::ENABLED; // avoid recursion.
                                }
                                let _q = RunOnDrop::new(|| querying.store(false, Ordering::Relaxed));
                                args.info
                                    .tree()
                                    .get(anchor)
                                    .map(|a| a.interactivity())
                                    .unwrap_or(Interactivity::BLOCKED)
                            } else {
                                Interactivity::ENABLED
                            }
                        });
                    }
                }
                self.widget.info(info)
            }

            fn init(&mut self) {
                let tree = WINDOW.widget_tree();
                if let Some(w) = tree.get(self.anchor.get()) {
                    self.anchor_info = Some((w.bounds_info(), w.border_info()));
                }

                self.interaction = self.mode.with(|m| m.interaction);
                self.info_changed_handle = Some(WIDGET_INFO_CHANGED_EVENT.subscribe(WIDGET.id()));

                self.widget.init();
            }

            fn deinit(&mut self) {
                self.anchor_info = None;
                self.info_changed_handle = None;
                self.widget.deinit();
                self.corner_radius_ctx_handle = None;
            }

            fn event(&mut self, update: &mut EventUpdate) {
                if let Some(args) = WIDGET_INFO_CHANGED_EVENT.on(update) {
                    if args.window_id == WINDOW.id() {
                        self.anchor_info = WINDOW
                            .widget_tree()
                            .get(self.anchor.get())
                            .map(|w| (w.bounds_info(), w.border_info()));
                    }
                }
                self.widget.event(update);
            }

            fn update(&mut self, updates: &mut WidgetUpdates) {
                if let Some(anchor) = self.anchor.get_new() {
                    self.anchor_info = WINDOW.widget_tree().get(anchor).map(|w| (w.bounds_info(), w.border_info()));
                    if self.mode.with(|m| m.interaction) {
                        WIDGET.info();
                    }
                    WIDGET.layout().render();
                }
                if let Some(mode) = self.mode.get_new() {
                    if mode.interaction != self.interaction {
                        self.interaction = mode.interaction;
                        WIDGET.info();
                    }
                    WIDGET.layout().render();
                }
                self.widget.update(updates);
            }

            fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
                if let Some((bounds, border)) = &self.anchor_info {
                    let mode = self.mode.get();

                    if !mode.visibility || bounds.inner_size() != PxSize::zero() {
                        return LAYOUT.with_constrains(
                            |c| match mode.size {
                                AnchorSize::Unbounded => PxConstrains2d::new_unbounded(),
                                AnchorSize::Window => c,
                                AnchorSize::InnerSize => PxConstrains2d::new_exact_size(bounds.inner_size()),
                                AnchorSize::InnerBorder => PxConstrains2d::new_exact_size(border.inner_size(bounds)),
                                AnchorSize::OuterSize => PxConstrains2d::new_exact_size(bounds.outer_size()),
                            },
                            || self.widget.measure(wm),
                        );
                    }
                }

                PxSize::zero()
            }
            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                if let Some((bounds, border)) = &self.anchor_info {
                    let mode = self.mode.get();

                    if !mode.visibility || bounds.inner_size() != PxSize::zero() {
                        // if we don't link visibility or anchor is not collapsed.

                        self.offset_point = match &mode.transform {
                            AnchorTransform::InnerOffset(p) => {
                                LAYOUT.with_constrains(|_| PxConstrains2d::new_exact_size(bounds.inner_size()), || p.layout())
                            }
                            AnchorTransform::InnerBorderOffset(p) => {
                                LAYOUT.with_constrains(|_| PxConstrains2d::new_exact_size(border.inner_size(bounds)), || p.layout())
                            }
                            AnchorTransform::OuterOffset(p) => {
                                LAYOUT.with_constrains(|_| PxConstrains2d::new_exact_size(bounds.outer_size()), || p.layout())
                            }
                            _ => PxPoint::zero(),
                        };

                        return LAYOUT.with_constrains(
                            |c| match mode.size {
                                AnchorSize::Unbounded => PxConstrains2d::new_unbounded(),
                                AnchorSize::Window => c,
                                AnchorSize::InnerSize => PxConstrains2d::new_exact_size(bounds.inner_size()),
                                AnchorSize::InnerBorder => PxConstrains2d::new_exact_size(border.inner_size(bounds)),
                                AnchorSize::OuterSize => PxConstrains2d::new_exact_size(bounds.outer_size()),
                            },
                            || {
                                if mode.corner_radius {
                                    let mut cr = border.corner_radius();
                                    if let AnchorSize::InnerBorder = mode.size {
                                        cr = cr.deflate(border.offsets());
                                    }
                                    CORNER_RADIUS_VAR.with_context_var(
                                        self.corner_radius_ctx_handle.get_or_insert_with(ContextInitHandle::new).clone(),
                                        cr,
                                        || BORDER.with_corner_radius(|| self.widget.layout(wl)),
                                    )
                                } else {
                                    self.widget.layout(wl)
                                }
                            },
                        );
                    }
                }

                // no visible mode match
                wl.collapse();
                PxSize::zero()
            }

            fn render(&self, frame: &mut FrameBuilder) {
                if let Some((bounds_info, border_info)) = &self.anchor_info {
                    let mode = self.mode.get();
                    if !mode.visibility || bounds_info.rendered().is_some() {
                        match mode.transform {
                            AnchorTransform::InnerOffset(_) => {
                                let point_in_window = bounds_info.inner_transform().transform_point(self.offset_point).unwrap_or_default();

                                frame.push_reference_frame(
                                    self.transform_key.into(),
                                    self.transform_key.bind(PxTransform::from(point_in_window.to_vector()), true),
                                    true,
                                    false,
                                    |frame| self.widget.render(frame),
                                )
                            }
                            AnchorTransform::InnerBorderOffset(_) => {
                                let point_in_window = border_info
                                    .inner_transform(bounds_info)
                                    .transform_point(self.offset_point)
                                    .unwrap_or_default();
                                frame.push_reference_frame(
                                    self.transform_key.into(),
                                    self.transform_key.bind(PxTransform::from(point_in_window.to_vector()), true),
                                    true,
                                    false,
                                    |frame| self.widget.render(frame),
                                )
                            }
                            AnchorTransform::OuterOffset(_) => {
                                let point_in_window = bounds_info.outer_transform().transform_point(self.offset_point).unwrap_or_default();
                                frame.push_reference_frame(
                                    self.transform_key.into(),
                                    self.transform_key.bind(PxTransform::from(point_in_window.to_vector()), true),
                                    true,
                                    false,
                                    |frame| self.widget.render(frame),
                                )
                            }
                            AnchorTransform::InnerTransform => frame.push_reference_frame(
                                self.transform_key.into(),
                                self.transform_key.bind(bounds_info.inner_transform(), true),
                                false,
                                false,
                                |frame| self.widget.render(frame),
                            ),
                            AnchorTransform::InnerBorderTransform => frame.push_reference_frame(
                                self.transform_key.into(),
                                self.transform_key.bind(border_info.inner_transform(bounds_info), true),
                                false,
                                false,
                                |frame| self.widget.render(frame),
                            ),
                            AnchorTransform::OuterTransform => frame.push_reference_frame(
                                self.transform_key.into(),
                                self.transform_key.bind(bounds_info.outer_transform(), true),
                                false,
                                false,
                                |frame| self.widget.render(frame),
                            ),
                            _ => self.widget.render(frame),
                        }
                    }
                }
            }

            fn render_update(&self, update: &mut FrameUpdate) {
                if let Some((bounds_info, border_info)) = &self.anchor_info {
                    let mode = self.mode.get();
                    if !mode.visibility || bounds_info.rendered().is_some() {
                        match mode.transform {
                            AnchorTransform::InnerOffset(_) => {
                                let point_in_window = bounds_info.inner_transform().transform_point(self.offset_point).unwrap_or_default();
                                update.with_transform(
                                    self.transform_key.update(PxTransform::from(point_in_window.to_vector()), true),
                                    false,
                                    |update| self.widget.render_update(update),
                                )
                            }
                            AnchorTransform::InnerBorderOffset(_) => {
                                let point_in_window = border_info
                                    .inner_transform(bounds_info)
                                    .transform_point(self.offset_point)
                                    .unwrap_or_default();
                                update.with_transform(
                                    self.transform_key.update(PxTransform::from(point_in_window.to_vector()), true),
                                    false,
                                    |update| self.widget.render_update(update),
                                )
                            }
                            AnchorTransform::OuterOffset(_) => {
                                let point_in_window = bounds_info.outer_transform().transform_point(self.offset_point).unwrap_or_default();
                                update.with_transform(
                                    self.transform_key.update(PxTransform::from(point_in_window.to_vector()), true),
                                    false,
                                    |update| self.widget.render_update(update),
                                )
                            }
                            AnchorTransform::InnerTransform => {
                                update.with_transform(self.transform_key.update(bounds_info.inner_transform(), true), false, |update| {
                                    self.widget.render_update(update)
                                });
                            }
                            AnchorTransform::InnerBorderTransform => {
                                update.with_transform(
                                    self.transform_key.update(border_info.inner_transform(bounds_info), true),
                                    false,
                                    |update| self.widget.render_update(update),
                                );
                            }
                            AnchorTransform::OuterTransform => {
                                update.with_transform(self.transform_key.update(bounds_info.outer_transform(), true), false, |update| {
                                    self.widget.render_update(update)
                                });
                            }
                            _ => self.widget.render_update(update),
                        }
                    }
                }
            }

            fn with_context<R, F>(&self, f: F) -> Option<R>
            where
                F: FnOnce() -> R,
            {
                self.widget.with_context(f)
            }
        }

        self.insert(
            layer,
            AnchoredWidget {
                anchor: anchor.into_var(),
                mode: mode.into_var(),
                widget: widget.cfg_boxed(),
                info_changed_handle: None,

                anchor_info: None,
                offset_point: PxPoint::zero(),
                interaction: false,

                transform_key: FrameValueKey::new_unique(),
                corner_radius_ctx_handle: None,
            },
        );
    }

    /// Remove the widget from the layers overlay in the next update.
    ///
    /// The `id` must the widget id of a previous inserted widget, nothing happens if the widget is not found.
    pub fn remove(&self, id: impl Into<WidgetId>) {
        WINDOW.with_state(|s| {
            s.req(&WINDOW_LAYERS_ID).items.remove(id);
        });
    }
}

static WINDOW_LAYERS_ID: StaticStateId<LayersCtx> = StaticStateId::new_unique();
static LAYER_INDEX_ID: StaticStateId<LayerIndex> = StaticStateId::new_unique();

/// Wrap around the window outer-most event node to create the layers.
///
/// This node is included in the [`NestGroup::EVENT`] group.
///
/// [`NestGroup::EVENT`]: crate::core::widget_builder::NestGroup::EVENT
pub fn layers(child: impl UiNode) -> impl UiNode {
    #[ui_node(struct LayersNode {
        children: impl UiNodeList,
        layered: EditableUiNodeListRef,
    })]
    impl UiNode for LayersNode {
        fn init(&mut self) {
            WINDOW.set_state(
                &WINDOW_LAYERS_ID,
                LayersCtx {
                    items: self.layered.clone(),
                },
            );

            self.children.init_all();
        }

        fn update(&mut self, updates: &mut WidgetUpdates) {
            let mut changed = false;

            self.children.update_all(updates, &mut changed);

            if changed {
                WIDGET.layout().render();
            }
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.children.with_node(0, |n| n.measure(wm))
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let mut size = PxSize::zero();
            self.children.for_each_mut(|i, n| {
                let s = n.layout(wl);
                if i == 0 {
                    size = s;
                }
                true
            });
            size
        }
    }

    let layers = EditableUiNodeList::new();
    let layered = layers.reference();

    let sorting_layers = SortingList::new(layers, |a, b| {
        let a = a.with_context(|| WIDGET.req_state(&LAYER_INDEX_ID)).unwrap_or(LayerIndex::DEFAULT);
        let b = b.with_context(|| WIDGET.req_state(&LAYER_INDEX_ID)).unwrap_or(LayerIndex::DEFAULT);

        a.cmp(&b)
    });

    LayersNode {
        children: ui_vec![child].chain(sorting_layers),
        layered,
    }
    .cfg_boxed()
}

/// Represents a layer in a window.
///
/// See [`LAYERS`] for more information.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct LayerIndex(pub u32);
impl LayerIndex {
    /// The top-most layer.
    ///
    /// Only widgets that are pretending to be a child window should use this layer, including menus,
    /// drop-downs, pop-ups and tool-tips.
    ///
    /// This is the [`u32::MAX`] value.
    pub const TOP_MOST: LayerIndex = LayerIndex(u32::MAX);

    /// The layer for *adorner* display items.
    ///
    /// Adorner widgets are related to another widget but not as a visual part of it, examples of adorners
    /// are resize handles in a widget visual editor, or an interactive help/guide feature.
    ///
    /// This is the [`TOP_MOST - u16::MAX`] value.
    pub const ADORNER: LayerIndex = LayerIndex(Self::TOP_MOST.0 - u16::MAX as u32);

    /// The default layer, just above the normal window content.
    ///
    /// This is the `0` value.
    pub const DEFAULT: LayerIndex = LayerIndex(0);

    /// Compute `self + other` saturating at the [`TOP_MOST`] bound instead of overflowing.
    ///
    /// [`TOP_MOST`]: Self::TOP_MOST
    pub fn saturating_add(self, other: impl Into<LayerIndex>) -> Self {
        Self(self.0.saturating_add(other.into().0))
    }

    /// Compute `self - other` saturating at the [`DEFAULT`] bound instead of overflowing.
    ///
    /// [`DEFAULT`]: Self::DEFAULT
    pub fn saturating_sub(self, other: impl Into<LayerIndex>) -> Self {
        Self(self.0.saturating_sub(other.into().0))
    }
}
impl_from_and_into_var! {
    fn from(index: u32) -> LayerIndex {
        LayerIndex(index)
    }
}
/// Calls [`LayerIndex::saturating_add`].
impl<T: Into<Self>> ops::Add<T> for LayerIndex {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        self.saturating_add(rhs)
    }
}
/// Calls [`LayerIndex::saturating_sub`].
impl<T: Into<Self>> ops::Sub<T> for LayerIndex {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        self.saturating_sub(rhs)
    }
}
/// Calls [`LayerIndex::saturating_add`].
impl<T: Into<Self>> ops::AddAssign<T> for LayerIndex {
    fn add_assign(&mut self, rhs: T) {
        *self = *self + rhs;
    }
}
/// Calls [`LayerIndex::saturating_sub`].
impl<T: Into<Self>> ops::SubAssign<T> for LayerIndex {
    fn sub_assign(&mut self, rhs: T) {
        *self = *self - rhs;
    }
}
impl ops::Mul<Factor> for LayerIndex {
    type Output = Self;

    fn mul(self, rhs: Factor) -> Self::Output {
        LayerIndex(self.0 * rhs)
    }
}
impl ops::MulAssign<Factor> for LayerIndex {
    fn mul_assign(&mut self, rhs: Factor) {
        self.0 *= rhs;
    }
}
impl ops::Div<Factor> for LayerIndex {
    type Output = Self;

    fn div(self, rhs: Factor) -> Self::Output {
        LayerIndex(self.0 / rhs)
    }
}
impl ops::DivAssign<Factor> for LayerIndex {
    fn div_assign(&mut self, rhs: Factor) {
        self.0 /= rhs;
    }
}

/// Options for [`AnchorMode::transform`].
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorTransform {
    /// Widget does not copy any position from the anchor widget.
    None,
    /// The point is resolved in the inner space of the anchor widget, transformed to the window space
    /// and then applied as a translate offset.
    InnerOffset(Point),
    /// The point is resolved in the inner space of the anchor widget offset by the anchor border widths, transformed
    /// to the window space and then applied as a translate offset.
    InnerBorderOffset(Point),

    /// The point is resolved in the outer space of the anchor widget, transformed to the window space
    /// and then applied as a translate offset.
    OuterOffset(Point),

    /// The full inner transform of the anchor object is applied to the widget.
    InnerTransform,

    /// The full inner transform of the anchor object is applied to the widget plus the border widths offset.
    InnerBorderTransform,

    /// The full outer transform of the anchor object is applied to the widget.
    OuterTransform,
}
impl_from_and_into_var! {
    /// `InnerOffset`.
    fn from(inner_offset: Point) -> AnchorTransform {
        AnchorTransform::InnerOffset(inner_offset)
    }
    /// `InnerOffset`.
    fn from<X: Into<Length>, Y: Into<Length>>(inner_offset: (X, Y)) -> AnchorTransform {
        Point::from(inner_offset).into()
    }
    /// `InnerOffset`.
    fn from(inner_offset: PxPoint) -> AnchorTransform {
        Point::from(inner_offset).into()
    }
    /// `InnerOffset`.
    fn from(inner_offset: DipPoint) -> AnchorTransform {
        Point::from(inner_offset).into()
    }
}

/// Options for [`AnchorMode::size`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorSize {
    /// Widget does not copy any size from the anchor widget, the available size is infinite, the
    /// final size is the desired size.
    ///
    /// Note that layered widgets do not affect the window size and a widget that overflows the content
    /// boundaries is clipped.
    Unbounded,
    /// Widget does not copy any size from the anchor widget, the available size and final size
    /// are the window's root size.
    Window,
    /// The available size and final size is the anchor widget's outer size.
    OuterSize,
    /// The available size and final size is the anchor widget's inner size.
    InnerSize,
    /// The available size and final size is the anchor widget's inner size offset by the border widths.
    InnerBorder,
}

/// Defines what properties the layered widget takes from the anchor widget.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorMode {
    /// What transforms are copied from the anchor widget and applied as a *parent* transform of the widget.
    pub transform: AnchorTransform,
    /// What size is copied from the anchor widget and used as the available size and final size of the widget.
    pub size: AnchorSize,
    /// If the widget is only layout if the anchor widget is not [`Collapsed`] and is only rendered
    /// if the anchor widget is rendered.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub visibility: bool,
    /// The widget [`interactivity`] is set to the the same as the anchor widget.
    ///
    /// [`interactivity`]: crate::core::widget_info::WidgetInfo::interactivity
    pub interaction: bool,

    /// The widget's corner radius is set for the layer.
    ///
    /// If `size` is [`InnerBorder`] the corner radius are deflated to fit the *inner* curve of the borders.
    ///
    /// [`InnerBorder`]: AnchorSize::InnerBorder
    pub corner_radius: bool,
}
impl AnchorMode {
    /// Mode where widget behaves like an unanchored widget, except that it is still only
    /// layout an rendered if the anchor widget exists in the same window.
    pub fn none() -> Self {
        AnchorMode {
            transform: AnchorTransform::None,
            size: AnchorSize::Window,
            visibility: false,
            interaction: false,
            corner_radius: false,
        }
    }

    /// Mode where the widget behaves like a [`foreground`] to the target widget.
    ///
    /// [`foreground`]: fn@crate::properties::foreground
    pub fn foreground() -> Self {
        AnchorMode {
            transform: AnchorTransform::InnerTransform,
            size: AnchorSize::InnerSize,
            visibility: true,
            interaction: false,
            corner_radius: true,
        }
    }

    /// Returns the mode with `transform` set.
    pub fn with_transform(mut self, transform: impl Into<AnchorTransform>) -> Self {
        self.transform = transform.into();
        self
    }

    /// Returns the mode with `size` set.
    pub fn with_size(mut self, size: impl Into<AnchorSize>) -> Self {
        self.size = size.into();
        self
    }

    /// Returns the mode with `visibility` set.
    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    /// Returns the mode with `interaction` set.
    pub fn with_interaction(mut self, interaction: bool) -> Self {
        self.interaction = interaction;
        self
    }

    /// Returns the mode with `corner_radius` set.
    pub fn with_corner_radius(mut self, corner_radius: bool) -> Self {
        self.corner_radius = corner_radius;
        self
    }
}
impl Default for AnchorMode {
    /// Transform `InnerOffset` top-left, size infinite, copy visibility and corner-radius.
    fn default() -> Self {
        AnchorMode {
            transform: AnchorTransform::InnerOffset(Point::top_left()),
            size: AnchorSize::Unbounded,
            visibility: true,
            interaction: false,
            corner_radius: true,
        }
    }
}
impl_from_and_into_var! {
    /// Custom transform, all else default.
    fn from(transform: AnchorTransform) -> AnchorMode {
        AnchorMode {
            transform,
            ..AnchorMode::default()
        }
    }
    /// Transform `InnerOffset`, all else default.
    fn from(inner_offset: Point) -> AnchorMode {
        AnchorTransform::from(inner_offset).into()
    }
    /// Transform `InnerOffset`, all else default.
    fn from(inner_offset: PxPoint) -> AnchorMode {
        AnchorTransform::from(inner_offset).into()
    }
    /// Transform `InnerOffset`, all else default.
    fn from(inner_offset: DipPoint) -> AnchorMode {
        AnchorTransform::from(inner_offset).into()
    }

    /// Custom transform and size, all else default.
    fn from<T: Into<AnchorTransform>, S: Into<AnchorSize>>((transform, size): (T, S)) -> AnchorMode {
        AnchorMode {
            transform: transform.into(),
            size: size.into(),
            ..AnchorMode::default()
        }
    }
}

/// Defines if a widget load affects the parent window load.
///
/// Widgets that support this behavior have a `block_window_load` property.
#[derive(Clone, Copy, Debug)]
pub enum BlockWindowLoad {
    /// Widget requests a [`WindowLoadingHandle`] and retains it until the widget is loaded.
    ///
    /// [`WindowLoadingHandle`]: crate::core::window::WindowLoadingHandle
    Enabled {
        /// Handle expiration deadline, if the widget takes longer than this deadline the window loads anyway.
        deadline: Deadline,
    },
    /// Widget does not hold back window load.
    Disabled,
}
impl BlockWindowLoad {
    /// Enabled value.
    pub fn enabled(deadline: impl Into<Deadline>) -> BlockWindowLoad {
        BlockWindowLoad::Enabled { deadline: deadline.into() }
    }

    /// Returns `true` if is enabled.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    /// Returns `true` if is disabled.
    pub fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// Returns the block deadline if is enabled and the deadline has not expired.
    pub fn deadline(self) -> Option<Deadline> {
        match self {
            BlockWindowLoad::Enabled { deadline } => {
                if deadline.has_elapsed() {
                    None
                } else {
                    Some(deadline)
                }
            }
            BlockWindowLoad::Disabled => None,
        }
    }
}
impl_from_and_into_var! {
    /// Converts `true` to `BlockWindowLoad::enabled(1.secs())` and `false` to `BlockWindowLoad::Disabled`.
    fn from(enabled: bool) -> BlockWindowLoad {
        if enabled {
            BlockWindowLoad::enabled(1.secs())
        } else {
            BlockWindowLoad::Disabled
        }
    }

    /// Converts to enabled with the duration timeout.
    fn from(enabled_timeout: Duration) -> BlockWindowLoad {
        BlockWindowLoad::enabled(enabled_timeout)
    }
}

/// Node that binds the [`COLOR_SCHEME_VAR`] to the [`actual_color_scheme`].
///
/// [`actual_color_scheme`]: crate::core::window::WindowVars::actual_color_scheme
pub fn color_scheme(child: impl UiNode) -> impl UiNode {
    with_context_var_init(child, COLOR_SCHEME_VAR, || WINDOW_CTRL.vars().actual_color_scheme().boxed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn layer_index_ops() {
        let idx = LayerIndex::DEFAULT;

        let p1 = idx + 1;
        let m1 = idx - 1;

        let mut idx = idx;

        idx += 1;
        assert_eq!(idx, p1);

        idx -= 2;
        assert_eq!(idx, m1);
    }
}

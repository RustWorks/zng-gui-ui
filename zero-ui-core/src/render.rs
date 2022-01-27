//! Frame render and metadata API.

use crate::{
    app::view_process::ViewRenderer,
    border::BorderSides,
    color::{RenderColor, RenderFilter},
    gradient::{RenderExtendMode, RenderGradientStop},
    units::*,
    var::impl_from_and_into_var,
    widget_info::WidgetRendered,
    window::WindowId,
    WidgetId,
};

use std::{marker::PhantomData, mem};

pub use zero_ui_view_api::webrender_api;

use webrender_api::*;

pub use zero_ui_view_api::FrameId;

/// A text font.
///
/// This trait is an interface for the renderer into the font API used in the application.
///
/// # Font API
///
/// The default font API is provided by [`FontManager`] that is included
/// in the app default extensions. The default font type is [`Font`] that implements this trait.
///
/// [`FontManager`]: crate::text::FontManager
/// [`Font`]: crate::text::Font
pub trait Font {
    /// Gets the instance key in the `renderer` namespace.
    ///
    /// The font configuration must be provided by `self`, except the `synthesis` that is used in the font instance.
    fn instance_key(&self, renderer: &ViewRenderer, synthesis: FontSynthesis) -> webrender_api::FontInstanceKey;
}

/// A loaded or loading image.
///
/// This trait is an interface for the renderer into the image API used in the application.
///
/// The ideal image format is BGRA with pre-multiplied alpha.
///
/// # Image API
///
/// The default image API is provided by [`ImageManager`] that is included
/// in the app default extensions. The default image type is [`Image`] that implements this trait.
///
/// [`ImageManager`]: crate::image::ImageManager
/// [`Image`]: crate::image::Image
pub trait Image {
    /// Gets the image key in the `renderer` namespace.
    ///
    /// The image must be loaded asynchronously by `self` and does not need to
    /// be loaded yet when the key is returned.
    fn image_key(&self, renderer: &ViewRenderer) -> webrender_api::ImageKey;

    /// Returns a value that indicates if the image is already pre-multiplied.
    ///
    /// The faster option is pre-multiplied, that is also the default return value.
    fn alpha_type(&self) -> webrender_api::AlphaType {
        webrender_api::AlphaType::PremultipliedAlpha
    }
}

/// Image scaling algorithm in the renderer.
///
/// If an image is not rendered at the same size as their source it must be up-scaled or
/// down-scaled. The algorithms used for this scaling can be selected using this `enum`.
///
/// Note that the algorithms used in the renderer value performance over quality and do a good
/// enough job for small or temporary changes in scale only, such as a small size correction or a scaling animation.
/// If and image is constantly rendered at a different scale you should considered scaling it on the CPU using a
/// slower but more complex algorithm or pre-scaling it before including in the app.
///
/// You can use the [`Image`] type to re-scale an image, image widgets probably can be configured to do this too.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImageRendering {
    /// Let the renderer select the algorithm, currently this is the same as [`CrispEdges`].
    ///
    /// [`CrispEdges`]: ImageRendering::CrispEdges
    Auto = 0,
    /// The image is scaled with an algorithm that preserves contrast and edges in the image,
    /// and which does not smooth colors or introduce blur to the image in the process.
    ///
    /// Currently the [Bilinear] interpolation algorithm is used.
    ///
    /// [Bilinear]: https://en.wikipedia.org/wiki/Bilinear_interpolation
    CrispEdges = 1,
    /// When scaling the image up, the image appears to be composed of large pixels.
    ///
    /// Currently the [Nearest-neighbor] interpolation algorithm is used.
    ///
    /// [Nearest-neighbor]: https://en.wikipedia.org/wiki/Nearest-neighbor_interpolation
    Pixelated = 2,
}
impl From<ImageRendering> for webrender_api::ImageRendering {
    fn from(r: ImageRendering) -> Self {
        use webrender_api::ImageRendering::*;
        match r {
            ImageRendering::Auto => Auto,
            ImageRendering::CrispEdges => CrispEdges,
            ImageRendering::Pixelated => Pixelated,
        }
    }
}

macro_rules! expect_inner {
    ($self:ident.$fn_name:ident) => {
        if $self.is_outer() {
            tracing::error!("called `{}` in outer context of `{}`", stringify!($fn_name), $self.widget_id);
        }
    };
}

struct WidgetData {
    filter: RenderFilter,
    flags: PrimitiveFlags,
}

/// A full frame builder.
pub struct FrameBuilder {
    frame_id: FrameId,
    pipeline_id: PipelineId,
    widget_id: WidgetId,

    renderer: Option<ViewRenderer>,

    scale_factor: Factor,

    display_list: DisplayListBuilder,

    hit_testable: bool,

    widget_data: Option<WidgetData>,
    widget_rendered: bool,

    clip_id: ClipId,
    spatial_id: SpatialId,

    clear_color: Option<RenderColor>,
}
impl FrameBuilder {
    /// New builder.
    ///
    /// * `frame_id` - Id of the new frame.
    /// * `widget_id` - Id of the window root widget.
    /// * `renderer` - Connection to the renderer connection that will render the frame, is `None` in renderless mode.
    /// * `scale_factor` - Scale factor that will be used to render the frame, usually the scale factor of the screen the window is at.
    /// * `used_data` - Data generated by a previous frame buffer, if set is recycled for a performance boost.
    /// because WebRender does not let us change the initial clear color.
    #[inline]
    pub fn new(
        frame_id: FrameId,
        root_id: WidgetId,
        renderer: Option<ViewRenderer>,
        scale_factor: Factor,
        used_data: Option<UsedFrameBuilder>,
    ) -> Self {
        let pipeline_id = renderer
            .as_ref()
            .and_then(|r| r.pipeline_id().ok())
            .unwrap_or_else(PipelineId::dummy);

        let mut display_list;
        let mut used = None;

        if let Some(u) = used_data {
            if u.pipeline_id() == pipeline_id {
                used = Some(u.display_list);
            }
        }

        if let Some(reuse) = used {
            display_list = reuse;
        } else {
            display_list = DisplayListBuilder::new(pipeline_id);
        }

        display_list.begin();

        let spatial_id = SpatialId::root_reference_frame(pipeline_id);
        FrameBuilder {
            frame_id,
            widget_id: root_id,
            pipeline_id,
            renderer,
            scale_factor,
            display_list,
            hit_testable: true,
            widget_data: Some(WidgetData {
                filter: vec![],
                flags: PrimitiveFlags::empty(),
            }),
            widget_rendered: false,

            clip_id: ClipId::root(pipeline_id),
            spatial_id,

            clear_color: None,
        }
    }

    /// [`new`](Self::new) with only the inputs required for renderless mode.
    pub fn new_renderless(frame_id: FrameId, root_id: WidgetId, scale_factor: Factor, hint: Option<UsedFrameBuilder>) -> Self {
        Self::new(frame_id, root_id, None, scale_factor, hint)
    }

    /// Pixel scale factor used by the renderer.
    ///
    /// All layout values are scaled by this factor in the renderer.
    #[inline]
    pub fn scale_factor(&self) -> Factor {
        self.scale_factor
    }

    /// Direct access to the current layer display list builder.
    ///
    /// # Careful
    ///
    /// This provides direct access to the underlying WebRender display list builder, modifying it
    /// can interfere with the working of the [`FrameBuilder`].
    ///
    /// Call [`open_widget_display`] before modifying the display list.
    ///
    /// Check the [`FrameBuilder`] source code before modifying the display list.
    ///
    /// Don't try to render using the [`FrameBuilder`] methods inside a custom clip or space, the methods will still
    /// use the [`clip_id`] and [`spatial_id`]. Custom items added to the display list
    /// should be self-contained and completely custom.
    ///
    /// If [`is_cancelling_widget`] don't modify the display list and try to
    /// early return pretending the operation worked.
    ///
    /// Call [`widget_rendered`] if you push anything to the display list.
    ///
    /// Only push hit-tests if [`is_hit_testable`] is `true`
    ///
    /// # WebRender
    ///
    /// The [`webrender`] crate used in the renderer is re-exported in `zero_ui_core::render::webrender`, and the
    /// [`webrender_api`] is re-exported in `webrender::api`.
    ///
    /// [`open_widget_display`]: Self::open_widget_display
    /// [`clip_id`]: Self::clip_id
    /// [`spatial_id`]: Self::spatial_id
    /// [`is_hit_testable`]: Self::is_hit_testable
    /// [`is_cancelling_widget`]: Self::is_cancelling_widget
    /// [`widget_rendered`]: Self::widget_rendered
    /// [`webrender`]: https://docs.rs/webrender
    /// [`webrender_api`]: https://docs.rs/webrender_api
    #[inline]
    pub fn display_list(&mut self) -> &mut DisplayListBuilder {
        &mut self.display_list
    }

    /// Indicates that something was rendered to [`display_list`].
    ///
    /// Note that only direct modification of [`display_list`] requires this method being called,
    /// the other rendering methods of this builder already flag this.
    ///
    /// [`display_list`]: Self::display_list
    pub fn widget_rendered(&mut self) {
        self.widget_rendered = true;
    }

    /// If is building a frame for a headless and renderless window.
    ///
    /// In this mode only the meta and layout information will be used as a *frame*. Methods still
    /// push to the [`display_list`](Self::display_list) when possible, custom methods should ignore this
    /// unless they need access to the [`renderer`](Self::renderer).
    #[inline]
    pub fn is_renderless(&self) -> bool {
        self.renderer.is_none()
    }

    /// Set the color used to clear the pixel frame before drawing this frame.
    ///
    /// Note the default clear color is white, and it is not retained, a property
    /// that sets the clear color must set it every render.
    ///
    /// Note that the clear color is always *rendered* first before all other layers, if more then
    /// one layer sets the clear color only the value set on the top-most layer is used.
    #[inline]
    pub fn set_clear_color(&mut self, color: RenderColor) {
        self.clear_color = Some(color);
    }

    /// Connection to the renderer that will render this frame.
    ///
    /// Returns `None` when in [renderless](Self::is_renderless) mode.
    #[inline]
    pub fn renderer(&self) -> Option<&ViewRenderer> {
        self.renderer.as_ref()
    }

    /// Id of the frame being build.
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Id of the current widget context.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Renderer pipeline ID or [`dummy`].
    ///
    /// [`dummy`]: PipelineId::dummy
    #[inline]
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Current clipping node.
    #[inline]
    pub fn clip_id(&self) -> ClipId {
        self.clip_id
    }

    /// Current spatial node.
    #[inline]
    pub fn spatial_id(&self) -> SpatialId {
        self.spatial_id
    }

    /// Current widget [`ItemTag`]. The first number is the raw [`widget_id`], the second number is reserved.
    ///
    /// For more details on how the ItemTag is used see [`FrameHitInfo::new`].
    ///
    /// [`widget_id`]: Self::widget_id
    #[inline]
    pub fn item_tag(&self) -> ItemTag {
        (self.widget_id.get(), 0)
    }

    /// Common item properties given a `clip_rect` and the current context.
    ///
    /// This is a common case helper, it also calls [`widget_rendered`].
    ///
    /// [`widget_rendered`]: Self::widget_rendered
    #[inline]
    pub fn common_item_ps(&mut self, clip_rect: PxRect) -> CommonItemProperties {
        self.widget_rendered();
        CommonItemProperties {
            clip_rect: clip_rect.to_wr(),
            clip_id: self.clip_id,
            spatial_id: self.spatial_id,
            flags: PrimitiveFlags::empty(),
        }
    }

    /// Generate a [`common_item_ps`] and pushes a hit-test [`item_tag`] if hit-testing is enabled.
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    /// [`item_tag`]: FrameBuilder::item_tag
    #[inline]
    pub fn common_hit_item_ps(&mut self, clip_rect: PxRect) -> CommonItemProperties {
        let item = self.common_item_ps(clip_rect);

        if self.is_hit_testable() {
            self.display_list.push_hit_test(&item, self.item_tag());
        }
        item
    }

    /// Returns `true` if hit-testing is enabled in the widget context, methods that use [`common_hit_item_ps`] automatically
    /// observe this value, custom display item implementer must also respect it.
    ///
    /// [`common_hit_item_ps`]: Self::common_hit_item_ps
    #[inline]
    pub fn is_hit_testable(&self) -> bool {
        self.hit_testable
    }

    /// Runs `f` while hit-tests are disabled, inside `f` [`is_hit_testable`] is `false`, after
    /// it is the current value.
    ///
    /// [`is_hit_testable`]: Self::is_hit_testable
    pub fn with_hit_tests_disabled(&mut self, f: impl FnOnce(&mut Self)) {
        let prev = mem::replace(&mut self.hit_testable, false);
        f(self);
        self.hit_testable = prev;
    }

    /// Start a new widget outer context, this sets [`is_outer`] to `true` until an inner call to [`push_inner`],
    /// during this period properties can configure the widget stacking context and actual rendering and transforms
    /// are discouraged.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    pub fn push_widget(&mut self, widget_id: WidgetId, rendered: &WidgetRendered, f: impl FnOnce(&mut Self)) {
        if self.widget_data.is_some() {
            tracing::error!(
                "called `push_widget` for `{widget_id}` without calling `push_inner` for the parent `{}`",
                self.widget_id
            );
        }

        let parent_rendered = mem::take(&mut self.widget_rendered);
        self.widget_data = Some(WidgetData {
            filter: vec![],
            flags: PrimitiveFlags::empty(),
        });

        f(self);

        self.widget_data = None;
        rendered.set(self.widget_rendered);
        self.widget_rendered |= parent_rendered;
    }

    /// Returns `true`  if the widget stacking context is still being build.
    ///
    /// This is `true` when inside a [`push_widget`] call but `false` when inside a [`push_inner`] call.
    ///
    /// [`push_widget`]: Self::push_widget
    /// [`push_inner`]: Self::push_inner
    pub fn is_outer(&self) -> bool {
        self.widget_data.is_some()
    }

    /// Includes a widget filter and continues the render build.
    ///
    /// This is `Ok(_)` only when not [`is_outer`].
    ///
    /// When [`push_inner`] is called a stacking context is created for the widget that includes the `filter`.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    #[inline]
    pub fn push_inner_filter(&mut self, filter: RenderFilter, f: impl FnOnce(&mut Self)) {
        if let Some(data) = self.widget_data.as_mut() {
            let mut filter = filter;
            filter.reverse(); // see `Self::open_widget_display` for why it is reversed.
            data.filter.extend(filter.iter().copied());

            f(self);
        } else {
            tracing::error!("called `push_inner_filter` inside inner context of `{}`", self.widget_id);
            f(self);
        }
    }

    /// Includes a widget opacity filter and continues the render build.
    ///
    /// This is `Ok(_)` only when not [`is_outer`].
    ///
    /// When [`push_inner`] is called a stacking context is created for the widget that includes the opacity filter.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    #[inline]
    pub fn push_inner_opacity(&mut self, bind: FrameBinding<f32>, f: impl FnOnce(&mut Self)) {
        if let Some(data) = self.widget_data.as_mut() {
            let value = match &bind {
                PropertyBinding::Value(v) => *v,
                PropertyBinding::Binding(_, v) => *v,
            };

            let filter = vec![FilterOp::Opacity(bind, value)];
            data.filter.push(filter[0]);

            f(self);
        } else {
            tracing::error!("called `push_inner_opacity` inside inner context of `{}`", self.widget_id);
            f(self);
        }
    }

    /// Include the `flags` on the widget stacking context flags.
    ///
    /// This is `Ok(_)` only when not [`is_outer`].
    ///
    /// When [`push_inner`] is called a stacking context is created for the widget that includes the `flags`.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    #[inline]
    pub fn push_inner_flags(&mut self, flags: PrimitiveFlags, f: impl FnOnce(&mut Self)) {
        if let Some(data) = self.widget_data.as_mut() {
            data.flags |= flags;
            f(self);
        } else {
            tracing::error!("called `push_inner_flags` inside inner context of `{}`", self.widget_id);
            f(self);
        }
    }

    /// Push the widget reference frame and stacking context then call `f` inside of it.
    pub fn push_inner(&mut self, transform: FrameBinding<RenderTransform>, f: impl FnOnce(&mut Self)) {
        if let Some(mut data) = self.widget_data.take() {
            let parent_spatial_id = self.spatial_id;

            self.spatial_id = self.display_list.push_reference_frame(
                PxPoint::zero().to_wr(),
                self.spatial_id,
                TransformStyle::Flat,
                transform,
                ReferenceFrameKind::Transform {
                    is_2d_scale_translation: false, // TODO track this
                    should_snap: false,
                },
                SpatialFrameId::widget_id_to_wr(self.widget_id, self.pipeline_id),
            );

            let has_stacking_ctx = !data.filter.is_empty() || !data.flags.is_empty();
            if has_stacking_ctx {
                // we want to apply filters in the top-to-bottom, left-to-right order they appear in
                // the widget declaration, but the widget declaration expands to have the top property
                // node be inside the bottom property node, so the bottom property ends up inserting
                // a filter first, because we cannot insert filters after the child node render is called
                // so we need to reverse the filters here. Left-to-right sequences are reversed on insert
                // so they get reversed again here and everything ends up in order.
                data.filter.reverse();

                self.display_list.push_simple_stacking_context_with_filters(
                    PxPoint::zero().to_wr(),
                    self.spatial_id,
                    data.flags,
                    &data.filter,
                    &[],
                    &[],
                );
            }

            f(self);

            if has_stacking_ctx {
                self.display_list.pop_stacking_context();
            }
            self.display_list.pop_reference_frame();

            self.spatial_id = parent_spatial_id;
        } else {
            tracing::error!("called `push_inner` more then once for `{}`", self.widget_id);
            f(self)
        }
    }

    /// Returns `true` if the widget reference frame and stacking context is pushed and now is time for rendering the widget.
    ///
    /// This is `true` when inside a [`push_inner`] call but `false` when inside a [`push_widget`] call.
    ///
    /// [`push_widget`]: Self::push_widget
    /// [`push_inner`]: Self::push_inner
    pub fn is_inner(&self) -> bool {
        self.widget_data.is_none()
    }

    /// Push a hit-test `rect` using [`common_item_ps`] if hit-testing is enable.
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    #[inline]
    pub fn push_hit_test(&mut self, rect: PxRect) {
        expect_inner!(self.push_hit_test);

        if self.hit_testable {
            let common_item_ps = self.common_item_ps(rect);
            self.display_list.push_hit_test(&common_item_ps, self.item_tag());
        }
    }

    /// Calls `f` with a new [`clip_id`] that clips to `bounds`.
    ///
    /// [`clip_id`]: FrameBuilder::clip_id
    #[inline]
    pub fn push_simple_clip(&mut self, bounds: PxSize, f: impl FnOnce(&mut FrameBuilder)) {
        expect_inner!(self.push_hit_test);

        let parent_clip_id = self.clip_id;

        self.clip_id = self.display_list.define_clip_rect(
            &SpaceAndClipInfo {
                spatial_id: self.spatial_id,
                clip_id: self.clip_id,
            },
            PxRect::from_size(bounds).to_wr(),
        );

        f(self);

        self.clip_id = parent_clip_id;
    }

    /// Calls `f` inside a scroll viewport space.
    pub fn push_scroll_frame(
        &mut self,
        scroll_id: ScrollId,
        viewport_size: PxSize,
        content_rect: PxRect,
        f: impl FnOnce(&mut FrameBuilder),
    ) {
        expect_inner!(self.push_hit_test);

        let parent_spatial_id = self.spatial_id;

        self.spatial_id = self.display_list.define_scroll_frame(
            parent_spatial_id,
            scroll_id.to_wr(self.pipeline_id),
            content_rect.to_wr(),
            PxRect::from_size(viewport_size).to_wr(),
            content_rect.origin.to_vector().to_wr(),
            SpatialFrameId::scroll_id_to_wr(scroll_id, self.pipeline_id),
        );

        f(self);

        self.spatial_id = parent_spatial_id;
    }

    /// Calls `f` inside a new reference frame transformed by `transform`.
    ///
    /// Note that this introduces a new reference frame, you can use the [`push_inner`] method to
    /// add to the widget reference frame, properties that use this method should be careful to update the
    /// [`WidgetLayout`] during arrange to match.
    ///
    /// [`push_inner`]: Self::push_inner
    /// [`WidgetLayout`]: crate::widget_info::WidgetLayout
    #[inline]
    pub fn push_reference_frame(
        &mut self,
        id: SpatialFrameId,
        transform: FrameBinding<RenderTransform>,
        is_2d_scale_translation: bool,
        f: impl FnOnce(&mut Self),
    ) {
        let parent_spatial_id = self.spatial_id;
        self.spatial_id = self.display_list.push_reference_frame(
            PxPoint::zero().to_wr(),
            parent_spatial_id,
            TransformStyle::Flat,
            transform,
            ReferenceFrameKind::Transform {
                is_2d_scale_translation,
                should_snap: false,
            },
            id.to_wr(self.pipeline_id),
        );

        f(self);

        self.display_list.pop_reference_frame();
        self.spatial_id = parent_spatial_id;
    }

    /// Calls `f` with added `filter` stacking context.
    ///
    /// Note that this introduces a new stacking context, you can use the [`push_widget_filter`] method to
    /// add to the widget stacking context.
    ///
    /// [`push_widget_filter`]: Self::push_widget_filter
    pub fn push_filter(&mut self, filter: &RenderFilter, f: impl FnOnce(&mut Self)) {
        expect_inner!(self.push_filter);

        self.display_list.push_simple_stacking_context_with_filters(
            PxPoint::zero().to_wr(),
            self.spatial_id,
            PrimitiveFlags::empty(),
            filter,
            &[],
            &[],
        );

        f(self);

        self.display_list.pop_stacking_context();
    }

    /// Push a border using [`common_hit_item_ps`].
    ///
    /// [`common_hit_item_ps`]: FrameBuilder::common_hit_item_ps
    #[inline]
    pub fn push_border(&mut self, bounds: PxRect, widths: PxSideOffsets, sides: BorderSides, radius: PxCornerRadius) {
        expect_inner!(self.push_border);

        let details = BorderDetails::Normal(NormalBorder {
            left: sides.left.into(),
            right: sides.right.into(),
            top: sides.top.into(),
            bottom: sides.bottom.into(),
            radius: radius.to_border_radius(),
            do_aa: true,
        });

        let info = self.common_hit_item_ps(bounds);

        self.display_list.push_border(&info, bounds.to_wr(), widths.to_wr(), details);
    }

    /// Push a text run using [`common_hit_item_ps`].
    ///
    /// [`common_hit_item_ps`]: FrameBuilder::common_hit_item_ps
    #[inline]
    pub fn push_text(&mut self, rect: PxRect, glyphs: &[GlyphInstance], font: &impl Font, color: ColorF, synthesis: FontSynthesis) {
        expect_inner!(self.push_text);

        if let Some(r) = &self.renderer {
            let instance_key = font.instance_key(r, synthesis);

            let item = self.common_hit_item_ps(rect);
            self.display_list.push_text(&item, rect.to_wr(), glyphs, instance_key, color, None);
        } else {
            self.widget_rendered();
        }
    }

    /// Push an image using [`common_hit_item_ps`].
    ///
    /// [`common_hit_item_ps`]: FrameBuilder::common_hit_item_ps
    pub fn push_image(&mut self, clip_rect: PxRect, img_size: PxSize, image: &impl Image, rendering: ImageRendering) {
        expect_inner!(self.push_image);

        if let Some(r) = &self.renderer {
            let image_key = image.image_key(r);
            let item = self.common_hit_item_ps(clip_rect);
            self.display_list.push_image(
                &item,
                PxRect::from_size(img_size).to_wr(),
                rendering.into(),
                image.alpha_type(),
                image_key,
                RenderColor::WHITE,
            )
        } else {
            self.widget_rendered();
        }
    }

    /// Push a color rectangle using [`common_hit_item_ps`].
    ///
    /// [`common_hit_item_ps`]: FrameBuilder::common_hit_item_ps
    #[inline]
    pub fn push_color(&mut self, rect: PxRect, color: FrameBinding<RenderColor>) {
        expect_inner!(self.push_color);

        let item = self.common_hit_item_ps(rect);
        self.display_list.push_rect_with_animation(&item, rect.to_wr(), color);
    }

    /// Push a repeating linear gradient rectangle using [`common_hit_item_ps`].
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    ///
    /// [`common_hit_item_ps`]: FrameBuilder::common_hit_item_ps
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn push_linear_gradient(
        &mut self,
        rect: PxRect,
        line: PxLine,
        stops: &[RenderGradientStop],
        extend_mode: RenderExtendMode,
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        debug_assert!(stops.len() >= 2);
        debug_assert!(stops[0].offset.abs() < 0.00001, "first color stop must be at offset 0.0");
        debug_assert!(
            (stops[stops.len() - 1].offset - 1.0).abs() < 0.00001,
            "last color stop must be at offset 1.0"
        );

        expect_inner!(self.push_linear_gradient);

        let item = self.common_hit_item_ps(rect);

        self.display_list.push_stops(stops);

        let gradient = Gradient {
            start_point: line.start.to_wr(),
            end_point: line.end.to_wr(),
            extend_mode,
        };
        self.display_list
            .push_gradient(&item, rect.to_wr(), gradient, tile_size.to_wr(), tile_spacing.to_wr());
    }

    /// Push a repeating radial gradient rectangle using [`common_hit_item_ps`].
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The  `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The `center` point is relative to the top-left of the tile, the `radius` is the distance between the first
    /// and last color stop in both directions and must be a non-zero positive value.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    ///
    /// [`common_hit_item_ps`]: FrameBuilder::common_hit_item_ps
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn push_radial_gradient(
        &mut self,
        rect: PxRect,
        center: PxPoint,
        radius: PxSize,
        stops: &[RenderGradientStop],
        extend_mode: RenderExtendMode,
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        debug_assert!(stops.len() >= 2);
        debug_assert!(stops[0].offset.abs() < 0.00001, "first color stop must be at offset 0.0");
        debug_assert!(
            (stops[stops.len() - 1].offset - 1.0).abs() < 0.00001,
            "last color stop must be at offset 1.0"
        );

        expect_inner!(self.push_radial_gradient);

        let item = self.common_hit_item_ps(rect);

        self.display_list.push_stops(stops);

        let gradient = RadialGradient {
            center: center.to_wr(),
            radius: radius.to_wr(),
            start_offset: 0.0, // TODO expose this?
            end_offset: 1.0,
            extend_mode,
        };
        self.display_list
            .push_radial_gradient(&item, rect.to_wr(), gradient, tile_size.to_wr(), tile_spacing.to_wr())
    }

    /// Push a repeating conic gradient rectangle using [`common_hit_item_ps`].
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The  `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    ///
    /// [`common_hit_item_ps`]: FrameBuilder::common_hit_item_ps
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn push_conic_gradient(
        &mut self,
        rect: PxRect,
        center: PxPoint,
        angle: AngleRadian,
        stops: &[RenderGradientStop],
        extend_mode: RenderExtendMode,
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        debug_assert!(stops.len() >= 2);
        debug_assert!(stops[0].offset.abs() < 0.00001, "first color stop must be at offset 0.0");
        debug_assert!(
            (stops[stops.len() - 1].offset - 1.0).abs() < 0.00001,
            "last color stop must be at offset 1.0"
        );

        expect_inner!(self.push_conic_gradient);

        let item = self.common_hit_item_ps(rect);

        self.display_list.push_stops(stops);

        GradientBuilder::new();

        let gradient = ConicGradient {
            center: center.to_wr(),
            angle: angle.0,
            start_offset: 0.0, // TODO expose this?
            end_offset: 1.0,
            extend_mode,
        };
        self.display_list
            .push_conic_gradient(&item, rect.to_wr(), gradient, tile_size.to_wr(), tile_spacing.to_wr())
    }

    /// Push a styled vertical or horizontal line.
    #[inline]
    pub fn push_line(
        &mut self,
        bounds: PxRect,
        orientation: crate::border::LineOrientation,
        color: RenderColor,
        style: crate::border::LineStyle,
    ) {
        expect_inner!(self.push_line);

        let item = self.common_hit_item_ps(bounds);

        match style.render_command() {
            RenderLineCommand::Line(style, thickness) => {
                self.display_list
                    .push_line(&item, &bounds.to_wr(), thickness, orientation.into(), &color, style)
            }
            RenderLineCommand::Border(style) => {
                use crate::border::LineOrientation as LO;
                let widths = match orientation {
                    LO::Vertical => PxSideOffsets::new(Px(0), Px(0), Px(0), bounds.width()),
                    LO::Horizontal => PxSideOffsets::new(bounds.height(), Px(0), Px(0), Px(0)),
                };
                let details = BorderDetails::Normal(NormalBorder {
                    left: BorderSide { color, style },
                    right: BorderSide {
                        color: RenderColor::TRANSPARENT,
                        style: BorderStyle::Hidden,
                    },
                    top: BorderSide { color, style },
                    bottom: BorderSide {
                        color: RenderColor::TRANSPARENT,
                        style: BorderStyle::Hidden,
                    },
                    radius: BorderRadius::uniform(0.0),
                    do_aa: false,
                });

                self.display_list.push_border(&item, bounds.to_wr(), widths.to_wr(), details);
            }
        }
    }

    /// Push a `color` dot to mark the `offset` using [`common_item_ps`].
    ///
    /// The *dot* is a circle of the `color` highlighted by an white outline and shadow.
    ///
    /// [`common_item_ps`]: Self::common_item_ps
    #[inline]
    pub fn push_debug_dot(&mut self, offset: PxPoint, color: impl Into<RenderColor>) {
        let scale = self.scale_factor();

        let radius = PxSize::splat(Px(6)) * scale;
        let color = color.into();

        let mut builder = GradientBuilder::new();
        builder.push(RenderGradientStop { offset: 0.0, color });
        builder.push(RenderGradientStop { offset: 0.5, color });
        builder.push(RenderGradientStop {
            offset: 0.6,
            color: RenderColor::WHITE,
        });
        builder.push(RenderGradientStop {
            offset: 0.7,
            color: RenderColor::WHITE,
        });
        builder.push(RenderGradientStop {
            offset: 0.8,
            color: RenderColor::BLACK,
        });
        builder.push(RenderGradientStop {
            offset: 1.0,
            color: RenderColor::TRANSPARENT,
        });

        let center = radius.to_vector().to_point();
        let gradient = builder.radial_gradient(center.to_wr(), radius.to_wr(), RenderExtendMode::Clamp);
        let stops = builder.into_stops();

        let bounds = radius * 2.0.fct();

        let offset = offset - radius.to_vector();

        let common_item_ps = self.common_item_ps(PxRect::new(offset, bounds));
        self.display_list.push_stops(&stops);
        self.display_list.push_radial_gradient(
            &common_item_ps,
            PxRect::new(offset, bounds).to_wr(),
            gradient,
            bounds.to_wr(),
            PxSize::zero().to_wr(),
        )
    }

    /// Finalizes the build.
    pub fn finalize(mut self, root_rendered: &WidgetRendered) -> (BuiltFrame, UsedFrameBuilder) {
        root_rendered.set(self.widget_rendered);

        let (pipeline_id, display_list) = self.display_list.end();
        let (payload, descriptor) = display_list.into_data();
        let clear_color = self.clear_color.unwrap_or(RenderColor::WHITE);

        let reuse = UsedFrameBuilder {
            display_list: self.display_list,
        };

        let frame = BuiltFrame {
            id: self.frame_id,
            pipeline_id,
            display_list: (payload, descriptor),
            clear_color,
        };

        (frame, reuse)
    }
}

/// Output of a [`FrameBuilder`].
pub struct BuiltFrame {
    /// Frame id.
    pub id: FrameId,
    /// Pipeline.
    pub pipeline_id: PipelineId,
    /// Built display list.
    pub display_list: (DisplayListPayload, BuiltDisplayListDescriptor),
    /// Clear color selected for the frame.
    pub clear_color: RenderColor,
}

/// Data from a previous [`FrameBuilder`], can be reuse in the next frame for a performance boost.
pub struct UsedFrameBuilder {
    display_list: DisplayListBuilder,
}
impl UsedFrameBuilder {
    /// Pipeline where this frame builder can be reused.
    pub fn pipeline_id(&self) -> PipelineId {
        self.display_list.pipeline_id
    }
}
enum RenderLineCommand {
    Line(LineStyle, f32),
    Border(BorderStyle),
}
impl crate::border::LineStyle {
    fn render_command(self) -> RenderLineCommand {
        use crate::border::LineStyle as LS;
        use RenderLineCommand::*;
        match self {
            LS::Solid => Line(LineStyle::Solid, 0.0),
            LS::Double => Border(BorderStyle::Double),
            LS::Dotted => Line(LineStyle::Dotted, 0.0),
            LS::Dashed => Line(LineStyle::Dashed, 0.0),
            LS::Groove => Border(BorderStyle::Groove),
            LS::Ridge => Border(BorderStyle::Ridge),
            LS::Wavy(thickness) => Line(LineStyle::Wavy, thickness),
            LS::Hidden => Border(BorderStyle::Hidden),
        }
    }
}

/// A frame quick update.
///
/// A frame update causes a frame render without needing to fully rebuild the display list. It
/// is a more performant but also more limited way of generating a frame.
///
/// Any [`FrameBindingKey`] used in the creation of the frame can be used for updating the frame.
pub struct FrameUpdate {
    bindings: DynamicProperties,
    current_clear_color: RenderColor,
    clear_color: Option<RenderColor>,
    scrolls: Vec<(ExternalScrollId, PxVector)>,
    frame_id: FrameId,
}
impl FrameUpdate {
    /// New frame update builder.
    ///
    /// * `frame_id` - Id of the frame that will be updated.
    /// * `clear_color` - The current clear color.
    /// * `used_data` - Data generated by a previous frame update, if set is recycled for a performance boost.
    pub fn new(frame_id: FrameId, clear_color: RenderColor, used_data: Option<UsedFrameUpdate>) -> Self {
        let hint = used_data.unwrap_or(UsedFrameUpdate {
            scrolls_capacity: 10,
            transforms_capacity: 100,
            floats_capacity: 100,
            colors_capacity: 100,
        });
        FrameUpdate {
            bindings: DynamicProperties {
                transforms: Vec::with_capacity(hint.transforms_capacity),
                floats: Vec::with_capacity(hint.floats_capacity),
                colors: Vec::with_capacity(hint.colors_capacity),
            },
            scrolls: Vec::with_capacity(hint.scrolls_capacity),
            clear_color: None,
            frame_id,
            current_clear_color: clear_color,
        }
    }

    /// The frame that will be updated.
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Change the color used to clear the pixel buffer when redrawing the frame.
    #[inline]
    pub fn set_clear_color(&mut self, color: RenderColor) {
        self.clear_color = Some(color);
    }

    /// Update a layout transform value.
    #[inline]
    pub fn update_transform(&mut self, new_value: FrameValue<RenderTransform>) {
        self.bindings.transforms.push(new_value);
    }

    /// Update a float value.
    #[inline]
    pub fn update_f32(&mut self, new_value: FrameValue<f32>) {
        self.bindings.floats.push(new_value);
    }

    /// Update a color value.
    #[inline]
    pub fn update_color(&mut self, new_value: FrameValue<RenderColor>) {
        self.bindings.colors.push(new_value)
    }

    /// Update a scroll frame offset.
    ///
    /// The `offset` is added to the offset used in the last full frame render.
    #[inline]
    pub fn update_scroll(&mut self, id: ExternalScrollId, offset: PxVector) {
        self.scrolls.push((id, offset))
    }

    /// Finalize the update.
    ///
    /// Returns the property updates, scroll updates and the new clear color if any was set.
    pub fn finalize(mut self) -> (BuiltFrameUpdate, UsedFrameUpdate) {
        if self.clear_color == Some(self.current_clear_color) {
            self.clear_color = None;
        }

        let used = UsedFrameUpdate {
            scrolls_capacity: self.scrolls.len(),
            transforms_capacity: self.bindings.transforms.len(),
            floats_capacity: self.bindings.floats.len(),
            colors_capacity: self.bindings.colors.len(),
        };

        let update = BuiltFrameUpdate {
            bindings: self.bindings,
            scrolls: self.scrolls,
            clear_color: self.clear_color,
        };

        (update, used)
    }
}

/// Output of a [`FrameBuilder`].
pub struct BuiltFrameUpdate {
    /// Webrender frame properties updates.
    pub bindings: DynamicProperties,
    /// Scroll updates.
    pub scrolls: Vec<(ExternalScrollId, PxVector)>,
    /// New clear color.
    pub clear_color: Option<RenderColor>,
}

/// Data from a previous [`FrameUpdate`], can be reuse in the next frame for a performance boost.
#[derive(Clone, Copy)]
pub struct UsedFrameUpdate {
    scrolls_capacity: usize,
    transforms_capacity: usize,
    floats_capacity: usize,
    colors_capacity: usize,
}

/// A frame value that can be updated without regenerating the full frame.
///
/// Use `FrameBinding::Value(value)` to not use the quick update feature.
///
/// Create a [`FrameBindingKey`] and use its [`bind`] method to setup a frame binding.
///
/// [`bind`]: FrameBindingKey::bind
pub type FrameBinding<T> = PropertyBinding<T>; // we rename this to not conflict with the zero_ui property terminology.

/// A frame value update.
pub type FrameValue<T> = PropertyValue<T>;

unique_id_64! {
    #[derive(Debug)]
    struct FrameBindingKeyId;
}

unique_id_64! {
    /// Unique ID of a scroll viewport.
    #[derive(Debug)]
    pub struct ScrollId;
}
unique_id_32! {
    /// Unique ID of a reference frame.
    #[derive(Debug)]
    pub struct SpatialFrameId;
}

impl ScrollId {
    /// Id of the implicit scroll ID at the root of all frames.
    ///
    /// This [`ExternalScrollId`] cannot be represented by [`ScrollId`] because
    /// it is the zero value.
    #[inline]
    pub fn wr_root(pipeline_id: PipelineId) -> ExternalScrollId {
        ExternalScrollId(0, pipeline_id)
    }

    /// To webrender [`ExternalScrollId`].
    #[inline]
    pub fn to_wr(self, pipeline_id: PipelineId) -> ExternalScrollId {
        ExternalScrollId(self.get(), pipeline_id)
    }
}

impl SpatialFrameId {
    const WIDGET_ID_FLAG: u64 = 1 << 63;
    const SCROLL_ID_FLAG: u64 = 1 << 62;
    const LIST_ID_FLAG: u64 = 1 << 61;

    /// Make a [`SpatialTreeItemKey`] from a [`WidgetId`], there is no collision
    /// with other keys generated.
    #[inline]
    pub fn widget_id_to_wr(self_: WidgetId, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        SpatialTreeItemKey::new(pipeline_id.0 as u64 | Self::WIDGET_ID_FLAG, self_.get())
    }

    /// Make a [`SpatialTreeItemKey`] from a [`ScrollId`], there is no collision
    /// with other keys generated.
    #[inline]
    pub fn scroll_id_to_wr(self_: ScrollId, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        SpatialTreeItemKey::new(pipeline_id.0 as u64 | Self::SCROLL_ID_FLAG, self_.get())
    }

    /// To webrender [`SpatialTreeItemKey`].
    #[inline]
    pub fn to_wr(self, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        SpatialTreeItemKey::new(pipeline_id.0 as u64, self.get() as u64)
    }

    /// Make [`SpatialTreeItemKey`] from a a spatial parent + item index, there is no collision
    /// with other keys generated.
    #[inline]
    pub fn item_to_wr(self, index: usize, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        let item = (index as u64) << 32;
        SpatialTreeItemKey::new(pipeline_id.0 as u64 | Self::LIST_ID_FLAG, self.get() as u64 | item)
    }
}

/// Unique key of a [`FrameBinding`] value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBindingKey<T> {
    id: FrameBindingKeyId,
    _type: PhantomData<T>,
}
impl<T> FrameBindingKey<T> {
    /// Generates a new unique ID.
    ///
    /// # Panics
    /// Panics if called more then `u64::MAX` times.
    #[inline]
    pub fn new_unique() -> Self {
        FrameBindingKey {
            id: FrameBindingKeyId::new_unique(),
            _type: PhantomData,
        }
    }

    fn property_key(&self) -> PropertyBindingKey<T> {
        PropertyBindingKey::new(self.id.get())
    }

    /// Create a binding with this key.
    #[inline]
    pub fn bind(self, value: T) -> FrameBinding<T> {
        FrameBinding::Binding(self.property_key(), value)
    }

    /// Create a value update with this key.
    #[inline]
    pub fn update(self, value: T) -> FrameValue<T> {
        FrameValue {
            key: self.property_key(),
            value,
        }
    }
}

/// A hit-test hit.
#[derive(Clone, Debug)]
pub struct HitInfo {
    /// ID of widget hit.
    pub widget_id: WidgetId,
}

/// A hit-test result.
#[derive(Clone, Debug)]
pub struct FrameHitInfo {
    window_id: WindowId,
    frame_id: FrameId,
    point: PxPoint,
    hits: Vec<HitInfo>,
}
impl FrameHitInfo {
    /// Initializes from a Webrender hit-test result.
    ///
    /// Only item tags produced by [`FrameBuilder`] are expected.
    ///
    /// The tag format is:
    ///
    /// * `u64`: Raw [`WidgetId`].
    /// * `u16`: Zero, reserved.
    #[inline]
    pub fn new(window_id: WindowId, frame_id: FrameId, point: PxPoint, hits: &HitTestResult) -> Self {
        let hits = hits
            .items
            .iter()
            .filter_map(|h| {
                if h.tag.0 == 0 || h.tag.1 != 0 {
                    None
                } else {
                    // SAFETY: we skip zero so the value is memory safe.
                    let widget_id = unsafe { WidgetId::from_raw(h.tag.0) };
                    Some(HitInfo { widget_id })
                }
            })
            .collect();

        FrameHitInfo {
            window_id,
            frame_id,
            point,
            hits,
        }
    }

    /// No hits info
    #[inline]
    pub fn no_hits(window_id: WindowId) -> Self {
        FrameHitInfo::new(window_id, FrameId::INVALID, PxPoint::new(Px(-1), Px(-1)), &HitTestResult::default())
    }

    /// The window that was hit-tested.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// The window frame that was hit-tested.
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// The point in the window that was hit-tested.
    #[inline]
    pub fn point(&self) -> PxPoint {
        self.point
    }

    /// All hits, from top-most.
    #[inline]
    pub fn hits(&self) -> &[HitInfo] {
        &self.hits
    }

    /// The top hit.
    #[inline]
    pub fn target(&self) -> Option<&HitInfo> {
        self.hits.first()
    }

    /// Finds the widget in the hit-test result if it was hit.
    #[inline]
    pub fn find(&self, widget_id: WidgetId) -> Option<&HitInfo> {
        self.hits.iter().find(|h| h.widget_id == widget_id)
    }

    /// If the widget is in was hit.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.hits.iter().any(|h| h.widget_id == widget_id)
    }

    /// Gets a clone of `self` that only contains the hits that also happen in `other`.
    #[inline]
    pub fn intersection(&self, other: &FrameHitInfo) -> FrameHitInfo {
        let mut hits: Vec<_> = self.hits.iter().filter(|h| other.contains(h.widget_id)).cloned().collect();
        hits.shrink_to_fit();

        FrameHitInfo {
            window_id: self.window_id,
            frame_id: self.frame_id,
            point: self.point,
            hits,
        }
    }
}

bitflags! {
    /// Configure if a synthetic font is generated for fonts that do not implement **bold** or *oblique* variants.
    pub struct FontSynthesis: u8 {
        /// No synthetic font generated, if font resolution does not find a variant the matches the requested propertied
        /// the properties are ignored and the normal font is returned.
        const DISABLED = 0;
        /// Enable synthetic bold. Font resolution finds the closest bold variant, the difference added using extra stroke.
        const BOLD = 1;
        /// Enable synthetic oblique. If the font resolution does not find an oblique or italic variant a skew transform is applied.
        const STYLE = 2;
        /// Enabled all synthetic font possibilities.
        const ENABLED = Self::BOLD.bits | Self::STYLE.bits;
    }
}
impl Default for FontSynthesis {
    /// [`FontSynthesis::ENABLED`]
    #[inline]
    fn default() -> Self {
        FontSynthesis::ENABLED
    }
}
impl_from_and_into_var! {
    /// Convert to full [`ENABLED`](FontSynthesis::ENABLED) or [`DISABLED`](FontSynthesis::DISABLED).
    fn from(enabled: bool) -> FontSynthesis {
        if enabled { FontSynthesis::ENABLED } else { FontSynthesis::DISABLED }
    }
}

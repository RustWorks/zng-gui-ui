use std::{cell::Cell, mem};

use linear_map::LinearMap;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use webrender_api::{self as wr, PipelineId};

use crate::{units::*, FrameId};

/// Represents a builder for display items that will be rendered in the view process.
#[derive(Debug)]
pub struct DisplayListBuilder {
    pipeline_id: PipelineId,
    frame_id: FrameId,
    list: Vec<DisplayItem>,

    clip_len: usize,
    space_len: usize,
    stack_ctx_len: usize,
}
impl DisplayListBuilder {
    /// New default.
    pub fn new(pipeline_id: PipelineId, frame_id: FrameId) -> Self {
        Self::with_capacity(pipeline_id, frame_id, 100)
    }

    /// New with pre-allocation.
    pub fn with_capacity(pipeline_id: PipelineId, frame_id: FrameId, capacity: usize) -> Self {
        Self {
            pipeline_id,
            frame_id,
            list: Vec::with_capacity(capacity),

            clip_len: 1,
            space_len: 1,
            stack_ctx_len: 1,
        }
    }

    /// Pipeline that will render the display items.
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Frame that will be rendered by this display list.
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Mark the start of a reuse range, the range can be completed with [`finish_reuse_range`].
    ///
    /// Reuse ranges can be nested.
    ///
    /// [`finish_reuse_range`]: Self::finish_reuse_range
    pub fn start_reuse_range(&mut self) -> ReuseStart {
        ReuseStart {
            pipeline_id: self.pipeline_id,
            frame_id: self.frame_id,
            start: self.list.len(),
            clip_len: self.clip_len,
            space_len: self.space_len,
            stack_ctx_len: self.stack_ctx_len,
        }
    }

    /// Mark the end of a reuse range.
    ///
    /// Panics if `start` was not generated by a call to [`start_reuse_range`] on the same builder, or clips, reference frames or
    /// stacking contexts where pushed inside it and not popped before the call to finish.
    ///
    /// [`start_reuse_range`]: Self::start_reuse_range
    pub fn finish_reuse_range(&mut self, start: ReuseStart) -> ReuseRange {
        assert_eq!(self.pipeline_id, start.pipeline_id, "reuse range not started by the same builder");
        assert_eq!(self.frame_id, start.frame_id, "reuse range not started by the same builder");
        assert_eq!(
            self.clip_len, start.clip_len,
            "reuse range cannot finish before all clips pushed inside it are popped"
        );
        assert_eq!(
            self.space_len, start.space_len,
            "reuse range cannot finish before all reference frames pushed inside it are popped"
        );
        assert_eq!(
            self.stack_ctx_len, start.stack_ctx_len,
            "reuse range cannot finish before all stacking contexts pushed inside it are popped"
        );
        debug_assert!(start.start <= self.list.len());

        ReuseRange {
            pipeline_id: self.pipeline_id,
            frame_id: self.frame_id,
            start: start.start,
            end: self.list.len(),
        }
    }

    /// Push a range of items to be copied from the previous display list on the same pipeline.
    ///
    /// Panics if `range` does not have a compatible pipeline id.
    pub fn push_reuse_range(&mut self, range: &ReuseRange) {
        assert_eq!(self.pipeline_id, range.pipeline_id);

        if !range.is_empty() {
            self.list.push(DisplayItem::Reuse {
                frame_id: range.frame_id,
                start: range.start,
                end: range.end,
            });
        }
    }

    /// Start a new spatial context, must be paired with a call to [`pop_reference_frame`].
    ///
    /// [`pop_reference_frame`]: Self::pop_reference_frame
    pub fn push_reference_frame(&mut self, key: wr::SpatialTreeItemKey, transform: FrameValue<PxTransform>, is_2d_scale_translation: bool) {
        self.space_len += 1;
        self.list.push(DisplayItem::PushReferenceFrame {
            key,
            transform,
            is_2d_scale_translation,
        });
    }

    /// Finish the spatial context started by a call to [`push_reference_frame`].
    ///
    /// [`push_reference_frame`]: Self::push_reference_frame
    pub fn pop_reference_frame(&mut self) {
        debug_assert!(self.space_len > 1);
        self.space_len -= 1;
        self.list.push(DisplayItem::PopReferenceFrame);
    }

    /// Start a new filters context, must be paired with a call to [`pop_stacking_context`].
    ///
    /// [`pop_stacking_context`]: Self::pop_stacking_context
    pub fn push_stacking_context(
        &mut self,
        blend_mode: wr::MixBlendMode,
        filters: &[FilterOp],
        filter_datas: &[wr::FilterData],
        filter_primitives: &[wr::FilterPrimitive],
    ) {
        self.stack_ctx_len += 1;
        self.list.push(DisplayItem::PushStackingContext {
            blend_mode,
            filters: filters.to_vec().into_boxed_slice(),
            filter_datas: filter_datas.to_vec().into_boxed_slice(),
            filter_primitives: filter_primitives.to_vec().into_boxed_slice(),
        })
    }

    /// Finish the filters context started by a call to [`push_stacking_context`].
    ///
    /// [`push_stacking_context`]: Self::push_stacking_context
    pub fn pop_stacking_context(&mut self) {
        debug_assert!(self.stack_ctx_len > 1);
        self.stack_ctx_len -= 1;
        self.list.push(DisplayItem::PopStackingContext);
    }

    /// Push a rectangular clip that will affect all pushed items until a paired call to [`pop_clip`].
    ///
    /// [`pop_clip`]: Self::pop_clip
    pub fn push_clip_rect(&mut self, clip_rect: PxRect, clip_out: bool) {
        self.clip_len += 1;
        self.list.push(DisplayItem::PushClipRect { clip_rect, clip_out });
    }

    /// Push a rectangular clip with rounded corners that will affect all pushed items until a paired call to [`pop_clip`].
    ///
    /// If `clip_out` is `true` only pixels outside the rounded rect are visible.
    ///
    /// [`pop_clip`]: Self::pop_clip
    pub fn push_clip_rounded_rect(&mut self, clip_rect: PxRect, corners: PxCornerRadius, clip_out: bool) {
        self.clip_len += 1;
        self.list.push(DisplayItem::PushClipRoundedRect {
            clip_rect,
            corners,
            clip_out,
        });
    }

    /// Pop a clip previously pushed by a call to [`push_clip_rect`]. Items pushed after this call are not
    /// clipped by the removed clip.
    ///
    /// [`push_clip_rect`]: Self::push_clip_rect
    pub fn pop_clip(&mut self) {
        debug_assert!(self.clip_len > 1);
        self.clip_len -= 1;
        self.list.push(DisplayItem::PopClip);
    }

    /// Push a normal border.
    #[allow(clippy::too_many_arguments)]
    pub fn push_border(
        &mut self,
        bounds: PxRect,
        widths: PxSideOffsets,
        top: wr::BorderSide,
        right: wr::BorderSide,
        bottom: wr::BorderSide,
        left: wr::BorderSide,
        radius: PxCornerRadius,
    ) {
        self.list.push(DisplayItem::Border {
            bounds,
            widths,
            sides: [top, right, bottom, left],
            radius,
        })
    }

    /// Push a text run.
    pub fn push_text(
        &mut self,
        clip_rect: PxRect,
        font_key: wr::FontInstanceKey,
        glyphs: &[wr::GlyphInstance],
        color: FrameValue<wr::ColorF>,
        options: wr::GlyphOptions,
    ) {
        self.list.push(DisplayItem::Text {
            clip_rect,
            font_key,
            glyphs: glyphs.to_vec().into_boxed_slice(),
            color,
            options,
        });
    }

    /// Push an image.
    pub fn push_image(
        &mut self,
        clip_rect: PxRect,
        image_key: wr::ImageKey,
        image_size: PxSize,
        rendering: wr::ImageRendering,
        alpha_type: wr::AlphaType,
    ) {
        self.list.push(DisplayItem::Image {
            clip_rect,
            image_key,
            image_size,
            rendering,
            alpha_type,
        })
    }

    /// Push a color rectangle.
    pub fn push_color(&mut self, clip_rect: PxRect, color: FrameValue<wr::ColorF>) {
        self.list.push(DisplayItem::Color { clip_rect, color })
    }

    /// Push a linear gradient rectangle.
    pub fn push_linear_gradient(
        &mut self,
        clip_rect: PxRect,
        gradient: wr::Gradient,
        stops: &[wr::GradientStop],
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        self.list.push(DisplayItem::LinearGradient {
            clip_rect,
            gradient,
            stops: stops.to_vec().into_boxed_slice(),
            tile_size,
            tile_spacing,
        })
    }

    /// Push a radial gradient rectangle.
    pub fn push_radial_gradient(
        &mut self,
        clip_rect: PxRect,
        gradient: wr::RadialGradient,
        stops: &[wr::GradientStop],
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        self.list.push(DisplayItem::RadialGradient {
            clip_rect,
            gradient,
            stops: stops.to_vec().into_boxed_slice(),
            tile_size,
            tile_spacing,
        });
    }

    /// Push a conic gradient rectangle.
    pub fn push_conic_gradient(
        &mut self,
        clip_rect: PxRect,
        gradient: wr::ConicGradient,
        stops: &[wr::GradientStop],
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        self.list.push(DisplayItem::ConicGradient {
            clip_rect,
            gradient,
            stops: stops.to_vec().into_boxed_slice(),
            tile_size,
            tile_spacing,
        });
    }

    /// Push a styled vertical or horizontal line.
    pub fn push_line(
        &mut self,
        clip_rect: PxRect,
        color: wr::ColorF,
        style: wr::LineStyle,
        wavy_line_thickness: f32,
        orientation: wr::LineOrientation,
    ) {
        self.list.push(DisplayItem::Line {
            clip_rect,
            color,
            style,
            wavy_line_thickness,
            orientation,
        })
    }

    /// Number of display items.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Returns `true` if the list has no display items.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Returns the display list an capacity suggestion for the next frame.
    pub fn finalize(self) -> (DisplayList, usize) {
        let cap = self.list.len();
        (
            DisplayList {
                pipeline_id: self.pipeline_id,
                frame_id: self.frame_id,
                list: self.list,
            },
            cap,
        )
    }
}

/// Represents the start of a display list reuse range.
///
/// See [`DisplayListBuilder::start_reuse_range`] for more details.
pub struct ReuseStart {
    pipeline_id: PipelineId,
    frame_id: FrameId,
    start: usize,

    clip_len: usize,
    space_len: usize,
    stack_ctx_len: usize,
}

/// Represents a display list reuse range.
///
/// See [`DisplayListBuilder::push_reuse_range`] for more details.
#[derive(Debug)]
pub struct ReuseRange {
    pipeline_id: PipelineId,
    frame_id: FrameId,
    start: usize,
    end: usize,
}
impl ReuseRange {
    /// Pipeline where the items can be reused.
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Frame that owns the reused items selected by this range.
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// If the reuse range did not capture any display item.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Represents a finalized display list.
///
/// See [`DisplayListBuilder::finalize`] for more details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayList {
    pipeline_id: PipelineId,
    frame_id: FrameId,
    list: Vec<DisplayItem>,
}
impl DisplayList {
    /// Pipeline that will render the display items.
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Frame that will be rendered by this display list.
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Convert the display list to a webrender display list, including the reuse items.
    pub fn to_webrender(self, cache: &mut DisplayListCache) -> wr::BuiltDisplayList {
        assert_eq!(self.pipeline_id, cache.pipeline_id);

        let r = Self::build(&self.list, cache);
        cache.insert(self);

        r
    }
    fn build(list: &[DisplayItem], cache: &mut DisplayListCache) -> wr::BuiltDisplayList {
        let _s = tracing::trace_span!("DisplayList::build").entered();

        let (mut wr_list, mut sc) = cache.begin_wr();

        for item in list {
            item.to_webrender(&mut wr_list, &mut sc, cache);
        }

        cache.end_wr(wr_list, sc)
    }
}

/// Frame value binding key.
///
/// See [`FrameValue`] for more details.
pub type FrameValueKey<T> = webrender_api::PropertyBindingKey<T>;

/// Represents a frame value that may be updated.
///
/// This value is send in a full frame request, after frame updates may be send targeting the key.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FrameValue<T> {
    /// Value that is updated with frame update requests.
    Bind {
        /// Key that will be used to update the value.
        key: FrameValueKey<T>,
        /// Initial value.
        value: T,
        /// If the value will update rapidly.
        ///
        /// This affects if the frame binding will be propagated to webrender,
        /// see [`FrameValue::into_wr`] for details.
        animating: bool,
    },
    /// Value is not updated, a new frame must be send to change this value.
    Value(T),
}
impl<T> FrameValue<T> {
    /// Reference the (initial) value.
    pub fn value(&self) -> &T {
        match self {
            FrameValue::Bind { value, .. } | FrameValue::Value(value) => value,
        }
    }

    /// Into the (initial) value.
    pub fn into_value(self) -> T {
        match self {
            FrameValue::Bind { value, .. } | FrameValue::Value(value) => value,
        }
    }

    /// Convert to webrender binding.
    ///
    /// Returns a webrender binding only if is animating, webrender behaves as if the value is animating
    /// if it is bound, skipping some caching, this can have a large performance impact in software mode, if
    /// a large area of the screen is painted with a bound color.
    pub fn into_wr<U>(self) -> wr::PropertyBinding<U>
    where
        U: From<T>,
    {
        match self {
            FrameValue::Bind {
                key,
                value,
                animating: true,
            } => wr::PropertyBinding::Binding(
                wr::PropertyBindingKey {
                    id: key.id,
                    _phantom: std::marker::PhantomData,
                },
                value.into(),
            ),
            FrameValue::Bind {
                value, animating: false, ..
            } => wr::PropertyBinding::Value(value.into()),
            FrameValue::Value(value) => wr::PropertyBinding::Value(value.into()),
        }
    }

    /// Returns `true` if a new frame must be generated.
    fn update_bindable(value: &mut T, animating: &mut bool, update: &FrameValueUpdate<T>) -> bool
    where
        T: PartialEq + Copy,
    {
        // if changed to `true`, needs a frame to register the binding.
        //
        // if changed to `false`, needs a frame to un-register the binding so that webrender can start caching
        // the tiles in the region again, we can't use the binding "one last time" because if a smaller region
        // continues animating it would keep refreshing the large region too.
        //
        // if continues to be `false` only needs to update if the value actually changed.
        let need_frame = (*animating != update.animating) || (!*animating && *value != update.value);

        *animating = update.animating;
        *value = update.value;

        need_frame
    }

    /// Returns `true` if a new frame must be generated.
    fn update_value(value: &mut T, update: &FrameValueUpdate<T>) -> bool
    where
        T: PartialEq + Copy,
    {
        if value != &update.value {
            *value = update.value;
            true
        } else {
            false
        }
    }
}

/// Represents an update targeting a previously setup [`FrameValue`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FrameValueUpdate<T> {
    /// Value key.
    pub key: FrameValueKey<T>,
    /// New value.
    pub value: T,
    /// If the value is updating rapidly.
    pub animating: bool,
}
impl<T> FrameValueUpdate<T> {
    /// Convert to webrender binding update.
    pub fn into_wr<U>(self) -> Option<wr::PropertyValue<U>>
    where
        U: From<T>,
    {
        if self.animating {
            Some(wr::PropertyValue {
                key: wr::PropertyBindingKey {
                    id: self.key.id,
                    _phantom: std::marker::PhantomData,
                },
                value: self.value.into(),
            })
        } else {
            None
        }
    }
}

struct CachedDisplayList {
    list: Vec<DisplayItem>,
    used: Cell<bool>,
}

/// View process side cache of [`DisplayList`] frames for a pipeline.
pub struct DisplayListCache {
    pipeline_id: PipelineId,
    lists: LinearMap<FrameId, CachedDisplayList>,
    space_and_clip: Option<SpaceAndClip>,

    latest_frame: FrameId,
    bindings: FxHashMap<wr::PropertyBindingId, (FrameId, usize)>,

    wr_list: Option<wr::DisplayListBuilder>,
}
impl DisplayListCache {
    /// New empty.
    pub fn new(pipeline_id: PipelineId) -> Self {
        DisplayListCache {
            pipeline_id,
            lists: LinearMap::new(),
            latest_frame: FrameId::INVALID,
            space_and_clip: Some(SpaceAndClip::new(pipeline_id)),
            bindings: FxHashMap::default(),
            wr_list: Some(wr::DisplayListBuilder::new(pipeline_id)),
        }
    }

    /// Pipeline where the items can be reused.
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    fn begin_wr(&mut self) -> (wr::DisplayListBuilder, SpaceAndClip) {
        let mut list = self.wr_list.take().unwrap();
        let sc = self.space_and_clip.take().unwrap();
        list.begin();
        (list, sc)
    }

    fn end_wr(&mut self, mut list: wr::DisplayListBuilder, sc: SpaceAndClip) -> wr::BuiltDisplayList {
        let r = list.end().1;
        self.wr_list = Some(list);
        self.space_and_clip = Some(sc);
        r
    }

    fn reuse(&self, frame_id: FrameId, start: usize, end: usize, wr_list: &mut wr::DisplayListBuilder, sc: &mut SpaceAndClip) {
        if let Some(l) = self.lists.get(&frame_id) {
            l.used.set(true);

            let range = l.list.get(start..end).unwrap_or_else(|| {
                tracing::error!("invalid reuse range ({start}..{end}), ignored");
                &[]
            });
            for item in range {
                item.to_webrender(wr_list, sc, self);
            }
        } else {
            tracing::error!("did not find reuse frame {frame_id:?}");
        }
    }

    fn insert(&mut self, list: DisplayList) {
        self.lists.retain(|_, l| l.used.take());

        for (i, item) in list.list.iter().enumerate() {
            item.register_bindings(&mut self.bindings, (list.frame_id, i));
        }

        self.latest_frame = list.frame_id;
        self.lists.insert(
            list.frame_id,
            CachedDisplayList {
                list: list.list,
                used: Cell::new(false),
            },
        );
    }

    fn get_update_target(&mut self, id: wr::PropertyBindingId) -> Option<&mut DisplayItem> {
        if let Some((frame_id, i)) = self.bindings.get(&id) {
            if let Some(list) = self.lists.get_mut(frame_id) {
                if let Some(item) = list.list.get_mut(*i) {
                    return Some(item);
                }
            }
        }
        None
    }

    /// Apply updates, returns the webrender update if the renderer can also be updated and there are any updates,
    /// or returns a new frame if a new frame must be rendered.
    pub fn update(
        &mut self,
        transforms: Vec<FrameValueUpdate<PxTransform>>,
        floats: Vec<FrameValueUpdate<f32>>,
        colors: Vec<FrameValueUpdate<wr::ColorF>>,
    ) -> Result<Option<wr::DynamicProperties>, wr::BuiltDisplayList> {
        let mut new_frame = false;

        for t in &transforms {
            if let Some(item) = self.get_update_target(t.key.id) {
                new_frame |= item.update_transform(t);
            }
        }
        for t in &floats {
            if let Some(item) = self.get_update_target(t.key.id) {
                new_frame |= item.update_float(t);
            }
        }
        for t in &colors {
            if let Some(item) = self.get_update_target(t.key.id) {
                new_frame |= item.update_color(t);
            }
        }

        if new_frame {
            let list = self.lists.get_mut(&self.latest_frame).expect("no frame to update");
            let list = mem::take(&mut list.list);
            let r = DisplayList::build(&list, self);
            self.lists.get_mut(&self.latest_frame).unwrap().list = list;

            Err(r)
        } else {
            let r = wr::DynamicProperties {
                transforms: transforms.into_iter().filter_map(FrameValueUpdate::into_wr).collect(),
                floats: floats.into_iter().filter_map(FrameValueUpdate::into_wr).collect(),
                colors: colors.into_iter().filter_map(FrameValueUpdate::into_wr).collect(),
            };
            if r.transforms.is_empty() && r.floats.is_empty() && r.colors.is_empty() {
                Ok(None)
            } else {
                Ok(Some(r))
            }
        }
    }
}

/// Represents one of the filters applied to a stacking context.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilterOp {
    /// Blur, width and height in pixels.
    Blur(f32, f32),
    /// Brightness, in [0..=1] range.
    Brightness(f32),
    /// Contrast, in [0..=1] range.
    Contrast(f32),
    /// Grayscale, in [0..=1] range.
    Grayscale(f32),
    /// Hue shift, in degrees.
    HueRotate(f32),
    /// Invert, in [0..=1] range.
    Invert(f32),
    /// Opacity, in [0..=1] range, can be bound.
    Opacity(FrameValue<f32>),
    /// Saturation, in [0..=1] range.
    Saturate(f32),
    /// Sepia, in [0..=1] range.
    Sepia(f32),
    /// Pixel perfect shadow.
    DropShadow(wr::Shadow),
    /// Custom filter.
    ///
    /// The color matrix is in the format of SVG color matrix, [0..5] is the first matrix row.
    ColorMatrix([f32; 20]),
    /// sRGB to linear RGB, as defined by SVG.
    SrgbToLinear,
    /// Linear RGB to sRGB, as defined by SVG.
    LinearToSrgb,

    /// SVG component transfer, the functions are defined in the stacking context `filter_data` parameter.
    ComponentTransfer,

    /// Fill with color.
    Flood(wr::ColorF),
}
impl FilterOp {
    /// To webrender filter-op.
    pub fn to_wr(self) -> wr::FilterOp {
        match self {
            FilterOp::Blur(w, h) => wr::FilterOp::Blur(w, h),
            FilterOp::Brightness(b) => wr::FilterOp::Brightness(b),
            FilterOp::Contrast(c) => wr::FilterOp::Contrast(c),
            FilterOp::Grayscale(g) => wr::FilterOp::Grayscale(g),
            FilterOp::HueRotate(h) => wr::FilterOp::HueRotate(h),
            FilterOp::Invert(i) => wr::FilterOp::Invert(i),
            FilterOp::Opacity(o) => wr::FilterOp::Opacity(o.into_wr(), *o.value()),
            FilterOp::Saturate(s) => wr::FilterOp::Saturate(s),
            FilterOp::Sepia(s) => wr::FilterOp::Sepia(s),
            FilterOp::DropShadow(d) => wr::FilterOp::DropShadow(d),
            FilterOp::ColorMatrix(m) => wr::FilterOp::ColorMatrix(m),
            FilterOp::SrgbToLinear => wr::FilterOp::SrgbToLinear,
            FilterOp::LinearToSrgb => wr::FilterOp::LinearToSrgb,
            FilterOp::ComponentTransfer => wr::FilterOp::ComponentTransfer,
            FilterOp::Flood(c) => wr::FilterOp::Flood(c),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DisplayItem {
    Reuse {
        frame_id: FrameId,
        start: usize,
        end: usize,
    },
    PushReferenceFrame {
        key: wr::SpatialTreeItemKey,
        transform: FrameValue<PxTransform>,
        is_2d_scale_translation: bool,
    },
    PopReferenceFrame,

    PushStackingContext {
        blend_mode: wr::MixBlendMode,
        filters: Box<[FilterOp]>,
        filter_datas: Box<[wr::FilterData]>,
        filter_primitives: Box<[wr::FilterPrimitive]>,
    },
    PopStackingContext,

    PushClipRect {
        clip_rect: PxRect,
        clip_out: bool,
    },
    PushClipRoundedRect {
        clip_rect: PxRect,
        corners: PxCornerRadius,
        clip_out: bool,
    },
    PopClip,

    Border {
        bounds: PxRect,
        widths: PxSideOffsets,
        sides: [wr::BorderSide; 4],
        radius: PxCornerRadius,
    },

    Text {
        clip_rect: PxRect,
        font_key: wr::FontInstanceKey,
        glyphs: Box<[wr::GlyphInstance]>,
        color: FrameValue<wr::ColorF>,
        options: wr::GlyphOptions,
    },

    Image {
        clip_rect: PxRect,
        image_key: wr::ImageKey,
        image_size: PxSize,
        rendering: wr::ImageRendering,
        alpha_type: wr::AlphaType,
    },

    Color {
        clip_rect: PxRect,
        color: FrameValue<wr::ColorF>,
    },

    LinearGradient {
        clip_rect: PxRect,
        gradient: wr::Gradient,
        stops: Box<[wr::GradientStop]>,
        tile_size: PxSize,
        tile_spacing: PxSize,
    },
    RadialGradient {
        clip_rect: PxRect,
        gradient: wr::RadialGradient,
        stops: Box<[wr::GradientStop]>,
        tile_size: PxSize,
        tile_spacing: PxSize,
    },
    ConicGradient {
        clip_rect: PxRect,
        gradient: wr::ConicGradient,
        stops: Box<[wr::GradientStop]>,
        tile_size: PxSize,
        tile_spacing: PxSize,
    },

    Line {
        clip_rect: PxRect,
        color: wr::ColorF,
        style: wr::LineStyle,
        wavy_line_thickness: f32,
        orientation: wr::LineOrientation,
    },
}
impl DisplayItem {
    fn to_webrender(&self, wr_list: &mut wr::DisplayListBuilder, sc: &mut SpaceAndClip, cache: &DisplayListCache) {
        match self {
            DisplayItem::Reuse { frame_id, start, end } => cache.reuse(*frame_id, *start, *end, wr_list, sc),

            DisplayItem::PushReferenceFrame {
                key,
                transform,
                is_2d_scale_translation,
            } => {
                let spatial_id = wr_list.push_reference_frame(
                    wr::units::LayoutPoint::zero(),
                    sc.spatial_id(),
                    wr::TransformStyle::Flat,
                    transform.into_wr(),
                    wr::ReferenceFrameKind::Transform {
                        is_2d_scale_translation: *is_2d_scale_translation,
                        should_snap: false,
                        paired_with_perspective: false,
                    },
                    *key,
                );
                sc.spatial.push(spatial_id);
            }
            DisplayItem::PopReferenceFrame => {
                wr_list.pop_reference_frame();
                sc.pop_spatial();
            }

            DisplayItem::PushStackingContext {
                blend_mode,
                filters,
                filter_datas,
                filter_primitives,
            } => {
                let clip = wr_list.define_clip_chain(None, [sc.clip_id()]);
                wr_list.push_stacking_context(
                    wr::units::LayoutPoint::zero(),
                    sc.spatial_id(),
                    wr::PrimitiveFlags::empty(),
                    Some(clip),
                    wr::TransformStyle::Flat,
                    *blend_mode,
                    &filters.iter().map(|f| f.to_wr()).collect::<Vec<_>>(),
                    filter_datas,
                    filter_primitives,
                    wr::RasterSpace::Screen,
                    wr::StackingContextFlags::empty(),
                )
            }
            DisplayItem::PopStackingContext => wr_list.pop_stacking_context(),

            DisplayItem::PushClipRect { clip_rect, clip_out } => {
                let clip_id = if *clip_out {
                    wr_list.define_clip_rounded_rect(
                        &sc.info(),
                        wr::ComplexClipRegion::new(clip_rect.to_wr(), PxCornerRadius::zero().to_wr(), wr::ClipMode::ClipOut),
                    )
                } else {
                    wr_list.define_clip_rect(&sc.info(), clip_rect.to_wr())
                };

                sc.clip.push(clip_id);
            }
            DisplayItem::PushClipRoundedRect {
                clip_rect,
                corners,
                clip_out,
            } => {
                let clip_id = wr_list.define_clip_rounded_rect(
                    &sc.info(),
                    wr::ComplexClipRegion::new(
                        clip_rect.to_wr(),
                        corners.to_wr(),
                        if *clip_out { wr::ClipMode::ClipOut } else { wr::ClipMode::Clip },
                    ),
                );
                sc.clip.push(clip_id);
            }
            DisplayItem::PopClip => sc.pop_clip(),

            DisplayItem::Text {
                clip_rect,
                font_key,
                glyphs,
                color,
                options,
            } => {
                let bounds = clip_rect.to_wr();
                wr_list.push_text(
                    &wr::CommonItemProperties {
                        clip_rect: bounds,
                        clip_id: sc.clip_id(),
                        spatial_id: sc.spatial_id(),
                        flags: wr::PrimitiveFlags::empty(),
                    },
                    bounds,
                    glyphs,
                    *font_key,
                    color.into_value(),
                    Some(*options),
                );
            }

            DisplayItem::Color { clip_rect, color } => {
                let bounds = clip_rect.to_wr();
                wr_list.push_rect_with_animation(
                    &wr::CommonItemProperties {
                        clip_rect: bounds,
                        clip_id: sc.clip_id(),
                        spatial_id: sc.spatial_id(),
                        flags: wr::PrimitiveFlags::empty(),
                    },
                    bounds,
                    color.into_wr(),
                )
            }

            DisplayItem::Border {
                bounds,
                widths,
                sides: [top, right, bottom, left],
                radius,
            } => {
                let bounds = bounds.to_wr();
                wr_list.push_border(
                    &wr::CommonItemProperties {
                        clip_rect: bounds,
                        clip_id: sc.clip_id(),
                        spatial_id: sc.spatial_id(),
                        flags: wr::PrimitiveFlags::empty(),
                    },
                    bounds,
                    widths.to_wr(),
                    wr::BorderDetails::Normal(wr::NormalBorder {
                        left: *left,
                        right: *right,
                        top: *top,
                        bottom: *bottom,
                        radius: radius.to_wr(),
                        do_aa: true,
                    }),
                );
            }

            DisplayItem::Image {
                clip_rect,
                image_key,
                image_size,
                rendering,
                alpha_type,
            } => {
                let bounds = clip_rect.to_wr();
                wr_list.push_image(
                    &wr::CommonItemProperties {
                        clip_rect: bounds,
                        clip_id: sc.clip_id(),
                        spatial_id: sc.spatial_id(),
                        flags: wr::PrimitiveFlags::empty(),
                    },
                    PxRect::from_size(*image_size).to_wr(),
                    *rendering,
                    *alpha_type,
                    *image_key,
                    wr::ColorF::WHITE,
                );
            }

            DisplayItem::LinearGradient {
                clip_rect,
                gradient,
                stops,
                tile_size,
                tile_spacing,
            } => {
                wr_list.push_stops(stops);
                let bounds = clip_rect.to_wr();
                wr_list.push_gradient(
                    &wr::CommonItemProperties {
                        clip_rect: bounds,
                        clip_id: sc.clip_id(),
                        spatial_id: sc.spatial_id(),
                        flags: wr::PrimitiveFlags::empty(),
                    },
                    bounds,
                    *gradient,
                    tile_size.to_wr(),
                    tile_spacing.to_wr(),
                )
            }
            DisplayItem::RadialGradient {
                clip_rect,
                gradient,
                stops,
                tile_size,
                tile_spacing,
            } => {
                wr_list.push_stops(stops);
                let bounds = clip_rect.to_wr();
                wr_list.push_radial_gradient(
                    &wr::CommonItemProperties {
                        clip_rect: bounds,
                        clip_id: sc.clip_id(),
                        spatial_id: sc.spatial_id(),
                        flags: wr::PrimitiveFlags::empty(),
                    },
                    bounds,
                    *gradient,
                    tile_size.to_wr(),
                    tile_spacing.to_wr(),
                )
            }
            DisplayItem::ConicGradient {
                clip_rect,
                gradient,
                stops,
                tile_size,
                tile_spacing,
            } => {
                wr_list.push_stops(stops);
                let bounds = clip_rect.to_wr();
                wr_list.push_conic_gradient(
                    &wr::CommonItemProperties {
                        clip_rect: bounds,
                        clip_id: sc.clip_id(),
                        spatial_id: sc.spatial_id(),
                        flags: wr::PrimitiveFlags::empty(),
                    },
                    bounds,
                    *gradient,
                    tile_size.to_wr(),
                    tile_spacing.to_wr(),
                )
            }
            DisplayItem::Line {
                clip_rect,
                color,
                style,
                wavy_line_thickness,
                orientation,
            } => {
                let bounds = clip_rect.to_wr();
                wr_list.push_line(
                    &wr::CommonItemProperties {
                        clip_rect: bounds,
                        clip_id: sc.clip_id(),
                        spatial_id: sc.spatial_id(),
                        flags: wr::PrimitiveFlags::empty(),
                    },
                    &bounds,
                    *wavy_line_thickness,
                    *orientation,
                    color,
                    *style,
                );
            }
        }
    }

    fn register_bindings(&self, bindings: &mut FxHashMap<wr::PropertyBindingId, (FrameId, usize)>, value: (FrameId, usize)) {
        match self {
            DisplayItem::PushReferenceFrame {
                transform: FrameValue::Bind { key, .. },
                ..
            } => {
                bindings.insert(key.id, value);
            }
            DisplayItem::PushStackingContext { filters, .. } => {
                for filter in filters.iter() {
                    if let FilterOp::Opacity(FrameValue::Bind { key, .. }) = filter {
                        bindings.insert(key.id, value);
                    }
                }
            }
            DisplayItem::Color {
                color: FrameValue::Bind { key, .. },
                ..
            } => {
                bindings.insert(key.id, value);
            }
            DisplayItem::Text {
                color: FrameValue::Bind { key, .. },
                ..
            } => {
                bindings.insert(key.id, value);
            }
            _ => {}
        }
    }

    /// Update the value and returns if a new full frame must be rendered.
    fn update_transform(&mut self, t: &FrameValueUpdate<PxTransform>) -> bool {
        match self {
            DisplayItem::PushReferenceFrame {
                transform:
                    FrameValue::Bind {
                        key,
                        value,
                        animating: animation,
                    },
                ..
            } if *key == t.key => FrameValue::update_bindable(value, animation, t),
            _ => false,
        }
    }
    fn update_float(&mut self, t: &FrameValueUpdate<f32>) -> bool {
        match self {
            DisplayItem::PushStackingContext { filters, .. } => {
                let mut new_frame = false;
                for filter in filters.iter_mut() {
                    match filter {
                        FilterOp::Opacity(FrameValue::Bind {
                            key,
                            value,
                            animating: animation,
                        }) if *key == t.key => {
                            new_frame |= FrameValue::update_bindable(value, animation, t);
                        }
                        _ => {}
                    }
                }
                new_frame
            }
            _ => false,
        }
    }
    fn update_color(&mut self, t: &FrameValueUpdate<wr::ColorF>) -> bool {
        match self {
            DisplayItem::Color {
                color:
                    FrameValue::Bind {
                        key,
                        value,
                        animating: animation,
                    },
                ..
            } if *key == t.key => FrameValue::update_bindable(value, animation, t),
            DisplayItem::Text {
                color: FrameValue::Bind { key, value, .. },
                ..
            } if *key == t.key => FrameValue::update_value(value, t),
            _ => false,
        }
    }
}

struct SpaceAndClip {
    spatial: Vec<wr::SpatialId>,
    clip: Vec<wr::ClipId>,
}
impl SpaceAndClip {
    pub fn new(pipeline_id: PipelineId) -> Self {
        let sid = wr::SpatialId::root_reference_frame(pipeline_id);
        let cid = wr::ClipId::root(pipeline_id);
        SpaceAndClip {
            spatial: vec![sid],
            clip: vec![cid],
        }
    }

    pub fn clip_id(&self) -> wr::ClipId {
        self.clip[self.clip.len() - 1]
    }

    pub fn spatial_id(&self) -> wr::SpatialId {
        self.spatial[self.spatial.len() - 1]
    }

    pub fn info(&self) -> wr::SpaceAndClipInfo {
        wr::SpaceAndClipInfo {
            spatial_id: self.spatial_id(),
            clip_id: self.clip_id(),
        }
    }

    pub fn pop_spatial(&mut self) {
        self.spatial.truncate(self.spatial.len() - 1);
    }

    pub fn pop_clip(&mut self) {
        self.clip.truncate(self.clip.len() - 1);
    }
}

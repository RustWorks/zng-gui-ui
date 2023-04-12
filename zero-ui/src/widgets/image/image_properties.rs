use super::*;
use std::fmt;

pub use crate::core::image::{ImageDownscale, ImageLimits};
pub use crate::core::render::ImageRendering;
use crate::core::window::{WindowLoadingHandle, WINDOW_CTRL};
use crate::widgets::window::nodes::BlockWindowLoad;
use nodes::CONTEXT_IMAGE_VAR;

/// Image layout mode.
///
/// This layout mode can be set to all images inside a widget using [`img_fit`], the [`image_presenter`] uses this value
/// to calculate the image final size.
///
/// The image desired size is its original size, either in pixels or DIPs after cropping and scaling.
///
/// [`img_fit`]: fn@img_fit
/// [`image_presenter`]: crate::widgets::image::nodes::image_presenter
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImageFit {
    /// The image original size is preserved, the image is clipped if larger then the final size.
    None,
    /// The image is resized to fill the final size, the aspect-ratio is not preserved.
    Fill,
    /// The image is resized to fit the final size, preserving the aspect-ratio.
    Contain,
    /// The image is resized to fill the final size while preserving the aspect-ratio.
    /// If the aspect ratio of the final size differs from the image, it is clipped.
    Cover,
    /// If the image is smaller then the final size applies the [`None`] layout, if its larger applies the [`Contain`] layout.
    ///
    /// [`None`]: ImageFit::None
    /// [`Contain`]: ImageFit::Contain
    ScaleDown,
}
impl fmt::Debug for ImageFit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ImageFit::")?
        }
        match self {
            Self::None => write!(f, "None"),
            Self::Fill => write!(f, "Fill"),
            Self::Contain => write!(f, "Contain"),
            Self::Cover => write!(f, "Cover"),
            Self::ScaleDown => write!(f, "ScaleDown"),
        }
    }
}

context_var! {
    /// The Image scaling algorithm in the renderer.
    ///
    /// Is [`ImageRendering::Auto`] by default.
    pub static IMAGE_RENDERING_VAR: ImageRendering = ImageRendering::Auto;

    /// If the image is cached.
    ///
    /// Is `true` by default.
    pub static IMAGE_CACHE_VAR: bool = true;

    /// Widget function for the content shown when the image does not load.
    pub static IMAGE_ERROR_GEN_VAR: WidgetFn<ImageErrorArgs> = WidgetFn::nil();

    /// Widget function for the content shown when the image is still loading.
    pub static IMAGE_LOADING_GEN_VAR: WidgetFn<ImageLoadingArgs> = WidgetFn::nil();

    /// Custom image load and decode limits.
    ///
    /// Set to `None` to use the [`IMAGES::limits`].
    pub static IMAGE_LIMITS_VAR: Option<ImageLimits> = None;

    /// Custom resize applied during image decode.
    ///
    /// Is `None` by default.
    pub static IMAGE_DOWNSCALE_VAR: Option<ImageDownscale> = None;

    /// The image layout mode.
    ///
    /// Is [`ImageFit::Contain`] by default.
    pub static IMAGE_FIT_VAR: ImageFit = ImageFit::Contain;

    /// Scaling applied to the image desired size.
    ///
    /// Does not scale by default, `1.0`.
    pub static IMAGE_SCALE_VAR: Factor2d = Factor2d::identity();

    /// If the image desired size is scaled by the screen scale factor.
    ///
    /// Is `true` by default.
    pub static IMAGE_SCALE_FACTOR_VAR: bool = true;

    /// If the image desired size is scaled considering the image and screen PPIs.
    ///
    /// Is `false` by default.
    pub static IMAGE_SCALE_PPI_VAR: bool = false;

    /// Align of the image in relation to the image widget final size.
    ///
    /// Is [`Align::CENTER`] by default.
    pub static IMAGE_ALIGN_VAR: Align = Align::CENTER;

    /// Offset applied to the image after all measure and arrange.
    pub static IMAGE_OFFSET_VAR: Vector = Vector::default();

    /// Simple clip applied to the image before layout.
    ///
    /// No cropping is done by default.
    pub static IMAGE_CROP_VAR: Rect = Rect::default();
}

/// Sets the [`ImageFit`] of all inner images.
///
/// This property sets the [`IMAGE_FIT_VAR`].
///
/// [`fit`]: fn@crate::widgets::image::fit
#[property(CONTEXT, default(IMAGE_FIT_VAR), impl(Image))]
pub fn img_fit(child: impl UiNode, fit: impl IntoVar<ImageFit>) -> impl UiNode {
    with_context_var(child, IMAGE_FIT_VAR, fit)
}

/// Sets the scale applied to all inner images.
///
/// The scaling is applied after [`img_scale_ppi`] if active.
///
/// By default not scaling is done.
///
/// [`img_scale_ppi`]: fn@img_scale_ppi
/// [`scale`]: fn@crate::widgets::image::scale
#[property(CONTEXT, default(IMAGE_SCALE_VAR), impl(Image))]
pub fn img_scale(child: impl UiNode, scale: impl IntoVar<Factor2d>) -> impl UiNode {
    with_context_var(child, IMAGE_SCALE_VAR, scale)
}

/// If the image desired size is scaled by the screen scale factor.
///
/// The image desired size is its original size after [`img_crop`], it is a pixel value, but widgets are layout using
/// device independent pixels that automatically scale in higher definition displays, when this property is enabled
/// the image size is also scaled so that the image will take the same screen space in all devices, the image can end
///
/// This is enabled by default.
///
/// [`img_crop`]: fn@img_crop
#[property(CONTEXT, default(IMAGE_SCALE_FACTOR_VAR), impl(Image))]
pub fn img_scale_factor(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, IMAGE_SCALE_FACTOR_VAR, enabled)
}

/// Sets if the image desired size is scaled considering the image and monitor PPI.
///
/// If the image desired size is scaled by PPI.
///
/// The image desired size is its original size, after [`img_crop`], and it can be in pixels or scaled considering
/// the image PPI, monitor PPI and scale factor.
///
/// By default this is `false`, if `true` the image is scaled in a attempt to recreate the original physical dimensions, but it
/// only works if the image and monitor PPI are set correctly. The monitor PPI can be set using the [`MONITORS`] service.
///
/// [`img_crop`]: fn@img_crop
/// [`MONITORS`]: zero_ui::core::window::MONITORS
///
/// [`scape_ppi`]: fn@crate::widgets::image::scape_ppi
#[property(CONTEXT, default(IMAGE_SCALE_PPI_VAR), impl(Image))]
pub fn img_scale_ppi(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, IMAGE_SCALE_PPI_VAR, enabled)
}

/// Sets the [`Align`] of all inner images within each image widget area.
///
/// If the image is smaller then the widget area it is aligned like normal, if it is larger the "viewport" it is aligned to clip,
/// for example, alignment [`BOTTOM_RIGHT`] makes a smaller image sit at the bottom-right of the widget and makes
/// a larger image bottom-right fill the widget, clipping the rest.
///
/// By default the alignment is [`CENTER`]. The [`BASELINE`] alignment is treaded the same as [`BOTTOM`].
///
/// [`BOTTOM_RIGHT`]: Align::BOTTOM_RIGHT
/// [`CENTER`]: Align::CENTER
/// [`BASELINE`]: Align::BASELINE
/// [`BOTTOM`]: Align::BOTTOM
///
/// [`img_align`]: fn@crate::widgets::image::img_align
#[property(CONTEXT, default(IMAGE_ALIGN_VAR), impl(Image))]
pub fn img_align(child: impl UiNode, fit: impl IntoVar<Align>) -> impl UiNode {
    with_context_var(child, IMAGE_ALIGN_VAR, fit)
}

/// Sets a [`Point`] that is an offset applied to all inner images within each image widget area.
///
/// Relative values are calculated from the widget final size. Note that this is different the applying the
/// [`offset`] property on the widget it-self, the widget is not moved just the image within the widget area.
///
/// This property sets the [`IMAGE_OFFSET_VAR`]. By default no offset is applied.
///
/// [`offset`]: fn@crate::properties::offset
/// [`img_offset`]: fn@crate::widgets::image::img_offset
#[property(CONTEXT, default(IMAGE_OFFSET_VAR), impl(Image))]
pub fn img_offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
    with_context_var(child, IMAGE_OFFSET_VAR, offset)
}

/// Sets a [`Rect`] that is a clip applied to all inner images before their layout.
///
/// Relative values are calculated from the image pixel size, the [`img_scale_ppi`] is only considered after.
/// Note that more complex clipping can be applied after to the full widget, this property exists primarily to
/// render selections of a [texture atlas].
///
/// By default no cropping is done.
///
/// [`img_scale_ppi`]: #fn@img_scale_ppi
/// [texture atlas]: https://en.wikipedia.org/wiki/Texture_atlas///
/// [`crop`]: fn@crate::widgets::image::crop
#[property(CONTEXT, default(IMAGE_CROP_VAR), impl(Image))]
pub fn img_crop(child: impl UiNode, crop: impl IntoVar<Rect>) -> impl UiNode {
    with_context_var(child, IMAGE_CROP_VAR, crop)
}

/// Sets the [`ImageRendering`] of all inner images.
///
/// If the image layout size is not the same as the `source` pixel size the image must be re-scaled
/// during rendering, this property selects what algorithm is used to do this re-scaling.
///
/// Note that the algorithms used in the renderer value performance over quality and do a good
/// enough job for small or temporary changes in scale only. If the image stays at a very different scale
/// after a short time a CPU re-scale task is automatically started to generate a better quality re-scaling.
///
/// If the image is an app resource known during build time you should consider pre-scaling it to match the screen
/// size at different DPIs using mipmaps.
///
/// This is [`ImageRendering::Auto`] by default.
///
/// [`rendering`]: fn@crate::widgets::image::rendering
#[property(CONTEXT, default(IMAGE_RENDERING_VAR), impl(Image))]
pub fn img_rendering(child: impl UiNode, rendering: impl IntoVar<ImageRendering>) -> impl UiNode {
    with_context_var(child, IMAGE_RENDERING_VAR, rendering)
}

/// Sets the cache mode of all inner images.
///
/// Sets if the [`source`] is cached.
///
/// By default this is `true`, meaning the image is loaded from cache and if not present it is inserted into
/// the cache, the cache lives for the app in the [`IMAGES`] service, the image can be manually removed from cache.
///
/// If set to `false` the image is always loaded and decoded on init or when [`source`] updates and is dropped when
/// the widget is deinited or dropped.
///
/// [`source`]: fn@crate::widgets::image::source
/// [`IMAGES`]: zero_ui::core::image::IMAGES
#[property(CONTEXT, default(IMAGE_CACHE_VAR), impl(Image))]
pub fn img_cache(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, IMAGE_CACHE_VAR, enabled)
}

/// Sets custom image load and decode limits.
///
/// If not set or set to `None` the [`IMAGES.limits`] is used.
///
/// See also [`img_downscale`] for a way to still display unexpected large images.
///
/// [`IMAGES.limits`]: crate::core::image::IMAGES::limits
/// [`img_downscale`]: fn@img_downscale
#[property(CONTEXT, default(IMAGE_LIMITS_VAR), impl(Image))]
pub fn img_limits(child: impl UiNode, limits: impl IntoVar<Option<ImageLimits>>) -> impl UiNode {
    with_context_var(child, IMAGE_LIMITS_VAR, limits)
}

/// Custom pixel resize applied during image load/decode.
///
/// Note that this resize affects the image actual pixel size directly when it is loading to force the image pixels to be within an expected size.
/// This property primary use is as error recover before the [`img_limits`] error happens, you set the limits to the size that should not even
/// be processed and set this property to the maximum size expected.
///
/// Changing this value after an image is already loaded or loading will cause the image to reload, image cache allocates different
/// entries for different downscale values, this means that this property should never be used for responsive resize,use the widget
/// size and other properties to efficiently resize an image on screen.
///
/// [`IMAGES.limits`]: crate::core::image::IMAGES::limits
/// [`img_limits`]: fn@img_limits
#[property(CONTEXT, default(IMAGE_DOWNSCALE_VAR), impl(Image))]
pub fn img_downscale(child: impl UiNode, downscale: impl IntoVar<Option<ImageDownscale>>) -> impl UiNode {
    with_context_var(child, IMAGE_DOWNSCALE_VAR, downscale)
}

/// If the [`CONTEXT_IMAGE_VAR`] is an error.
#[property(LAYOUT)]
pub fn is_error(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_is_state(child, CONTEXT_IMAGE_VAR.map(|m| m.is_error()), state)
}

/// If the [`CONTEXT_IMAGE_VAR`] is a successfully loaded image.
#[property(LAYOUT)]
pub fn is_loaded(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_is_state(child, CONTEXT_IMAGE_VAR.map(|m| m.is_loaded()), state)
}

/// Sets the [`wgt_fn!`] that is used to create a content for the error message.
///
/// [`wgt_fn!`]: crate::widgets::wgt_fn
#[property(CONTEXT, default(IMAGE_ERROR_GEN_VAR), impl(Image))]
pub fn img_error_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<ImageErrorArgs>>) -> impl UiNode {
    with_context_var(child, IMAGE_ERROR_GEN_VAR, wgt_fn)
}

/// Sets the [`wgt_fn!`] that is used to create a content for the error message.
///
/// [`wgt_fn!`]: crate::widgets::wgt_fn
#[property(CONTEXT, default(IMAGE_LOADING_GEN_VAR), impl(Image))]
pub fn img_loading_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<ImageLoadingArgs>>) -> impl UiNode {
    with_context_var(child, IMAGE_LOADING_GEN_VAR, wgt_fn)
}

/// Arguments for [`img_loading_fn`].
///
/// [`img_loading_fn`]: fn@img_loading_fn
#[derive(Clone, Debug)]
pub struct ImageLoadingArgs {}

/// Arguments for [`on_load`].
///
/// [`on_load`]: fn@on_load
#[derive(Clone, Debug)]
pub struct ImageLoadArgs {}

/// Arguments for [`on_error`] and [`img_error_fn`].
///
/// [`on_error`]: fn@on_error
/// [`img_error_fn`]: fn@img_error_fn
#[derive(Clone, Debug)]
pub struct ImageErrorArgs {
    /// Error message.
    pub error: Text,
}

/// Image load or decode error event.
///
/// This property calls `handler` every time the [`CONTEXT_IMAGE_VAR`] updates with a different error.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// # Route
///
/// This property is not routed, it works only inside a widget that loads images. There is also no *preview* event.
#[property(EVENT, impl(Image))]
pub fn on_error(child: impl UiNode, handler: impl WidgetHandler<ImageErrorArgs>) -> impl UiNode {
    #[ui_node(struct OnErrorNode {
        child: impl UiNode,
        handler: impl WidgetHandler<ImageErrorArgs>,
        error: Text,
    })]
    impl UiNode for OnErrorNode {
        fn init(&mut self) {
            WIDGET.sub_var(&CONTEXT_IMAGE_VAR);

            CONTEXT_IMAGE_VAR.with(|i| {
                if let Some(error) = i.error() {
                    self.error = error;
                    self.handler.event(&ImageErrorArgs { error: self.error.clone() });
                }
            });
            self.child.init();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if let Some(new_img) = CONTEXT_IMAGE_VAR.get_new() {
                if let Some(error) = new_img.error() {
                    if self.error != error {
                        self.error = error;
                        self.handler.event(&ImageErrorArgs { error: self.error.clone() });
                    }
                } else {
                    self.error = "".into();
                }
            }

            self.handler.update();
            self.child.update(updates);
        }
    }
    OnErrorNode {
        child,
        handler,
        error: "".into(),
    }
}

/// Image loaded event.
///
/// This property calls `handler` every time the [`CONTEXT_IMAGE_VAR`] updates with a successfully loaded image.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// # Route
///
/// This property is not routed, it works only inside a widget that loads images. There is also no *preview* event.
#[property(EVENT, impl(Image))]
pub fn on_load(child: impl UiNode, handler: impl WidgetHandler<ImageLoadArgs>) -> impl UiNode {
    #[ui_node(struct OnLoadNode {
        child: impl UiNode,
        handler: impl WidgetHandler<ImageLoadArgs>,
    })]
    impl UiNode for OnLoadNode {
        fn init(&mut self) {
            WIDGET.sub_var(&CONTEXT_IMAGE_VAR);

            if CONTEXT_IMAGE_VAR.with(Image::is_loaded) {
                self.handler.event(&ImageLoadArgs {});
            }
            self.child.init();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if let Some(new_img) = CONTEXT_IMAGE_VAR.get_new() {
                if new_img.is_loaded() {
                    self.handler.event(&ImageLoadArgs {});
                }
            }

            self.handler.update();
            self.child.update(updates);
        }
    }
    OnLoadNode { child, handler }
}

/// Block window load until image is loaded.
///
/// If the image widget is in the initial window content a [`WindowLoadingHandle`] is used to delay the window
/// visually opening until the source loads, fails to load or a timeout elapses. By default `true` sets the timeout to 1 second.
#[property(LAYOUT, default(false), impl(Image))]
pub fn img_block_window_load(child: impl UiNode, enabled: impl IntoValue<BlockWindowLoad>) -> impl UiNode {
    #[ui_node(struct ImageBlockWindowLoadNode {
        child: impl UiNode,
        enabled: BlockWindowLoad,
        block: Option<WindowLoadingHandle>,
    })]
    impl UiNode for ImageBlockWindowLoadNode {
        fn init(&mut self) {
            WIDGET.sub_var(&CONTEXT_IMAGE_VAR);

            if let Some(delay) = self.enabled.deadline() {
                self.block = WINDOW_CTRL.loading_handle(delay);
            }
            self.child.init();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if self.block.is_some() && !CONTEXT_IMAGE_VAR.with(Image::is_loading) {
                self.block = None;
            }
            self.child.update(updates);
        }
    }
    ImageBlockWindowLoadNode {
        child: child.cfg_boxed(),
        enabled: enabled.into(),
        block: None,
    }
    .cfg_boxed()
}

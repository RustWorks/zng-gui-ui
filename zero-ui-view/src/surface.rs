use std::{collections::VecDeque, fmt};

use winit::event_loop::EventLoopWindowTarget;

use tracing::span::EnteredSpan;
use webrender::{
    api::{
        ColorF, DocumentId, DynamicProperties, FontInstanceKey, FontInstanceOptions, FontInstancePlatformOptions, FontKey, FontVariation,
        IdNamespace, ImageKey, PipelineId,
    },
    RenderApi, Renderer, Transaction,
};
use zero_ui_view_api::{
    units::*, ApiExtensionId, ApiExtensionPayload, DisplayListCache, FrameId, FrameRequest, FrameUpdateRequest, HeadlessRequest, ImageId,
    ImageLoadedData, RenderMode, ViewProcessGen, WindowId,
};

use crate::{
    extensions::{
        BlobExtensionsImgHandler, DisplayListExtAdapter, RendererCommandArgs, RendererConfigArgs, RendererCreatedArgs, RendererExtension,
    },
    gl::{GlContext, GlContextManager},
    image_cache::{Image, ImageCache, ImageUseMap, WrImageCache},
    util::PxToWinit,
    AppEvent, AppEventSender, FrameReadyMsg, WrNotifier,
};

/// A headless "window".
pub(crate) struct Surface {
    id: WindowId,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    api: RenderApi,
    size: DipSize,
    scale_factor: f32,

    context: GlContext,
    renderer: Option<Renderer>,
    renderer_exts: Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
    image_use: ImageUseMap,

    display_list_cache: DisplayListCache,
    clear_color: Option<ColorF>,

    pending_frames: VecDeque<(FrameId, bool, Option<EnteredSpan>)>,
    rendered_frame_id: FrameId,
    resized: bool,
}
impl fmt::Debug for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .field("size", &self.size)
            .field("scale_factor", &self.scale_factor)
            .finish_non_exhaustive()
    }
}
impl Surface {
    pub fn open(
        gen: ViewProcessGen,
        mut cfg: HeadlessRequest,
        window_target: &EventLoopWindowTarget<AppEvent>,
        gl_manager: &mut GlContextManager,
        mut renderer_exts: Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
        event_sender: AppEventSender,
    ) -> Self {
        let id = cfg.id;

        let mut context = gl_manager.create_headless(id, window_target, cfg.render_mode);
        let size = cfg.size.to_px(cfg.scale_factor);
        context.resize(size.to_winit());
        let context = context;

        let mut opts = webrender::WebRenderOptions {
            // text-aa config from Firefox.
            enable_aa: true,
            enable_subpixel_aa: cfg!(not(target_os = "android")),

            renderer_id: Some((gen.get() as u64) << 32 | id.get() as u64),

            // this clear color paints over the one set using `Renderer::set_clear_color`.
            clear_color: ColorF::new(0.0, 0.0, 0.0, 0.0),

            allow_advanced_blend_equation: context.is_software(),
            clear_caches_with_quads: !context.is_software(),
            enable_gpu_markers: !context.is_software(),

            // extensions expect this to be set.
            workers: Some(crate::util::wr_workers()),

            //panic_on_gl_error: true,
            ..Default::default()
        };
        let mut blobs = BlobExtensionsImgHandler(vec![]);
        for (id, ext) in &mut renderer_exts {
            let cfg = cfg
                .extensions
                .iter()
                .position(|(k, _)| k == id)
                .map(|i| cfg.extensions.swap_remove(i).1);
            ext.configure(&mut RendererConfigArgs {
                config: cfg,
                options: &mut opts,
                blobs: &mut blobs.0,
            });
        }
        if !opts.enable_multithreading {
            for b in &mut blobs.0 {
                b.enable_multithreading(false);
            }
        }
        opts.blob_image_handler = Some(Box::new(blobs));

        let device_size = cfg.size.to_px(cfg.scale_factor).to_wr_device();

        let (mut renderer, sender) =
            webrender::create_webrender_instance(context.gl().clone(), WrNotifier::create(id, event_sender), opts, None).unwrap();
        renderer.set_external_image_handler(WrImageCache::new_boxed());

        let mut api = sender.create_api();
        let document_id = api.add_document(device_size);
        let pipeline_id = webrender::api::PipelineId(gen.get(), id.get());

        renderer_exts.retain_mut(|(_, ext)| {
            ext.renderer_created(&mut RendererCreatedArgs {
                renderer: &mut renderer,
                api_sender: &sender,
                api: &mut api,
                document_id,
                pipeline_id,
            });
            !ext.is_config_only()
        });

        Self {
            id,
            pipeline_id,
            document_id,
            api,
            size: cfg.size,
            scale_factor: cfg.scale_factor,

            context,
            renderer: Some(renderer),
            renderer_exts,
            image_use: ImageUseMap::default(),

            display_list_cache: DisplayListCache::new(pipeline_id),
            clear_color: None,

            pending_frames: VecDeque::new(),
            rendered_frame_id: FrameId::INVALID,
            resized: true,
        }
    }

    pub fn render_mode(&self) -> RenderMode {
        self.context.render_mode()
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn id_namespace(&self) -> IdNamespace {
        self.api.get_namespace_id()
    }

    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    pub fn frame_id(&self) -> FrameId {
        self.rendered_frame_id
    }

    pub fn set_size(&mut self, size: DipSize, scale_factor: f32) {
        if self.size != size || (self.scale_factor - scale_factor).abs() > 0.001 {
            self.size = size;
            self.scale_factor = scale_factor;
            self.context.make_current();
            let px_size = size.to_px(self.scale_factor);
            self.context.resize(px_size.to_winit());
            self.resized = true;
        }
    }

    pub fn use_image(&mut self, image: &Image) -> ImageKey {
        self.image_use.new_use(image, self.document_id, &mut self.api)
    }

    pub fn update_image(&mut self, key: ImageKey, image: &Image) {
        self.image_use.update_use(key, image, self.document_id, &mut self.api);
    }

    pub fn delete_image(&mut self, key: ImageKey) {
        self.image_use.delete(key, self.document_id, &mut self.api);
    }

    pub fn add_font(&mut self, font: Vec<u8>, index: u32) -> FontKey {
        let key = self.api.generate_font_key();
        let mut txn = webrender::Transaction::new();
        txn.add_raw_font(key, font, index);
        self.api.send_transaction(self.document_id, txn);
        key
    }

    pub fn delete_font(&mut self, key: FontKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font(key);
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn add_font_instance(
        &mut self,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<FontInstanceOptions>,
        plataform_options: Option<FontInstancePlatformOptions>,
        variations: Vec<FontVariation>,
    ) -> FontInstanceKey {
        let key = self.api.generate_font_instance_key();
        let mut txn = webrender::Transaction::new();
        txn.add_font_instance(key, font_key, glyph_size.to_wr().get(), options, plataform_options, variations);
        self.api.send_transaction(self.document_id, txn);
        key
    }

    pub fn delete_font_instance(&mut self, instance_key: FontInstanceKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font_instance(instance_key);
        self.api.send_transaction(self.document_id, txn);
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;
            let rect = PxRect::from_size(self.size.to_px(self.scale_factor)).to_wr_device();
            txn.set_document_view(rect);
        }
    }

    pub fn render(&mut self, frame: FrameRequest) {
        let _span = tracing::trace_span!("render").entered();

        let render_reasons = frame.render_reasons();

        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color);

        let mut txn = Transaction::new();
        txn.reset_dynamic_properties();
        txn.append_dynamic_properties(DynamicProperties {
            transforms: vec![],
            floats: vec![],
            colors: vec![],
        });

        let display_list = frame.display_list.to_webrender(
            &mut DisplayListExtAdapter {
                extensions: &mut self.renderer_exts,
                transaction: &mut txn,
                renderer: self.renderer.as_mut().unwrap(),
                api: &mut self.api,
            },
            &mut self.display_list_cache,
        );

        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color);
        self.clear_color = Some(frame.clear_color);

        txn.set_display_list(frame.id.epoch(), (frame.pipeline_id, display_list));

        txn.set_root_pipeline(self.pipeline_id);

        self.push_resize(&mut txn);

        txn.generate_frame(frame.id.get(), render_reasons);

        let frame_scope =
            tracing::trace_span!("<frame>", ?frame.id, capture_image = ?frame.capture_image, from_update = false, thread = "<webrender>")
                .entered();
        self.pending_frames.push_back((frame.id, frame.capture_image, Some(frame_scope)));

        self.api.send_transaction(self.document_id, txn);
    }

    pub fn render_update(&mut self, frame: FrameUpdateRequest) {
        let _span = tracing::trace_span!("render_update").entered();

        let render_reasons = frame.render_reasons();

        if let Some(color) = frame.clear_color {
            self.clear_color = Some(color);
            self.renderer.as_mut().unwrap().set_clear_color(color);
        }

        let resized = self.resized;

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        self.push_resize(&mut txn);
        txn.generate_frame(self.frame_id().get(), render_reasons);

        let frame_scope = match self.display_list_cache.update(
            &mut DisplayListExtAdapter {
                extensions: &mut self.renderer_exts,
                transaction: &mut txn,
                renderer: self.renderer.as_mut().unwrap(),
                api: &mut self.api,
            },
            frame.transforms,
            frame.floats,
            frame.colors,
            frame.extensions,
            resized,
        ) {
            Ok(p) => {
                if let Some(p) = p {
                    txn.append_dynamic_properties(p);
                }

                tracing::trace_span!("<frame-update>", ?frame.id, capture_image = ?frame.capture_image, thread = "<webrender>")
            }
            Err(d) => {
                txn.reset_dynamic_properties();
                txn.append_dynamic_properties(DynamicProperties {
                    transforms: vec![],
                    floats: vec![],
                    colors: vec![],
                });

                txn.set_display_list(frame.id.epoch(), (self.pipeline_id, d));

                tracing::trace_span!("<frame>", ?frame.id, capture_image = ?frame.capture_image, from_update = true, thread = "<webrender>")
            }
        };

        self.pending_frames
            .push_back((frame.id, frame.capture_image, Some(frame_scope.entered())));

        self.api.send_transaction(self.document_id, txn);
    }

    pub fn on_frame_ready(&mut self, msg: FrameReadyMsg, images: &mut ImageCache) -> (FrameId, Option<ImageLoadedData>) {
        let (frame_id, capture, _) = self.pending_frames.pop_front().unwrap_or((self.rendered_frame_id, false, None));
        self.rendered_frame_id = frame_id;

        let mut captured_data = None;

        if msg.composite_needed || capture {
            self.context.make_current();
            let renderer = self.renderer.as_mut().unwrap();

            if msg.composite_needed {
                renderer.update();
                renderer.render((self.size.to_px(self.scale_factor)).to_wr_device(), 0).unwrap();
                let _ = renderer.flush_pipeline_info();
            }
            if capture {
                captured_data = Some(images.frame_image_data(
                    renderer,
                    PxRect::from_size(self.size.to_px(self.scale_factor)),
                    true,
                    self.scale_factor,
                ));
            }
        }
        (frame_id, captured_data)
    }

    pub fn frame_image(&mut self, images: &mut ImageCache) -> ImageId {
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            PxRect::from_size(self.size.to_px(self.scale_factor)),
            true,
            self.id,
            self.rendered_frame_id,
            self.scale_factor,
        )
    }

    pub fn frame_image_rect(&mut self, images: &mut ImageCache, rect: PxRect) -> ImageId {
        let rect = PxRect::from_size(self.size.to_px(self.scale_factor)).intersection(&rect).unwrap();
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            rect,
            true,
            self.id,
            self.rendered_frame_id,
            self.scale_factor,
        )
    }

    /// Calls the render extension command.
    pub fn render_extension(&mut self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> ApiExtensionPayload {
        for (id, ext) in &mut self.renderer_exts {
            if *id == extension_id {
                return ext.command(&mut RendererCommandArgs {
                    renderer: self.renderer.as_mut().unwrap(),
                    api: &mut self.api,
                    request,
                });
            }
        }
        ApiExtensionPayload::unknown_extension(extension_id)
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        self.context.make_current();
        self.renderer.take().unwrap().deinit();
    }
}

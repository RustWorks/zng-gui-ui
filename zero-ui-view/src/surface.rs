use std::{collections::VecDeque, fmt};

use gleam::gl;
use glutin::{dpi::PhysicalSize, event_loop::EventLoopWindowTarget, ContextBuilder, GlRequest};
use webrender::{
    api::{
        BuiltDisplayList, DisplayListPayload, DocumentId, DynamicProperties, FontInstanceKey, FontInstanceOptions,
        FontInstancePlatformOptions, FontKey, FontVariation, HitTestResult, IdNamespace, ImageKey, PipelineId, RenderNotifier,
        ScrollClamping,
    },
    RenderApi, Renderer, RendererOptions, Transaction,
};
use zero_ui_view_api::{
    units::*, FrameId, FrameRequest, FrameUpdateRequest, HeadlessOpenData, HeadlessRequest, ImageId, ImageLoadedData, TextAntiAliasing,
    ViewProcessGen, WindowId,
};

use crate::{
    image_cache::{Image, ImageCache, ImageUseMap, WrImageCache},
    util::{GlContextManager, GlHeadlessContext},
    AppEvent, AppEventSender, FrameReadyMsg,
};

/// A headless "window".
pub(crate) struct Surface {
    id: WindowId,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    documents: Vec<DocumentId>,
    api: RenderApi,
    size: DipSize,
    scale_factor: f32,

    context: GlHeadlessContext,
    renderer: Option<Renderer>,
    image_use: ImageUseMap,
    rbos: [u32; 2],
    fbo: u32,

    pending_frames: VecDeque<(FrameId, bool)>,
    rendered_frame_id: FrameId,
    resized: bool,
}
impl fmt::Debug for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .field("document", &self.documents)
            .field("size", &self.size)
            .field("scale_factor", &self.scale_factor)
            .finish_non_exhaustive()
    }
}
impl Surface {
    pub fn open(
        gen: ViewProcessGen,
        cfg: HeadlessRequest,
        window_target: &EventLoopWindowTarget<AppEvent>,
        gl_manager: &mut GlContextManager,
        event_sender: impl AppEventSender,
    ) -> Self {
        let id = cfg.id;
        let context = ContextBuilder::new()
            .with_gl(GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .with_hardware_acceleration(None);

        let size_one = PhysicalSize::new(1, 1);
        #[cfg(target_os = "linux")]
        let context = {
            use glutin::platform::unix::HeadlessContextExt;
            match context.clone().build_surfaceless(window_target) {
                Ok(ctx) => ctx,
                Err(suf_e) => match context.clone().build_headless(window_target, size_one) {
                    Ok(ctx) => ctx,
                    Err(hea_e) => match context.build_osmesa(size_one) {
                        Ok(ctx) => ctx,
                        Err(osm_e) => panic!(
                            "failed all headless modes supported in linux\nsurfaceless: {:?}\n\nheadless: {:?}\n\n osmesa: {:?}",
                            suf_e, hea_e, osm_e
                        ),
                    },
                },
            }
        };
        #[cfg(not(target_os = "linux"))]
        let context = context
            .build_headless(window_target, size_one)
            .expect("failed to build headless context");

        #[cfg(software)]
        let context = gl_manager.manage_headless(id, context, None);
        #[cfg(not(software))]
        let context = gl_manager.manage_headless(id, context);

        let gl = context.gl();

        // manually create a surface.
        let rbos = gl.gen_renderbuffers(2);
        let rbos = [rbos[0], rbos[1]];
        let fbo = gl.gen_framebuffers(1)[0];

        resize(gl, rbos, cfg.size, cfg.scale_factor);

        gl.bind_framebuffer(gl::FRAMEBUFFER, fbo);
        gl.framebuffer_renderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::RENDERBUFFER, rbos[0]);
        gl.framebuffer_renderbuffer(gl::FRAMEBUFFER, gl::DEPTH_STENCIL_ATTACHMENT, gl::RENDERBUFFER, rbos[1]);

        let mut text_aa = cfg.text_aa;
        if let TextAntiAliasing::Default = cfg.text_aa {
            text_aa = TextAntiAliasing::Alpha;
        }

        let opts = RendererOptions {
            enable_aa: text_aa != TextAntiAliasing::Mono,
            enable_subpixel_aa: text_aa == TextAntiAliasing::Subpixel,
            renderer_id: Some((gen as u64) << 32 | id as u64),
            //panic_on_gl_error: true,
            // TODO expose more options to the user.
            ..Default::default()
        };

        let device_size = cfg.size.to_px(cfg.scale_factor).to_wr_device();

        let (mut renderer, sender) =
            webrender::Renderer::new(context.share_gl(), Box::new(Notifier { id, sender: event_sender }), opts, None).unwrap();
        renderer.set_external_image_handler(WrImageCache::new_boxed());

        let api = sender.create_api();
        let document_id = api.add_document(device_size);

        let pipeline_id = webrender::api::PipelineId(gen, id);

        Self {
            id,
            pipeline_id,
            document_id,
            documents: vec![],
            api,
            size: cfg.size,
            scale_factor: cfg.scale_factor,

            context,
            renderer: Some(renderer),
            image_use: ImageUseMap::default(),
            rbos,
            fbo,

            pending_frames: VecDeque::new(),
            rendered_frame_id: FrameId::INVALID,
            resized: true,
        }
    }

    pub fn open_document(&mut self, scale_factor: f32, initial_size: DipSize) -> HeadlessOpenData {
        let document_id = self.api.add_document(initial_size.to_px(scale_factor).to_wr_device());
        self.documents.push(document_id);
        HeadlessOpenData {
            id_namespace: self.id_namespace(),
            pipeline_id: self.pipeline_id,
            document_id,
        }
    }

    pub fn close_document(&mut self, document_id: DocumentId) {
        if let Some(i) = self.documents.iter().position(|&d| d == document_id) {
            self.documents.swap_remove(i);
            self.api.delete_document(document_id);
        }
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

    /// Root document ID.
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn frame_id(&self) -> FrameId {
        self.rendered_frame_id
    }

    pub fn size(&self) -> DipSize {
        self.size
    }

    pub fn set_size(&mut self, document_id: DocumentId, size: DipSize, scale_factor: f32) {
        if self.size != size || (self.scale_factor - scale_factor).abs() > 0.001 {
            if self.document_id == document_id {
                self.size = size;
                self.scale_factor = scale_factor;
                self.context.make_current();
                resize(self.context.gl(), self.rbos, size, scale_factor);
                self.resized = true;
            } else {
                todo!()
            }
        }
    }

    pub fn use_image(&mut self, image: &Image) -> ImageKey {
        self.image_use.new_use(image, self.document_id(), &mut self.api)
    }

    pub fn update_image(&mut self, key: ImageKey, image: &Image) {
        self.image_use.update_use(key, image, self.document_id(), &mut self.api);
    }

    pub fn delete_image(&mut self, key: ImageKey) {
        self.image_use.delete(key, self.document_id(), &mut self.api);
    }

    pub fn add_font(&mut self, font: Vec<u8>, index: u32) -> FontKey {
        let key = self.api.generate_font_key();
        let mut txn = webrender::Transaction::new();
        txn.add_raw_font(key, font, index);
        self.api.send_transaction(self.document_id(), txn);
        key
    }

    pub fn delete_font(&mut self, key: FontKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font(key);
        self.api.send_transaction(self.document_id(), txn);
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
        self.api.send_transaction(self.document_id(), txn);
        key
    }

    pub fn delete_font_instance(&mut self, instance_key: FontInstanceKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font_instance(instance_key);
        self.api.send_transaction(self.document_id(), txn);
    }

    pub fn set_text_aa(&mut self, aa: TextAntiAliasing) {
        todo!("need to rebuild the renderer? {:?}", aa)
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;
            let rect = PxRect::from_size(self.size.to_px(self.scale_factor)).to_wr_device();
            txn.set_document_view(rect);
        }
    }

    pub fn render(&mut self, frame: FrameRequest) {
        let render_reasons = frame.render_reasons();

        self.pending_frames.push_back((frame.id, frame.capture_image));
        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color);

        let mut txn = Transaction::new();
        txn.reset_dynamic_properties();
        txn.append_dynamic_properties(DynamicProperties {
            transforms: vec![],
            floats: vec![],
            colors: vec![],
        });

        let display_list = BuiltDisplayList::from_data(
            DisplayListPayload {
                items_data: frame.display_list.0.to_vec(),
                cache_data: frame.display_list.1.to_vec(),
                spatial_tree: frame.display_list.2.to_vec(),
            },
            frame.display_list.3,
        );
        let viewport_size = self.size.to_px(self.scale_factor).to_wr();
        txn.set_display_list(
            frame.id.epoch(),
            Some(frame.clear_color),
            viewport_size,
            (frame.pipeline_id, display_list),
        );
        txn.set_root_pipeline(self.pipeline_id);

        self.push_resize(&mut txn);

        txn.generate_frame(frame.id.get(), render_reasons);
        self.api.send_transaction(self.document_id(), txn);
    }

    pub fn render_update(&mut self, frame: FrameUpdateRequest) {
        let render_reasons = frame.render_reasons();

        if let Some(color) = frame.clear_color {
            self.renderer.as_mut().unwrap().set_clear_color(color);
        }

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        txn.reset_dynamic_properties();
        txn.append_dynamic_properties(frame.updates);
        for (scroll_id, offset) in frame.scroll_updates {
            txn.scroll_node_with_id(offset.to_point().to_wr(), scroll_id, ScrollClamping::NoClamping);
        }

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id().get(), render_reasons);
        self.api.send_transaction(self.document_id(), txn);
    }

    pub fn on_frame_ready<S: AppEventSender>(
        &mut self,
        msg: FrameReadyMsg,
        images: &mut ImageCache<S>,
    ) -> (FrameId, Option<ImageLoadedData>) {
        debug_assert!(self.document_id == msg.document_id || self.documents.contains(&msg.document_id));

        if self.document_id != msg.document_id {
            todo!("document rendering is not implemented in WR");
        }

        let (frame_id, capture) = self.pending_frames.pop_front().unwrap_or((self.rendered_frame_id, false));
        self.rendered_frame_id = frame_id;

        let mut captured_data = None;

        if msg.composite_needed || capture {
            self.context.make_current();
            let renderer = self.renderer.as_mut().unwrap();

            if msg.composite_needed {
                renderer.update();
                renderer.render((self.size.to_px(self.scale_factor)).to_wr_device(), 0).unwrap();
                let _ = renderer.flush_pipeline_info();

                self.context.upload_swgl();
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

    pub fn frame_image<S: AppEventSender>(&mut self, images: &mut ImageCache<S>) -> ImageId {
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            PxRect::from_size(self.size.to_px(self.scale_factor)),
            true,
            self.id,
            self.rendered_frame_id,
            self.scale_factor,
        )
    }

    pub fn frame_image_rect<S: AppEventSender>(&mut self, images: &mut ImageCache<S>, rect: PxRect) -> ImageId {
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

    pub fn hit_test(&mut self, point: DipPoint) -> (FrameId, HitTestResult) {
        let _p = tracing::trace_span!("hit_test").entered();
        (
            self.rendered_frame_id,
            self.api.hit_test(self.document_id(), point.to_px(self.scale_factor).to_wr_world()),
        )
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        self.context.make_current();

        self.renderer.take().unwrap().deinit();

        let gl = self.context.gl();
        gl.delete_framebuffers(&[self.fbo]);
        gl.delete_renderbuffers(&self.rbos);
    }
}

fn resize(gl: &dyn gl::Gl, rbos: [u32; 2], size: DipSize, scale_factor: f32) {
    let size = size.to_px(scale_factor);
    let width = size.width.0;
    let height = size.height.0;

    gl.bind_renderbuffer(gl::RENDERBUFFER, rbos[0]);
    gl.renderbuffer_storage(gl::RENDERBUFFER, gl::RGBA8, width, height);

    gl.bind_renderbuffer(gl::RENDERBUFFER, rbos[1]);
    gl.renderbuffer_storage(gl::RENDERBUFFER, gl::DEPTH24_STENCIL8, width, height);

    gl.viewport(0, 0, width, height);
}

struct Notifier<S> {
    id: WindowId,
    sender: S,
}
impl<S: AppEventSender> RenderNotifier for Notifier<S> {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Self {
            id: self.id,
            sender: self.sender.clone(),
        })
    }

    fn wake_up(&self, _: bool) {}

    fn new_frame_ready(&self, document_id: DocumentId, _scrolled: bool, composite_needed: bool, _render_time_ns: Option<u64>) {
        let msg = FrameReadyMsg {
            document_id,
            composite_needed,
            // scrolled,
        };
        let _ = self.sender.send(AppEvent::FrameReady(self.id, msg));
    }
}

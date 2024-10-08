//! Demo view extension custom renderer, integrated by drawing to a texture uses as an image.

/// App-process stuff, nodes.
pub mod app_side {
    use zng::prelude::UiNode;
    use zng_app::view_process::VIEW_PROCESS;
    use zng_view_api::api_extension::ApiExtensionId;

    /// Node that sends external display item and updates.
    pub fn custom_render_node() -> impl UiNode {
        crate::using_display_items::app_side::custom_ext_node(extension_id)
    }

    pub fn extension_id() -> ApiExtensionId {
        VIEW_PROCESS
            .extension_id(super::api::extension_name())
            .ok()
            .flatten()
            .unwrap_or(ApiExtensionId::INVALID)
    }
}

/// View-process stuff, the actual extension.
pub mod view_side {
    use zng::layout::PxRect;
    use zng_view::{
        extensions::{PxToWr as _, RenderItemArgs, RendererExtension},
        gleam::gl,
        webrender::api::{
            units::{DeviceIntSize, TexelRect},
            AlphaType, ColorF, CommonItemProperties, ExternalImageData, ExternalImageId, ExternalImageType, ImageDescriptor,
            ImageDescriptorFlags, ImageFormat, ImageKey, ImageRendering,
        },
    };
    use zng_view_api::api_extension::ApiExtensionId;

    zng_view::view_process_extension!(|exts| {
        exts.renderer(super::api::extension_name(), CustomExtension::new);
    });

    struct TextureInfo {
        // texture in OpenGL.
        texture: gl::GLuint,
        // texture in external image registry (for webrender).
        external_id: ExternalImageId,
        // texture in renderer (and display lists).
        image_key: ImageKey,
    }

    struct CustomExtension {
        // id of this extension, for tracing.
        _id: ApiExtensionId,

        texture: Option<TextureInfo>,
    }
    impl CustomExtension {
        fn new(id: ApiExtensionId) -> Self {
            Self { _id: id, texture: None }
        }
    }
    impl RendererExtension for CustomExtension {
        fn is_init_only(&self) -> bool {
            false // retain the extension after renderer creation.
        }

        fn renderer_inited(&mut self, args: &mut zng_view::extensions::RendererInitedArgs) {
            // gl available here and in `redraw`.
            //
            // dynamic textures can be generated by collecting request on `command` or on `render_push` and
            // generating on the next `redraw` that will happen after `render_push` or on request after `command`.

            let size = DeviceIntSize::splat(100);

            // OpenGL
            let texture = args.context.gl().gen_textures(1)[0];
            args.context.gl().bind_texture(gl::TEXTURE_2D, texture);
            let mut img = vec![0u8; size.width as usize * size.height as usize * 4];
            let mut line = 0u8;
            let mut col = 0u8;
            for rgba in img.chunks_exact_mut(4) {
                rgba[0] = 255;
                rgba[1] = 10 + line * 3;
                rgba[2] = 10 + line * 3;
                rgba[3] = 255;

                col = col.wrapping_add(1);
                if col == 0 {
                    line = line.wrapping_add(1);
                }
            }
            args.context.gl().tex_image_2d(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as _,
                size.width,
                size.height,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                Some(&img),
            );

            // Webrender
            let external_id = args
                .external_images
                .register_texture(TexelRect::new(0.0, 0.0, size.width as f32, size.height as f32), texture);

            let image_key = args.api.generate_image_key();
            let mut txn = zng_view::webrender::Transaction::new();
            txn.add_image(
                image_key,
                ImageDescriptor {
                    format: ImageFormat::RGBA8,
                    size,
                    stride: None,
                    offset: 0,
                    flags: ImageDescriptorFlags::IS_OPAQUE,
                },
                zng_view::webrender::api::ImageData::External(ExternalImageData {
                    id: external_id,
                    channel_index: 0,
                    image_type: ExternalImageType::TextureHandle(zng_view::webrender::api::ImageBufferKind::Texture2D),
                    normalized_uvs: false,
                }),
                None,
            );
            args.api.send_transaction(args.document_id, txn);

            self.texture = Some(TextureInfo {
                texture,
                external_id,
                image_key,
            });
        }

        fn renderer_deinited(&mut self, args: &mut zng_view::extensions::RendererDeinitedArgs) {
            if let Some(t) = self.texture.take() {
                let _ = t.external_id; // already cleanup by renderer deinit.
                args.context.gl().delete_textures(&[t.texture]);
            }
        }

        fn render_push(&mut self, args: &mut RenderItemArgs) {
            match args.payload.deserialize::<super::api::RenderPayload>() {
                Ok(p) => {
                    if let Some(t) = &self.texture {
                        let rect = PxRect::from_size(p.size).to_wr();
                        let props = CommonItemProperties {
                            clip_rect: rect,
                            clip_chain_id: args.sc.clip_chain_id(args.list),
                            spatial_id: args.sc.spatial_id(),
                            flags: args.sc.primitive_flags(),
                        };
                        args.list
                            .push_image(&props, rect, ImageRendering::Auto, AlphaType::Alpha, t.image_key, ColorF::WHITE);
                    }
                }
                Err(e) => tracing::error!("invalid display item, {e}"),
            }
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
}

pub mod api {
    use zng_view_api::api_extension::ApiExtensionName;

    pub use crate::using_display_items::api::*;

    pub fn extension_name() -> ApiExtensionName {
        ApiExtensionName::new("zng.examples.extend_renderer.using_gl_texture").unwrap()
    }
}

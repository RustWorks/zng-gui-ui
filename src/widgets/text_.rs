use crate::core::context::*;
use crate::core::font::*;
use crate::core::render::FrameBuilder;
use crate::core::types::Text;
use crate::core::types::*;
use crate::core::var::{IntoVar, ObjVar, Var};
use crate::core::UiNode;
use crate::impl_ui_node;
use std::{borrow::Cow, fmt, rc::Rc};

struct TextRun<T: Var<Text>> {
    text: T,

    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font: Option<FontInstance>,
    color: ColorF,
}

#[impl_ui_node(none)]
impl<T: Var<Text>> UiNode for TextRun<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        profile_scope!("text::init");

        self.color = *TextColor::var().get(ctx.vars);
        let font_size = *FontSize::var().get(ctx.vars);

        let font_family = FontFamily::var();
        let font_family = font_family.get(ctx.vars);
        let font = ctx.window_services.req::<Fonts>().get(font_family, font_size);

        let font_size = font_size as f32;

        let text = self.text.get(ctx.vars);
        let text = TextTransform::var().get(ctx.vars).transform(text);

        let (indices, dimensions) = font.glyph_layout(&text);
        let mut glyphs = Vec::with_capacity(indices.len());
        let mut offset = 0.;
        assert_eq!(indices.len(), dimensions.len());
        for (index, dimension) in indices.into_iter().zip(dimensions) {
            if let Some(dimension) = dimension {
                glyphs.push(GlyphInstance {
                    index,
                    point: LayoutPoint::new(offset, font_size),
                });
                offset += dimension.advance as f32;
            } else {
                offset += font_size / 4.;
            }
        }
        glyphs.shrink_to_fit();

        self.glyphs = glyphs;
        self.size = LayoutSize::new(offset, font_size * 1.3);
        self.font = Some(font);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        profile_scope!("text::update");

        if self.text.is_new(ctx.vars)
            || FontFamily::var().is_new(ctx.vars)
            || FontSize::var().is_new(ctx.vars)
            || TextTransform::var().is_new(ctx.vars)
        {
            self.init(ctx);
            ctx.updates.push_layout();
        }

        if let Some(&color) = TextColor::var().update(ctx.vars) {
            self.color = color;
            ctx.updates.push_render();
        }
    }

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.size
    }

    fn render(&self, frame: &mut FrameBuilder) {
        profile_scope!("text::render");

        frame.push_text(
            LayoutRect::from_size(self.size),
            &self.glyphs,
            self.font.as_ref().unwrap().instance_key(),
            self.color,
            None,
        )
    }
}

/// Simple text run.
///
/// # Configure
///
/// Text spans can be configured by setting [`font_family`](crate::properties::font_family),
/// [`font_size`](crate::properties::font_size) or [`text_color`](crate::properties::text_color)
/// in parent widgets.
///
/// # Example
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() -> () {
/// use zero_ui::widgets::{container, text};
/// use zero_ui::properties::{font_family, font_size};
///
/// let hello_txt = container! {
///     font_family: "Arial";
///     font_size: 18;
///     => text("Hello!")
/// };
/// # }
/// ```
pub fn text(text: impl IntoVar<Text>) -> impl UiNode {
    TextRun {
        text: text.into_var(),

        glyphs: vec![],
        size: LayoutSize::default(),
        font: None,
        color: ColorF::BLACK,
    }
}

context_var! {
    /// Font family of [`text`](crate::widgets::text) spans.
    pub struct FontFamily: Text = const Cow::Borrowed("Sans-Serif");

    /// Font size of [`text`](crate::widgets::text) spans.
    pub struct FontSize: u32 = const 14;

    /// Text color of [`text`](crate::widgets::text) spans.
    pub struct TextColor: ColorF = const ColorF::WHITE;

    pub struct TextTransform: TextTransformFn = return &TextTransformFn::None;
}

#[derive(Clone)]
pub enum TextTransformFn {
    None,
    Uppercase,
    Lowercase,
    Custom(Rc<dyn Fn(&str) -> Cow<str>>),
}

impl TextTransformFn {
    pub fn transform<'a, 'b>(&'a self, text: &'b str) -> Cow<'b, str> {
        match self {
            TextTransformFn::None => Cow::Borrowed(text),
            TextTransformFn::Uppercase => Cow::Owned(text.to_uppercase()),
            TextTransformFn::Lowercase => Cow::Owned(text.to_lowercase()),
            TextTransformFn::Custom(fn_) => fn_(text),
        }
    }

    pub fn custom(fn_: impl Fn(&str) -> Cow<str> + 'static) -> Self {
        TextTransformFn::Custom(Rc::new(fn_))
    }
}

impl fmt::Debug for TextTransformFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TextTransformFn::None => write!(f, "None"),
            TextTransformFn::Uppercase => write!(f, "Uppercase"),
            TextTransformFn::Lowercase => write!(f, "Lowercase"),
            TextTransformFn::Custom(_) => write!(f, "Custom"),
        }
    }
}

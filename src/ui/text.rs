use super::{
    ContextValue, ContextValueKey, HitTag, Hits, LayoutSize, NextFrame, NextUpdate, ReadValue, SetContextValue, Static,
    Ui, UiLeaf, UiValues,
};
use webrender::api::*;

pub struct Text {
    text: String,
    color: ColorF,
    hit_tag: HitTag,

    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font_instance_key: Option<FontInstanceKey>,
}

impl Text {
    pub fn new(text: &str, color: ColorF) -> Self {
        //https://harfbuzz.github.io/
        //https://crates.io/crates/unicode-bidi
        //https://www.geeksforgeeks.org/word-wrap-problem-dp-19/

        Text {
            text: text.to_owned(),
            glyphs: vec![],
            size: LayoutSize::default(),
            font_instance_key: None,
            color,
            hit_tag: HitTag::new(),
        }
    }

    fn update(&mut self, v: &mut UiValues, u: &mut NextUpdate) {
        if let (Some(font_family), Some(font_size)) = (v.get(*FONT_FAMILY), v.get(*FONT_SIZE)) {
            let font = u.font(&font_family, *font_size);

            let indices: Vec<_> = u
                .api
                .get_glyph_indices(font.font_key, &self.text)
                .into_iter()
                .filter_map(|i| i)
                .collect();
            let dimensions = u.api.get_glyph_dimensions(font.instance_key, indices.clone());

            let mut glyphs = Vec::with_capacity(indices.len());
            let mut offset = 0.;

            assert_eq!(indices.len(), dimensions.len());

            for (index, dim) in indices.into_iter().zip(dimensions) {
                if let Some(dim) = dim {
                    glyphs.push(GlyphInstance {
                        index,
                        point: LayoutPoint::new(offset, font.size as f32),
                    });

                    offset += dim.advance as f32;
                } else {
                    offset += font.size as f32 / 4.;
                }
            }
            glyphs.shrink_to_fit();

            self.glyphs = glyphs;
            self.size = LayoutSize::new(offset, font.size as f32 * 1.3);
            self.font_instance_key = Some(font.instance_key);
        } else {
            self.glyphs = vec![];
            self.size = LayoutSize::default();
            self.font_instance_key = None;
        }
    }
}

pub fn text(text: &str, color: ColorF) -> Text {
    Text::new(text, color)
}

impl UiLeaf for Text {
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.update(values, update);
    }

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.size
    }

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    fn context_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.update(values, update);
    }

    fn render(&self, f: &mut NextFrame) {
        if let Some(font) = self.font_instance_key {
            f.push_text(
                LayoutRect::from_size(self.size),
                &self.glyphs,
                font,
                self.color,
                Some(self.hit_tag),
            )
        }
    }
}
delegate_ui!(UiLeaf, Text);

lazy_static! {
    pub static ref FONT_FAMILY: ContextValueKey<String> = ContextValueKey::new();
    pub static ref FONT_SIZE: ContextValueKey<u32> = ContextValueKey::new();
}

pub type SetFontFamily<T, R> = SetContextValue<T, String, R>;
pub type SetFontSize<T, R> = SetContextValue<T, u32, R>;

pub trait Font: Ui + Sized {
    fn font_family(self, font: impl ToString) -> SetFontFamily<Self, Static<String>> {
        self.set_ctx_val(*FONT_FAMILY, Static(font.to_string()))
    }
    fn font_size(self, size: u32) -> SetFontSize<Self, Static<u32>> {
        self.set_ctx_val(*FONT_SIZE, Static(size))
    }

    fn font_family_dyn<F: ReadValue<String> + Clone + 'static>(self, font: F) -> SetFontFamily<Self, F> {
        self.set_ctx_val(*FONT_FAMILY, font)
    }
    fn font_size_dyn<S: ReadValue<u32> + Clone + 'static>(self, size: S) -> SetFontSize<Self, S> {
        self.set_ctx_val(*FONT_SIZE, size)
    }
}
impl<T: Ui> Font for T {}

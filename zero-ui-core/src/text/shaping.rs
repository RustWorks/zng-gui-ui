use std::{
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    mem,
};

use super::{
    font_features::RFontFeatures, lang, Font, FontList, FontRef, GlyphIndex, GlyphInstance, InternedStr, Lang, SegmentedText, TextSegment,
    TextSegmentKind,
};
use crate::{
    crate_util::{f32_cmp, IndexRange},
    units::*,
};

pub use font_kit::error::GlyphLoadingError;

/// Extra configuration for [`shape_text`](Font::shape_text).
#[derive(Debug, Clone)]
pub struct TextShapingArgs {
    /// Extra spacing to add after each character.
    pub letter_spacing: Px,

    /// Extra spacing to add after each space (U+0020 SPACE).
    pub word_spacing: Px,

    /// Height of each line.
    ///
    /// Default can be computed using [`FontMetrics::line_height`].
    ///
    /// [`FontMetrics::line_height`]: crate::text::FontMetrics::line_height
    pub line_height: Px,

    /// Extra spacing added in between lines.
    pub line_spacing: Px,

    /// Language of the text, also identifies if RTL.
    pub lang: Lang,

    /// Don't use font ligatures.
    pub ignore_ligatures: bool,

    /// Don't use font letter spacing.
    pub disable_kerning: bool,

    /// Width of the TAB character.
    pub tab_x_advance: Px,

    /// Extra space before the start of the first line.
    pub text_indent: Px,

    /// Finalized font features.
    pub font_features: RFontFeatures,
}
impl Default for TextShapingArgs {
    fn default() -> Self {
        TextShapingArgs {
            letter_spacing: Px(0),
            word_spacing: Px(0),
            line_height: Px(0),
            line_spacing: Px(0),
            lang: lang!(und),
            ignore_ligatures: false,
            disable_kerning: false,
            tab_x_advance: Px(0),
            text_indent: Px(0),
            font_features: RFontFeatures::default(),
        }
    }
}

/// Defines a range of segments in a [`ShapedText`] that form a line.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LineRange {
    /// Exclusive segment index, is the `segments.len()` for the last line and the index of the first
    /// segment after the line break for other lines.
    end: usize,
    /// Pixel width of the line.
    width: f32,
}

/// Defines the font of a range of glyphs in a [`ShapedText`].
#[derive(Clone)]
struct FontRange {
    font: FontRef,
    /// Exclusive glyph range end.
    end: usize,
}
impl PartialEq for FontRange {
    fn eq(&self, other: &Self) -> bool {
        self.font.ptr_eq(&other.font) && self.end == other.end
    }
}
impl fmt::Debug for FontRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontInfo")
            .field("font", &self.font.face().display_name().name())
            .field("end", &self.end)
            .finish()
    }
}

/// `Vec<TextSegment>` with helper methods.
#[derive(Debug, Default, Clone, PartialEq)]
struct TextSegmentVec(Vec<TextSegment>);
impl TextSegmentVec {
    /// Exclusive glyphs range of the segment.
    fn glyphs(&self, index: usize) -> IndexRange {
        let start = if index == 0 { 0 } else { self.0[index - 1].end };
        let end = self.0[index].end;
        IndexRange(start, end)
    }

    /// Exclusive glyphs range from an exclusive range of segments.
    fn glyphs_range(&self, range: IndexRange) -> IndexRange {
        let IndexRange(start, end) = range;

        let start = if start == 0 { 0 } else { self.0[start - 1].end };
        let end = self.0[end - 1].end;

        IndexRange(start, end)
    }

    /// Iter glyph ranges.
    fn iter_glyphs(&self) -> impl Iterator<Item = (TextSegmentKind, IndexRange)> + '_ {
        let mut start = 0;
        self.0.iter().map(move |s| {
            let r = IndexRange(start, s.end);
            start = s.end;
            (s.kind, r)
        })
    }

    fn first(&self) -> TextSegment {
        self.0[0]
    }

    fn last(&self) -> TextSegment {
        let l = self.0.len() - 1;
        self.0[l]
    }

    fn last_glyphs(&self) -> IndexRange {
        self.glyphs(self.0.len() - 1)
    }

    fn assert_contains(&self, index: usize) {
        if self.0.len() <= index {
            panic!("segment out of bounds, the len is {} but the segment is {}", self.0.len(), index)
        }
    }
}

/// `Vec<LineRange>` with helper methods.
#[derive(Debug, Default, Clone, PartialEq)]
struct LineRangeVec(Vec<LineRange>);
impl LineRangeVec {
    /// Exclusive segments range of the line.
    fn segs(&self, index: usize) -> IndexRange {
        let end = self.0[index].end;
        let start = if index == 0 { 0 } else { self.0[index - 1].end };
        IndexRange(start, end)
    }

    /// Line width.
    fn width(&self, index: usize) -> f32 {
        self.0[index].width
    }

    /// Iter segment ranges.
    fn iter_segs(&self) -> impl Iterator<Item = (f32, IndexRange)> + '_ {
        self.iter_segs_skip(0)
    }

    /// Iter segment ranges starting at a line.
    fn iter_segs_skip(&self, start_line: usize) -> impl Iterator<Item = (f32, IndexRange)> + '_ {
        let mut start = self.segs(start_line).start();
        self.0[start_line..].iter().map(move |l| {
            let r = IndexRange(start, l.end);
            start = l.end;
            (l.width, r)
        })
    }

    /// Returns `true` if there is more then one line.
    fn is_multi(&self) -> bool {
        self.0.len() > 1
    }

    fn first_mut(&mut self) -> &mut LineRange {
        &mut self.0[0]
    }

    fn last(&self) -> LineRange {
        self.0[self.0.len() - 1]
    }

    fn last_mut(&mut self) -> &mut LineRange {
        let l = self.0.len() - 1;
        &mut self.0[l]
    }

    fn max_width(&self) -> f32 {
        self.0.iter().map(|l| l.width).max_by(f32_cmp).unwrap()
    }
}

/// `Vec<FontRange>` with helper methods.
#[derive(Debug, Default, Clone, PartialEq)]
struct FontRangeVec(Vec<FontRange>);
impl FontRangeVec {
    /// Exclusive glyphs range of the segment.
    fn glyphs(&self, index: usize) -> IndexRange {
        let start = if index == 0 { 0 } else { self.0[index - 1].end };
        let end = self.0[index].end;
        IndexRange(start, end)
    }

    /// Iter glyph ranges.
    fn iter_glyphs(&self) -> impl Iterator<Item = (&FontRef, IndexRange)> + '_ {
        let mut start = 0;
        self.0.iter().map(move |f| {
            let r = IndexRange(start, f.end);
            start = f.end;
            (&f.font, r)
        })
    }

    /// Iter glyph ranges clipped by `glyphs_range`.
    fn iter_glyphs_clip(&self, glyphs_range: IndexRange) -> impl Iterator<Item = (&FontRef, IndexRange)> + '_ {
        let mut start = glyphs_range.start();
        let end = glyphs_range.end();
        let first_font = self.0.iter().position(|f| f.end > start).unwrap().saturating_sub(1);

        self.0[first_font..].iter().map_while(move |f| {
            let i = f.end.min(end);

            if i > start {
                let r = IndexRange(start, i);
                start = i;
                Some((&f.font, r))
            } else {
                None
            }
        })
    }

    /// Returns a reference to the font.
    fn font(&self, index: usize) -> &FontRef {
        &self.0[index].font
    }

    fn last_glyphs(&self) -> IndexRange {
        let last = self.0.len() - 1;
        self.glyphs(last)
    }

    fn first(&self) -> &FontRange {
        &self.0[0]
    }

    fn last(&self) -> &FontRange {
        &self.0[self.0.len() - 1]
    }

    fn last_mut(&mut self) -> &mut FontRange {
        let last = self.0.len() - 1;
        &mut self.0[last]
    }
}

/// Output of [text layout].
///
/// [text layout]: Font::shape_text
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText {
    glyphs: Vec<GlyphInstance>,
    /// segments of `glyphs`
    segments: TextSegmentVec,
    lines: LineRangeVec,
    fonts: FontRangeVec,

    padding: PxSideOffsets,
    size: PxSize,
    line_height: Px,
    line_spacing: Px,

    // offsets from the line_height bottom
    baseline: Px,
    overline: Px,
    strikethrough: Px,
    underline: Px,
    underline_descent: Px,
}
impl ShapedText {
    /// Alloc for shaping, value is invalid, see `Font::shape_text`.
    fn alloc() -> Self {
        Self {
            glyphs: Default::default(),
            segments: Default::default(),
            lines: Default::default(),
            fonts: Default::default(),
            padding: Default::default(),
            size: Default::default(),
            line_height: Default::default(),
            line_spacing: Default::default(),
            baseline: Default::default(),
            overline: Default::default(),
            strikethrough: Default::default(),
            underline: Default::default(),
            underline_descent: Default::default(),
        }
    }

    /// New empty text.
    pub fn new(font: &FontRef) -> Self {
        font.shape_text(&SegmentedText::new(""), &TextShapingArgs::default())
    }

    /// Glyphs by font.
    pub fn glyphs(&self) -> impl Iterator<Item = (&FontRef, &[GlyphInstance])> {
        self.fonts.iter_glyphs().map(move |(f, r)| (f, &self.glyphs[r.iter()]))
    }

    /// Glyphs by font in the range.
    fn glyphs_range(&self, range: IndexRange) -> impl Iterator<Item = (&FontRef, &[GlyphInstance])> {
        self.fonts.iter_glyphs_clip(range).map(|(f, r)| (f, &self.glyphs[r.iter()]))
    }

    /// Glyphs by font in the range, each glyph instance is paired with the *x-advance* to the next glyph or line end.
    fn glyphs_with_x_advance_range(
        &self,
        line_index: usize,
        glyph_range: IndexRange,
    ) -> impl Iterator<Item = (&FontRef, impl Iterator<Item = (GlyphInstance, f32)> + '_)> + '_ {
        let mut start = glyph_range.start();
        let segs_range = self.lines.segs(line_index);
        let line_end = self.segments.glyphs_range(segs_range).end();
        let line_x = self.padding.left;
        let line_width = self.lines.width(line_index);
        self.glyphs_range(glyph_range).map(move |(font, glyphs)| {
            let glyphs_with_adv = glyphs.iter().enumerate().map(move |(i, g)| {
                let gi = start + i + 1;

                let adv = if gi == line_end {
                    line_x.0 as f32 + line_width - g.point.x
                } else {
                    self.glyphs[gi].point.x - g.point.x
                };

                (*g, adv)
            });

            start += glyphs.len();

            (font, glyphs_with_adv)
        })
    }

    /// Glyphs segments.

    pub fn segments(&self) -> &[TextSegment] {
        &self.segments.0
    }

    /// Bounding box size, the width is the longest line, the height is the sum of line heights + spacing in between,
    /// no spacing is added before the first line and after the last line.

    pub fn size(&self) -> PxSize {
        self.size
    }

    /// Current applied offsets around the text block.
    ///
    /// Note this padding is already computed in all other values.

    pub fn padding(&self) -> PxSideOffsets {
        self.padding
    }

    /// Reshape text to have the new `padding`.
    ///
    /// The padding
    pub fn set_padding(&mut self, padding: PxSideOffsets) {
        if self.padding == padding {
            return;
        }

        let p = padding + self.padding * Px(-1); // no Sub impl

        let offset = PxVector::new(p.left, p.top);
        let offset_f32 = euclid::vec2(offset.x.0 as f32, offset.y.0 as f32);
        for g in &mut self.glyphs {
            g.point += offset_f32;
        }

        self.size.width += p.horizontal();
        self.size.height += p.vertical();
        self.padding = padding;
    }

    /// Height of a single line.

    pub fn line_height(&self) -> Px {
        self.line_height
    }

    /// Set the line height, reposition glyphs if needed.
    pub fn set_line_height(&mut self, line_height: Px) {
        let diff = line_height - self.line_height;
        if diff == Px(0) {
            return;
        }

        if !self.is_empty() {
            let line_diff = diff.0 as f32;
            let mut diff = 0.0;
            let center = line_diff / 2.0;

            for (_, r) in self.lines.iter_segs() {
                let r = self.segments.glyphs_range(r);
                for g in &mut self.glyphs[r.iter()] {
                    g.point.y += diff + center;
                }

                diff += line_diff;
            }
        }

        self.line_height = line_height;
        self.update_height();
    }
    fn update_height(&mut self) {
        let lines = Px(self.lines.0.len() as i32);
        self.size.height = self.line_height * lines + self.line_spacing * (lines - Px(1));
    }

    /// Vertical spacing in between lines.

    pub fn line_spacing(&self) -> Px {
        self.line_spacing
    }

    /// Set the line spacing, reposition glyphs if needed.
    pub fn set_line_spacing(&mut self, line_spacing: Px) {
        let diff = line_spacing - self.line_spacing;
        if diff == Px(0) {
            return;
        }

        if self.lines.is_multi() {
            let mut diff = diff.0 as f32;
            let line_diff = diff;

            for (_, r) in self.lines.iter_segs_skip(1) {
                let r = self.segments.glyphs_range(r);

                for g in &mut self.glyphs[r.iter()] {
                    g.point.y += diff;
                }

                diff += line_diff;
            }
        }

        self.line_spacing = line_spacing;
        self.update_height();
    }

    /// Vertical offset from the line bottom up that is the text baseline.
    ///
    /// The *line bottom* is the [`line_height`].
    ///
    /// [`line_height`]: Self::line_height

    pub fn baseline(&self) -> Px {
        self.baseline
    }

    /// Vertical offset from the bottom up that is the baseline of the last line considering the padding.
    ///
    /// The *bottom* is the [`size`] height.
    ///
    /// [`size`]: Self::size

    pub fn box_baseline(&self) -> Px {
        self.baseline + self.padding.bottom
    }

    /// Vertical offset from the line bottom up that is the overline placement.

    pub fn overline(&self) -> Px {
        self.overline
    }

    /// Vertical offset from the line bottom up that is the strikethrough placement.

    pub fn strikethrough(&self) -> Px {
        self.strikethrough
    }

    /// Vertical offset from the line bottom up that is the font defined underline placement.

    pub fn underline(&self) -> Px {
        self.underline
    }

    /// Vertical offset from the line bottom up that is the underline placement when the option for
    /// clearing all glyph descents is selected.

    pub fn underline_descent(&self) -> Px {
        self.underline_descent
    }

    /// No segments.

    pub fn is_empty(&self) -> bool {
        self.segments.0.is_empty()
    }

    /// Iterate over [`ShapedLine`] selections split by [`LineBreak`].
    ///
    /// [`LineBreak`]: TextSegmentKind::LineBreak

    pub fn lines(&self) -> impl Iterator<Item = ShapedLine> {
        self.lines.iter_segs().enumerate().map(move |(i, (w, r))| ShapedLine {
            text: self,
            seg_range: r,
            index: i,
            width: Px(w.round() as i32),
        })
    }

    /// Create an empty [`ShapedText`] with the same metrics as `self`.
    pub fn empty(&self) -> ShapedText {
        ShapedText {
            glyphs: vec![],
            segments: TextSegmentVec(vec![]),
            lines: LineRangeVec(vec![LineRange { end: 0, width: 0.0 }]),
            fonts: FontRangeVec(vec![FontRange {
                font: self.fonts.font(0).clone(),
                end: 0,
            }]),
            padding: PxSideOffsets::zero(),
            size: PxSize::zero(),
            line_height: self.line_height,
            line_spacing: self.line_spacing,
            baseline: self.baseline,
            overline: self.overline,
            strikethrough: self.strikethrough,
            underline: self.underline,
            underline_descent: self.underline_descent,
        }
    }

    /// Split the shaped text in two. The text is split at a `segment` that becomes the first segment of the second text.
    ///
    /// Padding is not included in the result, any padding set is removed before split.
    ///
    /// # Panics
    ///
    /// Panics if `segment` is out of the range.
    pub fn split(mut self, segment: usize) -> (ShapedText, ShapedText) {
        self.set_padding(self.padding * -Px(1));

        if segment == 0 {
            let a = self.empty();
            (a, self)
        } else {
            self.segments.assert_contains(segment);

            let g_end = self.segments.glyphs(segment).start();
            let l_end = self.lines.0.iter().position(|l| l.end > segment).unwrap();
            let f_end = self.fonts.0.iter().position(|f| f.end > g_end).unwrap();

            let mut b = ShapedText {
                glyphs: self.glyphs.drain(g_end..).collect(),
                segments: TextSegmentVec(self.segments.0.drain(segment..).collect()),
                lines: LineRangeVec(self.lines.0.drain(l_end..).collect()),
                fonts: FontRangeVec(self.fonts.0.drain(f_end..).collect()),
                padding: PxSideOffsets::zero(),
                size: PxSize::zero(),
                line_height: self.line_height,
                line_spacing: self.line_spacing,
                baseline: self.baseline,
                overline: self.overline,
                strikethrough: self.strikethrough,
                underline: self.underline,
                underline_descent: self.underline_descent,
            };

            if self.lines.0.is_empty() || self.lines.last().end <= self.segments.0.len() {
                self.lines.0.push(LineRange {
                    end: self.segments.0.len(),
                    width: 0.0,
                });
            }

            let LineRange { width: a_ll_width, .. } = self.lines.last_mut();
            let LineRange { width: b_fl_width, .. } = b.lines.first_mut();

            let mut x_offset = 0.0;

            if self.segments.last().kind == TextSegmentKind::LineBreak {
                *a_ll_width = 0.0;
            } else if b.segments.first().kind == TextSegmentKind::LineBreak {
                *a_ll_width = *b_fl_width;
            } else {
                x_offset = b.glyphs[0].point.x;
                *a_ll_width = x_offset;
            }

            *b_fl_width -= *a_ll_width;

            for l in &mut b.lines.0 {
                l.end -= self.segments.0.len();
            }

            for s in &mut b.segments.0 {
                s.end -= self.glyphs.len();
            }

            if self.fonts.0.is_empty() {
                self.fonts.0.push(FontRange {
                    font: b.fonts.font(0).clone(),
                    end: self.glyphs.len(),
                });
            } else {
                let last_font = self.fonts.last_mut();
                last_font.end = last_font.end.min(self.glyphs.len());
            }
            for f in &mut b.fonts.0 {
                f.end -= self.glyphs.len();
            }

            let b_fl_end = b.segments.glyphs(b.lines.segs(0).inclusive_end()).end();
            for g in &mut b.glyphs[..b_fl_end] {
                g.point.x -= x_offset;
            }

            self.size.width = Px(self.lines.max_width().round() as i32);
            self.update_height();

            b.size.width = Px(b.lines.max_width().round() as i32);
            b.update_height();

            if self.lines.is_multi() {
                let b_y_offset = (self.size.height - self.line_height - self.line_spacing).0 as f32;
                for g in &mut b.glyphs {
                    g.point.y -= b_y_offset;
                }
            }

            (self, b)
        }
    }

    /// Like [`split`] but the `segment` is not included in the result.
    ///
    /// [`split`]: Self::split
    pub fn split_remove(self, segment: usize) -> (ShapedText, ShapedText) {
        // split to include the segment to remove in the first text, then pop the segment.

        self.segments.assert_contains(segment);

        let aft_rmv = segment + 1;
        let (mut a, b) = if aft_rmv == self.segments.0.len() {
            let b = self.empty();
            (self, b)
        } else {
            self.split(aft_rmv)
        };

        a.pop_seg();

        (a, b)
    }
    fn pop_seg(&mut self) {
        if self.is_empty() {
            return;
        }

        if self.segments.0.len() == 1 {
            let padding = self.padding();
            *self = self.empty();
            self.set_padding(padding);
        } else {
            if self.segments.last().kind == TextSegmentKind::LineBreak {
                self.lines.0.pop();
            }

            let r = self.segments.last_glyphs();
            self.segments.0.pop();

            while self.fonts.last_glyphs().start() >= r.start() {
                self.fonts.0.pop();
            }
            self.fonts.last_mut().end = r.start();

            let last_line = self.lines.last_mut();
            last_line.end = self.segments.0.len();
            if r.start() < self.glyphs.len() {
                last_line.width = self.glyphs[r.start()].point.x;
                last_line.end = self.segments.0.len();
                self.glyphs.truncate(r.start());
            }

            self.size.width = Px(self.lines.max_width().round() as i32);
        }
    }

    /// Appends the `text` to the end of `self`.
    ///
    /// Line height and spacing of `self` is applied to `text`, aligning by baseline.
    ///
    /// If `text` has padding it is removed before pushing, if `self` has padding it is used.
    pub fn extend(&mut self, mut text: ShapedText) {
        if text.is_empty() {
            return;
        }

        text.set_padding(text.padding * -Px(1));
        text.set_line_height(self.line_height);
        text.set_line_spacing(self.line_spacing);

        if self.is_empty() {
            text.set_padding(self.padding);
            *self = text;
            return;
        }

        // y-offset of glyphs in `text`.
        let y_offset = (self.size.height - self.padding.bottom - self.line_height).0 as f32;
        for g in &mut text.glyphs {
            g.point.y += y_offset;
        }

        // x-offset of the glyphs in the first line of `text`.
        let x_offset = self.lines.last().width;
        let r = text.segments.glyphs_range(text.lines.segs(0));
        for g in &mut text.glyphs[r.iter()] {
            g.point.x += x_offset;
        }

        for line in &mut text.lines.0 {
            line.end += self.segments.0.len();
        }

        let mut lines = text.lines.0.into_iter();

        let last_line = self.lines.last_mut();
        let first_line = lines.next().unwrap();
        last_line.end = first_line.end;
        last_line.width += first_line.width;

        self.lines.0.extend(lines);

        for seg in &mut text.segments.0 {
            seg.end += self.glyphs.len();
        }
        self.segments.0.extend(text.segments.0);

        for font in &mut text.fonts.0 {
            font.end += self.glyphs.len();
        }
        let merge_font = self.fonts.last().font.ptr_eq(&text.fonts.first().font);
        let mut fonts = text.fonts.0.into_iter();
        if merge_font {
            self.fonts.last_mut().end = fonts.next().unwrap().end;
        }
        self.fonts.0.extend(fonts);

        self.glyphs.extend(text.glyphs);

        self.size.width = Px(self.lines.max_width().round() as i32);
        self.update_height();
    }
}

/// Represents a line selection of a [`ShapedText`].
#[derive(Clone, Copy)]
pub struct ShapedLine<'a> {
    text: &'a ShapedText,
    // range of segments of this line.
    seg_range: IndexRange,
    index: usize,
    width: Px,
}
impl<'a> fmt::Debug for ShapedLine<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapedLine")
            .field("seg_range", &self.seg_range)
            .field("index", &self.index)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}
impl<'a> ShapedLine<'a> {
    /// Bounds of the line.
    pub fn rect(&self) -> PxRect {
        let size = PxSize::new(self.width, self.text.line_height);
        let origin = PxPoint::new(Px(0), self.text.line_height * Px(self.index as i32));
        PxRect::new(origin, size)
    }

    /// Full overline, start point + width.

    pub fn overline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.overline)
    }

    /// Full strikethrough line, start point + width.

    pub fn strikethrough(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.strikethrough)
    }

    /// Full underline, not skipping.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns start point + width.

    pub fn underline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline)
    }

    /// Full underline, not skipping.
    ///
    /// The *y* is the baseline + descent + 1px.
    ///
    /// Returns start point + width.

    pub fn underline_descent(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline_descent)
    }

    /// Underline, skipping spaces.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns and iterator of start point + width for each word.

    pub fn underline_skip_spaces(&self) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.parts().filter(|s| s.is_word()).map(|s| s.underline()))
    }

    /// Underline, skipping spaces.
    ///
    /// The *y* is the baseline + descent + 1px.
    ///
    /// Returns and iterator of start point + width for each word.

    pub fn underline_descent_skip_spaces(&self) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.parts().filter(|s| s.is_word()).map(|s| s.underline_descent()))
    }

    /// Underline, skipping glyph descends that intersect the underline.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns an iterator of start point + width for continuous underline.

    pub fn underline_skip_glyphs(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.parts().flat_map(move |s| s.underline_skip_glyphs(thickness)))
    }

    /// Underline, skipping spaces and glyph descends that intersect the underline
    ///
    /// The *y* is defined by font metrics.
    ///
    /// Returns an iterator of start point + width for continuous underline.

    pub fn underline_skip_glyphs_and_spaces(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(
            self.parts()
                .filter(|s| s.is_word())
                .flat_map(move |s| s.underline_skip_glyphs(thickness)),
        )
    }

    fn decoration_line(&self, bottom_up_offset: Px) -> (PxPoint, Px) {
        let y = (self.text.line_height * Px((self.index as i32) + 1)) - bottom_up_offset;
        (PxPoint::new(Px(0), y + self.text.padding.top), self.width)
    }

    /// Text segments of the line, does not include the line-break that started the line, can include
    /// the line break that starts the next line.

    pub fn segments(&self) -> &'a [TextSegment] {
        &self.text.segments.0[self.seg_range.iter()]
    }

    /// Glyphs in the line.

    pub fn glyphs(&self) -> impl Iterator<Item = (&'a FontRef, &'a [GlyphInstance])> + 'a {
        let r = self.glyphs_range();
        self.text.glyphs_range(r)
    }

    /// Glyphs in the line paired with the *x-advance* to the next glyph or the end of the line.

    pub fn glyphs_with_x_advance(&self) -> impl Iterator<Item = (&'a FontRef, impl Iterator<Item = (GlyphInstance, f32)> + 'a)> + 'a {
        let r = self.glyphs_range();
        self.text.glyphs_with_x_advance_range(self.index, r)
    }

    fn glyphs_range(&self) -> IndexRange {
        self.text.segments.glyphs_range(self.seg_range)
    }

    /// Iterate over word and space segments in this line.

    pub fn parts(&self) -> impl Iterator<Item = ShapedSegment<'a>> {
        let text = self.text;
        let line_index = self.index;
        let last_i = self.seg_range.inclusive_end();
        self.seg_range.iter().map(move |i| ShapedSegment {
            text,
            line_index,
            index: i,
            is_last: i == last_i,
        })
    }
}

/// Merges lines defined by `(PxPoint, Px)`, assuming the `y` is equal.
struct MergingLineIter<I> {
    iter: I,
    line: Option<(PxPoint, Px)>,
}
impl<I> MergingLineIter<I> {
    pub fn new(iter: I) -> Self {
        MergingLineIter { iter, line: None }
    }
}
impl<I: Iterator<Item = (PxPoint, Px)>> Iterator for MergingLineIter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((point, width)) => {
                    if let Some((lp, lw)) = &mut self.line {
                        // merge line if touching or only skipping 1px, the lines are rounded to snap-to-pixels
                        // this can cause 1px errors.
                        let diff = point.x - (lp.x + *lw);
                        if diff <= Px(1) {
                            *lw += width + diff;
                            continue;
                        } else {
                            let r = (*lp, *lw);

                            *lp = point;
                            *lw = width;

                            return Some(r);
                        }
                    } else {
                        self.line = Some((point, width));
                        continue;
                    }
                }
                None => return self.line.take(),
            }
        }
    }
}

/// Represents a word or space selection of a [`ShapedText`].
#[derive(Clone, Copy)]
pub struct ShapedSegment<'a> {
    text: &'a ShapedText,
    line_index: usize,
    index: usize,
    is_last: bool,
}
impl<'a> fmt::Debug for ShapedSegment<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapedSegment")
            .field("line_index", &self.line_index)
            .field("index", &self.index)
            .field("is_last", &self.is_last)
            .finish_non_exhaustive()
    }
}
impl<'a> ShapedSegment<'a> {
    /// Segment kind.

    pub fn kind(&self) -> TextSegmentKind {
        self.text.segments.0[self.index].kind
    }

    /// If the segment kind is [`Word`].
    ///
    /// [`Word`]: TextSegmentKind::Word

    pub fn is_word(&self) -> bool {
        matches!(self.kind(), TextSegmentKind::Word)
    }

    /// If the segment kind is [`Space`] or [`Tab`].
    ///
    /// [`Space`]: TextSegmentKind::Space
    /// [`Tab`]: TextSegmentKind::Tab

    pub fn is_space(&self) -> bool {
        matches!(self.kind(), TextSegmentKind::Space | TextSegmentKind::Tab)
    }

    /// If this is the last segment of the line.

    pub fn is_last(&self) -> bool {
        self.is_last
    }

    fn glyph_range(&self) -> IndexRange {
        self.text.segments.glyphs(self.index)
    }

    /// Glyphs in the word or space.

    pub fn glyphs(&self) -> impl Iterator<Item = (&'a FontRef, &'a [GlyphInstance])> {
        let r = self.glyph_range();
        self.text.glyphs_range(r)
    }

    /// Glyphs in the word or space, paired with the *x-advance* to then next glyph or line end.

    pub fn glyphs_with_x_advance(&self) -> impl Iterator<Item = (&'a FontRef, impl Iterator<Item = (GlyphInstance, f32)> + 'a)> + 'a {
        let r = self.glyph_range();
        self.text.glyphs_with_x_advance_range(self.line_index, r)
    }

    fn x_width(&self) -> (Px, Px) {
        let IndexRange(start, end) = self.glyph_range();

        let start_x = self.text.glyphs[start].point.x;
        let end_x = if self.is_last {
            self.text.lines.width(self.line_index)
        } else {
            self.text.glyphs[end].point.x
        };

        (Px(start_x as i32), Px((end_x - start_x) as i32))
    }

    /// Bounds of the word or spaces.
    pub fn rect(&self) -> PxRect {
        let (x, width) = self.x_width();
        let size = PxSize::new(width, self.text.line_height);
        let origin = PxPoint::new(x, self.text.line_height * Px(self.line_index as i32));
        PxRect::new(origin, size)
    }

    fn decoration_line(&self, bottom_up_offset: Px) -> (PxPoint, Px) {
        let (x, width) = self.x_width();
        let y = (self.text.line_height * Px((self.line_index as i32) + 1)) - bottom_up_offset;
        (PxPoint::new(x, y + self.text.padding.top), width)
    }

    /// Overline spanning the word or spaces, start point + width.

    pub fn overline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.overline)
    }

    /// Strikethrough spanning the word or spaces, start point + width.

    pub fn strikethrough(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.strikethrough)
    }

    /// Underline spanning the word or spaces, not skipping.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns start point + width.

    pub fn underline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline)
    }

    /// Underline spanning the word or spaces, skipping glyph descends that intercept the line.
    ///
    /// Returns an iterator of start point + width for underline segments.

    pub fn underline_skip_glyphs(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        let y = (self.text.line_height * Px((self.line_index as i32) + 1)) - self.text.underline;
        let y = y + self.text.padding.top;
        let (x, _) = self.x_width();

        let line_y = -(self.text.baseline - self.text.underline).0 as f32;
        let line_y_range = (line_y, line_y - thickness.0 as f32);

        // space around glyph descends, thickness clamped to a minimum of 1px and a maximum of 0.2em (same as Firefox).
        let padding = (thickness.0 as f32).min(self.text.fonts.font(0).size().0 as f32 * 0.2).max(1.0);

        // no yield, only sadness
        struct UnderlineSkipGlyphs<'a, I, J> {
            line_y_range: (f32, f32),
            y: Px,
            padding: f32,
            min_width: Px,

            iter: I,
            resume: Option<(&'a FontRef, J)>,
            x: f32,
            width: f32,
        }
        impl<'a, I, J> UnderlineSkipGlyphs<'a, I, J> {
            fn line(&self) -> Option<(PxPoint, Px)> {
                fn f32_to_px(px: f32) -> Px {
                    Px(px.round() as i32)
                }
                let r = (PxPoint::new(f32_to_px(self.x), self.y), f32_to_px(self.width));
                if r.1 >= self.min_width {
                    Some(r)
                } else {
                    None
                }
            }
        }
        impl<'a, I, J> Iterator for UnderlineSkipGlyphs<'a, I, J>
        where
            I: Iterator<Item = (&'a FontRef, J)>,
            J: Iterator<Item = (GlyphInstance, f32)>,
        {
            type Item = (PxPoint, Px);

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    let continuation = self.resume.take().or_else(|| self.iter.next());
                    if let Some((font, mut glyphs_with_adv)) = continuation {
                        for (g, a) in &mut glyphs_with_adv {
                            if let Ok(Some((ex_start, ex_end))) = font.h_line_hits(g.index, self.line_y_range) {
                                self.width += ex_start - self.padding;
                                let r = self.line();
                                self.x += self.width + self.padding + ex_end + self.padding;
                                self.width = a - (ex_start + ex_end) - self.padding;

                                if r.is_some() {
                                    self.resume = Some((font, glyphs_with_adv));
                                    return r;
                                }
                            } else {
                                self.width += a;
                                // continue
                            }
                        }
                    } else {
                        let r = self.line();
                        self.width = 0.0;
                        return r;
                    }
                }
            }
        }
        UnderlineSkipGlyphs {
            line_y_range,
            y,
            padding,
            min_width: Px((padding / 2.0).max(1.0).ceil() as i32),

            iter: self.glyphs_with_x_advance(),
            resume: None,
            x: x.0 as f32,
            width: 0.0,
        }
    }

    /// Underline spanning the word or spaces, not skipping.
    ///
    /// The *y* is the baseline + descent + 1px.
    ///
    /// Returns start point + width.

    pub fn underline_descent(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline_descent)
    }
}

const WORD_CACHE_MAX_LEN: usize = 32;
const WORD_CACHE_MAX_ENTRIES: usize = 10_000;

#[derive(Hash, PartialEq, Eq)]
pub(super) struct WordCacheKey<S> {
    string: S,
    ctx_key: WordContextKey,
}
#[derive(Hash)]
struct WordCacheKeyRef<'a, S> {
    string: &'a S,
    ctx_key: &'a WordContextKey,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub(super) struct WordContextKey {
    lang: Lang,
    font_features: Option<Box<[usize]>>,
}
impl WordContextKey {
    pub fn new(config: &TextShapingArgs) -> Self {
        let is_64 = mem::size_of::<usize>() == mem::size_of::<u64>();

        let mut font_features = None;

        if !config.font_features.is_empty() {
            let mut features: Vec<_> = Vec::with_capacity(config.font_features.len() * if is_64 { 3 } else { 4 });
            for feature in &config.font_features {
                if is_64 {
                    let mut h = feature.tag().0 as usize;
                    h |= (feature.value() as usize) << 32;
                    features.push(h);
                } else {
                    features.push(feature.tag().0 as usize);
                    features.push(feature.value() as usize);
                }

                features.push(feature.start());
                features.push(feature.end());
            }

            font_features = Some(features.into_boxed_slice());
        }

        WordContextKey {
            lang: config.lang.clone(),
            font_features,
        }
    }
}

#[derive(Debug)]
pub(super) struct ShapedSegmentData {
    glyphs: Vec<ShapedGlyph>,
    x_advance: f32,
    y_advance: f32,
}
#[derive(Debug, Clone, Copy)]
struct ShapedGlyph {
    index: u32,
    //cluster: u32,
    point: (f32, f32),
}

impl Font {
    fn buffer_segment(&self, segment: &str, lang: &Lang) -> harfbuzz_rs::UnicodeBuffer {
        let mut buffer =
            harfbuzz_rs::UnicodeBuffer::new().set_direction(if lang.character_direction() == unic_langid::CharacterDirection::RTL {
                harfbuzz_rs::Direction::Rtl
            } else {
                harfbuzz_rs::Direction::Ltr
            });

        if let Some(lang) = to_buzz_lang(lang.language) {
            buffer = buffer.set_language(lang);
        }
        if let Some(script) = lang.script {
            buffer = buffer.set_script(to_buzz_script(script))
        }

        buffer.add_str(segment)
    }

    fn shape_segment_no_cache(&self, seg: &str, lang: &Lang, features: &[harfbuzz_rs::Feature]) -> ShapedSegmentData {
        let size_scale = self.metrics().size_scale;
        let to_layout = |p: i32| p as f32 * size_scale;

        let buffer = self.buffer_segment(seg, lang);
        let buffer = harfbuzz_rs::shape(self.harfbuzz_font(), buffer, features);

        let mut w_x_advance = 0.0;
        let mut w_y_advance = 0.0;
        let glyphs: Vec<_> = buffer
            .get_glyph_infos()
            .iter()
            .zip(buffer.get_glyph_positions())
            .map(|(i, p)| {
                let x_offset = to_layout(p.x_offset);
                let y_offset = to_layout(p.y_offset);
                let x_advance = to_layout(p.x_advance);
                let y_advance = to_layout(p.y_advance);

                let point = (w_x_advance + x_offset, w_y_advance + y_offset);
                w_x_advance += x_advance;
                w_y_advance += y_advance;

                ShapedGlyph {
                    index: i.codepoint,
                    // cluster: i.cluster,
                    point,
                }
            })
            .collect();

        ShapedSegmentData {
            glyphs,
            x_advance: w_x_advance,
            y_advance: w_y_advance,
        }
    }

    fn shape_segment(
        &self,
        seg: &str,
        word_ctx_key: &WordContextKey,
        lang: &Lang,
        features: &[harfbuzz_rs::Feature],
        out: impl FnOnce(&ShapedSegmentData),
    ) {
        if !(1..=WORD_CACHE_MAX_LEN).contains(&seg.len()) {
            let seg = self.shape_segment_no_cache(seg, lang, features);
            out(&seg);
        } else if let Some(small) = Self::to_small_word(seg) {
            let mut cache = self.small_word_cache.borrow_mut();

            if cache.len() > WORD_CACHE_MAX_ENTRIES {
                cache.clear();
            }

            let mut hasher = cache.hasher().build_hasher();
            WordCacheKeyRef {
                string: &small,
                ctx_key: word_ctx_key,
            }
            .hash(&mut hasher);
            let hash = hasher.finish();

            let seg = cache
                .raw_entry_mut()
                .from_hash(hash, |e| e.string == small && &e.ctx_key == word_ctx_key)
                .or_insert_with(|| {
                    let key = WordCacheKey {
                        string: small,
                        ctx_key: word_ctx_key.clone(),
                    };
                    let value = self.shape_segment_no_cache(seg, lang, features);
                    (key, value)
                })
                .1;

            out(seg)
        } else {
            let mut cache = self.word_cache.borrow_mut();

            if cache.len() > WORD_CACHE_MAX_ENTRIES {
                cache.clear();
            }

            let mut hasher = cache.hasher().build_hasher();
            WordCacheKeyRef {
                string: &seg,
                ctx_key: word_ctx_key,
            }
            .hash(&mut hasher);
            let hash = hasher.finish();

            let seg = cache
                .raw_entry_mut()
                .from_hash(hash, |e| e.string.as_str() == seg && &e.ctx_key == word_ctx_key)
                .or_insert_with(|| {
                    let key = WordCacheKey {
                        string: InternedStr::get_or_insert(seg),
                        ctx_key: word_ctx_key.clone(),
                    };
                    let value = self.shape_segment_no_cache(seg, lang, features);
                    (key, value)
                })
                .1;

            out(seg)
        }
    }

    /// Glyph index for the space `' ' ` character.
    pub fn space_index(&self) -> GlyphIndex {
        self.font.get_nominal_glyph(' ').unwrap_or(0)
    }

    /// Returns the horizontal advance of the space `' '` character.
    pub fn space_x_advance(&self) -> Px {
        let mut adv = 0.0;
        self.shape_segment(
            " ",
            &WordContextKey {
                lang: Lang::default(),
                font_features: None,
            },
            &Lang::default(),
            &[],
            |r| adv = r.x_advance,
        );

        Px(adv as i32)
    }

    /// Calculates a [`ShapedText`].
    pub fn shape_text(self: &FontRef, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        // let _scope = tracing::trace_span!("Font::shape_text", name=%self.face().display_name()).entered();

        let mut out = ShapedText::alloc();

        let metrics = self.metrics();

        out.line_height = config.line_height;
        out.line_spacing = config.line_spacing;

        let line_height = config.line_height.0 as f32;
        let line_spacing = config.line_spacing.0 as f32;
        let baseline = metrics.ascent + metrics.line_gap / 2.0;

        out.baseline = out.line_height - baseline;
        out.underline = out.baseline + metrics.underline_position;
        out.underline_descent = out.baseline + metrics.descent + Px(1);
        out.strikethrough = out.baseline + metrics.ascent / 3.0;
        out.overline = out.baseline + metrics.ascent;

        let dft_line_height = self.metrics().line_height().0 as f32;
        let center_height = (line_height - dft_line_height) / 2.0;

        let mut origin = euclid::point2::<_, ()>(0.0, baseline.0 as f32 + center_height);
        let mut max_line_x = 0.0;

        let word_ctx_key = WordContextKey::new(config);

        let letter_spacing = config.letter_spacing.0 as f32;
        let word_spacing = config.word_spacing.0 as f32;
        let tab_x_advance = config.tab_x_advance.0 as f32;
        let tab_index = self.space_index();

        for (seg, kind) in text.iter() {
            match kind {
                TextSegmentKind::Word => {
                    self.shape_segment(seg, &word_ctx_key, &config.lang, &config.font_features, |shaped_seg| {
                        out.glyphs.extend(shaped_seg.glyphs.iter().map(|gi| {
                            let r = GlyphInstance {
                                index: gi.index,
                                point: euclid::point2(gi.point.0 + origin.x, gi.point.1 + origin.y),
                            };
                            origin.x += letter_spacing;
                            r
                        }));
                        origin.x += shaped_seg.x_advance;
                        origin.y += shaped_seg.y_advance;
                    });
                }
                TextSegmentKind::Space => {
                    self.shape_segment(seg, &word_ctx_key, &config.lang, &config.font_features, |shaped_seg| {
                        out.glyphs.extend(shaped_seg.glyphs.iter().map(|gi| {
                            let r = GlyphInstance {
                                index: gi.index,
                                point: euclid::point2(gi.point.0 + origin.x, gi.point.1 + origin.y),
                            };
                            origin.x += word_spacing;
                            r
                        }));
                        origin.x += shaped_seg.x_advance;
                        origin.y += shaped_seg.y_advance;
                    });
                }
                TextSegmentKind::Tab => {
                    let point = euclid::point2(origin.x, origin.y);
                    origin.x += tab_x_advance;
                    out.glyphs.push(GlyphInstance { index: tab_index, point });
                }
                TextSegmentKind::LineBreak => {
                    out.lines.0.push(LineRange {
                        end: out.segments.0.len() + 1, // segment is pushed after match
                        width: origin.x,
                    });

                    max_line_x = origin.x.max(max_line_x);
                    origin.x = 0.0;
                    origin.y += line_height + line_spacing;
                }
            }

            out.segments.0.push(TextSegment {
                kind,
                end: out.glyphs.len(),
            });
        }

        out.lines.0.push(LineRange {
            end: out.segments.0.len(),
            width: origin.x,
        });

        // longest line width X line heights.
        out.size = PxSize::new(Px(origin.x.max(max_line_x).round() as i32), Px(0));
        out.update_height();

        out.fonts.0.push(FontRange {
            font: self.clone(),
            end: out.glyphs.len(),
        });

        out
    }

    /// Sends the sized vector path for a glyph to `sink`.
    pub fn outline(
        &self,
        glyph_id: GlyphIndex,
        hinting_options: OutlineHintingOptions,
        sink: &mut impl OutlineSink,
    ) -> Result<(), GlyphLoadingError> {
        struct AdapterSink<'a, S> {
            sink: &'a mut S,
            scale: f32,
        }
        impl<'a, S> AdapterSink<'a, S> {
            fn scale(&self, p: pathfinder_geometry::vector::Vector2F) -> euclid::Point2D<f32, Px> {
                euclid::point2(p.x() * self.scale, p.y() * self.scale)
            }
        }
        impl<'a, S: OutlineSink> font_kit::outline::OutlineSink for AdapterSink<'a, S> {
            fn move_to(&mut self, to: pathfinder_geometry::vector::Vector2F) {
                let to = self.scale(to);
                self.sink.move_to(to)
            }

            fn line_to(&mut self, to: pathfinder_geometry::vector::Vector2F) {
                let to = self.scale(to);
                self.sink.line_to(to)
            }

            fn quadratic_curve_to(&mut self, ctrl: pathfinder_geometry::vector::Vector2F, to: pathfinder_geometry::vector::Vector2F) {
                let ctrl = self.scale(ctrl);
                let to = self.scale(to);
                self.sink.quadratic_curve_to(ctrl, to)
            }

            fn cubic_curve_to(
                &mut self,
                ctrl: pathfinder_geometry::line_segment::LineSegment2F,
                to: pathfinder_geometry::vector::Vector2F,
            ) {
                let l_from = self.scale(ctrl.from());
                let l_to = self.scale(ctrl.to());
                let to = self.scale(to);
                self.sink.cubic_curve_to((l_from, l_to), to)
            }

            fn close(&mut self) {
                self.sink.close()
            }
        }

        let scale = self.metrics().size_scale;

        self.face()
            .font_kit()
            .outline(glyph_id, hinting_options, &mut AdapterSink { sink, scale })
    }

    /// Returns the boundaries of a glyph in pixel units.
    ///
    /// The rectangle origin is the bottom-left of the bounds relative to the baseline.
    pub fn typographic_bounds(&self, glyph_id: GlyphIndex) -> Result<euclid::Rect<f32, Px>, GlyphLoadingError> {
        let rect = self.face().font_kit().typographic_bounds(glyph_id)?;

        let scale = self.metrics().size_scale;
        let bounds = euclid::rect::<f32, Px>(
            rect.origin_x() * scale,
            rect.origin_y() * scale,
            rect.width() * scale,
            rect.height() * scale,
        );

        Ok(bounds)
    }

    /// Ray cast an horizontal line across the glyph and returns the entry and exit hits.
    ///
    /// The `line_y_range` are two vertical offsets relative to the baseline, the offsets define
    /// the start and inclusive end of the horizontal line, that is, `(underline, underline + thickness)`, note
    /// that positions under the baseline are negative so a 2px underline set 1px under the baseline becomes `(-1.0, -3.0)`.
    ///
    /// Returns `Ok(Some(x_enter, x_exit))` where the two values are x-advances, returns `None` if there is not hit, returns
    /// an error if the glyph is not found. The first x-advance is from the left typographic border to the first hit on the outline,
    /// the second x-advance is from the first across the outline to the exit hit.
    pub fn h_line_hits(&self, glyph_id: GlyphIndex, line_y_range: (f32, f32)) -> Result<Option<(f32, f32)>, GlyphLoadingError> {
        // Algorithm:
        //
        //  - Ignore curves, everything is direct line.
        //  - If a line-y crosses `line_y_range` register the min-x and max-x from the two points.
        //  - Same if a line is inside `line_y_range`.
        struct InterseptsSink {
            start: Option<euclid::Point2D<f32, Px>>,
            curr: euclid::Point2D<f32, Px>,
            under: (bool, bool),

            line_y_range: (f32, f32),
            hit: Option<(f32, f32)>,
        }
        impl OutlineSink for InterseptsSink {
            fn move_to(&mut self, to: euclid::Point2D<f32, Px>) {
                self.start = Some(to);
                self.curr = to;
                self.under = (to.y < self.line_y_range.0, to.y < self.line_y_range.1);
            }

            fn line_to(&mut self, to: euclid::Point2D<f32, Px>) {
                let under = (to.y < self.line_y_range.0, to.y < self.line_y_range.1);

                if self.under != under || under == (true, false) {
                    // crossed one or two y-range boundaries or both points are inside
                    self.under = under;

                    let (x0, x1) = if self.curr.x < to.x {
                        (self.curr.x, to.x)
                    } else {
                        (to.x, self.curr.x)
                    };
                    if let Some((min, max)) = &mut self.hit {
                        *min = min.min(x0);
                        *max = max.max(x1);
                    } else {
                        self.hit = Some((x0, x1));
                    }
                }

                self.curr = to;
                self.under = under;
            }

            fn quadratic_curve_to(&mut self, _: euclid::Point2D<f32, Px>, to: euclid::Point2D<f32, Px>) {
                self.line_to(to);
            }

            fn cubic_curve_to(&mut self, _: (euclid::Point2D<f32, Px>, euclid::Point2D<f32, Px>), to: euclid::Point2D<f32, Px>) {
                self.line_to(to);
            }

            fn close(&mut self) {
                if let Some(s) = self.start.take() {
                    if s != self.curr {
                        self.line_to(s);
                    }
                }
            }
        }
        let mut sink = InterseptsSink {
            start: None,
            curr: euclid::point2(0.0, 0.0),
            under: (false, false),

            line_y_range,
            hit: None,
        };
        self.outline(glyph_id, OutlineHintingOptions::None, &mut sink)?;

        Ok(sink.hit.map(|(a, b)| (a, b - a)))
    }
}

/// Hinting options for [`Font::outline`].
pub type OutlineHintingOptions = font_kit::hinting::HintingOptions;

/// Receives Bézier path rendering commands from [`Font::outline`].
///
/// The points are relative to the baseline, negative values under, positive over.
pub trait OutlineSink {
    /// Moves the pen to a point.
    fn move_to(&mut self, to: euclid::Point2D<f32, Px>);
    /// Draws a line to a point.
    fn line_to(&mut self, to: euclid::Point2D<f32, Px>);
    /// Draws a quadratic Bézier curve to a point.
    fn quadratic_curve_to(&mut self, ctrl: euclid::Point2D<f32, Px>, to: euclid::Point2D<f32, Px>);
    /// Draws a cubic Bézier curve to a point.
    ///
    /// The `ctrl` is a line (from, to).
    fn cubic_curve_to(&mut self, ctrl: (euclid::Point2D<f32, Px>, euclid::Point2D<f32, Px>), to: euclid::Point2D<f32, Px>);
    /// Closes the path, returning to the first point in it.
    fn close(&mut self);
}

impl FontList {
    /// Calculates a [`ShapedText`] using the [best](FontList::best) font in this list.
    pub fn shape_text(&self, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        let mut r = self.best().shape_text(text, config);

        if self.len() == 1 || r.is_empty() {
            return r;
        }

        // find segments that contain unresolved glyphs (`0`) and collect replacements:
        let mut replacement_segs = vec![];

        for (i, (_kind, gr)) in r.segments.iter_glyphs().enumerate() {
            let glyphs = &r.glyphs[gr.iter()];
            if glyphs.iter().any(|g| g.index == 0) {
                // try fallbacks:
                for font in &self[1..] {
                    let text = text.get_clone(i).unwrap();
                    let replacement = font.shape_text(&text, config);

                    if replacement.glyphs.iter().all(|g| g.index != 0) {
                        replacement_segs.push((i, replacement));
                        break;
                    }
                }
            }
        }

        if replacement_segs.is_empty() {
            r
        } else if r.segments.0.len() == replacement_segs.len() {
            // all segments replaced, concat replacements:
            let mut iter = replacement_segs.into_iter();
            let (_, mut r) = iter.next().unwrap();
            for (_, repl) in iter {
                r.extend(repl);
            }
            r
        } else {
            let mut parts = Vec::with_capacity(replacement_segs.len() * 2);
            for (i, repl) in replacement_segs.into_iter().rev() {
                let (new_r, tail) = r.split_remove(i);
                parts.push(tail);
                parts.push(repl);
                r = new_r;
            }
            for part in parts.into_iter().rev() {
                r.extend(part);
            }
            r
        }
    }
}

fn to_buzz_lang(lang: unic_langid::subtags::Language) -> Option<harfbuzz_rs::Language> {
    lang.as_str().parse().ok()
}

fn to_buzz_script(script: unic_langid::subtags::Script) -> harfbuzz_rs::Tag {
    let t: u32 = script.into();
    let t = t.to_le_bytes(); // Script is a TinyStr4 that uses LE
    harfbuzz_rs::Tag::from(&[t[0], t[1], t[2], t[3]])
}

#[cfg(test)]
mod tests {
    use crate::{app::App, text::*};

    fn test_font() -> FontRef {
        let mut app = App::default().run_headless(false);
        app.ctx()
            .services
            .fonts()
            .get_normal(&FontName::sans_serif(), &lang!(und))
            .unwrap()
            .sized(Px(20), vec![])
    }

    #[test]
    fn set_line_spacing() {
        let text = "0\n1\n2\n3\n4";
        test_line_spacing(text, Px(20), Px(0));
        test_line_spacing(text, Px(0), Px(20));
        test_line_spacing(text, Px(4), Px(6));
        test_line_spacing(text, Px(4), Px(4));
        test_line_spacing("a line\nanother\nand another", Px(20), Px(0));
        test_line_spacing("", Px(20), Px(0));
        test_line_spacing("a line", Px(20), Px(0));
    }
    fn test_line_spacing(text: &'static str, from: Px, to: Px) {
        let font = test_font();
        let mut config = TextShapingArgs {
            line_height: Px(40),
            line_spacing: from,
            ..Default::default()
        };

        let text = SegmentedText::new(text);
        let mut test = font.shape_text(&text, &config);

        config.line_spacing = to;
        let expected = font.shape_text(&text, &config);

        assert_eq!(from, test.line_spacing());
        test.set_line_spacing(to);
        assert_eq!(to, test.line_spacing());

        for (i, (g0, g1)) in test.glyphs.iter().zip(expected.glyphs.iter()).enumerate() {
            assert_eq!(g0, g1, "testing {from} to {to}, glyph {i} is not equal");
        }

        assert_eq!(test.size(), expected.size());
    }

    #[test]
    fn set_line_height() {
        let text = "0\n1\n2\n3\n4";
        test_line_height(text, Px(20), Px(10));
        test_line_height(text, Px(10), Px(20));
        test_line_height(text, Px(20), Px(20));
        test_line_height("a line\nanother\nand another", Px(20), Px(10));
        test_line_height("", Px(20), Px(10));
        test_line_height("a line", Px(20), Px(10));
    }
    fn test_line_height(text: &'static str, from: Px, to: Px) {
        let font = test_font();
        let mut config = TextShapingArgs {
            line_height: from,
            line_spacing: Px(20),
            ..Default::default()
        };

        let text = SegmentedText::new(text);
        let mut test = font.shape_text(&text, &config);

        config.line_height = to;
        let expected = font.shape_text(&text, &config);

        assert_eq!(from, test.line_height());
        test.set_line_height(to);
        assert_eq!(to, test.line_height());

        for (i, (g0, g1)) in test.glyphs.iter().zip(expected.glyphs.iter()).enumerate() {
            assert_eq!(g0, g1, "testing {from} to {to}, glyph {i} is not equal");
        }

        assert_eq!(test.size(), expected.size());
    }

    #[test]
    fn split_single_line() {
        test_split("a b", 1, "a", " b");
        test_split("one another", 1, "one", " another");
        test_split("one another then rest", 3, "one another", " then rest");
        test_split("at start", 0, "", "at start");
        test_split("at end", 2, "at ", "end");
    }
    fn test_split(full_text: &'static str, segment: usize, a: &'static str, b: &'static str) {
        let font = test_font();
        let config = TextShapingArgs::default();

        let seg_text = SegmentedText::new(full_text);
        let a = SegmentedText::new(a);
        let b = SegmentedText::new(b);

        let shaped_text = font.shape_text(&seg_text, &config);
        let expected_a = font.shape_text(&a, &config);
        let expected_b = font.shape_text(&b, &config);

        let (actual_a, actual_b) = shaped_text.split(segment);

        let full_text = escape(full_text);
        pretty_assertions::assert_eq!(expected_a, actual_a, "failed \"{full_text}\"");
        pretty_assertions::assert_eq!(expected_b, actual_b, "failed \"{full_text}\"");
    }

    #[test]
    #[should_panic(expected = "segment out of bounds, the len is 3 but the segment is 3")]
    fn split_out_of_range() {
        test_split("at len", 3, "at len", "");
    }

    #[test]
    fn split_multi_line() {
        test_split("a\nb", 1, "a", "\nb");
        test_split("a\nb", 2, "a\n", "b");

        test_split("a b\nc", 1, "a", " b\nc");
        test_split("a\nb c", 3, "a\nb", " c");

        test_split("one\nanother", 1, "one", "\nanother");
        test_split("one\nanother", 2, "one\n", "another");
    }

    #[test]
    fn split_remove_single_line() {
        test_split_remove("a b", 1, "a", "b");
        test_split_remove("one another", 1, "one", "another");
        test_split_remove("one another then rest", 3, "one another", "then rest");
        test_split_remove("at start", 0, "", " start");
        test_split_remove("at end", 2, "at ", "");
    }
    fn test_split_remove(full_text: &'static str, segment: usize, a: &'static str, b: &'static str) {
        let font = test_font();
        let config = TextShapingArgs::default();

        let seg_text = SegmentedText::new(full_text);
        let a = SegmentedText::new(a);
        let b = SegmentedText::new(b);

        let shaped_text = font.shape_text(&seg_text, &config);
        let expected_a = font.shape_text(&a, &config);
        let expected_b = font.shape_text(&b, &config);

        let (actual_a, actual_b) = shaped_text.split_remove(segment);

        let full_text = escape(full_text);
        pretty_assertions::assert_eq!(expected_a, actual_a, "failed \"{full_text}\"");
        pretty_assertions::assert_eq!(expected_b, actual_b, "failed \"{full_text}\"");
    }

    #[test]
    fn split_remove_multi_line() {
        test_split_remove("a\nb", 1, "a", "b");
        test_split_remove("a\nb", 2, "a\n", "");
        test_split_remove("a b\nc", 1, "a", "b\nc");
        test_split_remove("a\nb c", 3, "a\nb", "c");
        test_split_remove("one\nanother", 1, "one", "another");
        test_split_remove("one\nanother", 2, "one\n", "");
    }

    #[test]
    fn extend_single_line() {
        test_extend("a", " b");
        test_extend("first", " second");
        test_extend("", "empty");
        test_extend("empty", "");
    }
    fn test_extend(a: &'static str, b: &'static str) {
        let font = test_font();
        let config = TextShapingArgs::default();

        let s_a = SegmentedText::new(a);
        let s_b = SegmentedText::new(b);
        let s_expected = SegmentedText::new(a.to_owned() + b);

        let mut sh_a = font.shape_text(&s_a, &config);
        let sh_b = font.shape_text(&s_b, &config);
        let sh_expected = font.shape_text(&s_expected, &config);

        sh_a.extend(sh_b);

        pretty_assertions::assert_eq!(sh_expected, sh_a, "failed \"{}\" + \"{}\"", escape(a), escape(b));
    }

    #[test]
    fn extend_multi_line() {
        test_extend("a\n", "b");
        test_extend("a", "\nb");

        test_extend("first\n", "second");
        test_extend("first", "\nsecond");

        test_extend("\n", "\n");
        test_extend("a ", "b\nc");
    }

    fn escape(s: &str) -> String {
        s.replace('\n', "\\n")
    }
}

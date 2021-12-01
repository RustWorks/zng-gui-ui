//! Font resolving and text shaping.

pub use crate::render::webrender_api::GlyphInstance;
use crate::units::*;
use crate::var::impl_from_and_into_var;
use derive_more as dm;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::hash::Hash;
use std::{
    borrow::Cow,
    fmt, mem,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

pub use unicode_script::{self, Script};

pub mod font_features;
pub use font_features::FontFeatures;

mod font_loading;
pub use font_loading::*;

mod segmenting;
pub use segmenting::*;

mod shaping;
pub use shaping::*;

pub use font_kit::properties::{Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};

pub use crate::app::view_process::TextAntiAliasing;

/// Configuration of text wrapping for Chinese, Japanese, or Korean text.
#[derive(Copy, Clone)]
pub enum LineBreak {
    /// The same rule used by other languages.
    Auto,
    /// The least restrictive rule, good for short lines.
    Loose,
    /// The most common rule.
    Normal,
    /// The most stringent rule.
    Strict,
    /// Allow line breaks in between any character including punctuation.
    Anywhere,
}
impl Default for LineBreak {
    /// [`LineBreak::Auto`]
    fn default() -> Self {
        LineBreak::Auto
    }
}
impl fmt::Debug for LineBreak {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LineBreak::")?;
        }
        match self {
            LineBreak::Auto => write!(f, "Auto"),
            LineBreak::Loose => write!(f, "Loose"),
            LineBreak::Normal => write!(f, "Normal"),
            LineBreak::Strict => write!(f, "Strict"),
            LineBreak::Anywhere => write!(f, "Anywhere"),
        }
    }
}

/// Hyphenation configuration.
#[derive(Copy, Clone)]
pub enum Hyphens {
    /// Hyphens are never inserted in word breaks.
    None,
    /// Word breaks only happen in specially marked break characters: `-` and `\u{00AD} SHY`.
    ///
    /// * `U+2010` - The visible hyphen character.
    /// * `U+00AD` - The invisible hyphen character, is made visible in a word break.
    Manual,
    /// Hyphens are inserted like `Manual` and also using language specific hyphenation rules.
    // TODO https://sourceforge.net/projects/hunspell/files/Hyphen/2.8/
    Auto,
}
impl Default for Hyphens {
    /// [`Hyphens::Auto`]
    fn default() -> Self {
        Hyphens::Auto
    }
}
impl fmt::Debug for Hyphens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Hyphens::")?;
        }
        match self {
            Hyphens::None => write!(f, "None"),
            Hyphens::Manual => write!(f, "Manual"),
            Hyphens::Auto => write!(f, "Auto"),
        }
    }
}

/// Configure line breaks inside words during text wrap.
///
/// This value is only considered if it is impossible to fit the a word to a line.
///
/// Hyphens can be inserted in word breaks using the [`Hyphens`] configuration.
#[derive(Copy, Clone)]
pub enum WordBreak {
    /// Line breaks can be inserted in between letters of Chinese/Japanese/Korean text only.
    Normal,
    /// Line breaks can be inserted between any letter.
    BreakAll,
    /// Line breaks are not inserted between any letter.
    KeepAll,
}
impl Default for WordBreak {
    /// [`WordBreak::Normal`]
    fn default() -> Self {
        WordBreak::Normal
    }
}
impl fmt::Debug for WordBreak {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WordBreak::")?;
        }
        match self {
            WordBreak::Normal => write!(f, "Normal"),
            WordBreak::BreakAll => write!(f, "BreakAll"),
            WordBreak::KeepAll => write!(f, "KeepAll"),
        }
    }
}

/// Text alignment.
#[derive(Copy, Clone)]
pub enum TextAlign {
    /// `Left` in LTR or `Right` in RTL.
    Start,
    /// `Right` in LTR or `Left` in RTL.
    End,

    /// Left visually, independent of LTR/RTL.
    Left,

    /// Center visually.
    Center,

    /// Right visually, independent of LTR/RTL.
    Right,

    /// Adjust spacing to fill the available width.
    ///
    /// The justify spacing can be configured using [`Justify`].
    Justify(Justify),
}
impl TextAlign {
    /// Justify Auto.
    #[inline]
    pub fn justify() -> Self {
        TextAlign::Justify(Justify::Auto)
    }
}
impl Default for TextAlign {
    /// [`TextAlign::Start`].
    #[inline]
    fn default() -> Self {
        TextAlign::Start
    }
}
impl fmt::Debug for TextAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "TextAlign::")?;
        }
        match self {
            TextAlign::Start => write!(f, "Start"),
            TextAlign::End => write!(f, "End"),
            TextAlign::Left => write!(f, "Left"),
            TextAlign::Center => write!(f, "Center"),
            TextAlign::Right => write!(f, "Right"),
            TextAlign::Justify(j) => f.debug_tuple("Justify").field(j).finish(),
        }
    }
}

/// Text alignment justification mode.
#[derive(Copy, Clone)]
pub enum Justify {
    /// Selects the justification mode based on the language.
    /// For Chinese/Japanese/Korean uses `InterLetter` for the others uses `InterWord`.
    Auto,
    /// The text is justified by adding space between words.
    ///
    /// This only works if [`WordSpacing`](crate::units::WordSpacing) is set to auto.
    InterWord,
    /// The text is justified by adding space between letters.
    ///
    /// This only works if *letter spacing* is set to auto.
    InterLetter,
}
impl Default for Justify {
    /// [`Justify::Auto`]
    fn default() -> Self {
        Justify::Auto
    }
}
impl fmt::Debug for Justify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Justify::")?;
        }
        match self {
            Justify::Auto => write!(f, "Auto"),
            Justify::InterWord => write!(f, "InterWord"),
            Justify::InterLetter => write!(f, "InterLetter"),
        }
    }
}

/// Various metrics that apply to the entire [`FontFace`].
///
/// For OpenType fonts, these mostly come from the `OS/2` table.
#[derive(Clone, Debug)]
pub struct FontFaceMetrics {
    /// The number of font units per em.
    ///
    /// Font sizes are usually expressed in pixels per em; e.g. `12px` means 12 pixels per em.
    pub units_per_em: u32,

    /// The maximum amount the font rises above the baseline, in font units.
    pub ascent: f32,

    /// The maximum amount the font descends below the baseline, in font units.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: f32,

    /// Distance between baselines, in font units.
    pub line_gap: f32,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in font units.
    pub underline_position: f32,

    /// A suggested value for the underline thickness, in font units.
    pub underline_thickness: f32,

    /// The approximate amount that uppercase letters rise above the baseline, in font units.
    pub cap_height: f32,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: f32,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in font units.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounding_box: euclid::Rect<f32, ()>,
}
impl FontFaceMetrics {
    /// Compute [`FontMetrics`] given a font size in pixels.
    pub fn sized(&self, font_size_px: Px) -> FontMetrics {
        let size_scale = 1.0 / self.units_per_em as f32 * font_size_px.0 as f32;
        let s = move |f: f32| Px((f * size_scale).round() as i32);
        FontMetrics {
            size_scale,
            ascent: s(self.ascent),
            descent: s(self.descent),
            line_gap: s(self.line_gap),
            underline_position: s(self.underline_position),
            underline_thickness: s(self.underline_thickness),
            cap_height: s(self.cap_height),
            x_height: (s(self.x_height)),
            bounding_box: {
                let b = self.bounding_box;
                PxRect::new(
                    PxPoint::new(s(b.origin.x), s(b.origin.y)),
                    PxSize::new(s(b.size.width), s(b.size.height)),
                )
            },
        }
    }
}

/// Various metrics about a [`Font`].
///
/// You can compute these metrics from a [`FontFaceMetrics`]
#[derive(Clone, Debug)]
pub struct FontMetrics {
    /// Multiply this to a font EM value to get the size in pixels.
    pub size_scale: f32,

    /// The maximum amount the font rises above the baseline, in pixels.
    pub ascent: Px,

    /// The maximum amount the font descends below the baseline, in pixels.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: Px,

    /// Distance between baselines, in pixels.
    pub line_gap: Px,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in pixels.
    pub underline_position: Px,

    /// A suggested value for the underline thickness, in pixels.
    pub underline_thickness: Px,

    /// The approximate amount that uppercase letters rise above the baseline, in pixels.
    pub cap_height: Px,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: Px,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in pixels.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounding_box: PxRect,
}
impl FontMetrics {
    /// The font line height.
    pub fn line_height(&self) -> Px {
        self.ascent - self.descent + self.line_gap
    }
}

/// Text transform function.
#[derive(Clone)]
pub enum TextTransformFn {
    /// No transform.
    None,
    /// To UPPERCASE.
    Uppercase,
    /// to lowercase.
    Lowercase,
    /// Custom transform function.
    Custom(Rc<dyn Fn(Text) -> Text>),
}
impl TextTransformFn {
    /// Apply the text transform.
    pub fn transform(&self, text: Text) -> Text {
        match self {
            TextTransformFn::None => text,
            TextTransformFn::Uppercase => Text::owned(text.to_uppercase()),
            TextTransformFn::Lowercase => Text::owned(text.to_lowercase()),
            TextTransformFn::Custom(fn_) => fn_(text),
        }
    }

    /// New [`Custom`](Self::Custom).
    pub fn custom(fn_: impl Fn(Text) -> Text + 'static) -> Self {
        TextTransformFn::Custom(Rc::new(fn_))
    }
}
impl fmt::Debug for TextTransformFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "TextTransformFn::")?;
        }
        match self {
            TextTransformFn::None => write!(f, "None"),
            TextTransformFn::Uppercase => write!(f, "Uppercase"),
            TextTransformFn::Lowercase => write!(f, "Lowercase"),
            TextTransformFn::Custom(_) => write!(f, "Custom"),
        }
    }
}

/// Text white space transform.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum WhiteSpace {
    /// Text is not changed, all white spaces and line breaks are preserved.
    Preserve,
    /// Replace sequences of white space with a single `U+0020 SPACE` and trim lines. Line breaks are preserved.
    Merge,
    /// Replace sequences of white space and line breaks with `U+0020 SPACE` and trim the text.
    MergeNoBreak,
}
impl Default for WhiteSpace {
    /// [`WhiteSpace::Preserve`].
    #[inline]
    fn default() -> Self {
        WhiteSpace::Preserve
    }
}
impl WhiteSpace {
    /// Transform the white space of the text.
    #[inline]
    pub fn transform(self, text: Text) -> Text {
        match self {
            WhiteSpace::Preserve => text,
            WhiteSpace::Merge => text.split_ascii_whitespace().collect::<Vec<_>>().join(" ").into(),
            WhiteSpace::MergeNoBreak => text.split_whitespace().collect::<Vec<_>>().join(" ").into(),
        }
    }
}
impl fmt::Debug for WhiteSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WhiteSpace::")?;
        }
        match self {
            WhiteSpace::Preserve => write!(f, "Preserve"),
            WhiteSpace::Merge => write!(f, "Merge"),
            WhiteSpace::MergeNoBreak => write!(f, "MergeNoBreak"),
        }
    }
}

/// Font family name.
///
/// A possible value for the `font_family` property.
///
/// # Case Insensitive
///
/// Font family names are case-insensitive. `"Arial"` and `"ARIAL"` are equal and have the same hash.
#[derive(Clone)]
pub struct FontName {
    text: Text,
    is_ascii: bool,
}
impl fmt::Debug for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("FontName")
                .field("text", &self.text)
                .field("is_ascii", &self.is_ascii)
                .finish()
        } else {
            write!(f, "{:?}", self.text)
        }
    }
}
impl PartialEq for FontName {
    fn eq(&self, other: &Self) -> bool {
        self.unicase() == other.unicase()
    }
}
impl Eq for FontName {}
impl Hash for FontName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.unicase(), state)
    }
}
impl FontName {
    fn unicase(&self) -> unicase::UniCase<&str> {
        if self.is_ascii {
            unicase::UniCase::ascii(self)
        } else {
            unicase::UniCase::unicode(self)
        }
    }

    /// New font name.
    ///
    /// Note that the inner name value is a [`Text`] so you can define a font name using `&'static str` or `String`.
    ///
    /// Font names are case insensitive but the input casing is preserved, this casing shows during display and in
    /// the value of [`name`](Self::name).
    #[inline]
    pub fn new(name: impl Into<Text>) -> Self {
        let text = name.into();
        FontName {
            is_ascii: text.is_ascii(),
            text,
        }
    }

    /// New "serif" font.
    ///
    /// Serif fonts represent the formal text style for a script.
    #[inline]
    pub fn serif() -> Self {
        Self::new("serif")
    }

    /// New "sans-serif" font.
    ///
    /// Glyphs in sans-serif fonts, are generally low contrast (vertical and horizontal stems have the close to the same thickness)
    /// and have stroke endings that are plain — without any flaring, cross stroke, or other ornamentation.
    #[inline]
    pub fn sans_serif() -> Self {
        Self::new("sans-serif")
    }

    /// New "monospace" font.
    ///
    /// The sole criterion of a monospace font is that all glyphs have the same fixed width.
    #[inline]
    pub fn monospace() -> Self {
        Self::new("monospace")
    }

    /// New "cursive" font.
    ///
    /// Glyphs in cursive fonts generally use a more informal script style, and the result looks more
    /// like handwritten pen or brush writing than printed letter-work.
    #[inline]
    pub fn cursive() -> Self {
        Self::new("cursive")
    }

    /// New "fantasy" font.
    ///
    /// Fantasy fonts are primarily decorative or expressive fonts that contain decorative or expressive representations of characters.
    #[inline]
    pub fn fantasy() -> Self {
        Self::new("fantasy")
    }

    /// Reference the font name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.text
    }

    /// Unwraps into a [`Text`].
    #[inline]
    pub fn into_text(self) -> Text {
        self.text
    }
}
impl_from_and_into_var! {
    fn from(s: &'static str) -> FontName {
        FontName::new(s)
    }
    fn from(s: String) -> FontName {
        FontName::new(s)
    }
    fn from(s: Cow<'static, str>) -> FontName {
        FontName::new(s)
    }
}
impl fmt::Display for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
impl std::ops::Deref for FontName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.text.deref()
    }
}
impl AsRef<str> for FontName {
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

/// A list of [font names](FontName) in priority order.
///
/// # Example
///
/// This type is usually initialized using conversion:
///
/// ```
/// # use zero_ui_core::text::*;
/// fn foo(font_names: impl Into<FontNames>) { }
///
/// foo(["Arial", "sans-serif", "monospace"]);
/// ```
///
/// You can also use the specialized [`push`](Self::push) that converts:
///
/// ```
/// # use zero_ui_core::text::*;
/// let user_preference = "Comic Sans".to_owned();
///
/// let mut names = FontNames::empty();
/// names.push(user_preference);
/// names.push("Arial");
/// names.extend(FontNames::default());
/// ```
///
/// # Default
///
/// The default value is [`sans_serif`](FontName::sans_serif), [`serif`](FontName::serif),
/// [`monospace`](FontName::sans_serif), [`cursive`](FontName::sans_serif) and [`fantasy`](FontName::sans_serif).
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct FontNames(pub Vec<FontName>);
impl FontNames {
    /// Empty list.
    #[inline]
    pub fn empty() -> Self {
        FontNames(vec![])
    }

    /// Push a font name from any type that converts to [`FontName`].
    pub fn push(&mut self, font_name: impl Into<FontName>) {
        self.0.push(font_name.into())
    }
}
impl Default for FontNames {
    fn default() -> Self {
        FontNames(vec![
            FontName::sans_serif(),
            FontName::serif(),
            FontName::monospace(),
            FontName::cursive(),
            FontName::fantasy(),
        ])
    }
}
impl fmt::Debug for FontNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("FontNames").field(&self.0).finish()
        } else if self.0.is_empty() {
            write!(f, "[]")
        } else if self.0.len() == 1 {
            write!(f, "{:?}", self.0[0])
        } else {
            write!(f, "[{:?}, ", self.0[0])?;
            for name in &self.0[1..] {
                write!(f, "{:?}, ", name)?;
            }
            write!(f, "]")
        }
    }
}
impl_from_and_into_var! {
    fn from(font_name: &'static str) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_name: String) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_name: Text) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_names: Vec<FontName>) -> FontNames {
        FontNames(font_names)
    }

    fn from(font_names: Vec<&'static str>) -> FontNames {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }

    fn from(font_names: Vec<String>) -> FontNames {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }

    fn from(font_name: FontName) -> FontNames {
        FontNames(vec![font_name])
    }
}
impl Deref for FontNames {
    type Target = Vec<FontName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for FontNames {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::iter::Extend<FontName> for FontNames {
    fn extend<T: IntoIterator<Item = FontName>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}
impl IntoIterator for FontNames {
    type Item = FontName;

    type IntoIter = std::vec::IntoIter<FontName>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<const N: usize> From<[FontName; N]> for FontNames {
    fn from(font_names: [FontName; N]) -> Self {
        FontNames(font_names.into())
    }
}
impl<const N: usize> IntoVar<FontNames> for [FontName; N] {
    type Var = OwnedVar<FontNames>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}
impl<const N: usize> From<[&'static str; N]> for FontNames {
    fn from(font_names: [&'static str; N]) -> Self {
        FontNames(std::array::IntoIter::new(font_names).map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [&'static str; N] {
    type Var = OwnedVar<FontNames>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}
impl<const N: usize> From<[String; N]> for FontNames {
    fn from(font_names: [String; N]) -> Self {
        FontNames(std::array::IntoIter::new(font_names).map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [String; N] {
    type Var = OwnedVar<FontNames>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}
impl<const N: usize> From<[Text; N]> for FontNames {
    fn from(font_names: [Text; N]) -> Self {
        FontNames(std::array::IntoIter::new(font_names).map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [Text; N] {
    type Var = OwnedVar<FontNames>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into())
    }
}

static INTERN_POOL: Mutex<Option<HashSet<InternedStr>>> = parking_lot::const_mutex(None);

/// A reference-counted shared string.
///
/// # Equality
///
/// Equality is defined by the string buffer, a [`InternedStr`] has the same hash as a `&str`.
#[derive(Clone)]
pub struct InternedStr(Arc<String>);
impl InternedStr {
    /// Gets a reference to the string `s` in the interning pool.
    /// The string is inserted only if it is not present.
    pub fn get_or_insert(s: impl AsRef<str> + Into<String>) -> Self {
        let mut map = INTERN_POOL.lock();
        let map = map.get_or_insert_with(HashSet::default);
        if let Some(r) = map.get(s.as_ref()) {
            r.clone()
        } else {
            let s = InternedStr(Arc::new(s.into()));
            let r = s.clone();
            map.insert(s);
            r
        }
    }

    /// Reference the string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Intern the string for the duration of the process.
    pub fn permanent(&self) {
        let leak = Arc::clone(&self.0);
        let _ = Arc::into_raw(leak);
    }
}
impl Hash for InternedStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}
impl PartialEq for InternedStr {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for InternedStr {}
impl AsRef<str> for InternedStr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::borrow::Borrow<str> for InternedStr {
    fn borrow(&self) -> &str {
        &self.0
    }
}
impl Drop for InternedStr {
    fn drop(&mut self) {
        if Arc::strong_count(&self.0) == 2 {
            INTERN_POOL.lock().as_mut().unwrap().remove(self);
        }
    }
}

const INLINE_MAX: usize = mem::size_of::<usize>() * 3;

fn inline_to_str(d: &[u8; INLINE_MAX]) -> &str {
    let utf8 = if let Some(i) = d.iter().position(|&b| b == b'\0') {
        &d[..i]
    } else {
        &d[..]
    };
    unsafe { std::str::from_utf8_unchecked(utf8) }
}
fn str_to_inline(s: &str) -> [u8; INLINE_MAX] {
    let mut inline = [b'\0'; INLINE_MAX];
    (&mut inline[..s.len()]).copy_from_slice(s.as_bytes());
    inline
}

#[derive(Clone)]
enum TextData {
    Static(&'static str),
    Inline([u8; INLINE_MAX]),
    Interned(InternedStr, usize, usize),
    Owned(String),
}
impl fmt::Debug for TextData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(s) => write!(f, "Static({:?})", s),
            Self::Inline(d) => write!(f, "Inline({:?})", inline_to_str(d)),
            Self::Interned(s, _, _) => write!(f, "Interned({:?})", s.as_ref()),
            Self::Owned(s) => write!(f, "Owned({:?})", s),
        }
    }
}
impl fmt::Display for TextData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}
impl PartialEq for TextData {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}
impl Eq for TextData {}
impl Hash for TextData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.deref(), state)
    }
}
impl Deref for TextData {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            TextData::Static(s) => s,
            TextData::Inline(d) => inline_to_str(d),
            TextData::Interned(entry, start, len) => &entry.as_ref()[*start..*len],
            TextData::Owned(s) => s,
        }
    }
}

/// Text string type, can be owned, static, inlined or interned.
///
/// Note that this type dereferences to [`str`] so you can use all methods
/// of that type also. For mutation you can call [`to_mut`]
/// to access all mutating methods of [`String`]. The mutations that can be
/// implemented using only a borrowed `str` are provided as methods in this type.
///
/// [`to_mut`]: Text::to_mut
#[derive(Clone, dm::Display, PartialEq, Eq, Hash)]
pub struct Text(TextData);
impl Text {
    /// New text that is a `&'static str`.
    pub const fn from_static(s: &'static str) -> Text {
        Text(TextData::Static(s))
    }

    /// New text that is an owned [`String`].
    pub const fn owned(s: String) -> Text {
        Text(TextData::Owned(s))
    }

    /// New text that is a interned string or a more efficient representation.
    ///
    /// If `s` byte length is larger then the `size_of::<String>()` the string is lookup
    /// or inserted into the interned string cache.
    pub fn get_interned(s: impl AsRef<str> + Into<String>) -> Text {
        let len = s.as_ref().len();
        if len == 0 {
            Text(TextData::Static(""))
        } else if len <= INLINE_MAX {
            Text(TextData::Inline(str_to_inline(s.as_ref())))
        } else {
            Text(TextData::Interned(InternedStr::get_or_insert(s), 0, len))
        }
    }

    /// New empty text.
    pub const fn empty() -> Text {
        Self::from_static("")
    }

    /// Returns a clone of `self` that is not owned.
    pub fn to_interned(&self) -> Text {
        self.clone().into_intern()
    }

    /// Returns a clone of `self` that is not owned.
    pub fn into_intern(self) -> Text {
        let data = match self.0 {
            TextData::Owned(s) => {
                let len = s.len();
                if len == 0 {
                    TextData::Static("")
                } else if len <= INLINE_MAX {
                    TextData::Inline(str_to_inline(&s))
                } else {
                    TextData::Interned(InternedStr::get_or_insert(s), 0, len)
                }
            }
            d => d,
        };
        Text(data)
    }

    /// If the text is an owned [`String`].
    pub const fn is_owned(&self) -> bool {
        matches!(&self.0, TextData::Owned(_))
    }

    /// Acquires a mutable reference to a [`String`] buffer.
    ///
    /// Turns the text to owned if it was borrowed.
    pub fn to_mut(&mut self) -> &mut String {
        self.0 = match mem::replace(&mut self.0, TextData::Static("")) {
            TextData::Owned(s) => TextData::Owned(s),
            TextData::Static(s) => TextData::Owned(s.to_owned()),
            TextData::Inline(d) => TextData::Owned(inline_to_str(&d).to_owned()),
            TextData::Interned(a, s, l) => TextData::Owned(a.as_ref()[s..l].to_owned()),
        };

        if let TextData::Owned(s) = &mut self.0 {
            s
        } else {
            unreachable!()
        }
    }

    /// Extracts the owned string.
    ///
    /// Turns the text to owned if it was borrowed.
    pub fn into_owned(self) -> String {
        match self.0 {
            TextData::Owned(s) => s,
            TextData::Static(s) => s.to_owned(),
            TextData::Inline(d) => inline_to_str(&d).to_owned(),
            TextData::Interned(a, s, l) => a.as_ref()[s..l].to_owned(),
        }
    }

    /// Calls [`String::clear`] if the text is owned, otherwise
    /// replaces `self` with an empty str (`""`).
    pub fn clear(&mut self) {
        match &mut self.0 {
            TextData::Owned(s) => s.clear(),
            d => *d = TextData::Static(""),
        }
    }

    /// Removes the last character from the text and returns it.
    ///
    /// Returns None if this `Text` is empty.
    ///
    /// This method calls [`String::pop`] if the text is owned, otherwise
    /// reborrows a slice of the `str` without the last character.
    pub fn pop(&mut self) -> Option<char> {
        match &mut self.0 {
            TextData::Owned(s) => s.pop(),
            TextData::Static(s) => {
                if let Some((i, c)) = s.char_indices().last() {
                    *s = &s[..i];
                    Some(c)
                } else {
                    None
                }
            }
            TextData::Inline(d) => {
                let s = inline_to_str(d);
                if let Some((i, c)) = s.char_indices().last() {
                    if !s.is_empty() {
                        *d = str_to_inline(&s[..i]);
                    } else {
                        self.0 = TextData::Static("");
                    }
                    Some(c)
                } else {
                    None
                }
            }
            TextData::Interned(a, s, l) => {
                let s = &a.as_ref()[*s..*l];
                if let Some((i, c)) = s.char_indices().last() {
                    *l = i;
                    if i <= INLINE_MAX {
                        self.0 = TextData::Inline(str_to_inline(&s[..i]));
                    }
                    Some(c)
                } else {
                    None
                }
            }
        }
    }

    /// Shortens this `Text` to the specified length.
    ///
    /// If `new_len` is greater than the text's current length, this has no
    /// effect.
    ///
    /// This method calls [`String::truncate`] if the text is owned, otherwise
    /// reborrows a slice of the text.
    pub fn truncate(&mut self, new_len: usize) {
        match &mut self.0 {
            TextData::Owned(s) => s.truncate(new_len),
            TextData::Static(s) => {
                if new_len <= s.len() {
                    assert!(s.is_char_boundary(new_len));
                    *s = &s[..new_len];
                }
            }
            TextData::Inline(d) => {
                if new_len == 0 {
                    self.0 = TextData::Static("");
                } else {
                    let s = inline_to_str(d);
                    if new_len < s.len() {
                        assert!(s.is_char_boundary(new_len));
                        d[new_len..].iter_mut().for_each(|b| *b = b'\0');
                    }
                }
            }
            TextData::Interned(a, s, l) => {
                if new_len == 0 {
                    self.0 = TextData::Static("")
                } else {
                    let s = &a.as_ref()[*s..*l];
                    assert!(s.is_char_boundary(new_len));

                    if new_len > INLINE_MAX {
                        *l = new_len;
                    } else {
                        self.0 = TextData::Inline(str_to_inline(&s[..new_len]));
                    }
                }
            }
        }
    }

    /// Splits the text into two at the given index.
    ///
    /// Returns a new `Text`. `self` contains bytes `[0, at)`, and
    /// the returned `Text` contains bytes `[at, len)`. `at` must be on the
    /// boundary of a UTF-8 code point.
    ///
    /// This method calls [`String::split_off`] if the text is owned, otherwise
    /// reborrows slices of the text.
    pub fn split_off(&mut self, at: usize) -> Text {
        match &mut self.0 {
            TextData::Owned(s) => Text::owned(s.split_off(at)),
            TextData::Static(s) => {
                assert!(s.is_char_boundary(at));
                let other = &s[at..];
                *s = &s[at..];
                Text(TextData::Static(other))
            }
            TextData::Inline(d) => {
                let s = inline_to_str(d);
                assert!(s.is_char_boundary(at));
                let a_len = at;
                let b_len = s.len() - at;

                let r = Text(if b_len == 0 {
                    TextData::Static("")
                } else {
                    TextData::Inline(str_to_inline(&s[at..]))
                });

                if a_len == 0 {
                    self.0 = TextData::Static("");
                } else {
                    *d = str_to_inline(&s[..at]);
                }

                r
            }
            TextData::Interned(a, s, l) => {
                let s = &a.as_ref()[*s..*l];
                assert!(s.is_char_boundary(at));

                let a_len = at;
                let b_len = s.len() - at;

                let r = Text(if b_len == 0 {
                    TextData::Static("")
                } else if b_len <= INLINE_MAX {
                    TextData::Inline(str_to_inline(&s[at..]))
                } else {
                    TextData::Interned(a.clone(), at, b_len)
                });

                if a_len == 0 {
                    self.0 = TextData::Static("");
                } else if a_len <= INLINE_MAX {
                    self.0 = TextData::Inline(str_to_inline(&s[..at]));
                } else {
                    *l = a_len;
                }

                r
            }
        }
    }

    /// Borrow the text as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.deref()
    }
}
impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl Default for Text {
    /// Empty.
    fn default() -> Self {
        Self::empty()
    }
}
impl_from_and_into_var! {
    fn from(s: &'static str) -> Text {
        Text(TextData::Static(s))
    }
    fn from(s: String) -> Text {
        Text(TextData::Owned(s))
    }
    fn from(s: Cow<'static, str>) -> Text {
        match s {
            Cow::Borrowed(s) => Text(TextData::Static(s)),
            Cow::Owned(s) => Text(TextData::Owned(s))
        }
    }
    fn from(t: Text) -> String {
        t.into_owned()
    }
    fn from(t: Text) -> Cow<'static, str> {
        match t.0 {
            TextData::Static(s) => Cow::Borrowed(s),
            TextData::Owned(s) => Cow::Owned(s),
            TextData::Inline(d) => Cow::Owned(inline_to_str(&d).to_owned()),
            TextData::Interned(a, s, l) => Cow::Owned(a.as_ref()[s..l].to_owned()),
        }
    }
    fn from(t: Text) -> std::path::PathBuf {
        t.into_owned().into()
    }
}
impl From<Text> for Box<dyn std::error::Error> {
    fn from(err: Text) -> Self {
        err.into_owned().into()
    }
}
impl From<Text> for Box<dyn std::error::Error + Send + Sync> {
    fn from(err: Text) -> Self {
        err.into_owned().into()
    }
}
impl std::ops::Deref for Text {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl std::borrow::Borrow<str> for Text {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}
impl<'a> std::ops::Add<&'a str> for Text {
    type Output = Text;

    fn add(mut self, rhs: &'a str) -> Self::Output {
        self += rhs;
        self
    }
}
impl std::ops::AddAssign<&str> for Text {
    fn add_assign(&mut self, rhs: &str) {
        self.to_mut().push_str(rhs);
    }
}
impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq(*other)
    }
}
impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}
impl PartialEq<String> for Text {
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other)
    }
}
impl PartialEq<Text> for &str {
    fn eq(&self, other: &Text) -> bool {
        other.as_str().eq(*self)
    }
}
impl PartialEq<Text> for str {
    fn eq(&self, other: &Text) -> bool {
        other.as_str().eq(self)
    }
}
impl PartialEq<Text> for String {
    fn eq(&self, other: &Text) -> bool {
        other.as_str().eq(self)
    }
}

/// A trait for converting a value to a [`Text`].
///
/// This trait is automatically implemented for any type which implements the [`ToString`] trait.
///
/// You can use [`formatx!`](macro.formatx.html) to `format!` a text.
pub trait ToText {
    /// Converts the given value to an owned [`Text`].
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```
    /// use zero_ui_core::text::*;
    ///
    /// let expected = formatx!("10");
    /// let actual = 10.to_text();
    ///
    /// assert_eq!(expected, actual);
    /// ```
    fn to_text(&self) -> Text;
}
impl<T: ToString> ToText for T {
    fn to_text(&self) -> Text {
        self.to_string().into()
    }
}

pub use crate::render::FontSynthesis;

/// An offset in a text.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct TextPoint {
    /// Line index, 0 based.
    pub line: usize,
    /// Byte index in the line text. The byte is in a [char boundary](str::is_char_boundary) and is 0 based.
    pub index: usize,
}
impl TextPoint {
    /// New text point.
    #[inline]
    pub fn new(line: usize, index: usize) -> Self {
        TextPoint { line, index }
    }

    /// Compute a [`TextPointDisplay`] given the `line` that is pointed by `self`.
    ///
    /// The raw text point is not what a user expects, the first line is `0` and the *column* is a byte count not a character count.
    /// The return value can be displayed as a *Ln 1, Col 1* label.
    ///
    /// The input is the [`line`](Self::line) pointed by `self`, this method **panics** if the `line` length cannot accommodate
    /// the byte [`index`](Self::index).
    #[inline]
    pub fn display(self, line: &str) -> TextPointDisplay {
        TextPointDisplay::new(line, self)
    }
}

/// *Ln 1, Col 1* display info of a [`TextPoint`].
///
/// You can compute this value from [`TextPoint::display`].
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct TextPointDisplay {
    /// Line number, 1 based.
    pub line: usize,
    /// Character number, 1 based.
    pub column: usize,
}
impl TextPointDisplay {
    #[inline]
    fn new(line: &str, point: TextPoint) -> Self {
        TextPointDisplay {
            line: point.line + 1,
            column: line[0..point.index].chars().count(),
        }
    }
}
impl fmt::Display for TextPointDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ln {}, Col {}", self.line, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segmented_text1() {
        let t = SegmentedText::new("foo \n\nbar\n");

        use TextSegmentKind::*;
        let expected = vec![
            ("foo", Word),
            (" ", Space),
            ("\n", LineBreak),
            ("\n", LineBreak),
            ("bar", Word),
            ("\n", LineBreak),
        ];
        let actual: Vec<_> = t.iter().collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }
    #[test]
    fn segmented_text2() {
        let t = SegmentedText::new("baz  \r\n\r\n  fa".to_owned());

        use TextSegmentKind::*;
        let expected = vec![
            ("baz", Word),
            ("  ", Space),
            ("\r\n", LineBreak),
            ("\r\n", LineBreak),
            ("  ", Space),
            ("fa", Word),
        ];
        let actual: Vec<_> = t.iter().collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }
    #[test]
    fn segmented_text3() {
        let t = SegmentedText::new("\u{200B}	");

        use TextSegmentKind::*;
        let expected = vec![("\u{200B}", Word), ("\t", Tab)];
        let actual: Vec<_> = t.iter().collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn segmented_text4() {
        let t = SegmentedText::new("move to 0x0");

        use TextSegmentKind::*;
        let expected = vec![("move", Word), (" ", Space), ("to", Word), (" ", Space), ("0x0", Word)];
        let actual: Vec<_> = t.iter().collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }
}

///<span data-inline></span> Creates a [`Text`](crate::text::Text) by calling the `format!` macro and
/// wrapping the result in a `Cow::Owned`.
///
/// # Example
/// ```
/// # use zero_ui_core::text::formatx;
/// let text = formatx!("Hello {}", "World!");
/// ```
#[macro_export]
macro_rules! formatx {
    ($str:tt) => {
        $crate::text::Text::from_static($str)
    };
    ($($tt:tt)*) => {
        $crate::text::Text::owned(format!($($tt)*))
    };
}
#[doc(inline)]
pub use crate::formatx;
use crate::var::{IntoVar, OwnedVar};

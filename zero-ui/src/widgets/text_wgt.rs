use crate::prelude::new_widget::*;

pub mod nodes;
mod text_properties;

/// A configured text run.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::text;
///
/// let hello_txt = text! {
///     font_family = "Arial";
///     font_size = 18;
///     txt = "Hello!";
/// };
/// ```
/// # Shorthand
///
/// The `text!` macro provides shorthand syntax that matches the [`formatx!`] input, but outputs a text widget:
///
/// ```
/// # use zero_ui::prelude::text;
/// let txt = text!("Hello!");
///
/// let name = "World";
/// let fmt = text!("Hello {}!", name);
///
/// let expr = text!({
///     let mut s = String::new();
///     s.push('a');
///     s
/// });
/// ```
///
/// The code abode is equivalent to:
///
/// ```
/// # use zero_ui::prelude::text;
/// let txt = text! {
///     txt = zero_ui::core::text::formatx!("Hello!");
/// };
///
/// let name = "World";
/// let fmt = text! {
///     txt = zero_ui::core::text::formatx!("Hello {}!", name);
/// };
///
/// let expr = text! {
///     txt = {
///         let mut s = String::new();
///         s.push('a');
///         s
///     };
/// };
/// ```
///
/// [`formatx!`]: crate::core::text::formatx!
#[widget($crate::widgets::text {
    ($txt:literal) => {
        txt = $crate::core::text::formatx!($txt);
    };
    ($txt:expr) => {
        txt = $txt;
    };
    ($txt:tt, $($format:tt)*) => {
        txt = $crate::core::text::formatx!($txt, $($format)*);
    };
})]
pub mod text {
    use crate::prelude::new_widget::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use super::nodes;

    #[doc(inline)]
    pub use super::text_properties::{
        caret_color, font_aa, font_annotation, font_caps, font_char_variant, font_cn_variant, font_common_lig, font_contextual_alt,
        font_discretionary_lig, font_ea_width, font_family, font_features, font_historical_forms, font_historical_lig, font_jp_variant,
        font_kerning, font_num_fraction, font_num_spacing, font_numeric, font_ornaments, font_position, font_size, font_stretch,
        font_style, font_style_set, font_stylistic, font_swash, font_synthesis, font_variations, font_weight, hyphen_char, hyphens, lang,
        letter_spacing, line_break, line_height, line_spacing, overline, overline_color, paragraph_spacing, strikethrough,
        strikethrough_color, tab_length, txt_align, txt_color, txt_editable, txt_transform, txt_wrap, underline, underline_color,
        underline_position, underline_skip, white_space, word_break, word_spacing, *,
    };

    properties! {
        /// The text string.
        ///
        /// Set to an empty string (`""`) by default.
        pub txt(impl IntoVar<Text>) = "";

        /// Spacing in-between the text and borders.
        ///
        pub crate::properties::padding;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let child = nodes::render_text();
            let child = nodes::render_caret(child);
            let child = nodes::render_overlines(child);
            let child = nodes::render_strikethroughs(child);
            let child = nodes::render_underlines(child);
            wgt.set_child(child.boxed());

            wgt.push_intrinsic(NestGroup::CHILD_LAYOUT + 100, "layout_text", nodes::layout_text);

            let text = wgt.capture_var_or_default(property_id!(self::txt));
            wgt.push_intrinsic(NestGroup::EVENT, "resolve_text", |child| nodes::resolve_text(child, text));
        });
    }
}

///<span data-del-macro-root></span> A simple text run with **bold** font weight.
///
/// The input syntax is the same as the shorthand [`text!`].
///
/// # Configure
///
/// Apart from the font weight this widget can be configured with contextual properties like [`text!`].
///
/// [`text`]: mod@text
#[macro_export]
macro_rules! strong {
    ($txt:expr) => {
        $crate::widgets::text! {
            txt = $txt;
            font_weight = $crate::core::text::FontWeight::BOLD;
        }
    };
    ($txt:tt, $($format:tt)*) => {
        $crate::widgets::text! {
            txt = $crate::core::text::formatx!($txt, $($format)*);
            font_weight = $crate::core::text::FontWeight::BOLD;
        }
    };
}
#[doc(inline)]
pub use strong;

///<span data-del-macro-root></span> A simple text run with *italic* font style.
///
/// The input syntax is the same as the shorthand [`text!`].
///
/// # Configure
///
/// Apart from the font style this widget can be configured with contextual properties like [`text!`].
///
/// [`text`]: mod@text
#[macro_export]
macro_rules! em {
    ($txt:expr) => {
        $crate::widgets::text! {
            txt = $txt;
            font_style = FontStyle::Italic;
        }
    };
    ($txt:tt, $($format:tt)*) => {
        $crate::widgets::text! {
            txt = $crate::core::text::formatx!($txt, $($format)*);
            font_style = FontStyle::Italic;
        }
    };
}
#[doc(inline)]
pub use em;

/// Text box widget.
#[widget($crate::widgets::text_input)]
pub mod text_input {
    use super::*;

    inherit!(super::text);
    inherit!(style_mixin);

    properties! {
        /// Enabled by default.
        txt_editable = true;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the text input is pressed.
        capture_mouse = true;

        /// Enables keyboard focusing in the widget.
        focusable = true;

        /// Style generator used for the widget.
        ///
        /// Set to [`vis::STYLE_VAR`] by default, setting this property directly completely replaces the text input style,
        /// see [`vis::replace_style`] and [`vis::extend_style`] for other ways of modifying the style.
        style_gen = vis::STYLE_VAR;
    }

    #[doc(inline)]
    pub use super::text_input_vis as vis;
}

/// Text input style, visual properties and context vars.
pub mod text_input_vis {
    use super::*;

    context_var! {
        /// Text input style in a context.
        ///
        /// Is the [`default_style!`] by default.
        ///
        /// [`default_style!`]: mod@default_style
        pub static STYLE_VAR: StyleGenerator = StyleGenerator::new(|_, _| default_style!());

        /// Idle background dark and light color.
        pub static BASE_COLORS_VAR: ColorPair = (rgb(0.12, 0.12, 0.12), rgb(0.88, 0.88, 0.88));
    }

    /// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the text input style.
    #[property(CONTEXT, default(BASE_COLORS_VAR))]
    pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
        with_context_var(child, BASE_COLORS_VAR, color)
    }

    /// Sets the text input style in a context, the parent style is fully replaced.
    #[property(CONTEXT, default(STYLE_VAR))]
    pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        with_context_var(child, STYLE_VAR, style)
    }

    /// Extends the text input style in a context, the parent style is used, properties of the same name set in
    /// `style` override the parent style.
    #[property(CONTEXT, default(StyleGenerator::nil()))]
    pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleGenerator>) -> impl UiNode {
        style_mixin::with_style_extension(child, STYLE_VAR, style)
    }

    /// Default border color.
    pub fn border_color() -> impl Var<Rgba> {
        color_scheme_highlight(BASE_COLORS_VAR, 0.20)
    }

    /// Border color hovered.
    pub fn border_color_hovered() -> impl Var<Rgba> {
        color_scheme_highlight(BASE_COLORS_VAR, 0.30)
    }

    /// Border color focused.
    pub fn border_color_focused() -> impl Var<Rgba> {
        color_scheme_highlight(BASE_COLORS_VAR, 0.40)
    }

    /// Text input default style.
    #[widget($crate::widgets::text_input::vis::default_style)]
    pub mod default_style {
        use super::*;

        inherit!(style);

        properties! {
            /// Text padding.
            ///
            /// Is `(7, 15)` by default.
            pub crate::widgets::text::padding = (7, 15);

            /// Text cursor.
            pub crate::properties::cursor = CursorIcon::Text;

            /// Caret color.
            pub text_properties::caret_color;

            /// Text input base dark and light colors.
            ///
            /// All other text input style colors are derived from this pair.
            pub super::base_colors;

            /// Text input background color.
            pub crate::properties::background_color = color_scheme_pair(BASE_COLORS_VAR);

            /// Text input border.
            pub crate::properties::border = {
                widths: 1,
                sides: border_color().map_into(),
            };

            /// When the pointer device is over this text input or it is the return focus.
            when *#is_cap_hovered || *#is_return_focus {
                border = {
                    widths: 1,
                    sides: border_color_hovered().map_into(),
                };
            }

            /// When the text input has keyboard focus.
            when *#is_focused {
                border = {
                    widths: 1,
                    sides: border_color_focused().map_into(),
                };
            }

            /// When the text input is disabled.
            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}

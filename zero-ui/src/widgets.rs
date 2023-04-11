//! Common widgets.

pub mod layouts;

pub mod focusable;

mod ansi_text_wgt;
#[doc(inline)]
pub use ansi_text_wgt::ansi_text;

pub mod button;
#[doc(inline)]
pub use button::Button;

pub mod checkerboard;
#[doc(inline)]
pub use checkerboard::Checkerboard;

mod container;
#[doc(inline)]
pub use container::Container;

mod flood;
#[doc(inline)]
pub use flood::flood;

mod gradient;
#[doc(inline)]
pub use gradient::{
    conic_gradient, conic_gradient_ext, conic_gradient_full, linear_gradient, linear_gradient_ext, linear_gradient_full, radial_gradient,
    radial_gradient_ext, radial_gradient_full, reflecting_conic_gradient, reflecting_linear_gradient, reflecting_radial_gradient,
    repeating_conic_gradient, repeating_linear_gradient, repeating_radial_gradient,
};

mod image_wgt;
#[doc(inline)]
pub use image_wgt::image;

mod icon_wgt;
#[doc(inline)]
pub use icon_wgt::icon;

mod link_wgt;
#[doc(inline)]
pub use link_wgt::link;

mod markdown_wgt;
#[doc(inline)]
pub use markdown_wgt::markdown;

mod rule_line_wgt;
#[doc(inline)]
pub use rule_line_wgt::{hr, rule_line};

mod scroll_wgt;
#[doc(inline)]
pub use scroll_wgt::scroll;

mod switch_wgt;
#[doc(inline)]
pub use switch_wgt::switch;

mod text_wgt;
#[doc(inline)]
pub use text_wgt::{em, strong, text, text_input};

mod tip_wgt;
#[doc(inline)]
pub use tip_wgt::*;

mod toggle_wgt;
#[doc(inline)]
pub use toggle_wgt::toggle;

pub mod style;
#[doc(inline)]
pub use style::Style;

mod view;
#[doc(inline)]
pub use view::*;

mod window_wgt;
#[doc(inline)]
pub use window_wgt::window;

/// Minimal widget.
///
/// You can use this to create a quick new custom widget that is only used in one code place and can be created entirely
/// by properties and `when` conditions.
#[crate::core::widget($crate::widgets::Wgt)]
pub struct Wgt(crate::core::widget_base::WidgetBase);

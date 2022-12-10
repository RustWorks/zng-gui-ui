//! Common widgets.

pub mod layouts;
pub mod mixins;

mod ansi_text_wgt;
#[doc(inline)]
pub use ansi_text_wgt::ansi_text;

mod button_wgt;
#[doc(inline)]
pub use button_wgt::button;

mod checkerboard_wgt;
#[doc(inline)]
pub use checkerboard_wgt::checkerboard;

mod container_wgt;
#[doc(inline)]
pub use container_wgt::container;

mod flood;
#[doc(inline)]
pub use flood::flood;

mod gradient;
#[doc(inline)]
pub use gradient::{linear_gradient, linear_gradient_ext, linear_gradient_full, reflecting_linear_gradient, repeating_linear_gradient};

mod image_wgt;
#[doc(inline)]
pub use image_wgt::image;

mod icon_wgt;
#[doc(inline)]
pub use icon_wgt::icon;

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

mod toggle_wgt;
#[doc(inline)]
pub use toggle_wgt::{checkbox, toggle};

mod style_wgt;
#[doc(inline)]
pub use style_wgt::style;

mod view;
#[doc(inline)]
pub use view::*;

mod window_wgt;
#[doc(inline)]
pub use window_wgt::window;

/// A widget with only the implicit properties.
///
/// You can use this to shape a custom widget that will only
/// be used once. Instead of declaring a new widget type.
#[crate::core::widget($crate::widgets::blank)]
pub mod blank {
    inherit!(::zero_ui_core::widget_base::base);
}

//! Common widget properties.

pub mod inspector;

mod layout;
#[doc(inline)]
pub use layout::*;

mod visual;
#[doc(inline)]
pub use visual::*;

mod border_;
#[doc(inline)]
pub use border_::*;

pub mod commands;
pub mod events;
pub mod filters;
pub mod focus;
pub mod states;
pub mod transform;

mod capture;
#[doc(inline)]
pub use capture::*;

mod cursor_;
#[doc(inline)]
pub use cursor_::*;

mod lazy_;
#[doc(inline)]
pub use lazy_::*;

#[doc(inline)]
pub use crate::core::widget_base::{hit_test_mode, interactive};

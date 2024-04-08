#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
//!
//! Contextual layout units.
//!
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

pub mod context;
pub mod unit;

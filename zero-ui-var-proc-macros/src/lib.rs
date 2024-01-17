#![doc = include_str!("../../zero-ui-app/README.md")]
//!
//! Proc-macros for the `zero-ui-var` crate.
//!
//! Don't use directly.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use proc_macro::TokenStream;

#[macro_use]
extern crate quote;

#[macro_use]
mod util;

mod expr_var;
mod merge_var;
mod transitionable;
mod when_var;

/// Implement transition by delegating all type parts.
#[proc_macro_derive(Transitionable)]
pub fn transitionable(args: TokenStream) -> TokenStream {
    transitionable::expand(args)
}

#[doc(hidden)]
#[proc_macro]
pub fn expr_var(input: TokenStream) -> TokenStream {
    expr_var::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn when_var(input: TokenStream) -> TokenStream {
    when_var::expand(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn merge_var(input: TokenStream) -> TokenStream {
    merge_var::expand(input)
}

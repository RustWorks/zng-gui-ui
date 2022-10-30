use std::mem;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::{parse::Parse, spanned::Spanned, *};

use crate::{
    util::{self, parse_outer_attrs, path_span, ErrorRecoverable, Errors},
    widget_util::{self, WgtProperty, WgtWhen},
};

pub fn expand(args: proc_macro::TokenStream, input: proc_macro::TokenStream, mixin: bool) -> proc_macro::TokenStream {
    // the widget mod declaration.
    let mod_ = parse_macro_input!(input as ItemMod);

    if mod_.content.is_none() {
        let mut r = syn::Error::new(mod_.semi.span(), "only modules with inline content are supported")
            .to_compile_error()
            .to_token_stream();

        mod_.to_tokens(&mut r);

        return r.into();
    }

    let (mod_braces, items) = mod_.content.unwrap();

    // accumulate the most errors as possible before returning.
    let mut errors = Errors::default();

    let crate_core = util::crate_core();

    let vis = mod_.vis;
    let ident = mod_.ident;
    let mod_token = mod_.mod_token;
    let attrs = mod_.attrs;

    if mixin && !ident.to_string().ends_with("_mixin") {
        errors.push("mix-in names must end with suffix `_mixin`", ident.span());
    }

    // a `$crate` path to the widget module.
    let mod_path = match syn::parse::<ArgPath>(args) {
        Ok(a) => a.path,
        Err(e) => {
            errors.push_syn(e);
            quote! { $crate::missing_widget_path}
        }
    };
    let mod_path_str = mod_path.to_string();
    let mod_path_slug = mod_path_slug(&mod_path_str);

    let WidgetItems {
        uses,
        inherits,
        mut properties,
        include_fn,
        build_fn,
        others,
    } = WidgetItems::new(items, &mut errors);

    let mut include_item_imports = quote!();

    let mut has_parent = false;

    for inh in &inherits {
        let is_parent = !inh.has_mixin_suffix();
        if has_parent && is_parent {
            errors.push("can only inherit from one widget and multiple mix-ins", inh.path.span());
            continue;
        }

        has_parent |= is_parent;
        let attrs = &inh.attrs;
        let path = &inh.path;
        include_item_imports.extend(quote_spanned! {path_span(path)=>
            #(#attrs)*
            #path::__include__(__wgt__);
        });
    }

    if let Some(int) = &include_fn {
        include_item_imports.extend(quote_spanned! {int.span()=>
            self::include(__wgt__);
        })
    }

    let mut capture_decl = quote!();
    let mut pre_bind = quote!();

    for prop in properties.iter_mut().flat_map(|i| i.properties.iter_mut()) {
        capture_decl.extend(prop.declare_capture(&mod_path_slug));
        pre_bind.extend(prop.pre_bind_args(false, None, ""));
    }
    for (i, when) in properties.iter_mut().flat_map(|i| i.whens.iter_mut()).enumerate() {
        pre_bind.extend(when.pre_bind(false, i));
    }

    let mut include_items = quote!();

    for prop in properties.iter().flat_map(|i| i.properties.iter()) {
        if prop.has_args() {
            let cfg = &prop.attrs.cfg;
            let lints = &prop.attrs.lints;
            let args = prop.args_new(quote!(#crate_core::widget_builder));
            include_items.extend(quote! {
                #cfg
                #(#lints)*
                __wgt__.push_property(#crate_core::widget_builder::Importance::WIDGET, #args);
            });
        } else if prop.is_unset() {
            let cfg = &prop.attrs.cfg;
            let id = prop.property_id();
            include_items.extend(quote! {
                #cfg
                __wgt__.push_unset(#crate_core::widget_builder::Importance::WIDGET, #id);
            });
        }
    }

    for when in properties.iter().flat_map(|i| i.whens.iter()) {
        let cfg = &when.attrs.cfg;
        let lints = &when.attrs.lints;
        let args = when.when_new(quote!(#crate_core::widget_builder));
        include_items.extend(quote! {
            #cfg
            #(#lints)*
            __wgt__.push_when(#crate_core::widget_builder::Importance::WIDGET, #args);
        });
    }

    let macro_if_mixin = if mixin {
        quote! {
            (>> if mixin { $($tt:tt)* }) => {
                $($tt)*
            };
            (>> if !mixin { $($tt:tt)* }) => {
                // ignore
            };
        }
    } else {
        quote! {
            (>> if !mixin { $($tt:tt)* }) => {
                $($tt)*
            };
            (>> if mixin { $($tt:tt)* }) => {
                // ignore
            };
        }
    };

    let build = if mixin {
        if let Some(build) = &build_fn {
            errors.push("mix-ins cannot have a build function", build.sig.ident.span());
        }
        quote!()
    } else if let Some(build) = &build_fn {
        let out = &build.sig.output;
        let ident = &build.sig.ident;
        quote_spanned! {build.span()=>
            #[doc(hidden)]
            pub fn __build__(__wgt__: #crate_core::widget_builder::WidgetBuilder) #out {
                self::#ident(__wgt__)
            }
        }
    } else if let Some(inh) = inherits.iter().find(|m| !m.has_mixin_suffix()) {
        let path = &inh.path;
        let id = path.segments.last().map(|s| &s.ident).unwrap();
        let error = format!("cannot inherit build from `{id}`, it is a mix-in\nmix-ins with suffix `_mixin` are ignored when inheriting build, but this one was renamed");
        quote_spanned! {path_span(path)=>
            #path! {
                >> if mixin {
                    std::compile_error!{ #error }
                }
            }
            #path! {
                >> if !mixin {
                    #[doc(hidden)]
                    #[allow(unused_imports)]
                    pub use #path::__build__;
                }
            }
        }
    } else {
        errors.push(
            "missing `fn build(WidgetBuilder) -> T` function, must be provided or inherited",
            ident.span(),
        );
        quote! {
            #[doc(hidden)]
            pub fn __build__(_: #crate_core::widget_builder::WidgetBuilder) -> #crate_core::widget_instance::NilUiNode {
                #crate_core::widget_instance::NilUiNode
            }
        }
    };
    let build_final_export = if mixin {
        quote!()
    } else {
        quote! {
            pub use super::__build__ as build;
        }
    };

    let mut inherit_export = quote!();

    for Inherit { attrs, path } in inherits {
        let extra_super = if path.segments[0].ident == "super" {
            let sup = &path.segments[0].ident;
            quote_spanned!(sup.span()=> #sup::)
        } else {
            quote!()
        };
        inherit_export.extend(quote_spanned! {path_span(&path)=>
            #(#attrs)*
            #[allow(unused_imports)]
            pub use #extra_super #path::__properties__::*;
        });
    }
    for p in properties.iter().flat_map(|p| p.properties.iter()) {
        inherit_export.extend(p.reexport());
    }

    let macro_ident = ident!("__wgt_{}__", mod_path_slug);

    let mod_items = quote! {
        // custom items
        #(#others)*

        // use items (after custom items in case of custom macro_rules re-export)
        #(#uses)*

        #[doc(hidden)]
        #[allow(unused_imports)]
        pub mod __properties__ {
            use super::*;

            #inherit_export
        }

        #include_fn

        #[doc(hidden)]
        pub fn __include__(__wgt__: &mut #crate_core::widget_builder::WidgetBuilder) {
            #include_item_imports
            #pre_bind
            {
                use self::__properties__::*;
                #include_items
            }
        }

        #capture_decl
        #build_fn
        #build

        #[doc(hidden)]
        #[allow(unused_imports)]
        pub mod __widget__ {
            pub use #crate_core::{widget_new, widget_builder};

            pub use super::__include__ as include;
            #build_final_export

            pub fn mod_info() -> widget_builder::WidgetMod {
                static impl_id: widget_builder::StaticWidgetImplId = widget_builder::StaticWidgetImplId::new_unique();

                widget_builder::WidgetMod {
                    impl_id: impl_id.get(),
                    path: #mod_path_str,
                    location: widget_builder::source_location!(),
                }
            }

            pub fn new() -> widget_builder::WidgetBuilder {
                let mut wgt = widget_builder::WidgetBuilder::new(mod_info());
                include(&mut wgt);
                wgt
            }
        }
    };

    let mut mod_block = quote!();
    mod_braces.surround(&mut mod_block, |t| t.extend(mod_items));

    // rust-analyzer does not find the macro if we don't set the call_site here.
    let mod_path = util::set_stream_span(mod_path, Span::call_site());

    let r = quote! {
        #(#attrs)*
        #vis #mod_token #ident #mod_block

        #[doc(hidden)]
        #[macro_export]
        macro_rules! #macro_ident {
            #macro_if_mixin

            ($($tt:tt)*) => {
                #mod_path::__widget__::widget_new! {
                    widget { #mod_path }
                    new { $($tt)* }
                }
            };
        }
        #[doc(hidden)]
        #[allow(unused_imports)]
        #vis use #macro_ident as #ident;

        #errors
    };
    r.into()
}

struct ArgPath {
    path: TokenStream,
}
impl Parse for ArgPath {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        let fork = input.fork();
        match (fork.parse::<Token![$]>(), fork.parse::<syn::Path>()) {
            (Ok(_), Ok(p)) => {
                if fork.is_empty() {
                    if p.segments[0].ident == "crate" {
                        Ok(ArgPath {
                            path: input.parse().unwrap(),
                        })
                    } else {
                        Err(syn::Error::new(p.segments[0].ident.span(), "expected `crate`"))
                    }
                } else {
                    Err(syn::Error::new(fork.span(), "unexpected token"))
                }
            }
            (Ok(_), Err(e)) => {
                if !util::span_is_call_site(e.span()) {
                    Err(e)
                } else {
                    Err(syn::Error::new(util::last_span(input.parse().unwrap()), e.to_string()))
                }
            }
            _ => Err(syn::Error::new(
                input.span(),
                "expected a macro_rules `$crate` path to this widget mod",
            )),
        }
    }
}

struct WidgetItems {
    uses: Vec<ItemUse>,
    inherits: Vec<Inherit>,
    properties: Vec<Properties>,
    include_fn: Option<ItemFn>,
    build_fn: Option<ItemFn>,
    others: Vec<Item>,
}
impl WidgetItems {
    fn new(items: Vec<Item>, errors: &mut Errors) -> Self {
        let mut uses = vec![];
        let mut inherits = vec![];
        let mut properties = vec![];
        let mut include_fn = None;
        let mut build_fn = None;
        let mut others = vec![];

        for item in items {
            match item {
                Item::Use(use_) => {
                    uses.push(use_);
                }
                // match properties!
                Item::Macro(ItemMacro { mac, ident: None, .. }) if mac.path.get_ident().map(|i| i == "properties").unwrap_or(false) => {
                    match syn::parse2::<Properties>(mac.tokens) {
                        Ok(mut p) => {
                            errors.extend(mem::take(&mut p.errors));
                            properties.push(p)
                        }
                        Err(e) => errors.push_syn(e),
                    }
                }
                // match inherit!
                Item::Macro(ItemMacro {
                    mac, attrs, ident: None, ..
                }) if mac.path.get_ident().map(|i| i == "inherit").unwrap_or(false) => match parse2::<Inherit>(mac.tokens) {
                    Ok(mut ps) => {
                        ps.attrs.extend(attrs);
                        inherits.push(ps)
                    }
                    Err(e) => errors.push_syn(e),
                },

                // match fn include(..)
                Item::Fn(fn_) if fn_.sig.ident == "include" => {
                    include_fn = Some(fn_);
                }
                // match fn build(..)
                Item::Fn(fn_) if fn_.sig.ident == "build" => {
                    build_fn = Some(fn_);
                }
                // other user items.
                item => others.push(item),
            }
        }

        WidgetItems {
            uses,
            inherits,
            properties,
            include_fn,
            build_fn,
            others,
        }
    }
}

struct Inherit {
    attrs: Vec<Attribute>,
    path: Path,
}
impl Inherit {
    fn has_mixin_suffix(&self) -> bool {
        self.path
            .segments
            .last()
            .map(|s| s.ident.to_string().ends_with("_mixin"))
            .unwrap_or(false)
    }
}
impl Parse for Inherit {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        Ok(Inherit {
            attrs: vec![],
            path: input.parse()?,
        })
    }
}

struct Properties {
    errors: Errors,
    properties: Vec<WgtProperty>,
    whens: Vec<WgtWhen>,
}
impl Parse for Properties {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        let mut errors = Errors::default();
        let mut properties = vec![];
        let mut whens = vec![];

        while !input.is_empty() {
            let attrs = parse_outer_attrs(input, &mut errors);

            if input.peek(widget_util::keyword::when) {
                if let Some(mut when) = WgtWhen::parse(input, &mut errors) {
                    when.attrs = util::Attributes::new(attrs);
                    whens.push(when);
                }
            } else if input.peek(Token![pub])
                || input.peek(Ident)
                || input.peek(Token![crate])
                || input.peek(Token![super])
                || input.peek(Token![self])
            {
                // peek ident or path (including keywords because of super:: and self::). {
                match input.parse::<WgtProperty>() {
                    Ok(mut p) => {
                        p.attrs = util::Attributes::new(attrs);
                        if !input.is_empty() && p.semi.is_none() {
                            errors.push("expected `;`", input.span());
                            while !(input.is_empty()
                                || input.peek(Ident)
                                || input.peek(Token![crate])
                                || input.peek(Token![super])
                                || input.peek(Token![self])
                                || input.peek(Token![#]) && input.peek(token::Bracket))
                            {
                                // skip to next value item.
                                let _ = input.parse::<TokenTree>();
                            }
                        }
                        properties.push(p);
                    }
                    Err(e) => {
                        let (recoverable, e) = e.recoverable();
                        if recoverable {
                            errors.push_syn(e);
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else {
                errors.push("expected `when` or a property declaration", input.span());

                // suppress the "unexpected token" error from syn parse.
                let _ = input.parse::<TokenStream>();
            }
        }

        Ok(Properties { errors, properties, whens })
    }
}

fn mod_path_slug(path: &str) -> String {
    path.replace("crate", "").replace(':', "").replace('$', "").trim().replace(' ', "_")
}

/*
    NEW
*/

pub fn expand_new(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let NewArgs { widget, properties: mut p } = parse_macro_input!(args as NewArgs);

    let mut pre_bind = quote!();
    for prop in &mut p.properties {
        pre_bind.extend(prop.pre_bind_args(true, None, ""));

        if !matches!(&prop.vis, Visibility::Inherited) {
            p.errors.push("cannot reexport property from instance", prop.vis.span());
        }
    }
    for (i, when) in p.whens.iter_mut().enumerate() {
        pre_bind.extend(when.pre_bind(true, i));
    }

    let mut init = quote!();
    for p in &p.properties {
        let cfg = &p.attrs.cfg;
        if p.is_unset() {
            let id = p.property_id();
            init.extend(quote! {
                #cfg
                __wgt__.push_unset(#widget::__widget__::widget_builder::Importance::INSTANCE, #id);
            });
        } else {
            let args = p.args_new(quote!(#widget::__widget__::widget_builder));
            init.extend(quote! {
                #cfg
                __wgt__.push_property(#widget::__widget__::widget_builder::Importance::INSTANCE, #args);
            });
        }
    }

    for w in &p.whens {
        let cfg = &w.attrs.cfg;
        let args = w.when_new(quote!(#widget::__widget__::widget_builder));
        init.extend(quote! {
            #cfg
            __wgt__.push_when(#widget::__widget__::widget_builder::Importance::INSTANCE, #args);
        });
    }

    p.errors.to_tokens(&mut init);

    let r = quote! {
        {
            #pre_bind

            let mut __wgt__ = #widget::__widget__::new();
            {
                #[allow(unused_imports)]
                use #widget::__properties__::*;
                #init
            }
            #widget::__widget__::build(__wgt__)
        }
    };

    r.into()
}

struct NewArgs {
    widget: TokenStream,
    properties: Properties,
}
impl Parse for NewArgs {
    fn parse(input: parse::ParseStream) -> Result<Self> {
        Ok(Self {
            widget: non_user_braced!(input, "widget").parse().unwrap(),
            properties: non_user_braced!(input, "new").parse()?,
        })
    }
}
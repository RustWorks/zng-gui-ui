use crate::util;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::HashSet;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::*;

pub(crate) fn gen_ui_node(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = match syn::parse::<ItemImpl>(input.clone()) {
        Ok(i) => i,
        Err(e) => {
            // in case of major parsing error, like item not being an impl block. We give the args error
            // but do not remove the function.
            let mut r = proc_macro::TokenStream::from(e.to_compile_error());
            r.extend(input);
            return r;
        }
    };
    let args = parse_macro_input!(args as Args);

    let crate_ = util::crate_core();

    let mut ui_node_path = None;

    let ui_node = ident!("UiNode");

    // if the impl block has a trait path.
    if let Some((_, path, _)) = input.trait_ {
        // if the trait name is UiNode
        let seg = path.segments.last().unwrap();
        if seg.ident == ui_node {
            ui_node_path = Some(path);
        }
        // if the impl block is for a trait not named UiNode
        else {
            abort!(
                path.span(),
                "expected inherent impl or `UiNode` trait impl, found `{}`",
                quote! {#path}
            )
        }
    }

    let mut node_items = vec![];
    let mut node_items_missing_del_level = vec![];
    let mut other_items = vec![];
    let mut node_item_names = HashSet::new();

    let mut errors = util::Errors::default();

    // impl scope custom lints:
    //
    // we only have one lint, `zero_ui::missing_delegate`.
    let missing_delegate_level =
        take_missing_deletate_level(&mut input.attrs, &mut errors, &HashSet::new()).unwrap_or(util::LintLevel::Deny);
    let missing_delegate_ident = ident!("missing_delegate");
    let mut forbidden_lints = HashSet::new();
    if let util::LintLevel::Forbid = missing_delegate_level {
        forbidden_lints.insert(&missing_delegate_ident);
    }
    let forbidden_lints = forbidden_lints;

    for mut item in input.items {
        let mut is_node = false;

        if let ImplItem::Fn(m) = &mut item {
            // if we are in an UiNode impl
            if ui_node_path.is_some() {
                // assume the item is a method defined in the UiNode trait.
                is_node = true;
                node_item_names.insert(m.sig.ident.clone());
            }
            // if we are not in an UiNode impl but a method is annotated with `#[UiNode]`.
            else if let Some(index) = m.attrs.iter().position(|a| a.path().get_ident() == Some(&ui_node)) {
                // remove the marker attribute..
                m.attrs.remove(index);
                // ..and assume the item is a method defined in the UiNode trait.
                is_node = true;
                node_item_names.insert(m.sig.ident.clone());
            }

            if is_node {
                let item_level = take_missing_deletate_level(&mut m.attrs, &mut errors, &forbidden_lints).unwrap_or(missing_delegate_level);
                node_items_missing_del_level.push(item_level);
            }
        }

        if is_node {
            node_items.push(item);
        } else {
            other_items.push(item);
        }
    }

    let mut validate_manual_delegate = true;

    // validate layout/measure pair
    if let Some(layout) = &node_item_names.get(&ident!("layout")) {
        if !node_item_names.contains(&ident!("measure")) {
            errors.push(
                "`layout` is manual impl, but `measure` is auto impl, both must be auto or manual",
                layout.span(),
            );
        }
    } else if let Some(measure) = &node_item_names.get(&ident!("measure")) {
        if !node_item_names.contains(&ident!("measure")) {
            errors.push(
                "`measure` is manual impl, but `layout` is auto impl, both must be auto or manual",
                measure.span(),
            );
        }
    }

    let (new_node, args) = match args {
        Args::NewNode(args) => {
            let new_node = expand_new_node(args, &mut errors);
            let args = syn::parse2::<Args>(new_node.delegate.to_token_stream()).unwrap();
            (Some(new_node), args)
        }
        a => (None, a),
    };

    let auto_subs = if new_node.as_ref().map(|n| !n.auto_subs.is_empty()).unwrap_or(false) {
        quote!(self.auto_subs();)
    } else {
        quote!()
    };
    let validate_auto_subs = !auto_subs.is_empty();

    // collect default methods needed.
    let default_ui_items = match args {
        Args::NoDelegate => {
            validate_manual_delegate = false;
            no_delegate_absents(crate_.clone(), node_item_names, auto_subs)
        }
        Args::Delegate { delegate, delegate_mut } => delegate_absents(crate_.clone(), node_item_names, delegate, delegate_mut, auto_subs),
        Args::DelegateList {
            delegate_list,
            delegate_list_mut,
        } => delegate_list_absents(crate_.clone(), node_item_names, delegate_list, delegate_list_mut, auto_subs),
        Args::NewNode(_) => {
            unreachable!()
        }
    };

    if validate_manual_delegate {
        let validate = vec![
            ident!("init"),
            ident!("deinit"),
            ident!("info"),
            ident!("event"),
            ident!("update"),
            // ident!("measure"), // very common to avoid delegating
            ident!("layout"),
            ident!("render"),
            ident!("render_update"),
        ];

        // validate that manually implemented UiNode methods call the expected method in the struct child or children.

        for (manual_impl, level) in node_items.iter().zip(node_items_missing_del_level.into_iter()) {
            let mut validator = DelegateValidator::new(manual_impl);

            if level == util::LintLevel::Allow || !validate.contains(validator.ident) {
                continue;
            }

            validator.visit_impl_item(manual_impl);

            if !validator.delegates {
                let ident = validator.ident;
                errors.push(
                    format_args!(
                        "auto impl delegates call to `{ident}`, but this manual impl does not\n\
                        `#[{missing_delegate_level}(zero_ui::missing_delegate)]` is on",
                    ),
                    {
                        match manual_impl {
                            ImplItem::Fn(mtd) => mtd.block.span(),
                            _ => non_user_error!("expected a method"),
                        }
                    },
                );
            }
        }
    }
    if validate_auto_subs {
        if let Some(init_mtd) = node_items
            .iter()
            .map(|it| match it {
                ImplItem::Fn(mtd) => mtd,
                _ => non_user_error!("expected a method"),
            })
            .find(|it| it.sig.ident == "init")
        {
            let mut validator = AutoSubsValidator::new();

            validator.visit_impl_item_fn(init_mtd);

            if !validator.ok {
                errors.push(
                    "auto impl generates `auto_subs`, but custom init does not call it\n\
                add `self.auto_subs();` to custom init or remove `#[var]` and `#[event]` attributes",
                    init_mtd.block.span(),
                );
            }
        }
    }

    // if we are not in a `UiNode` impl and no method was tagged `#[UiNode]`.
    if ui_node_path.is_none() && node_items.is_empty() && !other_items.is_empty() {
        abort_call_site!("no `UiNode` method found, missing `UiNode for` in impl or `#[UiNode]` in methods")
    }

    let generics = input.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let self_ty = input.self_ty;

    let in_node_impl = ui_node_path.is_some();

    // we use the UiNode path provided by the user if possible
    // to avoid an unused import warning.
    let ui_node_path = ui_node_path
        .map(|p| p.to_token_stream())
        .unwrap_or_else(|| quote! { #crate_::widget_instance::UiNode });

    // modify impl header for new_node and collect
    let (impl_generics, self_ty, decl) = if let Some(new_node) = new_node {
        let gen = new_node.impl_generics;
        let custom_gen = &generics.params;
        let sep = if custom_gen.trailing_punct() || custom_gen.is_empty() {
            TokenStream::new()
        } else {
            quote!( , )
        };

        let mut node_custom_gen = TokenStream::new();
        let mut node_sep = TokenStream::new();
        let node_ident = new_node.ident;
        let mut self_ty_error = true;
        if let syn::Type::Path(p) = &*self_ty {
            if p.path.segments.len() == 1 {
                let seg = &p.path.segments[0];
                if seg.ident == node_ident {
                    self_ty_error = false;
                    if let PathArguments::AngleBracketed(a) = &seg.arguments {
                        node_custom_gen = a.args.to_token_stream();
                        if !a.args.trailing_punct() && !a.args.is_empty() {
                            node_sep = quote!( , );
                        }
                    }
                }
            }
        }
        if self_ty_error {
            errors.push(format!("expected `{node_ident}`"), self_ty.span());
        }
        let node_gen = new_node.node_generics;

        let impl_generics = quote! { <#custom_gen #sep #gen> };
        let self_ty = quote! { #node_ident<#node_custom_gen #node_sep #node_gen> };

        let mut decl = new_node.decl;

        if !new_node.auto_subs.is_empty() {
            let init = new_node.auto_subs;

            decl.extend(quote! {
                impl #impl_generics #self_ty #where_clause {
                    /// Init auto-generated event and var subscriptions.
                    fn auto_subs(&mut self) {
                        #init
                    }
                }
            });
        }

        (impl_generics, self_ty, decl)
    } else {
        (impl_generics.to_token_stream(), self_ty.to_token_stream(), quote!())
    };

    let result = if in_node_impl {
        quote! {
            #errors

            #decl

            impl #impl_generics #ui_node_path for #self_ty #where_clause {
                #(#node_items)*
                #(#default_ui_items)*
                #(#other_items)*
            }
        }
    } else {
        let input_attrs = input.attrs;
        quote! {
            #errors

            #decl

            #(#input_attrs)*
            impl #impl_generics #self_ty #where_clause {
                #(#other_items)*
            }

            impl #impl_generics #ui_node_path for #self_ty #where_clause {
                #(#node_items)*
                #(#default_ui_items)*
            }
        }
    };

    //let test = format!("{result}");
    //if test.contains("FocusOnInit") {
    //    println!("{test}");
    //}

    result.into()
}

macro_rules! make_absents {
    ($user_mtds:ident $([fn $ident:ident $($tt:tt)*])+) => {{
        let mut absents = vec![];
        let user_mtds = $user_mtds;
        $(
        if !user_mtds.contains(&ident!(stringify!($ident))) {
            absents.push(parse_quote! {

                #[allow(clippy::borrow_deref_ref)]
                fn $ident $($tt)*
            });
        }
        )+
        absents
    }};
}

fn no_delegate_absents(crate_: TokenStream, user_mtds: HashSet<Ident>, auto_init: TokenStream) -> Vec<ImplItem> {
    make_absents! { user_mtds
        [fn info(&self, info: &mut #crate_::widget_info::WidgetInfoBuilder) { }]

        [fn init(&mut self) { #auto_init }]

        [fn deinit(&mut self) { }]

        [fn update(&mut self, updates: &mut #crate_::context::WidgetUpdates) { }]

        [fn event(&mut self, update: &mut #crate_::event::EventUpdate) { }]

        [fn measure(&self, wm: &mut #crate_::widget_info::WidgetMeasure) -> #crate_::units::PxSize {
            #crate_::context::LAYOUT.constrains().fill_size()
        }]

        [fn layout(&mut self, wl: &mut #crate_::widget_info::WidgetLayout) -> #crate_::units::PxSize {
            #crate_::context::LAYOUT.constrains().fill_size()
        }]
        [fn render(&self, frame: &mut #crate_::render::FrameBuilder) { }]

        [fn render_update(&self, update: &mut #crate_::render::FrameUpdate) { }]
    }
}

fn delegate_absents(
    crate_: TokenStream,
    user_mtds: HashSet<Ident>,
    borrow: Expr,
    borrow_mut: Expr,
    auto_init: TokenStream,
) -> Vec<ImplItem> {
    let child = ident_spanned!(borrow.span()=> "child");
    let child_mut = ident_spanned!(borrow_mut.span()=> "child");

    let deref = quote_spanned! {borrow.span()=>
        &*#child
    };
    let deref_mut = quote_spanned! {borrow_mut.span()=>
        &mut *#child_mut
    };

    make_absents! { user_mtds
        [fn info(&self, info: &mut #crate_::widget_info::WidgetInfoBuilder) {
            let #child = {#borrow};
            #crate_::widget_instance::UiNode::info(#deref, info);
        }]

        [fn init(&mut self) {
            #auto_init
            let mut #child_mut = {#borrow_mut};
            #crate_::widget_instance::UiNode::init(#deref_mut);
        }]

        [fn deinit(&mut self) {
            let mut #child_mut = {#borrow_mut};
            #crate_::widget_instance::UiNode::deinit(#deref_mut);
        }]

        [fn update(&mut self, updates: &mut #crate_::context::WidgetUpdates) {
            let mut #child_mut = {#borrow_mut};
            #crate_::widget_instance::UiNode::update(#deref_mut, updates);
        }]

        [fn event(&mut self, update: &mut #crate_::event::EventUpdate) {
            let mut #child_mut = {#borrow_mut};
            #crate_::widget_instance::UiNode::event(#deref_mut, update);
        }]

        [fn measure(&self, wm: &mut #crate_::widget_info::WidgetMeasure) -> #crate_::units::PxSize {
            let mut #child = {#borrow};
            #crate_::widget_instance::UiNode::measure(#deref, wm)
        }]

        [fn layout(&mut self, wl: &mut #crate_::widget_info::WidgetLayout) -> #crate_::units::PxSize {
            let mut #child_mut = {#borrow_mut};
            #crate_::widget_instance::UiNode::layout(#deref_mut, wl)
        }]

        [fn render(&self, frame: &mut #crate_::render::FrameBuilder) {
            let #child = {#borrow};
            #crate_::widget_instance::UiNode::render(#deref, frame);
        }]

        [fn render_update(&self, update: &mut #crate_::render::FrameUpdate) {
            let #child = {#borrow};
            #crate_::widget_instance::UiNode::render_update(#deref, update);
        }]
    }
}

fn delegate_list_absents(
    crate_: TokenStream,
    user_mtds: HashSet<Ident>,
    borrow: Expr,
    borrow_mut: Expr,
    auto_init: TokenStream,
) -> Vec<ImplItem> {
    let children = ident_spanned!(borrow.span()=> "children");
    let children_mut = ident_spanned!(borrow_mut.span()=> "children");
    let deref = quote_spanned! {borrow.span()=>
        &*#children
    };
    let deref_mut = quote_spanned! {borrow_mut.span()=>
        &mut *#children_mut
    };
    make_absents! { user_mtds
        [fn info(&self, info: &mut #crate_::widget_info::WidgetInfoBuilder) {
            let #children = {#borrow};
            #crate_::widget_instance::ui_node_list_default::info_all(#deref, info);
        }]

        [fn init(&mut self) {
            #auto_init
            let #children_mut = {#borrow_mut};
            #crate_::widget_instance::ui_node_list_default::init_all(#deref_mut);
        }]

        [fn deinit(&mut self) {
            let #children_mut = {#borrow_mut};
            #crate_::widget_instance::ui_node_list_default::deinit_all(#deref_mut);
        }]

        [fn update(&mut self, updates: &mut #crate_::context::WidgetUpdates) {
            let #children_mut = {#borrow_mut};
            #crate_::widget_instance::ui_node_list_default::update_all(#deref_mut, updates);
        }]

        [fn event(&mut self, update: &mut #crate_::event::EventUpdate) {
            let #children_mut = {#borrow_mut};
            #crate_::widget_instance::ui_node_list_default::event_all(#deref_mut, update);
        }]

        [fn measure(&self, wm: &mut #crate_::widget_info::WidgetMeasure) -> #crate_::units::PxSize {
            let #children = {#borrow};
            #crate_::widget_instance::ui_node_list_default::measure_all(#deref, wm)
        }]

        [fn layout(&mut self, wl: &mut #crate_::widget_info::WidgetLayout) -> #crate_::units::PxSize {
            let #children_mut = {#borrow_mut};
            #crate_::widget_instance::ui_node_list_default::layout_all(#deref_mut, wl)
        }]

        [fn render(&self, frame: &mut #crate_::render::FrameBuilder) {
            let #children = {#borrow};
            #crate_::widget_instance::ui_node_list_default::render_all(#deref, frame);
        }]

        [fn render_update(&self, update: &mut #crate_::render::FrameUpdate) {
            let #children = {#borrow};
            #crate_::widget_instance::ui_node_list_default::render_update_all(#deref, update);
        }]
    }
}

/// Parsed macro arguments.
#[allow(clippy::large_enum_variant)]
enum Args {
    /// No arguments. Impl is for a leaf in the Ui tree.
    NoDelegate,
    /// `child` or `delegate=expr` and `delegate_mut=expr`. Impl is for
    /// an Ui that delegates each call to a single delegate.
    Delegate { delegate: Expr, delegate_mut: Expr },
    /// `children` or `delegate_list=expr` and `delegate_list_mut=expr`. Impl
    /// is for an Ui that delegates each call to multiple delegates.
    DelegateList { delegate_list: Expr, delegate_list_mut: Expr },
    /// New node mode.
    NewNode(ArgsNewNode),
}

impl Parse for Args {
    fn parse(args: ParseStream) -> Result<Self> {
        if args.peek(Token![struct]) {
            args.parse().map(Args::NewNode)
        } else if args.peek(Ident) {
            let arg0 = args.parse::<Ident>()?;

            let args = if arg0 == ident!("child") {
                Args::Delegate {
                    delegate: parse_quote_spanned!(arg0.span()=> &self.child),
                    delegate_mut: parse_quote_spanned!(arg0.span()=> &mut self.child),
                }
            } else if arg0 == ident!("children") {
                Args::DelegateList {
                    delegate_list: parse_quote_spanned!(arg0.span()=> &self.children),
                    delegate_list_mut: parse_quote_spanned!(arg0.span()=> &mut self.children),
                }
            } else if arg0 == ident!("none") {
                Args::NoDelegate
            } else {
                let delegate = ident!("delegate");
                let delegate_mut = ident!("delegate_mut");

                if arg0 == delegate || arg0 == delegate_mut {
                    let (delegate, delegate_mut) = parse_delegate_pair(args, arg0, delegate, delegate_mut)?;
                    Args::Delegate { delegate, delegate_mut }
                } else {
                    let delegate_list = ident!("delegate_list");
                    let delegate_list_mut = ident!("delegate_list_mut");

                    if arg0 == delegate_list || arg0 == delegate_list_mut {
                        let (delegate_list, delegate_list_mut) = parse_delegate_pair(args, arg0, delegate_list, delegate_list_mut)?;
                        Args::DelegateList {
                            delegate_list,
                            delegate_list_mut,
                        }
                    } else {
                        return Err(Error::new(
                            arg0.span(),
                            "expected `child`, `children`, `delegate`, or, `delegate_list`",
                        ));
                    }
                }
            };

            Ok(args)
        } else {
            Err(Error::new(
                Span::call_site(),
                "missing macro argument, expected `none`, `child`, `children`, `delegate`, `delegate_list`, or `struct`",
            ))
        }
    }
}

/// After parsing one of the delegate idents, parse the value and the other delegate.
///
/// Returns (immutable_expr, mutable_expr) independently of the order the delegates where written.
fn parse_delegate_pair(args: ParseStream, arg0: Ident, ident: Ident, ident_mut: Ident) -> Result<(Expr, Expr)> {
    // parse arg0 " = <expr>"
    let expr0 = parse_delegate_expr(args, &arg0)?;

    // get what ident is the second one, delegate pairs can be defined in any order.
    let expected_arg1 = if arg0 == ident { &ident_mut } else { &ident };

    // delegate pair are separated by comma (,)
    let comma = args
        .parse::<Token![,]>()
        .map_err(|_| Error::new(util::after_span(&expr0), format!("expected `, {expected_arg1} = <expr>`")))?;
    // delegate idents require a pair.
    let arg1: Ident = args
        .parse()
        .map_err(|_| Error::new(comma.span(), format!("expected `{expected_arg1} = <expr>`")))?;

    // second ident is not the expected pair.
    if &arg1 != expected_arg1 {
        return Err(Error::new(arg1.span(), format!("expected `{ident_mut}`")));
    }

    // parse arg1 " = <expr>"
    let expr1 = parse_delegate_expr(args, &arg1)?;

    // trailing comma.
    if args.peek(Token![,]) {
        args.parse::<Token![,]>().ok();
    }

    // result is (immutable_expr, mutable_expr)
    if arg0 == ident {
        Ok((expr0, expr1))
    } else {
        Ok((expr1, expr0))
    }
}
fn parse_delegate_expr(args: ParseStream, ident: &Ident) -> Result<Expr> {
    let colon = args
        .parse::<Token![=]>()
        .map_err(|_| Error::new(ident.span(), format!("expected `{ident} = <expr>`")))?;
    let expr: Expr = args
        .parse()
        .map_err(|_| Error::new(colon.span(), format!("expected `{ident} = <expr>`")))?;

    Ok(expr)
}

struct DelegateValidator<'a> {
    pub ident: &'a Ident,
    pub list_variant: Ident,
    pub list_specific_variant: Ident,
    args_count: u8,
    pub delegates: bool,

    todo_paths: Vec<Path>,
}
impl<'a> DelegateValidator<'a> {
    fn new(manual_impl: &'a ImplItem) -> Self {
        if let ImplItem::Fn(m) = manual_impl {
            DelegateValidator {
                ident: &m.sig.ident,
                list_variant: ident!("{}_all", m.sig.ident),
                list_specific_variant: ident!("item_{}", m.sig.ident),
                args_count: (m.sig.inputs.len() - 1) as u8,
                delegates: false,

                todo_paths: vec![parse_quote!(todo), parse_quote!(std::todo)],
            }
        } else {
            panic!("")
        }
    }
}
impl<'a, 'ast> Visit<'ast> for DelegateValidator<'a> {
    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        if (&i.method == self.ident && i.args.len() as u8 == self.args_count)
            || i.method == self.list_variant
            || i.method == self.list_specific_variant
        {
            self.delegates = true;
        }
        visit::visit_expr_method_call(self, i)
    }

    fn visit_expr_call(&mut self, i: &'ast ExprCall) {
        if let Expr::Path(p) = &*i.func {
            let len = p.path.segments.len();
            if len >= 2 {
                let q = &p.path.segments[len - 2].ident;
                if q == "UiNode" {
                    let method = &p.path.segments[len - 1].ident;
                    if (method == self.ident && i.args.len() as u8 == self.args_count + 1)
                        || method == &self.list_variant
                        || method == &self.list_specific_variant
                    {
                        self.delegates = true;
                    }
                }
            }
        }
        visit::visit_expr_call(self, i)
    }

    fn visit_macro(&mut self, i: &'ast Macro) {
        for p in &self.todo_paths {
            if &i.path == p {
                self.delegates = true;
                break;
            }
        }
        visit::visit_macro(self, i)
    }
}

struct AutoSubsValidator {
    ident: Ident,
    pub ok: bool,
}
impl AutoSubsValidator {
    fn new() -> Self {
        Self {
            ident: ident!("auto_subs"),
            ok: false,
        }
    }
}
impl<'ast> Visit<'ast> for AutoSubsValidator {
    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        if i.method == self.ident && i.args.is_empty() {
            self.ok = true;
        }
        visit::visit_expr_method_call(self, i)
    }
}

/// Removes and returns the `zero_ui::missing_delegate` level.
fn take_missing_deletate_level(
    attrs: &mut Vec<Attribute>,
    errors: &mut util::Errors,
    forbidden: &HashSet<&Ident>,
) -> Option<util::LintLevel> {
    let mut r = None;
    for (lint_ident, level, _) in util::take_zero_ui_lints(attrs, errors, forbidden) {
        if lint_ident == "missing_delegate" {
            r = Some(level);
        }
    }
    r
}

struct ArgsNewNode {
    ident: Ident,
    explicit_generics: Option<Generics>,
    fields: Punctuated<ArgsNewNodeField, Token![,]>,
}
impl Parse for ArgsNewNode {
    fn parse(input: ParseStream) -> Result<Self> {
        let _: Token![struct] = input.parse()?;
        let ident: Ident = input.parse()?;
        let explicit_generics = if input.peek(Token![<]) { Some(input.parse()?) } else { None };
        let inner;
        braced!(inner in input);
        let fields = Punctuated::parse_terminated(&inner)?;
        Ok(ArgsNewNode {
            ident,
            explicit_generics,
            fields,
        })
    }
}

struct ArgsNewNodeField {
    attrs: Vec<Attribute>,
    kind: ArgsNewNodeFieldKind,
    ident: Ident,
    ty: Type,
}
impl Parse for ArgsNewNodeField {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = Attribute::parse_outer(input)?;
        let ident = input.parse()?;
        let kind = ArgsNewNodeFieldKind::from_attrs(&mut attrs);
        let _: Token![:] = input.parse()?;
        let ty = input.parse()?;
        Ok(ArgsNewNodeField { attrs, kind, ident, ty })
    }
}

enum ArgsNewNodeFieldKind {
    Var,
    Event,
    Custom,
}
impl ArgsNewNodeFieldKind {
    fn from_attrs(attrs: &mut Vec<Attribute>) -> Self {
        let mut r = ArgsNewNodeFieldKind::Custom;
        let mut rmv = None;

        for (i, attr) in attrs.iter().enumerate() {
            if let Some(id) = attr.path().get_ident() {
                if id == "var" {
                    r = ArgsNewNodeFieldKind::Var;
                    rmv = Some(i);
                    break;
                } else if id == "event" {
                    r = ArgsNewNodeFieldKind::Event;
                    rmv = Some(i);
                    break;
                }
            }
        }

        if let Some(i) = rmv {
            attrs.remove(i);
        }

        r
    }
}

fn expand_new_node(args: ArgsNewNode, errors: &mut util::Errors) -> ExpandedNewNode {
    let mut delegate = ident!("none");
    let mut impl_generics = TokenStream::new();
    let mut node_generics = TokenStream::new();
    let mut node_fields = TokenStream::new();
    let mut auto_subs = TokenStream::new();

    let crate_ = crate::util::crate_core();

    if let Some(g) = args.explicit_generics {
        for p in g.params.into_iter() {
            if let GenericParam::Type(t) = p {
                t.to_tokens(&mut impl_generics);
                impl_generics.extend(quote!( , ));

                let cfg = util::Attributes::new(t.attrs).cfg;
                let ident = t.ident;
                node_generics.extend(quote! {
                    #cfg #ident ,
                });
            } else {
                errors.push("only type params are supported", p.span());
            }
        }
    }

    for ArgsNewNodeField {
        attrs, kind, ident, ty, ..
    } in args.fields
    {
        let attrs = util::Attributes::new(attrs);
        let cfg = attrs.cfg;
        let mut member_attrs = attrs.docs;
        member_attrs.extend(attrs.lints);
        member_attrs.extend(attrs.others);

        if ident == ident!("child") || ident == ident!("children") || ident == ident!("children_iter") {
            delegate = ident.clone();
        }

        match ty {
            Type::ImplTrait(t) => {
                let t_ident = ident!("T_{ident}");
                node_fields.extend(quote! {
                    #cfg
                    #(#member_attrs)*
                    #ident: #t_ident,
                });

                let bounds = t.bounds;
                impl_generics.extend(quote! {
                    #cfg
                    #t_ident: #bounds,
                });

                node_generics.extend(quote! {
                    #cfg
                    #t_ident,
                });
            }
            t => {
                node_fields.extend(quote! {
                    #cfg
                    #(#member_attrs)*
                    #ident: #t,
                });
            }
        }

        match kind {
            ArgsNewNodeFieldKind::Var => {
                auto_subs.extend(quote! {
                    #cfg
                    #crate_::context::WIDGET.sub_var(&self.#ident);
                });
            }
            ArgsNewNodeFieldKind::Event => {
                auto_subs.extend(quote! {
                    #cfg
                    #crate_::context::WIDGET.sub_event(&self.#ident);
                });
            }
            ArgsNewNodeFieldKind::Custom => {}
        }
    }

    let ident = args.ident;
    let decl = quote! {
        struct #ident<#impl_generics> {
            #node_fields
        }
    };

    ExpandedNewNode {
        delegate,
        ident,
        decl,
        impl_generics,
        node_generics,
        auto_subs,
    }
}

struct ExpandedNewNode {
    delegate: Ident,
    ident: Ident,
    decl: TokenStream,
    impl_generics: TokenStream,
    node_generics: TokenStream,
    auto_subs: TokenStream,
}

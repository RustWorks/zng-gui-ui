use crate::widget::*;
use proc_macro2::{Span, TokenStream};
use syn::{parse::*, *};

include!("util.rs");

/// `widget_new!` implementation
pub fn expand_widget_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}

/// Input error not caused by the user.
const ERROR: &str = "invalid non-user input";

struct WidgetNewInput {
    ident: Ident,
    imports: Vec<ItemUse>,
    default_child: DefaultBlock,
    default_self: DefaultBlock,
    whens: Vec<WhenBlock>,
    user_sets: Vec<PropertyAssign>,
    user_whens: Vec<WhenBlock>,
    user_child_expr: Expr,
}
impl Parse for WidgetNewInput {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![mod]>().expect(ERROR);
        let ident = input.parse().expect(ERROR);

        let mut imports = vec![];
        while input.peek(Token![use]) {
            imports.push(input.parse().expect(ERROR));
        }

        let default_child: DefaultBlock = input.parse().expect(ERROR);
        default_child.assert(DefaultBlockTarget::Child);

        let default_self: DefaultBlock = input.parse().expect(ERROR);
        default_self.assert(DefaultBlockTarget::Self_);

        let mut whens = vec![];
        while input.peek(keyword::when) {
            whens.push(input.parse().expect(ERROR));
        }

        input.parse::<keyword::input>().expect(ERROR);
        input.parse::<Token![:]>().expect(ERROR);

        fn input_stream(input: ParseStream) -> Result<ParseBuffer> {
            let inner;
            // this macro inserts a return Err(..) but we want to panic
            braced!(inner in input);
            Ok(inner)
        }
        let input = input_stream(input).expect(ERROR);

        let mut user_sets = vec![];
        let mut user_whens = vec![];
        while !input.is_empty() {
            let lookahead = input.lookahead1();

            // expect `when` at start or after `property:`
            if lookahead.peek(keyword::when) {
                user_whens.push(input.parse()?);
            }
            // expect `property:` only before `when` blocks.
            else if user_whens.is_empty() && lookahead.peek(Ident) {
                user_sets.push(input.parse()?);
            }
            // expect `=>` to be the last item.
            else if lookahead.peek(Token![=>]) {
                return Ok(WidgetNewInput {
                    ident,
                    imports,
                    default_child,
                    default_self,
                    whens,
                    user_sets,
                    user_whens,
                    user_child_expr: input.parse()?,
                });
            } else {
                return Err(lookahead.error());
            }
        }

        // if user input is empty, use a lookahead to make an error message.
        let lookahead = input.lookahead1();
        lookahead.peek(Ident);
        lookahead.peek(keyword::when);
        lookahead.peek(Token![=>]);
        Err(lookahead.error())
    }
}

impl DefaultBlock {
    pub fn assert(&self, expected: DefaultBlockTarget) {
        if self.target != expected {
            panic!("{}, expected default({})", ERROR, quote!(#expected))
        }

        for p in &self.properties {
            if !p.attrs.is_empty() {
                panic!("{}, unexpected attributes", ERROR)
            }
        }
    }
}

macro_rules! demo {
    ($($tt:tt)*) => {};
}

// Input:
demo! {
    /// Docs generated by all the docs attributes and property names.
    #[other_name_attrs]
    #[macro_export]// if pub
    macro_rules! button {
        ($($tt::tt)+) => {
            widget_new! {
                mod button;

                // uses with `crate` converted to `$crate`
                use $crate::something;

                default(child) {
                    // all the default(child) blocks grouped or an empty block
                }
                default(self) {
                    // all the default(self) blocks grouped or an empty block
                }

                // all the when blocks
                when(expr) {}
                when(expr) {}

                // user args
                input: {
                    // zero or more property assigns; required! not allowed.
                    // => child
                    $($tt)+
                }
            }
        };
    }

    #[doc(hidden)]
    pub mod button {
        use super::*;

        // => { child }
        pub fn child(child: impl Ui) -> impl Ui {
            child
        }

        // compile test of the property declarations
        #[allow(unused)]
        fn test(child: impl Ui) -> impl Ui {
            button! {
                => child
            }
        }
    }
}

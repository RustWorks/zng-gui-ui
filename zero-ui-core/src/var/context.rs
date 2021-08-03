use std::{cell::Cell, marker::PhantomData, thread::LocalKey};

use super::*;

/// A [`Var`] that represents a [`ContextVar`].
///
/// Context var types don't implement [`Var`] directly, to avoid problems with overlapping generics
/// this *proxy* zero-sized type is used.
#[derive(Clone)]
pub struct ContextVarProxy<C: ContextVar>(PhantomData<C>);
impl<C: ContextVar> ContextVarProxy<C> {
    /// New context var proxy.
    ///
    /// Prefer using [`ContextVar::new`] or the `new` generated using the [`context_var!`] macro.
    #[inline]
    pub fn new() -> Self {
        ContextVarProxy(PhantomData)
    }

    #[doc(hidden)]
    #[inline]
    pub fn static_ref() -> &'static Self {
        &ContextVarProxy(PhantomData)
    }
}

impl<C: ContextVar> Default for ContextVarProxy<C> {
    fn default() -> Self {
        ContextVarProxy(PhantomData)
    }
}

impl<C: ContextVar> crate::private::Sealed for ContextVarProxy<C> {}
impl<C: ContextVar> Var<C::Type> for ContextVarProxy<C> {
    type AsReadOnly = Self;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a C::Type {
        vars.as_ref().context_var::<C>().0
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a C::Type> {
        let info = vars.as_ref().context_var::<C>();
        if info.1 {
            Some(info.0)
        } else {
            None
        }
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|v| v.context_var::<C>().1)
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> C::Type {
        self.get_clone(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        vars.with_vars_read(|v| v.context_var::<C>().2)
    }

    #[inline]
    fn is_read_only<Vr: WithVars>(&self, _: &Vr) -> bool {
        true
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        true
    }

    #[inline]
    fn can_update(&self) -> bool {
        true
    }

    #[inline]
    fn modify<Vw, M>(&self, _: &Vw, _: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<C::Type>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set<Vw, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<C::Type>,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set_ne<Vw, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<C::Type>,
        C::Type: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }
}

impl<C: ContextVar> IntoVar<C::Type> for ContextVarProxy<C> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarValue<V: ContextVar> {
    _var: PhantomData<V>,
    value: Cell<(*const V::Type, bool, u32)>,
}
#[allow(missing_docs)]
impl<V: ContextVar> ContextVarValue<V> {
    #[inline]
    pub fn init() -> Self {
        ContextVarValue {
            _var: PhantomData,
            value: Cell::new((V::default_value() as *const V::Type, false, 0)),
        }
    }
}

/// See `ContextVar::thread_local_value`.
#[doc(hidden)]
pub struct ContextVarLocalKey<V: ContextVar> {
    local: &'static LocalKey<ContextVarValue<V>>,
}
#[allow(missing_docs)]
impl<V: ContextVar> ContextVarLocalKey<V> {
    #[inline]
    pub fn new(local: &'static LocalKey<ContextVarValue<V>>) -> Self {
        ContextVarLocalKey { local }
    }

    pub(super) fn get(&self) -> (*const V::Type, bool, u32) {
        self.local.with(|l| l.value.get())
    }

    pub(super) fn set(&self, value: (*const V::Type, bool, u32)) {
        self.local.with(|l| l.value.set(value))
    }

    pub(super) fn replace(&self, value: (*const V::Type, bool, u32)) -> (*const V::Type, bool, u32) {
        self.local.with(|l| l.value.replace(value))
    }
}

// used by context_var macro expansion.
#[doc(hidden)]
pub use once_cell::sync::OnceCell;

#[doc(hidden)]
#[macro_export]
macro_rules! __context_var_inner {
    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = const $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {

            static DEFAULT: $type = $default;
            &DEFAULT

        };);
    };

    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = once $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {

            static DEFAULT: $crate::var::OnceCell<$type> = $crate::var::OnceCell::new();
            DEFAULT.get_or_init(||{
                $default
            })

        };);
    };

    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = return $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {
            $default
        };);
    };


    (gen => $(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $DEFAULT:expr;) => {
        $(#[$outer])*
        /// # ContextVar
        /// This `struct` is a [`ContextVar`](crate::var::ContextVar).
        #[derive(Debug, Clone, Copy)]
        $vis struct $ident;

        impl $ident {
            std::thread_local! {
                static THREAD_LOCAL_VALUE: $crate::var::ContextVarValue<$ident> = $crate::var::ContextVarValue::init();
            }

            /// [`Var`](crate::var::Var) implementer that represents this context var.
            #[inline]
            #[allow(unused)]
            pub fn new() -> $crate::var::ContextVarProxy<Self> {
                $crate::var::ContextVarProxy::new()
            }

            /// Default value, used when the variable is not set in a context.
            #[inline]
            pub fn default_value() -> &'static $type {
                $DEFAULT
            }

            /// References the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn get<'a, Vr: AsRef<$crate::var::VarsRead>>(vars: &'a Vr) -> &'a $type {
                $crate::var::Var::get($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Returns a clone of the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn get_clone<Vr: $crate::var::WithVarsRead>(vars: &Vr) -> $type {
                $crate::var::Var::get_clone($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// References the value in the current `vars` context if it is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn get_new<'a, Vw: AsRef<$crate::var::Vars>>(vars: &'a Vw) -> Option<&'a $type> {
                $crate::var::Var::get_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Returns a clone of the value in the current `vars` context if it is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn clone_new<Vw: $crate::var::WithVars>(vars: &Vw) -> Option<$type> {
                $crate::var::Var::clone_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            // TODO generate copy fns when https://github.com/rust-lang/rust/issues/48214 is stable

            /// If the value in the current `vars` context is marked as new.
            #[inline]
            #[allow(unused)]
            pub fn is_new<Vw: $crate::var::WithVars>(vars: &Vw) -> bool {
                $crate::var::Var::is_new($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }

            /// Gets the version of the value in the current `vars` context.
            #[inline]
            #[allow(unused)]
            pub fn version<Vr: $crate::var::WithVarsRead>(vars: &Vr) -> u32 {
                $crate::var::Var::version($crate::var::ContextVarProxy::<Self>::static_ref(), vars)
            }
        }

        impl $crate::var::ContextVar for $ident {
            type Type = $type;

            #[inline]
            fn default_value() -> &'static Self::Type {
               Self::default_value()
            }

            #[inline]
            fn thread_local_value() -> $crate::var::ContextVarLocalKey<Self> {
                $crate::var::ContextVarLocalKey::new(&Self::THREAD_LOCAL_VALUE)
            }
        }

        impl $crate::var::IntoVar<$type> for $ident {
            type Var = $crate::var::ContextVarProxy<Self>;
            #[inline]
            fn into_var(self) -> Self::Var {
                $crate::var::ContextVarProxy::default()
            }
        }
    };
}

///<span data-inline></span> Declares new [`ContextVar`](crate::var::ContextVar) types.
///
/// # Examples
/// ```
/// # use zero_ui_core::var::context_var;
/// # #[derive(Debug, Clone)]
/// # struct NotConst(u8);
/// # fn init_val() -> NotConst { NotConst(10) }
/// #
/// context_var! {
///     /// A public documented context var.
///     pub struct FooVar: u8 = const 10;
///
///     // A private context var.
///     struct BarVar: NotConst = once init_val();
/// }
/// ```
///
/// # Default Value
///
/// All context variable have a default fallback value that is used when the variable is not setted in the context.
///
/// The default value is a `&'static T` where `T` is the variable value type that must auto-implement [`VarValue`](crate::var::VarValue).
///
/// There are three different ways of specifying how the default value is stored. The way is selected by a keyword
/// after the `=` and before the default value expression.
///
/// ## `const`
///
/// The default expression is evaluated to a `static` item that is referenced when the variable default is requested.
///
/// Required a constant expression.
///
/// ## `return`
///
/// The default expression is returned when the variable default is requested.
///
/// Requires an expression of type `&'static T` where `T` is the variable value type.
///
/// ## `once`
///
/// The default expression is evaluated once during the first request and the value is cached for the lifetime of the process.
///
/// Requires an expression of type `T` where `T` is the variable value type.
///
/// # Naming Convention
///
/// It is recommended that the type name ends with the `Var` suffix.
#[macro_export]
macro_rules! context_var {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $mode:ident $default:expr;)+) => {$(
        $crate::__context_var_inner!($(#[$outer])* $vis struct $ident: $type = $mode $default;);
    )+};
}
#[doc(inline)]
pub use crate::context_var;

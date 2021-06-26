use super::*;

use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

/// Initializes a new conditional var.
///
/// A condition var updates when the first `true` condition changes or the mapped var for the current condition changes.
///
/// # Syntax
///
/// The macro expects a list of `condition-var => condition-value-var`, the list is separated by comma.
/// The last condition must be the `_` token that maps to the value for when none of the conditions are `true`.
///
/// The `condition-var` must be an expression that evaluates to an `impl Var<bool>` type. The `condition-value-var` must
/// by any type that implements `IntoVar`. All condition values must be of the same [`VarValue`] type.
///
/// # Example
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) { }
/// let condition = var(true);
/// let when_false = var("condition: false".to_text());
///
/// let t = text(when_var! {
///     condition.clone() => "condition: true".to_text(),
///     _ => when_false.clone(),
/// });
/// ```
///
/// In the example if `condition` or `when_false` are modified the text updates.
///
/// # `cfg`
///
/// Every condition can be annotated with attributes, including `#[cfg(..)]`.
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) { }
/// # let condition0 = var(true);
/// # let condition1 = var(true);
/// let t = text(when_var! {
///     #[cfg(some_flag)]
///     condition0 => "is condition 0".to_text(),
///     #[cfg(not(some_flag))]
///     condition1 => "is condition 1".to_text(),
///     _ => "is default".to_text(),
/// });
/// ```
///
/// In the example above only one of the conditions will be compiled, the generated variable is the same
/// type as if you had written a single condition.
#[macro_export]
macro_rules! when_var {
    ($($tt:tt)*) => {
        $crate::var::__when_var! {
            $crate::var
            $($tt)*
        }
    }
}

#[doc(inline)]
pub use crate::when_var;

#[doc(hidden)]
pub use zero_ui_proc_macros::when_var as __when_var;

macro_rules! impl_rc_when_var {
    ($(
        $len:tt => $($n:tt),+;
    )+) => {$(
        $crate::paste!{
            impl_rc_when_var!{
                Var: [<RcWhen $len Var>];// RcWhen2Var
                Data: [<RcWhen $len VarData>];// RcWhen2VarData
                len: $len;//2
                C: $([<C $n>]),+;// C0, C1
                V: $([<V $n>]),+;// V0, V1
                n: $($n),+; // 0, 1
            }
        }
    )+};
    (
        Var: $RcMergeVar:ident;
        Data: $RcMergeVarData:ident;
        len: $len:tt;
        C: $($C:ident),+;
        V: $($V:ident),+;
        n: $($n:tt),+;
    ) => {
        #[doc(hidden)]
        pub struct $RcMergeVar<O: VarValue, D: Var<O>, $($C: Var<bool>),+ , $($V: Var<O>),+>(Rc<$RcMergeVarData<O, D, $($C),+ , $($V),+>>);
        struct $RcMergeVarData<O: VarValue, D: Var<O>, $($C: Var<bool>),+ , $($V: Var<O>),+> {
            _o: PhantomData<O>,

            default_value: D,
            default_version: Cell<u32>,

            conditions: ( $($C,)+ ),
            condition_versions: [Cell<u32>; $len],

            values: ( $($V,)+ ),
            value_versions: [Cell<u32>; $len],

            self_version: Cell<u32>,
        }
        #[allow(missing_docs)]// this is hidden
        impl<O: VarValue, D: Var<O>, $($C: Var<bool>),+ , $($V: Var<O>),+> $RcMergeVar<O, D, $($C),+ , $($V),+> {
            pub fn new(default_value: D, conditions: ($($C,)+), values: ($($V,)+)) -> Self {
                Self(
                    Rc::new($RcMergeVarData {
                        _o: PhantomData,

                        default_value,
                        default_version: Cell::new(0),

                        conditions,
                        condition_versions: array_init::array_init(|_|Cell::new(0)),

                        values: ($(values.$n,)+),
                        value_versions: array_init::array_init(|_|Cell::new(0)),

                        self_version: Cell::new(0),
                    })
                )
            }
        }

        impl<O: VarValue, D: Var<O>, $($C: Var<bool>),+ , $($V: Var<O>),+> Clone for $RcMergeVar<O, D, $($C),+ , $($V),+> {
            fn clone(&self) -> Self {
                Self(Rc::clone(&self.0))
            }
        }

        impl<O: VarValue, D: Var<O>, $($C: Var<bool>),+ , $($V: Var<O>),+> Var<O> for $RcMergeVar<O, D, $($C),+ , $($V),+> {
            type AsReadOnly = ReadOnlyVar<O, Self>;

            fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a O {
                let vars = vars.as_ref();

                $(
                    if *self.0.conditions.$n.get(vars) {
                        self.0.values.$n.get(vars)
                    }
                )else+
                else {
                    self.0.default_value.get(vars)
                }
            }
            fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a O> {
                let vars = vars.as_ref();

                let mut condition_is_new = false;
                $(
                    condition_is_new |= self.0.conditions.$n.is_new(vars);
                    if *self.0.conditions.$n.get(vars) {
                        return if condition_is_new {
                            Some(self.0.values.$n.get(vars))
                        } else {
                            self.0.values.$n.get_new(vars)
                        };
                    }
                )+

                if condition_is_new {
                    Some(self.0.default_value.get(vars))
                } else {
                    self.0.default_value.get_new(vars)
                }
            }
            fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> O {
                match Rc::try_unwrap(self.0) {
                    Ok(r) => vars.with(|vars| {
                        $(
                            if *r.conditions.$n.get(vars) {
                                r.values.$n.into_value(vars)
                            }
                        )else+
                        else {
                            r.default_value.into_value(vars)
                        }
                    }),
                    Err(e) => $RcMergeVar(e).get_clone(vars)
                }
            }
            fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
                vars.with_vars(|vars| {
                    let mut condition_is_new = false;

                    $(
                        condition_is_new |= self.0.conditions.$n.is_new(vars);
                        if *self.0.conditions.$n.get(vars) {
                            return condition_is_new || self.0.values.$n.is_new(vars);
                        }
                    )+
                    condition_is_new || self.0.default_value.is_new(vars)
                })
            }
            fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
                vars.with(|vars| {
                    let mut changed = false;

                    $(
                        let version = self.0.conditions.$n.version(vars);
                        if version != self.0.condition_versions[$n].get() {
                            changed = true;
                            self.0.condition_versions[$n].set(version);
                        }
                    )+

                    $(
                        let version = self.0.values.$n.version(vars);
                        if version != self.0.value_versions[$n].get() {
                            changed = true;
                            self.0.value_versions[$n].set(version);
                        }
                    )+

                    let version = self.0.default_value.version(vars);
                    if version != self.0.default_version.get() {
                        changed = true;
                        self.0.default_version.set(version);
                    }

                    if changed {
                        self.0.self_version.set(self.0.self_version.get().wrapping_add(1));
                    }

                    self.0.self_version.get()
                })
            }
            fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
                vars.with_vars(|vars| {
                    $(
                        if *self.0.conditions.$n.get(vars) {
                            self.0.values.$n.is_read_only(vars)
                        }
                    )else+
                    else {
                        self.0.default_value.is_read_only(vars)
                    }
                })
            }
            fn always_read_only(&self) -> bool {
                $(self.0.values.$n.always_read_only())&&+ && self.0.default_value.always_read_only()
            }
            #[inline]
            fn can_update(&self) -> bool {
                true
            }
            fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
            where
                Vw: WithVars,
                N: Into<O>
            {
                vars.with_vars(|vars| {
                    $(
                        if *self.0.conditions.$n.get(vars) {
                            self.0.values.$n.set(vars, new_value)
                        }
                    )else+
                    else {
                        self.0.default_value.set(vars, new_value)
                    }
                })
            }
            fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, VarIsReadOnly>
            where
                Vw: WithVars,
                N: Into<O>,
                O: PartialEq
            {
                vars.with_vars(|vars| {
                    $(
                        if *self.0.conditions.$n.get(vars) {
                            self.0.values.$n.set_ne(vars, new_value)
                        }
                    )else+
                    else {
                        self.0.default_value.set_ne(vars, new_value)
                    }
                })
            }

            fn modify<Vw: WithVars, F: FnOnce(&mut VarModify<O>) + 'static>(&self, vars: &Vw, change: F) -> Result<(), VarIsReadOnly> {
                vars.with_vars(|vars| {
                    $(
                        if *self.0.conditions.$n.get(vars) {
                            self.0.values.$n.modify(vars, change)
                        }
                    )else+
                    else {
                        self.0.default_value.modify(vars, change)
                    }
                })
            }

            #[inline]
            fn into_read_only(self) -> Self::AsReadOnly {
               ReadOnlyVar::new(self)
            }
        }

        impl<O: VarValue, D: Var<O>, $($C: Var<bool>),+ , $($V: Var<O>),+> IntoVar<O> for $RcMergeVar<O, D, $($C),+ , $($V),+>  {
            type Var = Self;

            #[inline]
            fn into_var(self) -> Self::Var {
                self
            }
        }
    };
}
impl_rc_when_var! {
    1 => 0;
    2 => 0, 1;
    3 => 0, 1, 2;
    4 => 0, 1, 2, 3;
    5 => 0, 1, 2, 3, 4;
    6 => 0, 1, 2, 3, 4, 5;
    7 => 0, 1, 2, 3, 4, 5, 6;
    8 => 0, 1, 2, 3, 4, 5, 6, 7;
}

/// A [`when_var!`] that uses dynamic dispatch to support any number of variables.
///
/// This type is a reference-counted pointer ([`Rc`]),
/// it implements the full [`Var`] read and write methods.
///
/// Don't use this type directly use the [macro](when_var!) instead.
pub struct RcWhenVar<O: VarValue>(Rc<RcWhenVarData<O>>);
struct RcWhenVarData<O: VarValue> {
    default_: BoxedVar<O>,
    default_version: Cell<u32>,

    whens: Box<[(BoxedVar<bool>, BoxedVar<O>)]>,
    when_versions: Box<[(Cell<u32>, Cell<u32>)]>,

    self_version: Cell<u32>,
}
impl<O: VarValue> RcWhenVar<O> {
    #[doc(hidden)]
    pub fn new(default_: BoxedVar<O>, whens: Box<[(BoxedVar<bool>, BoxedVar<O>)]>) -> Self {
        RcWhenVar(Rc::new(RcWhenVarData {
            default_,
            default_version: Cell::new(0),

            when_versions: whens.iter().map(|_| (Cell::new(0), Cell::new(0))).collect(),
            whens,

            self_version: Cell::new(0),
        }))
    }
}
impl<O: VarValue> Clone for RcWhenVar<O> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<O: VarValue> Var<O> for RcWhenVar<O> {
    type AsReadOnly = ReadOnlyVar<O, Self>;

    /// Gets the the first variable with `true` condition or the default variable.
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a O {
        let vars = vars.as_ref();

        for (c, v) in self.0.whens.iter() {
            if *c.get(vars) {
                return v.get(vars);
            }
        }
        self.0.default_.get(vars)
    }

    /// Gets the first variable with `true` condition if that condition or previous conditions are new.
    ///
    /// Gets the first variable with `true` condition if that variable value is new.
    ///
    /// Gets the default variable if any of the conditions are new and all are `false`.
    ///
    /// Gets the default variable if all conditions are `false` and the default variable value is new.
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a O> {
        let vars = vars.as_ref();

        let mut condition_is_new = false;
        for (c, v) in self.0.whens.iter() {
            condition_is_new |= c.is_new(vars);
            if *c.get(vars) {
                return if condition_is_new {
                    // a higher priority condition is new `false` of the current condition is new `true`.
                    Some(v.get(vars))
                } else {
                    v.get_new(vars)
                };
            }
        }

        if condition_is_new {
            Some(self.0.default_.get(vars))
        } else {
            self.0.default_.get_new(vars)
        }
    }

    /// Gets if [`get_new`](Self::get_new) will return `Some(_)` if called.
    ///
    /// This is slightly more performant than `when_var.get_new(vars).is_some()`.
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| {
            let mut condition_is_new = false;
            for (c, v) in self.0.whens.iter() {
                condition_is_new |= c.is_new(vars);
                if *c.get(vars) {
                    return condition_is_new || v.is_new(vars);
                }
            }
            condition_is_new || self.0.default_.is_new(vars)
        })
    }

    /// If `self` is the only reference calls `into_value` on the first variable with condition `true`.
    ///
    /// If `self` is not the only reference returns a clone of the value.
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> O {
        match Rc::try_unwrap(self.0) {
            Ok(r) => vars.with(move |vars| {
                for (c, v) in Vec::from(r.whens) {
                    if *c.get(vars) {
                        return v.into_value(vars);
                    }
                }

                r.default_.into_value(vars)
            }),
            Err(e) => RcWhenVar(e).get_clone(vars),
        }
    }

    /// Gets the version.
    ///
    /// The version is new when any of the condition and value variables version is new.
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        vars.with(|vars| {
            let mut changed = false;

            let dv = self.0.default_.version(vars);
            if dv != self.0.default_version.get() {
                changed = true;
                self.0.default_version.set(dv);
            }

            for ((c, v), (w_cv, w_vv)) in self.0.whens.iter().zip(self.0.when_versions.iter()) {
                let cv = c.version(vars);
                if cv != w_cv.get() {
                    changed = true;
                    w_cv.set(cv);
                }
                let vv = v.version(vars);
                if vv != w_vv.get() {
                    changed = true;
                    w_vv.set(vv);
                }
            }

            if changed {
                self.0.self_version.set(self.0.self_version.get().wrapping_add(1));
            }

            self.0.self_version.get()
        })
    }

    /// If the [current value variable](Self::get) is read-only.
    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| {
            for (c, v) in self.0.whens.iter() {
                if *c.get(vars) {
                    return v.is_read_only(vars);
                }
            }
            self.0.default_.is_read_only(vars)
        })
    }

    /// If all value variables (including default) are always read-only.
    fn always_read_only(&self) -> bool {
        self.0.whens.iter().all(|(_, v)| v.always_read_only()) && self.0.default_.always_read_only()
    }

    /// Always `true`.
    #[inline]
    fn can_update(&self) -> bool {
        true
    }

    /// Sets the [current value variable](Self::get).
    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<O>,
    {
        vars.with_vars(|vars| {
            for (c, v) in self.0.whens.iter() {
                if *c.get(vars) {
                    return v.set(vars, new_value);
                }
            }
            self.0.default_.set(vars, new_value)
        })
    }

    fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<O>,
        O: PartialEq,
    {
        vars.with_vars(|vars| {
            for (c, v) in self.0.whens.iter() {
                if *c.get(vars) {
                    return v.set_ne(vars, new_value);
                }
            }
            self.0.default_.set_ne(vars, new_value)
        })
    }

    /// Modify the [current value variable](Self::get).
    fn modify<Vw: WithVars, F: FnOnce(&mut VarModify<O>) + 'static>(&self, vars: &Vw, change: F) -> Result<(), VarIsReadOnly> {
        vars.with_vars(|vars| {
            for (c, v) in self.0.whens.iter() {
                if *c.get(vars) {
                    return v.modify(vars, change);
                }
            }
            self.0.default_.modify(vars, change)
        })
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }
}
impl<O: VarValue> IntoVar<O> for RcWhenVar<O> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

/// Builder used in [`when_var!`] when there is more then 8 conditions. Boxes the variables.
#[doc(hidden)]
pub struct WhenVarBuilderDyn<O: VarValue> {
    default_: BoxedVar<O>,
    whens: Vec<(BoxedVar<bool>, BoxedVar<O>)>,
}
#[allow(missing_docs)] // this is hidden
impl<O: VarValue> WhenVarBuilderDyn<O> {
    pub fn new<D: IntoVar<O>>(default_: D) -> Self {
        Self {
            default_: default_.into_var().boxed(),
            whens: vec![],
        }
    }

    pub fn push<C: Var<bool>, V: IntoVar<O>>(mut self, condition: C, value: V) -> Self {
        self.whens.push((condition.boxed(), value.into_var().boxed()));
        self
    }

    pub fn build(self) -> RcWhenVar<O> {
        RcWhenVar::new(self.default_, self.whens.into_boxed_slice())
    }
}

/// Builder used in [`when_var!`], designed to support #[cfg(..)] attributes in conditions.
#[doc(hidden)]
pub struct WhenVarBuilder<O: VarValue, D: Var<O>> {
    _v: PhantomData<O>,
    default_value: D,
}
#[allow(missing_docs)] // this is hidden
impl<O: VarValue, D: Var<O>> WhenVarBuilder<O, D> {
    /// Start the builder with the last item, it is the only *condition* that cannot be excluded by #[cfg(..)].
    pub fn new<ID: IntoVar<O, Var = D>>(default_value: ID) -> Self {
        Self {
            _v: PhantomData,
            default_value: default_value.into_var(),
        }
    }

    pub fn push<C0: Var<bool>, IV0: IntoVar<O>>(self, condition: C0, value: IV0) -> WhenVarBuilder1<O, D, C0, IV0::Var> {
        WhenVarBuilder1 {
            _v: self._v,
            default_value: self.default_value,
            condition: (condition,),
            value: (value.into_var(),),
        }
    }

    /// Only default condition included, if [`when_var!`] is implemented correctly other conditions where typed
    /// but are excluded by #[cfg(..)].
    pub fn build(self) -> D {
        self.default_value
    }
}
macro_rules! impl_when_var_builder {
    ($(
        $len:tt => $($n:tt),+ => $next_len:tt ;
    )+) => {$(
        $crate::paste!{
            impl_when_var_builder!{
                Builder: [<WhenVarBuilder $len>];// WhenVarBuilder2
                Var: [<RcWhen $len Var>];// RcWhen2Var
                C: $([<C $n>]),+;// C0, C1
                V: $([<V $n>]),+;// V0, V1
                n: $($n),+; // 0, 1
                BuilderNext: [<WhenVarBuilder $next_len>];//WhenVarBuilder3
            }
        }
    )+};
    (
        Builder: $Builder:ident;
        Var: $Var:ident;
        C: $($C:ident),+;
        V: $($V:ident),+;
        n: $($n:tt),+;
        BuilderNext: $BuilderNext:ident;
    ) => {
        #[doc(hidden)]
        pub struct $Builder<
            O: VarValue,
            D: Var<O>,
            $($C: Var<bool>,)*
            $($V: Var<O>,)*
        > {
            _v: PhantomData<O>,
            default_value: D,
            condition: ($($C,)*),
            value: ($($V,)*),
        }
        #[allow(missing_docs)] // this is hidden
        impl<
            O: VarValue,
            D: Var<O>,
            $($C: Var<bool>,)*
            $($V: Var<O>,)*
        > $Builder<O, D, $($C,)* $($V),*> {
            pub fn push<C: Var<bool>, IV: IntoVar<O>>(self, condition: C, value: IV) -> $BuilderNext<O, D, $($C,)* C, $($V,)* IV::Var> {
                $BuilderNext {
                    _v: self._v,
                    default_value: self.default_value,
                    condition: ( $(self.condition.$n,)* condition),
                    value: ( $(self.value.$n,)* value.into_var()),
                }
            }

            pub fn build(self) -> $Var<O, D, $($C,)* $($V,)*> {
                $Var::new(self.default_value, self.condition, self.value)
            }
        }
    }
}
impl_when_var_builder! {
    1 => 0 => 2;
    2 => 0, 1 => 3;
    3 => 0, 1, 2 => 4;
    4 => 0, 1, 2, 3 => 5;
    5 => 0, 1, 2, 3, 4 => 6;
    6 => 0, 1, 2, 3, 4, 5 => 7;
    7 => 0, 1, 2, 3, 4, 5, 6 => 8;
    8 => 0, 1, 2, 3, 4, 5, 6, 7 => 9;
}
/// Generic builder stops at WhenVarBuilder8, this only type
/// exists because of the nature of the [`impl_when_var_builder`] code.
#[doc(hidden)]
#[allow(unused)]
pub struct WhenVarBuilder9<O, D, C0, C1, C2, C3, C4, C5, C6, C7, C8, V0, V1, V2, V3, V4, V5, V6, V7, V8> {
    _v: PhantomData<O>,
    default_value: D,
    condition: (C0, C1, C2, C3, C4, C5, C6, C7, C8),
    value: (V0, V1, V2, V3, V4, V5, V6, V7, V8),
}

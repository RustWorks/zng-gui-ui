use super::*;

use std::cell::{Cell, RefCell, UnsafeCell};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::rc::Rc;

/// Initializes a new [`Var`](crate::var::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](crate::var::Var), minimal 2.
/// * `merge`: A function that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Example
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) {  }
/// let var0: RcVar<Text> = var_from("Hello");
/// let var1: RcVar<Text> = var_from("World");
///
/// let greeting_text = text(merge_var!(var0, var1, |a, b|formatx!("{} {}!", a, b)));
/// ```
#[macro_export]
macro_rules! merge_var {
    ($v0: expr, $v1: expr, $merge: expr) => {
        $crate::var::RcMerge2Var::new(($v0, $v1), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $merge: expr) => {
        $crate::var::RcMerge3Var::new(($v0, $v1, $v2), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $merge: expr) => {
        $crate::var::RcMerge4Var::new(($v0, $v1, $v2, $v3), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $merge: expr) => {
        $crate::var::RcMerge5Var::new(($v0, $v1, $v2, $v3, $v4), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $merge: expr) => {
        $crate::var::RcMerge6Var::new(($v0, $v1, $v2, $v3, $v4, $v5), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $merge: expr) => {
        $crate::var::RcMerge7Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $merge: expr) => {
        $crate::var::RcMerge8Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $($more_args:tt)+) => {
        compile_error!("merge_var is only implemented to a maximum of 8 variables")
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (var0, var1, .., merge_fn")
    };
}
#[doc(inline)]
pub use crate::merge_var;

macro_rules! impl_rc_merge_var {
    ($(
        $len:tt => $($n:tt),+;
    )+) => {$(
        $crate::paste!{
            impl_rc_merge_var!{
                Var: [<RcMerge $len Var>];// RcMerge2Var
                Data: [<RcMerge $len VarData>];// RcMerge2VarData
                len: $len;//2
                I: $([<I $n>]),+;// I0, I1
                V: $([<V $n>]),+;// V0, V1
                n: $($n),+; // 0, 1
            }
        }
    )+};

    (
        Var: $RcMergeVar:ident;
        Data: $RcMergeVarData:ident;
        len: $len:tt;
        I: $($I:ident),+;
        V: $($V:ident),+;
        n: $($n:tt),+;
    ) => {
        #[doc(hidden)]
        pub struct $RcMergeVar<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>(
            Rc<$RcMergeVarData<$($I,)+ O, $($V,)+ F>>,
        );

        struct $RcMergeVarData<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static> {
            _i: PhantomData<($($I),+)>,
            vars: ($($V),+),
            f: RefCell<F>,
            versions: [Cell<u32>; $len],
            output_version: Cell<u32>,
            output: UnsafeCell<MaybeUninit<O>>, // TODO: we are leaking memory here (drop not called), change to new RcVar method.
            last_update_id: Cell<Option<u32>>,
        }

        #[allow(missing_docs)]// this is all hidden.
        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static> $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            pub fn new(vars: ($($V),+), f: F) -> Self {
                Self(Rc::new($RcMergeVarData {
                    _i: PhantomData,
                    vars,
                    f: RefCell::new(f),
                    versions: array_init::array_init(|_|Cell::new(0)),
                    output_version: Cell::new(0),
                    output: UnsafeCell::new(MaybeUninit::uninit()),
                    last_update_id: Cell::new(None),
                }))
            }

            pub fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
                <Self as Var<O>>::get(self, vars)
            }

            pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
                <Self as Var<O>>::get_new(self, vars)
            }

            pub fn is_new(&self, vars: &Vars) -> bool {
                <Self as Var<O>>::is_new(self, vars)
            }

            pub fn version(&self, vars: &Vars) -> u32 {
                <Self as Var<O>>::version(self, vars)
            }

            pub fn can_update(&self) -> bool {
                <Self as Var<O>>::can_update(self)
            }

            fn output_uninit(&self) -> bool {
                self.0.last_update_id.get().is_none()
            }

            fn update_output(&self, vars: &VarsRead) {
                let last_update_id = Some(vars.update_id());
                if self.0.last_update_id.get() != last_update_id {
                    let versions = ($(self.0.vars.$n.version(vars)),+);
                    if $(self.0.versions[$n].get() != versions.$n)||+ || self.output_uninit() {
                        let value = (&mut *self.0.f.borrow_mut())($(self.0.vars.$n.get(vars)),+);

                        // SAFETY: This is safe because it only happens before the first borrow
                        // of this update, and borrows cannot exist across updates because source
                        // vars require a &mut Vars for changing version.
                        unsafe {
                            let m_uninit = &mut *self.0.output.get();
                            m_uninit.as_mut_ptr().write(value);
                        }

                        self.0.output_version.set(self.0.output_version.get().wrapping_add(1));
                        $(self.0.versions[$n].set(versions.$n);)+
                    }
                    self.0.last_update_id.set(last_update_id);
                }
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Clone for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            fn clone(&self) -> Self {
                $RcMergeVar(Rc::clone(&self.0))
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        Var<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            type AsReadOnly = ReadOnlyVar<O, Self>;

            type AsLocal = CloningLocalVar<O, Self>;

            fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a O {
                self.update_output(vars);

                // SAFETY:
                // This is safe because source require &mut Vars for updating.
                unsafe {
                    let inited = &*self.0.output.get();
                    &*inited.as_ptr()
                }
            }

            fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
                if self.is_new(vars) {
                    Some(self.get(vars))
                } else {
                    None
                }
            }

            fn is_new(&self, vars: &Vars) -> bool {
                $(self.0.vars.$n.is_new(vars))||+
            }

            fn version(&self, vars: &VarsRead) -> u32 {
                self.update_output(vars);
                self.0.output_version.get()
            }

            fn is_read_only(&self, _: &Vars) -> bool {
                true
            }

            fn always_read_only(&self) -> bool {
                true
            }

            fn can_update(&self) -> bool {
                $(self.0.vars.$n.can_update())||+
            }

            fn set(&self, _: &Vars, _: O) -> Result<(), VarIsReadOnly> {
                Err(VarIsReadOnly)
            }

            fn set_ne(&self, _: &Vars, _: O) -> Result<(), VarIsReadOnly>  where O: PartialEq {
                Err(VarIsReadOnly)
            }

            fn modify<F2: FnOnce(&mut VarModify<O>) + 'static>(&self, _: &Vars, _: F2) -> Result<(), VarIsReadOnly> {
                Err(VarIsReadOnly)
            }

            fn into_local(self) -> Self::AsLocal {
                CloningLocalVar::new(self)
            }

            fn into_read_only(self) -> Self::AsReadOnly {
                ReadOnlyVar::new(self)
            }
        }

        impl<$($I: VarValue,)+ O: VarValue, $($V: Var<$I>,)+ F: FnMut($(&$I),+) -> O + 'static>
        IntoVar<O> for $RcMergeVar<$($I,)+ O, $($V,)+ F> {
            type Var = Self;
            fn into_var(self) -> Self {
                self
            }
        }
    };
}

impl_rc_merge_var! {
    2 => 0, 1;
    3 => 0, 1, 2;
    4 => 0, 1, 2, 3;
    5 => 0, 1, 2, 3, 4;
    6 => 0, 1, 2, 3, 4, 5;
    7 => 0, 1, 2, 3, 4, 5, 6;
    8 => 0, 1, 2, 3, 4, 5, 6, 7;
    9 => 0, 1, 2, 3, 4, 5, 6, 7, 8;

    10 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9;
    11 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10;
    12 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11;
    13 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12;
    14 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13;
    15 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14;
    16 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15;
    17 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16;
    18 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17;
    19 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18;

    20 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19;
    21 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20;
    22 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21;
    23 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22;
    24 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23;
    25 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24;
    26 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25;
    27 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26;
    28 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27;
    29 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28;

    30 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29;
    31 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30;
    32 => 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31;
}

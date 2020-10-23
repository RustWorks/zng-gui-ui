use super::*;

/// A [`Var`] that locally owns the value.
///
/// This is [`always read-only`](VarObj::always_read_only), [cannot update](VarObj::can_update) and
/// is a [`VarLocal`].
#[derive(Clone, Default)]
pub struct OwnedVar<T: VarValue>(pub T);
impl<T: VarValue> protected::Var for OwnedVar<T> {}
impl<T: VarValue> VarObj<T> for OwnedVar<T> {
    fn get<'a>(&'a self, _: &'a Vars) -> &'a T {
        &self.0
    }

    fn get_new<'a>(&'a self, _: &'a Vars) -> Option<&'a T> {
        None
    }

    fn is_new(&self, _: &Vars) -> bool {
        false
    }

    fn version(&self, _: &Vars) -> u32 {
        0
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        false
    }

    fn set(&self, _: &Vars, _: T) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<T: VarValue> VarLocal<T> for OwnedVar<T> {
    fn get_local(&self) -> &T {
        &self.0
    }
    fn init_local(&mut self, _: &Vars) -> &T {
        &self.0
    }

    fn update_local(&mut self, _: &Vars) -> Option<&T> {
        None
    }
}
impl<T: VarValue> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;
    type AsLocal = Self;

    fn as_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        self
    }

    fn modify<F: FnOnce(&mut T) + 'static>(&self, _: &Vars, _: F) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn map<O: VarValue, F: FnMut(&T) -> O + 'static>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}

/// Wraps the value in an [`OwnedVar`] value.
impl<T: VarValue> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> OwnedVar<T> {
        OwnedVar(self)
    }
}

impl<T: VarValue> IntoVar<T> for OwnedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

use std::{fmt, mem, ops};

use crate::{context::LayoutMetrics, impl_from_and_into_var};

use super::{
    impl_length_comp_conversions, translate, AvailableSize, DipVector, LayoutMask, Length, LengthUnits, Point, PxVector, Scale2d, Transform,
};

/// 2D vector in [`Length`] units.
#[derive(Clone, Default, PartialEq)]
pub struct Vector {
    /// *x* displacement in length units.
    pub x: Length,
    /// *y* displacement in length units.
    pub y: Length,
}
impl fmt::Debug for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Vector").field("x", &self.x).field("y", &self.y).finish()
        } else {
            write!(f, "({:?}, {:?})", self.x, self.y)
        }
    }
}
impl fmt::Display for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.p$}, {:.p$})", self.x, self.y, p = p)
        } else {
            write!(f, "({}, {})", self.x, self.y)
        }
    }
}
impl Vector {
    /// New x, y from any [`Length`] unit.
    pub fn new<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Self {
        Vector { x: x.into(), y: y.into() }
    }

    /// New x, y from single value of any [`Length`] unit.
    pub fn splat(xy: impl Into<Length>) -> Self {
        let xy = xy.into();
        Vector { x: xy.clone(), y: xy }
    }

    /// ***x:*** [`Length::zero`], ***y:*** [`Length::zero`].
    #[inline]
    pub fn zero() -> Self {
        Self::new(Length::zero(), Length::zero())
    }

    /// `(1, 1)`.
    #[inline]
    pub fn one() -> Self {
        Self::new(1, 1)
    }

    /// `(1.px(), 1.px())`.
    #[inline]
    pub fn one_px() -> Self {
        Self::new(1.px(), 1.px())
    }

    /// Swap `x` and `y`.
    #[inline]
    pub fn yx(self) -> Self {
        Vector { y: self.x, x: self.y }
    }

    /// Returns `(x, y)`.
    #[inline]
    pub fn as_tuple(self) -> (Length, Length) {
        (self.x, self.y)
    }

    /// Returns `[x, y]`.
    #[inline]
    pub fn as_array(self) -> [Length; 2] {
        [self.x, self.y]
    }

    /// Returns a vector that computes the absolute layout vector of `self`.
    pub fn abs(&self) -> Vector {
        Vector {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Compute the vector in a layout context.
    #[inline]
    pub fn to_layout(&self, ctx: &LayoutMetrics, available_size: AvailableSize, default_value: PxVector) -> PxVector {
        PxVector::new(
            self.x.to_layout(ctx, available_size.width, default_value.x),
            self.y.to_layout(ctx, available_size.height, default_value.y),
        )
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`to_layout`].
    ///
    /// [`to_layout`]: Self::to_layout
    #[inline]
    pub fn affect_mask(&self) -> LayoutMask {
        self.x.affect_mask() | self.y.affect_mask()
    }

    /// Returns `true` if all values are [`Length::Default`].
    pub fn is_default(&self) -> bool {
        self.x.is_default() && self.y.is_default()
    }

    /// Replaces [`Length::Default`] values with `overwrite` values.
    pub fn replace_default(&mut self, overwrite: &Point) {
        self.x.replace_default(&overwrite.x);
        self.y.replace_default(&overwrite.y);
    }

    /// Cast to [`Point`].
    pub fn as_point(self) -> Point {
        Point { x: self.x, y: self.y }
    }

    /// Create a translate transform from `self`.
    pub fn into_transform(self) -> Transform {
        translate(self.x, self.y)
    }
}
impl_length_comp_conversions! {
    fn from(x: X, y: Y) -> Vector {
        Vector::new(x, y)
    }
}
impl_from_and_into_var! {
    fn from(p: PxVector) -> Vector {
        Vector::new(p.x, p.y)
    }
    fn from(p: DipVector) -> Vector {
        Vector::new(p.x, p.y)
    }
}
impl ops::Add for Vector {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Vector {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl ops::AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);

        self.x = x + rhs.x;
        self.y = y + rhs.y;
    }
}
impl ops::Sub for Vector {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Vector {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl ops::SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);

        self.x = x - rhs.x;
        self.y = y - rhs.y;
    }
}
impl<S: Into<Scale2d>> ops::Mul<S> for Vector {
    type Output = Self;

    fn mul(self, rhs: S) -> Self {
        let fct = rhs.into();

        Vector {
            x: self.x * fct.x,
            y: self.y * fct.y,
        }
    }
}
impl<S: Into<Scale2d>> ops::MulAssign<S> for Vector {
    fn mul_assign(&mut self, rhs: S) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);
        let fct = rhs.into();

        self.x = x * fct.x;
        self.y = y * fct.y;
    }
}
impl<S: Into<Scale2d>> ops::Div<S> for Vector {
    type Output = Self;

    fn div(self, rhs: S) -> Self {
        let fct = rhs.into();

        Vector {
            x: self.x / fct.x,
            y: self.y / fct.y,
        }
    }
}
impl<S: Into<Scale2d>> ops::DivAssign<S> for Vector {
    fn div_assign(&mut self, rhs: S) {
        let x = mem::take(&mut self.x);
        let y = mem::take(&mut self.y);
        let fct = rhs.into();

        self.x = x / fct.x;
        self.y = y / fct.y;
    }
}
impl ops::Neg for Vector {
    type Output = Self;

    fn neg(self) -> Self {
        Vector { x: -self.x, y: -self.y }
    }
}

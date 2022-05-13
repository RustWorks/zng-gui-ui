//! Angle, factor, length, time, byte and resolution units.

#[doc(inline)]
pub use zero_ui_view_api::units::*;

mod alignment;
pub use alignment::*;

mod angle;
pub use angle::*;

mod constrains;
pub use constrains::*;

mod byte;
pub use byte::*;

mod factor;
pub use factor::*;

mod grid;
pub use grid::*;

mod length;
pub use length::*;

mod line;
pub use line::*;

mod point;
pub use point::*;

mod rect;
pub use rect::*;

mod resolution;
pub use resolution::*;

mod side_offsets;
pub use side_offsets::*;

mod size;
pub use size::*;

mod text;
pub use text::*;

mod time;
pub use time::*;

mod transform;
pub use transform::*;

mod vector;
pub use vector::*;

/// Minimal difference between values in around the 0.0..=1.0 scale.
const EPSILON: f32 = 0.00001;
/// Minimal difference between values in around the 1.0..=100.0 scale.
const EPSILON_100: f32 = 0.001;

/// Implement From<{tuple of Into<Length>}> and IntoVar for Length compound types.
macro_rules! impl_length_comp_conversions {
    ($(
        $(#[$docs:meta])*
        fn from($($n:ident : $N:ident),+) -> $For:ty {
            $convert:expr
        }
    )+) => {
        $(
            impl<$($N),+> From<($($N),+)> for $For
            where
                $($N: Into<Length>,)+
            {
                $(#[$docs])*
                fn from(($($n),+) : ($($N),+)) -> Self {
                    $convert
                }
            }

            impl<$($N),+> crate::var::IntoVar<$For> for ($($N),+)
            where
            $($N: Into<Length> + Clone,)+
            {
                type Var = crate::var::LocalVar<$For>;

                $(#[$docs])*
                fn into_var(self) -> Self::Var {
                    crate::var::LocalVar(self.into())
                }
            }
        )+
    };
}
use impl_length_comp_conversions;

/// [`f32`] equality used in floating-point [`units`](crate::units).
///
/// * [`NaN`](f32::is_nan) values are equal.
/// * [`INFINITY`](f32::INFINITY) values are equal.
/// * [`NEG_INFINITY`](f32::NEG_INFINITY) values are equal.
/// * Finite values are equal if the difference is less than `epsilon`.
///
/// Note that this definition of equality is symmetric and reflexive, but it is **not** transitive, difference less then
/// epsilon can *accumulate* over a chain of comparisons breaking the transitive property:
///
/// ```
/// # use zero_ui_core::units::about_eq;
/// let e = 0.001;
/// let a = 0.0;
/// let b = a + e - 0.0001;
/// let c = b + e - 0.0001;
///
/// assert!(
///     about_eq(a, b, e) &&
///     about_eq(b, c, e) &&
///     !about_eq(a, c, e)
/// )
/// ```
///
/// See also [`about_eq_hash`].
pub fn about_eq(a: f32, b: f32, epsilon: f32) -> bool {
    if a.is_nan() {
        b.is_nan()
    } else if a.is_infinite() {
        b.is_infinite() && a.is_sign_positive() == b.is_sign_positive()
    } else {
        (a - b).abs() < epsilon
    }
}

/// [`f32`] hash compatible with [`about_eq`] equality.
pub fn about_eq_hash<H: std::hash::Hasher>(f: f32, epsilon: f32, state: &mut H) {
    let (group, f) = if f.is_nan() {
        (0u8, 0u64)
    } else if f.is_infinite() {
        (1, if f.is_sign_positive() { 1 } else { 2 })
    } else {
        let inv_epsi = if epsilon > EPSILON_100 { 100000.0 } else { 100.0 };
        (2, ((f as f64) * inv_epsi) as u64)
    };

    use std::hash::Hash;
    group.hash(state);
    f.hash(state);
}

/// [`f32`] ordering compatible with [`about_eq`] equality.
pub fn about_eq_ord(a: f32, b: f32, epsilon: f32) -> std::cmp::Ordering {
    if about_eq(a, b, epsilon) {
        std::cmp::Ordering::Equal
    } else if a > b {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Less
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{PI, TAU};

    use crate::context::LayoutMetrics;

    use super::*;

    #[test]
    pub fn zero() {
        all_equal(0.rad(), 0.grad(), 0.deg(), 0.turn());
    }

    #[test]
    pub fn half_circle() {
        all_equal(PI.rad(), 200.grad(), 180.deg(), 0.5.turn())
    }

    #[test]
    pub fn full_circle() {
        all_equal(TAU.rad(), 400.grad(), 360.deg(), 1.turn())
    }

    #[test]
    pub fn one_and_a_half_circle() {
        all_equal((TAU + PI).rad(), 600.grad(), 540.deg(), 1.5.turn())
    }

    #[test]
    pub fn modulo_rad() {
        assert_eq!(PI.rad(), (TAU + PI).rad().modulo());
    }

    #[test]
    pub fn modulo_grad() {
        assert_eq!(200.grad(), 600.grad().modulo());
    }

    #[test]
    pub fn modulo_deg() {
        assert_eq!(180.deg(), 540.deg().modulo());
    }

    #[test]
    pub fn modulo_turn() {
        assert_eq!(0.5.turn(), 1.5.turn().modulo());
    }

    #[test]
    pub fn length_expr_same_unit() {
        let a = Length::from(200);
        let b = Length::from(300);
        let c = a + b;

        assert_eq!(c, 500.dip());
    }

    #[test]
    pub fn length_expr_diff_units() {
        let a = Length::from(200);
        let b = Length::from(10.pct());
        let c = a + b;

        assert_eq!(c, Length::Expr(Box::new(LengthExpr::Add(200.into(), 10.pct().into()))))
    }

    #[test]
    pub fn length_expr_eval() {
        let l = (Length::from(200) - 100.pct()).abs();
        let ctx = LayoutMetrics::new(1.0.fct(), PxSize::new(Px(600), Px(400)), Px(0)).with_constrains(|c| c.with_fill(true, true));
        let l = l.layout(ctx.for_x(), Px(0));

        assert_eq!(l.0, (200i32 - 600i32).abs());
    }

    #[test]
    pub fn length_expr_clamp() {
        let l = Length::from(100.pct()).clamp(100, 500);
        assert!(matches!(l, Length::Expr(_)));

        let metrics = LayoutMetrics::new(1.0.fct(), PxSize::new(Px(200), Px(50)), Px(0)).with_constrains(|c| c.with_fill(true, true));

        let r = l.layout(metrics.for_x(), Px(0));
        assert_eq!(r.0, 200);

        let r = l.layout(metrics.for_y(), Px(0));
        assert_eq!(r.0, 100);

        let metrics = metrics.with_constrains(|c| c.with_max_width(Px(550)));
        let r = l.layout(metrics.for_x(), Px(0));
        assert_eq!(r.0, 500);
    }

    fn all_equal(rad: AngleRadian, grad: AngleGradian, deg: AngleDegree, turn: AngleTurn) {
        assert_eq!(rad, AngleRadian::from(grad));
        assert_eq!(rad, AngleRadian::from(deg));
        assert_eq!(rad, AngleRadian::from(turn));

        assert_eq!(grad, AngleGradian::from(rad));
        assert_eq!(grad, AngleGradian::from(deg));
        assert_eq!(grad, AngleGradian::from(turn));

        assert_eq!(deg, AngleDegree::from(rad));
        assert_eq!(deg, AngleDegree::from(grad));
        assert_eq!(deg, AngleDegree::from(turn));

        assert_eq!(turn, AngleTurn::from(rad));
        assert_eq!(turn, AngleTurn::from(grad));
        assert_eq!(turn, AngleTurn::from(deg));
    }
}

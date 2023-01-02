use std::{fmt, mem, ops};

use crate::core::{context::*, units::*, var::impl_from_and_into_var};

/// Defines a placement point in the previous item and the origin point of the next.
///
/// Defining stack direction like this allows expressing the traditional stack directions along an axis, as well as
/// intermediary for transition animations or diagonal directions.
///
/// Note that collapsed items (layout size zero) are skipped, so the previous and next items are both non-empty in layout.
///
/// # Alignment & Spacing
///
/// The direction type can express non-fill alignment and spacing by it self, but prefer using the [`stack::children_align`] and
/// [`stack::spacing`] properties as they are more readable and include fill alignment.
///
/// The [`stack!`] widget implements alignment along the axis that does not change, so if the computed layout vector
/// is zero in a dimension the items can fill in that dimension.
///
/// The [`stack!`] widget adds the spacing along non-zero axis for each item offset after the first, so the spacing is not
/// added for a perfect straight column or row, but it is added even for a single pixel shift *diagonal* stack.
///
/// [`stack::children_align`]: fn@crate::widgets::layouts::stack::children_align
/// [`stack::spacing`]: fn@crate::widgets::layouts::stack::spacing
/// [`stack!`]: mod@crate::widgets::layouts::stack
///
/// # Examples
///
/// TODO, use the ASCII drawings?
#[derive(Default, Clone)]
pub struct StackDirection {
    /// Point on the previous item where the next item is placed.
    pub place: Point,
    /// Point on the next item that is offset to match `place`.
    pub origin: Point,

    /// If `place.x` and `origin.x` are swapped in [`LayoutDirection::RTL`] contexts.
    pub is_rtl_aware: bool,
}

impl fmt::Debug for StackDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("StackDirection")
                .field("place", &self.place)
                .field("origin", &self.origin)
                .field("is_rtl_aware", &self.is_rtl_aware)
                .finish()
        } else if self.is_rtl_aware {
            write!(f, "({:?}, {:?}, {:?})", self.place, self.origin, self.is_rtl_aware)
        } else {
            write!(f, "({:?}, {:?})", self.place, self.origin)
        }
    }
}

impl fmt::Display for StackDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.p$}, {:.p$}", self.place, self.origin, p = p)?;
        } else {
            write!(f, "({}, {}", self.place, self.origin)?;
        }

        if self.is_rtl_aware {
            write!(f, ", {})", self.is_rtl_aware)
        } else {
            write!(f, ")")
        }
    }
}

impl StackDirection {
    /// New custom direction.
    pub fn new<P: Into<Point>, O: Into<Point>>(place: P, origin: O, is_rtl_aware: bool) -> Self {
        Self {
            place: place.into(),
            origin: origin.into(),
            is_rtl_aware,
        }
    }

    /// `((100.pct(), 0), (0, 0))`, items are placed in a row from left to right.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    pub fn left_to_right() -> Self {
        Self {
            place: (100.pct(), 0).into(),
            origin: (0, 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((0, 0), (100.pct(), 0))`, items are placed in a row from right to left.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    pub fn right_to_left() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (100.pct(), 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((100.pct(), 0), (0, 0), true)`, items are placed in a row from left to right or from right to left in RTL contexts.
    ///
    /// In [`LayoutDirection::RTL`] contexts the `place.x` and `origin.x` values are swapped before they are computed.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    pub fn start_to_end() -> Self {
        Self {
            place: (100.pct(), 0).into(),
            origin: (0, 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((0, 0), (100.pct(), 0)), true)`, items are placed in a row from right to left or from left to right in RTL contexts.
    ///
    /// In [`LayoutDirection::RTL`] contexts the `place.x` and `origin.x` values are swapped before they are computed.
    ///
    /// Alignment works on the `y` direction because it is not affected.
    pub fn end_to_start() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (100.pct(), 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `((0, 100.pct()), (0, 0))`, items are placed in a column from top to bottom.
    ///  
    /// Alignment works on the `x` direction because it is not affected.
    pub fn top_to_bottom() -> Self {
        Self {
            place: (0, 100.pct()).into(),
            origin: (0, 0).into(),
            is_rtl_aware: false,
        }
    }

    /// `(0, 0), (0, 100.pct())`, items are placed in a column from bottom to top.
    ///  
    /// Alignment works on the `x` direction because it is not affected.
    pub fn bottom_to_top() -> Self {
        Self {
            place: (0, 0).into(),
            origin: (0, 100.pct()).into(),
            is_rtl_aware: false,
        }
    }

    /// `(0, 0)`, items are just stacked in the Z order.
    ///
    /// Fill alignment works in both dimensions because they don't change.
    ///
    /// Note that items are always rendered in the order defined by the [`z_index`] property.
    ///
    /// [`z_index`]: fn@crate::prelude::z_index
    pub fn none() -> Self {
        Self {
            place: Point::zero(),
            origin: Point::zero(),
            is_rtl_aware: false,
        }
    }

    /// Compute offset of the next item.
    pub fn layout(&self, ctx: &LayoutMetrics, prev_item: PxRect, next_item: PxSize) -> PxVector {
        if self.is_rtl_aware && ctx.direction().is_ltr() {
            let mut d = self.clone();
            mem::swap(&mut d.place.x, &mut d.origin.x);
            d.is_rtl_aware = false;
            return d.layout_resolved_rtl(ctx, prev_item, next_item);
        }

        self.layout_resolved_rtl(ctx, prev_item, next_item)
    }
    pub(crate) fn layout_resolved_rtl(&self, ctx: &LayoutMetrics, prev_item: PxRect, next_item: PxSize) -> PxVector {
        let place = {
            let ctx = ctx.clone().with_constrains(|c| c.with_exact_size(prev_item.size));
            self.place.layout(&ctx, |_| PxPoint::zero())
        };
        let origin = {
            let ctx = ctx.clone().with_constrains(|c| c.with_exact_size(next_item));
            self.origin.layout(&ctx, |_| PxPoint::zero())
        };
        prev_item.origin.to_vector() + place.to_vector() - origin.to_vector()
    }

    /// Direction vector.
    ///
    /// Values are `0`, `1` or `-1`.
    pub fn vector(&self, ctx: LayoutDirection) -> euclid::Vector2D<i8, ()> {
        let size = PxSize::new(Px(10), Px(10));
        let p = self.layout(
            &LayoutMetrics::new(1.fct(), size, Px(10)).with_direction(ctx),
            PxRect::from_size(size),
            size,
        );

        pub(crate) fn v(px: Px) -> i8 {
            use std::cmp::Ordering::*;
            match px.cmp(&Px(0)) {
                Greater => 1,
                Equal => 0,
                Less => -1,
            }
        }
        euclid::vec2(v(p.x), v(p.y))
    }

    /// Filter `align` to the align operations needed to complement the stack direction.
    ///
    /// Alignment only operates in dimensions that have no movement.
    pub fn filter_align(&self, mut align: Align) -> Align {
        let d = self.vector(LayoutDirection::LTR);

        if d.x != 0 {
            align.x = 0.fct();
            align.x_rtl_aware = false;
        }
        if d.y != 0 {
            align.y = 0.fct();
        }

        align
    }

    /// Compute a [`LayoutMask`] that flags all contextual values that affect the result of [`layout`].
    ///
    /// [`layout`]: Self::layout
    pub fn affect_mask(&self) -> LayoutMask {
        self.place.affect_mask() | self.origin.affect_mask()
    }

    /// Returns `true` if all values are [`Length::Default`].
    pub fn is_default(&self) -> bool {
        self.place.is_default() && self.origin.is_default()
    }
}

impl_from_and_into_var! {
    /// New from place and origin, not RTL aware.
    fn from<P: Into<Point>, O: Into<Point>>((origin, size): (P, O)) -> StackDirection {
        (origin, size, false).into()
    }

    /// New from place, origin, and RTL aware flag.
    fn from<P: Into<Point>, O: Into<Point>>((origin, size, rtl_aware): (P, O, bool)) -> StackDirection {
        StackDirection::new(origin, size, rtl_aware)
    }
}

impl<S: Into<Factor2d>> ops::Mul<S> for StackDirection {
    type Output = Self;

    fn mul(mut self, rhs: S) -> Self {
        self *= rhs;
        self
    }
}

impl<'a, S: Into<Factor2d>> ops::Mul<S> for &'a StackDirection {
    type Output = StackDirection;

    fn mul(self, rhs: S) -> Self::Output {
        self.clone() * rhs
    }
}

impl<S: Into<Factor2d>> ops::MulAssign<S> for StackDirection {
    fn mul_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.place *= rhs;
        self.origin *= rhs;
    }
}

impl<S: Into<Factor2d>> ops::Div<S> for StackDirection {
    type Output = Self;

    fn div(mut self, rhs: S) -> Self {
        self /= rhs;
        self
    }
}

impl<'a, S: Into<Factor2d>> ops::Div<S> for &'a StackDirection {
    type Output = StackDirection;

    fn div(self, rhs: S) -> Self::Output {
        self.clone() / rhs
    }
}

impl<S: Into<Factor2d>> ops::DivAssign<S> for StackDirection {
    fn div_assign(&mut self, rhs: S) {
        let rhs = rhs.into();
        self.place /= rhs;
        self.origin /= rhs;
    }
}

impl ops::Add for StackDirection {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl ops::AddAssign for StackDirection {
    fn add_assign(&mut self, rhs: Self) {
        self.place += rhs.place;
        self.origin += rhs.origin;
    }
}

impl ops::Sub for StackDirection {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

impl ops::SubAssign for StackDirection {
    fn sub_assign(&mut self, rhs: Self) {
        self.place -= rhs.place;
        self.origin -= rhs.origin;
    }
}
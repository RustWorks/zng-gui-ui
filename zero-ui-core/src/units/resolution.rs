use std::fmt;

use crate::{var::animation::Transitionable, var::impl_from_and_into_var};

/// Pixels-per-inch resolution.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Transitionable)]
#[serde(transparent)]
pub struct Ppi(pub f32);
impl Ppi {
    /// Returns the minimum of the two resolutions.
    pub fn min(self, other: impl Into<Ppi>) -> Ppi {
        Ppi(self.0.min(other.into().0))
    }

    /// Returns the maximum of the two resolutions.
    pub fn max(self, other: impl Into<Ppi>) -> Ppi {
        Ppi(self.0.max(other.into().0))
    }
}
impl Default for Ppi {
    /// 96ppi.
    fn default() -> Self {
        Ppi(96.0)
    }
}
impl PartialEq for Ppi {
    fn eq(&self, other: &Self) -> bool {
        super::about_eq(self.0, other.0, 0.0001)
    }
}
impl std::hash::Hash for Ppi {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        super::about_eq_hash(self.0, 0.0001, state)
    }
}

/// Pixels-per-meter resolution.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Transitionable)]
#[serde(transparent)]
pub struct Ppm(pub f32);
impl Ppm {
    /// Returns the minimum of the two resolutions.
    pub fn min(self, other: impl Into<Ppm>) -> Ppm {
        Ppm(self.0.min(other.into().0))
    }

    /// Returns the maximum of the two resolutions.
    pub fn max(self, other: impl Into<Ppm>) -> Ppm {
        Ppm(self.0.max(other.into().0))
    }
}
impl Default for Ppm {
    /// 96ppi.
    fn default() -> Self {
        Ppi(96.0).into()
    }
}

/// Extension methods for initializing resolution units.
///
/// # Examples
///
/// ```
/// use zero_ui_core::units::*;
///
/// let ppm: Ppm = 96.dpi().into();
/// ```
pub trait ResolutionUnits {
    /// Pixels-per-inch.
    fn ppi(self) -> Ppi;
    /// Same as [`ppi`].
    ///
    /// [`ppi`]: ResolutionUnits::ppi.
    fn dpi(self) -> Ppi
    where
        Self: Sized,
    {
        self.ppi()
    }

    /// Pixels-per-meter.
    fn ppm(self) -> Ppm;
}
impl ResolutionUnits for u32 {
    fn ppi(self) -> Ppi {
        Ppi(self as f32)
    }

    fn ppm(self) -> Ppm {
        Ppm(self as f32)
    }
}
impl ResolutionUnits for f32 {
    fn ppi(self) -> Ppi {
        Ppi(self)
    }

    fn ppm(self) -> Ppm {
        Ppm(self)
    }
}

impl fmt::Display for Ppi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ppi", self.0)
    }
}
impl fmt::Display for Ppm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ppm", self.0)
    }
}
impl_from_and_into_var! {
    fn from(ppi: Ppi) -> Ppm {
        Ppm(ppi.0 * 39.3701)
    }

    fn from(ppm: Ppm) -> Ppi {
        Ppi(ppm.0 / 39.3701)
    }
}

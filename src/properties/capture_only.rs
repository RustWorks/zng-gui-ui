//! Properties that are only used by widgets directly by capturing them in the `new` or `new_child` function.

use crate::core::{property, types::WidgetId, var::IntoVar};

/// Widget id.
///
/// # Implicit
///
/// All widgets automatically inherit from [`implicit_mixin`](implicit_mixin) that defines an `id`
/// property that maps to this property and sets a default value of `WidgetId::new_unique()`.
///
/// The default widget `new` function captures this `id` property and uses in the default
/// [`Widget`](crate::core::Widget) implementation.
#[property(capture_only)]
pub fn widget_id(id: WidgetId) -> ! {}

/// Stack in-between spacing.
#[property(capture_only)]
pub fn stack_spacing(spacing: impl IntoVar<f32>) -> ! {}

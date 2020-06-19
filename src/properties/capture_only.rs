//! Properties that are only used by widgets directly by capturing then in the `new` or `new_child` function.

use crate::core::{property, types::WidgetId, var::IntoVar, UiNode};

/// Widget id.
///
/// # Placeholder
///
/// This property is a placeholder that does not do anything directly, widgets can
/// capture this value for their own initialization.
///
/// # Implicit
///
/// All widgets automatically inherit from [`implicit_mixin`](implicit_mixin) that defines an `id`
/// property that maps to this property and sets a default value of `WidgetId::new_unique()`.
///
/// The default widget `new` function captures this `id` property and uses in the default
/// [`Widget`](crate::core::Widget) implementation.
#[property(context)]
pub fn widget_id<C: UiNode>(child: C, id: WidgetId) -> C {
    let _id = id;
    error_println!("id property cannot be set directly, must be captured in widget!'s new()");
    child
}

/// Stack in-between spacing.
///
/// # Placeholder
///
/// This property is a placeholder that does not do anything directly, widgets can
/// capture this value for their own initialization.
#[property(context)]
pub fn stack_spacing<C: UiNode>(child: C, spacing: impl IntoVar<f32>) -> C {
    let _spacing = spacing;
    child
}

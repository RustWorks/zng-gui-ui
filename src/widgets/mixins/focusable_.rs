use crate::core::types::{rgba, LayoutSideOffsets};
use crate::core::var::context_var;
use crate::core::widget_mixin;
use crate::properties::{border, focusable, is_focused, BorderDetails};

context_var! {
    pub struct FocusedBorderWidths: LayoutSideOffsets = once LayoutSideOffsets::new_all_same(1.0);
    pub struct FocusedBorderDetails: BorderDetails = once BorderDetails::new_solid_color(rgba(0, 255, 255, 0.7));
}

widget_mixin! {
    /// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
    /// highlight border.
    pub focusable_mixin;

    default {

        /// Enables keyboard focusing in this widget.
        focusable: true;

        /// A Border that is visible when the widget is focused.
        focused_border -> border: {
            widths: LayoutSideOffsets::new_all_same(0.0),
            details: FocusedBorderDetails
        };
    }

    when self.is_focused {
        focused_border: {
            widths: FocusedBorderWidths,
            details: FocusedBorderDetails
        };
    }
}

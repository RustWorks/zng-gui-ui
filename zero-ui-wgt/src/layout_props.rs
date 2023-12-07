use std::fmt;

use zero_ui_layout::context::DIRECTION_VAR;

use crate::prelude::*;

/// Margin space around the widget.
///
/// This property adds side offsets to the widget inner visual, it will be combined with the other
/// layout properties of the widget to define the inner visual position and widget size.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # macro_rules! _demo { () => {
/// Button! {
///     margin = 10;
///     child = Text!("Click Me!")
/// }
/// # }}
/// ```
///
/// In the example the button has `10` layout pixels of space in all directions around it. You can
/// also control each side in specific:
///
/// ```
/// # macro_rules! _demo { () => {
/// Container! {
///     child = Button! {
///         margin = (10, 5.pct());
///         child = Text!("Click Me!")
///     };
///     margin = (1, 2, 3, 4);
/// }
/// # }}
/// ```
///
/// In the example the button has `10` pixels of space above and bellow and `5%` of the container width to the left and right.
/// The container itself has margin of `1` to the top, `2` to the right, `3` to the bottom and `4` to the left.
///
#[property(LAYOUT, default(0))]
pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
    let margin = margin.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&margin);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let margin = margin.layout();
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());
            *desired_size = LAYOUT.with_constraints(LAYOUT.constraints().with_less_size(size_increment), || wm.measure_block(child));
            desired_size.width += size_increment.width;
            desired_size.height += size_increment.height;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let margin = margin.layout();
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());

            *final_size = LAYOUT.with_constraints(LAYOUT.constraints().with_less_size(size_increment), || child.layout(wl));
            let mut translate = PxVector::zero();
            final_size.width += size_increment.width;
            translate.x = margin.left;
            final_size.height += size_increment.height;
            translate.y = margin.top;
            wl.translate(translate);
        }
        _ => {}
    })
}

/// Aligns the widget within the available space.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # macro_rules! _demo { () => {
/// Container! {
///     child = Button! {
///         align = Align::TOP;
///         child = Text!("Click Me!")
///     };
/// }
/// # }}
/// ```
///
/// In the example the button is positioned at the top-center of the container. See [`Align`] for
/// more details.
#[property(LAYOUT, default(Align::FILL))]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Align>) -> impl UiNode {
    let alignment = alignment.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&alignment);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let align = alignment.get();
            let child_size = LAYOUT.with_constraints(align.child_constraints(LAYOUT.constraints()), || wm.measure_block(child));
            *desired_size = align.measure(child_size, LAYOUT.constraints());
        }
        UiNodeOp::Layout { wl, final_size } => {
            let align = alignment.get();
            let child_size = LAYOUT.with_constraints(align.child_constraints(LAYOUT.constraints()), || child.layout(wl));
            let (size, offset, baseline) = align.layout(child_size, LAYOUT.constraints(), LAYOUT.direction());
            wl.translate(offset);
            if baseline {
                wl.translate_baseline(true);
            }
            *final_size = size;
        }
        _ => {}
    })
}

/// If the layout direction is right-to-left.
///
/// The `state` is bound to [`DIRECTION_VAR`].
#[property(LAYOUT)]
pub fn is_rtl(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_is_state(child, DIRECTION_VAR.map(|s| s.is_rtl()), state)
}

/// If the layout direction is left-to-right.
///
/// The `state` is bound to [`DIRECTION_VAR`].
#[property(LAYOUT)]
pub fn is_ltr(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_is_state(child, DIRECTION_VAR.map(|s| s.is_ltr()), state)
}

/// Inline mode explicitly selected for a widget.
///
/// See the [`inline`] property for more details.
///
/// [`inline`]: fn@inline
#[derive(Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InlineMode {
    /// Widget does inline if requested by the parent widget layout and is composed only of properties that support inline.
    ///
    /// This is the default behavior.
    #[default]
    Allow,
    /// Widget always does inline.
    ///
    /// If the parent layout does not setup an inline layout environment the widget it-self will. This
    /// can be used to force the inline visual, such as background clipping or any other special visual
    /// that is only enabled when the widget is inlined.
    ///
    /// Note that the widget will only inline if composed only of properties that support inline.
    Inline,
    /// Widget disables inline.
    ///
    /// If the parent widget requests inline the request does not propagate for child nodes and
    /// inline is disabled on the widget.
    Block,
}
impl fmt::Debug for InlineMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "InlineMode::")?;
        }
        match self {
            Self::Allow => write!(f, "Allow"),
            Self::Inline => write!(f, "Inline"),
            Self::Block => write!(f, "Block"),
        }
    }
}
impl_from_and_into_var! {
    fn from(inline: bool) -> InlineMode {
        if inline {
            InlineMode::Inline
        } else {
            InlineMode::Block
        }
    }
}

/// Enforce an inline mode on the widget.
///
/// Set to [`InlineMode::Inline`] to use the inline layout and visual even if the widget
/// is not in an inlining parent. Note that the widget will still not inline if it has properties
/// that disable inlining.
///
/// Set to [`InlineMode::Block`] to ensure the widget layouts as a block item if the parent
/// is inlining.
///
/// Note that even if set to [`InlineMode::Inline`] the widget will only inline if all properties support
/// inlining.
#[property(WIDGET, default(InlineMode::Allow))]
pub fn inline(child: impl UiNode, mode: impl IntoVar<InlineMode>) -> impl UiNode {
    let mode = mode.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&mode);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = match mode.get() {
                InlineMode::Allow => child.measure(wm),
                InlineMode::Inline => {
                    if LAYOUT.inline_constraints().is_none() {
                        // enable inline for content.
                        wm.with_inline_visual(|wm| child.measure(wm))
                    } else {
                        // already enabled by parent
                        child.measure(wm)
                    }
                }
                InlineMode::Block => {
                    // disable inline, method also disables in `WidgetMeasure`
                    wm.measure_block(child)
                }
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = match mode.get() {
                InlineMode::Allow => child.layout(wl),
                InlineMode::Inline => {
                    if LAYOUT.inline_constraints().is_none() {
                        wl.to_measure(None).with_inline_visual(|wm| child.measure(wm));
                        wl.with_inline_visual(|wl| child.layout(wl))
                    } else {
                        // already enabled by parent
                        child.layout(wl)
                    }
                }
                InlineMode::Block => {
                    if wl.inline().is_some() {
                        tracing::error!("inline enabled in `layout` when it signaled disabled in the previous `measure`");
                        wl.layout_block(child)
                    } else {
                        child.layout(wl)
                    }
                }
            };
        }
        _ => {}
    })
}

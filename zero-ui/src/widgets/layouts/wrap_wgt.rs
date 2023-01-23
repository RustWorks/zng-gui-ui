use crate::prelude::new_widget::*;

use task::parking_lot::Mutex;

/// Wrapping inline layout.
#[widget($crate::widgets::layouts::wrap)]
pub mod wrap {
    use super::*;

    use crate::widgets::text::{LINE_SPACING_VAR, TEXT_ALIGN_VAR};

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Space in-between items and rows.
        ///
        /// This property only defines the spacing for rows of this panel, but it is set
        /// to [`LINE_SPACING_VAR`] for rows and zero for *column space* by default, so you can use
        /// the [`line_spacing`] property if you want to affect all nested wrap and text widgets.
        ///
        /// [`LINE_SPACING_VAR`]: crate::widgets::text::LINE_SPACING_VAR
        /// [`line_spacing`]: fn@crate::widgets::text::txt_align
        pub spacing(impl IntoVar<GridSpacing>);

        /// Children align.
        ///
        /// This property only defines the align for children inside this panel, but it is set
        /// to [`TEXT_ALIGN_VAR`] by default, so you can use the [`txt_align`] property if you want
        /// to affect all nested wrap and text widgets.
        ///
        /// [`TEXT_ALIGN_VAR`]: crate::widgets::text::TEXT_ALIGN_VAR
        /// [`txt_align`]: fn@crate::widgets::text::txt_align
        pub children_align(impl IntoVar<Align>);

        /// Alignment of children in this widget and of nested wrap panels and texts.
        ///
        /// Note that this only sets the [`children_align`] if that property is not set (default) or is set to [`TEXT_ALIGN_VAR`].
        ///
        /// [`children_align`]: fn@children_align
        pub crate::widgets::text::txt_align;

        /// Spacing in-between rows of this widget and of nested wrap panels and texts.
        ///
        /// Note that this only sets the [`row_spacing`] if that property is no set (default), or is set to [`LINE_SPACING_VAR`] mapped to
        /// the [`GridSpacing::row`] value.
        ///
        /// [`row_spacing`]: fn@crate::widgets::text::row_spacing
        /// [`LINE_SPACING_VAR`]: crate::widgets::text::LINE_SPACING_VAR
        pub crate::widgets::text::line_spacing;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let child = node(
                wgt.capture_ui_node_list_or_empty(property_id!(self::children)),
                wgt.capture_var_or_else(property_id!(self::spacing), || {
                    LINE_SPACING_VAR.map(|s| GridSpacing {
                        column: Length::zero(),
                        row: s.clone(),
                    })
                }),
                wgt.capture_var_or_else(property_id!(self::children_align), || TEXT_ALIGN_VAR),
            );
            wgt.set_child(child);
        });
    }
    /// Wrap node.
    ///
    /// Can be used directly to inline widgets without declaring a wrap widget info. In the full `wrap!`
    /// this node is the inner most child of the widget and is wrapped by [`widget_base::nodes::children_layout`].
    pub fn node(children: impl UiNodeList, spacing: impl IntoVar<GridSpacing>, children_align: impl IntoVar<Align>) -> impl UiNode {
        WrapNode {
            children: ZSortingList::new(children),
            spacing: spacing.into_var(),
            children_align: children_align.into_var(),
            layout: Default::default(),
        }
    }
}

#[ui_node(struct WrapNode {
    children: impl UiNodeList,
    #[var] spacing: impl Var<GridSpacing>,
    #[var] children_align: impl Var<Align>,
    layout: Mutex<InlineLayout>
})]
impl UiNode for WrapNode {
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let mut any = false;
        self.children.update_all(ctx, updates, &mut any);

        if any || self.spacing.is_new(ctx) || self.children_align.is_new(ctx) {
            ctx.updates.layout();
        }
    }

    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        let spacing = self.spacing.get().layout(ctx, |_| PxGridSpacing::zero());
        self.layout
            .lock()
            .measure(ctx, wm, &self.children, self.children_align.get(), spacing)
    }

    #[allow_(zero_ui::missing_delegate)] // false positive
    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let spacing = self.spacing.get().layout(ctx, |_| PxGridSpacing::zero());
        self.layout
            .get_mut()
            .layout(ctx, wl, &mut self.children, self.children_align.get(), spacing)
    }
}

/// Info about a row managed by wrap.
#[derive(Default, Debug, Clone, Copy)]
struct RowInfo {
    size: PxSize,
    first_child: usize,
}

#[derive(Default)]
pub struct InlineLayout {
    first_wrapped: bool,
    rows: Vec<RowInfo>,
    desired_size: PxSize,
}
impl InlineLayout {
    pub fn measure(
        &mut self,
        ctx: &mut MeasureContext,
        wm: &mut WidgetMeasure,
        children: &impl UiNodeList,
        child_align: Align,
        spacing: PxGridSpacing,
    ) -> PxSize {
        let constrains = ctx.constrains();

        if let (None, Some(known)) = (ctx.inline_constrains(), constrains.fill_or_exact()) {
            return known;
        }

        self.measure_rows(ctx, children, child_align, spacing);

        if let Some(inline) = wm.inline() {
            inline.first_wrapped = self.first_wrapped;
            inline.last_wrapped = self.rows.len() > 1;
            inline.first = self.rows.first().map(|r| r.size).unwrap_or_default();
            inline.last = self.rows.last().map(|r| r.size).unwrap_or_default();
            inline.first_max_fill = inline.first.width;
            inline.last_max_fill = inline.last.width;
        }

        constrains.clamp_size(self.desired_size)
    }

    pub fn layout(
        &mut self,
        ctx: &mut LayoutContext,
        wl: &mut WidgetLayout,
        children: &mut impl UiNodeList,
        child_align: Align,
        spacing: PxGridSpacing,
    ) -> PxSize {
        if ctx.inline_constrains().is_none() {
            // if not already measured by parent inline
            self.measure_rows(&mut ctx.as_measure(), children, child_align, spacing);
        }

        let direction = ctx.direction();

        let constrains = ctx.constrains();
        let child_align_x = child_align.x(direction);
        let child_align_y = child_align.y();

        let panel_width = constrains.x.fill_or(self.desired_size.width);

        let (first, mid, last) = if let Some(s) = ctx.inline_constrains().map(|c| c.layout()) {
            (s.first, s.mid_clear, s.last)
        } else {
            // define our own first and last
            let mut first = PxRect::from_size(self.rows[0].size);
            let mut last = PxRect::from_size(self.rows.last().unwrap().size);

            first.origin.x = (panel_width - first.size.width) * child_align_x;
            last.origin.x = (panel_width - last.size.width) * child_align_x;
            last.origin.y = self.desired_size.height - last.size.height;

            if let Some(y) = constrains.y.fill_or_exact() {
                let align_y = (y - self.desired_size.height) * child_align_y;
                first.origin.y += align_y;
                last.origin.y += align_y;
            }

            (first, Px(0), last)
        };
        let panel_height = last.origin.y + last.size.height;

        let child_constrains = PxConstrains2d::new_unbounded().with_fill_x(true).with_max_x(panel_width);

        if let Some(inline) = wl.inline() {
            inline.rows.clear();
        }

        ctx.with_constrains(
            |_| child_constrains,
            |ctx| {
                let mut row = first;
                let mut row_advance = Px(0);
                let mut next_row_i = 1;

                children.for_each_mut(|i, child| {
                    if next_row_i < self.rows.len() && self.rows[next_row_i].first_child == i {
                        // new row
                        if let Some(inline) = wl.inline() {
                            inline.rows.push(row);
                        }
                        if next_row_i == self.rows.len() - 1 {
                            row = last;
                        } else {
                            row.origin.y += row.size.height + spacing.row;
                            if next_row_i == 1 {
                                // clear first row
                                row.origin.y += mid;
                            }

                            row.size = self.rows[next_row_i].size;
                            row.origin.x = (panel_width - row.size.width) * child_align_x;
                        }
                        row_advance = Px(0);

                        next_row_i += 1;
                    }

                    let child_inline = child.with_context(|ctx| ctx.widget_info.bounds.measure_inline()).flatten();
                    if let Some(child_inline) = child_inline {
                        let child_desired_size = child
                            .with_context(|c| c.widget_info.bounds.measure_outer_size())
                            .unwrap_or_default();
                        if child_desired_size.is_empty() {
                            // collapsed, continue.
                            wl.collapse_child(ctx, i);
                            return true;
                        }

                        let mut child_first = PxRect::from_size(child_inline.first);
                        let mut child_mid = Px(0);
                        let mut child_last = PxRect::from_size(child_inline.last);

                        if child_inline.last_wrapped {
                            // child wraps

                            debug_assert_eq!(self.rows[next_row_i].first_child, i + 1);

                            child_first.origin.x = row.origin.x + row_advance;
                            if let LayoutDirection::RTL = direction {
                                child_first.origin.x -= row_advance;
                            }
                            child_first.origin.y += (row.size.height - child_first.size.height) * child_align_y;
                            child_mid = (row.size.height - child_first.size.height).max(Px(0));
                            child_last.origin.y = child_desired_size.height - child_last.size.height;

                            let next_row = if next_row_i == self.rows.len() - 1 {
                                last
                            } else {
                                let mut r = row;
                                r.origin.y += child_last.origin.y + spacing.row;
                                r.size = self.rows[next_row_i].size;
                                r.origin.x = (panel_width - r.size.width) * child_align_x;
                                r
                            };
                            child_last.origin.x = next_row.origin.x;
                            if let LayoutDirection::RTL = direction {
                                child_last.origin.x += next_row.size.width - child_last.size.width;
                            }
                            child_last.origin.y += (next_row.size.height - child_last.size.height) * child_align_y;
                            child_last.origin.y += spacing.row;

                            ctx.with_inline(child_first, child_mid, child_last, |ctx| child.layout(ctx, wl));
                            wl.with_outer(child, false, |wl, _| {
                                wl.translate(PxVector::new(Px(0), row.origin.y));
                            });

                            // new row
                            if let Some(inline) = wl.inline() {
                                inline.rows.push(row);
                                child.with_context(|ctx| {
                                    if let Some(inner) = ctx.widget_info.bounds.inline() {
                                        if inner.rows.len() >= 3 {
                                            inline.rows.extend(inner.rows[1..inner.rows.len() - 1].iter().map(|r| {
                                                let mut r = *r;
                                                r.origin.y += row.origin.y;
                                                r
                                            }));
                                        }
                                    } else {
                                        tracing::error!("child inlined in measure, but not in layout")
                                    }
                                });
                            }
                            row = next_row;
                            row_advance = child_last.size.width + spacing.column;
                            next_row_i += 1;
                        } else {
                            // child inlined, but fits in the row

                            let mut offset = PxVector::new(row_advance, Px(0));
                            if let LayoutDirection::RTL = direction {
                                offset.x = row.size.width - child_last.size.width - offset.x;
                            }
                            offset.y = (row.size.height - child_inline.first.height) * child_align_y;

                            ctx.with_constrains(
                                |_| child_constrains.with_fill(false, false).with_max_size(child_inline.first),
                                |ctx| {
                                    ctx.with_inline(child_first, child_mid, child_last, |ctx| child.layout(ctx, wl));
                                },
                            );
                            wl.with_outer(child, false, |wl, _| wl.translate(row.origin.to_vector() + offset));
                            row_advance += child_last.size.width + spacing.column;
                        }
                    } else {
                        // inline block
                        let size = ctx.with_constrains(
                            |_| {
                                child_constrains
                                    .with_fill(false, false)
                                    .with_max(row.size.width - row_advance, row.size.height)
                            },
                            |ctx| ctx.with_inline_constrains(|_| None, |ctx| child.layout(ctx, wl)),
                        );
                        if size.is_empty() {
                            // collapsed, continue.
                            return true;
                        }

                        let mut offset = PxVector::new(row_advance, Px(0));
                        if let LayoutDirection::RTL = direction {
                            offset.x = row.size.width - size.width - offset.x;
                        }
                        offset.y = (row.size.height - size.height) * child_align_y;
                        wl.with_outer(child, false, |wl, _| wl.translate(row.origin.to_vector() + offset));
                        row_advance += size.width + spacing.column;
                    }

                    true
                });

                if let Some(inline) = wl.inline() {
                    // last row
                    inline.rows.push(row);
                }
            },
        );

        constrains.clamp_size(PxSize::new(panel_width, panel_height))
    }

    fn measure_rows(&mut self, ctx: &mut MeasureContext, children: &impl UiNodeList, child_align: Align, spacing: PxGridSpacing) {
        self.rows.clear();
        self.first_wrapped = false;
        self.desired_size = PxSize::zero();

        let constrains = ctx.constrains();
        let inline_constrains = ctx.inline_constrains();
        let child_inline_constrain = constrains.x.max_or(Px::MAX);
        let child_constrains = PxConstrains2d::new_unbounded()
            .with_fill_x(child_align.is_fill_x())
            .with_max_x(child_inline_constrain);
        let mut row = RowInfo::default();
        ctx.with_constrains(
            |_| child_constrains,
            |ctx| {
                children.for_each(|i, child| {
                    let mut inline_constrain = child_inline_constrain;
                    let mut wrap_clear_min = Px(0);
                    if self.rows.is_empty() && !self.first_wrapped {
                        if let Some(InlineConstrains::Measure(InlineConstrainsMeasure {
                            first_max, mid_clear_min, ..
                        })) = inline_constrains
                        {
                            inline_constrain = first_max;
                            wrap_clear_min = mid_clear_min;
                        }
                    }
                    if inline_constrain < Px::MAX {
                        inline_constrain -= row.size.width;
                    }

                    let (inline, size) = ctx.measure_inline(inline_constrain, row.size.height - spacing.row, child);

                    if size.is_empty() {
                        // collapsed, continue.
                        return true;
                    }

                    if let Some(inline) = inline {
                        if inline.first_wrapped {
                            // wrap by us, detected by child
                            if row.size.is_empty() {
                                debug_assert!(self.rows.is_empty());
                                self.first_wrapped = true;
                            } else {
                                row.size.width -= spacing.column;
                                row.size.width = row.size.width.max(Px(0));
                                self.desired_size.width = self.desired_size.width.max(row.size.width);
                                self.desired_size.height += row.size.height + spacing.row;
                                self.rows.push(row);
                            }

                            row.size = inline.first;
                            row.first_child = i;
                        } else {
                            row.size.width += inline.first.width;
                            row.size.height = row.size.height.max(inline.first.height);
                        }

                        if inline.last_wrapped {
                            // wrap by child
                            self.desired_size.width = self.desired_size.width.max(row.size.width);
                            self.desired_size.height += size.height - inline.first.height + spacing.row;

                            self.rows.push(row);
                            row.size = inline.last;
                            row.size.width += spacing.column;
                            row.first_child = i + 1;
                        } else {
                            // child inlined, but fit in row
                            row.size.width += spacing.column;
                        }
                    } else if size.width <= inline_constrain {
                        row.size.width += size.width + spacing.column;
                        row.size.height = row.size.height.max(size.height);
                    } else {
                        // wrap by us
                        if row.size.is_empty() {
                            debug_assert!(self.rows.is_empty());
                            self.first_wrapped = true;
                        } else {
                            row.size.width -= spacing.column;
                            row.size.width = row.size.width.max(Px(0));
                            self.desired_size.width = self.desired_size.width.max(row.size.width);
                            self.desired_size.height += row.size.height.max(wrap_clear_min);
                            if !self.rows.is_empty() || inline_constrains.is_none() {
                                self.desired_size.height += spacing.row;
                            }
                            self.rows.push(row);
                        }

                        row.size = size;
                        row.size.width += spacing.column;
                        row.first_child = i;
                    }

                    true
                });
            },
        );

        // last row
        row.size.width -= spacing.column;
        row.size.width = row.size.width.max(Px(0));
        self.desired_size.width = self.desired_size.width.max(row.size.width);
        self.desired_size.height += row.size.height;
        self.rows.push(row);
    }
}

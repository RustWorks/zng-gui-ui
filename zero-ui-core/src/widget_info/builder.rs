use crate::context::{MeasureContext, StateMapMut};

use super::*;

/// Tag for the [`WidgetInfo::meta`] state-map.
pub enum WidgetInfoMeta {}

/// Widget info tree builder.
///
/// See [`WidgetInfoTree`] for more details.
pub struct WidgetInfoBuilder {
    window_id: WindowId,

    node: tree::NodeId,
    widget_id: WidgetId,
    meta: OwnedStateMap<WidgetInfoMeta>,

    tree: Tree<WidgetInfoData>,
    lookup: IdMap<WidgetId, tree::NodeId>,
    interactivity_filters: InteractivityFilters,

    scale_factor: Factor,

    build_meta: OwnedStateMap<WidgetInfoMeta>,

    build_start: Instant,
    pushed_widgets: u32,

    out_of_bounds: Vec<tree::NodeId>,
}
impl WidgetInfoBuilder {
    /// Starts building a info tree with the root information.
    pub fn new(
        window_id: WindowId,
        root_id: WidgetId,
        root_bounds_info: WidgetBoundsInfo,
        root_border_info: WidgetBorderInfo,
        scale_factor: Factor,
        used_data: Option<UsedWidgetInfoBuilder>,
    ) -> Self {
        let used_data = used_data.unwrap_or_else(UsedWidgetInfoBuilder::fallback);
        let tree = Tree::with_capacity(
            WidgetInfoData {
                widget_id: root_id,
                bounds_info: root_bounds_info,
                border_info: root_border_info,
                meta: Arc::new(OwnedStateMap::new()),
                interactivity_filters: vec![],
                local_interactivity: Interactivity::ENABLED,
                cache: Mutex::new(WidgetInfoCache { interactivity: None }),
            },
            used_data.tree_capacity,
        );
        let mut lookup = IdMap::default();
        lookup.reserve(used_data.tree_capacity);
        let root_node = tree.root().id();
        lookup.insert(root_id, root_node);

        WidgetInfoBuilder {
            window_id,
            node: root_node,
            tree,
            interactivity_filters: Vec::with_capacity(used_data.interactivity_filters_capacity),
            out_of_bounds: Vec::with_capacity(used_data.out_of_bounds_capacity),
            lookup,
            meta: OwnedStateMap::new(),
            widget_id: root_id,
            scale_factor,
            build_meta: OwnedStateMap::new(),
            build_start: Instant::now(),
            pushed_widgets: 1, // root is always new.
        }
    }

    fn node(&mut self, id: tree::NodeId) -> tree::NodeMut<WidgetInfoData> {
        self.tree.index_mut(id)
    }

    /// Current widget id.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Widget tree build metadata.
    ///
    /// This metadata can be modified only by pushed widgets, **not** by the reused widgets.
    pub fn build_meta(&mut self) -> StateMapMut<WidgetInfoMeta> {
        self.build_meta.borrow_mut()
    }

    /// Current widget metadata.
    pub fn meta(&mut self) -> StateMapMut<WidgetInfoMeta> {
        self.meta.borrow_mut()
    }

    /// Calls `f` in a new widget context.
    ///
    /// Only call this in widget node implementations.
    ///
    /// # Panics
    ///
    /// If the `id` was already pushed or reused in this builder.
    pub fn push_widget(&mut self, id: WidgetId, bounds_info: WidgetBoundsInfo, border_info: WidgetBorderInfo, f: impl FnOnce(&mut Self)) {
        let parent_node = self.node;
        let parent_widget_id = self.widget_id;
        let parent_meta = mem::take(&mut self.meta);

        let was_out_of_bounds = bounds_info.is_actually_out_of_bounds();

        self.widget_id = id;
        self.node = self
            .node(parent_node)
            .push_child(WidgetInfoData {
                widget_id: id,
                bounds_info,
                border_info,
                meta: Arc::new(OwnedStateMap::new()),
                interactivity_filters: vec![],
                local_interactivity: Interactivity::ENABLED,
                cache: Mutex::new(WidgetInfoCache { interactivity: None }),
            })
            .id();

        if was_out_of_bounds {
            self.out_of_bounds.push(self.node);
        }

        self.pushed_widgets += 1;

        if self.lookup.insert(id, self.node).is_some() {
            panic!("pushed widget `{id:?}` was already pushed or reused");
        }

        f(self);

        let meta = Arc::new(mem::replace(&mut self.meta, parent_meta));
        let mut node = self.node(self.node);
        node.value().meta = meta;
        node.close();

        self.node = parent_node;
        self.widget_id = parent_widget_id;
    }

    /// Reuse the widget info branch from the previous tree.
    ///
    /// All info state is preserved in the new info tree, all [interactivity filters] registered by the widget also affect
    /// the new info tree.
    ///
    /// Only call this in widget node implementations that monitor the updates requested by their content.
    ///
    /// # Panics
    ///
    /// If the `ctx.path.widget_id()` was already pushed or reused in this builder.
    ///
    /// [interactivity filters]: Self::push_interactivity_filter
    pub fn push_widget_reuse(&mut self, ctx: &mut InfoContext) {
        let widget_id = ctx.path.widget_id();

        debug_assert_ne!(
            self.widget_id, widget_id,
            "can only call `push_widget` or `push_widget_reuse` for each widget"
        );

        let wgt = ctx
            .info_tree
            .get(widget_id)
            .unwrap_or_else(|| panic!("cannot reuse `{:?}`, not found in previous tree", ctx.path));

        self.tree.index_mut(self.node).push_reuse(
            wgt.node(),
            &mut |old_data| {
                let r = old_data.clone();
                r.cache.lock().interactivity = None;
                for filter in &r.interactivity_filters {
                    self.interactivity_filters.push(filter.clone());
                }
                r
            },
            &mut |new_node| {
                let wgt_id = new_node.value().widget_id;
                if self.lookup.insert(wgt_id, new_node.id()).is_some() {
                    panic!("reused widget `{wgt_id:?}` was already pushed or reused");
                }
                if new_node.value().bounds_info.is_actually_out_of_bounds() {
                    self.out_of_bounds.push(new_node.id());
                }
            },
        );
    }

    /// Add the `interactivity` bits to the current widget's interactivity, it will affect the widget and all descendants.
    ///
    /// Also see [`push_interactivity_filter`] to affect the interactivity of widgets outside the current one.
    ///
    /// [`push_interactivity_filter`]: Self::push_interactivity_filter
    pub fn push_interactivity(&mut self, interactivity: Interactivity) {
        let mut node = self.node(self.node);
        let v = node.value();
        v.local_interactivity |= interactivity;
    }

    /// Register a closure that returns the [`Interactivity`] allowed for each widget.
    ///
    /// Widgets [`interactivity`] is computed from all interactivity filters and parents. Interactivity filters are global to the
    /// widget tree, and are re-registered for the tree if the current widget is [reused].
    ///
    /// Note that the filter can make the assumption that parent widgets affect all descendants and if the filter is intended to
    /// affect only the current widget and descendants you can use [`push_interactivity`] instead.
    ///
    /// [`interactivity`]: WidgetInfo::interactivity
    /// [`push_interactivity`]: Self::push_interactivity
    /// [reused]: Self::push_widget_reuse
    pub fn push_interactivity_filter(&mut self, filter: impl Fn(&InteractivityFilterArgs) -> Interactivity + Send + Sync + 'static) {
        let filter = Arc::new(filter);
        self.interactivity_filters.push(filter.clone());
        self.node(self.node).value().interactivity_filters.push(filter);
    }

    /// Calls the `info` closure and returns the range of children inserted by it.
    pub fn with_children_range(&mut self, info: impl FnOnce(&mut Self)) -> ops::Range<usize> {
        let before_count = self.tree.index(self.node).children_count();
        info(self);
        before_count..self.tree.index(self.node).children_count()
    }

    /// Build the info tree.
    pub fn finalize(mut self) -> (WidgetInfoTree, UsedWidgetInfoBuilder) {
        let mut node = self.tree.root_mut();
        let meta = Arc::new(self.meta);
        node.value().meta = meta;
        node.close();

        let r = WidgetInfoTree(Arc::new(WidgetInfoTreeInner {
            window_id: self.window_id,
            lookup: self.lookup,
            interactivity_filters: self.interactivity_filters,
            build_meta: Arc::new(self.build_meta),

            frame: Mutex::new(WidgetInfoTreeFrame {
                stats: WidgetInfoTreeStats::new(self.build_start, self.tree.len() as u32 - self.pushed_widgets),
                stats_update: Default::default(),
                out_of_bounds: Arc::new(self.out_of_bounds),
                out_of_bounds_update: Default::default(),
                scale_factor: self.scale_factor,
                spatial_bounds: PxBox::zero(),
            }),

            tree: self.tree,
        }));

        let cap = UsedWidgetInfoBuilder {
            tree_capacity: r.0.tree.len(),
            interactivity_filters_capacity: r.0.interactivity_filters.len(),
            out_of_bounds_capacity: r.0.frame.lock().out_of_bounds.len(),
        };

        (r, cap)
    }
}

/// Info about the input inline connecting rows of the widget.
#[derive(Clone, Copy, Debug, Default)]
pub struct WidgetInlineMeasure {
    /// Maximum fill width possible on the first row.
    pub first_max_fill: Px,
    /// Maximum fill width possible on the last row.
    pub last_max_fill: Px,

    /// Offset from the bottom of the first row, positive up, that is the baseline of the first item in the row.
    pub first_baseline: Px,
    /// Offset from the bottom of the last row, positive up, that is the baseline of the last item in the row.
    pub last_baseline: Px,

    /// Preferred first size.
    ///
    /// In left-to-right direction the origin is `top_left`, in right-to-left direction the origin is `top_right - first.width`.
    pub first: PxSize,

    /// Indicates that `first` starts in the next row, not in the *current* row defined by the inline constrains.
    pub first_wrapped: bool,

    /// Preferred last size.
    ///
    /// In left-to-right direction the origin is `bottom_left - last.height`, in right-to-left direction
    /// the origin is `bottom_right - last`.
    pub last: PxSize,
}

/// Info about the inlined rows of the widget.
#[derive(Debug, Default)]
pub struct WidgetInlineInfo {
    /// Last layout rows of the widget.
    ///
    /// The rectangles are in the inner bounds space, from top-to-bottom.
    pub rows: Vec<PxRect>,

    /// Widget inner bounds when the rows where last updated.
    pub inner_bounds: PxRect,

    negative_space: Mutex<Option<Arc<Vec<PxRect>>>>,
}
impl WidgetInlineInfo {
    /// Gets the union of all row rectangles.
    pub fn union(&self) -> PxRect {
        self.rows.iter().fold(PxRect::zero(), |union, row| union.union(row))
    }

    /// Gets or computes the negative space of the [`rows`] in the [`inner_bounds`] space, that is, all the areas that are
    /// not covered by any row.
    ///
    /// This is computed on demand and cached.
    ///
    /// [`rows`]: Self::rows
    /// [`inner_bounds`]: Self::inner_bounds
    pub fn negative_space(&self) -> Arc<Vec<PxRect>> {
        let mut space = self.negative_space.lock();
        if space.is_none() {
            *space = Some(Arc::new(self.negatives_enveloped(self.inner_bounds)));
        }
        space.as_ref().unwrap().clone()
    }

    /// Invalidates the [`negative_space`] cache.
    ///
    /// [`negative_space`]: Self::negative_space
    pub fn invalidate_negative_space(&mut self) {
        *self.negative_space.get_mut() = None;
    }

    fn negatives_enveloped(&self, bounds: PxRect) -> Vec<PxRect> {
        let mut space = vec![];

        let bounds_max_x = bounds.max_x();
        let mut last_max_y = bounds.origin.y;

        for r in &self.rows {
            let spacing_y = r.origin.y - last_max_y;
            if spacing_y > Px(0) {
                space.push(PxRect::new(
                    PxPoint::new(bounds.origin.x, last_max_y),
                    PxSize::new(bounds.size.width, spacing_y),
                ));
            }
            last_max_y = r.max_y();

            let left = r.origin.x - bounds.origin.x;
            if left > Px(0) {
                space.push(PxRect::new(
                    PxPoint::new(bounds.origin.x, r.origin.y),
                    PxSize::new(left, r.size.height),
                ));
            }
            let max_x = r.max_x();
            let right = bounds_max_x - max_x;
            if right > Px(0) {
                space.push(PxRect::new(PxPoint::new(max_x, r.origin.y), PxSize::new(right, r.size.height)));
            }
        }
        let spacing_y = bounds.max_y() - last_max_y;
        if spacing_y > Px(0) {
            space.push(PxRect::new(
                PxPoint::new(bounds.origin.x, last_max_y),
                PxSize::new(bounds.size.width, spacing_y),
            ));
        }
        space
    }
}

/// Represents the in-progress measure pass for a widget tree.
#[derive(Default)]
pub struct WidgetMeasure {
    inline: Option<WidgetInlineMeasure>,
}
impl WidgetMeasure {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// If the parent widget is doing inline flow layout.
    pub fn is_inline(&self) -> bool {
        self.inline.is_some()
    }

    /// Mutable reference to the current widget's inline info.
    ///
    /// The widget must configure this to be inlined in parent layout. This is only `Some(_)` if inline is enabled.
    ///
    /// See [`WidgetInlineMeasure`] for more details.
    pub fn inline(&mut self) -> Option<&mut WidgetInlineMeasure> {
        self.inline.as_mut()
    }

    /// Sets [`is_inline`] to `false`.
    ///
    /// Must be called before child delegation, otherwise children that inline may render expecting to fit in
    /// the inline flow.
    ///
    /// [`is_inline`]: Self::is_inline
    pub fn disable_inline(&mut self) {
        self.inline = None;
    }

    /// Measure an widget.
    ///
    /// The `reuse` flag indicates if the cached measure or layout size can be returned instead of calling `measure`. It should
    /// only be `false` if the widget has a pending layout request.
    pub fn with_widget(
        &mut self,
        ctx: &mut MeasureContext,
        reuse: bool,
        measure: impl FnOnce(&mut MeasureContext, &mut Self) -> PxSize,
    ) -> PxSize {
        let snap = ctx.metrics.snapshot();
        if reuse {
            let measure_uses = ctx.widget_info.bounds.measure_metrics_used();
            if ctx
                .widget_info
                .bounds
                .measure_metrics()
                .map(|m| m.masked_eq(&snap, measure_uses))
                .unwrap_or(false)
            {
                let mut reused = false;
                if let Some(inline) = self.inline() {
                    if let Some(prev) = ctx.widget_info.bounds.measure_inline() {
                        *inline = prev;
                        reused = true;
                    }
                } else {
                    reused = ctx.widget_info.bounds.measure_inline().is_none();
                }

                if reused {
                    return ctx.widget_info.bounds.measure_outer_size();
                }
            }
        }

        let parent_inline = self.inline.take();
        let parent_uses = ctx.metrics.enter_widget_ctx();
        if ctx.inline_constrains().is_some() {
            self.inline = Some(Default::default());
        }

        let size = measure(ctx, self);

        let measure_uses = ctx.metrics.exit_widget_ctx(parent_uses);
        ctx.widget_info.bounds.set_measure_metrics(Some(snap), measure_uses);
        ctx.widget_info.bounds.set_measure_outer_size(size);
        ctx.widget_info.bounds.set_measure_inline(self.inline.take());
        self.inline = parent_inline;

        size
    }
}

/// Represents the in-progress layout pass for a widget tree.
pub struct WidgetLayout {
    t: WidgetLayoutTranslation,
    known_collapsed: bool,
    known_child_offset_changed: i32,
    child_offset_changed: i32,
    inline: Option<WidgetInlineInfo>,
}
impl WidgetLayout {
    /// Defines the root widget outer-bounds scope.
    ///
    /// The default window implementation calls this.
    pub fn with_root_widget(
        ctx: &mut LayoutContext,
        pass_id: LayoutPassId,
        layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize,
    ) -> PxSize {
        let mut wl = Self {
            t: WidgetLayoutTranslation {
                pass_id,
                offset_buf: PxVector::zero(),
                baseline: Px(0),
                offset_baseline: false,
                can_auto_hide: true,
                known: None,
                known_target: KnownTarget::Outer,
            },
            known_collapsed: false,
            known_child_offset_changed: 0,
            child_offset_changed: 0,
            inline: None,
        };
        let size = wl.with_widget(ctx, false, layout);
        wl.finish_known();
        if wl.child_offset_changed > 0 {
            ctx.updates.render_update();
        }
        size
    }

    fn finish_known(&mut self) {
        if let Some(bounds) = self.known.take() {
            if let KnownTarget::Outer = self.known_target {
                self.child_offset_changed += bounds.end_pass();
                let childs_changed = mem::take(&mut self.known_child_offset_changed) > 0;
                if childs_changed {
                    self.child_offset_changed += 1;
                    bounds.set_changed_child();
                }
            }
        }
    }

    /// Defines a widget outer-bounds scope, applies pending translations to the outer offset,
    /// calls `layout`, then sets the translation target to the outer bounds.
    ///
    /// If `reuse` is `true` and none of the used metrics have changed skips calling `layout` and returns the current outer-size, the
    /// outer transform is still updated.
    ///
    /// The default widget constructor calls this, see [`widget_base::nodes::widget`].
    ///
    /// [`widget_base::nodes::widget`]: crate::widget_base::nodes::widget
    pub fn with_widget(
        &mut self,
        ctx: &mut LayoutContext,
        reuse: bool,
        layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize,
    ) -> PxSize {
        self.finish_known(); // in case of WidgetList.
        self.baseline = Px(0);
        self.offset_baseline = false;
        self.can_auto_hide = true;
        let parent_child_offset_changed = mem::take(&mut self.child_offset_changed);
        let parent_inline = self.inline.take();

        ctx.widget_info.bounds.begin_pass(self.pass_id); // record prev state

        // drain preview translations.
        ctx.widget_info.bounds.set_outer_offset(mem::take(&mut self.offset_buf));

        let snap = ctx.metrics.snapshot();
        let mut uses = ctx.widget_info.bounds.metrics_used();
        let mut size = PxSize::zero();
        let mut reused = false;

        if reuse && ctx.widget_info.bounds.metrics().map(|m| m.masked_eq(&snap, uses)).unwrap_or(false) {
            size = ctx.widget_info.bounds.outer_size();
            reused = true;
        }

        if !reused {
            if ctx.inline_constrains().is_some() {
                self.inline = ctx.widget_info.bounds.take_inline();
                if self.inline.is_none() {
                    self.inline = Some(Default::default());
                }
            }

            let parent_uses = ctx.metrics.enter_widget_ctx();
            size = layout(ctx, self);
            uses = ctx.metrics.exit_widget_ctx(parent_uses);

            ctx.widget_info.bounds.set_outer_size(size);
            ctx.widget_info.bounds.set_inline(self.inline.take());
        }

        ctx.widget_info.bounds.set_metrics(Some(snap), uses);

        // setup returning translations target.
        self.finish_known();
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Outer;
        self.known_child_offset_changed = self.child_offset_changed;

        self.child_offset_changed += parent_child_offset_changed; // when parent inner closes this the flag is for the parent not this
        self.inline = parent_inline;

        size
    }

    /// Defines a widget inner-bounds scope, applies pending transforms to the inner transform,
    /// calls `layout`, then sets the transform target to the inner transform.
    ///
    /// This method also updates the border info.
    ///
    /// The default widget borders constructor calls this, see [`widget_base::nodes::inner`].
    ///
    /// [`widget_base::nodes::inner`]: crate::widget_base::nodes::inner
    pub fn with_inner(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> PxSize {
        #[cfg(debug_assertions)]
        if self.known.is_some() {
            tracing::error!("widget `{:?}` started inner bounds in the return path of another bounds", ctx.path)
        }
        self.finish_known();

        // drain preview translations.
        ctx.widget_info.bounds.set_inner_offset(mem::take(&mut self.offset_buf));
        ctx.widget_info.bounds.set_baseline(mem::take(&mut self.baseline));
        ctx.widget_info
            .bounds
            .set_inner_offset_baseline(mem::take(&mut self.offset_baseline));
        ctx.widget_info
            .bounds
            .set_can_auto_hide(mem::replace(&mut self.can_auto_hide, true));

        let size = ContextBorders::with_inner(ctx, |ctx| layout(ctx, self));

        ctx.widget_info.bounds.set_inner_size(size);

        // setup returning translations target.
        self.finish_known();
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Inner;

        size
    }

    /// Defines a widget child scope, drops the current layout target, calls `layout`, then returns the child size and
    /// `true` if there was no child widget inside `layout` and so the caller must render the [`child_offset`].
    ///
    /// If no inner widget is found and the baseline is set during the call to `layout` the baseline is set to the current widget's inner bounds.
    ///
    /// The default widget child layout constructor implements this, see [`widget_base::nodes::child_layout`].
    ///
    /// [`widget_base::nodes::child_layout`]: crate::widget_base::nodes::child_layout
    /// [`child_offset`]: WidgetBoundsInfo::child_offset
    pub fn with_child(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> (PxSize, bool) {
        self.finish_known(); // in case of WidgetList?

        let size = layout(ctx, self);

        let collapse = mem::take(&mut self.known_collapsed);
        if self.known.is_none() && !collapse {
            ctx.widget_info.bounds.set_child_offset(mem::take(&mut self.offset_buf));
            ctx.widget_info.bounds.set_baseline(mem::take(&mut self.baseline));
            ctx.widget_info
                .bounds
                .set_inner_offset_baseline(mem::take(&mut self.offset_baseline));
            ctx.widget_info
                .bounds
                .set_can_auto_hide(mem::replace(&mut self.can_auto_hide, true));

            // setup returning translations target.
            self.finish_known();
            self.known = Some(ctx.widget_info.bounds.clone());
            self.known_target = KnownTarget::Child;

            (size, true)
        } else {
            (size, false)
        }
    }

    /// Defines a widget children scope, drops the current layout target, calls `layout`, then intercepts all translations
    /// targeting the *child outer*, returns the panel node size.
    ///
    /// The caller must render the [`child_offset`].
    ///
    /// The [`widget_base::nodes::children_layout`] implements children bounds
    ///
    /// [`widget_base::nodes::children_layout`]: crate::widget_base::nodes::children_layout
    /// [`child_offset`]: WidgetBoundsInfo::child_offset
    pub fn with_children(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> PxSize {
        #[cfg(debug_assertions)]
        if self.known.is_some() {
            tracing::error!(
                "widget `{:?}` started children bounds in the return path of another bounds",
                ctx.path
            )
        }
        self.finish_known();

        // drain preview translations.
        ctx.widget_info.bounds.set_child_offset(mem::take(&mut self.offset_buf));

        let r = layout(ctx, self);

        // setup returning translations target.
        self.finish_known();
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Child;

        r
    }

    /// Overwrite the widget's outer translate, the `translate` closure is called with the
    /// [`WidgetLayoutTranslation`] set to apply directly to the `widget` outer info, after it returns `self` has
    /// the same state it had before.
    ///
    /// This is a limited version of the [`with_child`] method, useful for cases where multiple children need
    /// to be layout first before each child's position can be computed, in these scenarios this method avoids a second
    /// layout pass by using the [`UiNode::with_context`] to access and replace the outer transform.
    ///
    /// If `keep_previous` is `true` the new offset is *added* to the previous.
    ///
    /// Returns `None` if the `widget` node is not actually an widget.
    ///
    /// [`with_child`]: Self::with_child
    pub fn with_outer<N: UiNode, R>(
        &mut self,
        widget: &mut N,
        keep_previous: bool,
        translate: impl FnOnce(&mut WidgetLayoutTranslation, &mut N) -> R,
    ) -> Option<R> {
        let bounds = widget.with_context(|n| n.widget_info.bounds.clone())?;
        let r = self.with_outer_impl(bounds, widget, keep_previous, translate);
        Some(r)
    }

    fn with_outer_impl<T, R>(
        &mut self,
        bounds: WidgetBoundsInfo,
        target: &mut T,
        keep_previous: bool,
        translate: impl FnOnce(&mut WidgetLayoutTranslation, &mut T) -> R,
    ) -> R {
        bounds.begin_pass(self.pass_id);

        if !keep_previous {
            bounds.set_outer_offset(PxVector::zero());
        }

        let mut wl = WidgetLayoutTranslation {
            pass_id: self.pass_id,
            offset_buf: PxVector::zero(),
            offset_baseline: false,
            can_auto_hide: true,
            baseline: Px(0),
            known: Some(bounds),
            known_target: KnownTarget::Outer,
        };

        let size = translate(&mut wl, target);

        self.child_offset_changed += wl.known.unwrap().end_pass();

        size
    }

    /// Collapse the layout of `self` and descendants, the size and offsets are set to zero.
    ///
    /// Nodes that set the visibility to the equivalent of [`Collapsed`] must skip layout and return [`PxSize::zero`] as
    /// the the size, ignoring the min-size constrains, and call this method to update all the descendant
    /// bounds information to be a zero-sized point.
    ///
    /// Note that the widget will automatically not be rendered when collapsed.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse(&mut self, ctx: &mut LayoutContext) {
        self.finish_known();
        self.known_collapsed = true;

        let widget_id = ctx.path.widget_id();
        if let Some(w) = ctx.info_tree.get(widget_id) {
            for w in w.self_and_descendants() {
                let info = w.info();
                info.bounds_info.set_outer_size(PxSize::zero());
                info.bounds_info.set_inner_size(PxSize::zero());
                info.bounds_info.set_baseline(Px(0));
                info.bounds_info.set_inner_offset_baseline(false);
                info.bounds_info.set_can_auto_hide(true);
                info.bounds_info.set_outer_offset(PxVector::zero());
                info.bounds_info.set_inner_offset(PxVector::zero());
                info.bounds_info.set_child_offset(PxVector::zero());
                info.bounds_info.set_measure_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_is_collapsed(true);
            }
        } else {
            tracing::error!("collapse did not find `{}` in the info tree", widget_id)
        }
    }

    /// Collapse layout of all descendants, the size and offsets are set to zero.
    ///
    /// Widgets that control the visibility of their children can use this method and then, in the same layout pass, layout
    /// the children that should be visible.
    ///
    /// Note that the widgets will automatically not be rendered when collapsed.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse_descendants(&mut self, ctx: &mut LayoutContext) {
        let widget_id = ctx.path.widget_id();
        if let Some(w) = ctx.info_tree.get(widget_id) {
            for w in w.descendants() {
                let info = w.info();
                info.bounds_info.set_outer_size(PxSize::zero());
                info.bounds_info.set_inner_size(PxSize::zero());
                info.bounds_info.set_baseline(Px(0));
                info.bounds_info.set_inner_offset_baseline(false);
                info.bounds_info.set_can_auto_hide(true);
                info.bounds_info.set_outer_offset(PxVector::zero());
                info.bounds_info.set_inner_offset(PxVector::zero());
                info.bounds_info.set_child_offset(PxVector::zero());
                info.bounds_info.set_measure_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_is_collapsed(true);
            }
        } else {
            tracing::error!("collapse_descendants did not find `{}` in the info tree", widget_id)
        }
    }

    /// Collapse layout of the child and all its descendants, the size and offsets are set to zero.
    ///
    /// Widgets that control the visibility of their children can use this method and then, in the same layout pass, layout
    /// the children that should be visible.
    ///
    /// Note that the widgets will automatically not be rendered when collapsed.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse_child(&mut self, ctx: &mut LayoutContext, index: usize) {
        let widget_id = ctx.path.widget_id();
        if let Some(w) = ctx.info_tree.get(widget_id) {
            if let Some(w) = w.children().nth(index) {
                for w in w.self_and_descendants() {
                    let info = w.info();
                    info.bounds_info.set_outer_size(PxSize::zero());
                    info.bounds_info.set_inner_size(PxSize::zero());
                    info.bounds_info.set_baseline(Px(0));
                    info.bounds_info.set_inner_offset_baseline(false);
                    info.bounds_info.set_can_auto_hide(true);
                    info.bounds_info.set_outer_offset(PxVector::zero());
                    info.bounds_info.set_inner_offset(PxVector::zero());
                    info.bounds_info.set_child_offset(PxVector::zero());
                    info.bounds_info.set_measure_metrics(None, LayoutMask::NONE);
                    info.bounds_info.set_metrics(None, LayoutMask::NONE);
                    info.bounds_info.set_is_collapsed(true);
                }
            } else {
                tracing::error!(
                    "collapse_child out-of-bounds for `{}` in the children of `{}` in the info tree",
                    index,
                    widget_id
                )
            }
        } else {
            tracing::error!("collapse_child did not find `{}` in the info tree", widget_id)
        }
    }

    /// If the parent widget is doing inline layout and this widget signaled that it can support this
    /// during measure.
    ///
    /// See [`WidgetMeasure::inline`] for more details.
    pub fn is_inline(&self) -> bool {
        self.inline.is_some()
    }

    /// Mutable reference to the current widget's inline info.
    ///
    /// If the parent widget is doing inline layout and this widget signaled that it can support this
    /// during measure. You can use [`WidgetMeasure::disable_inline`] in the measure pass to layout as inline-block.
    ///
    /// When this is `Some(_)` the [`LayoutMetrics::inline_constrains`] is also `Some(_)`.
    ///
    /// See [`WidgetInlineInfo`] for more details.
    ///
    /// [`LayoutMetrics::inline_constrains`]: crate::context::LayoutMetrics::inline_constrains
    pub fn inline(&mut self) -> Option<&mut WidgetInlineInfo> {
        self.inline.as_mut()
    }
}
impl ops::Deref for WidgetLayout {
    type Target = WidgetLayoutTranslation;

    fn deref(&self) -> &Self::Target {
        &self.t
    }
}
impl ops::DerefMut for WidgetLayout {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.t
    }
}

enum KnownTarget {
    Outer,
    Inner,
    Child,
}

/// Identifies the layout pass of a window.
///
/// This value is different for each window layout, but the same for children of panels that do more then one layout pass.
pub type LayoutPassId = u32;

/// Mutable access to the offset of a widget bounds in [`WidgetLayout`].
///
/// Note that [`WidgetLayout`] dereferences to this type.
pub struct WidgetLayoutTranslation {
    pass_id: LayoutPassId,
    offset_buf: PxVector,
    baseline: Px,
    offset_baseline: bool,
    can_auto_hide: bool,

    known: Option<WidgetBoundsInfo>,
    known_target: KnownTarget,
}
impl WidgetLayoutTranslation {
    /// Gets the current window layout pass.
    ///
    /// Widgets can be layout more then once per window layout pass, you can use this ID to identify such cases.
    pub fn pass_id(&self) -> LayoutPassId {
        self.pass_id
    }

    /// Adds the `offset` to the closest *inner* bounds offset.
    pub fn translate(&mut self, offset: PxVector) {
        if let Some(info) = &self.known {
            match self.known_target {
                KnownTarget::Outer => {
                    let mut o = info.outer_offset();
                    o += offset;
                    info.set_outer_offset(o);
                }
                KnownTarget::Inner => {
                    let mut o = info.inner_offset();
                    o += offset;
                    info.set_inner_offset(o);
                }
                KnownTarget::Child => {
                    let mut o = info.child_offset();
                    o += offset;
                    info.set_child_offset(o);
                }
            }
        } else {
            self.offset_buf += offset;
        }
    }

    /// Set the baseline offset of the closest *inner* bounds. The offset is up from the bottom of the bounds.
    pub fn set_baseline(&mut self, baseline: Px) {
        if let Some(info) = &self.known {
            info.set_baseline(baseline);
        } else {
            self.baseline = baseline;
        }
    }

    /// If the inner offset of the last visited widget is added by its baseline on the *y* axis.
    pub fn translate_baseline(&mut self, enabled: bool) {
        if let Some(info) = &self.known {
            info.set_inner_offset_baseline(enabled);
        } else {
            self.offset_baseline = enabled;
        }
    }

    /// Sets if the widget only renders if [`outer_bounds`] intersects with the [`FrameBuilder::auto_hide_rect`].
    ///
    /// This is `true` by default.
    ///
    /// [`outer_bounds`]: WidgetBoundsInfo::outer_bounds
    /// [`FrameBuilder::auto_hide_rect`]: crate::render::FrameBuilder::auto_hide_rect
    pub fn allow_auto_hide(&mut self, enabled: bool) {
        if let Some(info) = &self.known {
            info.set_can_auto_hide(enabled)
        } else {
            self.can_auto_hide = enabled;
        }
    }
}

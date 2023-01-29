use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    mem, ops,
    sync::Arc,
};

use crate::{
    context_local, property,
    render::FrameValueKey,
    var::{IntoVar, Var},
};

use super::*;

/// Creates an [`UiNodeVec`] containing the arguments.
///  
/// Note that the items can be any type, `ui_vec!` automatically calls [`UiNode::boxed`] for each item.
///
/// # Examples
///
/// Create a vec containing a list of nodes/widgets:
///
/// ```
/// # use zero_ui_core::widget_instance::*;
/// # use zero_ui_core::widget_base::*;
/// # macro_rules! text { ($($tt:tt)*) => { NilUiNode } }
/// # use text as foo;
/// # use text as bar;
/// let widgets = ui_vec![
///     foo!("Hello"),
///     bar!("World!")
/// ];
/// ```
///
/// Create a vec containing the node repeated **n** times:
///
/// ```
/// # use zero_ui_core::widget_instance::*;
/// # use zero_ui_core::widget_base::*;
/// # macro_rules! text { ($($tt:tt)*) => { NilUiNode } }
/// # use text as foo;
/// let widgets = ui_vec![foo!(" . "); 10];
/// ```
///
/// Note that this is different from `vec![item; n]`, the node is not cloned, the expression is called **n** times to
/// generate the nodes.
#[macro_export]
macro_rules! ui_vec {
    () => { $crate::widget_instance::UiNodeVec::new() };
    ($node:expr; $n:expr) => {
        {
            let mut n: usize = $n;
            let mut vec = UiNodeVec::with_capacity(n);
            while n > 0 {
                vec.push($node);
                n -= 1;
            }
            vec
        }
    };
    ($($node:expr),+ $(,)?) => {
        $crate::widget_instance::UiNodeVec(vec![
            $($crate::widget_instance::UiNode::boxed($node)),*
        ])
    };
}
#[doc(inline)]
pub use crate::ui_vec;

impl UiNodeList for Vec<BoxedUiNode> {
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        f(&self[index])
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        f(&mut self[index])
    }

    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        for (i, node) in self.iter().enumerate() {
            if !f(i, node) {
                break;
            }
        }
    }

    fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        for (i, node) in self.iter_mut().enumerate() {
            if !f(i, node) {
                break;
            }
        }
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        vec.append(self)
    }
}

/// Vec of boxed UI nodes.
///
/// This is a thin wrapper around `Vec<BoxedUiNode>` that adds helper methods for pushing widgets without needing to box.
#[derive(Default)]
pub struct UiNodeVec(pub Vec<BoxedUiNode>);
impl UiNodeVec {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// New [`with_capacity`].
    ///
    /// [`with_capacity`]: Vec::with_capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Box and [`push`] the node.
    ///
    /// [`push`]: Vec::push
    pub fn push(&mut self, node: impl UiNode) {
        self.0.push(node.boxed())
    }

    /// Box and [`insert`] the node.
    ///
    /// [`insert`]: Vec::insert
    pub fn insert(&mut self, index: usize, node: impl UiNode) {
        self.0.insert(index, node.boxed())
    }
}
impl ops::Deref for UiNodeVec {
    type Target = Vec<BoxedUiNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for UiNodeVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<Vec<BoxedUiNode>> for UiNodeVec {
    fn from(vec: Vec<BoxedUiNode>) -> Self {
        Self(vec)
    }
}
impl From<UiNodeVec> for Vec<BoxedUiNode> {
    fn from(vec: UiNodeVec) -> Self {
        vec.0
    }
}
impl FromIterator<BoxedUiNode> for UiNodeVec {
    fn from_iter<T: IntoIterator<Item = BoxedUiNode>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl UiNodeList for UiNodeVec {
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        self.0.with_node(index, f)
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.0.with_node_mut(index, f)
    }

    fn for_each<F>(&self, f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        self.0.for_each(f)
    }

    fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        self.0.for_each_mut(f)
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        self.0.boxed()
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.0.drain_into(vec)
    }
}

/// Adds the `chain` method for all [`UiNodeList`] implementers.
pub trait UiNodeListChain: UiNodeList {
    /// Creates a new [`UiNodeList`] that chains `self` and `other`.
    ///
    /// Special features of each inner list type is preserved.
    fn chain<B>(self, other: B) -> UiNodeListChainImpl<Self, B>
    where
        B: UiNodeList,
        Self: Sized;
}
impl<A: UiNodeList> UiNodeListChain for A {
    fn chain<B>(self, other: B) -> UiNodeListChainImpl<Self, B>
    where
        B: UiNodeList,
    {
        UiNodeListChainImpl(self, other)
    }
}

/// Implements [`UiNodeListChain`].
pub struct UiNodeListChainImpl<A: UiNodeList, B: UiNodeList>(pub A, pub B);
impl<A: UiNodeList, B: UiNodeList> UiNodeList for UiNodeListChainImpl<A, B> {
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        assert_bounds(self.len(), index);

        if index < self.0.len() {
            self.0.with_node(index, f)
        } else {
            self.1.with_node(index - self.0.len(), f)
        }
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        assert_bounds(self.len(), index);

        if index < self.0.len() {
            self.0.with_node_mut(index, f)
        } else {
            self.1.with_node_mut(index - self.0.len(), f)
        }
    }

    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        let mut continue_iter = true;
        self.0.for_each(|i, n| {
            continue_iter = f(i, n);
            continue_iter
        });

        if continue_iter {
            let offset = self.0.len();
            self.1.for_each(move |i, n| f(i + offset, n))
        }
    }

    fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        let mut continue_iter = true;
        self.0.for_each_mut(|i, n| {
            continue_iter = f(i, n);
            continue_iter
        });

        if continue_iter {
            let offset = self.0.len();
            self.1.for_each_mut(move |i, n| f(i + offset, n))
        }
    }

    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.0.drain_into(vec);
        self.1.drain_into(vec);
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.0.init_all(ctx);
        self.1.init_all(ctx);
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.0.deinit_all(ctx);
        self.1.deinit_all(ctx);
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.0.event_all(ctx, update);
        self.1.event_all(ctx, update);
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.0.update_all(ctx, updates, observer);
        self.1.update_all(ctx, updates, &mut OffsetUiListObserver(self.0.len(), observer));
    }

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.0.render_all(ctx, frame);
        self.1.render_all(ctx, frame);
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.0.render_update_all(ctx, update);
        self.1.render_update_all(ctx, update);
    }
}

/// Represents the contextual parent [`SortingList`] during a list.
pub struct SortingListParent {}
impl SortingListParent {
    /// If the current call has a parent list.
    pub fn is_inside_list() -> bool {
        SORTING_LIST_PARENT.get().is_some()
    }

    /// Calls [`SortingList::invalidate_sort`] on the parent list.
    pub fn invalidate_sort() {
        if let Some(s) = &mut *SORTING_LIST_PARENT.write() {
            *s = true;
        }
    }

    fn with<R>(action: impl FnOnce() -> R) -> (R, bool) {
        let mut resort = Some(false);
        let r = SORTING_LIST_PARENT.with_context_opt(&mut resort, action);
        (r, resort.unwrap())
    }
}
context_local! {
    static SORTING_LIST_PARENT: Option<bool> = false;
}

/// Represents a sorted view into an [`UiNodeList`] that is not changed.
///
/// Note that the `*_all` methods are not sorted, only the other accessors map to the sorted position of nodes. The sorting is lazy
/// and gets invalidated on every init and every time there are changes observed in [`update_all`].
///
/// [`update_all`]: UiNodeList
pub struct SortingList<L, S>
where
    L: UiNodeList,
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + Send + 'static,
{
    list: L,

    map: RefCell<Vec<usize>>,
    sort: S,
}
impl<L, S> SortingList<L, S>
where
    L: UiNodeList,
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + Send + 'static,
{
    /// New from list and sort function.
    pub fn new(list: L, sort: S) -> Self {
        Self {
            list,
            map: RefCell::new(vec![]),
            sort,
        }
    }

    fn update_map(&self) {
        let mut map = self.map.borrow_mut();
        let len = self.list.len();

        if len == 0 {
            map.clear();
        } else if map.len() != len {
            map.clear();
            map.extend(0..len);
            map.sort_by(|&a, &b| self.list.with_node(a, |a| self.list.with_node(b, |b| (self.sort)(a, b))))
        }
    }

    /// Borrow the inner list.
    pub fn list(&self) -> &L {
        &self.list
    }

    /// Mutable borrow the inner list.
    ///
    /// You must call [`invalidate_sort`] if any modification may have affected sort without changing the list length.
    ///
    /// [`invalidate_sort`]: Self::invalidate_sort
    pub fn list_mut(&mut self) -> &mut L {
        &mut self.list
    }

    /// Invalidate the sort, the list will resort on the nest time the sorted positions are needed.
    ///
    /// Note that you can also invalidate sort from the inside using [`SortingListParent::invalidate_sort`].
    pub fn invalidate_sort(&self) {
        self.map.borrow_mut().clear()
    }

    fn with_map<R>(&self, f: impl FnOnce(&[usize], &L) -> R) -> R {
        self.update_map();

        let (r, resort) = SortingListParent::with(|| {
            let map = self.map.borrow();
            f(&map, &self.list)
        });

        if resort {
            self.invalidate_sort();
        }

        r
    }

    fn with_map_mut<R>(&mut self, f: impl FnOnce(&[usize], &mut L) -> R) -> R {
        self.update_map();

        let (r, resort) = SortingListParent::with(|| {
            let map = self.map.borrow();
            f(&map, &mut self.list)
        });

        if resort {
            self.invalidate_sort();
        }

        r
    }
}
impl<L, S> UiNodeList for SortingList<L, S>
where
    L: UiNodeList,
    S: Fn(&BoxedUiNode, &BoxedUiNode) -> Ordering + Send + 'static,
{
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        self.with_map(|map, list| list.with_node(map[index], f))
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.with_map_mut(|map, list| list.with_node_mut(map[index], f))
    }

    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        self.with_map(|map, list| {
            for (index, map) in map.iter().enumerate() {
                if !list.with_node(*map, |n| f(index, n)) {
                    break;
                }
            }
        });
    }

    fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        self.with_map_mut(|map, list| {
            for (index, map) in map.iter().enumerate() {
                if !list.with_node_mut(*map, |n| f(index, n)) {
                    break;
                }
            }
        });
    }

    fn len(&self) -> usize {
        self.list.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        let start = vec.len();
        self.list.drain_into(vec);
        vec[start..].sort_by(&self.sort);
        self.map.get_mut().clear();
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        let _ = SortingListParent::with(|| self.list.init_all(ctx));
        self.invalidate_sort();
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        let _ = SortingListParent::with(|| self.list.deinit_all(ctx));
        self.invalidate_sort();
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.list.event_all(ctx, update);
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut changed = false;
        let (_, resort) = SortingListParent::with(|| self.list.update_all(ctx, updates, &mut (observer, &mut changed as _)));
        if changed || resort {
            self.invalidate_sort();
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

static Z_INDEX_ID: StaticStateId<ZIndex> = StaticStateId::new_unique();

#[derive(Default, Clone, Debug)]
struct ZIndexContext {
    // used in `z_index` to validate that it will have an effect.
    panel_id: Option<WidgetId>,
    // set by `z_index` to signal a z-resort is needed.
    resort: bool,
}
impl ZIndexContext {
    fn with(panel_id: WidgetId, action: impl FnOnce()) -> bool {
        let ctx = ZIndexContext {
            panel_id: Some(panel_id),
            resort: false,
        };
        Z_INDEX.with_context(&mut Some(ctx), || {
            action();
            Z_INDEX.get().resort
        })
    }
}
context_local! {
    static Z_INDEX: ZIndexContext = ZIndexContext::default();
}

/// Position of a widget inside an [`UiNodeList`] render operation.
///
/// When two widgets have the same index their logical position defines the render order.
///
/// # Examples
///
/// Create a Z-index that causes the widget to render in front of all siblings that don't set Z-index.
///
/// ```
/// # use zero_ui_core::widget_instance::ZIndex;
/// #
/// let highlight_z = ZIndex::DEFAULT + 1;
/// ```
///
/// See [`z_index`] for more details.
///
/// [`z_index`]: fn@z_index
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZIndex(pub u32);
impl ZIndex {
    /// Widget is rendered first causing all overlapping siblings to render on top of it.
    ///
    /// The value is `0`.
    pub const BACK: ZIndex = ZIndex(0);

    /// Z-index of widgets that don't set the index.
    ///
    /// The value is `u32::MAX / 2`.
    pub const DEFAULT: ZIndex = ZIndex(u32::MAX / 2);

    /// Widget is rendered after all siblings causing it to render on top.
    pub const FRONT: ZIndex = ZIndex(u32::MAX);

    /// Computes `other` above `self`, caps at [`FRONT`].
    ///
    /// This is the default ZIndex addition, equivalent to `self + other`.
    ///
    /// [`FRONT`]: Self::FRONT
    pub fn saturating_add(self, other: impl Into<Self>) -> Self {
        ZIndex(self.0.saturating_add(other.into().0))
    }

    /// Computes `other` below `self`, stops at [`BACK`].
    ///
    /// This is the default ZIndex subtraction, equivalent to `self - other`.
    ///
    /// [`BACK`]: Self::BACK
    pub fn saturating_sub(self, other: impl Into<Self>) -> Self {
        ZIndex(self.0.saturating_sub(other.into().0))
    }

    /// Gets the index set on a widget.
    ///
    /// Returns `DEFAULT` if the node is not an widget.
    pub fn get(widget: &impl UiNode) -> ZIndex {
        widget
            .with_context(|ctx| ctx.widget_state.copy(&Z_INDEX_ID).unwrap_or_default())
            .unwrap_or_default()
    }
}
impl Default for ZIndex {
    fn default() -> Self {
        ZIndex::DEFAULT
    }
}
impl<Z: Into<ZIndex>> ops::Add<Z> for ZIndex {
    type Output = Self;

    fn add(self, rhs: Z) -> Self::Output {
        self.saturating_add(rhs)
    }
}
impl<Z: Into<ZIndex>> ops::Sub<Z> for ZIndex {
    type Output = Self;

    fn sub(self, rhs: Z) -> Self::Output {
        self.saturating_sub(rhs)
    }
}
impl fmt::Debug for ZIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let z = *self;
        if f.alternate() {
            write!(f, "ZIndex::")?;
        }

        if z == Self::DEFAULT {
            write!(f, "DEFAULT")
        } else if z == Self::BACK {
            write!(f, "BACK")
        } else if z == Self::FRONT {
            write!(f, "FRONT")
        } else if z > Self::DEFAULT {
            if z > Self::FRONT - 10000 {
                write!(f, "FRONT-{}", Self::FRONT.0 - z.0)
            } else {
                write!(f, "DEFAULT+{}", z.0 - Self::DEFAULT.0)
            }
        } else if z < Self::BACK + 10000 {
            write!(f, "BACK+{}", z.0 - Self::BACK.0)
        } else {
            write!(f, "DEFAULT-{}", Self::DEFAULT.0 - z.0)
        }
    }
}
impl_from_and_into_var! {
    fn from(index: u32) -> ZIndex {
        ZIndex(index)
    }
}

/// Defines the render order of a widget in a layout panel.
///
/// When set the widget will still update and layout according to their *logical* position in the list but
/// they will render according to the order defined by the [`ZIndex`] value.
///
/// Layout panels that support this property should mention it in their documentation, implementers
/// see [`PanelList`] for more details.
#[property(CONTEXT, default(ZIndex::DEFAULT))]
pub fn z_index(child: impl UiNode, index: impl IntoVar<ZIndex>) -> impl UiNode {
    #[ui_node(struct ZIndexNode {
        child: impl UiNode,
        #[var] index: impl Var<ZIndex>,
        valid: bool,
    })]
    impl UiNode for ZIndexNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let mut z_ctx = Z_INDEX.write();
            if z_ctx.panel_id != ctx.path.ancestors().next() || z_ctx.panel_id.is_none() {
                tracing::error!(
                    "property `z_index` set for `{}` but it is not the direct child of a Z-sorting panel",
                    ctx.path.widget_id()
                );
                self.valid = false;
            } else {
                self.valid = true;
                self.auto_subs(ctx);

                let index = self.index.get();
                if index != ZIndex::DEFAULT {
                    z_ctx.resort = true;
                    ctx.widget_state.set(&Z_INDEX_ID, self.index.get());
                }
            }
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.valid {
                if let Some(i) = self.index.get_new(ctx) {
                    let mut z_ctx = Z_INDEX.write();
                    debug_assert_eq!(z_ctx.panel_id, ctx.path.ancestors().next());
                    z_ctx.resort = true;
                    ctx.widget_state.set(&Z_INDEX_ID, i);
                }
            }

            self.child.update(ctx, updates);
        }
    }
    ZIndexNode {
        child,
        index: index.into_var(),
        valid: false,
    }
}

/// Represents an [`UiNodeList::update_all`] observer that can be used to monitor widget insertion, removal and re-order.
///
/// All indexes are in the context of the previous changes, if you are maintaining a *mirror* vector simply using the
/// [`Vec::insert`] and [`Vec::remove`] commands in the same order as they are received should keep the vector in sync.
///
/// This trait is implemented for `()`, to **not** observe simply pass on a `&mut ()`.
///
/// This trait is implemented for [`bool`], if any change happens the flag is set to `true`.
pub trait UiNodeListObserver {
    /// Large changes made to the list.
    fn reseted(&mut self);
    /// Widget inserted at the `index`.
    fn inserted(&mut self, index: usize);
    /// Widget removed from the `index`.
    fn removed(&mut self, index: usize);
    /// Widget removed and re-inserted.
    fn moved(&mut self, removed_index: usize, inserted_index: usize);
}
/// Does nothing.
impl UiNodeListObserver for () {
    fn reseted(&mut self) {}

    fn inserted(&mut self, _: usize) {}

    fn removed(&mut self, _: usize) {}

    fn moved(&mut self, _: usize, _: usize) {}
}
/// Sets to `true` for any change.
impl UiNodeListObserver for bool {
    fn reseted(&mut self) {
        *self = true;
    }

    fn inserted(&mut self, _: usize) {
        *self = true;
    }

    fn removed(&mut self, _: usize) {
        *self = true;
    }

    fn moved(&mut self, _: usize, _: usize) {
        *self = true;
    }
}

/// Represents an [`UiNodeListObserver`] that applies an offset to all indexes.
///
/// This type is useful for implementing [`UiNodeList`] that are composed of other lists.
pub struct OffsetUiListObserver<'o>(pub usize, pub &'o mut dyn UiNodeListObserver);
impl<'o> UiNodeListObserver for OffsetUiListObserver<'o> {
    fn reseted(&mut self) {
        self.1.reseted()
    }

    fn inserted(&mut self, index: usize) {
        self.1.inserted(index + self.0)
    }

    fn removed(&mut self, index: usize) {
        self.1.removed(index + self.0)
    }

    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
        self.1.moved(removed_index + self.0, inserted_index + self.0)
    }
}

impl UiNodeListObserver for (&mut dyn UiNodeListObserver, &mut dyn UiNodeListObserver) {
    fn reseted(&mut self) {
        self.0.reseted();
        self.1.reseted();
    }

    fn inserted(&mut self, index: usize) {
        self.0.inserted(index);
        self.1.inserted(index);
    }

    fn removed(&mut self, index: usize) {
        self.0.removed(index);
        self.1.removed(index);
    }

    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
        self.0.moved(removed_index, inserted_index);
        self.1.moved(removed_index, inserted_index);
    }
}

/// Represents an [`UiNodeList`] that can be modified using a connected sender.
pub struct EditableUiNodeList {
    vec: Vec<BoxedUiNode>,
    ctrl: EditableUiNodeListRef,
}
impl Default for EditableUiNodeList {
    fn default() -> Self {
        Self {
            vec: vec![],
            ctrl: EditableUiNodeListRef::new(),
        }
    }
}
impl Drop for EditableUiNodeList {
    fn drop(&mut self) {
        self.ctrl.0.lock().alive = false;
    }
}
impl EditableUiNodeList {
    /// New default empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// New from an already allocated vec.
    pub fn from_vec(vec: impl Into<Vec<BoxedUiNode>>) -> Self {
        let mut s = Self::new();
        s.vec = vec.into();
        s
    }

    /// Create a sender that can edit this list.
    pub fn reference(&self) -> EditableUiNodeListRef {
        self.ctrl.clone()
    }

    fn fullfill_requests(&mut self, ctx: &mut WidgetContext, observer: &mut dyn UiNodeListObserver) {
        if let Some(r) = self.ctrl.take_requests() {
            if r.clear {
                // if reset
                self.clear();
                observer.reseted();

                for (i, mut wgt) in r.insert {
                    wgt.init(ctx);
                    ctx.updates.info();
                    if i < self.len() {
                        self.insert(i, wgt);
                    } else {
                        self.push(wgt);
                    }
                }
                for mut wgt in r.push {
                    wgt.init(ctx);
                    ctx.updates.info();
                    self.push(wgt);
                }
                for (r, i) in r.move_index {
                    if r < self.len() {
                        let wgt = self.vec.remove(r);

                        if i < self.len() {
                            self.vec.insert(i, wgt);
                        } else {
                            self.vec.push(wgt);
                        }

                        ctx.updates.info();
                    }
                }
                for (id, to) in r.move_id {
                    if let Some(r) = self.vec.iter().position(|w| w.with_context(|ctx| ctx.id == id).unwrap_or(false)) {
                        let i = to(r, self.len());

                        if r != i {
                            let wgt = self.vec.remove(r);

                            if i < self.len() {
                                self.vec.insert(i, wgt);
                            } else {
                                self.vec.push(wgt);
                            }

                            ctx.updates.info();
                        }
                    }
                }
            } else {
                for id in r.remove {
                    if let Some(i) = self.vec.iter().position(|w| w.with_context(|ctx| ctx.id == id).unwrap_or(false)) {
                        let mut wgt = self.vec.remove(i);
                        wgt.deinit(ctx);
                        ctx.updates.info();

                        observer.removed(i);
                    }
                }

                for (i, mut wgt) in r.insert {
                    wgt.init(ctx);
                    ctx.updates.info();

                    if i < self.len() {
                        self.insert(i, wgt);
                        observer.inserted(i);
                    } else {
                        observer.inserted(self.len());
                        self.push(wgt);
                    }
                }

                for mut wgt in r.push {
                    wgt.init(ctx);
                    ctx.updates.info();

                    observer.inserted(self.len());
                    self.push(wgt);
                }

                for (r, i) in r.move_index {
                    if r < self.len() {
                        let wgt = self.vec.remove(r);

                        if i < self.len() {
                            self.vec.insert(i, wgt);

                            observer.moved(r, i);
                        } else {
                            let i = self.vec.len();

                            self.vec.push(wgt);

                            observer.moved(r, i);
                        }

                        ctx.updates.info();
                    }
                }

                for (id, to) in r.move_id {
                    if let Some(r) = self.vec.iter().position(|w| w.with_context(|ctx| ctx.id == id).unwrap_or(false)) {
                        let i = to(r, self.len());

                        if r != i {
                            let wgt = self.vec.remove(r);

                            if i < self.len() {
                                self.vec.insert(i, wgt);
                                observer.moved(r, i);
                            } else {
                                let i = self.vec.len();
                                self.vec.push(wgt);
                                observer.moved(r, i);
                            }

                            ctx.updates.info();
                        }
                    }
                }
            }
        }
    }
}
impl ops::Deref for EditableUiNodeList {
    type Target = Vec<BoxedUiNode>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl ops::DerefMut for EditableUiNodeList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
impl UiNodeList for EditableUiNodeList {
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        self.vec.with_node(index, f)
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.vec.with_node_mut(index, f)
    }

    fn for_each<F>(&self, f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        self.vec.for_each(f)
    }

    fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        self.vec.for_each_mut(f)
    }

    fn len(&self) -> usize {
        self.vec.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        vec.append(&mut self.vec)
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.ctrl.0.lock().target = Some(ctx.path.widget_id());
        self.vec.init_all(ctx);
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.ctrl.0.lock().target = None;
        self.vec.deinit_all(ctx);
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.fullfill_requests(ctx, observer);
        self.vec.update_all(ctx, updates, observer);
    }
}

/// See [`EditableUiNodeListRef::move_to`] for more details
type NodeMoveToFn = fn(usize, usize) -> usize;

/// Represents a sender to an [`EditableUiNodeList`].
#[derive(Clone)]
pub struct EditableUiNodeListRef(Arc<Mutex<EditRequests>>);
struct EditRequests {
    target: Option<WidgetId>,
    insert: Vec<(usize, BoxedUiNode)>,
    push: Vec<BoxedUiNode>,
    remove: Vec<WidgetId>,
    move_index: Vec<(usize, usize)>,
    move_id: Vec<(WidgetId, NodeMoveToFn)>,
    clear: bool,

    alive: bool,
}
impl EditableUiNodeListRef {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(EditRequests {
            target: None,
            insert: vec![],
            push: vec![],
            remove: vec![],
            move_index: vec![],
            move_id: vec![],
            clear: false,
            alive: true,
        })))
    }

    /// Returns `true` if the [`EditableUiNodeList`] still exists.
    pub fn alive(&self) -> bool {
        self.0.lock().alive
    }

    /// Request an update for the insertion of the `widget`.
    ///
    /// The `index` is resolved after all [`remove`] requests, if it is out-of-bounds the widget is pushed.
    ///
    /// The `widget` will inserted, inited and the info tree updated.
    ///
    /// [`remove`]: Self::remove
    pub fn insert(&self, updates: &mut impl WithUpdates, index: usize, widget: impl UiNode) {
        updates.with_updates(|u| {
            let mut s = self.0.lock();
            s.insert.push((index, widget.boxed()));
            u.update(s.target);
        })
    }

    /// Request an update for the insertion of the `widget` at the end of the list.
    ///
    /// The widget will be pushed after all [`insert`] requests.
    ///
    /// The `widget` will be inserted, inited and the info tree updated.
    ///
    /// [`insert`]: Self::insert
    pub fn push(&self, updates: &mut impl WithUpdates, widget: impl UiNode) {
        updates.with_updates(|u| {
            let mut s = self.0.lock();
            s.push.push(widget.boxed());
            u.update(s.target);
        })
    }

    /// Request an update for the removal of the widget identified by `id`.
    ///
    /// The widget will be deinitialized, dropped and the info tree will update, nothing happens
    /// if the widget is not found.
    pub fn remove(&self, updates: &mut impl WithUpdates, id: impl Into<WidgetId>) {
        updates.with_updates(|u| {
            let mut s = self.0.lock();
            s.remove.push(id.into());
            u.update(s.target);
        })
    }

    /// Request a widget remove and re-insert.
    ///
    /// If the `remove_index` is out of bounds nothing happens, if the `insert_index` is out-of-bounds
    /// the widget is pushed to the end of the vector, if `remove_index` and `insert_index` are equal nothing happens.
    ///
    /// Move requests happen after all other requests.
    pub fn move_index(&self, updates: &mut impl WithUpdates, remove_index: usize, insert_index: usize) {
        if remove_index != insert_index {
            updates.with_updates(|u| {
                let mut s = self.0.lock();
                s.move_index.push((remove_index, insert_index));
                u.update(s.target);
            })
        }
    }

    /// Request a widget move, the widget is searched by `id`, if found `get_move_to` id called with the index of the widget and length
    /// of the vector, it must return the index the widget is inserted after it is removed.
    ///
    /// If the widget is not found nothing happens, if the returned index is the same nothing happens, if the returned index
    /// is out-of-bounds the widget if pushed to the end of the vector.
    ///
    /// Move requests happen after all other requests.
    ///
    /// # Examples
    ///
    /// If the widget vectors is layout as a vertical stack to move the widget *up* by one stopping at the top:
    ///
    /// ```
    /// # fn demo(ctx: &mut zero_ui_core::context::WidgetContext, items: zero_ui_core::widget_instance::EditableUiNodeListRef) {
    /// items.move_id(ctx.updates, "my-widget", |i, _len| i.saturating_sub(1));
    /// # }
    /// ```
    ///
    /// And to move *down* stopping at the bottom:
    ///
    /// ```
    /// # fn demo(ctx: &mut zero_ui_core::context::WidgetContext, items: zero_ui_core::widget_instance::EditableUiNodeListRef) {
    /// items.move_id(ctx.updates, "my-widget", |i, _len| i.saturating_add(1));
    /// # }
    /// ```
    ///
    /// Note that if the returned index overflows the length the widget is
    /// pushed as the last item.
    ///
    /// The length can be used for implementing wrapping move *down*:
    ///
    /// ```
    /// # fn demo(ctx: &mut zero_ui_core::context::WidgetContext, items: zero_ui_core::widget_instance::EditableUiNodeListRef) {
    /// items.move_id(ctx.updates, "my-widget", |i, len| {
    ///     let next = i.saturating_add(1);
    ///     if next < len { next } else { 0 }
    /// });
    /// # }
    /// ```
    pub fn move_id(&self, updates: &mut impl WithUpdates, id: impl Into<WidgetId>, get_move_to: NodeMoveToFn) {
        updates.with_updates(|u| {
            let mut s = self.0.lock();
            s.move_id.push((id.into(), get_move_to));
            u.update(s.target);
        })
    }

    /// Request a removal of all current widgets.
    ///
    /// All other requests will happen after the clear.
    pub fn clear(&self, updates: &mut impl WithUpdates) {
        updates.with_updates(|u| {
            let mut s = self.0.lock();
            s.clear = true;
            u.update(s.target);
        })
    }

    fn take_requests(&self) -> Option<EditRequests> {
        let mut s = self.0.lock();

        if s.clear
            || !s.insert.is_empty()
            || !s.push.is_empty()
            || !s.remove.is_empty()
            || !s.move_index.is_empty()
            || !s.move_id.is_empty()
        {
            let empty = EditRequests {
                target: s.target,
                alive: s.alive,

                insert: vec![],
                push: vec![],
                remove: vec![],
                move_index: vec![],
                move_id: vec![],
                clear: false,
            };
            Some(mem::replace(&mut *s, empty))
        } else {
            None
        }
    }
}

fn many_list_index(lists: &Vec<BoxedUiNodeList>, index: usize) -> (usize, usize) {
    let mut offset = 0;

    for (li, list) in lists.iter().enumerate() {
        let i = index - offset;
        let len = list.len();

        if i < len {
            return (li, i);
        }

        offset += len;
    }

    panic!(
        "'index out of bounds: the len is {} but the index is {}",
        UiNodeList::len(lists),
        index
    );
}

impl UiNodeList for Vec<BoxedUiNodeList> {
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        let (l, i) = many_list_index(self, index);
        self[l].with_node(i, f)
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        let (l, i) = many_list_index(self, index);
        self[l].with_node_mut(i, f)
    }

    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        let mut offset = 0;
        for list in self {
            list.for_each(|i, n| f(i + offset, n));
            offset += list.len();
        }
    }

    fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        let mut offset = 0;
        for list in self {
            list.for_each_mut(|i, n| f(i + offset, n));
            offset += list.len();
        }
    }

    fn len(&self) -> usize {
        self.iter().map(|l| l.len()).sum()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        for mut list in self.drain(..) {
            list.drain_into(vec);
        }
    }

    fn is_empty(&self) -> bool {
        self.iter().all(|l| l.is_empty())
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        for list in self {
            list.init_all(ctx);
        }
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        for list in self {
            list.deinit_all(ctx);
        }
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut offset = 0;
        for list in self {
            list.update_all(ctx, updates, &mut OffsetUiListObserver(offset, observer));
            offset += list.len();
        }
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        for list in self {
            list.event_all(ctx, update);
        }
    }

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        for list in self {
            list.render_all(ctx, frame);
        }
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        for list in self {
            list.render_update_all(ctx, update);
        }
    }
}

/// Represents the final [`UiNodeList`] in a panel layout node.
///
/// Panel widgets should wrap their children list on this type to support [`z_index`] sorting list and easily track
/// item data. By default the item data is a [`PxVector`] that represents the offset of each item inside the panel,
/// but it can be any type that implements [`PanelListData`].
///
/// [`z_index`]: fn@z_index
pub struct PanelList<D>
where
    D: PanelListData,
{
    list: BoxedUiNodeList,
    data: Vec<D>,

    offset_key: FrameValueKey<PxTransform>,

    z_map: RefCell<Vec<u64>>,
    z_naturally_sorted: Cell<bool>,
}

impl<D> PanelList<D>
where
    D: PanelListData,
{
    /// New from `list` and default data.
    pub fn new(list: impl UiNodeList) -> Self {
        Self {
            data: {
                let mut d = vec![];
                d.resize_with(list.len(), Default::default);
                d
            },
            list: list.boxed(),
            offset_key: FrameValueKey::new_unique(),
            z_map: RefCell::new(vec![]),
            z_naturally_sorted: Cell::new(false),
        }
    }

    /// Into list and associated data.
    pub fn into_parts(self) -> (BoxedUiNodeList, Vec<D>, FrameValueKey<PxTransform>) {
        (self.list, self.data, self.offset_key)
    }

    /// New from list and associated data.
    ///
    /// # Panics
    ///
    /// Panics if the `list` and `data` don't have the same length.
    pub fn from_parts(list: BoxedUiNodeList, data: Vec<D>, offset_key: FrameValueKey<PxTransform>) -> Self {
        assert_eq!(list.len(), data.len());
        Self {
            list,
            data,
            offset_key,
            z_map: RefCell::new(vec![]),
            z_naturally_sorted: Cell::new(false),
        }
    }

    /// Visit the specific node and data, panic if `index` is out of bounds.
    pub fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode, &D) -> R,
    {
        let data = &self.data[index];
        self.list.with_node(index, move |n| f(n, data))
    }

    /// Visit the specific node, panic if `index` is out of bounds.
    pub fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode, &mut D) -> R,
    {
        let data = &mut self.data[index];
        self.list.with_node_mut(index, move |n| f(n, data))
    }

    /// Calls `f` for each node in the list with the index, return `true` to continue iterating, return `false` to stop.
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode, &D) -> bool,
    {
        self.list.for_each(move |i, n| f(i, n, &self.data[i]))
    }

    /// Calls `f` for each node in the list with the index, return `true` to continue iterating, return `false` to stop.
    pub fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode, &mut D) -> bool,
    {
        let data = &mut self.data;
        self.list.for_each_mut(move |i, n| f(i, n, &mut data[i]))
    }

    /// Iterate over the list in the Z order.
    pub fn for_each_z_sorted(&self, mut f: impl FnMut(usize, &BoxedUiNode, &D) -> bool) {
        if self.z_naturally_sorted.get() {
            self.for_each(f)
        } else {
            if self.z_map.borrow().len() != self.list.len() {
                self.z_sort();
            }

            if self.z_naturally_sorted.get() {
                self.for_each(f);
            } else {
                for &index in self.z_map.borrow().iter() {
                    let index = index as usize;
                    if !self.list.with_node(index, |node| f(index, node, &self.data[index])) {
                        break;
                    }
                }
            }
        }
    }

    /// Iterate over the list in the Z order.
    pub fn for_each_z_sorted_mut(&mut self, mut f: impl FnMut(usize, &mut BoxedUiNode, &mut D) -> bool) {
        if self.z_naturally_sorted.get() {
            self.for_each_mut(f)
        } else {
            if self.z_map.borrow().len() != self.list.len() {
                self.z_sort();
            }

            for &index in self.z_map.borrow().iter() {
                let index = index as usize;
                let data = &mut self.data[index];
                if !self.list.with_node_mut(index, |node| f(index, node, data)) {
                    break;
                }
            }
        }
    }

    fn z_sort(&self) {
        // We pack *z* and *i* as u32s in one u64 then create the sorted lookup table if
        // observed `[I].Z < [I-1].Z`, also records if any `Z != DEFAULT`:
        //
        // Advantages:
        //
        // - Makes `sort_unstable` stable.
        // - Only one alloc needed, just mask out Z after sorting.
        //
        // Disadvantages:
        //
        // - Only supports u32::MAX widgets.
        // - Uses 64-bit indexes in 32-bit builds.

        let len = self.list.len();
        assert!(len <= u32::MAX as usize);

        let mut prev_z = ZIndex::BACK;
        let mut need_map = false;
        let mut z_and_i = Vec::with_capacity(len);
        let mut has_non_default_zs = false;

        self.list.for_each(|i, node| {
            let z = ZIndex::get(node);
            z_and_i.push(((z.0 as u64) << 32) | i as u64);

            need_map |= z < prev_z;
            has_non_default_zs |= z != ZIndex::DEFAULT;
            prev_z = z;

            true
        });

        self.z_naturally_sorted.set(!need_map);

        if need_map {
            z_and_i.sort_unstable();

            for z in &mut z_and_i {
                *z &= u32::MAX as u64;
            }

            *self.z_map.borrow_mut() = z_and_i;
        } else {
            self.z_map.borrow_mut().clear();
        }
    }

    /// Gets the `index` sorted in the `list`.
    pub fn z_map(&self, index: usize) -> usize {
        if self.z_naturally_sorted.get() {
            return index;
        }

        if self.z_map.borrow().len() != self.list.len() {
            self.z_sort();
        }

        self.z_map.borrow()[index] as usize
    }

    /// Reference the associated data.
    pub fn data(&self) -> &[D] {
        &self.data
    }

    /// Mutable reference the associated data.
    pub fn data_mut(&mut self) -> &mut [D] {
        &mut self.data
    }

    /// Key used to define reference frames for each item.
    ///
    /// The default implementation of `render_all` uses this key and the item index.
    pub fn offset_key(&self) -> FrameValueKey<PxTransform> {
        self.offset_key
    }
}
impl<D> UiNodeList for PanelList<D>
where
    D: PanelListData,
{
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        self.list.with_node(index, f)
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.list.with_node_mut(index, f)
    }

    fn for_each<F>(&self, f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        self.list.for_each(f)
    }

    fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        self.list.for_each_mut(f)
    }

    fn len(&self) -> usize {
        self.list.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.list.drain_into(vec);
        self.data.clear();
        self.z_map.get_mut().clear();
        self.z_naturally_sorted.set(true);
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.z_map.get_mut().clear();
        let resort = ZIndexContext::with(ctx.path.widget_id(), || self.list.init_all(ctx));
        self.z_naturally_sorted.set(!resort);
        self.data.resize_with(self.list.len(), Default::default);
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.list.deinit_all(ctx);
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.list.event_all(ctx, update);
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut observer = PanelObserver {
            changed: false,
            data: &mut self.data,
            observer,
        };
        let resort = ZIndexContext::with(ctx.path.widget_id(), || self.list.update_all(ctx, updates, &mut observer));
        if resort || (observer.changed && self.z_naturally_sorted.get()) {
            self.z_map.get_mut().clear();
            self.z_naturally_sorted.set(false);
            ctx.updates.render();
        }
        self.data.resize_with(self.list.len(), Default::default);
    }

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.for_each_z_sorted(|i, child, data| {
            let offset = data.child_offset();
            if data.define_reference_frame() {
                frame.push_reference_frame(
                    (self.offset_key, i as u32).into(),
                    self.offset_key.bind_child(i as u32, offset.into(), false),
                    true,
                    true,
                    |frame| {
                        child.render(ctx, frame);
                    },
                );
            } else {
                todo!("!!: push_inner_transform?")
            }

            true
        });
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.for_each(|i, child, data| {
            let offset = data.child_offset();
            if data.define_reference_frame() {
                update.update_transform(self.offset_key.update_child(i as u32, offset.into(), false), true);
            }
            update.with_transform_value(&offset.into(), |update| {
                child.render_update(ctx, update);
            });

            true
        });
    }
}

/// Represents associated data in a [`PanelList`].
pub trait PanelListData: Default + Send + Any {
    /// Gets the child offset to be used in the default `render_all` and `render_update_all` implementations.
    fn child_offset(&self) -> PxVector;

    /// If a new reference frame should be created for the item during render.
    fn define_reference_frame(&self) -> bool;
}
impl PanelListData for PxVector {
    fn child_offset(&self) -> PxVector {
        *self
    }

    fn define_reference_frame(&self) -> bool {
        true
    }
}
impl PanelListData for () {
    fn child_offset(&self) -> PxVector {
        PxVector::zero()
    }

    fn define_reference_frame(&self) -> bool {
        false
    }
}

struct PanelObserver<'d, D>
where
    D: PanelListData,
{
    changed: bool,
    data: &'d mut Vec<D>,
    observer: &'d mut dyn UiNodeListObserver,
}
impl<'d, D> UiNodeListObserver for PanelObserver<'d, D>
where
    D: PanelListData,
{
    fn reseted(&mut self) {
        self.changed = true;
        self.data.clear();
        self.observer.reseted();
    }

    fn inserted(&mut self, index: usize) {
        self.changed = true;
        self.data.insert(index, Default::default());
        self.observer.inserted(index);
    }

    fn removed(&mut self, index: usize) {
        self.changed = true;
        self.data.remove(index);
        self.observer.removed(index);
    }

    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
        self.changed = true;
        let item = self.data.remove(removed_index);
        self.data.insert(inserted_index, item);
        self.observer.moved(removed_index, inserted_index);
    }
}

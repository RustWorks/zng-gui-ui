use parking_lot::Mutex;
use std::sync::{Arc, Weak};

use crate::{
    event::{Event, EventArgs, EventHandles},
    var::*,
    widget_instance::*,
};

type SlotId = usize;

struct SlotData<U> {
    item: Mutex<U>,
    slots: Mutex<SlotsData<U>>,
}
struct SlotsData<U> {
    // id of the next slot created.
    next_slot: SlotId,

    // slot and context where the node is inited.
    owner: Option<(SlotId, WidgetId)>,
    // slot and context that has requested ownership.
    move_request: Option<(SlotId, WidgetId)>,

    // node instance that must replace the current in the active slot.
    replacement: Option<U>,
}
impl<U> SlotsData<U> {
    fn next_slot(&mut self) -> SlotId {
        let r = self.next_slot;
        self.next_slot = self.next_slot.wrapping_add(1);
        r
    }
}
impl<U> Default for SlotsData<U> {
    fn default() -> Self {
        Self {
            next_slot: Default::default(),
            owner: Default::default(),
            move_request: Default::default(),
            replacement: Default::default(),
        }
    }
}

/// A reference counted [`UiNode`].
///
/// Nodes can only appear in one place of the UI tree at a time, this `struct` allows the
/// creation of ***slots*** that are [`UiNode`] implementers that can *exclusive take* the
/// referenced node as its child.
///
/// When a slot takes the node it is deinited in the previous UI tree place and reinited in the slot place.
///
/// Slots hold a strong reference to the node when they have it as their child and a weak reference when they don't.
pub struct ArcNode<U: UiNode>(Arc<SlotData<U>>);
impl<U: UiNode> Clone for ArcNode<U> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<U: UiNode> ArcNode<U> {
    /// New node.
    pub fn new(node: U) -> Self {
        ArcNode(Arc::new(SlotData {
            item: Mutex::new(node),
            slots: Mutex::default(),
        }))
    }

    /// New rc node that contains a weak reference to itself.
    ///
    /// **Note** the weak reference cannot be [upgraded](WeakNode::upgrade) during the call to `node`.
    pub fn new_cyclic(node: impl FnOnce(WeakNode<U>) -> U) -> Self {
        Self(Arc::new_cyclic(|wk| {
            let node = node(WeakNode(wk.clone()));
            SlotData {
                item: Mutex::new(node),
                slots: Mutex::default(),
            }
        }))
    }

    /// Creates a [`WeakNode<U>`] reference to this node.
    pub fn downgrade(&self) -> WeakNode<U> {
        WeakNode(Arc::downgrade(&self.0))
    }

    /// Replace the current node with the `new_node` in the current slot.
    ///
    /// The previous node is deinited and the `new_node` is inited.
    pub fn set(&self, new_node: U) {
        let mut slots = self.0.slots.lock();
        let slots = &mut *slots;
        if let Some((_, id)) = &slots.owner {
            // current node inited on a slot, signal it to replace.
            slots.replacement = Some(new_node);
            let _ = UPDATES.update(*id);
        } else {
            // node already not inited, just replace.
            *self.0.item.lock() = new_node;
        }
    }

    /// Create a *slot* node that takes ownership of this node when `var` updates to `true`.
    ///
    /// The slot node also takes ownership on init if the `var` is already `true`.
    ///
    /// The return type implements [`UiNode`].
    pub fn take_when(&self, var: impl IntoVar<bool>) -> TakeSlot<U, impl TakeOn> {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeWhenVar { var: var.into_var() },
            delegate_init: |n, ctx| n.init(ctx),
            delegate_deinit: |n, ctx| n.deinit(ctx),
            var_handles: VarHandles::default(),
            event_handles: EventHandles::default(),
        }
    }

    /// Create a *slot* node that takes ownership of this node when `event` updates and `filter` returns `true`.
    ///
    /// The slot node also takes ownership on init if `take_on_init` is `true`.
    ///
    /// The return type implements [`UiNode`].
    pub fn take_on<A: EventArgs>(
        &self,
        event: Event<A>,
        filter: impl FnMut(&A) -> bool + Send + 'static,
        take_on_init: bool,
    ) -> TakeSlot<U, impl TakeOn> {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeOnEvent {
                event,
                filter,
                take_on_init,
            },
            delegate_init: |n, ctx| n.init(ctx),
            delegate_deinit: |n, ctx| n.deinit(ctx),
            var_handles: VarHandles::default(),
            event_handles: EventHandles::default(),
        }
    }

    /// Create a *slot* node that takes ownership of this node as soon as the node is inited.
    ///
    /// This is equivalent to `self.take_when(true)`
    pub fn take_on_init(&self) -> TakeSlot<U, impl TakeOn> {
        self.take_when(true)
    }

    /// Calls `f` in the context of the node, it it can be locked and is a full widget.
    pub fn try_context<R>(&self, f: impl FnOnce(&mut WidgetNodeContext) -> R) -> Option<R> {
        self.0.item.try_lock()?.with_context(f)
    }

    /// Calls `f` in the context of the node, it it can be locked and is a full widget.
    pub fn try_context_mut<R>(&self, f: impl FnOnce(&mut WidgetNodeMutContext) -> R) -> Option<R> {
        self.0.item.try_lock()?.with_context_mut(f)
    }
}

/// `Weak` reference to a [`ArcNode<U>`].
pub struct WeakNode<U: UiNode>(Weak<SlotData<U>>);
impl<U: UiNode> Clone for WeakNode<U> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<U: UiNode> WeakNode<U> {
    /// Attempts to upgrade to a [`ArcNode<U>`].
    pub fn upgrade(&self) -> Option<ArcNode<U>> {
        self.0.upgrade().map(ArcNode)
    }
}

/// A reference counted [`UiNodeList`].
///
/// Nodes can only appear in one place of the UI tree at a time, this `struct` allows the
/// creation of ***slots*** that are [`UiNodeList`] implementers that can *exclusive take* the
/// referenced list as the children.
///
/// When a slot takes the list it is deinited in the previous UI tree place and reinited in the slot place.
///
/// Slots hold a strong reference to the list when they have it as their child and a weak reference when they don't.
pub struct ArcNodeList<L: UiNodeList>(Arc<SlotData<L>>);
impl<L: UiNodeList> Clone for ArcNodeList<L> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<L: UiNodeList> ArcNodeList<L> {
    /// New list.
    pub fn new(list: L) -> Self {
        ArcNodeList(Arc::new(SlotData {
            item: Mutex::new(list),
            slots: Mutex::default(),
        }))
    }

    /// New rc list that contains a weak reference to itself.
    ///
    /// **Note** the weak reference cannot be [upgraded](WeakNodeList::upgrade) during the call to `list`.
    pub fn new_cyclic(list: impl FnOnce(WeakNodeList<L>) -> L) -> Self {
        Self(Arc::new_cyclic(|wk| {
            let list = list(WeakNodeList(wk.clone()));
            SlotData {
                item: Mutex::new(list),
                slots: Mutex::default(),
            }
        }))
    }

    /// Creates a [`WeakNodeList<L>`] reference to this list.
    pub fn downgrade(&self) -> WeakNodeList<L> {
        WeakNodeList(Arc::downgrade(&self.0))
    }

    /// Replace the current list with the `new_list` in the current slot.
    ///
    /// The previous list is deinited and the `new_list` is inited.
    pub fn set(&self, new_list: L) {
        let mut slots = self.0.slots.lock();
        let slots = &mut *slots;
        if let Some((_, id)) = &slots.owner {
            // current node inited on a slot, signal it to replace.
            slots.replacement = Some(new_list);
            UPDATES.update(*id);
        } else {
            // node already not inited, just replace.
            *self.0.item.lock() = new_list;
        }
    }

    /// Create a *slot* node that takes ownership of this node when `var` updates to `true`.
    ///
    /// The slot node also takes ownership on init if the `var` is already `true`.
    ///
    /// The return type implements [`UiNodeList`].
    pub fn take_when(&self, var: impl IntoVar<bool>) -> TakeSlot<L, impl TakeOn> {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeWhenVar { var: var.into_var() },
            delegate_init: |n, ctx| n.init_all(ctx),
            delegate_deinit: |n, ctx| n.deinit_all(ctx),
            var_handles: VarHandles::default(),
            event_handles: EventHandles::default(),
        }
    }

    /// Create a *slot* node that takes ownership of this node when `event` updates and `filter` returns `true`.
    ///
    /// The slot node also takes ownership on init if `take_on_init` is `true`.
    ///
    /// The return type implements [`UiNodeList`].
    pub fn take_on<A: EventArgs>(
        &self,
        event: Event<A>,
        filter: impl FnMut(&A) -> bool + Send + 'static,
        take_on_init: bool,
    ) -> TakeSlot<L, impl TakeOn> {
        impls::TakeSlot {
            slot: self.0.slots.lock().next_slot(),
            rc: self.0.clone(),
            take: impls::TakeOnEvent {
                event,
                filter,
                take_on_init,
            },
            delegate_init: |n, ctx| n.init_all(ctx),
            delegate_deinit: |n, ctx| n.deinit_all(ctx),
            var_handles: VarHandles::default(),
            event_handles: EventHandles::default(),
        }
    }

    /// Create a *slot* node list that takes ownership of this list as soon as the node is inited.
    ///
    /// This is equivalent to `self.take_when(true)`
    pub fn take_on_init(&self) -> TakeSlot<L, impl TakeOn> {
        self.take_when(true)
    }

    /// Iterate over node contexts, if the list can be locked and the node is a full widget.
    pub fn for_each_ctx(&self, mut f: impl FnMut(usize, &mut WidgetNodeContext) -> bool) {
        if let Some(list) = self.0.item.try_lock() {
            list.for_each(|i, n| n.with_context(|ctx| f(i, ctx)).unwrap_or(true))
        }
    }

    /// Iterate over node contexts, if the list can be locked and the node is a full widget.
    pub fn for_each_ctx_mut(&self, mut f: impl FnMut(usize, &mut WidgetNodeMutContext) -> bool) {
        if let Some(mut list) = self.0.item.try_lock() {
            list.for_each_mut(|i, n| n.with_context_mut(|ctx| f(i, ctx)).unwrap_or(true))
        }
    }
}

/// `Weak` reference to a [`ArcNodeList<U>`].
pub struct WeakNodeList<L: UiNodeList>(Weak<SlotData<L>>);
impl<L: UiNodeList> Clone for WeakNodeList<L> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<L: UiNodeList> WeakNodeList<L> {
    /// Attempts to upgrade to a [`ArcNodeList<U>`].
    pub fn upgrade(&self) -> Option<ArcNodeList<L>> {
        self.0.upgrade().map(ArcNodeList)
    }
}

pub use impls::*;

mod impls {
    use std::sync::Arc;

    use crate::{
        context::*,
        event::{Event, EventArgs, EventHandles, EventUpdate},
        render::{FrameBuilder, FrameUpdate},
        units::PxSize,
        var::*,
        widget_info::{WidgetInfoBuilder, WidgetLayout},
        widget_instance::*,
    };

    use super::{SlotData, SlotId};

    #[doc(hidden)]
    pub trait TakeOn: Send + 'static {
        fn take_on_init(&mut self, ctx: &mut WidgetContext) -> bool {
            let _ = ctx;
            false
        }

        fn take_on_event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) -> bool {
            let _ = (ctx, update);
            false
        }

        fn take_on_update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) -> bool {
            let _ = (ctx, updates);
            false
        }
    }

    pub(super) struct TakeWhenVar<V: Var<bool>> {
        pub(super) var: V,
    }
    impl<V: Var<bool>> TakeOn for TakeWhenVar<V> {
        fn take_on_init(&mut self, ctx: &mut WidgetContext) -> bool {
            ctx.sub_var(&self.var);
            self.var.get()
        }

        fn take_on_update(&mut self, _: &mut WidgetContext, _: &mut WidgetUpdates) -> bool {
            self.var.get_new().unwrap_or(false)
        }
    }

    pub(super) struct TakeOnEvent<A: EventArgs, F: FnMut(&A) -> bool + Send + 'static> {
        pub(super) event: Event<A>,
        pub(super) filter: F,
        pub(super) take_on_init: bool,
    }
    impl<A: EventArgs, F: FnMut(&A) -> bool + Send + Send + 'static> TakeOn for TakeOnEvent<A, F> {
        fn take_on_init(&mut self, ctx: &mut WidgetContext) -> bool {
            ctx.sub_event(&self.event);
            self.take_on_init
        }

        fn take_on_event(&mut self, _: &mut WidgetContext, update: &mut EventUpdate) -> bool {
            if let Some(args) = self.event.on(update) {
                (self.filter)(args)
            } else {
                false
            }
        }
    }

    #[doc(hidden)]
    pub struct TakeSlot<U, T: TakeOn> {
        pub(super) slot: SlotId,
        pub(super) rc: Arc<SlotData<U>>,
        pub(super) take: T,

        pub(super) delegate_init: fn(&mut U, &mut WidgetContext),
        pub(super) delegate_deinit: fn(&mut U, &mut WidgetContext),
        pub(super) var_handles: VarHandles,
        pub(super) event_handles: EventHandles,
    }
    impl<U, T: TakeOn> TakeSlot<U, T> {
        fn on_init(&mut self, ctx: &mut WidgetContext) {
            if self.take.take_on_init(ctx) {
                self.take(ctx);
            }
        }

        fn on_deinit(&mut self, ctx: &mut WidgetContext) {
            let mut was_owner = false;
            {
                let mut slots = self.rc.slots.lock();
                let slots = &mut *slots;
                if let Some((slot, _)) = &slots.owner {
                    if *slot == self.slot {
                        slots.owner = None;
                        was_owner = true;
                    }
                }
            }

            if was_owner {
                ctx.with_handles(&mut self.var_handles, &mut self.event_handles, |ctx| {
                    (self.delegate_deinit)(&mut *self.rc.item.lock(), ctx)
                });
            }

            self.var_handles.clear();
            self.event_handles.clear();
        }

        fn on_event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if !self.is_owner() && self.take.take_on_event(ctx, update) {
                // request ownership.
                self.take(ctx);
            }
        }

        fn on_update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.is_owner() {
                let mut slots = self.rc.slots.lock();
                if let Some((_, id)) = slots.move_request {
                    // deinit to move to other slot.

                    let replacement = slots.replacement.take();
                    slots.owner = None;

                    drop(slots);

                    let mut node = self.rc.item.lock();
                    (self.delegate_deinit)(&mut node, ctx);

                    WIDGET.rebuild_info().layout().render();

                    if let Some(new) = replacement {
                        *node = new;
                    }

                    UPDATES.update(id);
                } else if let Some(mut new) = slots.replacement.take() {
                    // apply replacement.

                    drop(slots);

                    let mut node = self.rc.item.lock();
                    ctx.with_handles(&mut self.var_handles, &mut self.event_handles, |ctx| {
                        (self.delegate_deinit)(&mut node, ctx);
                    });
                    self.var_handles.clear();
                    self.event_handles.clear();

                    ctx.with_handles(&mut self.var_handles, &mut self.event_handles, |ctx| {
                        (self.delegate_init)(&mut new, ctx);
                    });
                    *node = new;

                    WIDGET.rebuild_info().layout().render();
                }
            } else if self.take.take_on_update(ctx, updates) {
                // request ownership.
                self.take(ctx);
            } else {
                let mut slots = self.rc.slots.lock();
                if let Some((slot, _)) = &slots.move_request {
                    if *slot == self.slot && slots.owner.is_none() {
                        slots.move_request = None;
                        // requested move in prev update, now can take ownership.
                        drop(slots);
                        self.take(ctx);
                    }
                }
            }
        }

        fn take(&mut self, ctx: &mut WidgetContext) {
            {
                let mut slots = self.rc.slots.lock();
                let slots = &mut *slots;
                if let Some((sl, id)) = &slots.owner {
                    if *sl != self.slot {
                        // currently inited in another slot, signal it to deinit.
                        slots.move_request = Some((self.slot, ctx.path.widget_id()));
                        UPDATES.update(*id);
                    }
                } else {
                    // no current owner, take ownership immediately.
                    slots.owner = Some((self.slot, ctx.path.widget_id()));
                }
            }

            if self.is_owner() {
                ctx.with_handles(&mut self.var_handles, &mut self.event_handles, |ctx| {
                    (self.delegate_init)(&mut *self.rc.item.lock(), ctx);
                });
                WIDGET.rebuild_info().layout().render();
            }
        }

        fn is_owner(&self) -> bool {
            self.rc
                .slots
                .lock()
                .owner
                .as_ref()
                .map(|(sl, _)| *sl == self.slot)
                .unwrap_or(false)
        }

        fn delegate_owned<R>(&self, del: impl FnOnce(&U) -> R) -> Option<R> {
            if self.is_owner() {
                Some(del(&*self.rc.item.lock()))
            } else {
                None
            }
        }
        fn delegate_owned_mut<R>(&mut self, del: impl FnOnce(&mut U) -> R) -> Option<R> {
            if self.is_owner() {
                Some(del(&mut *self.rc.item.lock()))
            } else {
                None
            }
        }

        fn delegate_owned_mut_with_handles<R>(
            &mut self,
            ctx: &mut WidgetContext,
            del: impl FnOnce(&mut WidgetContext, &mut U) -> R,
        ) -> Option<R> {
            if self.is_owner() {
                ctx.with_handles(&mut self.var_handles, &mut self.event_handles, |ctx| {
                    Some(del(ctx, &mut *self.rc.item.lock()))
                })
            } else {
                None
            }
        }
    }

    impl<U: UiNode, T: TakeOn> UiNode for TakeSlot<U, T> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.on_init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.on_deinit(ctx);
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            self.delegate_owned(|n| n.info(ctx, info));
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.on_event(ctx, update);
            self.delegate_owned_mut_with_handles(ctx, |ctx, n| n.event(ctx, update));
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.on_update(ctx, updates);
            self.delegate_owned_mut_with_handles(ctx, |ctx, n| n.update(ctx, updates));
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.delegate_owned(|n| n.measure(ctx, wm)).unwrap_or_default()
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            self.delegate_owned_mut(|n| n.layout(ctx, wl)).unwrap_or_default()
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.delegate_owned(|n| n.render(ctx, frame));
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.delegate_owned(|n| n.render_update(ctx, update));
        }

        fn is_widget(&self) -> bool {
            self.delegate_owned(UiNode::is_widget).unwrap_or(false)
        }

        fn with_context<R, F>(&self, f: F) -> Option<R>
        where
            F: FnOnce(&mut WidgetNodeContext) -> R,
        {
            self.delegate_owned(|n| n.with_context(f)).flatten()
        }

        fn with_context_mut<R, F>(&mut self, f: F) -> Option<R>
        where
            F: FnOnce(&mut WidgetNodeMutContext) -> R,
        {
            self.delegate_owned_mut(|n| n.with_context_mut(f)).flatten()
        }
    }

    impl<U: UiNodeList, T: TakeOn> UiNodeList for TakeSlot<U, T> {
        fn with_node<R, F>(&self, index: usize, f: F) -> R
        where
            F: FnOnce(&BoxedUiNode) -> R,
        {
            self.delegate_owned(move |l| l.with_node(index, f))
                .unwrap_or_else(|| panic!("index `{index}` is >= len `0`"))
        }

        fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
        where
            F: FnOnce(&mut BoxedUiNode) -> R,
        {
            self.delegate_owned_mut(move |l| l.with_node_mut(index, f))
                .unwrap_or_else(|| panic!("index `{index}` is >= len `0`"))
        }

        fn for_each<F>(&self, f: F)
        where
            F: FnMut(usize, &BoxedUiNode) -> bool,
        {
            self.delegate_owned(|l| l.for_each(f));
        }

        fn for_each_mut<F>(&mut self, f: F)
        where
            F: FnMut(usize, &mut BoxedUiNode) -> bool,
        {
            self.delegate_owned_mut(|l| l.for_each_mut(f));
        }

        fn len(&self) -> usize {
            self.delegate_owned(UiNodeList::len).unwrap_or(0)
        }

        fn boxed(self) -> BoxedUiNodeList {
            Box::new(self)
        }

        fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
            self.delegate_owned_mut(|l| l.drain_into(vec));
        }

        fn init_all(&mut self, ctx: &mut WidgetContext) {
            self.on_init(ctx);
            // delegation done in the handler
        }

        fn deinit_all(&mut self, ctx: &mut WidgetContext) {
            self.on_deinit(ctx);
            // delegation done in the handler
        }

        fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.on_event(ctx, update);
            self.delegate_owned_mut_with_handles(ctx, |ctx, l| l.event_all(ctx, update));
        }

        fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
            self.on_update(ctx, updates);
            let _ = observer;
            self.delegate_owned_mut_with_handles(ctx, |ctx, l| l.update_all(ctx, updates, observer));
        }

        fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.delegate_owned(|l| l.render_all(ctx, frame));
        }

        fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.delegate_owned(|l| l.render_update_all(ctx, update));
        }
    }
}

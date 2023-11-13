//! UI live inspector.
//!
//! Interactive UI inspectors can use this module as data source.

use std::{fmt, ops, sync::Arc};

use parking_lot::Mutex;
use zero_ui_view_api::window::FrameId;

use crate::{
    text::*,
    var::*,
    widget_builder::WidgetType,
    widget_info::{WidgetInfo, WidgetInfoTree},
    widget_instance::WidgetId,
    IdMap,
};

use super::{InspectorInfo, WidgetInfoInspectorExt};

#[derive(Default)]
struct InspectedTreeData {
    widgets: IdMap<WidgetId, InspectedWidget>,
    latest_frame: Option<ArcVar<FrameId>>,
}

/// Represents an actively inspected widget tree.
#[derive(Clone)]
pub struct InspectedTree {
    tree: ArcVar<WidgetInfoTree>,
    data: Arc<Mutex<InspectedTreeData>>,
}
impl fmt::Debug for InspectedTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InspectedTree")
            .field("tree", &self.tree.get())
            .finish_non_exhaustive()
    }
}
impl PartialEq for InspectedTree {
    fn eq(&self, other: &Self) -> bool {
        self.tree.var_ptr() == other.tree.var_ptr()
    }
}
impl InspectedTree {
    /// Initial inspection.
    pub fn new(tree: WidgetInfoTree) -> Self {
        Self {
            data: Arc::new(Mutex::new(InspectedTreeData::default())),
            tree: var(tree),
        }
    }

    /// Update inspection.
    ///
    /// # Panics
    ///
    /// Panics if info is not for the same window ID.
    pub fn update(&self, tree: WidgetInfoTree) {
        assert_eq!(self.tree.with(|t| t.window_id()), tree.window_id());

        // update and retain
        self.tree.set(tree.clone());

        let mut data = self.data.lock();
        let mut removed = false;
        for (k, v) in data.widgets.iter() {
            if let Some(w) = tree.get(*k) {
                v.update(w);
            } else {
                v.removed.set(true);
                removed = true;
            }
        }
        // update can drop children inspectors so we can't update inside the retain closure.
        data.widgets
            .retain(|k, v| v.info.strong_count() > 1 && (!removed || tree.get(*k).is_some()));

        if let Some(f) = &data.latest_frame {
            if f.strong_count() == 1 {
                data.latest_frame = None;
            } else {
                f.set(tree.stats().last_frame);
            }
        }
    }

    /// Update all render watcher variables.
    pub fn update_render(&self) {
        let mut data = self.data.lock();
        if let Some(f) = &data.latest_frame {
            if f.strong_count() == 1 {
                data.latest_frame = None;
            } else {
                f.set(self.tree.with(|t| t.stats().last_frame));
            }
        }
    }

    /// Create a weak reference to this tree.
    pub fn downgrade(&self) -> WeakInspectedTree {
        WeakInspectedTree {
            tree: self.tree.downgrade(),
            data: Arc::downgrade(&self.data),
        }
    }

    /// Latest info.
    pub fn tree(&self) -> impl Var<WidgetInfoTree> {
        self.tree.read_only()
    }

    /// Gets a widget inspector if the widget is in the latest info.
    pub fn inspect(&self, widget_id: WidgetId) -> Option<InspectedWidget> {
        match self.data.lock().widgets.entry(widget_id) {
            hashbrown::hash_map::Entry::Occupied(e) => Some(e.get().clone()),
            hashbrown::hash_map::Entry::Vacant(e) => self.tree.with(|t| {
                t.get(widget_id)
                    .map(|w| e.insert(InspectedWidget::new(w, self.downgrade())).clone())
            }),
        }
    }

    /// Gets a widget inspector for the root widget.
    pub fn inspect_root(&self) -> InspectedWidget {
        self.inspect(self.tree.with(|t| t.root().id())).unwrap()
    }

    /// Latest frame updated using [`update_render`].
    ///
    /// [`update_render`]: Self::update_render
    pub fn last_frame(&self) -> impl Var<FrameId> {
        let mut data = self.data.lock();
        data.latest_frame
            .get_or_insert_with(|| var(self.tree.with(|t| t.stats().last_frame)))
            .clone()
    }
}

/// Represents a weak reference to a [`InspectedTree`].
#[derive(Clone)]
pub struct WeakInspectedTree {
    tree: types::WeakArcVar<WidgetInfoTree>,
    data: std::sync::Weak<Mutex<InspectedTreeData>>,
}
impl WeakInspectedTree {
    /// Try to get a strong reference to the inspected tree.
    pub fn upgrade(&self) -> Option<InspectedTree> {
        Some(InspectedTree {
            tree: self.tree.upgrade()?,
            data: self.data.upgrade()?,
        })
    }
}

struct InspectedWidgetCache {
    tree: WeakInspectedTree,
    children: Option<BoxedVar<Vec<InspectedWidget>>>,
    parent_property_name: Option<BoxedVar<Txt>>,
}

/// Represents an actively inspected widget.
///
/// See [`InspectedTree::inspect`].
#[derive(Clone)]
pub struct InspectedWidget {
    info: ArcVar<WidgetInfo>,
    removed: ArcVar<bool>,
    cache: Arc<Mutex<InspectedWidgetCache>>,
}
impl fmt::Debug for InspectedWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InspectedWidget")
            .field("info", &self.info.get())
            .field("removed", &self.removed.get())
            .finish_non_exhaustive()
    }
}
impl PartialEq for InspectedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.info.var_ptr() == other.info.var_ptr()
    }
}
impl Eq for InspectedWidget {}
impl InspectedWidget {
    /// Initial inspection.
    fn new(info: WidgetInfo, tree: WeakInspectedTree) -> Self {
        Self {
            info: var(info),
            removed: var(false),
            cache: Arc::new(Mutex::new(InspectedWidgetCache {
                tree,
                children: None,
                parent_property_name: None,
            })),
        }
    }

    /// Update inspection.
    ///
    /// # Panics
    ///
    /// Panics if info is not for the same widget ID.
    fn update(&self, info: WidgetInfo) {
        assert_eq!(self.info.with(|i| i.id()), info.id());
        self.info.set(info);

        let mut cache = self.cache.lock();
        if let Some(c) = &cache.children {
            if c.strong_count() == 1 {
                cache.children = None;
            }
        }
        if let Some(c) = &cache.parent_property_name {
            if c.strong_count() == 1 {
                cache.parent_property_name = None;
            }
        }
    }

    /// If this widget inspector is permanently disconnected and will not update.
    ///
    /// This is set to `true` when an inspected widget is not found after an update, when `true`
    /// this inspector will not update even if the same widget ID is re-inserted in another update.
    pub fn removed(&self) -> impl Var<bool> {
        self.removed.read_only()
    }

    /// Latest info.
    pub fn info(&self) -> impl Var<WidgetInfo> {
        self.info.read_only()
    }

    /// Widget id.
    pub fn id(&self) -> WidgetId {
        self.info.with(|i| i.id())
    }

    /// Count of ancestor widgets.
    pub fn depth(&self) -> impl Var<usize> {
        self.info.map(|w| w.depth()).actual_var()
    }

    /// Count of descendant widgets.
    pub fn descendants_len(&self) -> impl Var<usize> {
        self.info.map(|w| w.descendants_len()).actual_var()
    }

    /// Widget type, if the widget was built with inspection info.
    pub fn wgt_type(&self) -> impl Var<Option<WidgetType>> {
        self.info.map(|w| Some(w.inspector_info()?.builder.widget_type())).actual_var()
    }

    /// Widget macro name, or `"<widget>!"` if widget was not built with inspection info.
    pub fn wgt_macro_name(&self) -> impl Var<Txt> {
        self.info
            .map(|w| match w.inspector_info().map(|i| i.builder.widget_type()) {
                Some(t) => formatx!("{}!", t.name()),
                None => Txt::from_static("<widget>!"),
            })
            .actual_var()
    }

    /// Gets the parent's property that has this widget as an input.
    ///
    /// Is an empty string if the widget is not inserted by any property.
    pub fn parent_property_name(&self) -> impl Var<Txt> {
        let mut cache = self.cache.lock();
        cache
            .parent_property_name
            .get_or_insert_with(|| {
                self.info
                    .map(|w| {
                        Txt::from_static(
                            w.parent_property()
                                .map(|(p, _)| w.parent().unwrap().inspect_property(p).unwrap().property().name)
                                .unwrap_or(""),
                        )
                    })
                    .actual_var()
                    .boxed()
            })
            .clone()
    }

    /// Inspect the widget children.
    pub fn children(&self) -> impl Var<Vec<InspectedWidget>> {
        let mut cache = self.cache.lock();
        let cache = &mut *cache;
        cache
            .children
            .get_or_insert_with(|| {
                let tree = cache.tree.clone();
                self.info
                    .map(move |w| {
                        if let Some(tree) = tree.upgrade() {
                            assert_eq!(&tree.tree.get(), w.tree());

                            w.children().map(|w| tree.inspect(w.id()).unwrap()).collect()
                        } else {
                            vec![]
                        }
                    })
                    .actual_var()
                    .boxed()
            })
            .clone()
    }

    /// Inspect the builder, properties and intrinsic nodes that make up the widget.
    ///
    /// Is `None` when the widget is built without inspector info collection.
    pub fn inspector_info(&self) -> impl Var<Option<InspectedInfo>> {
        self.info.map(move |w| w.inspector_info().map(InspectedInfo)).actual_var().boxed()
    }

    /// Create a variable that probes info after every frame is rendered.
    pub fn render_watcher<T: VarValue>(&self, mut probe: impl FnMut(&WidgetInfo) -> T + Send + 'static) -> impl Var<T> {
        merge_var!(
            self.info.clone(),
            self.cache.lock().tree.upgrade().unwrap().last_frame(),
            move |w, _| probe(w)
        )
    }
}

/// [`InspectorInfo`] that can be placed in a variable.
#[derive(Clone)]
pub struct InspectedInfo(pub Arc<InspectorInfo>);
impl fmt::Debug for InspectedInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl PartialEq for InspectedInfo {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl ops::Deref for InspectedInfo {
    type Target = InspectorInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//! Widget and property types.

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt, mem,
    rc::Rc,
};

use linear_map::LinearMap;

use crate::{
    handler::WidgetHandler,
    impl_from_and_into_var,
    inspector::SourceLocation,
    ui_list::BoxedUiNodeList,
    ui_list::BoxedWidgetList,
    var::{var, AnyVar, AnyVarValue, BoxedVar, RcVar, StateVar, Var, VarHandle, VarValue, Vars, WithVars},
    AdoptiveNode, BoxedUiNode, BoxedWidget, NilUiNode, UiNode, UiNodeList, Widget, WidgetList,
};

pub use crate::inspector::source_location;

#[doc(hidden)]
#[macro_export]
macro_rules! when_condition_expr_var {
    ($($tt:tt)*) => {
        $crate::var::Var::boxed($crate::var::expr_var!{$($tt)*})
    };
}
#[doc(hidden)]
pub use when_condition_expr_var;

/// Property priority in a widget.
///
/// See [the property doc](crate::property#priority) for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// [Context](crate::property#context) property.
    Context,
    /// [Event](crate::property#event) property.
    Event,
    /// [Layout](crate::property#layout) property.
    Layout,
    /// [Size](crate::property#size) property.
    Size,
    /// [Border](crate::property#border) property.
    Border,
    /// [Fill](crate::property#fill) property.
    Fill,
    /// [Child Context](crate::property#child-context) property.
    ChildContext,
    /// [Child Layout](crate::property#child-layout) property.
    ChildLayout,
}
impl Priority {
    /// All priorities, from outermost([`Context`]) to innermost([`ChildLayout`]).
    ///
    /// [`Context`]: Priority::Context
    /// [`ChildLayout`]: Priority::ChildLayout
    pub const ITEMS: [Priority; 8] = [
        Priority::Context,
        Priority::Event,
        Priority::Layout,
        Priority::Size,
        Priority::Border,
        Priority::Fill,
        Priority::ChildContext,
        Priority::ChildLayout,
    ];
}

/// Kind of property input.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum InputKind {
    /// Input is `impl IntoVar<T>`, build value is `BoxedVar<T>`.
    Var,
    /// Input and build value is `StateVar`.
    StateVar,
    /// Input is `impl IntoValue<T>`, build value is `T`.
    Value,
    /// Input is `impl UiNode`, `impl Widget`, `impl WidgetHandler<A>`, ``, build value is `InputTakeout`.
    Takeout,
}

/// Represents a value that cannot be cloned and can only be used in one instance.
pub struct InputTakeout {
    val: Rc<RefCell<Option<Box<dyn Any>>>>,
}
impl InputTakeout {
    fn new(val: Box<dyn Any>) -> Self {
        InputTakeout {
            val: Rc::new(RefCell::new(Some(val))),
        }
    }

    /// New from `impl UiNode` input.
    pub fn new_ui_node(node: impl UiNode) -> Self {
        Self::new(Box::new(node.boxed()))
    }

    /// New from `impl Widget` input.
    pub fn new_widget(wgt: impl Widget) -> Self {
        Self::new(Box::new(wgt.boxed_wgt()))
    }

    /// New from `impl WidgetHandler<A>` input.
    pub fn new_widget_handler<A>(handler: impl WidgetHandler<A>) -> Self
    where
        A: Clone + 'static,
    {
        Self::new(Box::new(handler.boxed()))
    }

    /// New from `impl UiNodeList` input.
    pub fn new_ui_node_list(list: impl UiNodeList) -> Self {
        Self::new(Box::new(list.boxed()))
    }

    /// New from `impl WidgetList` input.
    pub fn new_widget_list(list: impl WidgetList) -> Self {
        Self::new(Box::new(list.boxed_wgt()))
    }

    /// If the args was not spend yet.
    pub fn is_available(&self) -> bool {
        self.val.borrow().is_some()
    }

    fn take<T: Any>(&self) -> T {
        *self
            .val
            .borrow_mut()
            .take()
            .expect("input takeout already used")
            .downcast::<T>()
            .expect("input takeout was of the requested type")
    }

    /// Takes the value for an `impl UiNode` input.
    pub fn take_ui_node(&self) -> BoxedUiNode {
        self.take()
    }

    /// Takes the value for an `impl UiNode` input.
    pub fn take_widget(&self) -> BoxedWidget {
        self.take()
    }

    /// Takes the value for an `impl WidgetHandler<A>` input.
    pub fn take_widget_handler<A: Clone + 'static>(&self) -> Box<dyn WidgetHandler<A>> {
        self.take()
    }

    /// Takes the value for an `impl UiNodeList` input.
    pub fn take_ui_node_list(&self) -> BoxedUiNodeList {
        self.take()
    }

    /// Takes the value for an `impl WidgetList` input.
    pub fn take_widget_list(&self) -> BoxedWidgetList {
        self.take()
    }
}

/// Property info.
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// Property insert order.
    pub priority: Priority,

    /// Unique type ID that identifies the property.
    pub unique_id: TypeId,
    /// Property original name.
    pub name: &'static str,

    /// Property declaration location.
    pub location: SourceLocation,

    /// Function that constructs the default args for the property.
    pub default: Option<fn(PropertyInstInfo) -> Box<dyn PropertyArgs>>,

    /// Property inputs info, always at least one.
    pub inputs: Box<[PropertyInput]>,
}

/// Property instance info.
#[derive(Debug, Clone)]
pub struct PropertyInstInfo {
    /// Property name in this instance.
    ///
    /// This can be different from [`PropertyInfo::name`] if the property was renamed by the widget.
    pub name: &'static str,

    /// Property instantiation location.
    pub location: SourceLocation,
}
impl PropertyInstInfo {
    /// No info.
    pub fn none() -> Self {
        PropertyInstInfo {
            name: "",
            location: SourceLocation {
                file: "",
                line: 0,
                column: 0,
            },
        }
    }

    /// Returns `true` if there is no instance info.
    pub fn is_none(&self) -> bool {
        self.name.is_empty()
    }
}

/// Property input info.
#[derive(Debug, Clone)]
pub struct PropertyInput {
    /// Input name.
    pub name: &'static str,
    /// Input kind.
    pub kind: InputKind,
    /// Type as defined by kind.
    pub ty: TypeId,
    /// Type name.
    pub ty_name: &'static str,
}

/// Represents a property instantiation request.
pub trait PropertyArgs {
    /// Property info.
    fn property(&self) -> PropertyInfo;

    /// Instance info.
    fn instance(&self) -> PropertyInstInfo;

    /// Unique ID.
    fn id(&self) -> PropertyId {
        PropertyId {
            unique_id: self.property().unique_id,
            name: self.instance().name,
        }
    }

    /// Gets a [`InputKind::Value`].
    fn value(&self, i: usize) -> &dyn AnyVarValue {
        panic_input(&self.property(), i, InputKind::Value)
    }

    /// Gets a [`InputKind::Var`].
    ///
    /// Is a `BoxedVar<T>`.
    fn var(&self, i: usize) -> &dyn AnyVar {
        panic_input(&self.property(), i, InputKind::Var)
    }

    /// Gets a [`InputKind::StateVar`].
    fn state_var(&self, i: usize) -> &StateVar {
        panic_input(&self.property(), i, InputKind::StateVar)
    }

    /// Gets a [`InputKind::Takeout`].
    fn takeout(&self, i: usize) -> &InputTakeout {
        panic_input(&self.property(), i, InputKind::Takeout)
    }

    /// Create a property instance with args clone or taken.
    fn instantiate(&self, child: BoxedUiNode) -> BoxedUiNode;
}

#[doc(hidden)]
pub fn panic_input(info: &PropertyInfo, i: usize, kind: InputKind) -> ! {
    if i > info.inputs.len() {
        panic!("index out of bounds, the input len is {}, but the index is {i}", info.inputs.len())
    } else if info.inputs[i].kind != kind {
        panic!(
            "invalid input request `{:?}`, but `{}` is `{:?}`",
            kind, info.inputs[i].name, info.inputs[i].kind
        )
    } else {
        panic!("invalid input `{}`", info.inputs[i].name)
    }
}

/*

 WIDGET

*/

enum WidgetItem {
    Instrinsic(AdoptiveNode<BoxedUiNode>),
    Property {
        importance: Importance,
        args: Box<dyn PropertyArgs>,
    },
}

/// Value that indicates the override importance of a property instance, higher overrides lower.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Importance(pub usize);
impl Importance {
    /// Importance of default values defined in the widget declaration.
    pub const WIDGET: Importance = Importance(1000);
    /// Importance of values defined in the widget instantiation.
    pub const INSTANCE: Importance = Importance(1000 * 10);
}
impl_from_and_into_var! {
    fn from(imp: usize) -> Importance {
        Importance(imp)
    }
}

/// Unique identifier of a property, properties with the same id override each other in a widget and are joined
/// into a single instance is assigned in when blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyId {
    /// The [`PropertyInfo::unique_id`].
    pub unique_id: TypeId,
    /// The [`PropertyInstInfo::name`].
    pub name: &'static str,
}

/// Represents what member and how it was accessed in a [`WhenInput`].
#[derive(Clone, Copy, Debug)]
pub enum WhenInputMember {
    /// Member was accessed by name.
    Named(&'static str),
    /// Member was accessed by index.
    Index(usize),
}

/// Input var read in a `when` condition expression.
#[derive(Clone)]
pub struct WhenInput {
    /// Property.
    pub property: PropertyId,
    /// What member and how it was accessed for this input.
    pub member: WhenInputMember,
    /// Input var.
    pub var: WhenInputVar,
}
impl fmt::Debug for WhenInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WhenInput")
            .field("property", &self.property)
            .field("member", &self.member)
            .finish_non_exhaustive()
    }
}

enum WhenInputVarActual<T: VarValue> {
    None,
    Some { var: RcVar<T>, handle: VarHandle },
}
impl<T: VarValue> WhenInputVarActual<T> {
    fn bind_init(&mut self, vars: &Vars, other: &impl Var<T>) {
        match self {
            WhenInputVarActual::None => {
                let var = var(other.get());
                *self = Self::Some {
                    handle: other.bind(&var),
                    var,
                }
            }
            WhenInputVarActual::Some { var, handle } => {
                var.set(vars, other.get());
                *handle = other.bind(var);
            }
        }
    }

    fn bind_init_value(&mut self, vars: &Vars, value: T) {
        match self {
            WhenInputVarActual::None => {
                *self = Self::Some {
                    var: var(value),
                    handle: VarHandle::dummy(),
                }
            }
            WhenInputVarActual::Some { var, handle } => {
                *handle = VarHandle::dummy();
                var.set(vars, value);
            }
        }
    }
}
trait AnyWhenInputVarActual: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn is_some(&self) -> bool;
}
impl<T: VarValue> AnyWhenInputVarActual for WhenInputVarActual<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn is_some(&self) -> bool {
        matches!(self, Self::Some { .. })
    }
}

/// Represents a [`WhenInput`] variable that can be rebound.
#[derive(Clone)]
pub struct WhenInputVar {
    var: Rc<RefCell<dyn AnyWhenInputVarActual>>,
}
impl WhenInputVar {
    /// New for property without default value.
    pub fn new<T: VarValue>() -> (Self, impl Var<T>) {
        let rc: Rc<RefCell<dyn AnyWhenInputVarActual>> = Rc::new(RefCell::new(WhenInputVarActual::<T>::None));
        (
            WhenInputVar { var: rc.clone() },
            crate::var::types::ContextualizedVar::new(Rc::new(move || {
                match rc.borrow().as_any().downcast_ref::<WhenInputVarActual<T>>().unwrap() {
                    WhenInputVarActual::Some { var, .. } => var.read_only(),
                    WhenInputVarActual::None => panic!("when var input not inited"),
                }
            })),
        )
    }

    /// Returns `true` if a default or bound value has inited the variable and it is of type `T`.
    ///
    /// Note that attempting to use the [`WhenInfo::state`] when this is `false` will cause a panic.
    pub fn can_use(&self) -> bool {
        self.var.borrow().is_some()
    }

    /// Assign and bind the input var from `other`, after this call [`can_use`] is `true`.
    ///
    /// # Panics
    ///
    /// If `T` is not the same that was used to create the input var.
    pub fn bind<T: VarValue>(&self, vars: impl WithVars, other: &impl Var<T>) {
        vars.with_vars(|vars| self.validate_borrow_mut::<T>().bind_init(vars, other))
    }

    /// Assigns the input var to `value` and removes any previous binding, after this call [`can_use`] is `true`.
    ///
    /// # Panics
    ///
    /// If `T` is not the same that was used to create the input var.
    pub fn bind_value<T: VarValue>(&self, vars: impl WithVars, value: T) {
        vars.with_vars(|vars| self.validate_borrow_mut::<T>().bind_init_value(vars, value))
    }

    fn validate_borrow_mut<T: VarValue>(&self) -> std::cell::RefMut<WhenInputVarActual<T>> {
        std::cell::RefMut::map(self.var.borrow_mut(), |var| {
            match var.as_mut_any().downcast_mut::<WhenInputVarActual<T>>() {
                Some(a) => a,
                None => panic!("incorrect when input var type"),
            }
        })
    }
}

/// Represents a `when` block in a widget.
pub struct WhenInfo {
    /// Properties referenced in the when condition expression.
    ///
    /// They are type erased `RcVar<T>` instances and can be rebound, other variable references (`*#{var}`) are imbedded in
    /// the build expression and cannot be modified.
    pub inputs: Box<[WhenInput]>,

    /// Output of the when expression.
    ///
    /// # Panics
    ///
    /// If used when [`can_use`] is `false`.
    pub state: BoxedVar<bool>,

    /// Properties assigned in the when block, in the build widget they are joined with the default value and assigns
    /// from other when blocks into a single property instance set to `when_var!` inputs.
    pub assigns: Box<[Box<dyn PropertyArgs>]>,

    /// The condition expression code.
    pub expr: &'static str,
}
impl WhenInfo {
    /// Returns `true` if the [`state`] var is valid because it does not depend of any property input or all
    /// property inputs are inited with a value or have a default.
    pub fn can_use(&self) -> bool {
        self.inputs.iter().all(|i| i.var.can_use())
    }
}

/// Widget instance builder.
#[derive(Default)]
pub struct WidgetBuilder {
    child: Option<BoxedUiNode>,
    items: Vec<(Priority, WidgetItem)>,
    unset: LinearMap<PropertyId, Importance>,
    whens: Vec<(Importance, WhenInfo)>,
}
impl WidgetBuilder {
    /// New empty default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert intrinsic node, that is a core functionality node of the widget that cannot be overridden.
    pub fn insert_intrinsic(&mut self, priority: Priority, node: AdoptiveNode<BoxedUiNode>) {
        self.items.push((priority, WidgetItem::Instrinsic(node)));
    }

    /// Insert/override a property.
    pub fn insert_property(&mut self, importance: Importance, args: Box<dyn PropertyArgs>) {
        let property_id = args.id();
        let info = args.property();
        if let Some(i) = self.property_position(property_id) {
            match &self.items[i].1 {
                WidgetItem::Property { importance: imp, .. } => {
                    if *imp <= importance {
                        // override
                        self.items[i] = (info.priority, WidgetItem::Property { importance, args })
                    }
                }
                WidgetItem::Instrinsic(_) => unreachable!(),
            }
        } else {
            if let Some(imp) = self.unset.get(&property_id) {
                if *imp >= importance {
                    return; // unset blocks.
                }
            }
            self.items.push((info.priority, WidgetItem::Property { importance, args }))
        }
    }

    fn property_position(&self, property_id: PropertyId) -> Option<usize> {
        self.items.iter().position(|(_, item)| match item {
            WidgetItem::Property { args, .. } => args.id() == property_id,
            WidgetItem::Instrinsic(_) => false,
        })
    }

    /// Insert a `name = unset!;` property.
    pub fn insert_unset(&mut self, importance: Importance, property_id: PropertyId) {
        let check;

        match self.unset.entry(property_id) {
            linear_map::Entry::Occupied(mut e) => {
                let i = e.get_mut();
                check = *i < importance;
                *i = importance;
            }
            linear_map::Entry::Vacant(e) => {
                check = true;
                e.insert(importance);
            }
        }

        if check {
            self.items.retain(|(_, it)| match it {
                WidgetItem::Property { importance: imp, args } => args.id() != property_id || *imp > importance,
                WidgetItem::Instrinsic(_) => true,
            });
        }
    }

    /// Remove the property that matches the `property_id!(..)`.
    pub fn remove_property(&mut self, property_id: PropertyId) -> Option<(Importance, Box<dyn PropertyArgs>)> {
        if let Some(i) = self.property_position(property_id) {
            match self.items.remove(i).1 {
                // can't be swap remove for ordering of equal priority.
                WidgetItem::Property { importance, args, .. } => Some((importance, args)),
                WidgetItem::Instrinsic(_) => unreachable!(),
            }
        } else {
            None
        }

        // this method is used to remove "captures", that means we need to remove `when` assigns and a clone of the conditions too?
    }

    /// Insert a `when` block.
    pub fn insert_when(&mut self, importance: Importance, when: WhenInfo) {
        self.whens.push((importance, when))
    }

    /// If a child not is already set in the builder.
    ///
    /// If build without child the [`NilUiNode`] is used as the innermost node.
    pub fn has_child(&self) -> bool {
        self.child.is_some()
    }

    /// Set/replace the inner most node of the widget.
    pub fn set_child(&mut self, node: BoxedUiNode) -> Option<BoxedUiNode> {
        self.child.replace(node)
    }

    fn sort_items(&mut self) {
        self.items.sort_by(|(a_pri, a_item), (b_pri, b_item)| match a_pri.cmp(b_pri) {
            std::cmp::Ordering::Equal => match (a_item, b_item) {
                // INSTANCE importance is innermost of DEFAULT.
                (WidgetItem::Property { importance: a_imp, .. }, WidgetItem::Property { importance: b_imp, .. }) => a_imp.cmp(b_imp),
                // Intrinsic is outermost of priority items.
                (WidgetItem::Property { .. }, WidgetItem::Instrinsic(_)) => std::cmp::Ordering::Greater,
                (WidgetItem::Instrinsic(_), WidgetItem::Property { .. }) => std::cmp::Ordering::Less,
                (WidgetItem::Instrinsic(_), WidgetItem::Instrinsic(_)) => std::cmp::Ordering::Equal,
            },
            ord => ord,
        });

        self.whens.sort_by_key(|(imp, _)| *imp);
    }

    /// Instantiate and link all property and intrinsic nodes, returns the outermost node.
    pub fn build(mut self) -> BoxedUiNode {
        self.sort_items();

        let mut child = self.child.unwrap_or_else(|| NilUiNode.boxed());

        for (_, item) in self.items {
            match item {
                WidgetItem::Instrinsic(node) => {
                    let (c, n) = node.into_parts();
                    *c.borrow_mut() = mem::replace(&mut child, n);
                }
                WidgetItem::Property { args, .. } => {
                    child = args.instantiate(child);
                }
            }
        }

        child
    }

    /// Build to a new editable node.
    pub fn build_editable(mut self) -> EditableWgtNode {
        self.sort_items();

        let child = self.child.unwrap_or_else(|| NilUiNode.boxed());
        let child = Rc::new(RefCell::new(child));

        let mut prev_pri = Priority::Context;
        let mut priority_ranges = [0; Priority::ITEMS.len()];

        let mut items = Vec::with_capacity(self.items.len());
        for (i, (priority, item)) in self.items.into_iter().enumerate() {
            if prev_pri != priority {
                prev_pri = priority;
                for idx in &mut priority_ranges[priority as usize..] {
                    *idx = i;
                }
            }

            match item {
                WidgetItem::Instrinsic(node) => {
                    let (child, node) = node.into_parts();
                    let node = Rc::new(RefCell::new(node));
                    items.push(EditableItem {
                        child,
                        node,
                        snapshot_node: None,
                        property: None,
                    });
                }
                WidgetItem::Property { importance, args } => {
                    let node = AdoptiveNode::new(|child| args.instantiate(child.boxed()));
                    let (child, node) = node.into_parts();

                    todo!("takeout")
                }
            }
        }

        let node = items.last().map(|it| it.node.clone()).unwrap_or_else(|| child.clone());

        EditableWgtNode {
            id: EditableWgtNodeId::new_unique(),
            child,
            items,
            priority_ranges,
            node,
            is_inited: false,
            is_bound: false,
            unset: self.unset,
        }
    }

    /// Build into an existing editable node, overrides/extends it.
    pub fn build_into(mut self, node: &mut EditableWgtNode) {
        for (id, imp) in self.unset {
            let check;
            if let Some(prev_imp) = node.unset.insert(id, imp) {
                check = prev_imp < imp;
            } else {
                check = true;
            }

            if check {
                todo!()
            }
        }
        todo!()
    }
}

unique_id_32! {
    struct EditableWgtNodeId;
}
impl fmt::Debug for EditableWgtNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("EditableWgtNodeId").field(&self.sequential()).finish()
    }
}

struct EditableItem {
    // item child, the `Rc` does not change, only the interior.
    child: Rc<RefCell<BoxedUiNode>>,
    // item node, the `Rc` changes, but it always points to the same node.
    node: Rc<RefCell<BoxedUiNode>>,
    // original `node`, preserved when parent is set, reused when unset.
    snapshot_node: Option<Rc<RefCell<BoxedUiNode>>>,

    // property source args or `None` for intrinsic.
    property: Option<(Importance, Box<dyn PropertyArgs>)>,
}

/// Represents a built [`WidgetBuilder`] node that can still be modified when deinited.
pub struct EditableWgtNode {
    // Unique ID used to validate snapshots.
    id: EditableWgtNodeId,

    // innermost child.
    //
    // The Rc changes to the `child` of the innermost property when bound and a new Rc when unbound,
    // the interior only changes when `replace_child` is used.
    child: Rc<RefCell<BoxedUiNode>>,

    // property and intrinsic nodes from innermost to outermost.
    items: Vec<EditableItem>,
    // exclusive end of each priority range in `properties`
    priority_ranges: [usize; Priority::ITEMS.len()],

    // outermost node.
    //
    // The Rc changes to the `node` of the outermost property, the interior is not modified from here.
    node: Rc<RefCell<BoxedUiNode>>,

    is_inited: bool,
    is_bound: bool,

    // unset requests, already applied.
    unset: LinearMap<PropertyId, Importance>,
}
impl EditableWgtNode {
    /// If the node is inited in a context, if `true` the node cannot be restored into a builder.
    pub fn is_inited(&self) -> bool {
        self.is_inited
    }

    fn delink(&mut self) {
        assert!(!self.is_inited);

        if !mem::take(&mut self.is_bound) {
            return;
        }

        todo!()
    }

    fn link(&mut self) {
        assert!(!self.is_inited);

        if mem::replace(&mut self.is_bound, true) {
            return;
        }

        todo!()
    }

    /// Take a snapshot that can be used to restore the node to a pre-injection state.
    pub fn snapshot(&self) -> DynUiNodeSnapshot {
        assert!(!self.is_inited);
        todo!()
    }

    /// Restore the node properties.
    pub fn restore(&mut self, snapshot: DynUiNodeSnapshot) {
        self.delink();
        todo!()
    }

    /// Insert/override nodes from `other` onto `self`.
    ///
    /// Intrinsic nodes are moved in, property nodes of the same name, id and >= importance replace self, when conditions and assigns
    /// are rebuild.
    pub fn inject(&mut self, other: EditableWgtNode) {
        self.delink();
        todo!()
    }
}

/// Represents a state of a [`DynUiNode`], can be used to restore the node.
pub struct DynUiNodeSnapshot {}

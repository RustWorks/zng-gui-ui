//! Accessibility and automation types.

use std::{num::NonZeroU32, ops};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::units::{DipRect, DipVector};

/// Accessibility role of a node in the accessibility tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AccessRole {
    /// Clickable widgets that trigger a response when activated by the user.
    Button,
    /// checkable interactive widget.
    ///
    /// Must also set [`AccessState::Checked`].
    CheckBox,
    /// Identifies a cell in a grid widget.
    GridCell,
    /// Interactive reference to a resource
    Link,
    /// Indicates the widget is an option in a set of choices contained by a menu or menu-bar.
    MenuItem,
    /// Widget is a checkable option in a menu.
    ///
    /// Must also set [`AccessState::Checked`].
    MenuItemCheckBox,
    /// Widget is a selectable option in a menu where only one option can be selected at a time.
    MenuItemRadio,
    /// Selectable items in a list-box.
    Option,
    /// Defines a widget that displays the progress status for tasks that take a long time.
    ///
    /// The [`AccessState::ValueNow`] and other value states define the progress.
    ProgressBar,
    /// Selectable items in a list where only one item may be selected at a time.
    Radio,
    /// Widget controls the scrolling of content within a viewing area.
    ///
    /// Must also set [`AccessState::Controls`] and [`AccessState::ValueNow`] to define
    /// the scroll widget and amount scrolled. By default the value min/max is 0/100.
    ScrollBar,
    /// Identifies a text-box that is used for searching.
    SearchBox,
    /// Defines an input where the user selects a value from within a given range.
    ///
    /// The [`AccessState::ValueNow`] and other value states define the range and value.
    Slider,
    /// Defines a type of range that expects the user to select a value from among discrete choices.
    SpinButton,
    /// Identifies a check-box with named states.
    Switch,
    /// Identifies a widget in a tab-list that selects the active tab in a tab-panel.
    Tab,
    /// Identifies a container for the active tab.
    TabPanel,
    /// Identifies a widget that allows the input of free-form text.
    TextBox,
    /// Identifies an item in a tree widget.
    TreeItem,

    /// Identifies a widget as an input that controls another widget,
    /// such as a list-box or grid, that can dynamically pop up to help the user set the value of that input.
    ComboBox,
    /// Identifies a container of columns, rows and cells.
    Grid,
    /// Identifies a list of selectable items.
    ListBox,
    /// Identifies a composite widget that offers a list of choices to the user.
    Menu,
    /// Identifies the part of a menu that always stays visible.
    MenuBar,
    /// Identifies a group of radio buttons.
    RadioGroup,
    /// Identifies the widget that serves as the container for a set of tabs. The selected tab content
    /// is shown in a [`TabPanel`].
    ///
    /// [`TabPanel`]: Self::TabPanel
    TabList,
    /// Widget that allows the user to select one or more items from a hierarchically organized collection.
    Tree,
    /// Identifies an widget as being grid whose rows can be expanded and collapsed in the same manner as for a tree.
    TreeGrid,

    /// Indicates to assistive technologies that an widget and all of its children should be treated similar to a desktop application.
    Application,
    /// Indicates a section of a page that could easily stand on its own.
    Article,
    /// Identifies a widget as being a cell in a tabular container that does not contain column or row header information.
    Cell,
    /// Identifies a widget as being a cell in a row contains header information for a column.
    ColumnHeader,
    /// Indicates the widget is a definition of a term or concept.
    Definition,
    /// Focusable content within complex composite widgets or applications
    /// for which assistive technologies can switch reading context back to a reading mode.
    Document,
    /// Identifies a dynamic scrollable list of articles in which articles are added to or
    /// removed from either end of the list as the user scrolls.
    Feed,
    /// Identify a figure inside page content where appropriate semantics do not already exist.
    Figure,
    /// Identifies a set of user interface objects that is not intended to be included in a page
    /// summary or table of contents by assistive technologies.
    Group,
    /// Defines a heading to a page or section, with [`AccessState::Level`] defining structure.
    Heading,
    /// Identifies a widget container that should be considered as a single image.
    Img,
    /// Identifies a list of items.
    List,
    /// Identifies an item inside a list of items.
    ListItem,
    /// Indicates that the content represents a mathematical expression.
    Math,
    /// Identifies a section whose content is parenthetic or ancillary to the main content.
    Note,
    /// Identifies an area visible to assistive technologies, but without any specific role.
    Presentation,
    /// Identifies a row of cells within a tabular structure.
    Row,
    /// Identifies a group of rows within a tabular structure.
    RowGroup,
    /// Identifies a cell containing header information for a row within a tabular structure.
    RowHeader,
    /// Identifies a divider that separates and distinguishes sections of content or groups of menu items.
    Separator,
    /// Identifies the widget containing the role as having a non-interactive table structure containing data arranged in rows and columns.
    Table,
    /// Identifies a word or phrase with an optional corresponding [`Definition`].
    ///
    /// [`Definition`]: Self::Definition
    Term,
    /// Defines the containing widget as a collection of commonly used function buttons or controls represented in a compact visual form.
    ToolBar,
    /// Identifies a contextual text bubble that displays a description for an element that appears on pointer hover or keyboard focus.
    ToolTip,

    /// Identifies the global header, which usually includes a logo, company name, search feature, and possibly the global navigation or a slogan.
    Banner,
    /// Identifies a supporting section that relates to the main content.
    Complementary,
    /// Identifies a footer, containing identifying information such as copyright information, navigation links, and privacy statements.
    ContentInfo,
    /// Identify a group of widgets that are a register form.
    Form,
    /// Identifies the primary content.
    Main,
    /// Identifies major groups of links used for navigating the app.
    Navigation,
    /// Identifies significant areas. Usually set with [`AccessState::Label`].
    Region,
    /// Identifies the search area or form.
    Search,

    /// Identifies important, and usually time-sensitive, information.
    Alert,
    /// Identifies a widget that creates a live region where new information is added in a
    /// meaningful order and old information may disappear.
    Log,
    /// Identifies a live region containing non-essential information which changes frequently.
    Marquee,
    /// Identifies a live region containing advisory information for the user that is not
    /// important enough to be an alert.
    Status,
    /// Indicates to assistive technologies that a widget is a numerical counter listing the amount
    /// of elapsed time from a starting point or the remaining time until an end point.
    /// Assistive technologies will not announce updates to a timer.
    Timer,

    /// Identifies a modal alert dialogs that interrupt a user's workflow to communicate an important message and require a response.
    AlertDialog,
    /// Identifies a widget that has content separate from the normal window and is presented as an overlay.
    Dialog,
}

/// Accessibility attribute of a node in the accessibility tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AccessState {
    /// Inputting text triggers display of one or more predictions of the user's intended
    /// value for a [`ComboBox`], [`SearchBox`], or [`TextBox`].
    ///
    /// [`ComboBox`]: AccessRole::ComboBox
    /// [`SearchBox`]: AccessRole::SearchBox
    /// [`TextBox`]: AccessRole::TextBox
    AutoComplete(AutoComplete),

    /// If the widget is checked (`Some(true)`), unchecked (`Some(false)`), or if the checked status is indeterminate (`None`).
    Checked(Option<bool>),

    /// Represents the current page within a set of pages such as the link to the current document in a breadcrumb.
    CurrentPage,
    /// Represents the current step within a process such as the current step in an enumerated multi step checkout flow .
    CurrentStep,
    /// Represents the current location within an environment or context such as the image that is visually
    /// highlighted as the current component of a flow chart.
    CurrentLocation,
    /// Represents the current date within a collection of dates such as the current date within a calendar.
    CurrentDate,
    /// Represents the current time within a set of times such as the current time within a timetable.
    CurrentTime,
    /// Represents the current item within a set.
    CurrentItem,

    /// Indicates that the widget is perceivable but disabled, so it is not editable or otherwise operable.
    Disabled,

    /// Indicates that the widget is an error message for the referenced node.
    ///
    /// The other widget must be [`Invalid`].
    ///
    /// [`Invalid`]: Self::Invalid
    ErrorMessage(AccessNodeId),

    /// Indicate that the widget toggles the visibility of related widgets.
    ///
    /// Use  [`Controls`], or [`Owns`] to indicate the widgets that change visibility based on
    /// this value.
    ///
    /// [`Controls`]: Self::Controls
    /// [`Owns`]: Self::Owns
    Expanded(bool),

    /// Indicates the availability and type of interactive popup widget.
    HasPopup(Popup),

    /// Indicates the entered value does not conform to the format expected by the application.
    Invalid,
    /// Indicates the entered value contains a grammatical error.
    InvalidGrammar,
    /// Indicates the entered value contains a spelling error.
    InvalidSpelling,

    /// Defines a string value that labels the widget.
    Label(String),

    /// Defines the hierarchical level of an widget within a structure.
    Level(NonZeroU32),
    /// Indicates whether the widget is modal when displayed.
    Modal,
    /// Indicates whether a [`TextBox`] accepts multiple lines of input.
    ///
    /// [`TextBox`]: AccessRole::TextBox
    MultiLine,
    /// Indicates that the user may select more than one item from the current selectable descendants.
    MultiSelectable,
    /// Indicates whether the widget's orientation is horizontal, vertical, or unknown/ambiguous.
    Orientation(Orientation),
    /// Short hint (a word or short phrase) intended to help the user with data entry when a form control has no value.
    Placeholder(String),
    /// If the toggle button is pressed.
    Pressed,
    /// Indicates that the widget is not editable, but is otherwise operable.
    ReadOnly,
    /// Indicates that user input is required on the widget before a form may be submitted.
    Required,
    /// Indicates that the widget is selected.
    Selected,
    /// Indicates if items in a table or grid are sorted in ascending or descending order.
    Sort(SortDirection),
    /// Defines the maximum value (inclusive).
    ValueMax(f64),
    /// Defines the minimum value (inclusive).
    ValueMin(f64),
    /// Defines the current value.
    ValueNow(f64),
    /// Defines a human readable version of the [`ValueNow`].
    ///
    /// [`ValueNow`]: Self::ValueNow
    ValueText(String),

    /// Indicates that a widget will be updated, and describes the types of
    /// updates the user agents, assistive technologies, and user can expect from the live region.
    Live {
        /// How the changes must be notified.
        indicator: LiveIndicator,
        /// What changes are expected.
        changes: LiveChange,
        /// If the live region must be re-read entirely after each update.
        atomic: bool,
        /// Indicates the live area being modified and that assistive technologies may want
        /// to wait until the changes are complete before informing the user about the update.
        busy: bool,
    },

    /// Identifies the currently active widget when focus is on a composite widget, [`ComboBox`], [`TextBox`], [`Group`], or [`Application`].
    ///
    /// [`ComboBox`]: AccessRole::ComboBox
    /// [`TextBox`]: AccessRole::TextBox
    /// [`Group`]: AccessRole::Group
    /// [`Application`]: AccessRole::Application
    ActiveDescendant(AccessNodeId),

    /// Defines the total number of columns in a [`Table`], [`Grid`], or [`TreeGrid`] when not all columns are present in tree.
    ///
    /// The value `0` indicates that not all columns are in the widget and the application cannot determinate the exact number.
    ///
    /// [`Table`]: AccessRole::Table
    /// [`Grid`]: AccessRole::Grid
    /// [`TreeGrid`]: AccessRole::TreeGrid
    ColCount(usize),
    /// Defines an widget's column index in the parent table or grid.
    ColIndex(usize),
    /// Defines the number of columns spanned by the widget in the parent table or grid.
    ColSpan(usize),
    /// Identifies the widget(s) whose contents or presence are controlled by this widget.
    Controls(Vec<AccessNodeId>),
    /// Identifies the widget(s) that describes this widget.
    DescribedBy(Vec<AccessNodeId>),
    /// identifies the widget(s) that provide additional information related to this widget.
    Details(Vec<AccessNodeId>),
    /// Override the next node in reading order.
    FlowTo(AccessNodeId),
    /// Identifies the widget(s) that labels the widget it is applied to.
    LabelledBy(Vec<AccessNodeId>),
    /// Identifies widget(s) in order to define a visual, functional, or contextual relationship between a parent and its child
    /// widgets when the tree hierarchy cannot be used to represent the relationship.
    Owns,
    /// Defines the widget's number or position in the current set of list items or tree items when not all items are present in the tree.
    PosInSet,
    /// Defines the total number of rows in a [`Table`], [`Grid`], or [`TreeGrid`] when not all rows are present in tree.
    ///
    /// The value `0` indicates that not all rows are in the widget and the application cannot determinate the exact number.
    ///
    /// [`Table`]: AccessRole::Table
    /// [`Grid`]: AccessRole::Grid
    /// [`TreeGrid`]: AccessRole::TreeGrid
    RowCount,
    /// Defines an widget's row index in the parent table or grid.
    RowIndex(usize),
    /// Defines the number of rows spanned by the widget in the parent table or grid.
    RowSpan(usize),
    /// Defines the number of items in the current set of list items or tree items when not all items in the set are present in the tree.
    SetSize(usize),
}

/// Defines how a live update is communicated to the user.
///
/// See [`AccessState::Sort`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LiveIndicator {
    /// Indicates that updates to the region have the highest priority and should be presented to the user immediately.
    Assertive,
    /// Indicates that updates to the region should **not** be presented to the user unless the user is currently focused on that region.
    OnlyIfFocused,
    /// Indicates that updates to the region should be presented at the next graceful opportunity, such as at the end of
    /// speaking the current sentence or when the user pauses typing.
    Polite,
}

/// Sort direction.
///
/// See [`AccessState::Sort`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortDirection {
    /// Items are sorted in ascending order by this column.
    Ascending,
    /// Items are sorted in descending order by this column.
    Descending,
}

/// Widget orientation.
///
/// See [`AccessState::Orientation`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Orientation {
    /// Widget is horizontal.
    Horizontal,
    /// Widget is vertical.
    Vertical,
}

/// Popup type.
///
/// See [`AccessState::HasPopup`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Popup {
    /// The popup is a menu.
    Menu,
    /// The popup is a list-box.
    ListBox,
    /// The popup is a tree.
    Tree,
    /// The popup is a grid.
    Grid,
    /// The popup is a dialog.
    Dialog,
}

bitflags! {
    /// Defines how inputting text could trigger display of one or more predictions of the user's intended value.
    ///
    /// See [`AccessState::AutoComplete`].
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct AutoComplete: u8 {

        /// Text suggesting one way to complete the provided input may be dynamically inserted after the caret.
        const INLINE = 0b01;

        /// When a user is providing input, a widget containing a collection of values that
        /// could complete the provided input may be displayed.
        const LIST = 0b10;

        /// An input to offer both models at the same time. When a user is providing input,
        /// a widget containing a collection of values that could complete the provided input
        /// may be displayed. If displayed, one value in the collection is automatically selected,
        /// and the text needed to complete the automatically selected value appears after the caret in the input.
        const BOTH = 0b11;
    }
}

bitflags! {
    /// Changes a live region makes.
    ///
    /// See [`AccessState::Live`].
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct LiveChange: u8 {
        /// Descendant nodes are added.
        const ADD = 0b001;
        /// Descendant nodes are removed.
        const REMOVE = 0b010;
        /// Text content changes.
        const TEXT = 0b100;
        /// All changes are possible.
        const ALL = 0b111;
    }
}

/// Identifies an accessibility widget node.
///
/// Note IDs are defined by the app-process, usually they are the `WidgetId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccessNodeId(pub u64);

/// Accessibility command.
///
/// The command must run in the context of the target widow and widget, see [`Event::AccessCommand`] for more details.
///
/// [`Event::AccessCommand`]: crate::Event::AccessCommand
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AccessCommand {
    /// Run the click action on the widget.
    ///
    /// If `true` run the primary (default) action, if `false` run the context action.
    Click(bool),

    /// Focus or escape focus on the widget.
    ///
    /// If `true` the widget is focused, if `false` and the widget is already focused does ESC.
    Focus(bool),

    /// Sets this widget as the starting point for the next TAB navigation.
    ///
    /// If the user presses TAB the focus will move to the next logical focusable after this widget,
    /// but this widget will not be focused by this request.
    SetNextTabStart,

    /// Expand or collapse the widget content.
    SetExpanded(bool),

    /// Increment by steps.
    ///
    /// Associated value is usually is -1 or 1.
    Increment(i8),

    /// Show or hide the widget's tooltip.
    SetToolTipVis(bool),

    /// Scroll command.
    Scroll(ScrollCommand),

    /// Insert the text.
    ReplaceSelectedText(String),

    /// Set the text selection.
    ///
    /// The two *points* are defined by the widget and string byte char index. The
    /// start can be before or after (textually). The byte index must be at the start of
    /// a grapheme and UTF-8 char.
    SelectText {
        /// Selection start.
        start: (AccessNodeId, usize),
        /// Selection end, where the caret is positioned.
        caret: (AccessNodeId, usize),
    },

    /// Replace the value of the control with the specified value and
    /// reset the selection, if applicable.
    SetString(String),

    /// Replace the value of the control with the specified value and
    /// reset the selection, if applicable.
    SetNumber(f64),
}
impl AccessCommand {
    /// Gets the command discriminant without associated data.
    pub fn name(&self) -> AccessCommandName {
        match self {
            AccessCommand::Click(_) => AccessCommandName::Click,
            AccessCommand::Focus(_) => AccessCommandName::Focus,
            AccessCommand::SetNextTabStart => AccessCommandName::SetNextTabStart,
            AccessCommand::SetExpanded(_) => AccessCommandName::SetExpanded,
            AccessCommand::Increment(_) => AccessCommandName::Increment,
            AccessCommand::SetToolTipVis(_) => AccessCommandName::SetToolTipVis,
            AccessCommand::Scroll(_) => AccessCommandName::Scroll,
            AccessCommand::ReplaceSelectedText(_) => AccessCommandName::ReplaceSelectedText,
            AccessCommand::SelectText { .. } => AccessCommandName::SelectText,
            AccessCommand::SetString(_) => AccessCommandName::SetString,
            AccessCommand::SetNumber(_) => AccessCommandName::SetNumber,
        }
    }
}

/// Accessibility command without associated data.
///
/// See [`AccessCommand::name`] for more details.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AccessCommandName {
    /// [`AccessCommand::Click`]
    Click,

    /// [`AccessCommand::Focus`]
    Focus,

    /// [`AccessCommand::SetNextTabStart`]
    SetNextTabStart,

    /// [`AccessCommand::SetExpanded`]
    SetExpanded,

    /// [`AccessCommand::Increment`]
    Increment,

    /// [`AccessCommand::SetToolTipVis`]
    SetToolTipVis,

    /// [`AccessCommand::Scroll`]
    Scroll,

    /// [`AccessCommand::ReplaceSelectedText`]
    ReplaceSelectedText,

    /// [`AccessCommand::SelectText`]
    SelectText,

    /// [`AccessCommand::SetString`]
    SetString,

    /// [`AccessCommand::SetNumber`]
    SetNumber,
}

/// Accessibility scroll command.
///
/// The command must run in the context of the target widow and widget, see [`AccessCommand::Scroll`] for more details.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScrollCommand {
    /// Scroll page up.
    ///
    /// If the scroll-box only scrolls horizontally this is the same as `ScrollLeft`.
    PageUp,
    /// Scroll page down.
    ///
    /// If the scroll-box only scrolls horizontally this is the same as `ScrollRight`.
    PageDown,
    /// Scroll page left.
    PageLeft,
    /// Scroll page right.
    PageRight,

    /// Scroll until the widget is fully visible.
    ScrollTo,
    /// Scroll until the rectangle (in the widget space) is fully visible.
    ScrollToRect(DipRect),

    /// Set the horizontal and vertical scroll offset.
    SetScrollOffset(DipVector),
}

/// Represents a widget in the access info tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessNode {
    /// Widget ID.
    pub id: AccessNodeId,
    /// Accessibility role.
    pub role: AccessRole,
    /// Commands the widget supports.
    pub commands: Vec<AccessCommandName>,
    /// Accessibility state.
    pub state: Vec<AccessState>,
    /// Number of children.
    ///
    /// See [`AccessTree::push`] for more details.
    pub children_count: u32,
    /// Number of descendants.
    ///
    /// See [`AccessTree::push`] for more details.
    pub descendants_count: u32,
}

/// Accessibility info tree for a window.
pub struct AccessTree(Vec<AccessNode>);
impl AccessTree {
    /// New tree with root node.
    pub fn new(root: AccessNode) -> Self {
        let mut s = Self(vec![]);
        s.push(root);
        s
    }

    /// Pushes a node on the tree.
    ///
    /// If `children_count` is not zero the children must be pushed immediately after, each child
    /// pushes their children immediately after too. A tree `(a(a.a, a.b, a.c), b)` pushes `[a, a.a, a.b, a.c, b]`.
    pub fn push(&mut self, node: AccessNode) {
        self.0.push(node);
    }

    /// Root node.
    pub fn root(&self) -> AccessNodeRef {
        AccessNodeRef { tree: self, index: 0 }
    }
}
impl ops::Deref for AccessTree {
    type Target = [AccessNode];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Reference an access node in a tree.
pub struct AccessNodeRef<'a> {
    tree: &'a AccessTree,
    index: usize,
}
impl<'a> AccessNodeRef<'a> {
    /// iterate over all descendant nodes.
    pub fn descendants(&self) -> impl ExactSizeIterator<Item = AccessNodeRef> {
        let range = self.index..(self.index + self.descendants_count as usize);
        let tree = self.tree;
        range.map(move |i| AccessNodeRef { tree, index: i })
    }

    /// Iterate over children nodes.
    pub fn children(&self) -> impl ExactSizeIterator<Item = AccessNodeRef> {
        struct ChildrenIter<'a> {
            tree: &'a AccessTree,
            count: usize,
            index: usize,
        }
        impl<'a> Iterator for ChildrenIter<'a> {
            type Item = AccessNodeRef<'a>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.count > 0 {
                    let item = AccessNodeRef {
                        tree: self.tree,
                        index: self.index,
                    };
                    self.count -= 1;

                    self.index += item.descendants_count as usize;

                    Some(item)
                } else {
                    None
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.count, Some(self.count))
            }
        }
        impl<'a> ExactSizeIterator for ChildrenIter<'a> {}
        ChildrenIter {
            tree: self.tree,
            count: self.children_count as usize,
            index: self.index + 1,
        }
    }
}
impl<'a> ops::Deref for AccessNodeRef<'a> {
    type Target = AccessNode;

    fn deref(&self) -> &Self::Target {
        &self.tree[self.index]
    }
}
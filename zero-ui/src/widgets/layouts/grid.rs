//! Grid widgets, properties and nodes..

use crate::prelude::new_widget::*;

/// Grid layout with cells of variable sizes.
#[widget($crate::widgets::layouts::Grid)]
pub struct Grid(WidgetBase);
impl Grid {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|w| {
            let child = node(
                w.capture_ui_node_list_or_empty(property_id!(Self::cells)),
                w.capture_ui_node_list_or_empty(property_id!(Self::columns)),
                w.capture_ui_node_list_or_empty(property_id!(Self::rows)),
                w.capture_var_or_else(property_id!(Self::auto_grow_fn), WidgetFn::nil),
                w.capture_var_or_else(property_id!(Self::auto_grow_mode), || AutoGrowMode::Rows(u32::MAX)),
                w.capture_var_or_default(property_id!(Self::spacing)),
            );
            w.set_child(child);
        });
    }
}

/// Cell widget items.
///
/// Cells can select their own column, row using the properties in the [`Cell!`] widget. Note that
/// you don't need to use the `cell!` widget, only the properties.
///
/// Cells can also be set to span multiple columns using the [`Cell!`] properties.
///
/// If the column or row is not explicitly set the widget is positioned in the logical index, the column
/// `i % columns` and the row  `i / columns`.
///
/// [`Cell!`]: struct@Cell
#[property(CHILD, capture, widget_impl(Grid))]
pub fn cells(child: impl UiNode, cells: impl UiNodeList) -> impl UiNode {}

/// Column definitions.
///
/// You can define columns with any widget, but the [`Column!`] widget is recommended. The column widget width defines
/// the width of the cells assigned to it, the [`Column::width`] property can be used to enforce a width, otherwise the
/// column is sized by the widest cell.
///
/// The grid uses the [`WIDGET_SIZE`] value to select one of three layout modes for columns:
///
/// * *Cell*, used for columns that do not set width or set it to [`Length::Default`].
/// * *Exact*, used for columns that set the width to a different unit.
/// * *Leftover*, used for columns that set width to an [`lft`] value.
///
/// The column layout follows these steps:
///
/// 1 - All *Exact* column widgets are layout, their final width defines the column width.
/// 2 - All cell widgets with span `1` in *Cell* columns are measured, the widest defines the fill width constrain,
/// the columns is layout using this constrain, the final width defines the column width.
/// 3 - All *Leftover* cells are layout with the leftover grid width divided among all columns in this mode.
///
/// So given the columns `200 | 1.lft() | 1.lft()` and grid width of `1000` with spacing `5` the final widths are `200 | 395 | 395`,
/// for `200 + 5 + 395 + 5 + 395 = 1000`.
///
/// Note that the column widget is not the parent of the cells that match it, the column widget is rendered under cell and row widgets.
/// Properties like `padding` and `align` only affect the column visual, not the cells, similarly contextual properties like `text_color`
/// don't affect the cells.
///
/// [`Column!`]: struct@Column
/// [`lft`]: LengthUnits::lft
#[property(CHILD, capture, widget_impl(Grid))]
pub fn columns(child: impl UiNode, cells: impl UiNodeList) -> impl UiNode {}

/// Row definitions.
///
/// Same behavior as [`columns`], but in the ***y*** dimension.
///
/// [`columns`]: fn@columns
#[property(CHILD, capture, widget_impl(Grid))]
pub fn rows(child: impl UiNode, cells: impl UiNodeList) -> impl UiNode {}

/// Widget function used when new rows or columns are needed to cover a cell placement.
///
/// The function is used according to the [`auto_grow_mode`]. Note that *imaginary* rows or columns are used if
/// the function is [ `WidgetFn::nil` ].
///
/// [`auto_grow_mode`]: fn@auto_grow_mode
#[property(CONTEXT, capture, widget_impl(Grid))]
pub fn auto_grow_fn(child: impl UiNode, auto_grow: impl IntoVar<WidgetFn<AutoGrowFnArgs>>) -> impl UiNode {}

/// Maximum inclusive index that can be covered by auto-generated columns or rows. If a cell is outside this index and
/// is not covered by predefined columns or rows a new one is auto generated for it, but if the cell is also outside this
/// max it is *collapsed*.
///
/// Is `AutoGrowMode::Rows(u32::MAX)` by default.
#[property(CONTEXT, capture, widget_impl(Grid))]
pub fn auto_grow_mode(child: impl UiNode, mode: impl IntoVar<AutoGrowMode>) -> impl UiNode {}

/// Space in-between cells.
#[property(LAYOUT, capture, widget_impl(Grid))]
pub fn spacing(child: impl UiNode, spacing: impl IntoVar<GridSpacing>) -> impl UiNode {}

/// Grid node.
///
/// Can be used directly to layout widgets without declaring a grid widget info. This node is the child
/// of the `Grid!` widget.
pub fn node(
    cells: impl UiNodeList,
    columns: impl UiNodeList,
    rows: impl UiNodeList,
    auto_grow_fn: impl IntoVar<WidgetFn<AutoGrowFnArgs>>,
    auto_grow_mode: impl IntoVar<AutoGrowMode>,
    spacing: impl IntoVar<GridSpacing>,
) -> impl UiNode {
    let auto_columns: Vec<BoxedUiNode> = vec![];
    let auto_rows: Vec<BoxedUiNode> = vec![];
    GridNode {
        children: vec![
            vec![columns.boxed(), auto_columns.boxed()].boxed(),
            vec![rows.boxed(), auto_rows.boxed()].boxed(),
            PanelList::new(cells).boxed(),
        ],
        spacing: spacing.into_var(),
        auto_grow_fn: auto_grow_fn.into_var(),
        auto_grow_mode: auto_grow_mode.into_var(),

        info: Default::default(),
    }
}

pub use column::Column;

/// Column widget and properties.
pub mod column {
    use super::*;

    /// Grid column definition.
    ///
    /// This widget is layout to define the actual column width, it is not the parent
    /// of the cells, only the `width` and `align` properties affect the cells.
    ///
    /// See the [`Grid::columns`] property for more details.
    ///
    /// # Shorthand
    ///
    /// The `Column!` macro provides a shorthand init that sets the width, `grid::Column!(1.lft())` instantiates
    /// a column with width of *1 leftover*.
    #[widget($crate::widgets::layouts::grid::Column {
        ($width:expr) => {
            width = $width;
        };
    })]
    pub struct Column(WidgetBase);
    impl Column {
        widget_impl! {
            /// Column max width.
            pub crate::properties::max_width(max: impl IntoVar<Length>);

            /// Column min width.
            pub crate::properties::min_width(min: impl IntoVar<Length>);

            /// Column width.
            pub crate::properties::width(width: impl IntoVar<Length>);
        }
    }

    /// Column index, total in the parent widget set by the parent.
    pub(super) static INDEX_ID: StaticStateId<(usize, usize)> = StaticStateId::new_unique();

    /// If the column index is even.
    ///
    /// Column index is zero-based, so the first column is even, the next [`is_odd`].
    ///
    /// [`is_odd`]: fn@is_odd
    #[property(CONTEXT, widget_impl(Column))]
    pub fn is_even(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        widget_state_is_state(child, |w| w.get(&INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 == 0, |_| false, state)
    }

    /// If the column index is odd.
    ///
    /// Column index is zero-based, so the first column [`is_even`], the next one is odd.
    ///
    /// [`is_even`]: fn@is_even
    #[property(CONTEXT, widget_impl(Column))]
    pub fn is_odd(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        widget_state_is_state(child, |w| w.get(&INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 != 0, |_| false, state)
    }

    /// If the column is the first.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn is_first(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        widget_state_is_state(
            child,
            |w| {
                let (i, l) = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                i == 0 && l > 0
            },
            |_| false,
            state,
        )
    }

    /// If the column is the last.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn is_last(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        widget_state_is_state(
            child,
            |w| {
                let (i, l) = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                i < l && i == l - 1
            },
            |_| false,
            state,
        )
    }

    /// Get the column index for custom `when` expressions.
    ///
    /// The column index is zero-based.
    ///
    /// # Examples
    ///
    /// This uses `get_index` to give every third column a different background.
    ///
    /// ```
    /// # use zero_ui::{widgets::layouts::grid, properties::background_color, core::color::colors};
    /// # let _scope = zero_ui::core::app::App::minimal();
    /// # let _ =
    /// grid::Column! {
    ///     background_color = colors::GRAY;    
    ///
    ///     when *#get_index % 3 == 0 {
    ///         background_color = colors::DARK_GRAY;
    ///     }
    /// }
    /// # ;
    /// ```
    #[property(CONTEXT, widget_impl(Column))]
    pub fn get_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(&INDEX_ID).copied().unwrap_or((0, 0)).0;
                if a != i {
                    Some(a)
                } else {
                    None
                }
            },
            |_, &i| if i != 0 { Some(0) } else { None },
            state,
        )
    }

    /// Get the column index and number of columns.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn get_index_len(child: impl UiNode, state: impl IntoVar<(usize, usize)>) -> impl UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                if a != i {
                    Some(a)
                } else {
                    None
                }
            },
            |_, &i| if i != (0, 0) { Some((0, 0)) } else { None },
            state,
        )
    }

    /// Get the column index, starting from the last column at `0`.
    #[property(CONTEXT, widget_impl(Column))]
    pub fn get_rev_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                let a = a.1 - a.0;
                if a != i {
                    Some(a)
                } else {
                    None
                }
            },
            |_, &i| if i != 0 { Some(0) } else { None },
            state,
        )
    }

    /// Get the column index as a factor of the total number of columns.
    #[property(CONTEXT, default(var(0.fct())), widget_impl(Column))]
    pub fn get_index_fct(child: impl UiNode, state: impl IntoVar<Factor>) -> impl UiNode {
        widget_state_get_state(
            child,
            |w, &f| {
                let a = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                let a = a.1 as f32 / a.0 as f32;
                let a = Factor(a);
                if a != f {
                    Some(a)
                } else {
                    None
                }
            },
            |_, &f| if f != 0.fct() { Some(0.fct()) } else { None },
            state,
        )
    }
}

pub use row::Row;

/// Row widget and properties.
pub mod row {
    use super::*;

    /// Grid row definition.
    ///
    /// This widget is layout to define the actual row height, it is not the parent
    /// of the cells, only the `height` property affect the cells.
    ///
    /// See the [`Grid::rows`] property for more details.
    ///
    /// # Shorthand
    ///
    /// The `Row!` macro provides a shorthand init that sets the height, `grid::Row!(1.lft())` instantiates
    /// a row with height of *1 leftover*.
    #[widget($crate::widgets::layouts::grid::Row {
        ($height:expr) => {
            height = $height;
        };
    })]
    pub struct Row(WidgetBase);
    impl Row {
        widget_impl! {
            /// Row max height.
            pub crate::properties::max_height(max: impl IntoVar<Length>);

            /// Row min height.
            pub crate::properties::min_height(max: impl IntoVar<Length>);

            /// Row height.
            pub crate::properties::height(max: impl IntoVar<Length>);
        }
    }

    /// Row index, total in the parent widget set by the parent.
    pub(super) static INDEX_ID: StaticStateId<(usize, usize)> = StaticStateId::new_unique();

    /// If the row index is even.
    ///
    /// Row index is zero-based, so the first row is even, the next [`is_odd`].
    ///
    /// [`is_odd`]: fn@is_odd
    #[property(CONTEXT, widget_impl(Row))]
    pub fn is_even(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        widget_state_is_state(child, |w| w.get(&INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 == 0, |_| false, state)
    }

    /// If the row index is odd.
    ///
    /// Row index is zero-based, so the first row [`is_even`], the next one is odd.
    ///
    /// [`is_even`]: fn@is_even
    #[property(CONTEXT, widget_impl(Row))]
    pub fn is_odd(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        widget_state_is_state(child, |w| w.get(&INDEX_ID).copied().unwrap_or((0, 0)).0 % 2 != 0, |_| false, state)
    }

    /// If the row is the first.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn is_first(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        widget_state_is_state(
            child,
            |w| {
                let (i, l) = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                i == 0 && l > 0
            },
            |_| false,
            state,
        )
    }

    /// If the row is the last.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn is_last(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        widget_state_is_state(
            child,
            |w| {
                let (i, l) = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                i < l && i == l - 1
            },
            |_| false,
            state,
        )
    }

    /// Get the row index for custom `when` expressions.
    ///
    /// The row index is zero-based.
    ///
    /// # Examples
    ///
    /// This uses `get_index` to give every third row a different background.
    ///
    /// ```
    /// # use zero_ui::{widgets::layouts::grid, properties::background_color, core::color::colors};
    /// # let _scope = zero_ui::core::app::App::minimal();
    /// # let _ =
    /// grid::Row! {
    ///     background_color = colors::GRAY;    
    ///
    ///     when *#get_index % 3 == 0 {
    ///         background_color = colors::DARK_GRAY;
    ///     }
    /// }
    /// # ;
    /// ```
    #[property(CONTEXT, widget_impl(Row))]
    pub fn get_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(&INDEX_ID).copied().unwrap_or((0, 0)).0;
                if a != i {
                    Some(a)
                } else {
                    None
                }
            },
            |_, &i| if i != 0 { Some(0) } else { None },
            state,
        )
    }

    /// Get the column index and number of columns.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn get_index_len(child: impl UiNode, state: impl IntoVar<(usize, usize)>) -> impl UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                if a != i {
                    Some(a)
                } else {
                    None
                }
            },
            |_, &i| if i != (0, 0) { Some((0, 0)) } else { None },
            state,
        )
    }

    /// Get the row index, starting from the last row at `0`.
    #[property(CONTEXT, widget_impl(Row))]
    pub fn get_rev_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
        widget_state_get_state(
            child,
            |w, &i| {
                let a = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                let a = a.1 - a.0;
                if a != i {
                    Some(a)
                } else {
                    None
                }
            },
            |_, &i| if i != 0 { Some(0) } else { None },
            state,
        )
    }

    /// Get the row index as a factor of the total number of rows.
    #[property(CONTEXT, default(0.fct()), widget_impl(Row))]
    pub fn get_index_fct(child: impl UiNode, state: impl IntoVar<Factor>) -> impl UiNode {
        widget_state_get_state(
            child,
            |w, &f| {
                let a = w.get(&INDEX_ID).copied().unwrap_or((0, 0));
                let a = a.1 as f32 / a.0 as f32;
                let a = Factor(a);
                if a != f {
                    Some(a)
                } else {
                    None
                }
            },
            |_, &f| if f != 0.fct() { Some(0.fct()) } else { None },
            state,
        )
    }
}

pub use cell::Cell;

/// Cell widget and properties.
pub mod cell {
    use super::*;

    /// Grid cell container.
    ///
    /// This widget defines properties that position and size widgets in a [`Grid!`].
    ///
    /// See the [`Grid::cells`] property for more details.
    #[widget($crate::widgets::layouts::grid::Cell)]
    pub struct Cell(Container);

    /// Represents values set by cell properties in a widget.
    #[derive(Clone, Copy, Debug)]
    pub struct CellInfo {
        /// The [`column`] value.
        ///
        /// [`column`]: fn@column
        pub column: usize,

        /// The [`column_span`] value.
        ///
        /// [`column_span`]: fn@column_span
        pub column_span: usize,

        /// The [`row`] value.
        ///
        /// [`row`]: fn@row
        pub row: usize,

        /// The [`row_span`] value.
        ///
        /// [`row_span`]: fn@row_span
        pub row_span: usize,
    }
    impl Default for CellInfo {
        fn default() -> Self {
            Self {
                column: usize::MAX,
                column_span: 1,
                row: usize::MAX,
                row_span: 1,
            }
        }
    }
    impl CellInfo {
        /// Compute or correct the column and row of the cell.
        ///
        /// The `logical_index` is the index of the cell widget in the cell node list.
        pub fn actual(mut self, logical_index: usize, columns_len: usize) -> Self {
            if self.column == usize::MAX {
                self.column = logical_index % columns_len;
            } else {
                self.column = self.column.min(columns_len - 1);
            }
            if self.row == usize::MAX {
                self.row = logical_index / columns_len
            }
            self
        }

        /// Get the cell info stored in the [`WIDGET`] state.
        pub fn get() -> Self {
            WIDGET.get_state(&INFO_ID).unwrap_or_default()
        }

        /// Get the cell info stored in the `wgt` state.
        pub fn get_wgt(wgt: &impl UiNode) -> Self {
            wgt.with_context(Self::get).unwrap_or_default()
        }
    }

    /// Id for widget state set by cell properties.
    ///
    /// The parent grid uses this info to position and size the cell widget.
    pub static INFO_ID: StaticStateId<CellInfo> = StaticStateId::new_unique();

    /// Cell column index.
    ///
    /// If not set or set to [`usize::MAX`] the cell is positioned based on the logical index.
    ///
    /// This property sets the [`INFO_ID`].
    #[property(CONTEXT, default(usize::MAX), widget_impl(Cell))]
    pub fn column(child: impl UiNode, col: impl IntoVar<usize>) -> impl UiNode {
        with_widget_state_modify(child, &INFO_ID, col, CellInfo::default, |i, &c| {
            if i.column != c {
                i.column = c;
                WIDGET.layout();
            }
        })
    }

    /// Cell row index.
    ///
    /// If not set or out-of-bounds the cell is positioned based on the logical index.
    ///
    /// This property sets the [`INFO_ID`].
    #[property(CONTEXT, default(usize::MAX), widget_impl(Cell))]
    pub fn row(child: impl UiNode, row: impl IntoVar<usize>) -> impl UiNode {
        with_widget_state_modify(child, &INFO_ID, row, CellInfo::default, |i, &r| {
            if i.row != r {
                i.row = r;
                WIDGET.layout();
            }
        })
    }

    /// Cell column span.
    ///
    /// Number of *cells* this one spans over horizontally, starting from the column index and spanning to the right.
    ///
    /// Is `1` by default, the index is clamped between `1..max` where max is the maximum number of valid columns
    /// to the right of the cell column index.
    ///
    /// Note that the cell does not influence the column width if it spans over multiple columns.
    ///
    /// This property sets the [`INFO_ID`].
    #[property(CONTEXT, default(1), widget_impl(Cell))]
    pub fn column_span(child: impl UiNode, span: impl IntoVar<usize>) -> impl UiNode {
        with_widget_state_modify(child, &INFO_ID, span, CellInfo::default, |i, &s| {
            if i.column_span != s {
                i.column_span = s;
                WIDGET.layout();
            }
        })
    }

    /// Cell row span.
    ///
    /// Number of *cells* this one spans over vertically, starting from the row index and spanning down.
    ///
    /// Is `1` by default, the index is clamped between `1..max` where max is the maximum number of valid rows
    /// down from the cell column index.
    ///
    /// Note that the cell does not influence the row height if it spans over multiple rows.
    ///
    /// This property sets the [`INFO_ID`].
    #[property(CONTEXT, default(1), widget_impl(Cell))]
    pub fn row_span(child: impl UiNode, span: impl IntoVar<usize>) -> impl UiNode {
        with_widget_state_modify(child, &INFO_ID, span, CellInfo::default, |i, &s| {
            if i.row_span != s {
                i.row_span = s;
                WIDGET.layout();
            }
        })
    }
}

#[derive(Clone, Copy)]
struct ColRowMeta(f32);
impl ColRowMeta {
    /// `width` or `height` contains the largest cell or `Px::MIN` if cell measure is pending.
    fn is_default(self) -> bool {
        self.0.is_sign_negative() && self.0.is_infinite()
    }

    /// Return the leftover factor if the column or row must be measured on a fraction of the leftover space.
    fn is_leftover(self) -> Option<Factor> {
        if self.0 >= 0.0 {
            Some(Factor(self.0))
        } else {
            None
        }
    }

    /// `width` or `height` contains the final length or is pending layout `Px::MIN`.
    fn is_exact(self) -> bool {
        self.0.is_nan()
    }

    fn exact() -> Self {
        Self(f32::NAN)
    }

    fn leftover(f: Factor) -> Self {
        Self(f.0.max(0.0))
    }
}
impl Default for ColRowMeta {
    fn default() -> Self {
        Self(f32::NEG_INFINITY)
    }
}

#[derive(Clone, Copy)]
struct ColumnInfo {
    meta: ColRowMeta,
    x: Px,
    width: Px,
}
impl Default for ColumnInfo {
    fn default() -> Self {
        Self {
            meta: ColRowMeta::default(),
            x: Px::MIN,
            width: Px::MIN,
        }
    }
}
#[derive(Clone, Copy)]
struct RowInfo {
    meta: ColRowMeta,
    y: Px,
    height: Px,
}
impl Default for RowInfo {
    fn default() -> Self {
        Self {
            meta: ColRowMeta::default(),
            y: Px::MIN,
            height: Px::MIN,
        }
    }
}

#[derive(Default)]
struct GridInfo {
    columns: Vec<ColumnInfo>,
    rows: Vec<RowInfo>,
}
impl GridInfo {
    fn is_collapse(&self) -> bool {
        self.columns.is_empty() && self.rows.is_empty()
    }
}

fn downcast_auto(cols_or_rows: &mut BoxedUiNodeList) -> &mut Vec<BoxedUiNode> {
    cols_or_rows.as_any().downcast_mut::<Vec<BoxedUiNodeList>>().unwrap()[1]
        .as_any()
        .downcast_mut()
        .unwrap()
}

#[ui_node(struct GridNode {
    // [[columns, auto_columns], [rows, auto_rows], cells]
    children: Vec<BoxedUiNodeList>,
    #[var] auto_grow_fn: impl Var<WidgetFn<AutoGrowFnArgs>>,
    #[var] auto_grow_mode: impl Var<AutoGrowMode>,
    #[var] spacing: impl Var<GridSpacing>,

    info: GridInfo,
})]
impl GridNode {
    fn collapse_info(&mut self) {
        self.info.columns.clear();
        self.info.rows.clear();
    }

    // add/remove info entries, auto-grow/shrink
    fn update_info(&mut self) {
        let auto_mode = self.auto_grow_mode.get();

        // max needed column or row in the auto_mode axis.
        let mut max_custom = 0;
        let mut max_auto_placed_i = 0;
        self.children[2].for_each(|i, c| {
            let info = c.with_context(cell::CellInfo::get).unwrap_or_default();

            let n = match auto_mode {
                AutoGrowMode::Rows(_) => info.row,
                AutoGrowMode::Columns(_) => info.column,
            };
            if n == usize::MAX {
                max_auto_placed_i = i;
            } else {
                max_custom = max_custom.max(n);
            }
        });

        let mut imaginary_cols = 0;
        let mut imaginary_rows = 0;

        match auto_mode {
            AutoGrowMode::Rows(max) => {
                let columns_len = self.children[0].len();
                if columns_len == 0 {
                    self.collapse_info();
                    return;
                }

                let max_auto_placed = max_auto_placed_i / columns_len + 1;
                let max_needed = max_auto_placed.max(max_custom).min(max as usize);

                let rows_len = self.children[1].len();

                #[allow(clippy::comparison_chain)]
                if rows_len < max_needed {
                    let auto = downcast_auto(&mut self.children[1]);
                    let mut index = rows_len;

                    let view = self.auto_grow_fn.get();
                    if view.is_nil() {
                        imaginary_rows = max_needed - rows_len;
                    } else {
                        while index < max_needed {
                            let mut row = view(AutoGrowFnArgs { mode: auto_mode, index });
                            row.init();
                            auto.push(row);
                            index += 1;
                        }
                    }
                } else if rows_len > max_needed {
                    let remove = rows_len - max_needed;
                    let auto = downcast_auto(&mut self.children[1]);
                    for mut auto in auto.drain(auto.len().saturating_sub(remove)..) {
                        auto.deinit();
                    }
                }
            }
            AutoGrowMode::Columns(max) => {
                let rows_len = self.children[1].len();
                if rows_len == 0 {
                    self.collapse_info();
                    return;
                }

                let max_auto_placed = max_auto_placed_i / rows_len + 1;
                let max_needed = max_auto_placed.max(max_custom).min(max as usize);

                let cols_len = self.children[0].len();

                #[allow(clippy::comparison_chain)]
                if cols_len < max_needed {
                    let auto = downcast_auto(&mut self.children[0]);
                    let mut index = cols_len;

                    let view = self.auto_grow_fn.get();
                    if view.is_nil() {
                        imaginary_cols = max_needed - cols_len;
                    } else {
                        while index < max_needed {
                            let mut column = view(AutoGrowFnArgs { mode: auto_mode, index });
                            column.init();
                            auto.push(column);
                            index += 1;
                        }
                    }
                } else if cols_len > max_needed {
                    let remove = cols_len - max_needed;
                    let auto = downcast_auto(&mut self.children[0]);
                    for mut auto in auto.drain(auto.len().saturating_sub(remove)..) {
                        auto.deinit();
                    }
                }
            }
        }

        // Set index for column and row.
        let columns_len = self.children[0].len() + imaginary_cols;
        self.children[0].for_each(|i, c| {
            c.with_context(|| {
                let prev = WIDGET.set_state(&column::INDEX_ID, (i, columns_len));
                if prev != Some((i, columns_len)) {
                    WIDGET.update();
                }
            });
        });
        let rows_len = self.children[1].len() + imaginary_rows;
        self.children[1].for_each(|i, r| {
            r.with_context(|| {
                let prev = WIDGET.set_state(&row::INDEX_ID, (i, rows_len));
                if prev != Some((i, rows_len)) {
                    WIDGET.update();
                }
            });
        });

        self.info.columns.resize(columns_len, ColumnInfo::default());
        self.info.rows.resize(rows_len, RowInfo::default());
    }

    #[UiNode]
    fn init(&mut self) {
        self.auto_subs();
        self.children.init_all();
        self.update_info();
    }

    #[UiNode]
    fn deinit(&mut self) {
        self.children.deinit_all();
        downcast_auto(&mut self.children[0]).clear();
        downcast_auto(&mut self.children[1]).clear();
    }

    #[UiNode]
    fn update(&mut self, updates: &WidgetUpdates) {
        if self.spacing.is_new() {
            WIDGET.layout();
        }

        let mut any = false;
        self.children.update_all(updates, &mut any);

        if self.auto_grow_fn.is_new() || self.auto_grow_mode.is_new() {
            for mut auto in downcast_auto(&mut self.children[0]).drain(..) {
                auto.deinit();
            }
            for mut auto in downcast_auto(&mut self.children[1]).drain(..) {
                auto.deinit();
            }
            any = true;
        }
        if any {
            self.update_info();
            WIDGET.layout();
        }
    }

    fn layout_info(&mut self, wm: &mut WidgetMeasure) -> (PxGridSpacing, PxSize) {
        let info = &mut self.info;
        if info.is_collapse() {
            return (PxGridSpacing::zero(), PxSize::zero());
        }

        let spacing = self.spacing.layout();
        let constraints = LAYOUT.constraints();

        let fill_x = constraints.x.fill_or_exact();
        let fill_y = constraints.y.fill_or_exact();

        let mut children = self.children.iter_mut();
        let columns = children.next().unwrap();
        let rows = children.next().unwrap();
        let cells = children.next().unwrap();

        // layout exact columns&rows, mark others for next passes.

        let mut has_default = false;
        let mut has_leftover_cols = false;
        let mut has_leftover_rows = false;

        columns.for_each(|ci, col| {
            let col_kind = WIDGET_SIZE.get_wgt(col).width;

            let col_info = &mut info.columns[ci];

            col_info.x = Px::MIN;
            col_info.width = Px::MIN;

            match col_kind {
                WidgetLength::Default => {
                    col_info.meta = ColRowMeta::default();
                    has_default = true;
                }
                WidgetLength::Leftover(f) => {
                    col_info.meta = ColRowMeta::leftover(f);
                    has_leftover_cols = true;
                }
                WidgetLength::Exact => {
                    col_info.width = col.measure(wm).width;
                    col_info.meta = ColRowMeta::exact();
                }
            }
        });
        rows.for_each(|ri, row| {
            let row_kind = WIDGET_SIZE.get_wgt(row).height;

            let row_info = &mut info.rows[ri];

            row_info.y = Px::MIN;
            row_info.height = Px::MIN;

            match row_kind {
                WidgetLength::Default => {
                    row_info.meta = ColRowMeta::default();
                    has_default = true;
                }
                WidgetLength::Leftover(f) => {
                    row_info.meta = ColRowMeta::leftover(f);
                    has_leftover_rows = true;
                }
                WidgetLength::Exact => {
                    row_info.height = row.measure(wm).height;
                    row_info.meta = ColRowMeta::exact();
                }
            }
        });

        // Measure cells when needed, collect widest/tallest.
        //  - For `Default` columns&rows to get their size.
        //  - For `leftover` columns&rows when the grid with no fill or exact size, to get the `1.lft()` length.
        let columns_len = info.columns.len();
        if has_default || (fill_x.is_none() && has_leftover_cols) || (fill_y.is_none() && has_leftover_rows) {
            let c = LAYOUT.constraints();
            cells.for_each(|i, cell| {
                let cell_info = cell::CellInfo::get_wgt(cell);
                if cell_info.column_span > 1 || cell_info.row_span > 1 {
                    return; // continue;
                }
                let cell_info = cell_info.actual(i, columns_len);

                let col = &mut info.columns[cell_info.column];
                let row = &mut info.rows[cell_info.row];

                let col_is_default = col.meta.is_default() || (fill_x.is_none() && col.meta.is_leftover().is_some());
                let col_is_exact = !col_is_default && col.meta.is_exact();
                let col_is_leftover = !col_is_default && col.meta.is_leftover().is_some();

                let row_is_default = row.meta.is_default() || (fill_y.is_none() && row.meta.is_leftover().is_some());
                let row_is_exact = !row_is_default && row.meta.is_exact();
                let row_is_leftover = !row_is_default && row.meta.is_leftover().is_some();

                if col_is_default {
                    if row_is_default {
                        // (default, default)
                        let size = LAYOUT.with_constraints(c.with_fill(false, false), || cell.measure(wm));

                        col.width = col.width.max(size.width);
                        row.height = row.height.max(size.height);
                    } else if row_is_exact {
                        // (default, exact)
                        let size = LAYOUT.with_constraints(c.with_exact_y(row.height).with_fill(false, false), || cell.measure(wm));

                        col.width = col.width.max(size.width);
                    } else {
                        debug_assert!(row_is_leftover);
                        // (default, leftover)
                        let size = LAYOUT.with_constraints(c.with_fill(false, false), || cell.measure(wm));

                        col.width = col.width.max(size.width);
                    }
                } else if col_is_exact {
                    if row_is_default {
                        // (exact, default)
                        let size = LAYOUT.with_constraints(c.with_exact_x(col.width).with_fill(false, false), || cell.measure(wm));

                        row.height = row.height.max(size.height);
                    }
                } else if row_is_default {
                    debug_assert!(col_is_leftover);
                    // (leftover, default)
                    let size = LAYOUT.with_constraints(c.with_fill(false, false), || cell.measure(wm));

                    row.height = row.height.max(size.height);
                }
            });
        }

        // distribute leftover grid space to columns
        if has_leftover_cols {
            let mut no_fill_1_lft = Px(0);
            let mut used_width = Px(0);
            let mut total_factor = Factor(0.0);
            let mut leftover_count = 0;
            let mut max_factor = 0.0_f32;

            for col in &mut info.columns {
                if let Some(f) = col.meta.is_leftover() {
                    if fill_x.is_none() {
                        no_fill_1_lft = no_fill_1_lft.max(col.width);
                        col.width = Px::MIN;
                    }
                    max_factor = max_factor.max(f.0);
                    total_factor += f;
                    leftover_count += 1;
                } else if col.width > Px(0) {
                    used_width += col.width;
                }
            }

            // handle big leftover factors
            if total_factor.0.is_infinite() {
                total_factor = Factor(0.0);

                if max_factor.is_infinite() {
                    // +inf takes all space
                    for col in &mut info.columns {
                        if let Some(f) = col.meta.is_leftover() {
                            if f.0.is_infinite() {
                                col.meta = ColRowMeta::leftover(Factor(1.0));
                                total_factor.0 += 1.0;
                            } else {
                                col.meta = ColRowMeta::leftover(Factor(0.0));
                            }
                        }
                    }
                } else {
                    // scale down every factor to fit
                    let scale = f32::MAX / max_factor / leftover_count as f32;
                    for col in &mut info.columns {
                        if let Some(f) = col.meta.is_leftover() {
                            let f = Factor(f.0 * scale);
                            col.meta = ColRowMeta::leftover(f);
                            total_factor += f;
                        }
                    }
                }
            }

            // individual factors under `1.0` behave like `Length::Relative`.
            if total_factor < Factor(1.0) {
                total_factor = Factor(1.0);
            }

            let mut leftover_width = if let Some(w) = fill_x {
                let vis_columns = info.columns.iter().filter(|c| c.width != Px(0)).count() as i32;
                w - used_width - spacing.column * Px(vis_columns - 1).max(Px(0))
            } else {
                // grid has no width, so `1.lft()` is defined by the widest cell measured using `Default` constraints.
                let mut unbounded_width = used_width;
                for col in &info.columns {
                    if let Some(f) = col.meta.is_leftover() {
                        unbounded_width += no_fill_1_lft * f;
                    }
                }
                let bounded_width = constraints.x.clamp(unbounded_width);
                bounded_width - used_width
            };
            leftover_width = leftover_width.max(Px(0));

            let view_columns_len = columns.len();

            // find extra leftover space from columns that can't fully fill their requested leftover length.
            let mut settled_all = false;
            while !settled_all && leftover_width > Px(0) {
                settled_all = true;

                for (i, col) in info.columns.iter_mut().enumerate() {
                    let lft = if let Some(lft) = col.meta.is_leftover() {
                        lft
                    } else {
                        continue;
                    };

                    let width = lft.0 * leftover_width.0 as f32 / total_factor.0;
                    col.width = Px(width as i32);

                    if i < view_columns_len {
                        let size = LAYOUT.with_constraints(LAYOUT.constraints().with_fill_x(true).with_max_x(col.width), || {
                            columns.with_node(i, |col| col.measure(wm))
                        });

                        if col.width != size.width {
                            // reached a max/min, convert this column to "exact" and remove it from
                            // the leftover pool.
                            settled_all = false;

                            col.width = size.width;
                            col.meta = ColRowMeta::exact();

                            if size.width != Px(0) {
                                leftover_width -= size.width + spacing.column;
                                total_factor -= lft;
                                if total_factor < Factor(1.0) {
                                    total_factor = Factor(1.0);
                                }
                            }
                        }
                    }
                }
            }

            leftover_width = leftover_width.max(Px(0));

            // finish settled leftover columns that can fill the requested leftover length.
            for col in &mut info.columns {
                let lft = if let Some(lft) = col.meta.is_leftover() {
                    lft
                } else {
                    continue;
                };

                let width = lft.0 * leftover_width.0 as f32 / total_factor.0;
                col.width = Px(width as i32);
                col.meta = ColRowMeta::exact();
            }
        }
        // distribute leftover grid space to rows
        if has_leftover_rows {
            let mut no_fill_1_lft = Px(0);
            let mut used_height = Px(0);
            let mut total_factor = Factor(0.0);
            let mut leftover_count = 0;
            let mut max_factor = 0.0_f32;

            for row in &mut info.rows {
                if let Some(f) = row.meta.is_leftover() {
                    if fill_y.is_none() {
                        no_fill_1_lft = no_fill_1_lft.max(row.height);
                        row.height = Px::MIN;
                    }
                    max_factor = max_factor.max(f.0);
                    total_factor += f;
                    leftover_count += 1;
                } else if row.height > Px(0) {
                    used_height += row.height;
                }
            }

            // handle big leftover factors
            if total_factor.0.is_infinite() {
                total_factor = Factor(0.0);

                if max_factor.is_infinite() {
                    // +inf takes all space
                    for row in &mut info.rows {
                        if let Some(f) = row.meta.is_leftover() {
                            if f.0.is_infinite() {
                                row.meta = ColRowMeta::leftover(Factor(1.0));
                                total_factor.0 += 1.0;
                            } else {
                                row.meta = ColRowMeta::leftover(Factor(0.0));
                            }
                        }
                    }
                } else {
                    // scale down every factor to fit
                    let scale = f32::MAX / max_factor / leftover_count as f32;
                    for row in &mut info.rows {
                        if let Some(f) = row.meta.is_leftover() {
                            let f = Factor(f.0 * scale);
                            row.meta = ColRowMeta::leftover(f);
                            total_factor += f;
                        }
                    }
                }
            }

            // individual factors under `1.0` behave like `Length::Relative`.
            if total_factor < Factor(1.0) {
                total_factor = Factor(1.0);
            }

            let mut leftover_height = if let Some(h) = fill_y {
                let vis_rows = info.rows.iter().filter(|c| c.height != Px(0)).count() as i32;
                h - used_height - spacing.row * Px(vis_rows - 1).max(Px(0))
            } else {
                // grid has no height, so `1.lft()` is defined by the tallest cell measured using `Default` constraints.
                let mut unbounded_height = used_height;
                for row in &info.rows {
                    if let Some(f) = row.meta.is_leftover() {
                        unbounded_height += no_fill_1_lft * f;
                    }
                }
                let bounded_height = constraints.x.clamp(unbounded_height);
                bounded_height - used_height
            };
            leftover_height = leftover_height.max(Px(0));

            let view_rows_len = rows.len();

            // find extra leftover space from leftover that can't fully fill their requested leftover length.
            let mut settled_all = false;
            while !settled_all && leftover_height > Px(0) {
                settled_all = true;

                for (i, row) in info.rows.iter_mut().enumerate() {
                    let lft = if let Some(lft) = row.meta.is_leftover() {
                        lft
                    } else {
                        continue;
                    };

                    let height = lft.0 * leftover_height.0 as f32 / total_factor.0;
                    row.height = Px(height as i32);

                    if i < view_rows_len {
                        let size = LAYOUT.with_constraints(LAYOUT.constraints().with_fill_y(true).with_max_y(row.height), || {
                            rows.with_node(i, |row| row.measure(wm))
                        });

                        if row.height != size.height {
                            // reached a max/min, convert this row to "exact" and remove it from
                            // the leftover pool.
                            settled_all = false;

                            row.height = size.height;
                            row.meta = ColRowMeta::exact();

                            if size.height != Px(0) {
                                leftover_height -= size.height + spacing.row;
                                total_factor -= lft;
                                if total_factor < Factor(1.0) {
                                    total_factor = Factor(1.0);
                                }
                            }
                        }
                    }
                }
            }

            leftover_height = leftover_height.max(Px(0));

            // finish settled leftover rows that can fill the requested leftover length.
            for row in &mut info.rows {
                let lft = if let Some(lft) = row.meta.is_leftover() {
                    lft
                } else {
                    continue;
                };

                let height = lft.0 * leftover_height.0 as f32 / total_factor.0;
                row.height = Px(height as i32);
                row.meta = ColRowMeta::exact();
            }
        }

        // compute column&row offsets
        let mut x = Px(0);
        for col in &mut info.columns {
            col.x = x;
            if col.width != Px(0) {
                x += col.width + spacing.column;
            }
        }
        let mut y = Px(0);
        for row in &mut info.rows {
            row.y = y;
            if row.height != Px(0) {
                y += row.height + spacing.row;
            }
        }

        (spacing, PxSize::new((x - spacing.column).max(Px(0)), (y - spacing.row).max(Px(0))))
    }

    #[UiNode]
    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        if let Some(size) = LAYOUT.constraints().fill_or_exact() {
            size
        } else {
            self.layout_info(wm).1
        }
    }

    #[UiNode]
    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let (spacing, grid_size) = self.layout_info(&mut WidgetMeasure::new());
        let c = LAYOUT.constraints();

        let info = &mut self.info;
        if info.is_collapse() {
            wl.collapse_descendants();
            return LAYOUT.constraints().fill_or_exact().unwrap_or_default();
        }

        let mut children = self.children.iter_mut();
        let columns = children.next().unwrap();
        let rows = children.next().unwrap();
        let cells = children.next().unwrap();
        let cells: &mut PanelList = cells.as_any().downcast_mut().unwrap();

        let info = &info;

        // layout columns
        columns.layout_each(
            wl,
            |ci, col, wl| {
                let info = info.columns[ci];
                LAYOUT.with_constraints(c.with_exact(info.width, grid_size.height), || col.layout(wl))
            },
            |_, _| PxSize::zero(),
        );
        // layout rows
        rows.layout_each(
            wl,
            |ri, row, wl| {
                let info = info.rows[ri];
                LAYOUT.with_constraints(c.with_exact(grid_size.width, info.height), || row.layout(wl))
            },
            |_, _| PxSize::zero(),
        );
        // layout and translate cells
        let cells_offset = columns.len() + rows.len();

        cells.layout_each(
            wl,
            |i, cell, o, wl| {
                let cell_info = cell::CellInfo::get_wgt(cell).actual(i, info.columns.len());

                if cell_info.column >= info.columns.len() || cell_info.row >= info.rows.len() {
                    wl.collapse_child(cells_offset + i);
                    return PxSize::zero(); // continue;
                }

                let cell_offset = PxVector::new(info.columns[cell_info.column].x, info.rows[cell_info.row].y);
                let mut cell_size = PxSize::zero();

                for col in cell_info.column..(cell_info.column + cell_info.column_span).min(info.columns.len()) {
                    if info.columns[col].width != Px(0) {
                        cell_size.width += info.columns[col].width + spacing.column;
                    }
                }
                cell_size.width -= spacing.column;

                for row in cell_info.row..(cell_info.row + cell_info.row_span).min(info.rows.len()) {
                    if info.rows[row].height != Px(0) {
                        cell_size.height += info.rows[row].height + spacing.row;
                    }
                }
                cell_size.height -= spacing.row;

                if cell_size.is_empty() {
                    wl.collapse_child(cells_offset + i);
                    return PxSize::zero(); // continue;
                }

                let (_, define_ref_frame) = LAYOUT.with_constraints(c.with_exact_size(cell_size), || wl.with_child(|wl| cell.layout(wl)));
                o.child_offset = cell_offset;
                o.define_reference_frame = define_ref_frame;

                cell_size
            },
            |_, _| PxSize::zero(),
        );

        c.fill_size_or(grid_size)
    }

    #[UiNode]
    fn render(&mut self, frame: &mut FrameBuilder) {
        let info = &self.info;

        let mut children = self.children.iter_mut();
        let columns = children.next().unwrap();
        let rows = children.next().unwrap();
        let cells: &mut PanelList = children.next().unwrap().as_any().downcast_mut().unwrap();
        let offset_key = cells.offset_key();

        columns.for_each(|i, child| {
            let offset = PxVector::new(info.columns[i].x, Px(0));
            frame.push_reference_frame(
                (offset_key, i as u32).into(),
                FrameValue::Value(offset.into()),
                true,
                true,
                |frame| {
                    child.render(frame);
                },
            );
        });
        let i_extra = columns.len();
        rows.for_each(|i, child| {
            let offset = PxVector::new(Px(0), info.rows[i].y);
            frame.push_reference_frame(
                (offset_key, (i + i_extra) as u32).into(),
                FrameValue::Value(offset.into()),
                true,
                true,
                |frame| {
                    child.render(frame);
                },
            );
        });
        let i_extra = i_extra + rows.len();
        cells.for_each_z_sorted(|i, child, data| {
            if data.define_reference_frame {
                frame.push_reference_frame(
                    (offset_key, (i + i_extra) as u32).into(),
                    FrameValue::Value(data.child_offset.into()),
                    true,
                    true,
                    |frame| {
                        child.render(frame);
                    },
                );
            } else {
                frame.push_child(data.child_offset, |frame| child.render(frame));
            }
        });
    }

    #[UiNode]
    fn render_update(&mut self, update: &mut FrameUpdate) {
        let info = &self.info;
        let mut children = self.children.iter_mut();
        let columns = children.next().unwrap();
        let rows = children.next().unwrap();
        let cells: &mut PanelList = children.next().unwrap().as_any().downcast_mut().unwrap();

        columns.for_each(|i, child| {
            let offset = PxVector::new(info.columns[i].x, Px(0));
            update.with_transform_value(&offset.into(), |update| {
                child.render_update(update);
            });
        });
        rows.for_each(|i, child| {
            let offset = PxVector::new(Px(0), info.rows[i].y);
            update.with_transform_value(&offset.into(), |update| {
                child.render_update(update);
            });
        });
        cells.for_each(|_, child, data| {
            if data.define_reference_frame {
                update.with_transform_value(&data.child_offset.into(), |update| {
                    child.render_update(update);
                });
            } else {
                update.with_child(data.child_offset, |update| {
                    child.render_update(update);
                })
            }
        })
    }
}

/// Arguments for [`grid::auto_grow_fn`].
///
/// [`grid::auto_grow_fn`]: fn@grid::auto_grow_fn.
#[derive(Clone, Debug)]
pub struct AutoGrowFnArgs {
    /// Auto-grow direction.
    pub mode: AutoGrowMode,
    /// Column index.
    pub index: usize,
}

/// Grid auto-grow direction.
///
/// The associated value is the maximum columns or rows that are allowed in the grid.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AutoGrowMode {
    /// Auto generate columns.
    Columns(u32),
    /// Auto generate rows.
    Rows(u32),
}
impl AutoGrowMode {
    /// Value that does not generate any new row or column.
    pub fn disabled() -> Self {
        Self::Rows(0)
    }

    /// Columns, not specific maximum limit.
    pub fn columns() -> Self {
        Self::Columns(u32::MAX)
    }

    /// Rows, not specific maximum limit.
    pub fn rows() -> Self {
        Self::Columns(u32::MAX)
    }

    /// Set the maximum columns or rows allowed.
    pub fn with_limit(self, limit: u32) -> Self {
        match self {
            AutoGrowMode::Columns(_) => AutoGrowMode::Columns(limit),
            AutoGrowMode::Rows(_) => AutoGrowMode::Rows(limit),
        }
    }
}
//! Commands that control the editable text.
//!
//! Most of the normal text editing is controlled by keyboard events, the [`EDIT_CMD`]
//! command allows for arbitrary text editing without needing to simulate keyboard events.
//!
//! The [`nodes::resolve_text`] node implements [`EDIT_CMD`] when the text is editable.

use std::{any::Any, borrow::Cow, fmt, ops, sync::Arc};

use crate::core::{
    gesture::{shortcut, CommandShortcutExt, ShortcutFilter},
    task::parking_lot::Mutex,
    undo::*,
};

use super::{
    nodes::{LayoutText, ResolvedText},
    *,
};

command! {
    /// Applies the [`TextEditOp`] into the text if it is editable.
    ///
    /// The request must be set as the command parameter.
    pub static EDIT_CMD;

    /// Applies the [`TextSelectOp`] into the text if it is editable.
    ///
    /// The request must be set as the command parameter.
    pub static SELECT_CMD;

    /// Select all text.
    ///
    /// The request is the same as [`SELECT_CMD`] with [`TextSelectOp::select_all`].
    pub static SELECT_ALL_CMD = {
        name: "Select All",
        shortcut: shortcut!(CTRL+'A'),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };;
}

struct SharedTextEditOp {
    data: Box<dyn Any + Send>,
    op: Box<dyn FnMut(&BoxedVar<Txt>, &mut dyn Any, UndoFullOp) + Send>,
}

/// Represents a text edit operation that can be send to an editable text using [`EDIT_CMD`].
#[derive(Clone)]
pub struct TextEditOp(Arc<Mutex<SharedTextEditOp>>);
impl fmt::Debug for TextEditOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextEditOp").finish_non_exhaustive()
    }
}
impl TextEditOp {
    /// New text edit operation.
    ///
    /// The editable text widget that handles [`EDIT_CMD`] will call `op` during event handling in
    /// the [`nodes::resolve_text`] context. You can position the caret using [`ResolvedText::caret`],
    /// the text widget will detect changes to it and react accordingly (updating caret position and animation),
    /// the caret index is also snapped to the nearest grapheme start.
    ///
    /// The `op` arguments are the text variable, a custom data `D` and what [`UndoFullOp`] query, all
    /// text edit operations must be undoable, first [`UndoOp::Redo`] is called to "do", then undo and redo again
    /// if the user requests undo & redo. The text variable is always read-write when `op` is called, more than
    /// one op can be called before the text variable updates, and [`ResolvedText::pending_edit`] is always false.
    pub fn new<D>(data: D, mut op: impl FnMut(&BoxedVar<Txt>, &mut D, UndoFullOp) + Send + 'static) -> Self
    where
        D: Send + Any + 'static,
    {
        Self(Arc::new(Mutex::new(SharedTextEditOp {
            data: Box::new(data),
            op: Box::new(move |var, data, o| op(var, data.downcast_mut().unwrap(), o)),
        })))
    }

    /// Insert operation.
    ///
    /// The `insert` text is inserted at the current caret index or at `0`, or replaces the current selection,
    /// after insert the caret is positioned after the inserted text.
    pub fn insert(insert: impl Into<Txt>) -> Self {
        struct InsertData {
            insert: Txt,
            selection_state: SelectionState,
            removed: Txt,
        }
        let data = InsertData {
            insert: insert.into(),
            selection_state: SelectionState::Initial,
            removed: Txt::from_static(""),
        };
        #[derive(Clone, Copy)]
        enum SelectionState {
            Initial,
            Caret(CaretIndex),
            Selection(CaretIndex, CaretIndex),
        }

        Self::new(data, move |txt, data, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let insert = &data.insert;

                if let SelectionState::Initial = data.selection_state {
                    if let Some(range) = caret.selection_range() {
                        data.selection_state = SelectionState::Selection(range.start, range.end);
                    } else {
                        data.selection_state = SelectionState::Caret(caret.index.unwrap_or(CaretIndex::ZERO));
                    }
                }
                match data.selection_state {
                    SelectionState::Initial => unreachable!(),
                    SelectionState::Caret(insert_idx) => {
                        let i = insert_idx.index;
                        txt.modify(clmv!(insert, |args| {
                            args.to_mut().to_mut().insert_str(i, insert.as_str());
                        }))
                        .unwrap();

                        let mut i = insert_idx;
                        i.index += insert.len();
                        caret.set_index(i);
                        caret.selection_index = None;
                    }
                    SelectionState::Selection(start, end) => {
                        let char_range = start.index..end.index;
                        txt.with(|t| {
                            let r = &t[char_range.clone()];
                            if r != data.removed {
                                data.removed = Txt::from_str(r);
                            }
                        });

                        txt.modify(clmv!(insert, |args| {
                            args.to_mut().to_mut().replace_range(char_range, insert.as_str());
                        }))
                        .unwrap();

                        caret.set_char_index(start.index + insert.len());
                        caret.selection_index = None;
                    }
                }
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let len = data.insert.len();
                let insert_idx = match data.selection_state {
                    SelectionState::Initial => unreachable!(),
                    SelectionState::Caret(c) => c,
                    SelectionState::Selection(start, _) => start,
                };
                let i = insert_idx.index;
                let removed = &data.removed;

                txt.modify(clmv!(removed, |args| {
                    args.to_mut().to_mut().replace_range(i..i + len, removed.as_str());
                }))
                .unwrap();

                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();
                caret.set_char_selection(insert_idx.index, insert_idx.index + removed.len());
            }
            UndoFullOp::Info { info } => {
                let mut label = Txt::from_static("\"");
                for (i, mut c) in data.insert.chars().take(21).enumerate() {
                    if i == 20 {
                        c = '…';
                    } else if c == '\n' {
                        c = '↵';
                    } else if c == '\t' {
                        c = '→';
                    } else if c == '\r' {
                        continue;
                    }
                    label.push(c);
                }
                label.push('"');
                *info = Some(Arc::new(label));
            }
            UndoFullOp::Merge {
                next_data,
                within_undo_interval,
                merged,
                ..
            } => {
                if within_undo_interval {
                    if let Some(next_data) = next_data.downcast_mut::<InsertData>() {
                        if let (SelectionState::Caret(mut after_idx), SelectionState::Caret(caret)) =
                            (data.selection_state, next_data.selection_state)
                        {
                            after_idx.index += data.insert.len();

                            if after_idx.index == caret.index {
                                data.insert.push_str(&next_data.insert);
                                *merged = true;
                            }
                        }
                    }
                }
            }
        })
    }

    /// Remove one *backspace range* ending at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::backspace_range`] for more details about what is removed.
    pub fn backspace() -> Self {
        Self::backspace_impl(SegmentedText::backspace_range)
    }
    // Remove one *backspace word range* ending at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::backspace_word_range`] for more details about what is removed.
    pub fn backspace_word() -> Self {
        Self::backspace_impl(SegmentedText::backspace_word_range)
    }
    fn backspace_impl(backspace_range: fn(&SegmentedText, usize, u32) -> std::ops::Range<usize>) -> Self {
        struct BackspaceData {
            caret: Option<CaretIndex>,
            count: u32,
            removed: Txt,
        }
        let data = BackspaceData {
            caret: None,
            count: 1,
            removed: Txt::from_static(""),
        };

        Self::new(data, move |txt, data, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let caret_idx = *data.caret.get_or_insert_with(|| caret.index.unwrap_or(CaretIndex::ZERO));
                let rmv = backspace_range(&ctx.text, caret_idx.index, data.count);
                if rmv.is_empty() {
                    data.removed = Txt::from_static("");
                    return;
                }

                txt.with(|t| {
                    let r = &t[rmv.clone()];
                    if r != data.removed {
                        data.removed = Txt::from_str(r);
                    }
                });

                txt.modify(move |args| {
                    args.to_mut().to_mut().replace_range(rmv, "");
                })
                .unwrap();

                let mut c = caret_idx;
                c.index -= data.removed.len();
                caret.set_index(c);
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                if data.removed.is_empty() {
                    return;
                }

                let caret_idx = data.caret.unwrap();
                let removed = &data.removed;

                let mut undo_idx = caret_idx;
                undo_idx.index -= removed.len();
                let i = undo_idx.index;
                txt.modify(clmv!(removed, |args| {
                    args.to_mut().to_mut().insert_str(i, removed.as_str());
                }))
                .unwrap();

                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();
                caret.set_index(caret_idx);
            }
            UndoFullOp::Info { info } => {
                *info = Some(if data.count == 1 {
                    Arc::new("⌫")
                } else {
                    Arc::new(formatx!("⌫ (x{})", data.count))
                })
            }
            UndoFullOp::Merge {
                next_data,
                within_undo_interval,
                merged,
                ..
            } => {
                if within_undo_interval {
                    if let Some(next_data) = next_data.downcast_mut::<BackspaceData>() {
                        let mut undone_caret = data.caret.unwrap();
                        undone_caret.index -= data.removed.len();

                        if undone_caret.index == next_data.caret.unwrap().index {
                            data.count += next_data.count;

                            next_data.removed.push_str(&data.removed);
                            data.removed = std::mem::take(&mut next_data.removed);
                            *merged = true;
                        }
                    }
                }
            }
        })
    }

    /// Remove one *delete range* starting at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::delete_range`] for more details about what is removed.
    pub fn delete() -> Self {
        Self::delete_impl(SegmentedText::delete_range)
    }
    /// Remove one *delete word range* starting at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::delete_word_range`] for more details about what is removed.
    pub fn delete_word() -> Self {
        Self::delete_impl(SegmentedText::delete_word_range)
    }
    fn delete_impl(delete_range: fn(&SegmentedText, usize, u32) -> std::ops::Range<usize>) -> Self {
        struct DeleteData {
            caret: Option<CaretIndex>,
            count: u32,
            removed: Txt,
        }
        let data = DeleteData {
            caret: None,
            count: 1,
            removed: Txt::from_static(""),
        };

        Self::new(data, move |txt, data, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let caret_idx = *data.caret.get_or_insert_with(|| caret.index.unwrap_or(CaretIndex::ZERO));

                let rmv = delete_range(&ctx.text, caret_idx.index, data.count);

                if rmv.is_empty() {
                    data.removed = Txt::from_static("");
                    return;
                }

                txt.with(|t| {
                    let r = &t[rmv.clone()];
                    if r != data.removed {
                        data.removed = Txt::from_str(r);
                    }
                });
                txt.modify(move |args| {
                    args.to_mut().to_mut().replace_range(rmv, "");
                })
                .unwrap();

                caret.set_index(caret_idx); // (re)start caret animation
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let removed = &data.removed;

                if data.removed.is_empty() {
                    return;
                }

                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let caret_idx = data.caret.unwrap();

                let i = caret_idx.index;
                txt.modify(clmv!(removed, |args| {
                    args.to_mut().to_mut().insert_str(i, removed.as_str());
                }))
                .unwrap();

                caret.set_index(caret_idx); // (re)start caret animation
            }
            UndoFullOp::Info { info } => {
                *info = Some(if data.count == 1 {
                    Arc::new("⌦")
                } else {
                    Arc::new(formatx!("⌦ (x{})", data.count))
                })
            }
            UndoFullOp::Merge {
                next_data,
                within_undo_interval,
                merged,
                ..
            } => {
                if within_undo_interval {
                    if let Some(next_data) = next_data.downcast_ref::<DeleteData>() {
                        if data.caret == next_data.caret {
                            data.count += next_data.count;
                            data.removed.push_str(&next_data.removed);
                            *merged = true;
                        }
                    }
                }
            }
        })
    }

    /// Replace operation.
    ///
    /// The `select_before` is removed, and `insert` inserted at the `select_before.start`, after insertion
    /// the `select_after` is applied, you can use an empty insert to just remove.
    ///
    /// All indexes are snapped to the nearest grapheme, you can use empty ranges to just position the caret.
    pub fn replace(mut select_before: ops::Range<usize>, insert: impl Into<Txt>, mut select_after: ops::Range<usize>) -> Self {
        let insert = insert.into();
        let mut removed = Txt::from_static("");

        Self::new((), move |txt, _, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
                let ctx = ResolvedText::get();

                select_before.start = ctx.text.snap_grapheme_boundary(select_before.start);
                select_before.end = ctx.text.snap_grapheme_boundary(select_before.end);

                txt.with(|t| {
                    let r = &t[select_before.clone()];
                    if r != removed {
                        removed = Txt::from_str(r);
                    }
                });

                txt.modify(clmv!(select_before, insert, |args| {
                    args.to_mut().to_mut().replace_range(select_before, insert.as_str());
                }))
                .unwrap();

                let mut caret = ctx.caret.lock();
                caret.set_char_selection(select_after.start, select_after.end);
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let ctx = ResolvedText::get();

                select_after.start = ctx.text.snap_grapheme_boundary(select_after.start);
                select_after.end = ctx.text.snap_grapheme_boundary(select_after.end);

                txt.modify(clmv!(select_after, removed, |args| {
                    args.to_mut().to_mut().replace_range(select_after, removed.as_str());
                }))
                .unwrap();

                ctx.caret.lock().set_char_selection(select_before.start, select_before.end);
            }
            UndoFullOp::Info { info } => *info = Some(Arc::new("replace")),
            UndoFullOp::Merge { .. } => {}
        })
    }

    /// Applies [`TEXT_TRANSFORM_VAR`] and [`WHITE_SPACE_VAR`] to the text.
    pub fn apply_transforms() -> Self {
        let mut prev = Txt::from_static("");
        let mut transform = None::<(TextTransformFn, WhiteSpace)>;
        Self::new((), move |txt, _, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
                let (t, w) = transform.get_or_insert_with(|| (TEXT_TRANSFORM_VAR.get(), WHITE_SPACE_VAR.get()));

                let new_txt = txt.with(|txt| {
                    let transformed = t.transform(txt);
                    let white_spaced = w.transform(transformed.as_ref());
                    if let Cow::Owned(w) = white_spaced {
                        Some(w)
                    } else if let Cow::Owned(t) = transformed {
                        Some(t)
                    } else {
                        None
                    }
                });

                if let Some(t) = new_txt {
                    if txt.with(|t| t != prev.as_str()) {
                        prev = txt.get();
                    }
                    let _ = txt.set(t);
                }
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                if txt.with(|t| t != prev.as_str()) {
                    let _ = txt.set(prev.clone());
                }
            }
            UndoFullOp::Info { info } => *info = Some(Arc::new("transform")),
            UndoFullOp::Merge { .. } => {}
        })
    }

    pub(super) fn call(self, text: &BoxedVar<Txt>) {
        {
            let mut op = self.0.lock();
            let op = &mut *op;
            (op.op)(text, &mut *op.data, UndoFullOp::Op(UndoOp::Redo));
        }
        UNDO.register(UndoTextEditOp::new(self));
    }
}

/// Parameter for [`EDIT_CMD`], apply the request and don't register undo.
#[derive(Debug, Clone)]
pub(super) struct UndoTextEditOp {
    pub target: WidgetId,
    edit_op: TextEditOp,
    exec_op: UndoOp,
}
impl UndoTextEditOp {
    fn new(edit_op: TextEditOp) -> Self {
        Self {
            target: WIDGET.id(),
            edit_op,
            exec_op: UndoOp::Undo,
        }
    }

    pub(super) fn call(&self, text: &BoxedVar<Txt>) {
        let mut op = self.edit_op.0.lock();
        let op = &mut *op;
        (op.op)(text, &mut *op.data, UndoFullOp::Op(self.exec_op))
    }
}
impl UndoAction for UndoTextEditOp {
    fn undo(self: Box<Self>) -> Box<dyn RedoAction> {
        EDIT_CMD.scoped(self.target).notify_param(Self {
            target: self.target,
            edit_op: self.edit_op.clone(),
            exec_op: UndoOp::Undo,
        });
        self
    }

    fn info(&mut self) -> Arc<dyn UndoInfo> {
        let mut op = self.edit_op.0.lock();
        let op = &mut *op;
        let mut info = None;
        let none_var = LocalVar(Txt::from_static("")).boxed();
        (op.op)(&none_var, &mut *op.data, UndoFullOp::Info { info: &mut info });

        info.unwrap_or_else(|| Arc::new("text edit"))
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn merge(self: Box<Self>, mut args: UndoActionMergeArgs) -> Result<Box<dyn UndoAction>, (Box<dyn UndoAction>, Box<dyn UndoAction>)> {
        if let Some(next) = args.next.as_any().downcast_mut::<Self>() {
            let mut merged = false;

            {
                let mut op = self.edit_op.0.lock();
                let op = &mut *op;
                let none_var = LocalVar(Txt::from_static("")).boxed();

                let mut next_op = next.edit_op.0.lock();

                (op.op)(
                    &none_var,
                    &mut *op.data,
                    UndoFullOp::Merge {
                        next_data: &mut *next_op.data,
                        prev_timestamp: args.prev_timestamp,
                        within_undo_interval: args.within_undo_interval,
                        merged: &mut merged,
                    },
                );
            }

            if merged {
                return Ok(self);
            }
        }

        Err((self, args.next))
    }
}
impl RedoAction for UndoTextEditOp {
    fn redo(self: Box<Self>) -> Box<dyn UndoAction> {
        EDIT_CMD.scoped(self.target).notify_param(Self {
            target: self.target,
            edit_op: self.edit_op.clone(),
            exec_op: UndoOp::Redo,
        });
        self
    }

    fn info(&mut self) -> Arc<dyn UndoInfo> {
        let mut op = self.edit_op.0.lock();
        let op = &mut *op;
        let mut info = None;
        let none_var = LocalVar(Txt::from_static("")).boxed();
        (op.op)(&none_var, &mut *op.data, UndoFullOp::Info { info: &mut info });

        info.unwrap_or_else(|| Arc::new("text edit"))
    }
}

/// Represents a text selection operation that can be send to an editable text using [`SELECT_CMD`].
#[derive(Clone)]
pub struct TextSelectOp {
    op: Arc<Mutex<dyn FnMut() + Send>>,
}
impl fmt::Debug for TextSelectOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSelectOp").finish_non_exhaustive()
    }
}
impl TextSelectOp {
    /// New text select operation.
    ///
    /// The editable text widget that handles [`SELECT_CMD`] will call `op` during event handling in
    /// the [`nodes::layout_text`] context. You can position the caret using [`ResolvedText::caret`],
    /// the text widget will detect changes to it and react accordingly (updating caret position and animation),
    /// the caret index is also snapped to the nearest grapheme start.
    pub fn new(op: impl FnMut() + Send + 'static) -> Self {
        Self {
            op: Arc::new(Mutex::new(op)),
        }
    }

    /// Clear selection and move the caret to the next insert index.
    ///
    /// This is the `Right` key operation.
    pub fn next() -> Self {
        Self::new(|| next_prev(true, SegmentedText::next_insert_index))
    }

    /// Extend or shrink selection by moving the caret to the next insert index.
    ///
    /// This is the `SHIFT+Right` key operation.
    pub fn select_next() -> Self {
        Self::new(|| next_prev(false, SegmentedText::next_insert_index))
    }

    /// Clear selection and move the caret to the previous insert index.
    ///
    /// This is the `Left` key operation.
    pub fn prev() -> Self {
        Self::new(|| next_prev(true, SegmentedText::prev_insert_index))
    }

    /// Extend or shrink selection by moving the caret to the previous insert index.
    ///
    /// This is the `SHIFT+Left` key operation.
    pub fn select_prev() -> Self {
        Self::new(|| next_prev(false, SegmentedText::prev_insert_index))
    }

    /// Clear selection and move the caret to the next word insert index.
    ///
    /// This is the `CTRL+Right` shortcut operation.
    pub fn next_word() -> Self {
        Self::new(|| next_prev(true, SegmentedText::next_word_index))
    }

    /// Extend or shrink selection by moving the caret to the next word insert index.
    ///
    /// This is the `CTRL+SHIFT+Right` shortcut operation.
    pub fn select_next_word() -> Self {
        Self::new(|| next_prev(false, SegmentedText::next_word_index))
    }

    /// Clear selection and move the caret to the previous word insert index.
    ///
    /// This is the `CTRL+Left` shortcut operation.
    pub fn prev_word() -> Self {
        Self::new(|| next_prev(true, SegmentedText::prev_word_index))
    }

    /// Extend or shrink selection by moving the caret to the previous word insert index.
    ///
    /// This is the `CTRL+SHIFT+Left` shortcut operation.
    pub fn select_prev_word() -> Self {
        Self::new(|| next_prev(false, SegmentedText::prev_word_index))
    }

    /// Clear selection and move the caret to the nearest insert index on the previous line.
    ///
    /// This is the `Up` key operation.
    pub fn line_up() -> Self {
        Self::new(|| line_up_down(true, -1))
    }

    /// Extend or shrink selection by moving the caret to the nearest insert index on the previous line.
    ///
    /// This is the `SHIFT+Up` key operation.
    pub fn select_line_up() -> Self {
        Self::new(|| line_up_down(false, -1))
    }

    /// Clear selection and move the caret to the nearest insert index on the next line.
    ///
    /// This is the `Down` key operation.
    pub fn line_down() -> Self {
        Self::new(|| line_up_down(true, 1))
    }

    /// Extend or shrink selection by moving the caret to the nearest insert index on the next line.
    ///
    /// This is the `SHIFT+Down` key operation.
    pub fn select_line_down() -> Self {
        Self::new(|| line_up_down(false, 1))
    }

    /// Clear selection and move the caret one viewport up.
    ///
    /// This is the `PageUp` key operation.
    pub fn page_up() -> Self {
        Self::new(|| page_up_down(true, -1))
    }

    /// Extend or shrink selection by moving the caret one viewport up.
    ///
    /// This is the `SHIFT+PageUp` key operation.
    pub fn select_page_up() -> Self {
        Self::new(|| page_up_down(false, -1))
    }

    /// Clear selection and move the caret one viewport down.
    ///
    /// This is the `PageDown` key operation.
    pub fn page_down() -> Self {
        Self::new(|| page_up_down(true, 1))
    }

    /// Extend or shrink selection by moving the caret one viewport down.
    ///
    /// This is the `SHIFT+PageDown` key operation.
    pub fn select_page_down() -> Self {
        Self::new(|| page_up_down(false, 1))
    }

    /// Clear selection and move the caret to the start of the line.
    ///
    /// This is the `Home` key operation.
    pub fn line_start() -> Self {
        Self::new(|| line_start_end(true, |li| li.text_range().start))
    }

    /// Extend or shrink selection by moving the caret to the start of the line.
    ///
    /// This is the `SHIFT+Home` key operation.
    pub fn select_line_start() -> Self {
        Self::new(|| line_start_end(false, |li| li.text_range().start))
    }

    /// Clear selection and move the caret to the end of the line (before the line-break if any).
    ///
    /// This is the `End` key operation.
    pub fn line_end() -> Self {
        Self::new(|| line_start_end(true, |li| li.text_caret_range().end))
    }

    /// Extend or shrink selection by moving the caret to the end of the line (before the line-break if any).
    ///
    /// This is the `SHIFT+End` key operation.
    pub fn select_line_end() -> Self {
        Self::new(|| line_start_end(false, |li| li.text_caret_range().end))
    }

    /// Clear selection and move the caret to the text start.
    ///
    /// This is the `CTRL+Home` shortcut operation.
    pub fn text_start() -> Self {
        Self::new(|| text_start_end(true, |_| 0))
    }

    /// Extend or shrink selection by moving the caret to the text start.
    ///
    /// This is the `CTRL+SHIFT+Home` shortcut operation.
    pub fn select_text_start() -> Self {
        Self::new(|| text_start_end(false, |_| 0))
    }

    /// Clear selection and move the caret to the text end.
    ///
    /// This is the `CTRL+End` shortcut operation.
    pub fn text_end() -> Self {
        Self::new(|| text_start_end(true, |s| s.len()))
    }

    /// Extend or shrink selection by moving the caret to the text end.
    ///
    /// This is the `CTRL+SHIFT+End` shortcut operation.
    pub fn select_text_end() -> Self {
        Self::new(|| text_start_end(false, |s| s.len()))
    }

    /// Clear selection and move the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down operation.
    pub fn nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            nearest_to(true, window_point);
        })
    }

    /// Extend or shrink selection by moving the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down when holding SHIFT operation.
    pub fn select_nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            nearest_to(false, window_point);
        })
    }

    /// Replace or extend selection with the word nearest to the `window_point`
    ///
    /// This is the mouse primary button double click.
    pub fn select_word_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        Self::new(move || select_line_word_nearest_to(replace_selection, true, window_point))
    }

    /// Replace or extend selection with the line nearest to the `window_point`
    ///
    /// This is the mouse primary button triple click.
    pub fn select_line_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        Self::new(move || select_line_word_nearest_to(replace_selection, false, window_point))
    }

    /// Select the full text.
    pub fn select_all() -> Self {
        Self::new(|| {
            let ctx = ResolvedText::get();
            let mut c = ctx.caret.lock();
            c.set_char_selection(0, ctx.text.text().len());
            c.skip_next_scroll = true;
        })
    }

    pub(super) fn call(self) {
        (self.op.lock())();
    }
}

fn next_prev(clear_selection: bool, insert_index_fn: fn(&SegmentedText, usize) -> usize) {
    let ctx = ResolvedText::get();
    let mut c = ctx.caret.lock();
    let mut i = c.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        c.selection_index = None;
    } else if c.selection_index.is_none() {
        c.selection_index = Some(i);
    }
    i.index = insert_index_fn(&ctx.text, i.index);
    c.set_index(i);
    c.used_retained_x = false;
}

fn line_up_down(clear_selection: bool, diff: i8) {
    let diff = diff as isize;
    let ctx = ResolvedText::get();
    let layout = LayoutText::get();

    let mut c = ctx.caret.lock();
    let mut i = c.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        c.selection_index = None;
    } else if c.selection_index.is_none() {
        c.selection_index = Some(i);
    }

    c.used_retained_x = true;

    if layout.caret_origin.is_some() {
        let last_line = layout.shaped_text.lines_len().saturating_sub(1);
        let li = i.line;
        let next_li = li.saturating_add_signed(diff).min(last_line);
        if li != next_li {
            match layout.shaped_text.line(next_li) {
                Some(l) => {
                    i.line = next_li;
                    i.index = match l.nearest_seg(layout.caret_retained_x) {
                        Some(s) => s.nearest_char_index(layout.caret_retained_x, ctx.text.text()),
                        None => l.text_range().end,
                    }
                }
                None => i = CaretIndex::ZERO,
            };
            i.index = ctx.text.snap_grapheme_boundary(i.index);
            c.set_index(i);
        }
    }

    if c.index.is_none() {
        c.set_index(CaretIndex::ZERO);
        c.selection_index = None;
    }
}

fn page_up_down(clear_selection: bool, diff: i8) {
    let diff = diff as i32;
    let resolved = ResolvedText::get();
    let layout = LayoutText::get();

    let mut c = resolved.caret.lock();
    let mut i = c.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        c.selection_index = None;
    } else if c.selection_index.is_none() {
        c.selection_index = Some(i);
    }

    let page_y = layout.viewport.height * Px(diff);
    c.used_retained_x = true;
    if layout.caret_origin.is_some() {
        let li = i.line;
        if let Some(li) = layout.shaped_text.line(li) {
            let target_line_y = li.rect().origin.y + page_y;
            match layout.shaped_text.nearest_line(target_line_y) {
                Some(l) => {
                    i.line = l.index();
                    i.index = match l.nearest_seg(layout.caret_retained_x) {
                        Some(s) => s.nearest_char_index(layout.caret_retained_x, resolved.text.text()),
                        None => l.text_range().end,
                    }
                }
                None => i = CaretIndex::ZERO,
            };
            i.index = resolved.text.snap_grapheme_boundary(i.index);
            c.set_index(i);
        }
    }

    if c.index.is_none() {
        c.set_index(CaretIndex::ZERO);
        c.selection_index = None;
    }
}

fn line_start_end(clear_selection: bool, index: impl FnOnce(ShapedLine) -> usize) {
    let resolved = ResolvedText::get();
    let layout = LayoutText::get();

    let mut c = resolved.caret.lock();
    let mut i = c.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        c.selection_index = None;
    } else if c.selection_index.is_none() {
        c.selection_index = Some(i);
    }

    if let Some(li) = layout.shaped_text.line(i.line) {
        i.index = index(li);
        c.set_index(i);
        c.used_retained_x = false;
    }
}

fn text_start_end(clear_selection: bool, index: impl FnOnce(&str) -> usize) {
    let resolved = ResolvedText::get();

    let mut c = resolved.caret.lock();
    let mut i = c.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        c.selection_index = None;
    } else if c.selection_index.is_none() {
        c.selection_index = Some(i);
    }

    i.index = index(resolved.text.text());

    c.set_index(i);
    c.used_retained_x = false;
}

fn nearest_to(clear_selection: bool, window_point: DipPoint) {
    let resolved = ResolvedText::get();
    let layout = LayoutText::get();

    let mut c = resolved.caret.lock();
    let mut i = c.index.unwrap_or(CaretIndex::ZERO);

    if clear_selection {
        c.selection_index = None;
    } else if c.selection_index.is_none() {
        c.selection_index = Some(i);
    }

    c.used_retained_x = false;

    //if there was at least one layout
    let info = layout.render_info.lock();
    if let Some(pos) = info
        .transform
        .inverse()
        .and_then(|t| t.project_point(window_point.to_px(info.scale_factor.0)))
    {
        //if has rendered
        i = match layout.shaped_text.nearest_line(pos.y) {
            Some(l) => CaretIndex {
                line: l.index(),
                index: match l.nearest_seg(pos.x) {
                    Some(s) => s.nearest_char_index(pos.x, resolved.text.text()),
                    None => l.text_range().end,
                },
            },
            None => CaretIndex::ZERO,
        };
        i.index = resolved.text.snap_grapheme_boundary(i.index);
        c.set_index(i);
    }

    if c.index.is_none() {
        c.set_index(CaretIndex::ZERO);
        c.selection_index = None;
    }
}

fn select_line_word_nearest_to(replace_selection: bool, select_word: bool, window_point: DipPoint) {
    let resolved = ResolvedText::get();
    let layout = LayoutText::get();

    let mut c = resolved.caret.lock();

    //if there was at least one layout
    let info = layout.render_info.lock();
    if let Some(pos) = info
        .transform
        .inverse()
        .and_then(|t| t.project_point(window_point.to_px(info.scale_factor.0)))
    {
        //if has rendered
        if let Some(l) = layout.shaped_text.nearest_line(pos.y) {
            let range = if select_word {
                l.nearest_seg(pos.x).map(|seg| seg.text_range()).unwrap_or_else(|| l.text_range())
            } else {
                l.actual_text_caret_range()
            };

            if replace_selection {
                c.selection_index = Some(CaretIndex {
                    line: l.index(),
                    index: range.start,
                });
            }
            c.set_index(CaretIndex {
                line: l.index(),
                index: range.end,
            });
            return;
        };
    }

    if c.index.is_none() {
        c.set_index(CaretIndex::ZERO);
        c.selection_index = None;
    }
}

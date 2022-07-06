use super::*;

/// Full address of a widget in a specific [`WidgetInfoTree`].
#[derive(Clone)]
pub struct WidgetPath {
    pub(super) node_id: Option<(WidgetInfoTreeId, tree::NodeId)>,
    window_id: WindowId,
    path: Box<[WidgetId]>,
}
impl PartialEq for WidgetPath {
    /// Paths are equal if they share the same [window](Self::window_id) and [widget paths](Self::widgets_path).
    fn eq(&self, other: &Self) -> bool {
        self.window_id == other.window_id && self.path == other.path
    }
}
impl Eq for WidgetPath {}
impl PartialEq<InteractionPath> for WidgetPath {
    fn eq(&self, other: &InteractionPath) -> bool {
        other == self
    }
}
impl fmt::Debug for WidgetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("WidgetPath")
                .field("window_id", &self.window_id)
                .field("path", &self.path)
                .finish_non_exhaustive()
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for WidgetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}//", self.window_id)?;
        for w in self.ancestors() {
            write!(f, "{w}/")?;
        }
        write!(f, "{}", self.widget_id())
    }
}
impl WidgetPath {
    pub(super) fn new_internal(window_id: WindowId, path: Box<[WidgetId]>, tree_id: WidgetInfoTreeId, node_id: tree::NodeId) -> Self {
        Self {
            node_id: Some((tree_id, node_id)),
            window_id,
            path,
        }
    }

    /// New custom widget path.
    ///
    /// The path is not guaranteed to have ever existed.
    pub fn new<P: Into<Box<[WidgetId]>>>(window_id: WindowId, path: P) -> WidgetPath {
        WidgetPath {
            node_id: None,
            window_id,
            path: path.into(),
        }
    }

    /// Id of the window that contains the widgets.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Widgets that contain [`widget_id`](WidgetPath::widget_id), root first.
    pub fn ancestors(&self) -> &[WidgetId] {
        &self.path[..self.path.len() - 1]
    }

    /// The widget.
    pub fn widget_id(&self) -> WidgetId {
        self.path[self.path.len() - 1]
    }

    /// [`ancestors`](WidgetPath::ancestors) and [`widget_id`](WidgetPath::widget_id), root first.
    pub fn widgets_path(&self) -> &[WidgetId] {
        &self.path[..]
    }

    /// If the `widget_id` is part of the path.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.path.iter().any(move |&w| w == widget_id)
    }

    /// Make a path to an ancestor id that is contained in the current path.
    pub fn ancestor_path(&self, ancestor_id: WidgetId) -> Option<Cow<WidgetPath>> {
        self.path.iter().position(|&id| id == ancestor_id).map(|i| {
            if i == self.path.len() - 1 {
                Cow::Borrowed(self)
            } else {
                Cow::Owned(WidgetPath {
                    node_id: None,
                    window_id: self.window_id,
                    path: self.path[..i].to_vec().into_boxed_slice(),
                })
            }
        })
    }

    /// Get the inner most widget parent shared by both `self` and `other`.
    pub fn shared_ancestor<'a>(&'a self, other: &'a WidgetPath) -> Option<Cow<'a, WidgetPath>> {
        if self.window_id == other.window_id {
            if let Some(i) = self.path.iter().zip(other.path.iter()).position(|(a, b)| a != b) {
                if i == 0 {
                    None
                } else {
                    let path = self.path[..i].to_vec().into_boxed_slice();
                    Some(Cow::Owned(WidgetPath {
                        node_id: None,
                        window_id: self.window_id,
                        path,
                    }))
                }
            } else if self.path.len() <= other.path.len() {
                Some(Cow::Borrowed(self))
            } else {
                Some(Cow::Borrowed(other))
            }
        } else {
            None
        }
    }

    /// Gets a path to the root widget of this path.
    pub fn root_path(&self) -> Cow<WidgetPath> {
        if self.path.len() == 1 {
            Cow::Borrowed(self)
        } else {
            Cow::Owned(WidgetPath {
                node_id: None,
                window_id: self.window_id,
                path: Box::new([self.path[0]]),
            })
        }
    }
}

/// Represents a [`WidgetPath`] with extra [`Interactivity`] for each widget.
#[derive(Clone)]
pub struct InteractionPath {
    path: WidgetPath,
    blocked: usize,
    disabled: usize,
}
impl PartialEq for InteractionPath {
    /// Paths are equal if the are the same window, widgets and interactivity.
    fn eq(&self, other: &Self) -> bool {
        self.as_path() == other.as_path() && self.blocked == other.blocked && self.disabled == other.disabled
    }
}
impl Eq for InteractionPath {}
impl PartialEq<WidgetPath> for InteractionPath {
    /// Paths are equal if the are the same window, widgets and interactivity.
    fn eq(&self, other: &WidgetPath) -> bool {
        self.as_path() == other
    }
}
impl fmt::Debug for InteractionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("InteractionPath")
                .field("window_id", &self.window_id)
                .field("path", &self.path)
                .field("blocked", &self.blocked_index())
                .field("disabled", &self.disabled_index())
                .finish_non_exhaustive()
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for InteractionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_path())
    }
}
impl InteractionPath {
    pub(super) fn new_internal(path: WidgetPath, blocked: usize, disabled: usize) -> Self {
        Self { path, blocked, disabled }
    }

    /// New custom interactivity path.
    ///
    /// The path is not guaranteed to have ever existed.
    pub fn new<P: IntoIterator<Item = (WidgetId, Interactivity)>>(window_id: WindowId, path: P) -> InteractionPath {
        let iter = path.into_iter();
        let mut path = Vec::with_capacity(iter.size_hint().0);
        let mut blocked = None;
        let mut disabled = None;
        for (i, (w, intera)) in iter.enumerate() {
            path.push(w);
            if blocked.is_none() && intera.contains(Interactivity::BLOCKED) {
                blocked = Some(i);
            }
            if disabled.is_none() && intera.contains(Interactivity::DISABLED) {
                disabled = Some(i);
            }
        }
        let len = path.len();
        InteractionPath {
            path: WidgetPath::new(window_id, path),
            blocked: blocked.unwrap_or(len),
            disabled: disabled.unwrap_or(len),
        }
    }

    /// New custom widget path with all widgets enabled.
    ///
    /// The path is not guaranteed to have ever existed.
    pub fn new_enabled<P: Into<Box<[WidgetId]>>>(window_id: WindowId, path: P) -> InteractionPath {
        let path = WidgetPath::new(window_id, path);
        Self::from_enabled(path)
    }

    /// New custom interactivity path with all widgets enabled.
    pub fn from_enabled(path: WidgetPath) -> InteractionPath {
        let len = path.path.len();
        InteractionPath {
            path,
            blocked: len,
            disabled: len,
        }
    }

    /// Dereferences to the path.
    pub fn as_path(&self) -> &WidgetPath {
        &self.path
    }

    /// Index of first [`BLOCKED`].
    ///
    /// [`BLOCKED`]: Interactivity::BLOCKED
    pub fn blocked_index(&self) -> Option<usize> {
        if self.blocked < self.path.path.len() {
            Some(self.blocked)
        } else {
            None
        }
    }
    /// Index of first [`DISABLED`].
    ///
    /// [`DISABLED`]: Interactivity::DISABLED
    pub fn disabled_index(&self) -> Option<usize> {
        if self.disabled < self.path.path.len() {
            Some(self.disabled)
        } else {
            None
        }
    }

    /// Interactivity for each widget, root first.
    pub fn interaction_path(&self) -> impl Iterator<Item = Interactivity> {
        struct InteractivityIter {
            range: ops::Range<usize>,
            blocked: usize,
            disabled: usize,
        }
        impl Iterator for InteractivityIter {
            type Item = Interactivity;

            fn next(&mut self) -> Option<Self::Item> {
                self.range.next().map(|i| {
                    let mut intera = Interactivity::ENABLED;
                    if self.blocked <= i {
                        intera |= Interactivity::BLOCKED;
                    }
                    if self.disabled <= i {
                        intera |= Interactivity::DISABLED;
                    }
                    intera
                })
            }
        }

        InteractivityIter {
            range: 0..self.path.path.len(),
            blocked: self.blocked,
            disabled: self.disabled,
        }
    }

    /// Search for the interactivity value associated with the widget in the path.
    pub fn interactivity_of(&self, widget_id: WidgetId) -> Option<Interactivity> {
        self.path.widgets_path().iter().position(|&w| w == widget_id).map(|i| {
            let mut intera = Interactivity::ENABLED;
            if self.blocked <= i {
                intera |= Interactivity::BLOCKED;
            }
            if self.disabled <= i {
                intera |= Interactivity::DISABLED;
            }
            intera
        })
    }

    /// Interactivity of the widget.
    pub fn interactivity(&self) -> Interactivity {
        let mut intera = Interactivity::ENABLED;
        let len = self.path.path.len();
        if self.blocked < len {
            intera |= Interactivity::BLOCKED;
        }
        if self.disabled < len {
            intera |= Interactivity::DISABLED;
        }
        intera
    }

    /// Zip widgets and interactivity.
    pub fn zip(&self) -> impl Iterator<Item = (WidgetId, Interactivity)> + '_ {
        self.path.widgets_path().iter().copied().zip(self.interaction_path())
    }

    /// Gets the [`ENABLED`] or [`DISABLED`] part of the path, or none if the widget is blocked at the root.
    ///
    /// [`ENABLED`]: Interactivity::ENABLED
    /// [`DISABLED`]: Interactivity::DISABLED
    pub fn unblocked(self) -> Option<InteractionPath> {
        if self.blocked < self.path.path.len() {
            if self.blocked == 0 {
                return None;
            }
            let blocked = self.blocked - 1;
            Some(InteractionPath {
                path: WidgetPath {
                    node_id: None,
                    window_id: self.path.window_id,
                    path: self.path.path[blocked..].to_vec().into_boxed_slice(),
                },
                blocked,
                disabled: self.disabled,
            })
        } else {
            Some(self)
        }
    }

    /// Gets the [`ENABLED`] part of the path, or none if the widget is not enabled at the root.
    ///
    /// [`ENABLED`]: Interactivity::ENABLED
    pub fn enabled(self) -> Option<WidgetPath> {
        let enabled_end = self.blocked.min(self.disabled);

        if enabled_end < self.path.path.len() {
            if enabled_end == 0 {
                return None;
            }
            Some(WidgetPath {
                node_id: None,
                window_id: self.path.window_id,
                path: self.path.path[..enabled_end].to_vec().into_boxed_slice(),
            })
        } else {
            Some(self.path)
        }
    }

    /// Make a path to an ancestor id that is contained in the current path.
    pub fn ancestor_path(&self, ancestor_id: WidgetId) -> Option<Cow<InteractionPath>> {
        self.widgets_path().iter().position(|&id| id == ancestor_id).map(|i| {
            if i == self.path.path.len() - 1 {
                Cow::Borrowed(self)
            } else {
                Cow::Owned(InteractionPath {
                    path: WidgetPath {
                        node_id: None,
                        window_id: self.window_id,
                        path: self.path.path[..i].to_vec().into_boxed_slice(),
                    },
                    blocked: self.blocked,
                    disabled: self.disabled,
                })
            }
        })
    }

    /// Get the inner most widget parent shared by both `self` and `other` with the same interactivity.
    pub fn shared_ancestor<'a>(&'a self, other: &'a InteractionPath) -> Option<Cow<'a, InteractionPath>> {
        if self.window_id == other.window_id {
            if let Some(i) = self.zip().zip(other.zip()).position(|(a, b)| a != b) {
                if i == 0 {
                    None
                } else {
                    let path = self.path.path[..i].to_vec().into_boxed_slice();
                    Some(Cow::Owned(InteractionPath {
                        path: WidgetPath {
                            node_id: None,
                            window_id: self.window_id,
                            path,
                        },
                        blocked: self.blocked,
                        disabled: self.disabled,
                    }))
                }
            } else if self.path.path.len() <= other.path.path.len() {
                Some(Cow::Borrowed(self))
            } else {
                Some(Cow::Borrowed(other))
            }
        } else {
            None
        }
    }

    /// Gets a path to the root widget of this path.
    pub fn root_path(&self) -> Cow<InteractionPath> {
        if self.path.path.len() == 1 {
            Cow::Borrowed(self)
        } else {
            Cow::Owned(InteractionPath {
                path: WidgetPath {
                    node_id: None,
                    window_id: self.window_id,
                    path: Box::new([self.path.path[0]]),
                },
                blocked: self.blocked,
                disabled: self.disabled,
            })
        }
    }
}
impl ops::Deref for InteractionPath {
    type Target = WidgetPath;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}
impl From<InteractionPath> for WidgetPath {
    fn from(p: InteractionPath) -> Self {
        p.path
    }
}

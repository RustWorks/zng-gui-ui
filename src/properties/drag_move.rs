//! Properties that make a widget draggable.

use crate::core::keyboard::ModifiersState;
use crate::core::mouse::MouseButton;
use crate::prelude::new_property::*;

/// Enable widget move by pressing and moving the pointer.
#[property(outer)]
pub fn draggable(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    struct DraggableNode<C: UiNode, E: Var<bool>> {
        child: C,
        enabled: E,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for DraggableNode<C, E> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            if *self.enabled.get(ctx.vars) {
                todo!();
            }
        }
    }
    DraggableNode {
        child,
        enabled: enabled.into_var(),
    }
}

cancelable_event_args! {
    /// Drag move started.
    pub struct DragStartedArgs {
        /// Widget being dragged.
        pub target: WidgetId,

        /// How the drag operation started.
        pub source: DragEventSource,

        /// Modifiers pressed when drag started.
        pub modifiers: ModifiersState,

        ..

        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target == ctx.path.widget_id()
        }
    }

    /// Drag move on going.
    pub struct DragMovedArgs {
        /// Widget being dragged.
        pub target: WidgetId,

        /// How the drag operation started.
        pub source: DragEventSource,

        /// Modifiers currently pressed.
        pub modifiers: ModifiersState,

        /// Accumulated move since the drag started.
        pub offset: LayoutPoint,

        /// Move since previous added in this event.
        pub delta: LayoutPoint,

        ..

        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target == ctx.path.widget_id()
        }
    }
}
event_args! {
    /// Drag move completed or canceled.
    pub struct DragStoppedArgs {
        /// Widget being dragged.
        pub target: WidgetId,

        /// How the drag operation started.
        pub source: DragEventSource,

        /// Accumulated move since the drag started.
        ///
        /// This offset is now applied to the widget if not [`canceled`](Self::canceled).
        pub offset: LayoutPoint,

        /// Is some if the drag-move was canceled.
        pub canceled: Option<DragCancelSource>,

        ..

        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target == ctx.path.widget_id()
        }
    }
}

/// Source of a drag-move event.
#[derive(Debug, Clone)]
pub enum DragEventSource {
    /// Drag started by mouse press.
    Mouse {
        /// Which mouse button generated the event.
        button: MouseButton,

        /// Position of the mouse in the coordinates of [`target`](DragStartedArgs::target).
        position: LayoutPoint,
    },
    /// Drag started by a shortcut press.
    Shortcut {
        /// The shortcut.
        shortcut: Shortcut,

        /// Direction the shortcut represents.
        direction: MoveDirection,

        /// If the shortcut event was generated by holding a key pressed.
        repeat: bool,
    },
}

/// Direction of a keyboard drag move.
#[derive(Debug, Clone, Copy)]
pub enum MoveDirection {
    /// Move up.
    Up,
    /// Move down.
    Down,
    /// Move left.
    Left,
    /// Move right.
    Right,

    /// Move up and right at the same time.
    UpRight,
    /// Move up and left at the same time.
    UpLeft,
    /// Move down and right at the same time.
    DownRight,
    /// Move down and left at the same time.
    DownLeft,
}

/// What caused the drag operation to cancel.
#[derive(Debug, Clone, Copy)]
pub enum DragCancelSource {
    /// Drag canceled by a handler of [`DragStartedArgs`] calling cancel.
    DragStart,
    /// Drag canceled by a handler of [`DragMovedArgs`] calling cancel.
    DragMove,

    /// A drag cancel shortcut was pressed.
    Shortcut,

    /// Drag canceled because it required pointer capture and we lost said capture.
    LostCapture,
}

/*
# TODO

* Way to two-way bind the offset.
* Drag events, drag_started, drag_move, drag_completed.
* Cancelable.
* is_dragging, is_draggable?
* Coerce drag (can use for snapping, containing with area, dragging along single dimension).
* Capture the mouse.

Should this have an app extension?

*/

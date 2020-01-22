use super::*;
use context::Events;
pub use glutin::event::{ModifiersState, MouseButton};
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Instant;

/// [Event] arguments.
pub trait EventArgs: Debug + Clone + 'static {
    /// Gets the instant this event happen.
    fn timestamp(&self) -> Instant;
}

/// Identifies an event type.
pub trait Event: 'static {
    /// Event arguments.
    type Args: EventArgs;
}

struct EventChannelInner<T> {
    data: UnsafeCell<Vec<T>>,
    is_high_pressure: bool,
}

struct EventChannel<T: 'static> {
    r: Rc<EventChannelInner<T>>,
}
impl<T: 'static> Clone for EventChannel<T> {
    fn clone(&self) -> Self {
        EventChannel { r: Rc::clone(&self.r) }
    }
}
impl<T: 'static> EventChannel<T> {
    pub(crate) fn notify(
        self,
        new_update: T,
        _assert_events_not_borrowed: &mut Events,
        cleanup: &mut Vec<Box<dyn FnOnce()>>,
    ) {
        // SAFETY: This is safe because borrows are bound to the `Events` instance
        // so if we have a mutable reference to it no event value is borrowed.
        let data = unsafe { &mut *self.r.data.get() };
        data.push(new_update);

        if data.len() == 1 {
            // register for cleanup once
            cleanup.push(Box::new(move || {
                unsafe { &mut *self.r.data.get() }.clear();
            }))
        }
    }

    /// Gets a reference to the updates that happened in between calls of [UiNode::update].
    pub fn updates<'a>(&'a self, _events: &'a Events) -> &'a [T] {
        // SAFETY: This is safe because we are bounding the value lifetime with
        // the `Events` lifetime and we require a mutable reference to `Events` to
        // modify the value.
        unsafe { &*self.r.data.get() }.as_ref()
    }

    /// Gets if this update is notified using the [UiNode::update_hp] method.
    pub fn is_high_pressure(&self) -> bool {
        self.r.is_high_pressure
    }
}

/// Read-only reference to an event channel.
pub struct EventListener<T: 'static> {
    chan: EventChannel<T>,
}
impl<T: 'static> Clone for EventListener<T> {
    fn clone(&self) -> Self {
        EventListener {
            chan: self.chan.clone(),
        }
    }
}
impl<T: 'static> EventListener<T> {
    /// Gets a reference to the updates that happened in between calls of [UiNode::update].
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        self.chan.updates(events)
    }

    /// Gets if this update is notified using the [UiNode::update_hp] method.
    pub fn is_high_pressure(&self) -> bool {
        self.chan.is_high_pressure()
    }

    /// Listener that never updates.
    pub fn never(is_high_pressure: bool) -> Self {
        EventListener {
            chan: EventEmitter::new(is_high_pressure).chan,
        }
    }
}

/// Read-write reference to an event channel.
pub struct EventEmitter<T: 'static> {
    chan: EventChannel<T>,
}
impl<T: 'static> Clone for EventEmitter<T> {
    fn clone(&self) -> Self {
        EventEmitter {
            chan: self.chan.clone(),
        }
    }
}
impl<T: 'static> EventEmitter<T> {
    /// New event emitter.
    ///
    /// # Arguments
    /// * `is_high_pressure`: If this event is notified using the [UiNode::update_hp] method.
    pub fn new(is_high_pressure: bool) -> Self {
        EventEmitter {
            chan: EventChannel {
                r: Rc::new(EventChannelInner {
                    data: UnsafeCell::default(),
                    is_high_pressure,
                }),
            },
        }
    }

    /// Gets a reference to the updates that happened in between calls of [UiNode::update].
    pub fn updates<'a>(&'a self, events: &'a Events) -> &'a [T] {
        self.chan.updates(events)
    }

    /// Gets if this event is notified using the [UiNode::update_hp] method.
    pub fn is_high_pressure(&self) -> bool {
        self.chan.is_high_pressure()
    }

    /// Gets a new event listener linked with this emitter.
    pub fn listener(&self) -> EventListener<T> {
        EventListener {
            chan: self.chan.clone(),
        }
    }

    pub(crate) fn notify(
        self,
        new_update: T,
        assert_events_not_borrowed: &mut Events,
        cleanup: &mut Vec<Box<dyn FnOnce()>>,
    ) {
        self.chan.notify(new_update, assert_events_not_borrowed, cleanup);
    }
}

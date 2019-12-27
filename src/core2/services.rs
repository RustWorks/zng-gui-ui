use super::{
    AppContext, AppExtension, AppRegister, Event, EventArgs, EventEmitter, EventListener, Service, UiRoot, WindowId,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

/// New window event.
pub struct NewWindow;

/// [NewWindow] event args.
#[derive(Debug, Clone)]
pub struct NewWindowArgs {
    pub timestamp: Instant,
    pub window_id: WindowId,
}

impl EventArgs for NewWindowArgs {
    fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

impl Event for NewWindow {
    type Args = NewWindowArgs;
}

pub(crate) struct WindowsExt {
    service: Windows,
    new_window: EventEmitter<NewWindowArgs>,
}

impl Default for WindowsExt {
    fn default() -> Self {
        WindowsExt {
            service: Windows::default(),
            new_window: EventEmitter::new(false),
        }
    }
}

impl AppExtension for WindowsExt {
    fn register(&mut self, r: &mut AppRegister) {
        r.register_service::<Windows>(self.service.clone());
        r.register_event::<NewWindow>(self.new_window.listener());
    }
}

struct NewWindowRequest {
    new: Box<dyn FnOnce(&mut AppContext) -> UiRoot>,
    notifier: EventEmitter<NewWindowArgs>,
}

/// Windows service.
#[derive(Clone, Default)]
pub struct Windows {
    requests: Rc<RefCell<Vec<NewWindowRequest>>>,
}

impl Service for Windows {}

impl Windows {
    /// Requests a new window. Returns a notice that gets updated once
    /// when the window is launched.
    pub fn new_window(
        &self,
        new_window: impl FnOnce(&mut AppContext) -> UiRoot + 'static,
    ) -> EventListener<NewWindowArgs> {
        let request = NewWindowRequest {
            new: Box::new(new_window),
            notifier: EventEmitter::new(false),
        };
        let notice = request.notifier.listener();
        self.requests.borrow_mut().push(request);
        notice
    }
}

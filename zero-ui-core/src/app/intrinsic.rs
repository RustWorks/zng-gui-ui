use std::fmt;

use crate::app::*;
use crate::event::*;
use crate::shortcut;
use crate::var::*;

pub(super) struct AppIntrinsic {
    #[allow(dead_code)]
    exit_handle: CommandHandle,
    pending_exit: Option<PendingExit>,
}
struct PendingExit {
    handle: EventPropagationHandle,
    response: ResponderVar<ExitCancelled>,
}
impl AppIntrinsic {
    /// Pre-init intrinsic services and commands, must be called before extensions init.
    pub(super) fn pre_init(is_headed: bool, with_renderer: bool, view_process_exe: Option<PathBuf>, device_events: bool) -> Self {
        if is_headed {
            debug_assert!(with_renderer);

            let view_evs_sender = UPDATES.sender();
            VIEW_PROCESS.start(view_process_exe, device_events, false, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
        } else if with_renderer {
            let view_evs_sender = UPDATES.sender();
            VIEW_PROCESS.start(view_process_exe, false, true, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
        }

        AppIntrinsic {
            exit_handle: EXIT_CMD.subscribe(true),
            pending_exit: None,
        }
    }

    /// Returns if exit was requested and not cancelled.
    pub(super) fn exit(&mut self) -> bool {
        if let Some(pending) = self.pending_exit.take() {
            if pending.handle.is_stopped() {
                pending.response.respond(ExitCancelled);
                false
            } else {
                true
            }
        } else {
            false
        }
    }
}
impl AppExtension for AppIntrinsic {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = EXIT_CMD.on(update) {
            args.handle_enabled(&self.exit_handle, |_| {
                APP_PROCESS.exit();
            });
        }
    }

    fn update(&mut self) {
        if let Some(response) = APP_PROCESS_SV.write().take_requests() {
            let args = ExitRequestedArgs::now();
            self.pending_exit = Some(PendingExit {
                handle: args.propagation().clone(),
                response,
            });
            EXIT_REQUESTED_EVENT.notify(args);
        }
    }
}

app_local! {
    pub(super) static APP_PROCESS_SV: AppProcessService = const {
        AppProcessService {
            exit_requests: None,
        }
    };
}

pub(super) struct AppProcessService {
    exit_requests: Option<ResponderVar<ExitCancelled>>,
}
impl AppProcessService {
    pub(super) fn take_requests(&mut self) -> Option<ResponderVar<ExitCancelled>> {
        self.exit_requests.take()
    }

    fn exit(&mut self) -> ResponseVar<ExitCancelled> {
        if let Some(r) = &self.exit_requests {
            r.response_var()
        } else {
            let (responder, response) = response_var();
            self.exit_requests = Some(responder);
            UPDATES.update(None);
            response
        }
    }
}

/// Service for managing the application process.
///
/// This service is available in all apps.
#[allow(non_camel_case_types)]
pub struct APP_PROCESS;
impl APP_PROCESS {
    /// Register a request for process exit with code `0` in the next update.
    ///
    /// The [`EXIT_REQUESTED_EVENT`] will be raised, and if not cancelled the app process will exit.
    ///
    /// Returns a response variable that is updated once with the unit value [`ExitCancelled`]
    /// if the exit operation is cancelled.
    ///
    /// See also the [`EXIT_CMD`] that also causes an exit request.
    pub fn exit(&self) -> ResponseVar<ExitCancelled> {
        APP_PROCESS_SV.write().exit()
    }
}

command! {
    /// Represents the app process [`exit`] request.
    ///
    /// [`exit`]: APP_PROCESS::exit
    pub static EXIT_CMD = {
        name: "Exit",
        info: "Close all windows and exit.",
        shortcut: shortcut!(Exit),
    };
}

/// Cancellation message of an [exit request].
///
/// [exit request]: APP_PROCESS::exit
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitCancelled;
impl fmt::Display for ExitCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exit request cancelled")
    }
}

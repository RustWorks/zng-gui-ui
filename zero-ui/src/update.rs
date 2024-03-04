//! App update service and other types.
//!
//! The [`UPDATES`] service can execute arbitrary futures and setup update handlers. It can also be used to request update,
//! info rebuild, layout and render for any widget. Note that from inside the widget you should use the [`WIDGET`] service instead,
//! as it is more efficient.
//!
//! The example below setups a handler that is called every app update.
//!
//! ```
//! use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//!
//! zero_ui::update::UPDATES
//!   .on_pre_update(app_hn!(|args: &zero_ui::update::UpdateArgs, _| {
//!       println!("pre_update #{}", args.count);
//!   }))
//!   .perm();
//! ```
//!
//! Updates are coalesced, multiple requests for the same widget will cause it to only update once, and multiple widgets
//! can update on the same pass. See the [Main Loop] docs in the `app` module for more details.
//!
//! [`WIDGET`]: crate::widget::WIDGET
//! [Main Loop]: crate::app#main-loop
//!
//! # Full API
//!
//! See [`zero_ui_app::update`] for the full update API.

pub use zero_ui_app::update::{
    ContextUpdates, EventUpdate, InfoUpdates, LayoutUpdates, OnUpdateHandle, RenderUpdates, UpdateArgs, UpdateDeliveryList, UpdateOp,
    UpdateSubscribers, UpdatesTraceUiNodeExt, WeakOnUpdateHandle, WidgetUpdates, UPDATES,
};

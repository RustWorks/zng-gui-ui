//! Command events.
//!
//! Commands are [events](Event) that represent app actions.

/*!
<script>
// hide re-exported `self`. We need to `pub use crate::command;` to inline the macro
// but that the path to the `command` module too.
document.addEventListener('DOMContentLoaded', function() {
    var macros = document.getElementById('modules');
    macros.nextElementSibling.remove();
    macros.remove();

    var side_bar_anchor = document.querySelector("li a[href='#modules']").remove();
 })
</script>
 */

use std::{
    any::{type_name, Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    thread::LocalKey,
    time::Instant,
};

use crate::{
    context::{InfoContext, OwnedStateMap, StateId, StateMapMut, StateValue, UpdatesTrace, WidgetContext, WidgetContextMut, WindowContext},
    crate_util::{Handle, HandleOwner},
    event::{Event, EventPropagationHandle, Events, WithEvents},
    handler::WidgetHandler,
    impl_ui_node,
    text::{Text, ToText},
    var::{types::ReadOnlyVar, *},
    widget_info::{EventSlot, WidgetInfoBuilder, WidgetSubscriptions},
    window::WindowId,
    UiNode, WidgetId,
};

/// <span data-del-macro-root></span> Declares new [`Command`] types.
///
/// The macro generates a unit `struct` that implements [`Event`] with arguments type [`CommandArgs`] and implements [`Command`].
/// The most used methods of [`Event`] and [`Command`] are also *re-exported* as associated methods.
///
/// # Conventions
///
/// Command types have the `Command` suffix, for example a command for the clipboard *copy* action is called `CopyCommand`.
/// Public and user facing commands also set the [`CommandNameExt`] and [`CommandInfoExt`] with localized display text.
///
/// # Shortcuts
///
/// You can give commands one or more shortcuts using the [`CommandShortcutExt`], the [`GestureManager`] notifies commands
/// that match a pressed shortcut automatically.
///
/// # Examples
///
/// Declare two commands:
///
/// ```
/// use zero_ui_core::command::command;
///
/// command! {
///     /// Command docs.
///     pub FooCommand;
///
///     pub(crate) BarCommand;
/// }
/// ```
///
/// You can also initialize metadata:
///
/// ```
/// use zero_ui_core::{command::{command, CommandNameExt, CommandInfoExt}, gesture::{CommandShortcutExt, shortcut}};
///
/// command! {
///     /// Represents the **foo** action.
///     ///
///     /// # Metadata
///     ///
///     /// This command initializes with the following metadata:
///     ///
///     /// | metadata     | value                             |
///     /// |--------------|-----------------------------------|
///     /// | [`name`]     | "Foo!"                            |
///     /// | [`info`]     | "Does the foo! thing."            |
///     /// | [`shortcut`] | `CTRL+F`                          |
///     ///
///     /// [`name`]: CommandNameExt
///     /// [`info`]: CommandInfoExt
///     /// [`shortcut`]: CommandShortcutExt
///     pub FooCommand
///         .init_name("Foo!")
///         .init_info("Does the foo! thing.")
///         .init_shortcut(shortcut!(CTRL+F));
/// }
/// ```
///
/// The initialization uses the [command extensions] pattern and runs once for each app, so usually just once.
///
/// [`Command`]: crate::command::Command
/// [`CommandArgs`]: crate::command::CommandArgs
/// [`CommandNameExt`]: crate::command::CommandNameExt
/// [`CommandInfoExt`]: crate::command::CommandInfoExt
/// [`CommandShortcutExt`]: crate::gesture::CommandShortcutExt
/// [`GestureManager`]: crate::gesture::GestureManager
/// [`Event`]: crate::event::Event
/// [command extensions]: crate::command::Command#extensions
#[macro_export]
macro_rules! command {
    ($(
        $(#[$outer:meta])*
        $vis:vis $Command:ident $(
                 .$init:ident( $($args:tt)* )
        )*;
    )+) => {$(

        $(#[$outer])*
        #[derive(Clone, Copy, Debug)]
        $vis struct $Command;
        impl $Command {
            std::thread_local! {
                static COMMAND: $crate::command::CommandValue = $crate::command::CommandValue::init($Command, ||{
                    #[allow(path_statements)] {
                        $Command $(
                        .$init( $($args)* )
                        )*;
                    }
                });
            }

            /// Gets the event arguments if the update is for this command type and scope.
            #[allow(unused)]
            pub fn update<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<$Command>> {
                if let Some(args) = args.args_for::<Self>() {
                    if args.scope == $crate::command::CommandScope::App {
                        Some(args)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }

            /// Gets the event arguments if the update is for this command type disregarding the scope.
            #[allow(unused)]
            pub fn update_any_scope<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<$Command>> {
                args.args_for::<Self>()
            }

            /// Schedule an event update if the command has handlers, enabled or disabled.
            ///
            /// The `parameter` is an optional value for the command handler.
            ///
            /// Returns `true` if notified.
            #[allow(unused)]
            pub fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, parameter: Option<$crate::command::CommandParam>) -> bool {
                let scope = $crate::command::Command::scope(self);
                if let Some(enabled) = Self::COMMAND.with(move |c| c.enabled_value(scope)) {
                    events.with_events(|evs| {
                        evs.notify($Command, $crate::command::CommandArgs::now(parameter, $crate::command::Command::scope(self), enabled))
                    });

                    true
                } else {
                    false
                }
            }

            /// Gets a read-only variable that indicates if the command has at least one enabled handler.
            ///
            /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
            /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
            /// visible but disabled.
            #[allow(unused)]
            pub fn enabled(self) -> $crate::var::ReadOnlyRcVar<bool> {
                <Self as $crate::command::Command>::enabled(self)
            }

            /// Gets a read-only variable that indicates if the command has at least one handler.
            ///
            /// When this is `false` the command can be considered *not relevant* in the current app state
            /// and associated command trigger widgets can be hidden.
            #[allow(unused)]
            pub fn has_handlers(self) -> $crate::var::types::ReadOnlyRcVar<bool> {
                <Self as $crate::command::Command>::has_handlers(self)
            }

            /// Create a new handle to this command.
            ///
            /// A handle indicates that there is an active *handler* for the event, the handle can also
            /// be used to set the [`enabled`](Self::enabled) state.
            #[allow(unused)]
            pub fn new_handle<Evs: $crate::event::WithEvents>(self, events: &mut Evs, enabled: bool) -> $crate::command::CommandHandle {
                <Self as $crate::command::Command>::new_handle(self, events, enabled)
            }

            /// Get a scoped command derived from this command type.
            #[allow(unused)]
            pub fn scoped<S: Into<$crate::command::CommandScope>>(self, scope: S) -> $crate::command::ScopedCommand<Self> {
                <Self as $crate::command::Command>::scoped(self, scope)
            }
        }
        impl $crate::event::Event for $Command {
            type Args = $crate::command::CommandArgs;


            fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, args: Self::Args) {
                let scope = $crate::command::Command::scope(self);
                if Self::COMMAND.with(move |c| c.enabled_value(scope).is_some()) {
                    events.with_events(|evs| evs.notify($Command, args));
                }
            }

            fn update<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<Self>> {
                self.update(args)
            }

            fn slot(self) -> $crate::widget_info::EventSlot {
                Self::COMMAND.with(move |c| c.slot())
            }
        }
        impl $crate::command::Command for $Command {
            type AppScopeCommand = Self;


            fn thread_local_value(self) -> &'static std::thread::LocalKey<$crate::command::CommandValue> {
                &Self::COMMAND
            }


            fn scoped<S: Into<$crate::command::CommandScope>>(self, scope: S) ->  $crate::command::ScopedCommand<Self> {
                $crate::command::ScopedCommand{ command: self, scope: scope.into() }
            }

            fn notify_cmd<Evs: $crate::event::WithEvents>(self, events: &mut Evs, parameter: Option<$crate::command::CommandParam>) -> bool {
                self.notify(events, parameter)
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::command;

/// Identifies a command type.
///
/// Use the [`command!`] to declare command types, it declares command types with optional
/// [metadata](#metadata) initialization.
///
/// ```
/// # use zero_ui_core::command::*;
/// # pub trait CommandFooBarExt: Sized { fn init_foo(self, foo: bool) -> Self { self } fn init_bar(self, bar: bool) -> Self { self } }
/// # impl<C: Command> CommandFooBarExt for C { }
/// command! {
///     /// Foo-bar command.
///     pub FooBarCommand
///         .init_foo(false)
///         .init_bar(true);
/// }
/// ```
///
/// # Metadata
///
/// Commands can have metadata associated with then, this metadata is extendable and can be used to enable
/// command features such as command shortcuts. The metadata can be accessed using [`with_meta`], metadata
/// extensions are implemented using extension traits. See [`CommandMeta`] for more details.
///
/// # Handles
///
/// Unlike other events, commands only notify if it has at least one handler, handlers
/// must call [`new_handle`] to indicate that the command is relevant to the current app state and
/// [set its enabled] flag to indicate that the handler can fulfill command requests.
///
/// Properties that setup a handler for a command event should do this automatically and are usually
/// paired with a *can_foo* context property that sets the enabled flag. You can use [`on_command`]
/// to declare command handler properties.
///
/// # Scopes
///
/// Commands are *global* by default, meaning an enabled handle anywhere in the app enables it everywhere.
/// You can call [`scoped`] to declare *sub-commands* that are new commands that represent a command type in a limited
/// scope only, See [`ScopedCommand<C>`] for details.
///
/// [`command!`]: macro@crate::command::command
/// [`new_handle`]: Command::new_handle
/// [set its enabled]: CommandHandle::set_enabled
/// [`with_meta`]: Command::with_meta
/// [`scoped`]: Command::scoped
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Command: Event<Args = CommandArgs> {
    /// The root command type.
    ///
    /// This should be `Self` by default, and will be once [this] is stable.
    ///
    /// [this]: https://github.com/rust-lang/rust/issues/29661
    #[doc(hidden)]
    type AppScopeCommand: Command;

    /// Thread-local storage for command.
    #[doc(hidden)]
    fn thread_local_value(self) -> &'static LocalKey<CommandValue>;

    /// Runs `f` with access to the metadata state-map. The first map is the root command map,
    /// the second optional map is the scoped command map.
    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.with_meta(f, scope))
    }

    /// Gets a read-only variable that indicates if the command has at least one enabled handler.
    ///
    /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
    /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
    /// visible but disabled.
    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.enabled(scope))
    }

    /// Gets if the command has at least one handler, enabled or disabled.
    fn enabled_value(self) -> Option<bool> {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.enabled_value(scope))
    }

    /// Gets a read-only variable that indicates if the command has at least one handler.
    ///
    /// When this is `false` the command can be considered *not relevant* in the current app state
    /// and associated command trigger widgets can be hidden.
    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.has_handlers(scope))
    }

    /// Gets if the command has at least one handler.
    fn has_handlers_value(self) -> bool {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.has_handlers_value(scope))
    }

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`enabled`](Self::enabled) state.
    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        let tl = self.thread_local_value();
        let scope = self.scope();
        tl.with(move |c| c.new_handle(events, tl, scope, enabled))
    }

    /// Gets a [`AnyCommand`] that represents this command.
    fn as_any(self) -> AnyCommand {
        AnyCommand(self.thread_local_value(), self.scope())
    }

    /// The scope the command applies too.
    ///
    /// Scoped commands represent "a command in a scope" as a new command.
    ///
    /// The default value is [`CommandScope::App`].
    fn scope(self) -> CommandScope {
        CommandScope::App
    }

    /// Get a scoped command derived from this command type.
    ///
    /// Returns a new command that represents the command type in the `scope`.
    /// See [`ScopedCommand`] for details.
    fn scoped<S: Into<CommandScope>>(self, scope: S) -> ScopedCommand<Self::AppScopeCommand>;

    /// Schedule an event update if the command has handlers, enabled or disabled.
    ///
    /// The `parameter` is an optional value for the command handler.
    ///
    /// Returns `true` if notified.
    fn notify_cmd<Evs: WithEvents>(self, events: &mut Evs, parameter: Option<CommandParam>) -> bool;
}

/// Represents the scope of a [`Command`].
///
/// See [`ScopedCommand<C>`] for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandScope {
    /// Default scope, this is the scope of command types declared using [`command!`].
    App,
    /// Scope of a window.
    Window(WindowId),
    /// Scope of a widget.
    Widget(WidgetId),
}
impl From<WidgetId> for CommandScope {
    fn from(id: WidgetId) -> Self {
        CommandScope::Widget(id)
    }
}
impl From<WindowId> for CommandScope {
    fn from(id: WindowId) -> CommandScope {
        CommandScope::Window(id)
    }
}
impl<'a> From<&'a WidgetContext<'a>> for CommandScope {
    /// Widget scope from the `ctx.path.widget_id()`.
    fn from(ctx: &'a WidgetContext<'a>) -> Self {
        CommandScope::Widget(ctx.path.widget_id())
    }
}
impl<'a> From<&'a WindowContext<'a>> for CommandScope {
    /// Window scope from the `ctx.window_id`.
    fn from(ctx: &'a WindowContext<'a>) -> CommandScope {
        CommandScope::Window(*ctx.window_id)
    }
}
impl<'a> From<&'a WidgetContextMut> for CommandScope {
    /// Widget scope from the `ctx.widget_id()`.
    fn from(ctx: &'a WidgetContextMut) -> Self {
        CommandScope::Widget(ctx.widget_id())
    }
}

/// A command that is a command type in a scope.
///
/// Normal commands apply globally, if there is a handler enabled in any context the status
/// variables indicate its availability. You can use [`Command::scoped`] to change this by
/// creating a new *command* that represents a command type in a *scope* only. The scope can
/// be any of the [`CommandScope`] values.
///
/// # Examples
///
/// Get the a command type scoped to a window:
///
/// ```
/// # use zero_ui_core::{command::*, context::*};
/// # command! { pub FooCommand; }
/// # struct FooNode { cmd: ScopedCommand<FooCommand> }
/// # impl FooNode {
/// fn init(&mut self, ctx: &mut WindowContext) {
///     self.cmd = FooCommand.scoped(*ctx.window_id);
/// }
/// # }
/// ```
///
/// # Enabled & Has Handlers
///
/// The [`enabled`] and [`has_handlers`] variables are only `true` when there is
/// a handler created using the same scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*, handler::*, var::*, units::*};
/// # command! { pub FooCommand; }
/// # TestWidgetContext::doc_test((),
/// async_hn!(|mut ctx, _| {
///     let cmd = FooCommand;
///     let cmd_scoped = cmd.scoped(ctx.window_id());
///
///     let enabled = cmd.enabled();
///     let enabled_scoped = cmd_scoped.enabled();
///
///     let handle = cmd_scoped.new_handle(&mut ctx, true);
///     ctx.update().await;
///
///     assert!(!enabled.copy(&ctx));
///     assert!(enabled_scoped.copy(&ctx));
/// })
/// # );
/// ```
///
/// In the example above, only the `enabled_scoped` is `true` after only the `cmd_scoped` is enabled.
///
/// # Metadata
///
/// Metadata is *inherited* from the [not scoped] command type but can be overwritten for the scoped command
/// only, so you can rename or give a different shortcut for the command only in the scope.
///
/// ```
/// # use zero_ui_core::{var::*, command::*, handler::*, context::*};
/// # command! { pub FooCommand; }
/// # TestWidgetContext::doc_test((),
/// async_hn!(|ctx, _| {
///     let cmd = FooCommand;
///     let cmd_scoped = FooCommand.scoped(ctx.window_id());
///
///     // same initial value:
///     assert_eq!(cmd.name().get_clone(&ctx), cmd_scoped.name().get_clone(&ctx));
///     
///     // set a name for all commands, including scoped not overridden:
///     cmd.name().set(&ctx, "Foo!");
///     ctx.update().await;
///     assert_eq!("Foo!", cmd_scoped.name().get_clone(&ctx));
///
///     // name is overridden in the scoped command only:
///     cmd_scoped.name().set(&ctx, "Scoped Only!");
///     ctx.update().await;
///     assert_eq!("Scoped Only!", cmd_scoped.name().get_clone(&ctx));
///     assert_eq!("Foo!", cmd.name().get_clone(&ctx));
///
///     // scoped command no-longer affected:
///     cmd.name().set(&ctx, "F");
///     ctx.update().await;
///     assert_eq!("F", cmd.name().get_clone(&ctx));
///     assert_eq!("Scoped Only!", cmd_scoped.name().get_clone(&ctx));
/// })
/// # );
/// ```
///
/// See [`CommandMetaVar<T>`] for details of how this is implemented.
///
/// # Notify
///
/// Calling [`notify`] from a scoped command **notifies the base type** but sets the [`CommandArgs::scope`]
/// the event will be handled by handlers for the same scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*};
/// # command! { pub FooCommand; }
/// # fn init(ctx: &mut WindowContext) {
/// let notified = FooCommand.scoped(*ctx.window_id).notify(ctx, None);
/// # }  
/// ```
///
/// In the example above `notified` is `true` only if there are any handlers for the same scope.
///
/// # Update
///
/// Calling [`update`] from a command detects updates for the same command type if the [`CommandArgs::scope`]
/// is equal to the command scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*, event::*};
/// # command! { pub FooCommand; }
/// # struct FooNode;
/// # impl FooNode {
/// fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
///     if let Some(args) = FooCommand.scoped(ctx.path.window_id()).update(args) {
///         println!("{:?}", args.scope);
///     }
/// }
/// # }
/// ```
///
/// The example will print only for commands on the scope of [`CommandScope::Window`] with the same id.
///
/// # App Scope
///
/// It is possible to create a scoped command using the [`App`] scope. In this
/// case the scoped command behaves exactly like a default command type.
///
/// [`enabled`]: ScopedCommand::enabled
/// [`notify`]: ScopedCommand::notify
/// [`update`]: ScopedCommand::update
/// [`has_handlers`]: ScopedCommand::has_handlers
/// [`App`]: CommandScope::App
/// [`name`]: CommandNameExt::name
#[derive(Debug, Clone, Copy)]
pub struct ScopedCommand<C: Command> {
    /// Base command type.
    pub command: C,

    /// Command scope.
    pub scope: CommandScope,
}
impl<C: Command> ScopedCommand<C> {
    /// Gets a read-only variable that indicates if the command has at least one enabled handler in the scope.
    ///
    /// You can use this in a notifier widget that *knows* the limited scope it applies too, unlike the general
    /// enabled, the widget will only enable if there is an active handler in the scope.
    #[allow(unused)]
    pub fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        <Self as Command>::enabled(self)
    }

    /// Gets a read-only variable that indicates if the command has at least one handler in the scope.
    #[allow(unused)]
    pub fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        <Self as Command>::has_handlers(self)
    }

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`enabled`](Self::enabled) state.
    #[allow(unused)]
    pub fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        <Self as Command>::new_handle(self, events, enabled)
    }

    /// Schedule an event update if the command has handlers, enabled or disabled.
    ///
    /// The event type notified is the `C` type, not `Self`. The scope is passed in the [`CommandArgs`].
    ///
    /// The `parameter` is an optional value for the command handler.
    ///
    /// Returns `true` if notified.
    pub fn notify<Evs: WithEvents>(self, events: &mut Evs, parameter: Option<CommandParam>) -> bool {
        let scope = self.scope();
        if let Some(enabled) = self.thread_local_value().with(move |c| c.enabled_value(scope)) {
            events.with_events(|evs| evs.notify(self.command, CommandArgs::now(parameter, self.scope, enabled)));
            true
        } else {
            false
        }
    }

    /// Gets the event arguments if the update is for this command type and scope.
    ///
    /// Returns `Some(args)` if the event type is the `C` type, and the [`CommandArgs::scope`] is equal.
    pub fn update<U: crate::event::EventUpdateArgs>(self, args: &U) -> Option<&crate::event::EventUpdate<C>> {
        if let Some(args) = args.args_for::<C>() {
            if args.scope == self.scope {
                Some(args)
            } else {
                None
            }
        } else {
            None
        }
    }
}
impl<C: Command> Event for ScopedCommand<C> {
    type Args = CommandArgs;

    fn notify<Evs: WithEvents>(self, events: &mut Evs, args: Self::Args) {
        if self.enabled_value().is_some() {
            events.with_events(|events| events.notify(self.command, args));
        }
    }

    fn update<U: crate::event::EventUpdateArgs>(self, args: &U) -> Option<&crate::event::EventUpdate<Self>> {
        self.update(args).map(|a| a.transmute_event::<Self>())
    }

    fn slot(self) -> EventSlot {
        self.thread_local_value().with(move |c| c.slot())
    }
}
impl<C: Command> Command for ScopedCommand<C> {
    type AppScopeCommand = C;

    fn thread_local_value(self) -> &'static LocalKey<CommandValue> {
        self.command.thread_local_value()
    }

    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.with_meta(f, scope))
    }

    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.enabled(scope))
    }

    fn enabled_value(self) -> Option<bool> {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.enabled_value(scope))
    }

    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.has_handlers(scope))
    }

    fn has_handlers_value(self) -> bool {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.has_handlers_value(scope))
    }

    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        let key = self.command.thread_local_value();
        let scope = self.scope;
        key.with(move |c| c.new_handle(events, key, scope, enabled))
    }

    fn scope(self) -> CommandScope {
        self.scope
    }

    fn scoped<S: Into<CommandScope>>(self, scope: S) -> ScopedCommand<C> {
        ScopedCommand {
            command: self.command,
            scope: scope.into(),
        }
    }

    fn notify_cmd<Evs: WithEvents>(self, events: &mut Evs, parameter: Option<CommandParam>) -> bool {
        self.notify(events, parameter)
    }

    fn as_any(self) -> AnyCommand {
        let mut any = self.command.as_any();
        any.1 = self.scope;
        any
    }
}

/// Represents a reference counted `dyn Any` object.
#[derive(Clone)]
pub struct CommandParam(pub Rc<dyn Any>);
impl CommandParam {
    /// New param.
    pub fn new(param: impl Any + 'static) -> Self {
        CommandParam(Rc::new(param))
    }

    /// Gets the [`TypeId`] of the parameter.
    pub fn type_id(&self) -> TypeId {
        self.0.type_id()
    }

    /// Gets a typed reference to the parameter if it is of type `T`.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    /// Returns `true` if the parameter type is `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.0.is::<T>()
    }
}
impl fmt::Debug for CommandParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CommandParam").field(&self.0.type_id()).finish()
    }
}

/// Represents a [`Command`] type.
#[derive(Clone, Copy)]
pub struct AnyCommand(&'static LocalKey<CommandValue>, CommandScope);
impl AnyCommand {
    #[doc(hidden)]
    pub fn new(c: &'static LocalKey<CommandValue>, scope: CommandScope) -> Self {
        AnyCommand(c, scope)
    }

    pub(crate) fn update_state(&self, vars: &Vars) {
        let scope = self.1;
        self.0.with(|c| c.update_state(vars, scope))
    }

    pub(crate) fn on_exit(&self) {
        self.0.with(|c| c.on_exit());
    }

    /// Gets the [`TypeId`] of the command represented by `self`.
    pub fn command_type_id(self) -> TypeId {
        self.0.with(|c| c.command_type_id)
    }

    /// Gets the scope of the command represented by `self`.
    pub fn scope(self) -> CommandScope {
        self.1
    }

    /// Gets the [`type_name`] of the command represented by `self`.
    pub fn command_type_name(self) -> &'static str {
        self.0.with(|c| c.command_type_name)
    }

    /// If the command `C` is represented by `self`.
    pub fn is<C: Command>(self) -> bool {
        self.command_type_id() == TypeId::of::<C>()
    }

    /// Schedule an event update if the command has handlers, enabled or disabled.
    ///
    /// The event type notified is the inner command type, the scope is passed in the [`CommandArgs`].
    ///
    /// The `parameter` is an optional value for the command handler.
    ///
    /// Returns `true` if notified.
    pub fn notify<Evs: WithEvents>(self, events: &mut Evs, parameter: Option<CommandParam>) -> bool {
        if let Some(enabled) = self.0.with(|c| c.enabled_value(self.1)) {
            events.with_events(|events| Event::notify(self, events, CommandArgs::now(parameter, self.1, enabled)));
            true
        } else {
            false
        }
    }

    /// Schedule an event update if the command is enabled, linked with the external `propagation`.
    pub fn notify_linked<Evs: WithEvents>(
        self,
        events: &mut Evs,
        parameter: Option<CommandParam>,
        propagation: &EventPropagationHandle,
    ) -> bool {
        if let Some(enabled) = self.0.with(|c| c.enabled_value(self.1)) {
            events.with_events(|events| {
                Event::notify(
                    self,
                    events,
                    CommandArgs::new(Instant::now(), propagation.clone(), parameter, self.1, enabled),
                )
            });

            true
        } else {
            false
        }
    }
}
impl fmt::Debug for AnyCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "any {}:{:?}", self.command_type_name(), self.scope())
    }
}
impl Event for AnyCommand {
    type Args = CommandArgs;

    fn notify<Evs: WithEvents>(self, events: &mut Evs, args: Self::Args) {
        let scope = self.1;
        self.0.with(move |c| {
            if c.enabled_value(scope).is_some() {
                events.with_events(|e| (c.notify)(e, args))
            }
        });
    }
    fn update<U: crate::event::EventUpdateArgs>(self, _: &U) -> Option<&crate::event::EventUpdate<Self>> {
        // TODO use a closure in the value and then transmute to Self?
        panic!("`AnyCommand` does not support `Event::update`");
    }

    fn slot(self) -> EventSlot {
        self.0.with(|c| c.slot)
    }
}

impl Command for AnyCommand {
    type AppScopeCommand = Self;

    fn thread_local_value(self) -> &'static LocalKey<CommandValue> {
        self.0
    }

    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        let scope = self.1;
        self.0.with(move |c| c.with_meta(f, scope))
    }

    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.1;
        self.0.with(move |c| c.enabled(scope))
    }

    fn enabled_value(self) -> Option<bool> {
        let scope = self.1;
        self.0.with(move |c| c.enabled_value(scope))
    }

    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.1;
        self.0.with(move |c| c.has_handlers(scope))
    }

    fn has_handlers_value(self) -> bool {
        let scope = self.1;
        self.0.with(move |c| c.has_handlers_value(scope))
    }

    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        let key = self.0;
        let scope = self.1;
        key.with(move |c| c.new_handle(events, key, scope, enabled))
    }

    fn as_any(self) -> AnyCommand {
        self
    }

    fn scope(self) -> CommandScope {
        self.1
    }

    fn notify_cmd<Evs: WithEvents>(self, events: &mut Evs, parameter: Option<CommandParam>) -> bool {
        self.notify(events, parameter)
    }

    fn scoped<S: Into<CommandScope>>(self, scope: S) -> ScopedCommand<Self> {
        ScopedCommand {
            command: self,
            scope: scope.into(),
        }
    }
}

unique_id_64! {
    /// Unique identifier of a command metadata state variable.
    ///
    /// This type is very similar to [`StateId`], but `T` is the value type of the metadata variable.
    pub struct CommandMetaVarId<T: (StateValue + VarValue)>: StateId;
}
impl<T: StateValue + VarValue> CommandMetaVarId<T> {
    fn app(self) -> StateId<RcVar<T>> {
        let id = self.get();
        // SAFETY:
        // id: We "inherit" from `StateId` so there is no repeated IDs.
        // type: only our private code can get this ID and we only use it in the app level state-map.
        unsafe { StateId::from_raw(id) }
    }

    fn scope(self) -> StateId<RcCowVar<T, RcVar<T>>> {
        let id = self.get();
        // SAFETY:
        // id: We "inherit" from `StateId` so there is no repeated IDs.
        // type: only our private code can get this ID and we only use it in the scope level state-map.
        unsafe { StateId::from_raw(id) }
    }
}

impl<T: StateValue + VarValue> fmt::Debug for CommandMetaVarId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(debug_assertions)]
        let t = std::any::type_name::<T>();
        #[cfg(not(debug_assertions))]
        let t = "$T";

        if f.alternate() {
            writeln!(f, "CommandMetaVarId<{t} {{")?;
            writeln!(f, "   id: {},", self.get())?;
            writeln!(f, "   sequential: {}", self.sequential())?;
            writeln!(f, "}}")
        } else {
            write!(f, "CommandMetaVarId<{t}>({})", self.sequential())
        }
    }
}

/// Access to metadata of a command.
///
/// The metadata storage can be accessed using the [`Command::with_meta`]
/// method, you should declare and extension trait that adds methods that return [`CommandMetaVar`] or
/// [`ReadOnlyCommandMetaVar`] that are stored in the [`CommandMeta`]. An initialization builder method for
/// each value also must be provided to integrate with the [`command!`] macro.
///
/// # Examples
///
/// ```
/// use zero_ui_core::{command::*, var::*};
///
/// static COMMAND_FOO_ID: StaticCommandMetaVarId<bool> = StaticCommandMetaVarId::new_unique();
/// static COMMAND_BAR_ID: StaticCommandMetaVarId<bool> = StaticCommandMetaVarId::new_unique();
///
/// /// FooBar command values.
/// pub trait CommandFooBarExt {
///     /// Gets read/write *foo*.
///     fn foo(self) -> CommandMetaVar<bool>;
///
///     /// Gets read-only *bar*.
///     fn bar(self) -> ReadOnlyCommandMetaVar<bool>;
///
///     /// Gets a read-only var derived from other metadata.
///     fn foo_and_bar(self) -> BoxedVar<bool>;
///
///     /// Init *foo*.
///     fn init_foo(self, foo: bool) -> Self;
///
///     /// Init *bar*.
///     fn init_bar(self, bar: bool) -> Self;
/// }
///
/// impl<C: Command> CommandFooBarExt for C {
///     fn foo(self) -> CommandMetaVar<bool> {
///         self.with_meta(|m| m.get_var_or_default(&COMMAND_FOO_ID))
///     }
///
///     fn bar(self) -> ReadOnlyCommandMetaVar<bool> {
///         self.with_meta(|m| m.get_var_or_insert(&COMMAND_BAR_ID, ||true)).into_read_only()
///     }
///
///     fn foo_and_bar(self) -> BoxedVar<bool> {
///         merge_var!(self.foo(), self.bar(), |f, b| *f && *b).boxed()
///     }
///
///     fn init_foo(self, foo: bool) -> Self {
///         self.with_meta(|m| m.init_var(&COMMAND_FOO_ID, foo));
///         self
///     }
///
///     fn init_bar(self, bar: bool) -> Self {
///         self.with_meta(|m| m.init_var(&COMMAND_BAR_ID, bar));
///         self
///     }
/// }
/// ```
///
/// [`command!`]: macro@crate::command::command
pub struct CommandMeta<'a> {
    meta: StateMapMut<'a, CommandMetaState>,
    scope: Option<StateMapMut<'a, CommandMetaState>>,
}
impl<'a> CommandMeta<'a> {
    /// Clone a meta value identified by a [`StateId`].
    ///
    /// If the key is not set in the app, insert it using `init` to produce a value.
    pub fn get_or_insert<T, F>(&mut self, id: impl Into<StateId<T>>, init: F) -> T
    where
        T: StateValue + Clone,
        F: FnOnce() -> T,
    {
        let id = id.into();
        if let Some(scope) = &mut self.scope {
            if let Some(value) = scope.get(id) {
                value.clone()
            } else if let Some(value) = self.meta.get(id) {
                value.clone()
            } else {
                let value = init();
                let r = value.clone();
                scope.set(id, value);
                r
            }
        } else {
            self.meta.entry(id).or_insert_with(init).clone()
        }
    }

    /// Clone a meta value identified by a [`StateId`].
    ///
    /// If the key is not set, insert the default value and returns a clone of it.
    pub fn get_or_default<T>(&mut self, id: impl Into<StateId<T>>) -> T
    where
        T: StateValue + Clone + Default,
    {
        self.get_or_insert(id, Default::default)
    }

    /// Set the meta value associated with the [`StateId`].
    ///
    /// Returns the previous value if any was set.
    pub fn set<T>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>)
    where
        T: StateValue + Clone,
    {
        if let Some(scope) = &mut self.scope {
            scope.set(id, value);
        } else {
            self.meta.set(id, value);
        }
    }

    /// Set the metadata value only if it was not set.
    ///
    /// This does not set the scoped override, only the command type metadata.
    pub fn init<T>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>)
    where
        T: StateValue + Clone,
    {
        self.meta.entry(id).or_insert(value);
    }

    /// Clone a meta variable identified by a [`CommandMetaVarId`].
    ///
    /// The variable is read-write and is clone-on-write if the command is scoped,
    /// call [`into_read_only`] to make it read-only.
    ///
    /// [`into_read_only`]: Var::into_read_only
    pub fn get_var_or_insert<T, F>(&mut self, id: impl Into<CommandMetaVarId<T>>, init: F) -> CommandMetaVar<T>
    where
        T: StateValue + VarValue,
        F: FnOnce() -> T,
    {
        let id = id.into();
        if let Some(scope) = &mut self.scope {
            let meta = &mut self.meta;
            scope
                .entry(id.scope())
                .or_insert_with(|| {
                    let var = meta.entry(id.app()).or_insert_with(|| var(init())).clone();
                    CommandMetaVar::new(var)
                })
                .clone()
        } else {
            let var = self.meta.entry(id.app()).or_insert_with(|| var(init())).clone();
            CommandMetaVar::pass_through(var)
        }
    }

    /// Clone a meta variable identified by a [`CommandMetaVarId`].
    ///
    /// Inserts a variable with the default value if no variable is in the metadata.
    pub fn get_var_or_default<T>(&mut self, id: impl Into<CommandMetaVarId<T>>) -> CommandMetaVar<T>
    where
        T: StateValue + VarValue + Default,
    {
        self.get_var_or_insert(id, Default::default)
    }

    /// Set the metadata variable if it was not set.
    ///
    /// This does not set the scoped override, only the command type metadata.
    pub fn init_var<T>(&mut self, id: impl Into<CommandMetaVarId<T>>, value: impl Into<T>)
    where
        T: StateValue + VarValue,
    {
        self.meta.entry(id.into().app()).or_insert_with(|| var(value.into()));
    }
}

/// Read-write command metadata variable.
///
/// If you get this variable from a not scoped command, setting it sets
/// the value for all scopes. If you get this variable using a scoped command
/// setting it overrides only the value for the scope, see [`ScopedCommand`] for more details.
///
/// The aliased type is an [`RcVar`] wrapped in a [`RcCowVar`], for not scoped commands the
/// [`RcCowVar::pass_through`] is used so that the wrapped [`RcVar`] is set directly on assign
/// but the variable type matches that from a scoped command.
///
/// [`ScopedCommand`]: ScopedCommand#metadata
pub type CommandMetaVar<T> = RcCowVar<T, RcVar<T>>;

/// Read-only command metadata variable.
///
/// To convert a [`CommandMetaVar<T>`] into this var call [`into_read_only`].
///
/// [`into_read_only`]: Var::into_read_only
pub type ReadOnlyCommandMetaVar<T> = ReadOnlyVar<T, CommandMetaVar<T>>;

/// Adds the [`name`](CommandNameExt) metadata.
pub trait CommandNameExt: Command {
    /// Gets a read-write variable that is the display name for the command.
    fn name(self) -> CommandMetaVar<Text>;

    /// Sets the initial name if it is not set.
    fn init_name(self, name: impl Into<Text>) -> Self;

    /// Gets a read-only variable that formats the name and first shortcut in the following format: name (first_shortcut)
    /// Note: If no shortcuts are available this method returns the same as [`name`](Self::name)
    fn name_with_shortcut(self) -> BoxedVar<Text>
    where
        Self: crate::gesture::CommandShortcutExt;
}
static COMMAND_NAME_ID: StaticCommandMetaVarId<Text> = StaticCommandMetaVarId::new_unique();
impl<C: Command> CommandNameExt for C {
    fn name(self) -> CommandMetaVar<Text> {
        self.with_meta(|m| {
            m.get_var_or_insert(&COMMAND_NAME_ID, || {
                let name = type_name::<C>();
                name.strip_suffix("Command").unwrap_or(name).to_text()
            })
        })
    }

    fn init_name(self, name: impl Into<Text>) -> Self {
        self.with_meta(|m| m.init_var(&COMMAND_NAME_ID, name.into()));
        self
    }

    fn name_with_shortcut(self) -> BoxedVar<Text>
    where
        Self: crate::gesture::CommandShortcutExt,
    {
        crate::merge_var!(self.name(), self.shortcut(), |name, shortcut| {
            if shortcut.is_empty() {
                name.clone()
            } else {
                crate::formatx!("{name} ({})", shortcut[0])
            }
        })
        .boxed()
    }
}

/// Adds the [`info`](CommandInfoExt) metadata.
pub trait CommandInfoExt: Command {
    /// Gets a read-write variable that is a short informational string about the command.
    fn info(self) -> CommandMetaVar<Text>;

    /// Sets the initial info if it is not set.
    fn init_info(self, info: impl Into<Text>) -> Self;
}
static COMMAND_INFO_ID: StaticCommandMetaVarId<Text> = StaticCommandMetaVarId::new_unique();
impl<C: Command> CommandInfoExt for C {
    fn info(self) -> CommandMetaVar<Text> {
        self.with_meta(|m| m.get_var_or_insert(&COMMAND_INFO_ID, || "".to_text()))
    }

    fn init_info(self, info: impl Into<Text>) -> Self {
        self.with_meta(|m| m.init_var(&COMMAND_INFO_ID, info.into()));
        self
    }
}

enum CommandMetaState {}

/// A handle to a [`Command`].
///
/// Holding the command handle indicates that the command is relevant in the current app state.
/// The handle needs to be enabled to indicate that the command primary action can be executed.
///
/// You can use the [`Command::new_handle`] method in a command type to create a handle.
pub struct CommandHandle {
    handle: Handle<CommandHandleData>,
    local_enabled: Cell<bool>,
}
impl CommandHandle {
    /// Sets if the command event handler is active.
    ///
    /// When at least one [`CommandHandle`] is enabled the command is [`enabled`](Command::enabled).
    pub fn set_enabled(&self, enabled: bool) {
        if self.local_enabled.get() != enabled {
            UpdatesTrace::log_var::<bool>();

            self.local_enabled.set(enabled);
            let data = self.handle.data();

            if enabled {
                let check = data.enabled_count.fetch_add(1, Ordering::Relaxed);
                if check == usize::MAX {
                    data.enabled_count.store(usize::MAX, Ordering::Relaxed);
                    panic!("CommandHandle reached usize::MAX")
                }
            } else {
                data.enabled_count.fetch_sub(1, Ordering::Relaxed);
            };
        }
    }

    /// Returns if this handle has enabled the command.
    pub fn is_enabled(&self) -> bool {
        self.local_enabled.get()
    }

    /// Returns a dummy [`CommandHandle`] that is not connected to any command.
    pub fn dummy() -> Self {
        CommandHandle {
            handle: Handle::dummy(CommandHandleData::default()),
            local_enabled: Cell::new(false),
        }
    }
}
impl fmt::Debug for CommandHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandHandle")
            .field("handle", &self.handle)
            .field("local_enabled", &self.local_enabled)
            .finish()
    }
}
impl Drop for CommandHandle {
    fn drop(&mut self) {
        if self.local_enabled.get() {
            self.handle.data().enabled_count.fetch_sub(1, Ordering::Relaxed);
        }
    }
}
#[derive(Default)]
struct CommandHandleData {
    enabled_count: AtomicUsize,
}

struct ScopedValue {
    handle: HandleOwner<CommandHandleData>,
    enabled: RcVar<bool>,
    has_handlers: RcVar<bool>,
    meta: OwnedStateMap<CommandMetaState>,
    registered: bool,
}
impl Default for ScopedValue {
    fn default() -> Self {
        ScopedValue {
            enabled: var(false),
            has_handlers: var(false),
            handle: HandleOwner::dropped(CommandHandleData::default()),
            meta: OwnedStateMap::default(),
            registered: false,
        }
    }
}

#[doc(hidden)]
pub struct CommandValue {
    command_type_id: TypeId,
    command_type_name: &'static str,

    scopes: RefCell<HashMap<CommandScope, ScopedValue>>,
    slot: EventSlot,

    handle: HandleOwner<CommandHandleData>,

    enabled: RcVar<bool>,

    has_handlers: RcVar<bool>,

    meta: RefCell<OwnedStateMap<CommandMetaState>>,

    meta_init: Box<dyn Fn()>,
    pending_init: Cell<bool>,
    registered: Cell<bool>,

    notify: Box<dyn Fn(&mut Events, CommandArgs)>,
}
#[allow(missing_docs)] // this is all hidden
impl CommandValue {
    pub fn init<C: Command, I: Fn() + 'static>(command: C, meta_init: I) -> Self {
        CommandValue {
            command_type_id: TypeId::of::<C>(),
            command_type_name: type_name::<C>(),
            scopes: RefCell::default(),
            handle: HandleOwner::dropped(CommandHandleData::default()),
            enabled: var(false),
            has_handlers: var(false),
            meta: RefCell::default(),
            meta_init: Box::new(meta_init),
            pending_init: Cell::new(true),
            registered: Cell::new(false),
            slot: EventSlot::next(),
            notify: Box::new(move |events, args| events.notify(command, args)),
        }
    }

    fn update_state(&self, vars: &Vars, scope: CommandScope) {
        if let CommandScope::App = scope {
            self.has_handlers.set_ne(vars, self.has_handlers_value(scope));
            self.enabled.set_ne(vars, self.enabled_value(scope).unwrap_or(false));
        } else {
            let mut has_handlers = false;
            let mut enabled = false;
            if let Some(data) = self.scopes.borrow().get(&scope) {
                has_handlers = !data.handle.is_dropped();
                enabled = data.handle.data().enabled_count.load(Ordering::Relaxed) > 0;
            }

            let scopes = self.scopes.borrow_mut();
            let scope = scopes.get(&scope).unwrap();
            scope.has_handlers.set_ne(vars, has_handlers);
            scope.enabled.set_ne(vars, enabled);
        }
    }

    pub fn on_exit(&self) {
        self.registered.set(false);
        self.scopes.borrow_mut().clear();
        self.meta.borrow_mut().clear();
        self.pending_init.set(true);
    }

    pub fn new_handle<Evs: WithEvents>(
        &self,
        events: &mut Evs,
        key: &'static LocalKey<CommandValue>,
        scope: CommandScope,
        enabled: bool,
    ) -> CommandHandle {
        events.with_events(|ev| self.new_handle_impl(ev, key, scope, enabled))
    }
    fn new_handle_impl(
        &self,
        events: &mut Events,
        key: &'static LocalKey<CommandValue>,
        scope: CommandScope,
        enabled: bool,
    ) -> CommandHandle {
        if let CommandScope::App = scope {
            if !self.registered.get() {
                self.registered.set(true);
                events.register_command(AnyCommand(key, CommandScope::App));
            }
            let r = CommandHandle {
                handle: self.handle.reanimate(),
                local_enabled: Cell::new(false),
            };
            if enabled {
                r.set_enabled(true);
            }
            r
        } else {
            let mut scopes = self.scopes.borrow_mut();
            let value = scopes.entry(scope).or_insert_with(|| {
                // register scope first time and can create variables with the updated values already.
                events.register_command(AnyCommand(key, scope));
                ScopedValue {
                    enabled: var(enabled),
                    has_handlers: var(true),
                    handle: HandleOwner::dropped(CommandHandleData::default()),
                    meta: OwnedStateMap::new(),
                    registered: true,
                }
            });
            if !value.registered {
                // register scope first time.
                events.register_command(AnyCommand(key, scope));
                value.registered = true;
            }
            let r = CommandHandle {
                handle: value.handle.reanimate(),
                local_enabled: Cell::new(false),
            };
            if enabled {
                r.set_enabled(true);
            }
            r
        }
    }

    pub fn slot(&self) -> EventSlot {
        self.slot
    }

    pub fn enabled(&self, scope: CommandScope) -> ReadOnlyVar<bool, RcVar<bool>> {
        if let CommandScope::App = scope {
            ReadOnlyVar::new(self.enabled.clone())
        } else {
            let var = self.scopes.borrow_mut().entry(scope).or_default().enabled.clone();
            ReadOnlyVar::new(var)
        }
    }

    pub fn enabled_value(&self, scope: CommandScope) -> Option<bool> {
        if let CommandScope::App = scope {
            if self.handle.is_dropped() {
                None
            } else {
                Some(self.handle.data().enabled_count.load(Ordering::Relaxed) > 0)
            }
        } else if let Some(value) = self.scopes.borrow().get(&scope) {
            if value.handle.is_dropped() {
                None
            } else {
                Some(value.handle.data().enabled_count.load(Ordering::Relaxed) > 0)
            }
        } else {
            None
        }
    }

    pub fn has_handlers(&self, scope: CommandScope) -> ReadOnlyVar<bool, RcVar<bool>> {
        if let CommandScope::App = scope {
            ReadOnlyVar::new(self.has_handlers.clone())
        } else {
            let var = self.scopes.borrow_mut().entry(scope).or_default().has_handlers.clone();
            ReadOnlyVar::new(var)
        }
    }

    pub fn has_handlers_value(&self, scope: CommandScope) -> bool {
        if let CommandScope::App = scope {
            !self.handle.is_dropped()
        } else if let Some(value) = self.scopes.borrow().get(&scope) {
            !value.handle.is_dropped()
        } else {
            false
        }
    }

    pub fn with_meta<F, R>(&self, f: F, scope: CommandScope) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        if self.pending_init.take() {
            (self.meta_init)()
        }

        if let CommandScope::App = scope {
            f(&mut CommandMeta {
                meta: self.meta.borrow_mut().borrow_mut(),
                scope: None,
            })
        } else {
            let mut scopes = self.scopes.borrow_mut();
            let scope = scopes.entry(scope).or_default();
            f(&mut CommandMeta {
                meta: self.meta.borrow_mut().borrow_mut(),
                scope: Some(scope.meta.borrow_mut()),
            })
        }
    }
}

crate::event_args! {
    /// Event args for command events.
    pub struct CommandArgs {
        /// Optional parameter for the command handler.
        pub param: Option<CommandParam>,

        /// Scope of command that notified.
        pub scope: CommandScope,

        /// If the command handle was enabled when the command notified.
        ///
        /// If `false` the command primary action must not run, but a secondary "disabled interaction"
        /// that indicates what conditions enable the command is recommended.
        pub enabled: bool,

        ..

        /// Broadcast to all widgets for [`CommandScope::App`].
        ///
        /// Broadcast to all widgets in the window for [`CommandScope::Window`].
        ///
        /// Target ancestors and widget for [`CommandScope::Widget`], if it is found.
        fn delivery_list(&self) -> EventDeliveryList {
            match self.scope {
                CommandScope::Widget(id) => EventDeliveryList::find_widget(id),
                CommandScope::Window(id) => EventDeliveryList::window(id),
                _ => EventDeliveryList::all(),
            }
        }
    }
}
impl CommandArgs {
    /// Returns a reference to a parameter of `T` if [`parameter`](#structfield.parameter) is set to a value of `T`.
    pub fn param<T: Any>(&self) -> Option<&T> {
        self.param.as_ref().and_then(|p| p.downcast_ref::<T>())
    }

    /// Returns [`param`] if is enabled interaction.
    ///
    /// [`param`]: Self::param()
    pub fn enabled_param<T: Any>(&self) -> Option<&T> {
        if self.enabled {
            self.param::<T>()
        } else {
            None
        }
    }

    /// Returns [`param`] if is disabled interaction.
    ///
    /// [`param`]: Self::param()
    pub fn disabled_param<T: Any>(&self) -> Option<&T> {
        if !self.enabled {
            self.param::<T>()
        } else {
            None
        }
    }

    /// Stops propagation and call `handler` if the command and local handler are enabled and was not handled.
    ///
    /// This is the default behavior of commands, when a command has a handler it is *relevant* in the context, and overwrites
    /// lower priority handlers, but if the handler is disabled the command primary action is not run.
    ///
    /// Returns the `handler` result if it was called.
    #[allow(unused)]
    pub fn handle_enabled<F, R>(&self, local_handle: &CommandHandle, handler: F) -> Option<R>
    where
        F: FnOnce(&Self) -> R,
    {
        let mut result = None;
        self.handle(|args| {
            if args.enabled && local_handle.is_enabled() {
                result = Some(handler(args));
            }
        });
        result
    }
}

/// Helper for declaring command handlers.
pub fn on_command<U, C, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    C: Command,
    CB: FnMut(&mut WidgetContext) -> C + 'static,
    E: Var<bool>,
    EB: FnMut(&mut WidgetContext) -> E + 'static,
    H: WidgetHandler<CommandArgs>,
{
    struct OnCommandNode<U, C, CB, E, EB, H> {
        child: U,
        command: Option<C>,
        command_builder: CB,
        enabled: Option<E>,
        enabled_builder: EB,
        handler: H,
        handle: Option<CommandHandle>,
    }
    #[impl_ui_node(child)]
    impl<U, C, CB, E, EB, H> UiNode for OnCommandNode<U, C, CB, E, EB, H>
    where
        U: UiNode,
        C: Command,
        CB: FnMut(&mut WidgetContext) -> C + 'static,
        E: Var<bool>,
        EB: FnMut(&mut WidgetContext) -> E + 'static,
        H: WidgetHandler<CommandArgs>,
    {
        fn info(&self, ctx: &mut InfoContext, widget_builder: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget_builder);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(self.command.expect("OnCommandNode not initialized"))
                .var(ctx, self.enabled.as_ref().unwrap())
                .handler(&self.handler);

            self.child.subscriptions(ctx, subs);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            let enabled = (self.enabled_builder)(ctx);
            let is_enabled = enabled.copy(ctx);
            self.enabled = Some(enabled);

            let command = (self.command_builder)(ctx);
            self.command = Some(command);

            self.handle = Some(command.new_handle(ctx, is_enabled));
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.command.expect("OnCommandNode not initialized").update(args) {
                self.child.event(ctx, args);

                if !args.propagation().is_stopped() {
                    self.handler.event(ctx, args);
                }
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.handler.update(ctx);

            if let Some(enabled) = self.enabled.as_ref().expect("OnCommandNode not initialized").copy_new(ctx) {
                self.handle.as_ref().unwrap().set_enabled(enabled);
            }
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.handle = None;
            self.command = None;
            self.enabled = None;
        }
    }

    OnCommandNode {
        child: child.cfg_boxed(),
        command: None,
        command_builder,
        enabled: None,
        enabled_builder,
        handler,
        handle: None,
    }
    .cfg_boxed()
}

/// Helper for declaring command preview handlers.
pub fn on_pre_command<U, C, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    C: Command,
    CB: FnMut(&mut WidgetContext) -> C + 'static,
    E: Var<bool>,
    EB: FnMut(&mut WidgetContext) -> E + 'static,
    H: WidgetHandler<CommandArgs>,
{
    struct OnPreCommandNode<U, C, CB, E, EB, H> {
        child: U,
        command: Option<C>,
        command_builder: CB,
        enabled: Option<E>,
        enabled_builder: EB,
        handler: H,
        handle: Option<CommandHandle>,
    }
    #[impl_ui_node(child)]
    impl<U, C, CB, E, EB, H> UiNode for OnPreCommandNode<U, C, CB, E, EB, H>
    where
        U: UiNode,
        C: Command,
        CB: FnMut(&mut WidgetContext) -> C + 'static,
        E: Var<bool>,
        EB: FnMut(&mut WidgetContext) -> E + 'static,
        H: WidgetHandler<CommandArgs>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            let enabled = (self.enabled_builder)(ctx);
            let is_enabled = enabled.copy(ctx);
            self.enabled = Some(enabled);

            let command = (self.command_builder)(ctx);
            self.command = Some(command);

            self.handle = Some(command.new_handle(ctx, is_enabled));
        }

        fn info(&self, ctx: &mut InfoContext, widget_builder: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget_builder);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(self.command.expect("OnPreCommandNode not initialized"))
                .var(ctx, self.enabled.as_ref().unwrap())
                .handler(&self.handler);

            self.child.subscriptions(ctx, subs);
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.command.expect("OnPreCommandNode not initialized").update(args) {
                if !args.propagation().is_stopped() {
                    self.handler.event(ctx, args);
                }

                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);

            if let Some(enabled) = self.enabled.as_ref().expect("OnPreCommandNode not initialized").copy_new(ctx) {
                self.handle.as_ref().unwrap().set_enabled(enabled);
            }

            self.child.update(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.handle = None;
            self.command = None;
            self.enabled = None;
        }
    }
    OnPreCommandNode {
        child: child.cfg_boxed(),
        command: None,
        command_builder,
        enabled: None,
        enabled_builder,
        handler,
        handle: None,
    }
    .cfg_boxed()
}

#[cfg(test)]
mod tests {
    use crate::context::TestWidgetContext;

    use super::*;

    command! {
        FooCommand;
        BarCommand;
    }

    #[test]
    fn parameter_none() {
        let _ = CommandArgs::now(None, CommandScope::App, true);
    }

    #[test]
    fn enabled() {
        let mut ctx = TestWidgetContext::new();
        assert!(FooCommand.enabled_value().is_none());

        let handle = FooCommand.new_handle(&mut ctx, true);
        assert_eq!(Some(true), FooCommand.enabled_value());

        handle.set_enabled(false);
        assert_eq!(Some(false), FooCommand.enabled_value());

        handle.set_enabled(true);
        assert_eq!(Some(true), FooCommand.enabled_value());

        drop(handle);
        assert!(FooCommand.enabled_value().is_none());
    }

    #[test]
    fn enabled_scoped() {
        let mut ctx = TestWidgetContext::new();

        let cmd = FooCommand;
        let cmd_scoped = FooCommand.scoped(ctx.window_id);
        assert!(cmd.enabled_value().is_none());
        assert!(cmd_scoped.enabled_value().is_none());

        let handle_scoped = cmd_scoped.new_handle(&mut ctx, true);
        assert!(cmd.enabled_value().is_none());
        assert_eq!(Some(true), cmd_scoped.enabled_value());

        handle_scoped.set_enabled(false);
        assert!(cmd.enabled_value().is_none());
        assert_eq!(Some(false), cmd_scoped.enabled_value());

        handle_scoped.set_enabled(true);
        assert!(cmd.enabled_value().is_none());
        assert_eq!(Some(true), cmd_scoped.enabled_value());

        drop(handle_scoped);
        assert!(cmd.enabled_value().is_none());
        assert!(cmd_scoped.enabled_value().is_none());
    }

    #[test]
    fn has_handlers() {
        let mut ctx = TestWidgetContext::new();
        assert!(!FooCommand.has_handlers_value());

        let handle = FooCommand.new_handle(&mut ctx, false);
        assert!(FooCommand.has_handlers_value());

        drop(handle);
        assert!(!FooCommand.has_handlers_value());
    }

    #[test]
    fn has_handlers_scoped() {
        let mut ctx = TestWidgetContext::new();

        let cmd = FooCommand;
        let cmd_scoped = FooCommand.scoped(ctx.window_id);

        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.has_handlers_value());

        let handle = cmd_scoped.new_handle(&mut ctx, false);

        assert!(!cmd.has_handlers_value());
        assert!(cmd_scoped.has_handlers_value());

        drop(handle);

        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.has_handlers_value());
    }

    // there are also integration tests in tests/command.rs
}

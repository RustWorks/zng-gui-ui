//! Command events.
//!
//! Commands are [events](Event) that represent app actions.

use std::{
    any::{type_name, Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    thread::LocalKey,
};

use crate::{
    context::{OwnedStateMap, WidgetContext, WidgetContextMut, WindowContext},
    crate_util::{Handle, HandleOwner},
    event::{Event, Events, WithEvents},
    handler::WidgetHandler,
    impl_ui_node,
    state::{StateKey, StateMap},
    state_key,
    text::{Text, ToText},
    var::{var, BoxedVar, IntoVar, RcCowVar, RcVar, ReadOnlyVar, Var, VarValue, Vars},
    window::WindowId,
    UiNode, WidgetId,
};

/// Declares new [`Command`](crate::command::Command) types.
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
                static COMMAND: $crate::command::CommandValue = $crate::command::CommandValue::init::<$Command, _>(||{
                    #[allow(path_statements)] {
                        $Command $(
                        .$init( $($args)* )
                        )*;
                    }
                });
            }

            /// Gets the event arguments if the update is for this event.
            #[inline(always)]
            #[allow(unused)]
            pub fn update<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<$Command>> {
                <Self as $crate::event::Event>::update(self, args)
            }

            /// Schedule an event update if the command is enabled.
            ///
            /// The `parameter` is an optional value for the command handler.
            ///
            /// Returns `true` if notified, only notifies if the command is enabled.
            #[inline]
            #[allow(unused)]
            pub fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, parameter: Option<std::rc::Rc<dyn std::any::Any>>) -> bool {
                let enabled = Self::COMMAND.with(|c| c.enabled_value());
                if enabled {
                    events.with_events(|evs| {
                        evs.notify::<Self>($crate::command::CommandArgs::now(parameter, $crate::command::Command::scope(self)))
                    });
                }
                enabled
            }

            /// Gets a read-only variable that indicates if the command has at least one enabled handler.
            ///
            /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
            /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
            /// visible but disabled.
            #[inline]
            #[allow(unused)]
            pub fn enabled(self) -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                <Self as $crate::command::Command>::enabled(self)
            }

            /// Gets a read-only variable that indicates if the command has at least one handler.
            ///
            /// When this is `false` the command can be considered *not relevant* in the current app state
            /// and associated command trigger widgets can be hidden.
            #[inline]
            #[allow(unused)]
            pub fn has_handlers(self) -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                <Self as $crate::command::Command>::has_handlers(self)
            }

            /// Create a new handle to this command.
            ///
            /// A handle indicates that there is an active *handler* for the event, the handle can also
            /// be used to set the [`enabled`](Self::enabled) state.
            #[inline]
            #[allow(unused)]
            pub fn new_handle<Evs: $crate::event::WithEvents>(self, events: &mut Evs, enabled: bool) -> $crate::command::CommandHandle {
                <Self as $crate::command::Command>::new_handle(self, events, enabled)
            }

            /// Get a scoped command derived from this command type.
            #[inline]
            #[allow(unused)]
            pub fn scoped<S: Into<$crate::command::CommandScope>>(self, scope: S) -> $crate::command::ScopedCommand<Self> {
                <Self as $crate::command::Command>::scoped(self, scope)
            }
        }
        impl $crate::event::Event for $Command {
            type Args = $crate::command::CommandArgs;

            #[inline(always)]
            fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, args: Self::Args) {
                if Self::COMMAND.with(|c| c.enabled_value()) {
                    events.with_events(|evs| evs.notify::<Self>(args));
                }
            }
        }
        impl $crate::command::Command for $Command {
            type AppScopeCommand = Self;

            #[inline]
            fn thread_local_value(self) -> &'static std::thread::LocalKey<$crate::command::CommandValue> {
                &Self::COMMAND
            }

            #[inline]
            fn scoped<S: Into<$crate::command::CommandScope>>(self, scope: S) ->  $crate::command::ScopedCommand<Self> {
                $crate::command::ScopedCommand{ command: self, scope: scope.into() }
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::command;

/// Identifies a command type.
///
/// Use [`command!`](macro@crate::command::command) to declare.
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Command: Event<Args = CommandArgs> {
    /// The root command type.
    ///
    /// This should be `Self` by default, and will be once [this] is stable.
    ///
    /// [this]: https://github.com/rust-lang/rust/issues/29661
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
        self.thread_local_value().with(|c| c.with_meta(f))
    }

    /// Gets a read-only variable that indicates if the command has at least one enabled handler.
    ///
    /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
    /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
    /// visible but disabled.
    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        self.thread_local_value().with(|c| c.enabled())
    }

    /// Gets if the command has at least one enabled handler.
    fn enabled_value(self) -> bool {
        self.thread_local_value().with(|c| c.enabled_value())
    }

    /// Gets a read-only variable that indicates if the command has at least one handler.
    ///
    /// When this is `false` the command can be considered *not relevant* in the current app state
    /// and associated command trigger widgets can be hidden.
    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        self.thread_local_value().with(|c| c.has_handlers())
    }

    /// Gets if the command has at least one handler.
    fn has_handlers_value(self) -> bool {
        self.thread_local_value().with(|c| c.has_handlers_value())
    }

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`enabled`](Self::enabled) state.
    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        let tl = self.thread_local_value();
        let scope = self.scope();
        tl.with(|c| c.new_handle(events, tl, scope, enabled))
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
}

/// Represents the scope of a [scoped command].
///
/// [scoped command]: Command::scoped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandScope {
    /// Default scope, the command notifies in all scopes.
    App,
    /// A window and its content.
    Window(WindowId),
    /// A widget and its content.
    Widget(WidgetId),
    /// A custom scope.
    Custom(TypeId, u64),
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
    fn from(ctx: &'a WidgetContext<'a>) -> Self {
        CommandScope::Widget(ctx.path.widget_id())
    }
}
impl<'a> From<&'a WindowContext<'a>> for CommandScope {
    fn from(ctx: &'a WindowContext<'a>) -> CommandScope {
        CommandScope::Window(*ctx.window_id)
    }
}
impl<'a> From<&'a WidgetContextMut> for CommandScope {
    fn from(ctx: &'a WidgetContextMut) -> Self {
        CommandScope::Widget(ctx.widget_id())
    }
}

/// A command that is a command type in a specific scope.
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
/// a handler created using the same scope, handlers created for other scopes or
/// [not scoped] do not activate a scoped command. On the other hand, scoped handlers
/// activate [`enabled`] and [`has_handlers`] of the [not scoped] command **and** of the scoped command.
///
/// ```
/// # use zero_ui_core::{command::*, context::*};
/// # command! { pub FooCommand; }
/// # fn init(ctx: &mut WindowContext) {
/// let a = FooCommand.enabled();
/// let b = FooCommand.scoped(*ctx.window_id).enabled();
/// # }  
/// ```
///
/// In the example above, `a` is `true` when there is any handler enabled, scoped or not, but `b` is only
/// `true` when there is a handler created using the same `window_id` and enabled.
///
/// # Metadata
///
/// Metadata is *inherited* from the [not scoped] command type but can be overwritten for the scoped command
/// only, so you can rename or give a different shortcut for the command only in the scope.
///
/// ```
/// # use zero_ui_core::{var::*, command::*, handler::*};
/// # command! { pub FooCommand; }
/// # fn demo() -> impl WidgetHandler<()> {
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
///     cmd_scoped.name().set(&ctx, "Scope Only!");
///     ctx.update().await;
///     assert_eq!("Scoped Only!", cmd_scoped.name().get_clone(&ctx));
///     assert_eq!("Foo!", cmd.name().get_clone(&ctx));
///
///     // scoped command no-longer affected:
///     cmd.name().set(&ctx, "F");
///     assert_eq!("F", cmd.name().get_clone(&ctx));
///     assert_eq!("Scoped Only!", cmd_scoped.name().get_clone(&ctx));
/// })
/// # }
/// ```
///
/// See [`CommandMetaVar<T>`] for details.
///
/// # Notify
///
/// Calling [`notify`] from a scoped command **notifies the base type** but sets the [`CommandArgs::scope`]
/// the event will be handled by handlers for the same scope and by [not scoped] handlers.
///
/// ```
/// # use zero_ui_core::{command::*, context::*};
/// # command! { pub FooCommand; }
/// # fn init(ctx: &mut WindowContext) {
/// let notified = FooCommand.notify(ctx, None);
/// # }  
/// ```
///
/// In the example above `notified` is `true` if there are any enabled handlers for the same scope or [not scoped].
///
/// # Update
///
/// Calling [`update`] from a scoped command detects updates for the same command base type if the [`CommandArgs::scope`]
/// is equal to the command scope or is [not scoped]. Unless the command [not scoped], in this case it will detect updates
/// from any scope.
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
/// The example will print for [`CommandScope::Window`] with the same id and for [`CommandScope::App`].
///
/// [`enabled`]: ScopedCommand::enabled
/// [`notify`]: ScopedCommand::notify
/// [`update`]: ScopedCommand::update
/// [`has_handlers`]: ScopedCommand::has_handlers
/// [not scoped]: CommandScope::App
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
    #[inline]
    #[allow(unused)]
    pub fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        <Self as Command>::enabled(self)
    }

    /// Gets a read-only variable that indicates if the command has at least one handler in the scope.
    #[inline]
    #[allow(unused)]
    pub fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        <Self as Command>::has_handlers(self)
    }

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`enabled`](Self::enabled) state.
    #[inline]
    #[allow(unused)]
    pub fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        <Self as Command>::new_handle(self, events, enabled)
    }

    /// Schedule an event update if the command is enabled.
    ///
    /// The event notified is the `C` command, not `Self`. The scope is passed in the [`CommandArgs`].
    ///
    /// The `parameter` is an optional value for the command handler.
    ///
    /// Returns `true` if notified, only notifies if the command is enabled.
    pub fn notify<Evs: WithEvents>(self, events: &mut Evs, parameter: Option<Rc<dyn Any>>) -> bool {
        let enabled = self.thread_local_value().with(|c| c.enabled_value());
        if enabled {
            events.with_events(|evs| evs.notify::<C>(CommandArgs::now(parameter, self.scope)));
        }
        enabled
    }

    /// Gets the event arguments if the update is for this command and is of a compatible scope.
    ///
    /// The scope is compatible if it is [`CommandScope::App`] or is equal to the `scope`.
    pub fn update<U: crate::event::EventUpdateArgs>(self, args: &U) -> Option<&crate::event::EventUpdate<Self>> {
        if let Some(args) = args.args_for::<C>() {
            if args.scope == CommandScope::App || self.scope == CommandScope::App || args.scope == self.scope {
                Some(args.transmute_event::<Self>())
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
        if self.enabled_value() {
            events.with_events(|events| events.notify::<C>(args));
        }
    }

    fn update<U: crate::event::EventUpdateArgs>(self, args: &U) -> Option<&crate::event::EventUpdate<Self>> {
        self.update(args)
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
        self.command.thread_local_value().with(|c| c.with_meta_scoped(f, scope))
    }

    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope;
        self.command.thread_local_value().with(|c| c.enabled_scoped(scope))
    }

    fn enabled_value(self) -> bool {
        let scope = self.scope;
        self.command.thread_local_value().with(|c| c.enabled_value_scoped(scope))
    }

    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope;
        self.command.thread_local_value().with(|c| c.has_handlers_scoped(scope))
    }

    fn has_handlers_value(self) -> bool {
        let scope = self.scope;
        self.command.thread_local_value().with(|c| c.has_handlers_value_scoped(scope))
    }

    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        let key = self.command.thread_local_value();
        let scope = self.scope;
        key.with(|c| c.new_handle(events, key, scope, enabled))
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

    fn as_any(self) -> AnyCommand {
        let mut any = self.command.as_any();
        any.1 = self.scope;
        any
    }
}

/// Represents a [`Command`] type.
#[derive(Clone, Copy)]
pub struct AnyCommand(&'static LocalKey<CommandValue>, CommandScope);
impl AnyCommand {
    #[inline]
    #[doc(hidden)]
    pub fn new(c: &'static LocalKey<CommandValue>, scope: CommandScope) -> Self {
        AnyCommand(c, scope)
    }

    pub(crate) fn update_state(&self, vars: &Vars) {
        self.0.with(|c| c.update_state(vars))
    }

    /// Gets the [`TypeId`] of the command represented by `self`.
    #[inline]
    pub fn command_type_id(self) -> TypeId {
        self.0.with(|c| c.command_type_id)
    }

    /// Gets the [`type_name`] of the command represented by `self`.
    #[inline]
    pub fn command_type_name(self) -> &'static str {
        self.0.with(|c| c.command_type_name)
    }

    /// If the command `C` is represented by `self`.
    #[inline]
    pub fn is<C: Command>(self) -> bool {
        self.command_type_id() == TypeId::of::<C>()
    }

    /// Schedule an event update for the command represented by `self`.
    #[inline]
    pub fn notify(self, events: &mut Events, parameter: Option<Rc<dyn Any>>) {
        Event::notify(self, events, CommandArgs::now(parameter, self.1))
    }
}
impl fmt::Debug for AnyCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "any {}", self.command_type_name())
    }
}
impl Event for AnyCommand {
    type Args = CommandArgs;

    fn notify<Evs: WithEvents>(self, events: &mut Evs, args: Self::Args) {
        self.0.with(|c| {
            if c.enabled_value() {
                events.with_events(|e| (c.notify)(e, args))
            }
        });
    }
    fn update<U: crate::event::EventUpdateArgs>(self, _: &U) -> Option<&crate::event::EventUpdate<Self>> {
        // TODO use a closure in the value and then transmute to Self?
        panic!("`AnyCommand` does not support `Event::update`");
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
        self.0.with(move |c| c.with_meta_scoped(f, self.1))
    }

    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        self.0.with(|c| c.enabled_scoped(self.1))
    }

    fn enabled_value(self) -> bool {
        self.0.with(|c| c.enabled_value_scoped(self.1))
    }

    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        self.0.with(|c| c.has_handlers_scoped(self.1))
    }

    fn has_handlers_value(self) -> bool {
        self.0.with(|c| c.has_handlers_value_scoped(self.1))
    }

    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        self.0.with(|c| c.new_handle(events, self.0, self.1, enabled))
    }

    fn as_any(self) -> AnyCommand {
        self
    }

    fn scope(self) -> CommandScope {
        self.1
    }

    fn scoped<S: Into<CommandScope>>(self, scope: S) -> ScopedCommand<Self> {
        ScopedCommand {
            command: self,
            scope: scope.into(),
        }
    }
}

struct AppCommandMetaKey<S>(PhantomData<S>);
impl<S: StateKey> StateKey for AppCommandMetaKey<S>
where
    S::Type: VarValue,
{
    type Type = RcVar<S::Type>;
}

struct ScopedCommandMetaKey<S>(PhantomData<S>);
impl<S: StateKey> StateKey for ScopedCommandMetaKey<S>
where
    S::Type: VarValue,
{
    type Type = RcCowVar<S::Type, RcVar<S::Type>>;
}

/// Access to metadata of a command.
pub struct CommandMeta<'a> {
    meta: &'a mut StateMap,
    scope: Option<&'a mut StateMap>,
}
impl<'a> CommandMeta<'a> {
    /// Clone a meta value identified by a [`StateKey`] type.
    ///
    /// If the key is not set in the app, insert it using `init` to produce a value.
    pub fn get_or_insert<S, F>(&mut self, _key: S, init: F) -> S::Type
    where
        S: StateKey,
        F: FnOnce() -> S::Type,
        S::Type: Clone,
    {
        if let Some(scope) = &mut self.scope {
            if let Some(value) = scope.get::<S>() {
                value.clone()
            } else if let Some(value) = self.meta.get::<S>() {
                value.clone()
            } else {
                let value = init();
                let r = value.clone();
                scope.set::<S>(value);
                r
            }
        } else {
            self.meta.entry::<S>().or_insert_with(init).clone()
        }
    }

    /// Clone a meta value identified by a [`StateKey`] type.
    ///
    /// If the key is not set, insert the default value and returns a clone of it.
    pub fn get_or_default<S>(&mut self, key: S) -> S::Type
    where
        S: StateKey,
        S::Type: Clone + Default,
    {
        self.get_or_insert(key, Default::default)
    }

    /// Set the meta value associated with the [`StateKey`] type.
    ///
    /// Returns the previous value if any was set.
    pub fn set<S>(&mut self, _key: S, value: S::Type)
    where
        S: StateKey,
        S::Type: Clone,
    {
        if let Some(scope) = &mut self.scope {
            scope.set::<S>(value);
        } else {
            self.meta.set::<S>(value);
        }
    }

    /// Set the metadata value only if it was not set.
    ///
    /// This does not set the scoped override, only the command type metadata.
    pub fn init<S>(&mut self, _key: S, value: S::Type)
    where
        S: StateKey,
        S::Type: Clone,
    {
        self.meta.entry::<S>().or_insert(value);
    }

    /// Clone a meta variable identified by a [`StateKey`] type.
    ///
    /// The variable is read-write and is clone-on-write if the command is scoped,
    /// call [`into_read_only`] to make it read-only.
    ///
    /// Note that the the [`StateKey`] type is the variable value type, the variable
    /// type is [`CommandMetaVar<S::Type>`]. This is done to ensure that the associated
    /// metadata implements the *scoped inheritance* of values correctly.
    ///
    /// [`into_read_only`]: Var::into_read_only
    pub fn get_var_or_insert<S, F>(&mut self, _key: S, init: F) -> CommandMetaVar<S::Type>
    where
        S: StateKey,
        F: FnOnce() -> S::Type,
        S::Type: VarValue,
    {
        if let Some(scope) = &mut self.scope {
            let meta = &mut self.meta;
            scope
                .entry::<ScopedCommandMetaKey<S>>()
                .or_insert_with(|| {
                    let var = meta.entry::<AppCommandMetaKey<S>>().or_insert_with(|| var(init())).clone();
                    CommandMetaVar::new(var)
                })
                .clone()
        } else {
            let var = self.meta.entry::<AppCommandMetaKey<S>>().or_insert_with(|| var(init())).clone();
            CommandMetaVar::pass_through(var)
        }
    }

    /// Clone a meta variable identified by a [`StateKey`] type.
    ///
    /// Inserts a variable with the default value if no variable is in the metadata.
    pub fn get_var_or_default<S>(&mut self, key: S) -> CommandMetaVar<S::Type>
    where
        S: StateKey,
        S::Type: VarValue + Default,
    {
        self.get_var_or_insert(key, Default::default)
    }

    /// Set the metadata variable if it was not set.
    ///
    /// This does not set the scoped override, only the command type metadata.
    pub fn init_var<S>(&mut self, _key: S, value: S::Type)
    where
        S: StateKey,
        S::Type: VarValue,
    {
        self.meta.entry::<AppCommandMetaKey<S>>().or_insert_with(|| var(value));
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
state_key! {
    struct CommandNameKey: Text;
}
impl<C: Command> CommandNameExt for C {
    fn name(self) -> CommandMetaVar<Text> {
        self.with_meta(|m| {
            m.get_var_or_insert(CommandNameKey, || {
                let name = type_name::<C>();
                name.strip_suffix("Command").unwrap_or(name).to_text()
            })
        })
    }

    fn init_name(self, name: impl Into<Text>) -> Self {
        self.with_meta(|m| m.init_var(CommandNameKey, name.into()));
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
                crate::formatx!("{} ({})", name, shortcut[0])
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
state_key! {
    struct CommandInfoKey: Text;
}
impl<C: Command> CommandInfoExt for C {
    fn info(self) -> CommandMetaVar<Text> {
        self.with_meta(|m| m.get_var_or_insert(CommandInfoKey, || "".to_text()))
    }

    fn init_info(self, info: impl Into<Text>) -> Self {
        self.with_meta(|m| m.init_var(CommandNameKey, info.into()));
        self
    }
}

/// A handle to a [`Command`].
///
/// Holding the command handle indicates that the command is relevant in the current app state.
/// The handle needs to be enabled to indicate that the command can be issued.
///
/// You can use the [`Command::new_handle`] method in a command type to create a handle.
pub struct CommandHandle {
    handle: Handle<AtomicUsize>,
    local_enabled: Cell<bool>,
}
impl CommandHandle {
    /// Sets if the command event handler is active.
    ///
    /// When at least one [`CommandHandle`] is enabled the command is [`enabled`](Command::enabled).
    pub fn set_enabled(&self, enabled: bool) {
        if self.local_enabled.get() != enabled {
            self.local_enabled.set(enabled);
            if enabled {
                self.handle.data().fetch_add(1, Ordering::Relaxed);
            } else {
                self.handle.data().fetch_sub(1, Ordering::Relaxed);
            };
        }
    }

    /// Returns a dummy [`CommandHandle`] that is not connected to any command.
    pub fn dummy() -> Self {
        CommandHandle {
            handle: Handle::dummy(AtomicUsize::new(0)),
            local_enabled: Cell::new(false),
        }
    }
}
impl Drop for CommandHandle {
    fn drop(&mut self) {
        self.set_enabled(false);
    }
}

struct ScopedValue {
    handle: HandleOwner<AtomicUsize>,

    enabled: RcVar<bool>,
    has_handlers: RcVar<bool>,
    meta: OwnedStateMap,
}
impl Default for ScopedValue {
    fn default() -> Self {
        ScopedValue {
            handle: HandleOwner::dropped(AtomicUsize::new(0)),
            enabled: var(false),
            has_handlers: var(false),
            meta: OwnedStateMap::default(),
        }
    }
}

#[doc(hidden)]
pub struct CommandValue {
    command_type_id: TypeId,
    command_type_name: &'static str,

    scopes: RefCell<HashMap<CommandScope, ScopedValue>>,

    handle: HandleOwner<AtomicUsize>,

    enabled: RcVar<bool>,

    has_handlers: RcVar<bool>,

    meta: RefCell<OwnedStateMap>,

    meta_init: Cell<Option<Box<dyn FnOnce()>>>,
    registered: Cell<bool>,

    notify: Box<dyn Fn(&mut Events, CommandArgs)>,
}
#[allow(missing_docs)] // this is all hidden
impl CommandValue {
    pub fn init<C: Command, I: FnOnce() + 'static>(meta_init: I) -> Self {
        CommandValue {
            command_type_id: TypeId::of::<C>(),
            command_type_name: type_name::<C>(),
            scopes: RefCell::default(),
            handle: HandleOwner::dropped(AtomicUsize::new(0)),
            enabled: var(false),
            has_handlers: var(false),
            meta: RefCell::default(),
            meta_init: Cell::new(Some(Box::new(meta_init))),
            registered: Cell::new(false),
            notify: Box::new(|events, args| events.notify::<C>(args)),
        }
    }

    fn update_state(&self, vars: &Vars) {
        self.has_handlers.set_ne(vars, self.has_handlers_value());
        self.enabled.set_ne(vars, self.enabled_value());
    }

    pub fn new_handle<Evs: WithEvents>(
        &self,
        events: &mut Evs,
        key: &'static LocalKey<CommandValue>,
        scope: CommandScope,
        enabled: bool,
    ) -> CommandHandle {
        if !self.registered.get() {
            self.registered.set(true);
            events.with_events(|e| e.register_command(AnyCommand(key, scope)));
        }
        let r = CommandHandle {
            handle: self.handle.reanimate(),
            local_enabled: Cell::new(false),
        };
        if enabled {
            r.set_enabled(true);
        }
        r
    }

    pub fn enabled(&self) -> ReadOnlyVar<bool, RcVar<bool>> {
        ReadOnlyVar::new(self.enabled.clone())
    }
    pub fn enabled_scoped(&self, scope: CommandScope) -> ReadOnlyVar<bool, RcVar<bool>> {
        if let CommandScope::App = scope {
            self.enabled()
        } else {
            let var = self.scopes.borrow_mut().entry(scope).or_default().enabled.clone();
            ReadOnlyVar::new(var)
        }
    }

    pub fn enabled_value(&self) -> bool {
        self.has_handlers_value() && self.handle.data().load(Ordering::Relaxed) > 0
    }
    pub fn enabled_value_scoped(&self, scope: CommandScope) -> bool {
        if let CommandScope::App = scope {
            self.enabled_value()
        } else if let Some(scope) = self.scopes.borrow().get(&scope) {
            !scope.handle.is_dropped() && scope.handle.data().load(Ordering::Relaxed) > 0
        } else {
            false
        }
    }

    pub fn has_handlers(&self) -> ReadOnlyVar<bool, RcVar<bool>> {
        ReadOnlyVar::new(self.has_handlers.clone())
    }
    pub fn has_handlers_scoped(&self, scope: CommandScope) -> ReadOnlyVar<bool, RcVar<bool>> {
        if let CommandScope::App = scope {
            self.has_handlers()
        } else {
            let var = self.scopes.borrow_mut().entry(scope).or_default().has_handlers.clone();
            ReadOnlyVar::new(var)
        }
    }

    pub fn has_handlers_value(&self) -> bool {
        !self.handle.is_dropped()
    }
    pub fn has_handlers_value_scoped(&self, scope: CommandScope) -> bool {
        if let CommandScope::App = scope {
            self.has_handlers_value()
        } else if let Some(scope) = self.scopes.borrow().get(&scope) {
            !scope.handle.is_dropped()
        } else {
            false
        }
    }

    pub fn with_meta<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        if let Some(init) = self.meta_init.take() {
            init()
        }
        f(&mut CommandMeta {
            meta: &mut self.meta.borrow_mut().0,
            scope: None,
        })
    }
    pub fn with_meta_scoped<F, R>(&self, f: F, scope: CommandScope) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        if let CommandScope::App = scope {
            self.with_meta(f)
        } else {
            if let Some(init) = self.meta_init.take() {
                init()
            }

            let mut scopes = self.scopes.borrow_mut();
            let scope = scopes.entry(scope).or_default();
            f(&mut CommandMeta {
                meta: &mut self.meta.borrow_mut().0,
                scope: Some(&mut scope.meta.0),
            })
        }
    }
}

crate::event_args! {
    /// Event args for command events.
    pub struct CommandArgs {
        /// Optional parameter for the command handler.
        pub parameter: Option<Rc<dyn Any>>,

        /// Scope of command that notified.
        pub scope: CommandScope,

        ..

        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            match self.scope {
                CommandScope::App => true,
                CommandScope::Window(id) => ctx.path.window_id() == id,
                CommandScope::Widget(id) => ctx.path.contains(id),
                CommandScope::Custom(_, _) => true,
            }
        }
    }
}
impl CommandArgs {
    /// Returns a reference to a parameter of `T` if [`parameter`](#structfield.parameter) is set to a value of `T`.
    #[inline]
    pub fn parameter<T: Any>(&self) -> Option<&T> {
        self.parameter.as_ref().and_then(|p| p.downcast_ref::<T>())
    }
}

/// Helper for declaring command properties.
#[inline]
pub fn on_command<U, C, E, H>(child: U, command: C, enabled: E, handler: H) -> impl UiNode
where
    U: UiNode,
    C: Command,
    E: IntoVar<bool>,
    H: WidgetHandler<CommandArgs>,
{
    struct OnCommandNode<U, C, E, H> {
        child: U,
        command: C,
        enabled: E,
        handler: H,
        handle: Option<CommandHandle>,
    }
    #[impl_ui_node(child)]
    impl<U, C, E, H> UiNode for OnCommandNode<U, C, E, H>
    where
        U: UiNode,
        C: Command,
        E: Var<bool>,
        H: WidgetHandler<CommandArgs>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            let enabled = self.enabled.copy(ctx);
            self.handle = Some(self.command.new_handle(ctx, enabled));
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.command.update(args) {
                self.child.event(ctx, args);

                if !args.stop_propagation_requested() && self.enabled.copy(ctx) {
                    self.handler.event(ctx, args);
                }
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.handler.update(ctx);

            if let Some(enabled) = self.enabled.copy_new(ctx) {
                self.handle.as_ref().unwrap().set_enabled(enabled);
            }
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.handle = None;
        }
    }
    OnCommandNode {
        child,
        command,
        enabled: enabled.into_var(),
        handler,
        handle: None,
    }
}

/// Helper for declaring command properties.
#[inline]
pub fn on_pre_command<U, C, E, H>(child: U, command: C, enabled: E, handler: H) -> impl UiNode
where
    U: UiNode,
    C: Command,
    E: IntoVar<bool>,
    H: WidgetHandler<CommandArgs>,
{
    struct OnPreviewCommandNode<U, C, E, H> {
        child: U,
        command: C,
        enabled: E,
        handler: H,
        handle: Option<CommandHandle>,
    }
    #[impl_ui_node(child)]
    impl<U, C, E, H> UiNode for OnPreviewCommandNode<U, C, E, H>
    where
        U: UiNode,
        C: Command,
        E: Var<bool>,
        H: WidgetHandler<CommandArgs>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            let enabled = self.enabled.copy(ctx);
            self.handle = Some(self.command.new_handle(ctx, enabled));
            self.child.init(ctx);
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.command.update(args) {
                if !args.stop_propagation_requested() && self.enabled.copy(ctx) {
                    self.handler.event(ctx, args);
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);

            if let Some(enabled) = self.enabled.copy_new(ctx) {
                self.handle.as_ref().unwrap().set_enabled(enabled);
            }
            self.child.update(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.handle = None;
        }
    }
    OnPreviewCommandNode {
        child,
        command,
        enabled: enabled.into_var(),
        handler,
        handle: None,
    }
}

/// Declare command properties.
#[macro_export]
macro_rules! command_property {
    ($(
        $(#[$on_command_attrs:meta])*
        $vis:vis fn $command:ident: $Command:path;
    )+) => {$($crate::paste! {

        $crate::var::context_var! {
            struct [<Can $Command Var>]: bool = const true;
        }

        $(#[$on_command_attrs])*
        ///
        /// # Enable
        ///
        #[doc = "You can control if this property is enabled by setting the [`can_"$command"`](fn.can_"$command".html)."]
        /// property in the same widget or a parent widget.
        ///
        /// # Preview
        ///
        #[doc = "You can preview this command event using [`on_pre_"$command"`](fn.on_pre_"$command".html)."]
        /// Otherwise the handler is only called after the widget content has a chance of handling the event by stopping propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::property(event, default( $crate::handler::hn!(|_, _|{}) ))]
        pub fn [<on_ $command>](
            child: impl $crate::UiNode,
            handler: impl $crate::handler::WidgetHandler<$crate::command::CommandArgs>
        ) -> impl $crate::UiNode {
            $crate::command::on_command(child, $Command, [<Can $Command Var>], handler)
        }

        #[doc = "Preview [`on_"$command"`](fn.on_"$command".html) command event."]
        ///
        /// # Preview
        ///
        /// Preview event properties call the handler before the main event property and before the widget content, if you stop
        /// the propagation of a preview event the main event handler is not called.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property, note that only the code before the fist `.await` is *preview*,
        /// subsequent code runs in widget updates.
        #[$crate::property(event, default( $crate::handler::hn!(|_, _|{}) ))]
        pub fn [<on_pre_ $command>](
            child: impl $crate::UiNode,
            handler: impl $crate::handler::WidgetHandler<$crate::command::CommandArgs>
        ) -> impl $crate::UiNode {
            $crate::command::on_pre_command(child, $Command, [<Can $Command Var>], handler)
        }

        #[doc = "Enable/Disable the [`on_"$command"`](fn.on_"$command".html) command event in the widget or its content."]
        ///
        /// # Commands
        ///
        /// TODO
        #[$crate::property(context, allowed_in_when = false, default( true ))]
        pub fn [<can_ $command>](
            child: impl $crate::UiNode,
            enabled: impl $crate::var::IntoVar<bool>
        ) -> impl $crate::UiNode {
            $crate::var::with_context_var(child, [<Can $Command Var>], enabled)
        }

    })+}
}
#[doc(inline)]
pub use crate::command_property;

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
        let _ = CommandArgs::now(None, CommandScope::App);
    }

    #[test]
    fn enabled() {
        let mut ctx = TestWidgetContext::new();
        assert!(!FooCommand.enabled_value());

        let handle = FooCommand.new_handle(&mut ctx, true);
        assert!(FooCommand.enabled_value());

        handle.set_enabled(false);
        assert!(!FooCommand.enabled_value());

        handle.set_enabled(true);
        assert!(FooCommand.enabled_value());

        drop(handle);
        assert!(!FooCommand.enabled_value());
    }

    #[test]
    fn enabled_scoped() {
        let mut ctx = TestWidgetContext::new();

        let cmd = FooCommand;
        let cmd_scoped = FooCommand.scoped(ctx.window_id);
        assert!(!cmd.enabled_value());
        assert!(!cmd_scoped.enabled_value());

        let handle_scoped = cmd_scoped.new_handle(&mut ctx, true);
        assert!(cmd.enabled_value());
        assert!(cmd_scoped.enabled_value());

        handle_scoped.set_enabled(false);
        assert!(!cmd.enabled_value());
        assert!(!cmd_scoped.enabled_value());

        handle_scoped.set_enabled(true);
        assert!(cmd.enabled_value());
        assert!(cmd_scoped.enabled_value());

        drop(handle_scoped);
        assert!(!cmd.enabled_value());
        assert!(!cmd_scoped.enabled_value());
    }

    #[test]
    fn has_handlers() {
        todo!()
    }

    #[test]
    fn has_handlers_scoped() {
        todo!()
    }
    // there are also integration tests in tests/command.rs
}

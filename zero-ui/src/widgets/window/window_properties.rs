use std::marker::PhantomData;
use std::time::Duration;

use crate::core::color::ColorScheme;
use crate::core::config::{ConfigKey, CONFIG};
use crate::core::text::formatx;
use crate::core::window::{
    AutoSize, FrameCaptureMode, MonitorQuery, WindowChrome, WindowIcon, WindowId, WindowLoadingHandle, WindowState, WindowVars, MONITORS,
    WINDOW_CTRL, WINDOW_LOAD_EVENT,
};
use crate::prelude::new_property::*;
use serde::{Deserialize, Serialize};

use super::Window;

fn bind_window_var<T, V>(child: impl UiNode, user_var: impl IntoVar<T>, select: impl Fn(&WindowVars) -> V + Send + 'static) -> impl UiNode
where
    T: VarValue + PartialEq,
    V: Var<T>,
{
    #[ui_node(struct BindWindowVarNode<T: VarValue + PartialEq, SV: Var<T>> {
        _t: PhantomData<T>,
        child: impl UiNode,
        user_var: impl Var<T>,
        select: impl Fn(&WindowVars) -> SV + Send + 'static,
    })]
    impl UiNode for BindWindowVarNode {
        fn init(&mut self) {
            let window_var = (self.select)(&WINDOW_CTRL.vars());
            if !self.user_var.capabilities().is_always_static() {
                let binding = self.user_var.bind_bidi(&window_var);
                WIDGET.push_var_handles(binding);
            }
            window_var.set_ne(self.user_var.get()).unwrap();
            self.child.init();
        }
    }
    BindWindowVarNode {
        _t: PhantomData,
        child,
        user_var: user_var.into_var(),
        select,
    }
}

// Properties that set the full value.
macro_rules! set_properties {
    ($(
        $ident:ident: $Type:ty,
    )+) => {
        $(paste::paste! {
            #[doc = "Binds the [`"$ident "`](WindowVars::"$ident ") window var with the property value."]
            ///
            /// The binding is bidirectional and the window variable is assigned on init.
            #[property(CONTEXT, widget_impl(Window))]
            pub fn $ident(child: impl UiNode, $ident: impl IntoVar<$Type>) -> impl UiNode {
                bind_window_var(child, $ident, |w|w.$ident().clone())
            }
        })+
    }
}
set_properties! {
    position: Point,
    monitor: MonitorQuery,

    state: WindowState,

    size: Size,
    min_size: Size,
    max_size: Size,

    font_size: Length,

    chrome: WindowChrome,
    icon: WindowIcon,
    title: Txt,

    auto_size: AutoSize,
    auto_size_origin: Point,

    resizable: bool,
    movable: bool,

    always_on_top: bool,

    visible: bool,
    taskbar_visible: bool,

    parent: Option<WindowId>,
    modal: bool,

    color_scheme: Option<ColorScheme>,

    frame_capture_mode: FrameCaptureMode,

    renderer_debug: RendererDebug,
}

macro_rules! map_properties {
    ($(
        $ident:ident . $member:ident = $name:ident : $Type:ty,
    )+) => {$(paste::paste! {
        #[doc = "Binds the `"$member "` of the [`"$ident "`](WindowVars::"$ident ") window var with the property value."]
        ///
        /// The binding is bidirectional and the window variable is assigned on init.
        #[property(CONTEXT, widget_impl(Window))]
        pub fn $name(child: impl UiNode, $name: impl IntoVar<$Type>) -> impl UiNode {
            bind_window_var(child, $name, |w|w.$ident().map_ref_bidi(|v| &v.$member, |v|&mut v.$member))
        }
    })+}
}
map_properties! {
    position.x = x: Length,
    position.y = y: Length,
    size.width = width: Length,
    size.height = height: Length,
    min_size.width = min_width: Length,
    min_size.height = min_height: Length,
    max_size.width = max_width: Length,
    max_size.height = max_height: Length,
}

/// Window clear color.
///
/// Color used to *clear* the previous frame pixels before rendering a new frame.
/// It is visible if window content does not completely fill the content area, this
/// can happen if you do not set a background or the background is semi-transparent, also
/// can happen during very fast resizes.
#[property(CONTEXT, default(colors::WHITE), widget_impl(Window))]
pub fn clear_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    #[ui_node(struct ClearColorNode {
        child: impl UiNode,
        #[var] clear_color: impl Var<Rgba>,
    })]
    impl UiNode for ClearColorNode {
        fn update(&mut self, updates: &WidgetUpdates) {
            if self.clear_color.is_new() {
                WIDGET.render_update();
            }
            self.child.update(updates);
        }
        fn render(&mut self, frame: &mut FrameBuilder) {
            frame.set_clear_color(self.clear_color.get().into());
            self.child.render(frame);
        }
        fn render_update(&mut self, update: &mut FrameUpdate) {
            update.set_clear_color(self.clear_color.get().into());
            self.child.render_update(update);
        }
    }
    ClearColorNode {
        child,
        clear_color: color.into_var(),
    }
}

/// Window persistence config.
///
/// See the [`save_state`] property for more details.
///
/// [`save_state`]: fn@save_state
#[derive(Clone, Debug)]
pub enum SaveState {
    /// Save & restore state.
    Enabled {
        /// Config key that identifies the window.
        ///
        /// If `None` a key is generated for the window, using the [`window_key`] method.
        ///
        /// [`window_key`]: Self::window_key
        key: Option<ConfigKey>,
        /// Maximum time to keep the window in the loading state awaiting for the config to load.
        ///
        /// If the config fails to load in this time frame the window is opened in it's default state.
        ///
        /// This is one second by default.
        loading_timeout: Duration,
    },
    /// Don't save & restore state.
    Disabled,
}
impl Default for SaveState {
    /// Enabled, no key, delay 1s.
    fn default() -> Self {
        SaveState::Enabled {
            key: None,
            loading_timeout: 1.secs(),
        }
    }
}
impl SaveState {
    /// Default, enabled, no key, delay 1s.
    pub fn enabled() -> Self {
        Self::default()
    }

    /// Gets the config key used for the window identified by `id`.
    pub fn window_key(&self, id: WindowId) -> Option<ConfigKey> {
        match self {
            SaveState::Enabled { key, .. } => Some(key.clone().unwrap_or_else(|| {
                let name = id.name();
                if name.is_empty() {
                    formatx!("window.sequential({}).state", id.sequential())
                } else {
                    formatx!("window.{name}.state")
                }
            })),
            SaveState::Disabled => None,
        }
    }

    /// Get the `loading_timeout` if is enabled and the duration is greater than zero.
    pub fn loading_timeout(&self) -> Option<Duration> {
        match self {
            SaveState::Enabled { loading_timeout, .. } => {
                if *loading_timeout == Duration::ZERO {
                    None
                } else {
                    Some(*loading_timeout)
                }
            }
            SaveState::Disabled => None,
        }
    }

    /// Returns `true` if is enabled.
    pub fn is_enabled(&self) -> bool {
        match self {
            SaveState::Enabled { .. } => true,
            SaveState::Disabled => false,
        }
    }
}
impl_from_and_into_var! {
    /// Convert `true` to default config and `false` to `None`.
    fn from(persist: bool) -> SaveState {
        if persist {
            SaveState::default()
        } else {
            SaveState::Disabled
        }
    }
}

/// Save and restore the window state.
///
/// If enabled a config entry is created for the window state in [`CONFIG`], and if a config backend is set
/// the window state is persisted on change and restored when the app reopens.
///
/// This property is enabled by default in the `Window!` widget, it is recommended to open the window with a name if
/// the app can open more than one window.
#[property(CONTEXT, default(SaveState::Disabled), widget_impl(Window))]
pub fn save_state(child: impl UiNode, enabled: impl IntoValue<SaveState>) -> impl UiNode {
    enum Task {
        None,
        Read {
            rsp: ResponseVar<Option<WindowStateCfg>>,
            #[allow(dead_code)] // hold handle alive
            loading: Option<WindowLoadingHandle>,
        },
    }

    #[ui_node(struct SaveStateNode {
        child: impl UiNode,
        enabled: SaveState,

        task: Task,
    })]
    impl UiNode for SaveStateNode {
        fn init(&mut self) {
            if let Some(key) = self.enabled.window_key(WINDOW.id()) {
                let vars = WINDOW_CTRL.vars();
                WIDGET
                    .sub_event(&WINDOW_LOAD_EVENT)
                    .sub_var(&vars.state())
                    .sub_var(&vars.restore_rect());

                let rsp = CONFIG.read(key);
                let loading = self.enabled.loading_timeout().and_then(|t| WINDOW_CTRL.loading_handle(t));
                rsp.subscribe(WIDGET.id()).perm();

                self.task = Task::Read { rsp, loading };
            }

            self.child.init();
        }

        fn deinit(&mut self) {
            self.child.deinit();
        }

        fn event(&mut self, update: &EventUpdate) {
            self.child.event(update);
            if WINDOW_LOAD_EVENT.has(update) {
                self.task = Task::None;
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if let Task::Read { rsp, .. } = &mut self.task {
                if let Some(rsp) = rsp.rsp() {
                    if let Some(s) = rsp {
                        let window_vars = WINDOW_CTRL.vars();
                        window_vars.state().set_ne(s.state);
                        let restore_rect: DipRect = s.restore_rect.cast();

                        let visible = MONITORS.available_monitors().iter().any(|m| m.dip_rect().intersects(&restore_rect));
                        if visible {
                            window_vars.position().set_ne(restore_rect.origin);
                        }

                        window_vars.size().set_ne(restore_rect.size);
                    }
                    self.task = Task::None;
                }
            } else if self.enabled.is_enabled() {
                let vars = WINDOW_CTRL.vars();
                if vars.state().is_new() || vars.restore_rect().is_new() {
                    let cfg = WindowStateCfg {
                        state: vars.state().get(),
                        restore_rect: vars.restore_rect().get().cast(),
                    };

                    if let Some(key) = self.enabled.window_key(WINDOW.id()) {
                        CONFIG.write(key, cfg);
                    }
                }
            }
            self.child.update(updates);
        }
    }
    SaveStateNode {
        child,
        enabled: enabled.into(),
        task: Task::None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WindowStateCfg {
    state: WindowState,
    restore_rect: euclid::Rect<f32, Dip>,
}
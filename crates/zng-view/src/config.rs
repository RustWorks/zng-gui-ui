use zng_view_api::config::{AnimationsConfig, ColorScheme, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig, TouchConfig};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod dconf;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use dconf as platform;

mod other;
#[cfg(not(any(
    windows,
    target_os = "macos",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
use other as platform;

pub fn font_aa() -> FontAntiAliasing {
    platform::font_aa()
}

pub fn multi_click_config() -> MultiClickConfig {
    platform::multi_click_config()
}

pub fn animations_config() -> AnimationsConfig {
    platform::animations_config()
}

pub fn key_repeat_config() -> KeyRepeatConfig {
    platform::key_repeat_config()
}

pub fn touch_config() -> TouchConfig {
    platform::touch_config()
}

pub fn color_scheme_config() -> ColorScheme {
    platform::color_scheme_config()
}

pub fn locale_config() -> LocaleConfig {
    platform::locale_config()
}

/// Return handle must be called on exit.
#[must_use]
pub fn spawn_listener(event_loop: crate::AppEventSender) -> Option<Box<dyn FnOnce()>> {
    platform::spawn_listener(event_loop)
}

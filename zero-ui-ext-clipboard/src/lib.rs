//! Clipboard app extension, service and commands.
//!
//! This module is a thin wrapper around the [`VIEW_PROCESS`] provided clipboard service.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::path::PathBuf;

use zero_ui_app::{
    event::{command, CommandInfoExt, CommandNameExt},
    shortcut::{shortcut, CommandShortcutExt, ShortcutFilter},
    view_process::{ViewClipboard, VIEW_PROCESS},
    AppExtension,
};
use zero_ui_ext_image::{ImageHasher, ImageVar, Img, IMAGES};
use zero_ui_txt::Txt;
use zero_ui_view_api::ViewProcessOffline;

use zero_ui_view_api::clipboard as clipboard_api;
use zero_ui_view_api::ipc::IpcBytes;

/// Clipboard app extension.
///
/// # Services
///
/// Services provided by this extension.
///
/// * [`CLIPBOARD`]
#[derive(Default)]
pub struct ClipboardManager {}

impl AppExtension for ClipboardManager {}

/// Error getting or setting the clipboard.
///
/// The [`CLIPBOARD`] service already logs the error.
#[derive(Debug, Clone)]
pub enum ClipboardError {
    /// No view-process available to process the request.
    ViewProcessOffline,
    /// View-process or operating system does not support the data type.
    NotSupported,
    /// Cannot set image in clipboard because it has not finished loading or loaded with error.
    ImageNotLoaded,
    /// Other error.
    ///
    /// The string can be a debug description of the error, only suitable for logging.
    Other(Txt),
}

/// Clipboard service.
///
/// This service is a thin wrapper around the [`VIEW_PROCESS`] provided clipboard service. This means
/// the clipboard will not work in headless app without renderer mode.
pub struct CLIPBOARD;
impl CLIPBOARD {
    fn view(&self) -> Result<&ViewClipboard, ClipboardError> {
        match VIEW_PROCESS.clipboard() {
            Ok(c) => Ok(c),
            Err(ViewProcessOffline) => Err(ClipboardError::ViewProcessOffline),
        }
    }

    fn get<T>(
        &self,
        getter: impl FnOnce(&ViewClipboard) -> Result<Result<T, clipboard_api::ClipboardError>, ViewProcessOffline>,
    ) -> Result<Option<T>, ClipboardError> {
        let r = self.view().and_then(|v| match getter(v) {
            Ok(r) => match r {
                Ok(r) => Ok(Some(r)),
                Err(e) => match e {
                    clipboard_api::ClipboardError::NotFound => Ok(None),
                    clipboard_api::ClipboardError::NotSupported => Err(ClipboardError::NotSupported),
                    clipboard_api::ClipboardError::Other(e) => Err(ClipboardError::Other(e)),
                },
            },
            Err(ViewProcessOffline) => Err(ClipboardError::ViewProcessOffline),
        });
        if let Err(e) = &r {
            tracing::error!("clipboard get error, {e:?}");
        }
        r
    }

    fn set(
        &self,
        setter: impl FnOnce(&ViewClipboard) -> Result<Result<(), clipboard_api::ClipboardError>, ViewProcessOffline>,
    ) -> Result<(), ClipboardError> {
        let r = self.view().and_then(|v| match setter(v) {
            Ok(r) => match r {
                Ok(()) => Ok(()),
                Err(e) => match e {
                    clipboard_api::ClipboardError::NotFound => {
                        Err(ClipboardError::Other(Txt::from_static("not found error in set operation")))
                    }
                    clipboard_api::ClipboardError::NotSupported => Err(ClipboardError::NotSupported),
                    clipboard_api::ClipboardError::Other(e) => Err(ClipboardError::Other(e)),
                },
            },
            Err(ViewProcessOffline) => Err(ClipboardError::ViewProcessOffline),
        });
        if let Err(e) = &r {
            tracing::error!("clipboard set error, {e:?}");
        }
        r
    }

    /// Gets a text string from the clipboard.
    pub fn text(&self) -> Result<Option<Txt>, ClipboardError> {
        self.get(|v| v.read_text()).map(|s| s.map(|s| Txt::from_str(&s)))
    }
    /// Sets the text string on the clipboard, returns `Ok(())` if the operation succeeded.
    pub fn set_text(&self, txt: impl Into<Txt>) -> Result<(), ClipboardError> {
        self.set(|v| v.write_text(txt.into()))
    }

    /// Gets an image from the clipboard.
    ///
    /// The image is loaded in parallel and cached by the [`IMAGES`] service.
    pub fn image(&self) -> Result<Option<ImageVar>, ClipboardError> {
        self.get(|v| v.read_image()).map(|i| {
            i.map(|img| {
                let mut hash = ImageHasher::new();
                hash.update("zero_ui_core::CLIPBOARD");
                hash.update(img.id().unwrap().get().to_be_bytes());

                match IMAGES.register(hash.finish(), img) {
                    Ok(r) => r,
                    Err((_, r)) => r,
                }
            })
        })
    }

    /// Set the image on the clipboard if it is loaded.
    pub fn set_image(&self, img: &Img) -> Result<(), ClipboardError> {
        if let Some(img) = img.view() {
            self.set(|v| v.write_image(img))
        } else {
            Err(ClipboardError::ImageNotLoaded)
        }
    }

    /// Gets a file list from the clipboard.
    pub fn file_list(&self) -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        self.get(|v| v.read_file_list())
    }

    /// Sets the file list on the clipboard.
    pub fn set_file_list(&self, list: impl Into<Vec<PathBuf>>) -> Result<(), ClipboardError> {
        self.set(|v| v.write_file_list(list.into()))
    }

    /// Gets custom data from the clipboard.
    ///
    /// The current view-process must support `data_type`.
    pub fn extension(&self, data_type: impl Into<Txt>) -> Result<Option<IpcBytes>, ClipboardError> {
        self.get(|v| v.read_extension(data_type.into()))
    }

    /// Set a custom data on the clipboard.
    ///
    /// The current view-process must support `data_type`.
    pub fn set_extension(&self, data_type: impl Into<Txt>, data: IpcBytes) -> Result<(), ClipboardError> {
        self.set(|v| v.write_extension(data_type.into(), data))
    }
}

command! {
    /// Represents the clipboard **cut** action.
    pub static CUT_CMD = {
        name: "Cut",
        info: "Remove the selection and place it in the clipboard.",
        shortcut: [shortcut!(CTRL+'X'), shortcut!(SHIFT+Delete), shortcut!(Cut)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the clipboard **copy** action.
    pub static COPY_CMD = {
        name: "Copy",
        info: "Place a copy of the selection in the clipboard.",
        shortcut: [shortcut!(CTRL+'C'), shortcut!(CTRL+Insert), shortcut!(Copy)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Represents the clipboard **paste** action.
    pub static PASTE_CMD = {
        name: "Paste",
        info: "Insert content from the clipboard.",
        shortcut: [shortcut!(CTRL+'V'), shortcut!(SHIFT+Insert), shortcut!(Paste)],
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };
}
//! Ready-made message dialogs.

use super::{DialogWrapper, MessageIcon};

/// Builders for the common message dialogs: IntelliJ's `Messages.showInfoMessage`,
/// `showErrorDialog`, `showYesNoDialog` and friends. They return a [`DialogWrapper`] you
/// show while your flag is set, like any other: nothing here blocks.
///
/// Every dialog has the icon of its kind left of the message, and the buttons IntelliJ
/// gives it. In a yes/no dialog <kbd>Y</kbd> answers yes ([`ExitCode::Primary`], IntelliJ's
/// `YES` = `OK_EXIT_CODE`) and <kbd>N</kbd> no ([`ExitCode::Secondary`], `NO` =
/// `CANCEL_EXIT_CODE`).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::openapi::ui::{ExitCode, Messages};
///
/// let mut asking = true;
/// if asking {
///     match Messages::yes_no_dialog("Delete branch?", "The branch “feature” has unmerged commits.")
///         .show(ui.ctx())
///     {
///         Some(ExitCode::Primary) => { asking = false; /* delete it */ }
///         Some(_) => asking = false,
///         None => {}
///     }
/// }
/// # });
/// ```
///
/// [`ExitCode::Primary`]: super::ExitCode::Primary
/// [`ExitCode::Secondary`]: super::ExitCode::Secondary
#[derive(Debug, Clone, Copy)]
pub struct Messages;

impl Messages {
    /// An information message with an OK button (`showInfoMessage`).
    pub fn info_dialog<'a>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> DialogWrapper<'a> {
        DialogWrapper::new(title)
            .message(message)
            .message_icon(MessageIcon::Information)
            .primary("OK")
    }

    /// A warning with an OK button (`showWarningDialog`).
    pub fn warning_dialog<'a>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> DialogWrapper<'a> {
        DialogWrapper::new(title)
            .message(message)
            .message_icon(MessageIcon::Warning)
            .primary("OK")
    }

    /// An error with an OK button (`showErrorDialog`).
    pub fn error_dialog<'a>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> DialogWrapper<'a> {
        DialogWrapper::new(title)
            .message(message)
            .message_icon(MessageIcon::Error)
            .primary("OK")
    }

    /// A question with OK and Cancel (`showOkCancelDialog`).
    pub fn ok_cancel_dialog<'a>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> DialogWrapper<'a> {
        DialogWrapper::new(title)
            .message(message)
            .message_icon(MessageIcon::Question)
            .primary("OK")
            .secondary("Cancel")
    }

    /// A question with Yes and No (`showYesNoDialog`), answered by the keys <kbd>Y</kbd>
    /// and <kbd>N</kbd> as well.
    pub fn yes_no_dialog<'a>(
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> DialogWrapper<'a> {
        DialogWrapper::new(title)
            .message(message)
            .message_icon(MessageIcon::Question)
            .primary("Yes")
            .secondary("No")
            .yes_no_keys(true)
    }
}

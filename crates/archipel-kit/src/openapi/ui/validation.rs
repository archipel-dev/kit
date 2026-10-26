//! Dialog validation: what is wrong with the input, found a moment after typing stops.

use crate::tokens::motion;
use std::hash::{Hash, Hasher};

/// One problem found in a dialog's input: IntelliJ's `ValidationInfo`.
///
/// An error blocks the dialog's OK button; a [warning](ValidationInfo::warning) or an error
/// [marked](ValidationInfo::with_ok_enabled) `with_ok_enabled` only informs.
///
/// ```
/// use archipel_kit::openapi::ui::ValidationInfo;
///
/// let problem = ValidationInfo::error("Name must not be empty");
/// assert!(problem.blocks_ok());
/// assert!(!ValidationInfo::error("Name is long").with_ok_enabled().blocks_ok());
/// assert!(!ValidationInfo::warning("Name already exists").blocks_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidationInfo {
    /// What is wrong, as a sentence.
    pub message: String,
    /// The field the message is about: it gets the focus when the user confirms despite the
    /// problem (IntelliJ `component`).
    pub field: Option<egui::Id>,
    /// A warning does not block OK (IntelliJ `asWarning`).
    pub warning: bool,
    /// An error that does not block OK (IntelliJ `withOKEnabled`).
    pub ok_enabled: bool,
}

impl ValidationInfo {
    /// An error: the dialog cannot be confirmed until it is fixed.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field: None,
            warning: false,
            ok_enabled: false,
        }
    }

    /// A warning: shown in the warning color, OK stays enabled.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            warning: true,
            ..Self::error(message)
        }
    }

    /// Attaches the message to the field with this id.
    pub fn for_field(mut self, field: egui::Id) -> Self {
        self.field = Some(field);
        self
    }

    /// Keeps OK enabled in spite of this message.
    pub fn with_ok_enabled(mut self) -> Self {
        self.ok_enabled = true;
        self
    }

    /// Whether this message keeps the dialog from being confirmed.
    pub fn blocks_ok(&self) -> bool {
        !self.warning && !self.ok_enabled
    }
}

/// The validation state of a dialog, owned by the caller: the problems found so far and
/// the pending check. Keep it next to the dialog's input.
///
/// [`DialogValidation::update`] runs your check on the first frame, and after that
/// [300 ms](motion::DIALOG_VALIDATION_DELAY) after the input last changed (IntelliJ's
/// validation delay), so typing is not interrupted by messages for half-typed values. Show the
/// result with [`DialogWrapper::validation`](super::DialogWrapper::validation); the dialog
/// shows the messages above its buttons and disables OK while one blocks.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use archipel_kit::openapi::ui::{DialogValidation, DialogWrapper, ValidationInfo};
///
/// struct NewBranch { name: String, validation: DialogValidation }
/// let mut state = NewBranch { name: String::new(), validation: DialogValidation::new() };
///
/// state.validation.update(ui.ctx(), &state.name, || {
///     if state.name.trim().is_empty() {
///         vec![ValidationInfo::error("Enter a branch name")]
///     } else {
///         vec![]
///     }
/// });
/// let _ = DialogWrapper::new("New Branch")
///     .primary("Create")
///     .validation(&state.validation)
///     .show(ui.ctx());
/// # });
/// ```
#[derive(Debug, Clone, Default)]
pub struct DialogValidation {
    infos: Vec<ValidationInfo>,
    fingerprint: Option<u64>,
    changed_at: Option<f64>,
}

impl DialogValidation {
    /// A validation that has found nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The problems found by the last check.
    pub fn infos(&self) -> &[ValidationInfo] {
        &self.infos
    }

    /// Whether a problem keeps the dialog from being confirmed.
    pub fn blocks_ok(&self) -> bool {
        self.infos.iter().any(ValidationInfo::blocks_ok)
    }

    /// Whether the input changed and the check has not run on it yet.
    pub fn is_pending(&self) -> bool {
        self.changed_at.is_some()
    }

    /// Call it every frame with the input the check looks at. When `input` differs from the
    /// last frame's, a check is scheduled; when the schedule is due `validate` runs and its
    /// result replaces the problems. Returns whether `validate` ran.
    ///
    /// The first call checks at once.
    pub fn update<T: Hash + ?Sized>(
        &mut self,
        ctx: &egui::Context,
        input: &T,
        validate: impl FnOnce() -> Vec<ValidationInfo>,
    ) -> bool {
        let now = ctx.input(|i| i.time);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        input.hash(&mut hasher);
        let fingerprint = hasher.finish();
        if self.fingerprint.is_none() {
            self.fingerprint = Some(fingerprint);
            self.infos = validate();
            return true;
        }
        if self.fingerprint != Some(fingerprint) {
            self.fingerprint = Some(fingerprint);
            self.changed_at = Some(now);
        }
        let Some(changed_at) = self.changed_at else {
            return false;
        };
        let delay = f64::from(motion::DIALOG_VALIDATION_DELAY);
        if now - changed_at >= delay {
            self.changed_at = None;
            self.infos = validate();
            true
        } else {
            ctx.request_repaint_after_secs((delay - (now - changed_at)) as f32);
            false
        }
    }

    /// Replaces the problems at once and cancels a pending check. Use it when the user
    /// confirms: the last input may not be validated yet (IntelliJ's `doOKAction` validates
    /// again). Returns whether the dialog may be confirmed.
    pub fn validate_now(&mut self, infos: Vec<ValidationInfo>) -> bool {
        self.infos = infos;
        self.changed_at = None;
        !self.blocks_ok()
    }
}

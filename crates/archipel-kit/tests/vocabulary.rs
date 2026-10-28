//! The component vocabulary shared with the terminal kit (TUI-CMP-010): the egui kit lists
//! what it does not render from the model's props yet, and the list only goes down.

use archipel_kit::model::vocabulary::{PENDING_EGUI, VOCABULARY};

/// The most entries `PENDING_EGUI` may have. Lower it when an egui component takes the
/// model's props; never raise it.
const PENDING_BASELINE: usize = 15;

#[test]
fn pending_egui_components_are_in_the_vocabulary_and_do_not_grow() {
    let names: Vec<_> = VOCABULARY.iter().map(|entry| entry.name).collect();
    for pending in PENDING_EGUI {
        assert!(
            names.contains(pending),
            "{pending} is not in the vocabulary"
        );
    }
    assert!(
        PENDING_EGUI.len() <= PENDING_BASELINE,
        "{} pending components: {PENDING_EGUI:?}",
        PENDING_EGUI.len()
    );
    eprintln!("egui components pending: {}", PENDING_EGUI.len());
}

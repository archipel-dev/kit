//! Regressions found during native gallery and PR review.
use archipel_kit::{
    Theme,
    testing::{self, Queryable},
    ui::{Bucket, Histogram},
};

#[test]
fn large_counts_and_non_tooltip_behaviour_keep_hover_tooltip_stable() {
    let buckets = [Bucket::new("large", u64::MAX), Bucket::new("small", 1)];
    let mut h = testing::harness(Theme::dark(), |ui| {
        Histogram::new("counts", &buckets)
            .with(|_: &mut egui::Ui, response: egui::Response| response)
            .show(ui);
    });
    h.set_size(egui::vec2(400.0, 260.0));
    h.run();
    let rect = h.get_by_label_contains("Histogram of counts").rect();
    h.hover_at(egui::pos2(
        rect.left() + rect.width() * 0.25,
        rect.top() + 30.0,
    ));
    h.run_steps(60);
    for _ in 0..6 {
        h.run_steps(1);
        assert!(
            h.query_by_label("large").is_some(),
            "tooltip stays visible every frame"
        );
        assert!(h.query_by_label_contains("100.0%").is_some());
    }
}

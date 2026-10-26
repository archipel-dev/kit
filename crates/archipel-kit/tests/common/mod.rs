//! Shared by the component tests.

/// Snapshots double as the images of `docs/components/*.md`.
#[allow(dead_code)]
pub const IMG: &str = "../../docs/components/img";

/// Moves the harness clock forward by `seconds` and runs one frame (tests never sleep).
#[allow(dead_code)]
pub fn advance<S>(harness: &mut archipel_kit::testing::Harness<'_, S>, seconds: f64) {
    let now = harness.ctx.input(|i| i.time);
    harness.input_mut().time = Some(now + seconds);
    harness.step();
}

/// Runs frames until `seconds` have passed on the harness clock, in steps of 50 ms.
#[allow(dead_code)]
pub fn advance_by<S>(harness: &mut archipel_kit::testing::Harness<'_, S>, seconds: f64) {
    let mut left = seconds;
    while left > 1e-9 {
        let step = left.min(0.05);
        advance(harness, step);
        left -= step;
    }
}

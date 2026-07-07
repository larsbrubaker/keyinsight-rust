//! The user interface layer: agg-gui widgets mirroring the SwiftUI views.
//!
//! Ports `Sources/KeyInSight/UI/` — arriving with the UI phase of the port;
//! pure geometry (KeyboardLayout, calibration median) lands first because
//! the engine and tests depend on it.

mod keyboard_layout;

pub use keyboard_layout::{KeyboardKey, KeyboardLayout};

/// Median of a sample list — `CalibrationSheet.median` in Swift (the
/// calibration flow's latency estimator). Lives here until the full
/// CalibrationSheet widget is ported.
pub fn median(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "median needs at least one sample");
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite samples"));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

#[cfg(test)]
mod tests {
    /// From TimelineTests in `TempoTests.swift` (`medianCalculation`).
    #[test]
    fn median_calculation() {
        assert_eq!(super::median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(super::median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
        assert_eq!(super::median(&[10.0]), 10.0);
    }
}

//! Pure keyboard geometry: keys with fractional frames (x/width in 0…1),
//! white keys first so black keys draw on top. Testable without any UI.
//!
//! Ports the `KeyboardLayout` struct from `UI/PianoKeyboardView.swift`
//! (the SwiftUI view itself becomes an agg-gui widget in the UI phase).

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyboardKey {
    pub midi: u8,
    pub is_black: bool,
    /// Fractional horizontal position and width (0…1 across the strip).
    pub x: f64,
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardLayout {
    pub low_midi: u8,
    pub high_midi: u8,
    pub keys: Vec<KeyboardKey>,
}

fn is_white_pc(midi: i32) -> bool {
    matches!(midi % 12, 0 | 2 | 4 | 5 | 7 | 9 | 11)
}

impl KeyboardLayout {
    /// Snaps the range outward to full octaves (C…B) and guarantees at
    /// least two octaves so the strip doesn't jump around between
    /// exercises.
    pub fn covering(low: u8, high: u8) -> Self {
        let mut lo = low as i32 - (low as i32) % 12; // down to C
        let mut hi = high as i32 + (11 - (high as i32) % 12); // up to B
        while hi - lo < 23 {
            // at least 2 octaves
            if lo >= 12 {
                lo -= 12;
            }
            if hi - lo < 23 {
                hi += 12;
            }
        }
        let low_midi = lo.max(0) as u8;
        let high_midi = hi.min(127) as u8;

        let whites: Vec<i32> = (low_midi as i32..=high_midi as i32)
            .filter(|&m| is_white_pc(m))
            .collect();
        let white_width = 1.0 / whites.len() as f64;
        let white_index = |midi: i32| whites.iter().position(|&w| w == midi);

        let mut white: Vec<KeyboardKey> = Vec::new();
        let mut black: Vec<KeyboardKey> = Vec::new();
        for midi in low_midi as i32..=high_midi as i32 {
            if let Some(index) = white_index(midi) {
                white.push(KeyboardKey {
                    midi: midi as u8,
                    is_black: false,
                    x: index as f64 * white_width,
                    width: white_width,
                });
            } else if let Some(left_white) = white_index(midi - 1) {
                // Black key straddles the boundary after its left white.
                let width = white_width * 0.62;
                black.push(KeyboardKey {
                    midi: midi as u8,
                    is_black: true,
                    x: (left_white + 1) as f64 * white_width - width / 2.0,
                    width,
                });
            }
        }
        let mut keys = white;
        keys.extend(black);
        Self {
            low_midi,
            high_midi,
            keys,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Ports `Tests/KeyInSightTests/KeyboardLayoutTests.swift`.

    use super::*;

    #[test]
    fn snaps_to_full_octaves_with_two_octave_minimum() {
        // A tight range (E4..G4) still yields at least two octaves, C-aligned.
        let layout = KeyboardLayout::covering(64, 67);
        assert_eq!(layout.low_midi % 12, 0); // starts on a C
        assert_eq!(layout.high_midi % 12, 11); // ends on a B
        assert!(layout.high_midi as i32 - layout.low_midi as i32 >= 23);
    }

    #[test]
    fn white_and_black_key_counts_for_two_octaves() {
        let layout = KeyboardLayout::covering(60, 83); // C4..B5
        let whites: Vec<&KeyboardKey> = layout.keys.iter().filter(|k| !k.is_black).collect();
        let blacks: Vec<&KeyboardKey> = layout.keys.iter().filter(|k| k.is_black).collect();
        assert_eq!(whites.len(), 14);
        assert_eq!(blacks.len(), 10);
        // White keys tile the strip exactly.
        let total_white: f64 = whites.iter().map(|k| k.width).sum();
        assert!((total_white - 1.0).abs() < 0.001);
        // Black keys draw after whites (on top) and sit within bounds.
        assert!(layout.keys[layout.keys.len() - blacks.len()..]
            .iter()
            .all(|k| k.is_black));
        assert!(blacks.iter().all(|k| k.x > 0.0 && k.x + k.width < 1.0));
    }

    #[test]
    fn covers_requested_range() {
        let layout = KeyboardLayout::covering(35, 84); // B1..C6
        assert!(layout.low_midi <= 35 && layout.high_midi >= 84);
        assert!(layout.keys.iter().any(|k| k.midi == 60)); // middle C present
    }
}

//! How a made chip differs from its drawing.

use std::collections::BTreeMap;

/// One way a chip may come out: each layer's height and the widths of its features off nominal.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Variation {
    /// Each layer's height as a multiple of nominal; one when the layer is not named.
    pub height_scale: BTreeMap<String, f64>,
    /// What is added to the width of every duct of a layer, in metres.
    pub width_bias: BTreeMap<String, f64>,
    /// A further width error of each duct on its own: its standard deviation in metres, and the
    /// seed its values are drawn from.
    pub mismatch: Option<(f64, u64)>,
}

impl Variation {
    /// The chip as drawn.
    pub fn nominal() -> Self {
        Self::default()
    }

    /// The height and width of duct number `index`, drawn `width` wide on `layer` of height `height`.
    pub(crate) fn of(&self, layer: &str, height: f64, width: f64, index: u64) -> (f64, f64) {
        let own = self.mismatch.map_or(0.0, |(sigma, seed)| sigma * normal(seed, index));
        (
            height * self.height_scale.get(layer).copied().unwrap_or(1.0),
            width + self.width_bias.get(layer).copied().unwrap_or(0.0) + own,
        )
    }
}

/// A uniform value in (0, 1) fixed by `seed` and `index`.
fn uniform(seed: u64, index: u64) -> f64 {
    let mut state = seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    state = (state ^ (state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((state ^ (state >> 31)) >> 11) as f64 / (1_u64 << 53) as f64 + f64::EPSILON
}

/// A standard normal value fixed by `seed` and `index`.
pub(crate) fn normal(seed: u64, index: u64) -> f64 {
    let (first, second) = (uniform(seed, 2 * index), uniform(seed, 2 * index + 1));
    (-2.0 * first.ln()).sqrt() * (std::f64::consts::TAU * second).cos()
}

#[cfg(test)]
mod tests {
    use crate::variation::{Variation, normal};

    #[test]
    fn a_variation_changes_height_by_layer_and_width_by_layer_and_by_duct() {
        let variation = Variation {
            height_scale: [("flow".to_owned(), 1.1)].into(),
            width_bias: [("flow".to_owned(), 1e-6)].into(),
            mismatch: None,
        };
        assert_eq!(Variation::nominal().of("flow", 20e-6, 100e-6, 0), (20e-6, 100e-6));
        let (height, width) = variation.of("flow", 20e-6, 100e-6, 0);
        assert!((height - 22e-6).abs() < 1e-15 && (width - 101e-6).abs() < 1e-15);
        assert_eq!(variation.of("trench", 1e-6, 1e-6, 0), (1e-6, 1e-6));
        let scattered = Variation {
            mismatch: Some((0.3e-6, 7)),
            ..Variation::default()
        };
        assert_ne!(
            scattered.of("flow", 20e-6, 100e-6, 0).1,
            scattered.of("flow", 20e-6, 100e-6, 1).1
        );
        assert_eq!(
            scattered.of("flow", 20e-6, 100e-6, 5),
            scattered.of("flow", 20e-6, 100e-6, 5)
        );
    }

    #[test]
    fn the_seeded_normal_values_have_zero_mean_and_unit_spread() {
        let values: Vec<f64> = (0..20_000).map(|index| normal(42, index)).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let spread = (values.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt();
        assert!(mean.abs() < 0.03 && (spread - 1.0).abs() < 0.03, "{mean} {spread}");
    }
}

//! What a process says about each mask and about how masks print.

use serde::{Deserialize, Serialize};

/// Which part of a mask lets light through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    /// Clear inside the outlines: a negative resist stays where features are.
    #[default]
    Clear,
    /// Opaque inside the outlines, for a positive resist.
    Dark,
}

/// How one layer's mask is made from its drawing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaskSpec {
    pub layer: String,
    pub tone: Tone,
    /// Whether the mask is written as the mirror image of the drawing.
    pub mirror: bool,
    /// How far every edge is moved outward on the mask, in metres; negative moves it inward.
    pub bias: f64,
}

impl MaskSpec {
    /// A mask drawn as the layer is: clear, unmirrored, with no bias.
    pub fn plain(layer: &str) -> Self {
        Self {
            layer: layer.to_owned(),
            tone: Tone::Clear,
            mirror: false,
            bias: 0.0,
        }
    }
}

/// A first-order picture of how a mask prints: the exposure is the mask blurred, and resist
/// stays where the exposure passes a threshold.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PrintModel {
    /// The standard deviation of the blur, in metres.
    pub blur: f64,
    /// The exposure, as a fraction of that far inside a large feature, at which resist stays.
    pub threshold: f64,
    /// How far a printed width may be from its drawn one, in metres.
    pub tolerance: f64,
    /// Whether the blur and threshold were fitted to measurements of this process.
    pub calibrated: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WrittenMask {
    layer: String,
    #[serde(default)]
    tone: Tone,
    #[serde(default)]
    mirror: bool,
    #[serde(default)]
    bias_um: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WrittenPrint {
    blur_um: f64,
    threshold: f64,
    tolerance_um: f64,
    #[serde(default)]
    calibrated: bool,
}

impl From<WrittenMask> for MaskSpec {
    fn from(written: WrittenMask) -> Self {
        Self {
            layer: written.layer,
            tone: written.tone,
            mirror: written.mirror,
            bias: written.bias_um * 1e-6,
        }
    }
}

impl WrittenPrint {
    /// The model, or why what is written cannot be one.
    pub(crate) fn model(&self) -> Result<PrintModel, String> {
        if self.blur_um <= 0.0 || self.tolerance_um <= 0.0 {
            return Err("the print model's blur and tolerance must be more than zero".to_owned());
        }
        if !(self.threshold > 0.0 && self.threshold < 1.0) {
            return Err(format!(
                "the print model's threshold is {}, and must lie between 0 and 1",
                self.threshold
            ));
        }
        Ok(PrintModel {
            blur: self.blur_um * 1e-6,
            threshold: self.threshold,
            tolerance: self.tolerance_um * 1e-6,
            calibrated: self.calibrated,
        })
    }
}

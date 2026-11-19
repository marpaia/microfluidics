//! The layers a chip is built from.

use serde::Serialize;

/// One layer of the master: every feature on it is built to the same height.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayerSpec {
    pub name: String,
    /// In metres: the height of the layer on the master, and the depth of its features in the chip.
    pub height: f64,
    /// The layer's number in a GDSII file.
    pub gds: u16,
}

/// A design's layers in build order, shallowest first.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct LayerStack(Vec<LayerSpec>);

impl LayerStack {
    /// The layers, ordered by height. Fails when two share a name or a GDSII number.
    pub fn new(mut layers: Vec<LayerSpec>) -> Result<Self, String> {
        layers.sort_by(|a, b| a.height.total_cmp(&b.height));
        for (at, layer) in layers.iter().enumerate() {
            if layers[..at].iter().any(|held| held.name == layer.name) {
                return Err(format!("layer `{}` is stated twice", layer.name));
            }
            if let Some(held) = layers[..at].iter().find(|held| held.gds == layer.gds) {
                return Err(format!(
                    "layers `{}` and `{}` share GDSII number {}",
                    held.name, layer.name, layer.gds
                ));
            }
        }
        Ok(Self(layers))
    }

    pub fn get(&self, name: &str) -> Option<&LayerSpec> {
        self.0.iter().find(|layer| layer.name == name)
    }

    /// The height of the layer `name`, or zero for a layer the stack does not have.
    pub fn height(&self, name: &str) -> f64 {
        self.get(name).map_or(0.0, |layer| layer.height)
    }

    pub fn iter(&self) -> impl Iterator<Item = &LayerSpec> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

//! The faults a chip's layout can have.

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LayoutError {
    #[error("the chip has no features")]
    Empty,
    #[error("`{0}` has no area")]
    Degenerate(String),
    #[error("`{0}` and `{1}` overlap")]
    Overlap(String, String),
    #[error("`{0}` lies outside the chip's outline")]
    OutsideChip(String),
    #[error("layer `{layer}` is {height_um} um high, which leaves no roof in a chip {thickness_um} um thick")]
    TooDeep {
        layer: String,
        height_um: f64,
        thickness_um: f64,
    },
    #[error("the slab, {} x {} mm, does not fit on the glass, {} x {} mm", slab_mm[0], slab_mm[1], glass_mm[0], glass_mm[1])]
    OffGlass { slab_mm: [f64; 2], glass_mm: [f64; 2] },
}

//! Microfluidic chip layout: structures elaborated into a chip, its check against a fabrication
//! process, and its photomasks.

mod bias;
mod builder;
mod bundled;
mod chip;
mod connectivity;
mod error;
mod export;
mod fingerprint;
#[cfg(test)]
mod fixtures;
mod format;
mod gds;
mod geometry;
mod label;
mod mask;
mod maskspec;
mod plan;
mod print;
mod process;
mod rectangle;
mod route;
mod rules;
mod structure;
mod wafer;

pub use crate::builder::{
    Build, BuildError, Builder, Elaborated, Instance, Origin, Overrides, ParamSpec, Place, RegionSpec, Tunable,
};
pub use crate::chip::Chip;
pub use crate::connectivity::{Connectivity, Net, connectivity};
pub use crate::error::LayoutError;
pub use crate::export::{ExportError, Exported, export};
pub use crate::fingerprint::fingerprint;
pub use crate::format::{Dxf, Format, Gds, Svg, read_dxf};
pub use crate::gds::{
    GdsError, GdsLibrary, GdsStructure, Reference, read as gds_read, readback as gds_readback, write as gds_write,
};
pub use crate::geometry::{Direction, Point, Rect, TOLERANCE};
pub use crate::mask::{Mask, MaskSet, prepare};
pub use crate::maskspec::{MaskSpec, PrintModel, Tone};
pub use crate::plan::Layout;
pub use crate::print::{check as print_check, solve_bias};
pub use crate::process::{Process, ProcessError, Rule, RuleKind, Tolerances};
pub use crate::rectangle::{Port, Rectangle};
pub use crate::route::{Routed, route};
pub use crate::rules::{Features, check, waivers};
pub use crate::structure::{Structure, Translates};
pub use crate::wafer::{Block, Placed, Wafer, WaferError, WaferSpec, assemble};

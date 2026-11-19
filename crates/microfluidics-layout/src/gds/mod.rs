//! GDSII: the stream format mask shops read.
//!
//! One file holds every layer: a structure for each structure, and a top structure `CHIP` with the
//! chip's own outlines and an array reference for each array. Coordinates are whole nanometres;
//! one user unit is a micrometre.

mod read;
mod write;

use std::collections::BTreeMap;

use microfluidics_core::{Check, Finding};

pub use crate::gds::read::read;
pub use crate::gds::write::write;
use crate::mask::MaskSet;

/// The name of the structure that holds the whole chip.
pub const TOP: &str = "CHIP";
/// The most columns one array reference holds.
pub(crate) const MAX_COLUMNS: u32 = 32_767;
/// How deep references may nest when a file is flattened.
pub(crate) const MAX_DEPTH: usize = 16;

pub(crate) const HEADER: u16 = 0x0002;
pub(crate) const BGNLIB: u16 = 0x0102;
pub(crate) const LIBNAME: u16 = 0x0206;
pub(crate) const UNITS: u16 = 0x0305;
pub(crate) const ENDLIB: u16 = 0x0400;
pub(crate) const BGNSTR: u16 = 0x0502;
pub(crate) const STRNAME: u16 = 0x0606;
pub(crate) const ENDSTR: u16 = 0x0700;
pub(crate) const BOUNDARY: u16 = 0x0800;
pub(crate) const SREF: u16 = 0x0A00;
pub(crate) const AREF: u16 = 0x0B00;
pub(crate) const LAYER: u16 = 0x0D02;
pub(crate) const DATATYPE: u16 = 0x0E02;
pub(crate) const XY: u16 = 0x1003;
pub(crate) const ENDEL: u16 = 0x1100;
pub(crate) const SNAME: u16 = 0x1206;
pub(crate) const COLROW: u16 = 0x1302;
pub(crate) const STRANS: u16 = 0x1A01;
pub(crate) const ANGLE: u16 = 0x1C05;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("not a GDSII file this tool wrote: {0}")]
pub struct GdsError(pub String);

#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub structure: String,
    pub quarter_turns: u8,
    pub columns: u32,
    pub rows: u32,
    /// The origin, the far end of the columns, and the far end of the rows, in nanometres.
    pub lattice: Vec<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GdsStructure {
    pub name: String,
    /// Closed outlines by layer number, in nanometres, without the repeated closing point.
    pub boundaries: Vec<(u16, Vec<(i32, i32)>)>,
    pub references: Vec<Reference>,
}

/// What a GDSII file holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GdsLibrary {
    pub structures: BTreeMap<String, GdsStructure>,
}

/// Read back a written file and compare it with the masks, outline for outline.
pub fn readback(masks: &MaskSet, bytes: &[u8]) -> Check {
    let library = match read(bytes) {
        Ok(library) => library,
        Err(error) => return Check::not_run("gds-readback", &error.to_string()),
    };
    // An outline as its corners in a fixed order, so that where it starts does not matter.
    let canonical = |mut outline: Vec<(i32, i32)>| {
        outline.sort_unstable();
        outline
    };
    let mut findings = Vec::new();
    let mut metrics = BTreeMap::new();
    for mask in &masks.masks {
        let mut written: Vec<Vec<(i32, i32)>> = library.flattened(mask.layer.gds).into_iter().map(canonical).collect();
        let mut planned: Vec<Vec<(i32, i32)>> = (masks.flattened(mask).iter())
            .map(|rectangle| {
                canonical(
                    rectangle
                        .polygon
                        .iter()
                        .map(|point| (crate::gds::write::nm(point.x), crate::gds::write::nm(point.y)))
                        .collect(),
                )
            })
            .collect();
        written.sort_unstable();
        planned.sort_unstable();
        metrics.insert(format!("{}.outlines", mask.layer.name), written.len() as f64);
        if written != planned {
            findings.push(Finding {
                rule: "gds-readback".to_owned(),
                subject: mask.layer.name.clone(),
                measured: Some(written.len() as f64),
                limit: Some(planned.len() as f64),
                message: format!(
                    "layer {} reads back as {} outlines that are not the {} planned",
                    mask.layer.name,
                    written.len(),
                    planned.len()
                ),
                waived: None,
            });
        }
    }
    metrics.insert("bytes".to_owned(), bytes.len() as f64);
    metrics.insert("structures".to_owned(), library.structures.len() as f64);
    Check::from_findings("gds-readback", findings, metrics, &[])
}

#[cfg(test)]
mod tests {
    use crate::gds::read::unreal;
    use crate::gds::write::real;
    use crate::gds::{read, readback, write};
    use crate::mask::prepare;

    #[test]
    fn reals_survive_the_excess_64_encoding() {
        for value in [1e-9, 1e-3, 1.0, 90.0, 270.0, -0.5, 12345.678] {
            assert!((unreal(&real(value)) / value - 1.0).abs() < 1e-14, "{value}");
        }
        // The values every GDSII file carries: a nanometre, and a thousandth.
        assert_eq!(real(1e-9)[..7], [0x39, 0x44, 0xB8, 0x2F, 0xA0, 0x9B, 0x5A]);
        assert_eq!(real(1e-3)[..7], [0x3E, 0x41, 0x89, 0x37, 0x4B, 0xC6, 0xA7]);
        assert_eq!(real(0.0), [0; 8]);
    }

    #[test]
    fn a_file_reads_back_as_the_masks_it_was_written_from() {
        let chip = crate::fixtures::arrayed();
        let masks = prepare(&chip, None);
        let bytes = write(&masks);
        let library = read(&bytes).unwrap();
        assert_eq!(library.structures.keys().collect::<Vec<_>>(), ["CHIP", "tooth"]);
        assert_eq!((library.flattened(1).len(), library.flattened(2).len()), (100, 1));
        let check = readback(&masks, &bytes);
        assert!(check.passes(), "{check:?}");
        assert_eq!(check.metrics["trench.outlines"], 100.0);

        // A file whose outline has moved does not pass.
        let mut moved = masks.clone();
        moved.masks[1].rectangles[0].polygon[0].x += 1e-6;
        assert!(!readback(&moved, &bytes).passes());
        assert!(read(&bytes[..10]).is_err());
    }
}

//! Reading GDSII records, and flattening what they hold.

use crate::gds::{
    ANGLE, AREF, BGNSTR, BOUNDARY, COLROW, ENDEL, ENDSTR, GdsError, GdsLibrary, GdsStructure, LAYER, MAX_DEPTH,
    Reference, SNAME, SREF, STRNAME, TOP, XY,
};

pub(crate) fn unreal(bytes: &[u8]) -> f64 {
    let mantissa = bytes[1..8]
        .iter()
        .fold(0_u64, |held, byte| (held << 8) | u64::from(*byte)) as f64
        / 2f64.powi(56);
    let value = mantissa * 16f64.powi(i32::from(bytes[0] & 0x7F) - 64);
    if bytes[0] & 0x80 == 0 { value } else { -value }
}

/// Read a GDSII file.
pub fn read(bytes: &[u8]) -> Result<GdsLibrary, GdsError> {
    let fault = |message: &str| GdsError(message.to_owned());
    let mut library = GdsLibrary::default();
    let mut structure: Option<GdsStructure> = None;
    // The element being read: its kind, layer, the structure it refers to, turn, lattice and points.
    let (mut element, mut layer, mut named, mut turns, mut size, mut xy) =
        (0_u16, 0_u16, String::new(), 0_u8, (1_u32, 1_u32), Vec::new());
    let mut at = 0;
    while at + 4 <= bytes.len() {
        let length = usize::from(u16::from_be_bytes([bytes[at], bytes[at + 1]]));
        let kind = u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]);
        if length < 4 || at + length > bytes.len() {
            return Err(fault("a record runs past the end of the file"));
        }
        let data = &bytes[at + 4..at + length];
        at += length;
        let short = |index: usize| {
            data.get(2 * index..2 * index + 2)
                .map(|pair| i16::from_be_bytes([pair[0], pair[1]]))
        };
        let string = || String::from_utf8_lossy(data).trim_end_matches('\0').to_owned();
        match kind {
            BGNSTR => structure = Some(GdsStructure::default()),
            STRNAME => {
                structure
                    .as_mut()
                    .ok_or_else(|| fault("a name outside a structure"))?
                    .name = string()
            }
            ENDSTR => {
                let done = structure
                    .take()
                    .ok_or_else(|| fault("a structure ends that did not begin"))?;
                library.structures.insert(done.name.clone(), done);
            }
            BOUNDARY | SREF | AREF => (element, turns, size) = (kind, 0, (1, 1)),
            LAYER => layer = short(0).ok_or_else(|| fault("a layer without a number"))? as u16,
            SNAME => named = string(),
            ANGLE => turns = ((unreal(data) / 90.0).round() as i64).rem_euclid(4) as u8,
            COLROW => size = (short(0).unwrap_or(1) as u32, short(1).unwrap_or(1) as u32),
            XY => {
                let number = |four: &[u8]| i32::from_be_bytes([four[0], four[1], four[2], four[3]]);
                xy = data
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|pair| (number(&pair[..4]), number(&pair[4..])))
                    .collect();
            }
            ENDEL => {
                let into = structure
                    .as_mut()
                    .ok_or_else(|| fault("an element outside a structure"))?;
                let mut points = std::mem::take(&mut xy);
                if element == BOUNDARY {
                    points.pop();
                    into.boundaries.push((layer, points));
                } else {
                    let (columns, rows) = size;
                    into.references.push(Reference {
                        structure: std::mem::take(&mut named),
                        quarter_turns: turns,
                        columns,
                        rows,
                        lattice: points,
                    });
                }
            }
            _ => {}
        }
    }
    if library.structures.is_empty() {
        return Err(fault("it holds no structures"));
    }
    Ok(library)
}

impl GdsLibrary {
    /// Every outline on a layer of the top structure, references expanded, in nanometres.
    pub fn flattened(&self, layer: u16) -> Vec<Vec<(i32, i32)>> {
        let mut outlines = Vec::new();
        self.expand(TOP, layer, (0, 0), 0, 0, &mut outlines);
        outlines
    }

    fn expand(
        &self,
        name: &str,
        layer: u16,
        origin: (i32, i32),
        turns: u8,
        depth: usize,
        outlines: &mut Vec<Vec<(i32, i32)>>,
    ) {
        let Some(structure) = self.structures.get(name).filter(|_| depth < MAX_DEPTH) else {
            return;
        };
        let turned = |(x, y): (i32, i32)| match turns % 4 {
            0 => (x, y),
            1 => (-y, x),
            2 => (-x, -y),
            _ => (y, -x),
        };
        let placed = |point: (i32, i32)| (origin.0 + turned(point).0, origin.1 + turned(point).1);
        for (_, outline) in structure.boundaries.iter().filter(|(held, _)| *held == layer) {
            outlines.push(outline.iter().map(|point| placed(*point)).collect());
        }
        for reference in &structure.references {
            let &[start, along, across] = reference.lattice.as_slice() else {
                if let Some(start) = reference.lattice.first() {
                    self.expand(
                        &reference.structure,
                        layer,
                        placed(*start),
                        turns + reference.quarter_turns,
                        depth + 1,
                        outlines,
                    );
                }
                continue;
            };
            let step = |end: (i32, i32), count: u32, index: u32| {
                let count = i64::from(count.max(1));
                let part = |from: i32, to: i32| (i64::from(to - from) * i64::from(index) / count) as i32;
                (part(start.0, end.0), part(start.1, end.1))
            };
            for row in 0..reference.rows {
                for column in 0..reference.columns {
                    let (by_column, by_row) = (
                        step(along, reference.columns, column),
                        step(across, reference.rows, row),
                    );
                    let at = (start.0 + by_column.0 + by_row.0, start.1 + by_column.1 + by_row.1);
                    self.expand(
                        &reference.structure,
                        layer,
                        placed(at),
                        turns + reference.quarter_turns,
                        depth + 1,
                        outlines,
                    );
                }
            }
        }
    }
}

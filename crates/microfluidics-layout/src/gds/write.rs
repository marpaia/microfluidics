//! Writing GDSII records.

use crate::gds::{
    ANGLE, AREF, BGNLIB, BGNSTR, BOUNDARY, COLROW, DATATYPE, ENDEL, ENDLIB, ENDSTR, HEADER, LAYER, LIBNAME,
    MAX_COLUMNS, SNAME, STRANS, STRNAME, TOP, UNITS, XY,
};
use crate::geometry::Point;
use crate::mask::MaskSet;
use crate::rectangle::Rectangle;

fn record(out: &mut Vec<u8>, kind: u16, data: &[u8]) {
    out.extend(((data.len() + 4) as u16).to_be_bytes());
    out.extend(kind.to_be_bytes());
    out.extend(data);
}

fn shorts(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|value| value.to_be_bytes()).collect()
}

fn text(name: &str) -> Vec<u8> {
    let mut bytes = name.as_bytes().to_vec();
    if bytes.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

/// An eight-byte GDSII real: a sign, a base-16 exponent in excess 64, and a 56-bit mantissa.
pub(crate) fn real(value: f64) -> [u8; 8] {
    if value == 0.0 {
        return [0; 8];
    }
    let (mut mantissa, mut exponent) = (value.abs(), 64_i32);
    while mantissa >= 1.0 {
        mantissa /= 16.0;
        exponent += 1;
    }
    while mantissa < 1.0 / 16.0 {
        mantissa *= 16.0;
        exponent -= 1;
    }
    let mut bytes = ((mantissa * 2f64.powi(56)) as u64).to_be_bytes();
    bytes[0] = (exponent as u8) | if value < 0.0 { 0x80 } else { 0 };
    bytes
}

pub(crate) fn nm(metres: f64) -> i32 {
    (metres * 1e9).round() as i32
}

fn points(points: &[Point]) -> Vec<u8> {
    points
        .iter()
        .flat_map(|point| [nm(point.x).to_be_bytes(), nm(point.y).to_be_bytes()])
        .flatten()
        .collect()
}

fn boundary(out: &mut Vec<u8>, layer: u16, rectangle: &Rectangle) {
    record(out, BOUNDARY, &[]);
    record(out, LAYER, &shorts(&[layer as i16]));
    record(out, DATATYPE, &shorts(&[0]));
    // A boundary closes on its first point.
    let closed: Vec<Point> = rectangle
        .polygon
        .iter()
        .chain(rectangle.polygon.first())
        .copied()
        .collect();
    record(out, XY, &points(&closed));
    record(out, ENDEL, &[]);
}

/// A structure name GDSII allows: letters, digits, `_`, `?` and `$`, at most 32 long.
fn structure_name(name: &str) -> String {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '?' | '$');
    if name.len() <= 32 && name.chars().all(allowed) {
        name.to_owned()
    } else {
        format!(
            "C_{}",
            &microfluidics_core::Sha256Hex::of_bytes(name.as_bytes()).as_str()[..24]
        )
    }
}

/// The masks as one GDSII file.
pub fn write(masks: &MaskSet) -> Vec<u8> {
    let number = |layer: &str| masks.layers.get(layer).map_or(0, |layer| layer.gds);
    let mut out = Vec::new();
    // Every date is zero: a file depends on nothing but the design.
    record(&mut out, HEADER, &shorts(&[600]));
    record(&mut out, BGNLIB, &shorts(&[0; 12]));
    record(&mut out, LIBNAME, &text("MICROFLUIDICS"));
    record(&mut out, UNITS, &[real(1e-3), real(1e-9)].concat());
    for structure in masks.structures.values() {
        record(&mut out, BGNSTR, &shorts(&[0; 12]));
        record(&mut out, STRNAME, &text(&structure_name(&structure.name)));
        for rectangle in &structure.rectangles {
            boundary(&mut out, number(&rectangle.layer), rectangle);
        }
        record(&mut out, ENDSTR, &[]);
    }
    record(&mut out, BGNSTR, &shorts(&[0; 12]));
    record(&mut out, STRNAME, &text(TOP));
    for mask in &masks.masks {
        for rectangle in &mask.rectangles {
            boundary(&mut out, mask.layer.gds, rectangle);
        }
        for array in &mask.translates {
            // One reference holds a row of at most 32,767 placements.
            for start in (0..array.count).step_by(MAX_COLUMNS as usize) {
                let columns = (array.count - start).min(MAX_COLUMNS);
                let origin = array.origin_of(start);
                let along = Point::new(
                    origin.x + f64::from(columns) * array.step.x,
                    origin.y + f64::from(columns) * array.step.y,
                );
                let across = Point::new(origin.x - array.step.y, origin.y + array.step.x);
                record(&mut out, AREF, &[]);
                record(&mut out, SNAME, &text(&structure_name(&array.structure)));
                record(&mut out, STRANS, &shorts(&[0]));
                record(&mut out, ANGLE, &real(90.0 * f64::from(array.quarter_turns)));
                record(&mut out, COLROW, &shorts(&[columns as i16, 1]));
                record(&mut out, XY, &points(&[origin, along, across]));
                record(&mut out, ENDEL, &[]);
            }
        }
    }
    record(&mut out, ENDSTR, &[]);
    record(&mut out, ENDLIB, &[]);
    out
}

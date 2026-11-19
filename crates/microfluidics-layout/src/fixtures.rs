//! Helpers the tests share.

use microfluidics_core::model::PortKind;
use microfluidics_core::model::{LayerSpec, LayerStack};

use crate::chip::Chip;
use crate::geometry::{Direction, Point};
use crate::plan::Layout;
use crate::rectangle::{Port, Rectangle};

/// Whether two lengths, areas or volumes agree to nine significant figures.
pub(crate) fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs())
}

/// A 3 mm thick chip of `size_mm` with a 1.1 um `trench` layer and a 20 um `flow` layer.
/// Regions are `(name, layer, [x0, y0, x1, y1])`, access points `[x, y]`, and ports
/// `(name, [x, y])` facing down on the flow layer; all in micrometres.
pub(crate) fn chip(
    size_mm: [f64; 2],
    rectangles: &[(&str, &str, [f64; 4])],
    access: &[[f64; 2]],
    ports: &[(&str, [f64; 2])],
) -> Chip {
    let um = 1e-6;
    let layers = vec![
        LayerSpec {
            name: "trench".to_owned(),
            height: 1.1 * um,
            gds: 1,
        },
        LayerSpec {
            name: "flow".to_owned(),
            height: 20.0 * um,
            gds: 2,
        },
    ];
    let layout = Layout {
        rectangles: (rectangles.iter())
            .map(|(name, layer, at)| {
                Rectangle::rect(
                    name,
                    layer,
                    Point::new(at[0] * um, at[1] * um),
                    Point::new(at[2] * um, at[3] * um),
                )
            })
            .collect(),
        ports: (ports.iter())
            .map(|(name, at)| Port {
                name: (*name).to_owned(),
                kind: PortKind::Inlet,
                position: Point::new(at[0] * um, at[1] * um),
                width: 100.0 * um,
                layer: "flow".to_owned(),
                direction: Direction::MinusY,
            })
            .collect(),
        access: access.iter().map(|at| Point::new(at[0] * um, at[1] * um)).collect(),
        ..Layout::default()
    };
    Chip::new(
        [size_mm[0] * 1e-3, size_mm[1] * 1e-3, 3e-3],
        [size_mm[0] * 1e-3 + 10e-3, size_mm[1] * 1e-3 + 10e-3, 1e-3],
        LayerStack::new(layers).unwrap(),
        layout,
    )
    .unwrap()
}

/// A 100 um channel with one hundred 25 x 1.2 um teeth off its left side, held as an row.
pub(crate) fn arrayed() -> Chip {
    let mut chip = chip(
        [2.0, 2.0],
        &[("channel", "flow", [950.0, 500.0, 1050.0, 1500.0])],
        &[],
        &[],
    );
    let tooth = Rectangle::rect("tooth", "trench", Point::new(0.0, 0.0), Point::new(25e-6, 1.2e-6));
    chip.layout.structures.insert(
        "tooth".to_owned(),
        crate::structure::Structure {
            name: "tooth".to_owned(),
            rectangles: vec![tooth],
        },
    );
    chip.layout.translates.push(crate::structure::Translates {
        name: "left".to_owned(),
        structure: "tooth".to_owned(),
        origin: Point::new(925e-6, 600e-6),
        step: Point::new(0.0, 4e-6),
        count: 100,
        quarter_turns: 0,
        first: 0,
    });
    chip
}

//! Rows of rectangles alike stay rows: through mask preparation, the GDSII file, and a wafer.

// A test stops on the first thing that is not as expected, in its helpers as in its cases.
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use microfluidics_core::Status;
use microfluidics_core::model::LayerSpec;
use microfluidics_layout::{
    Builder, Chip, MaskSet, Origin, Overrides, Place, Point, Process, RegionSpec, WaferError, WaferSpec, assemble,
    export, gds_readback, gds_write, prepare,
};

const WAFER: &str = r#"
name = "six"
diameter_mm = 100.0
edge_exclusion_mm = 5.0
street_mm = 1.0

[[chips]]
design = "lanes.py"
columns = 3
rows = 2
"#;

fn process() -> Process {
    Process::bundled("conservative-su8-pdms").unwrap()
}

/// A chip of `size` holding `count` lanes 2.6 mm apart: each a channel `length` long with
/// `per_side` trenches off each side, 4 um apart.
fn lanes(size: [f64; 2], count: u32, length: f64, per_side: u32) -> Chip {
    let um = 1e-6;
    let origin = Origin::new("lanes.py", 1);
    let mut design = Builder::design("lanes", &Overrides::default());
    design.slab([size[0], size[1], 4e-3], &origin).unwrap();
    design.glass([120e-3, 120e-3, 1e-3], &origin).unwrap();
    for (name, height, gds) in [("trench", 1.1 * um, 1), ("flow", 20.0 * um, 2)] {
        let layer = LayerSpec {
            name: name.to_owned(),
            height,
            gds,
        };
        design.layer(layer, &origin).unwrap();
    }
    let lane = RegionSpec {
        name: "lane".to_owned(),
        ..RegionSpec::default()
    };
    let first = (length - 4.0 * um * f64::from(per_side - 1) - 1.2 * um) / 2.0;
    for index in 0..count {
        let mut child = design
            .child(&format!("lane[{index}]"), &lane, &BTreeMap::new(), &origin)
            .unwrap();
        let (from, to) = (Point::new(-50.0 * um, 0.0), Point::new(50.0 * um, length));
        child.rectangle("channel", "flow", from, to, &origin).unwrap();
        for (name, from, to) in [("left", -75.0 * um, -50.0 * um), ("right", 50.0 * um, 75.0 * um)] {
            let (from, to) = (Point::new(from, first), Point::new(to, first + 1.2 * um));
            let step = Point::new(0.0, 4.0 * um);
            child
                .translates(name, "trench", from, to, step, per_side, &origin)
                .unwrap();
        }
        let place = Place::At {
            point: Point::new(3e-3 + 2.6e-3 * f64::from(index), 2e-3),
            turned: 0,
        };
        design.place(child, &place, &origin).unwrap();
    }
    design.finish().unwrap().chip
}

#[test]
fn a_hundred_thousand_trenches_are_ten_lanes_of_two_rows() {
    let chip = lanes([30e-3, 26e-3], 10, 21e-3, 5000);
    assert_eq!(
        (
            chip.layout.translates.len(),
            chip.layout.structures.len(),
            chip.layout.feature_count()
        ),
        (20, 1, 100_010)
    );
    let masks = prepare(&chip, Some(&process()));
    let written = gds_write(&masks);
    assert!(written.len() < 20_000, "{}", written.len());
    assert_eq!(gds_readback(&masks, &written).status, Status::Satisfied);
}

fn chips() -> BTreeMap<String, MaskSet> {
    let chip = lanes([24.2e-3, 18e-3], 8, 12e-3, 100);
    [("lanes.py".to_owned(), prepare(&chip, Some(&process())))].into()
}

#[test]
fn a_wafer_holds_its_chips_named_within_its_edge_and_reads_back_from_its_file() {
    let wafer = assemble(&WaferSpec::parse("six.toml", WAFER).unwrap(), &chips()).unwrap();
    let labels: Vec<&str> = wafer.placements.iter().map(|placed| placed.label.as_str()).collect();
    assert_eq!((labels.len(), labels[0], labels[5]), (6, "lanes-1", "lanes-6"));
    assert_eq!((wafer.alignment_marks, wafer.masks.size), (2, [0.1, 0.1]));
    // Every chip's trenches are still rows of one outline, and everything lies on the wafer.
    let one = &chips()["lanes.py"];
    assert_eq!(wafer.masks.masks[0].translates.len(), 6 * one.masks[0].translates.len());
    for mask in &wafer.masks.masks {
        for rectangle in wafer.masks.flattened(mask) {
            let bounds = rectangle.bounds();
            let reach = [bounds.min, bounds.max].map(|point| (point.x - 0.05).hypot(point.y - 0.05));
            assert!(
                reach.iter().all(|reach| *reach <= 0.045 + 1e-9),
                "{} at {reach:?}",
                rectangle.name
            );
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let exported = export(&wafer.masks, Some(&process()), dir.path()).unwrap();
    assert!(
        exported.checks.iter().all(|check| check.status == Status::Satisfied),
        "{:?}",
        exported.checks
    );

    let wide = WaferSpec::parse("six.toml", &WAFER.replace("columns = 3", "columns = 4")).unwrap();
    let error = assemble(&wide, &chips()).unwrap_err();
    assert!(
        matches!(&error, WaferError::DoesNotFit { label, .. } if label == "lanes-1"),
        "{error}"
    );
}

//! The last steps of mask preparation: bias, and the mirror image a mask is written as.

use std::collections::{BTreeMap, BTreeSet};

use crate::geometry::{Point, Rect};
use crate::mask::{MaskSet, sides};
use crate::rectangle::Rectangle;
use crate::structure::Structure;

/// The narrowest a feature is left when a bias would close it, in metres: one grid step.
const LEAST: f64 = 1e-9;

/// `rect` with every edge moved outward by `bias`, never narrower than one grid step.
fn grown(rect: Rect, bias: f64) -> Rect {
    let along = |min: f64, max: f64| {
        let half = ((max - min) / 2.0 + bias).max(LEAST / 2.0);
        let middle = (min + max) / 2.0;
        (middle - half, middle + half)
    };
    let ((min_x, max_x), (min_y, max_y)) = (along(rect.min.x, rect.max.x), along(rect.min.y, rect.max.y));
    Rect {
        min: Point::new(min_x, min_y),
        max: Point::new(max_x, max_y),
    }
}

fn with(rectangle: &Rectangle, bias: f64) -> Rectangle {
    match sides(rectangle) {
        Some(rect) => {
            let rect = grown(rect, bias);
            Rectangle::rect(&rectangle.name, &rectangle.layer, rect.min, rect.max)
        }
        None => rectangle.clone(),
    }
}

/// Keep only the structures some row places.
fn pruned(set: &mut MaskSet) {
    let placed: BTreeSet<&str> = (set.masks.iter())
        .flat_map(|mask| mask.translates.iter().map(|row| row.structure.as_str()))
        .collect();
    let kept: BTreeMap<String, Structure> = (set.structures.iter())
        .filter(|(name, _)| placed.contains(name.as_str()))
        .map(|(name, structure)| (name.clone(), structure.clone()))
        .collect();
    set.structures = kept;
}

/// Move every edge of every mask by the mask's bias. An row stays an row, of a structure grown.
pub(crate) fn grow(set: &mut MaskSet) {
    let mut structures = set.structures.clone();
    for mask in set.masks.iter_mut().filter(|mask| mask.spec.bias != 0.0) {
        let bias = mask.spec.bias;
        mask.rectangles = mask.rectangles.iter().map(|rectangle| with(rectangle, bias)).collect();
        for row in &mut mask.translates {
            let Some(structure) = set.structures.get(&row.structure) else {
                continue;
            };
            let way = if bias > 0.0 { "grown" } else { "shrunk" };
            let name = format!("{}_{way}{}", structure.name, (bias.abs() * 1e9).round() as i64);
            let rectangles = structure
                .rectangles
                .iter()
                .map(|rectangle| with(rectangle, bias))
                .collect();
            structures.entry(name.clone()).or_insert(Structure {
                name: name.clone(),
                rectangles,
            });
            row.structure = name;
        }
    }
    set.structures = structures;
    pruned(set);
}

impl MaskSet {
    /// The masks as they are written to file: each mask its process mirrors is turned over
    /// about the chip's vertical centre line. Rules and the print check are run on the masks as
    /// prepared, which lie as the chip is drawn.
    pub fn as_written(&self) -> MaskSet {
        let mut set = self.clone();
        let width = self.size[0];
        let over = |rectangle: &Rectangle, about: f64| Rectangle {
            name: rectangle.name.clone(),
            layer: rectangle.layer.clone(),
            polygon: rectangle
                .polygon
                .iter()
                .map(|point| Point::new(about - point.x, point.y))
                .collect(),
        };
        for mask in set.masks.iter_mut().filter(|mask| mask.spec.mirror) {
            mask.rectangles = mask.rectangles.iter().map(|rectangle| over(rectangle, width)).collect();
            for row in &mut mask.translates {
                let Some(structure) = self.structures.get(&row.structure) else {
                    continue;
                };
                // A placement mirrored is its structure mirrored, turned the other way.
                let name = format!("{}_mirrored", structure.name);
                set.structures.entry(name.clone()).or_insert(Structure {
                    name: name.clone(),
                    rectangles: structure
                        .rectangles
                        .iter()
                        .map(|rectangle| over(rectangle, 0.0))
                        .collect(),
                });
                row.structure = name;
                row.origin.x = width - row.origin.x;
                row.step.x = -row.step.x;
                row.quarter_turns = (4 - row.quarter_turns % 4) % 4;
            }
        }
        pruned(&mut set);
        set
    }
}

#[cfg(test)]
mod tests {
    use crate::fixtures::{chip, close};
    use crate::geometry::Point;
    use crate::mask::prepare;
    use crate::process::Process;
    use crate::rectangle::Rectangle;
    use crate::structure::{Structure, Translates};

    const DECK: &str = "name = \"p\"\n[[masks]]\nlayer = \"trench\"\nbias_um = 0.2\nmirror = true\n[[masks]]\nlayer = \"flow\"\ntone = \"dark\"\nbias_um = -1.0\n";

    #[test]
    fn every_edge_moves_by_the_bias_and_arrays_stay_arrays() {
        let mut chip = chip(
            [2.0, 2.0],
            &[("channel", "flow", [950.0, 500.0, 1050.0, 1500.0])],
            &[],
            &[],
        );
        let tooth = Rectangle::rect("tooth", "trench", Point::new(0.0, 0.0), Point::new(25e-6, 1.2e-6));
        chip.layout.structures.insert(
            "tooth".into(),
            Structure {
                name: "tooth".into(),
                rectangles: vec![tooth],
            },
        );
        chip.layout.translates = vec![Translates {
            name: "left".into(),
            structure: "tooth".into(),
            origin: Point::new(100e-6, 600e-6),
            step: Point::new(0.0, 4e-6),
            count: 50,
            quarter_turns: 1,
            first: 0,
        }];
        let process = Process::parse("p.toml", DECK).unwrap();
        let masks = prepare(&chip, Some(&process));
        let (trench, flow) = (&masks.masks[0], &masks.masks[1]);
        // The channel is 1 um narrower on each side; each tooth 0.2 um wider on each.
        let channel = flow.rectangles[0].bounds();
        assert!(close(channel.min.x, 951e-6) && close(channel.max.y, 1499e-6));
        assert_eq!(
            (trench.translates.len(), trench.translates[0].structure.as_str()),
            (1, "tooth_grown200")
        );
        assert_eq!(masks.structures.keys().collect::<Vec<_>>(), ["tooth_grown200"]);
        let first = masks.flattened(trench)[0].bounds();
        assert!(
            close(first.width(), 1.6e-6) && close(first.height(), 25.4e-6),
            "{first:?}"
        );

        // Written, the trench mask is turned over about the chip's centre line; the flow mask is not.
        let written = masks.as_written();
        let drawn = masks.flattened(trench);
        let turned = written.flattened(&written.masks[0]);
        assert_eq!(turned.len(), 50);
        for (was, is) in drawn.iter().zip(&turned) {
            let (was, is) = (was.bounds(), is.bounds());
            assert!(close(is.min.x, 2e-3 - was.max.x) && close(is.max.x, 2e-3 - was.min.x));
            assert!(close(is.min.y, was.min.y) && close(is.max.y, was.max.y));
        }
        assert_eq!(written.masks[1], masks.masks[1]);
    }

    #[test]
    fn a_bias_that_would_close_a_feature_leaves_it_one_grid_step_wide() {
        let chip = chip([2.0, 2.0], &[("line", "flow", [100.0, 100.0, 101.0, 900.0])], &[], &[]);
        let masks = prepare(&chip, Some(&Process::parse("p.toml", DECK).unwrap()));
        assert!(close(masks.masks[1].rectangles[0].bounds().width(), 1e-9));
    }
}

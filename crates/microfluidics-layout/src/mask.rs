//! Mask preparation: the chip as drawn becomes what each mask exposes.
//!
//! Layers are aligned to each other only to within a tolerance, so a shallow feature that opens
//! into a deeper one is drawn into it by the distance the process's overlap rules give: the two
//! then still meet when the masks sit that far apart. Arrays stay translates wherever every element
//! is extended alike. Every edge is then moved by the bias the process gives the mask.

use std::collections::BTreeMap;

use serde::Serialize;

use microfluidics_core::model::{LayerSpec, LayerStack};

use crate::bias::grow;
use crate::chip::Chip;
use crate::geometry::{Point, Rect, TOLERANCE};
use crate::maskspec::MaskSpec;
use crate::process::Process;
use crate::rectangle::Rectangle;
use crate::rules::{Features, joined};
use crate::structure::{Structure, Translates};

/// One mask: every feature of one layer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mask {
    pub layer: LayerSpec,
    /// How the mask is made from the layer's drawing.
    pub spec: MaskSpec,
    pub rectangles: Vec<Rectangle>,
    pub translates: Vec<Translates>,
    /// How many features were drawn into a deeper one.
    pub extended: usize,
}

/// The masks of a chip, shallowest layer first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaskSet {
    /// The chip's size along X and Y.
    pub size: [f64; 2],
    pub layers: LayerStack,
    pub structures: BTreeMap<String, Structure>,
    pub masks: Vec<Mask>,
}

impl MaskSet {
    /// Every rectangle of one mask, with translates expanded.
    pub fn flattened(&self, mask: &Mask) -> Vec<Rectangle> {
        let translates = mask
            .translates
            .iter()
            .filter_map(|row| Some(row.rectangles(self.structures.get(&row.structure)?)));
        mask.rectangles.iter().cloned().chain(translates.flatten()).collect()
    }

    /// How many outlines a mask exposes.
    pub fn outlines(&self, mask: &Mask) -> usize {
        let arrayed = |row: &Translates| {
            self.structures
                .get(&row.structure)
                .map_or(0, |structure| structure.rectangles.len() * row.count as usize)
        };
        mask.rectangles.len() + mask.translates.iter().map(arrayed).sum::<usize>()
    }
}

impl<'a> Features<'a> {
    /// The masks of `chip`, as the features the rules are checked on.
    pub fn prepared(masks: &'a MaskSet, chip: &'a Chip) -> Self {
        Self {
            size: masks.size,
            layers: &masks.layers,
            rectangles: masks.masks.iter().flat_map(|mask| masks.flattened(mask)).collect(),
            ports: &chip.layout.ports,
            access: &chip.layout.access,
            prepared: true,
        }
    }
}

/// `rect` drawn into every deeper feature it shares an edge with. `deeper` holds each deeper
/// feature's bounds and how far a feature of this layer is drawn into it.
fn drawn_into(rect: Rect, deeper: &[(Rect, f64)]) -> Rect {
    let mut drawn = rect;
    let meets = |a: f64, b: f64| (a - b).abs() <= TOLERANCE;
    for (other, overlap) in deeper.iter().filter(|(other, _)| joined(&rect, other)) {
        if meets(rect.max.x, other.min.x) {
            drawn.max.x = drawn.max.x.max(rect.max.x + overlap.min(other.width()));
        } else if meets(rect.min.x, other.max.x) {
            drawn.min.x = drawn.min.x.min(rect.min.x - overlap.min(other.width()));
        } else if meets(rect.max.y, other.min.y) {
            drawn.max.y = drawn.max.y.max(rect.max.y + overlap.min(other.height()));
        } else if meets(rect.min.y, other.max.y) {
            drawn.min.y = drawn.min.y.min(rect.min.y - overlap.min(other.height()));
        }
    }
    drawn
}

/// The sides of what is drawn, when its boundary is four of them along the axes.
pub(crate) fn sides(rectangle: &Rectangle) -> Option<Rect> {
    let bounds = rectangle.bounds();
    (rectangle.polygon.len() == 4 && (rectangle.area() - bounds.width() * bounds.height()).abs() <= TOLERANCE)
        .then_some(bounds)
}

fn rectangle_of(name: &str, layer: &str, rect: Rect) -> Rectangle {
    Rectangle::rect(name, layer, rect.min, rect.max)
}

/// The masks of `chip` under `process`: each shallow feature drawn into the deeper one it opens
/// into, then every edge moved by the mask's bias. Without a process features are drawn exactly
/// to their edges.
pub fn prepare(chip: &Chip, process: Option<&Process>) -> MaskSet {
    let flat = chip.layout.flattened();
    let mut set = MaskSet {
        size: [chip.size[0], chip.size[1]],
        layers: chip.layers.clone(),
        structures: BTreeMap::new(),
        masks: Vec::new(),
    };
    for layer in chip.layers.iter() {
        // Every deeper feature, with how far this layer's features are drawn into it.
        let deeper: Vec<(Rect, f64)> = (flat.iter())
            .filter(|rectangle| chip.layers.height(&rectangle.layer) > layer.height + TOLERANCE)
            .map(|rectangle| {
                (
                    rectangle.bounds(),
                    process.map_or(0.0, |process| process.overlap(&layer.name, &rectangle.layer)),
                )
            })
            .filter(|(_, overlap)| *overlap > 0.0)
            .collect();
        let mut mask = Mask {
            layer: layer.clone(),
            spec: process.map_or_else(|| MaskSpec::plain(&layer.name), |process| process.mask(&layer.name)),
            rectangles: Vec::new(),
            translates: Vec::new(),
            extended: 0,
        };
        let draw = |rectangle: &Rectangle, mask: &mut Mask| match sides(rectangle) {
            Some(rect) => {
                let drawn = drawn_into(rect, &deeper);
                mask.extended += usize::from(drawn != rect);
                mask.rectangles
                    .push(rectangle_of(&rectangle.name, &rectangle.layer, drawn));
            }
            None => mask.rectangles.push(rectangle.clone()),
        };
        for rectangle in chip
            .layout
            .rectangles
            .iter()
            .filter(|rectangle| rectangle.layer == layer.name)
        {
            draw(rectangle, &mut mask);
        }
        for row in &chip.layout.translates {
            let Some(structure) = chip.layout.structures.get(&row.structure) else {
                continue;
            };
            if structure
                .rectangles
                .iter()
                .all(|rectangle| rectangle.layer != layer.name)
            {
                continue;
            }
            // An row of one rectangle whose every element is drawn alike stays an row.
            let single = (structure.rectangles.len() == 1)
                .then(|| sides(&structure.rectangles[0]))
                .flatten();
            let elements: Vec<Rectangle> = row
                .rectangles(structure)
                .filter(|rectangle| rectangle.layer == layer.name)
                .collect();
            let change = |rectangle: &Rectangle| {
                let (rect, drawn) = (rectangle.bounds(), drawn_into(rectangle.bounds(), &deeper));
                [
                    drawn.min.x - rect.min.x,
                    drawn.min.y - rect.min.y,
                    drawn.max.x - rect.max.x,
                    drawn.max.y - rect.max.y,
                ]
            };
            let first = elements.first().map(change);
            let alike = |rectangle: &Rectangle| {
                first.is_some_and(|first| {
                    change(rectangle)
                        .iter()
                        .zip(first)
                        .all(|(a, b)| (a - b).abs() <= TOLERANCE)
                })
            };
            match (single, first) {
                (Some(_), Some(first)) if elements.iter().all(alike) => {
                    if first.iter().all(|side| side.abs() <= TOLERANCE) {
                        set.structures
                            .entry(structure.name.clone())
                            .or_insert_with(|| structure.clone());
                        mask.translates.push(row.clone());
                        continue;
                    }
                    let drawn = drawn_into(elements[0].bounds(), &deeper);
                    let nm = |metres: f64| (metres * 1e9).round() as i64;
                    let name = format!("{}_{}x{}", layer.name, nm(drawn.width()), nm(drawn.height()));
                    let outline = Rect {
                        min: Point::new(0.0, 0.0),
                        max: Point::new(drawn.width(), drawn.height()),
                    };
                    set.structures.entry(name.clone()).or_insert_with(|| Structure {
                        name: name.clone(),
                        rectangles: vec![rectangle_of(&name, &layer.name, outline)],
                    });
                    mask.extended += elements.len();
                    mask.translates.push(Translates {
                        structure: name,
                        origin: drawn.min,
                        quarter_turns: 0,
                        ..row.clone()
                    });
                }
                _ => elements.iter().for_each(|rectangle| draw(rectangle, &mut mask)),
            }
        }
        set.masks.push(mask);
    }
    grow(&mut set);
    set
}

#[cfg(test)]
mod tests {
    use crate::chip::Chip;
    use crate::fixtures::{chip, close};
    use crate::geometry::Point;
    use crate::mask::prepare;
    use crate::process::Process;
    use crate::rectangle::Rectangle;
    use crate::structure::{Structure, Translates};

    /// A 100 um channel with a 25 x 1.2 um trench off each side.
    fn lane() -> Chip {
        let rectangles = [
            ("channel", "flow", [950.0, 500.0, 1050.0, 1500.0]),
            ("trench_left", "trench", [925.0, 1000.0, 950.0, 1001.2]),
            ("trench_right", "trench", [1050.0, 1000.0, 1075.0, 1001.2]),
        ];
        chip([2.0, 2.0], &rectangles, &[], &[])
    }

    fn process() -> Process {
        Process::parse(
            "p.toml",
            "name = \"p\"\n[[rules]]\nname = \"into\"\nkind = \"overlap\"\nmin_um = 3.0\n",
        )
        .unwrap()
    }

    #[test]
    fn a_shallow_feature_is_drawn_into_the_deeper_one_it_opens_into() {
        let masks = prepare(&lane(), Some(&process()));
        let counts: Vec<(&str, usize, usize)> = masks
            .masks
            .iter()
            .map(|mask| (mask.layer.name.as_str(), mask.rectangles.len(), mask.extended))
            .collect();
        assert_eq!(counts, [("trench", 2, 2), ("flow", 1, 0)]);
        // Each trench is 3 um longer, toward the channel.
        let left = masks.masks[0].rectangles[0].bounds();
        assert!(close(left.min.x, 925e-6) && close(left.max.x, 953e-6));
        let right = masks.masks[0].rectangles[1].bounds();
        assert!(close(right.min.x, 1047e-6) && close(right.max.x, 1075e-6));
        assert_eq!(prepare(&lane(), None).masks[0].extended, 0);
    }

    #[test]
    fn an_array_whose_elements_are_drawn_alike_stays_an_array() {
        let mut chip = lane();
        chip.layout.rectangles.retain(|rectangle| rectangle.layer == "flow");
        let teeth = Rectangle::rect("tooth", "trench", Point::new(0.0, 0.0), Point::new(25e-6, 1.2e-6));
        chip.layout.structures.insert(
            "tooth".into(),
            Structure {
                name: "tooth".into(),
                rectangles: vec![teeth],
            },
        );
        let row = |name: &str, x: f64, count: u32| Translates {
            name: name.into(),
            structure: "tooth".into(),
            origin: Point::new(x, 600e-6),
            step: Point::new(0.0, 4e-6),
            count,
            quarter_turns: 0,
            first: 0,
        };
        chip.layout.translates = vec![row("left", 925e-6, 100), row("apart", 100e-6, 5)];
        let masks = prepare(&chip, Some(&process()));
        let trench = &masks.masks[0];
        assert_eq!(
            (trench.translates.len(), trench.rectangles.len(), trench.extended),
            (2, 0, 100)
        );
        // The row at the channel has a longer structure; the one standing apart keeps its own.
        assert_eq!(
            (
                trench.translates[0].structure.as_str(),
                trench.translates[1].structure.as_str()
            ),
            ("trench_28000x1200", "tooth")
        );
        assert_eq!(masks.outlines(trench), 105);
        let drawn = masks.flattened(trench);
        assert!(close(drawn[0].bounds().max.x, 953e-6) && close(drawn[99].bounds().min.y, 600e-6 + 99.0 * 4e-6));

        // An row that only partly meets the channel is drawn element by element.
        chip.layout.translates = vec![row("straddling", 925e-6, 300)];
        let straddling = prepare(&chip, Some(&process()));
        assert_eq!(
            (
                straddling.masks[0].translates.len(),
                straddling.masks[0].rectangles.len()
            ),
            (0, 300)
        );
        assert_eq!(straddling.masks[0].extended, 225);
    }
}

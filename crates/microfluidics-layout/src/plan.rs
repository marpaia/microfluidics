//! The layout: rectangles, structures and translates, with ports, access points and fiducials.
//!
//! Coordinates are metres. A layout holds each feature once: either as a rectangle of its own, or
//! as a structure that translates place many times.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::geometry::{Point, Rect, bounds};
use crate::rectangle::{Port, Rectangle};
use crate::structure::{Structure, Translates};

/// A point one layer is aligned to another by: the centre of an alignment mark.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Fiducial {
    pub name: String,
    pub position: Point,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Layout {
    pub rectangles: Vec<Rectangle>,
    pub structures: BTreeMap<String, Structure>,
    pub translates: Vec<Translates>,
    pub ports: Vec<Port>,
    /// Where tubing reaches the layout from outside: the centres of punched holes.
    pub access: Vec<Point>,
    /// The centres of the marks later layers are aligned to the first by.
    pub fiducials: Vec<Fiducial>,
}

impl Layout {
    /// Every rectangle, with translates expanded.
    pub fn flattened(&self) -> Vec<Rectangle> {
        let translates = self
            .translates
            .iter()
            .filter_map(|row| Some(row.rectangles(self.structures.get(&row.structure)?)));
        self.rectangles.iter().cloned().chain(translates.flatten()).collect()
    }

    /// How many rectangles the layout has when flattened.
    pub fn feature_count(&self) -> usize {
        let arrayed = |row: &Translates| {
            self.structures
                .get(&row.structure)
                .map_or(0, |structure| structure.rectangles.len() * row.count as usize)
        };
        self.rectangles.len() + self.translates.iter().map(arrayed).sum::<usize>()
    }

    /// The bounding rectangle of every rectangle.
    pub fn envelope(&self) -> Rect {
        bounds(
            self.flattened()
                .iter()
                .flat_map(|rectangle| rectangle.polygon.iter().copied()),
        )
    }

    /// The area drawn on `layer`.
    pub fn area(&self, layer: &str) -> f64 {
        let own: f64 = self
            .rectangles
            .iter()
            .filter(|rectangle| rectangle.layer == layer)
            .map(Rectangle::area)
            .sum();
        let arrayed = |row: &Translates| {
            let structure = self.structures.get(&row.structure);
            let once: f64 = structure.map_or(0.0, |structure| {
                structure
                    .rectangles
                    .iter()
                    .filter(|rectangle| rectangle.layer == layer)
                    .map(Rectangle::area)
                    .sum()
            });
            once * f64::from(row.count)
        };
        own + self.translates.iter().map(arrayed).sum::<f64>()
    }

    /// Add `other`, turned about its origin by whole quarter turns and then moved, with `prefix`
    /// before every name. Structures are shared by name.
    pub(crate) fn merge(&mut self, other: Layout, prefix: &str, quarter_turns: u8, by: Point) {
        self.rectangles.extend(other.rectangles.iter().map(|rectangle| {
            let mut placed = rectangle.placed(quarter_turns, by);
            placed.name = format!("{prefix}{}", rectangle.name);
            placed
        }));
        self.translates
            .extend(other.translates.iter().map(|row| row.placed(prefix, quarter_turns, by)));
        self.ports.extend(other.ports.into_iter().map(|port| Port {
            name: format!("{prefix}{}", port.name),
            position: port.position.turned(quarter_turns).moved(by),
            direction: port.direction.turned(quarter_turns),
            ..port
        }));
        self.access.extend(
            other
                .access
                .into_iter()
                .map(|point| point.turned(quarter_turns).moved(by)),
        );
        self.fiducials
            .extend(other.fiducials.into_iter().map(|fiducial| Fiducial {
                name: format!("{prefix}{}", fiducial.name),
                position: fiducial.position.turned(quarter_turns).moved(by),
            }));
        self.structures.extend(other.structures);
    }
}

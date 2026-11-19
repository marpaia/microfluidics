//! Regions on a layer, and the ports where channels meet a structure's edge.

use serde::Serialize;

use microfluidics_core::model::PortKind;

use crate::geometry::{Direction, Point, Rect, bounds};

/// A planar rectangle on one layer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rectangle {
    pub name: String,
    pub layer: String,
    /// A simple polygon; either winding.
    pub polygon: Vec<Point>,
}

impl Rectangle {
    pub fn rect(name: &str, layer: &str, min: Point, max: Point) -> Self {
        let polygon = vec![min, Point::new(max.x, min.y), max, Point::new(min.x, max.y)];
        Self {
            name: name.to_owned(),
            layer: layer.to_owned(),
            polygon,
        }
    }

    /// The enclosed area (shoelace formula).
    pub fn area(&self) -> f64 {
        area(&self.polygon)
    }

    pub fn bounds(&self) -> Rect {
        bounds(self.polygon.iter().copied())
    }

    /// The rectangle turned about the origin by whole quarter turns, then moved.
    pub(crate) fn placed(&self, quarter_turns: u8, by: Point) -> Rectangle {
        let polygon = self
            .polygon
            .iter()
            .map(|point| point.turned(quarter_turns).moved(by))
            .collect();
        Rectangle {
            name: self.name.clone(),
            layer: self.layer.clone(),
            polygon,
        }
    }
}

/// The area a closed outline encloses.
pub(crate) fn area(outline: &[Point]) -> f64 {
    let count = outline.len();
    let twice: f64 = (0..count)
        .map(|at| (outline[at], outline[(at + 1) % count]))
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum();
    twice.abs() / 2.0
}

/// A fluid port: where a channel meets a structure's edge.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Port {
    pub name: String,
    pub kind: PortKind,
    /// The centre of the channel opening.
    pub position: Point,
    pub width: f64,
    pub layer: String,
    pub direction: Direction,
}

//! A design's fingerprint: everything about an elaborated chip that must not change when the
//! way it is written does.
//!
//! Lengths are whole nanometres and lists are sorted, so two elaborations of one chip give one
//! fingerprint whatever order they were built in. A port is known by where it is, not by what
//! it is called, so a port may be renamed without the fingerprint changing.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::builder::Elaborated;
use crate::geometry::Point;

fn nm(metres: f64) -> i64 {
    (metres * 1e9).round() as i64
}

fn place(point: Point) -> String {
    format!("@{},{}", nm(point.x), nm(point.y))
}

/// The fingerprint of `elaborated`.
pub fn fingerprint(elaborated: &Elaborated) -> Value {
    let chip = &elaborated.chip;
    let layout = &chip.layout;
    // A edge's end, or a joint's port, by position when it is a port and by name when it is a
    // vertex inside a structure.
    let positions: BTreeMap<&str, Point> = layout
        .ports
        .iter()
        .map(|port| (port.name.as_str(), port.position))
        .collect();
    let end = |name: &str| {
        positions
            .get(name)
            .map_or_else(|| name.to_owned(), |point| place(*point))
    };

    let mut flat: Vec<Value> = (layout.flattened().iter())
        .map(|rectangle| {
            let bounds = rectangle.bounds();
            json!([
                rectangle.name,
                rectangle.layer,
                nm(bounds.min.x),
                nm(bounds.min.y),
                nm(bounds.max.x),
                nm(bounds.max.y)
            ])
        })
        .collect();
    flat.sort_by_key(Value::to_string);
    let mut translates: Vec<Value> = (layout.translates.iter())
        .map(|row| {
            let outlines = layout
                .structures
                .get(&row.structure)
                .map_or(0, |structure| structure.rectangles.len());
            json!([row.name, row.count, outlines])
        })
        .collect();
    translates.sort_by_key(Value::to_string);
    let mut ports: Vec<Value> = (layout.ports.iter())
        .map(|port| {
            let (kind, direction) = (format!("{:?}", port.kind), format!("{:?}", port.direction));
            json!([
                nm(port.position.x),
                nm(port.position.y),
                nm(port.width),
                port.layer,
                direction,
                kind
            ])
        })
        .collect();
    ports.sort_by_key(Value::to_string);
    let mut access: Vec<Value> = layout
        .access
        .iter()
        .map(|point| json!([nm(point.x), nm(point.y)]))
        .collect();
    access.sort_by_key(Value::to_string);
    let mut edges: Vec<Value> = (chip.edges.iter())
        .map(|edge| {
            let ducts: Vec<Value> = (edge.ducts.iter())
                .map(|duct| json!([duct.layer, nm(duct.width), nm(duct.length)]))
                .collect();
            json!([edge.element, end(&edge.from), end(&edge.to), ducts])
        })
        .collect();
    edges.sort_by_key(Value::to_string);
    let mut joints: Vec<Value> = (chip.joints.iter())
        .map(|(a, b)| {
            let mut pair = [end(a), end(b)];
            pair.sort();
            json!(pair)
        })
        .collect();
    joints.sort_by_key(Value::to_string);
    let layers: Vec<Value> = (chip.layers.iter())
        .map(|layer| json!([layer.name, nm(layer.height), layer.gds]))
        .collect();
    let tunables: Vec<Value> = (elaborated.tunables.iter())
        .map(|tunable| {
            json!([
                tunable.path,
                tunable.value.si,
                tunable.low.si,
                tunable.high.si,
                tunable.count
            ])
        })
        .collect();

    json!({
        "name": elaborated.name,
        "process": elaborated.process,
        "size_nm": [nm(chip.size[0]), nm(chip.size[1]), nm(chip.size[2])],
        "glass_nm": [nm(chip.glass[0]), nm(chip.glass[1]), nm(chip.glass[2])],
        "layers": layers,
        "rectangles": flat,
        "translates": translates,
        "ports": ports,
        "access": access,
        "edges": edges,
        "joints": joints,
        "quantities": elaborated.quantities,
        "tunables": tunables,
        "operating": elaborated.operating,
        "requirements": elaborated.requirements,
    })
}

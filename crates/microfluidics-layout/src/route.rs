//! Routing a channel between two ports.
//!
//! A channel is a path of straight runs along X and Y, from one port through any points given
//! to another. It leaves its first port along that port's direction and enters its last against
//! that port's. Where two points are not in line the route turns a corner: once to change axis,
//! or twice, half-way, to arrive along the axis it left on.

use microfluidics_core::model::DuctSpec;

use crate::geometry::{Direction, Point, TOLERANCE};
use crate::rectangle::{Port, Rectangle};

/// A routed channel: its outlines, and the ducts fluid passes through.
#[derive(Debug, Clone, PartialEq)]
pub struct Routed {
    pub rectangles: Vec<Rectangle>,
    pub ducts: Vec<DuctSpec>,
}

fn vertical(direction: Direction) -> bool {
    matches!(direction, Direction::PlusY | Direction::MinusY)
}

fn in_line(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= TOLERANCE || (a.y - b.y).abs() <= TOLERANCE
}

/// The corners of the route: every point where it starts, turns or ends.
fn corners(from: &Port, to: &Port, via: &[Point]) -> Vec<Point> {
    let mut line = vec![from.position];
    let mut heading_vertical = vertical(from.direction);
    let targets: Vec<Point> = via.iter().copied().chain([to.position]).collect();
    for (at, &target) in targets.iter().enumerate() {
        let here = line[line.len() - 1];
        if in_line(here, target) {
            if (here.x - target.x).abs() > TOLERANCE || (here.y - target.y).abs() > TOLERANCE {
                heading_vertical = (here.x - target.x).abs() <= TOLERANCE;
                line.push(target);
            }
            continue;
        }
        let last = at + 1 == targets.len();
        if last && heading_vertical == vertical(to.direction) {
            // Arrive along the axis the route is on: jog across half-way.
            if heading_vertical {
                let middle = (here.y + target.y) / 2.0;
                line.extend([Point::new(here.x, middle), Point::new(target.x, middle), target]);
            } else {
                let middle = (here.x + target.x) / 2.0;
                line.extend([Point::new(middle, here.y), Point::new(middle, target.y), target]);
            }
        } else {
            // Carry on to the target's level, then turn toward it.
            let corner = if heading_vertical {
                Point::new(here.x, target.y)
            } else {
                Point::new(target.x, here.y)
            };
            line.extend([corner, target]);
            heading_vertical = !heading_vertical;
        }
    }
    line
}

/// The rectangles and ducts of the channel `name`, `width` wide on `layer`, from `from` to `to`
/// through `via`.
pub fn route(name: &str, layer: &str, width: f64, from: &Port, to: &Port, via: &[Point]) -> Result<Routed, String> {
    if from.layer != layer || to.layer != layer {
        return Err(format!(
            "the channel `{name}` is on layer `{layer}`, but joins ports on `{}` and `{}`",
            from.layer, to.layer
        ));
    }
    let line = corners(from, to, via);
    if line.len() < 2 {
        return Err(format!(
            "the channel `{name}` has no length: `{}` and `{}` are at one place",
            from.name, to.name
        ));
    }
    let along =
        |a: Point, b: Point, direction: Direction| (b.x - a.x) * direction.unit().x + (b.y - a.y) * direction.unit().y;
    if along(line[0], line[1], from.direction) <= 0.0 {
        return Err(format!(
            "the channel `{name}` would leave `{}` against the way that port faces",
            from.name
        ));
    }
    if along(line[line.len() - 2], line[line.len() - 1], to.direction) >= 0.0 {
        return Err(format!("the channel `{name}` would enter `{}` from behind", to.name));
    }

    let half = width / 2.0;
    let runs = line.len() - 1;
    let mut rectangles = Vec::new();
    let mut length = 0.0;
    let add = |min: Point, max: Point, rectangles: &mut Vec<Rectangle>| {
        let part = if runs == 1 {
            name.to_owned()
        } else {
            format!("{name}.{}", rectangles.len())
        };
        rectangles.push(Rectangle::rect(&part, layer, min, max));
    };
    for run in 0..runs {
        let (a, b) = (line[run], line[run + 1]);
        length += (b.x - a.x).abs() + (b.y - a.y).abs();
        // A run stops half a width short of each corner; a square fills the corner.
        let (start, end) = (
            if run == 0 { 0.0 } else { half },
            if run + 1 == runs { 0.0 } else { half },
        );
        let run_length = (b.x - a.x).abs() + (b.y - a.y).abs() - start - end;
        if run_length <= TOLERANCE {
            return Err(format!("the channel `{name}` turns twice within its own width"));
        }
        let unit = Point::new(
            (b.x - a.x).signum() * f64::from(u8::from((b.x - a.x).abs() > TOLERANCE)),
            (b.y - a.y).signum() * f64::from(u8::from((b.y - a.y).abs() > TOLERANCE)),
        );
        let (p, q) = (
            Point::new(a.x + unit.x * start, a.y + unit.y * start),
            Point::new(b.x - unit.x * end, b.y - unit.y * end),
        );
        let across = Point::new(unit.y.abs() * half, unit.x.abs() * half);
        add(
            Point::new(p.x.min(q.x) - across.x, p.y.min(q.y) - across.y),
            Point::new(p.x.max(q.x) + across.x, p.y.max(q.y) + across.y),
            &mut rectangles,
        );
        if run + 1 < runs {
            add(
                Point::new(b.x - half, b.y - half),
                Point::new(b.x + half, b.y + half),
                &mut rectangles,
            );
        }
    }
    Ok(Routed {
        rectangles,
        ducts: vec![DuctSpec {
            layer: layer.to_owned(),
            width,
            length,
        }],
    })
}

#[cfg(test)]
mod tests {
    use microfluidics_core::model::PortKind;

    use crate::fixtures::close;
    use crate::geometry::{Direction, Point};
    use crate::rectangle::Port;
    use crate::route::route;

    fn port(name: &str, x: f64, y: f64, direction: Direction) -> Port {
        Port {
            name: name.into(),
            kind: PortKind::Both,
            position: Point::new(x * 1e-6, y * 1e-6),
            width: 100e-6,
            layer: "flow".into(),
            direction,
        }
    }

    #[test]
    fn ports_in_line_are_joined_by_one_straight_run() {
        let routed = route(
            "riser",
            "flow",
            100e-6,
            &port("a", 500.0, 100.0, Direction::PlusY),
            &port("b", 500.0, 600.0, Direction::MinusY),
            &[],
        )
        .unwrap();
        assert_eq!(
            (routed.rectangles.len(), routed.rectangles[0].name.as_str()),
            (1, "riser")
        );
        let bounds = routed.rectangles[0].bounds();
        assert!(
            close(bounds.min.x, 450e-6)
                && close(bounds.max.x, 550e-6)
                && close(bounds.min.y, 100e-6)
                && close(bounds.max.y, 600e-6)
        );
        assert!(close(routed.ducts[0].length, 500e-6));
    }

    #[test]
    fn ports_out_of_line_are_joined_through_corners_without_overlap() {
        // Both face along Y: the route jogs across half-way.
        let jogged = route(
            "r",
            "flow",
            100e-6,
            &port("a", 500.0, 100.0, Direction::PlusY),
            &port("b", 1500.0, 1100.0, Direction::MinusY),
            &[],
        )
        .unwrap();
        assert_eq!(jogged.rectangles.len(), 5);
        assert!(close(jogged.ducts[0].length, 2000e-6));
        for (at, a) in jogged.rectangles.iter().enumerate() {
            assert!(
                jogged.rectangles[at + 1..]
                    .iter()
                    .all(|b| !a.bounds().interiors_overlap(&b.bounds()))
            );
        }
        let area: f64 = jogged.rectangles.iter().map(|rectangle| rectangle.area()).sum();
        assert!(close(area, 2000e-6 * 100e-6));
        // One faces along Y and one along X: a single corner.
        let turned = route(
            "r",
            "flow",
            100e-6,
            &port("a", 500.0, 100.0, Direction::PlusY),
            &port("b", 1500.0, 1100.0, Direction::MinusX),
            &[],
        )
        .unwrap();
        assert_eq!(turned.rectangles.len(), 3);
        // Through a point given: up, across to it, and on.
        let via = [Point::new(900e-6, 400e-6)];
        let through = route(
            "r",
            "flow",
            100e-6,
            &port("a", 500.0, 100.0, Direction::PlusY),
            &port("b", 900.0, 1100.0, Direction::MinusY),
            &via,
        )
        .unwrap();
        assert!(close(through.ducts[0].length, (300.0 + 400.0 + 700.0) * 1e-6));
    }

    #[test]
    fn a_route_that_cannot_be_made_says_why() {
        let (a, b) = (
            port("a", 500.0, 100.0, Direction::PlusY),
            port("b", 500.0, 600.0, Direction::MinusY),
        );
        let below = port("c", 500.0, 0.0, Direction::PlusY);
        assert!(
            route("r", "flow", 100e-6, &a, &below, &[])
                .unwrap_err()
                .contains("against the way that port faces")
        );
        let behind = port("b", 500.0, 600.0, Direction::PlusY);
        assert!(
            route("r", "flow", 100e-6, &a, &behind, &[])
                .unwrap_err()
                .contains("from behind")
        );
        assert!(
            route("r", "trench", 100e-6, &a, &b, &[])
                .unwrap_err()
                .contains("is on layer `trench`")
        );
        let near = port("b", 530.0, 600.0, Direction::MinusY);
        assert!(
            route("r", "flow", 100e-6, &a, &near, &[])
                .unwrap_err()
                .contains("turns twice within its own width")
        );
    }
}

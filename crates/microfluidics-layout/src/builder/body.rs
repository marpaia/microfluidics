//! What a body is made of: rectangles, ports, access points, edges and routed channels.

use microfluidics_core::model::{DuctSpec, EdgeSpec, PortKind};

use crate::builder::{BuildError, Builder, Origin, listed};
use crate::geometry::{Direction, Point, TOLERANCE};
use crate::plan::Fiducial;
use crate::rectangle::{Port, Rectangle};
use crate::route::route;
use crate::structure::{Structure, Translates};

/// The name of the vertex at a region's access point, within the region.
pub(crate) const ACCESS: &str = "access";
/// The name of a region's fiducial, within the region.
pub(crate) const FIDUCIAL: &str = "fiducial";

fn corners(name: &str, from: Point, to: Point, at: &Origin) -> Result<(Point, Point), BuildError> {
    // Drawn on the grid: a corner between two grid points would be written to a mask as one of
    // them, and which one would depend on what it was drawn beside.
    let (from, to) = (from.snapped(), to.snapped());
    let (min, max) = (
        Point::new(from.x.min(to.x), from.y.min(to.y)),
        Point::new(from.x.max(to.x), from.y.max(to.y)),
    );
    if max.x - min.x <= TOLERANCE || max.y - min.y <= TOLERANCE {
        return Err(BuildError::at(at, format!("`{name}` has no area")));
    }
    Ok((min, max))
}

impl Builder {
    /// A rectangle on a layer, by two opposite corners.
    pub fn rectangle(
        &mut self,
        name: &str,
        layer: &str,
        from: Point,
        to: Point,
        at: &Origin,
    ) -> Result<(), BuildError> {
        let layer = self.layer_named(layer, at)?;
        let (min, max) = corners(name, from, to, at)?;
        self.name(name, at)?;
        self.built
            .layout
            .rectangles
            .push(Rectangle::rect(name, layer, min, max));
        Ok(())
    }

    /// `count` rectangles alike, each `step` from the last: element `i` is `name[i]`. They are
    /// held as one outline placed along a line, however many there are.
    #[allow(clippy::too_many_arguments)]
    pub fn translates(
        &mut self,
        name: &str,
        layer: &str,
        from: Point,
        to: Point,
        step: Point,
        count: u32,
        at: &Origin,
    ) -> Result<(), BuildError> {
        let layer = self.layer_named(layer, at)?;
        let (min, max) = corners(&format!("{name}[0]"), from, to, at)?;
        if count == 0 {
            return Err(BuildError::at(at, format!("`{name}` is a row of no rectangles")));
        }
        self.name(name, at)?;
        if count == 1 {
            self.built
                .layout
                .rectangles
                .push(Rectangle::rect(&format!("{name}[0]"), layer, min, max));
            return Ok(());
        }
        let (width, height) = (max.x - min.x, max.y - min.y);
        let nm = |metres: f64| (metres * 1e9).round() as i64;
        let structure = format!("{layer}_{}x{}", nm(width), nm(height));
        let outline = Rectangle::rect(&structure, layer, Point::new(0.0, 0.0), Point::new(width, height));
        self.built
            .layout
            .structures
            .entry(structure.clone())
            .or_insert_with(|| Structure {
                name: structure.clone(),
                rectangles: vec![outline],
            });
        self.built.layout.translates.push(Translates {
            name: name.to_owned(),
            structure,
            origin: min,
            step: step.snapped(),
            count,
            quarter_turns: 0,
            first: 0,
        });
        Ok(())
    }

    /// A port: where a channel meets this body's edge.
    #[allow(clippy::too_many_arguments)]
    pub fn port(
        &mut self,
        name: &str,
        kind: PortKind,
        position: Point,
        width: f64,
        layer: &str,
        facing: Direction,
        at: &Origin,
    ) -> Result<(), BuildError> {
        let layer = self.layer_named(layer, at)?.to_owned();
        if width <= 0.0 {
            return Err(BuildError::at(
                at,
                format!("the port `{name}` must have a width above zero"),
            ));
        }
        self.name(name, at)?;
        self.built.layout.ports.push(Port {
            name: name.to_owned(),
            kind,
            position: position.snapped(),
            width,
            layer,
            direction: facing,
        });
        Ok(())
    }

    /// Where a punched hole brings tubing to this body. Edges reach it as the vertex `access`.
    pub fn access(&mut self, position: Point, at: &Origin) -> Result<(), BuildError> {
        self.name(ACCESS, at)?;
        self.built.layout.access.push(position.snapped());
        Ok(())
    }

    /// The centre of a mark this body draws for aligning one layer to another.
    pub fn fiducial(&mut self, position: Point, at: &Origin) -> Result<(), BuildError> {
        self.name(FIDUCIAL, at)?;
        self.built.layout.fiducials.push(Fiducial {
            name: FIDUCIAL.to_owned(),
            position: position.snapped(),
        });
        Ok(())
    }

    /// A vertex inside this body: a place flow passes that is not a port.
    pub fn vertex(&mut self, name: &str, at: &Origin) -> Result<(), BuildError> {
        self.name(name, at)
    }

    /// A way fluid passes through this body, between two of its ports or vertices, as ducts in series.
    pub fn edge(&mut self, from: &str, to: &str, ducts: Vec<DuctSpec>, at: &Origin) -> Result<(), BuildError> {
        for duct in &ducts {
            self.layer_named(&duct.layer, at)?;
        }
        if ducts.is_empty() || ducts.iter().any(|duct| duct.width <= 0.0 || duct.length < 0.0) {
            return Err(BuildError::at(
                at,
                "an edge has at least one duct, each with a width above zero and a length not below",
            ));
        }
        for end in [from, to] {
            if !self.named.contains(end) {
                return Err(BuildError::at(
                    at,
                    format!("an edge ends at `{end}`, which is not a port or a vertex of this region"),
                ));
            }
        }
        self.built.edges.push(EdgeSpec {
            element: String::new(),
            from: from.to_owned(),
            to: to.to_owned(),
            ducts,
        });
        Ok(())
    }

    /// Something a region knows about itself, such as how many places it has for observing structures.
    pub fn quantity(&mut self, name: &str, value: f64, at: &Origin) -> Result<(), BuildError> {
        self.name(name, at)?;
        self.built.quantities.insert(name.to_owned(), value);
        Ok(())
    }

    fn port_named(&self, name: &str, at: &Origin) -> Result<Port, BuildError> {
        let found = self.built.layout.ports.iter().find(|held| held.name == name).cloned();
        found.ok_or_else(|| {
            let known = listed(self.built.layout.ports.iter().map(|port| port.name.as_str()));
            BuildError::at(
                at,
                format!("there is no port `{name}` here; the ports so far are {known}"),
            )
        })
    }

    /// A channel routed between two ports of this body, along X and Y, through any points given.
    #[allow(clippy::too_many_arguments)]
    pub fn channel(
        &mut self,
        name: &str,
        layer: &str,
        width: f64,
        from: &str,
        to: &str,
        via: &[Point],
        at: &Origin,
    ) -> Result<(), BuildError> {
        let layer = self.layer_named(layer, at)?;
        let (from, to) = (self.port_named(from, at)?, self.port_named(to, at)?);
        let routed = route(name, layer, width, &from, &to, via).map_err(|message| BuildError::at(at, message))?;
        self.name(name, at)?;
        self.built
            .layout
            .rectangles
            .extend(routed.rectangles.into_iter().map(|mut rectangle| {
                for point in &mut rectangle.polygon {
                    *point = point.snapped();
                }
                rectangle
            }));
        self.built.edges.push(EdgeSpec {
            element: name.to_owned(),
            from: from.name,
            to: to.name,
            ducts: routed.ducts,
        });
        Ok(())
    }
}

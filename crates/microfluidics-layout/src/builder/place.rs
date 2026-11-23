//! Placing a region: its parameters settled, a body for it to be built in, and where it goes.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use microfluidics_core::Quantity;
use microfluidics_core::model::EdgeSpec;

use crate::builder::model::{Built, Instance, Parameter};
use crate::builder::{BuildError, Builder, MAX_DEPTH, Origin, ParamSpec, Place, RegionSpec, listed};
use crate::geometry::{Point, TOLERANCE, snapped};

/// Hold a parameter's value to its kind and range.
fn admitted(region: &RegionSpec, param: &ParamSpec, value: Quantity) -> Result<(), String> {
    let what = format!("`{}` of `{}`", param.name, region.name);
    value.expect(param.default.dim, &what)?;
    if param.count && (value.si.fract() != 0.0 || value.si < 0.0) {
        return Err(format!("{what} is a whole number, not {}", value.si));
    }
    if let Some((low, high)) = param.range {
        let slack = TOLERANCE.max(1e-12 * high.si.abs());
        if value.si < low.si - slack || value.si > high.si + slack {
            return Err(format!(
                "{what} is {}, outside {}..{}",
                value.written(),
                low.written(),
                high.written()
            ));
        }
    }
    Ok(())
}

impl Builder {
    /// A body for the instance `instance` of `region`, with its parameters settled: each is what
    /// `given` holds, or its default, replaced by an override of its path when there is one,
    /// and held to its kind and range.
    pub fn child(
        &self,
        instance: &str,
        region: &RegionSpec,
        given: &BTreeMap<String, Quantity>,
        at: &Origin,
    ) -> Result<Builder, BuildError> {
        if self.depth >= MAX_DEPTH {
            let message = format!(
                "regions nest more than {MAX_DEPTH} deep; does `{}` contain itself?",
                region.name
            );
            return Err(BuildError::at(at, message));
        }
        if let Some(unknown) = given
            .keys()
            .find(|name| !region.params.iter().any(|param| &param.name == *name))
        {
            let known = listed(region.params.iter().map(|param| param.name.as_str()));
            let message = format!(
                "`{}` has no parameter `{unknown}`; its parameters are {known}",
                region.name
            );
            return Err(BuildError::at(at, message));
        }
        let path = format!("{}{instance}.", self.path);
        let mut settled = Vec::new();
        for param in &region.params {
            let written = given.get(&param.name).copied().unwrap_or(param.default);
            let value = self
                .overrides
                .0
                .get(&format!("{path}{}", param.name))
                .copied()
                .unwrap_or(written);
            admitted(region, param, value).map_err(|message| BuildError::at(at, message))?;
            settled.push((
                param.name.clone(),
                Parameter {
                    value,
                    range: param.range,
                    count: param.count,
                },
            ));
        }
        Ok(Builder {
            name: instance.to_owned(),
            params: region
                .params
                .iter()
                .map(|param| format!("{path}{}", param.name))
                .collect(),
            path,
            depth: self.depth + 1,
            overrides: Rc::clone(&self.overrides),
            layers: Rc::clone(&self.layers),
            built: Built::default(),
            origins: BTreeMap::new(),
            named: BTreeSet::new(),
            region: region.name.clone(),
            instances: Vec::new(),
            settled,
            origin: at.clone(),
            slab: None,
            glass: None,
            stack: Vec::new(),
            processes: Vec::new(),
            own_conditions: self.own_conditions,
        })
    }

    /// Place what `child` built into this body, with the name it was made for before every name
    /// of its own.
    pub fn place(&mut self, child: Builder, place: &Place, at: &Origin) -> Result<(), BuildError> {
        let (name, made) = (child.name, child.built);
        let region = child.region;
        let (turns, by) = match place {
            Place::At { point, turned } => (turned % 4, point.snapped()),
            Place::Attach { port, to, offset } => {
                let own = made
                    .layout
                    .ports
                    .iter()
                    .find(|held| &held.name == port)
                    .ok_or_else(|| {
                        let known = listed(made.layout.ports.iter().map(|port| port.name.as_str()));
                        BuildError::at(at, format!("`{region}` has no port `{port}`; its ports are {known}"))
                    })?;
                let target = self
                    .built
                    .layout
                    .ports
                    .iter()
                    .find(|held| &held.name == to)
                    .ok_or_else(|| {
                        let known = listed(self.built.layout.ports.iter().map(|port| port.name.as_str()));
                        BuildError::at(
                            at,
                            format!("there is no port `{to}` here; the ports so far are {known}"),
                        )
                    })?;
                let turns = (0..4)
                    .find(|turns| own.direction.turned(*turns) == target.direction.opposite())
                    .unwrap_or(0);
                let offset = snapped(*offset);
                let (moved, along) = (own.position.turned(turns), target.direction.unit());
                let anchor = Point::new(
                    target.position.x + offset * along.x,
                    target.position.y + offset * along.y,
                );
                // Ports placed together are joined; with a gap between them, a channel must join them.
                if offset.abs() <= TOLERANCE {
                    self.built
                        .joints
                        .push((format!("{name}.{}", own.name), target.name.clone()));
                }
                (turns, Point::new(anchor.x - moved.x, anchor.y - moved.y))
            }
        };
        self.name(&name, &child.origin)?;
        self.params.extend(child.params);
        let prefix = format!("{name}.");
        self.instances.push(Instance {
            path: name.clone(),
            region,
            parameters: (child.settled.iter())
                .map(|(parameter, held)| (parameter.clone(), held.value))
                .collect(),
        });
        self.instances.extend(child.instances.into_iter().map(|held| Instance {
            path: format!("{prefix}{}", held.path),
            ..held
        }));
        let built = &mut self.built;
        built.edges.extend(made.edges.into_iter().map(|edge| EdgeSpec {
            element: if edge.element.is_empty() {
                name.clone()
            } else {
                format!("{prefix}{}", edge.element)
            },
            from: format!("{prefix}{}", edge.from),
            to: format!("{prefix}{}", edge.to),
            ducts: edge.ducts,
        }));
        built.parameters.extend(
            child
                .settled
                .into_iter()
                .map(|(parameter, held)| (format!("{prefix}{parameter}"), held)),
        );
        built.parameters.extend(
            made.parameters
                .into_iter()
                .map(|(path, held)| (format!("{prefix}{path}"), held)),
        );
        built.joints.extend(
            made.joints
                .into_iter()
                .map(|(a, b)| (format!("{prefix}{a}"), format!("{prefix}{b}"))),
        );
        built.quantities.extend(
            made.quantities
                .into_iter()
                .map(|(quantity, value)| (format!("{prefix}{quantity}"), value)),
        );
        built.layout.merge(made.layout, &prefix, turns, by);
        self.origins.extend(
            child
                .origins
                .into_iter()
                .map(|(path, origin)| (format!("{prefix}{path}"), origin)),
        );
        Ok(())
    }
}

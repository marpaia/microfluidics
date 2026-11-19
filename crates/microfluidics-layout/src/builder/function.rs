//! How a design is run and what it must do: its operation, its requirements, and what may be
//! tuned to meet them.

use std::collections::{BTreeMap, BTreeSet};

use microfluidics_core::model::{Drive, Measure, Operating, Requirement};

use crate::builder::body::ACCESS;
use crate::builder::model::Tunable;
use crate::builder::{BuildError, Builder, Origin, listed};

impl Builder {
    /// Every name a requirement or a drive can speak of: what edges belong to, and the vertices
    /// they join.
    fn places(&self) -> BTreeSet<&str> {
        let edges = self
            .built
            .edges
            .iter()
            .flat_map(|edge| [edge.element.as_str(), edge.from.as_str(), edge.to.as_str()]);
        let ports = self.built.layout.ports.iter().map(|port| port.name.as_str());
        edges.chain(ports).filter(|name| !name.is_empty()).collect()
    }

    /// The vertex at the access point of the instance `place`.
    fn access_of(&self, place: &str, at: &Origin) -> Result<String, BuildError> {
        let vertex = format!("{place}.{ACCESS}");
        if self.places().contains(vertex.as_str()) {
            return Ok(vertex);
        }
        let ending = format!(".{ACCESS}");
        let holders: Vec<&str> = self
            .places()
            .into_iter()
            .filter_map(|name| name.strip_suffix(ending.as_str()))
            .collect();
        let known = listed(holders.into_iter());
        Err(BuildError::at(
            at,
            format!("`{place}` has no access point; the access points are at {known}"),
        ))
    }

    /// How the chip is run: the viscosity of what flows, what is applied at each access point,
    /// and what enters there. `drives` and `feeds` name instances that have an access point.
    pub fn operate(
        &mut self,
        viscosity: f64,
        drives: &[(String, Drive)],
        feeds: &[(String, BTreeMap<String, f64>)],
        at: &Origin,
    ) -> Result<(), BuildError> {
        self.design_only(at, "says how it is run")?;
        if !self.own_conditions {
            return Ok(());
        }
        if self.built.operating.is_some() {
            return Err(BuildError::at(at, "a design says how it is run once"));
        }
        if viscosity <= 0.0 {
            return Err(BuildError::at(at, "the viscosity must be above zero"));
        }
        let mut operating = Operating {
            viscosity,
            ..Operating::default()
        };
        for (place, drive) in drives {
            if operating.drives.insert(self.access_of(place, at)?, *drive).is_some() {
                return Err(BuildError::at(at, format!("`{place}` is driven twice")));
            }
        }
        for (place, fractions) in feeds {
            let total: f64 = fractions.values().sum();
            if fractions.values().any(|fraction| *fraction < 0.0) || total > 1.0 + 1e-9 {
                let message = "the fractions fed at a point are each at least zero and together at most 100%";
                return Err(BuildError::at(at, message));
            }
            operating.feeds.insert(self.access_of(place, at)?, fractions.clone());
        }
        if !operating
            .drives
            .values()
            .any(|drive| matches!(drive, Drive::Pressure(_)))
        {
            let message = "at least one access point is driven with a pressure: flow needs somewhere to go";
            return Err(BuildError::at(at, message));
        }
        self.built.operating = Some(operating);
        Ok(())
    }

    /// Something the chip must do. Its subjects are what flow passes through, or vertices, by path.
    pub fn require(&mut self, requirement: Requirement, at: &Origin) -> Result<(), BuildError> {
        self.design_only(at, "states requirements")?;
        if !self.own_conditions {
            return Ok(());
        }
        let name = &requirement.name;
        if self.built.requirements.iter().any(|held| &held.name == name) {
            return Err(BuildError::at(at, format!("the requirement `{name}` is stated twice")));
        }
        if requirement.subjects.is_empty() != (requirement.measure == Measure::Yield) {
            let message = match requirement.measure {
                Measure::Yield => "the yield is of the whole chip and takes no places".to_owned(),
                _ => format!("the requirement `{name}` names nowhere to measure"),
            };
            return Err(BuildError::at(at, message));
        }
        if requirement.measure == Measure::Drop && requirement.subjects.len() != 2 {
            return Err(BuildError::at(at, "a drop takes two places: where it is from, and to"));
        }
        let places = self.places();
        if let Some(unknown) = requirement
            .subjects
            .iter()
            .find(|subject| !places.contains(subject.as_str()))
        {
            let elements: BTreeSet<&str> = self.built.edges.iter().map(|edge| edge.element.as_str()).collect();
            let known = listed(elements.into_iter());
            let message = format!("`{unknown}` is nothing flow passes here; what flow passes through is {known}");
            return Err(BuildError::at(at, message));
        }
        self.origins.insert(name.clone(), at.clone());
        self.built.requirements.push(requirement);
        Ok(())
    }

    /// Allow the parameter at `path` to be changed, within its range, to meet the requirements.
    pub fn tune(&mut self, path: &str, at: &Origin) -> Result<(), BuildError> {
        self.design_only(at, "says what may be tuned")?;
        let Some(held) = self.built.parameters.get(path) else {
            let ranged = self
                .built
                .parameters
                .iter()
                .filter(|(_, held)| held.range.is_some())
                .map(|(path, _)| path.as_str());
            let known = listed(ranged);
            let message = format!("there is no parameter `{path}`; those with a range to tune within are {known}");
            return Err(BuildError::at(at, message));
        };
        let Some((low, high)) = held.range else {
            return Err(BuildError::at(
                at,
                format!("`{path}` has no range to be tuned within; give it one"),
            ));
        };
        if self.built.tunables.iter().any(|tunable| tunable.path == path) {
            return Err(BuildError::at(at, format!("`{path}` is tuned twice")));
        }
        self.built.tunables.push(Tunable {
            path: path.to_owned(),
            value: held.value,
            low,
            high,
            count: held.count,
        });
        Ok(())
    }
}

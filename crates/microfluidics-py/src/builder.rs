//! The builder: what the package's `Region` and `Design` classes call to make a design.

use std::collections::BTreeMap;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use microfluidics_core::model::{Bound as Limit, Drive, DuctSpec, LayerSpec, Measure, PortKind, Requirement};
use microfluidics_core::{Dim, Quantity};
use microfluidics_layout::{Builder, Direction, Origin, Overrides, ParamSpec, Place, Point, RegionSpec};

use crate::design::{PyElaborated, PyOverrides};
use crate::error::{raised, stated};
use crate::quantity::{object, quantity, si};

/// A point given as a pair of lengths.
fn point(value: &Bound<'_, PyAny>) -> PyResult<Point> {
    let (x, y): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
        value.extract().map_err(|_| stated("a point is a pair of lengths"))?;
    Ok(Point::new(
        si(&x, Dim::LENGTH, "a point's x")?,
        si(&y, Dim::LENGTH, "a point's y")?,
    ))
}

fn length(value: &Bound<'_, PyAny>, what: &str) -> PyResult<f64> {
    si(value, Dim::LENGTH, what)
}

fn facing(name: &str) -> PyResult<Direction> {
    match name {
        "up" => Ok(Direction::PlusY),
        "down" => Ok(Direction::MinusY),
        "left" => Ok(Direction::MinusX),
        "right" => Ok(Direction::PlusX),
        other => Err(stated(format!("`{other}` is not a way to face"))),
    }
}

fn kind(name: &str) -> PyResult<PortKind> {
    match name {
        "inlet" => Ok(PortKind::Inlet),
        "outlet" => Ok(PortKind::Outlet),
        "both" => Ok(PortKind::Both),
        other => Err(stated(format!("`{other}` is not a kind of port"))),
    }
}

/// The values a dictionary of arguments holds, by parameter.
fn values(given: &Bound<'_, PyDict>) -> PyResult<BTreeMap<String, Quantity>> {
    let mut held = BTreeMap::new();
    for (name, value) in given.iter() {
        held.insert(name.extract::<String>()?, quantity(&value)?);
    }
    Ok(held)
}

/// What a region states of itself, from the package's description of it: its name, what it
/// is, its file, and each parameter as `(name, default, low, high, count)`.
fn spec(name: &str, doc: &str, source: &str, params: &Bound<'_, PyAny>) -> PyResult<RegionSpec> {
    let mut held = Vec::new();
    for param in params.try_iter()? {
        let (name, default, low, high, count): (String, Bound<'_, PyAny>, Bound<'_, PyAny>, Bound<'_, PyAny>, bool) =
            param?.extract()?;
        let range = if low.is_none() || high.is_none() {
            None
        } else {
            Some((quantity(&low)?, quantity(&high)?))
        };
        held.push(ParamSpec {
            name,
            default: quantity(&default)?,
            range,
            count,
        });
    }
    Ok(RegionSpec {
        name: name.to_owned(),
        doc: doc.to_owned(),
        source: source.to_owned(),
        params: held,
    })
}

/// Where a region goes: at a point, or attached by a port.
fn place(
    at: Option<&Bound<'_, PyAny>>,
    turned: u8,
    attach: Option<String>,
    to: Option<String>,
    offset: Option<&Bound<'_, PyAny>>,
) -> PyResult<Place> {
    match (at, attach, to) {
        (Some(at), None, None) => Ok(Place::At {
            point: point(at)?,
            turned: turned % 4,
        }),
        (None, Some(port), Some(to)) => {
            let offset = offset.map_or(Ok(0.0), |offset| length(offset, "an offset"))?;
            Ok(Place::Attach { port, to, offset })
        }
        _ => Err(stated(
            "a region is placed at a point, or attached by one of its ports to another",
        )),
    }
}

/// One body being built.
#[pyclass(unsendable, name = "Builder", module = "microfluidics._engine")]
pub struct PyBuilder {
    inner: Option<Builder>,
}

impl PyBuilder {
    fn body(&mut self) -> PyResult<&mut Builder> {
        self.inner
            .as_mut()
            .ok_or_else(|| stated("this body has already been placed or finished"))
    }

    fn held(&self) -> PyResult<&Builder> {
        self.inner
            .as_ref()
            .ok_or_else(|| stated("this body has already been placed or finished"))
    }
}

#[pymethods]
impl PyBuilder {
    /// A builder for the design `name`.
    #[staticmethod]
    #[pyo3(signature = (name, overrides=None))]
    fn design(name: &str, overrides: Option<&PyOverrides>) -> Self {
        let none = Overrides::default();
        Self {
            inner: Some(Builder::design(
                name,
                overrides.map_or(&none, |overrides| &overrides.inner),
            )),
        }
    }

    /// The path of this body from the design, with a trailing dot; empty for the design.
    fn prefix(&self) -> PyResult<String> {
        Ok(self.held()?.prefix().to_owned())
    }

    fn rectangle(
        &mut self,
        name: &str,
        layer: &str,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let (start, end) = (point(start)?, point(end)?);
        self.body()?
            .rectangle(name, layer, start, end, &Origin::new(file, line))
            .map_err(raised)
    }

    #[allow(clippy::too_many_arguments)]
    fn translates(
        &mut self,
        name: &str,
        layer: &str,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        by: &Bound<'_, PyAny>,
        count: u32,
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let (start, end, by) = (point(start)?, point(end)?, point(by)?);
        self.body()?
            .translates(name, layer, start, end, by, count, &Origin::new(file, line))
            .map_err(raised)
    }

    #[allow(clippy::too_many_arguments)]
    fn port(
        &mut self,
        name: &str,
        kind_of: &str,
        at: &Bound<'_, PyAny>,
        width: &Bound<'_, PyAny>,
        layer: &str,
        faces: &str,
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let (position, width) = (point(at)?, length(width, "a port's width")?);
        self.body()?
            .port(
                name,
                kind(kind_of)?,
                position,
                width,
                layer,
                facing(faces)?,
                &Origin::new(file, line),
            )
            .map_err(raised)
    }

    fn access(&mut self, at: &Bound<'_, PyAny>, file: &str, line: u32) -> PyResult<()> {
        let position = point(at)?;
        self.body()?.access(position, &Origin::new(file, line)).map_err(raised)
    }

    fn vertex(&mut self, name: &str, file: &str, line: u32) -> PyResult<()> {
        self.body()?.vertex(name, &Origin::new(file, line)).map_err(raised)
    }

    /// An edge between two ports or vertices, through ducts given as `(layer, width, length)`.
    fn edge(
        &mut self,
        start: &str,
        end: &str,
        ducts: Vec<(String, Bound<'_, PyAny>, Bound<'_, PyAny>)>,
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let mut held = Vec::new();
        for (layer, width, long) in &ducts {
            held.push(DuctSpec {
                layer: layer.clone(),
                width: length(width, "a duct's width")?,
                length: length(long, "a duct's length")?,
            });
        }
        self.body()?
            .edge(start, end, held, &Origin::new(file, line))
            .map_err(raised)
    }

    fn quantity(&mut self, name: &str, value: &Bound<'_, PyAny>, file: &str, line: u32) -> PyResult<()> {
        let value = quantity(value)?.si;
        self.body()?
            .quantity(name, value, &Origin::new(file, line))
            .map_err(raised)
    }

    #[allow(clippy::too_many_arguments)]
    fn channel(
        &mut self,
        name: &str,
        layer: &str,
        width: &Bound<'_, PyAny>,
        start: &str,
        end: &str,
        via: Vec<Bound<'_, PyAny>>,
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let width = length(width, "a channel's width")?;
        let via = via.iter().map(point).collect::<PyResult<Vec<Point>>>()?;
        self.body()?
            .channel(name, layer, width, start, end, &via, &Origin::new(file, line))
            .map_err(raised)
    }

    /// The height of a layer, or `None` when the design has no such layer.
    fn height(&self, py: Python<'_>, layer: &str) -> PyResult<Option<Py<PyAny>>> {
        self.held()?
            .height(layer)
            .map(|height| object(py, Quantity::length(height)))
            .transpose()
    }

    /// The value this body's parameter `name` was settled at.
    fn settled(&self, py: Python<'_>, name: &str) -> PyResult<Option<Py<PyAny>>> {
        self.held()?.settled(name).map(|value| object(py, value)).transpose()
    }

    /// A body for an instance of a region, its parameters settled.
    #[allow(clippy::too_many_arguments)]
    fn child(
        &self,
        instance: &str,
        name: &str,
        doc: &str,
        source: &str,
        params: &Bound<'_, PyAny>,
        given: &Bound<'_, PyDict>,
        file: &str,
        line: u32,
    ) -> PyResult<PyBuilder> {
        let region = spec(name, doc, source, params)?;
        let child = self
            .held()?
            .child(instance, &region, &values(given)?, &Origin::new(file, line))
            .map_err(raised)?;
        Ok(PyBuilder { inner: Some(child) })
    }

    /// Place what `child` built into this body.
    #[pyo3(signature = (child, file, line, at=None, turned=0, attach=None, to=None, offset=None))]
    #[allow(clippy::too_many_arguments)]
    fn place(
        &mut self,
        child: &Bound<'_, PyBuilder>,
        file: &str,
        line: u32,
        at: Option<&Bound<'_, PyAny>>,
        turned: u8,
        attach: Option<String>,
        to: Option<String>,
        offset: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let place = place(at, turned, attach, to, offset)?;
        let built = child
            .borrow_mut()
            .inner
            .take()
            .ok_or_else(|| stated("this region has already been placed"))?;
        self.body()?
            .place(built, &place, &Origin::new(file, line))
            .map_err(raised)
    }

    fn slab(
        &mut self,
        width: &Bound<'_, PyAny>,
        depth: &Bound<'_, PyAny>,
        thickness: &Bound<'_, PyAny>,
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let size = [
            length(width, "a slab's width")?,
            length(depth, "a slab's depth")?,
            length(thickness, "a slab's thickness")?,
        ];
        self.body()?.slab(size, &Origin::new(file, line)).map_err(raised)
    }

    fn glass(
        &mut self,
        width: &Bound<'_, PyAny>,
        depth: &Bound<'_, PyAny>,
        thickness: &Bound<'_, PyAny>,
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let size = [
            length(width, "a glass's width")?,
            length(depth, "a glass's depth")?,
            length(thickness, "a glass's thickness")?,
        ];
        self.body()?.glass(size, &Origin::new(file, line)).map_err(raised)
    }

    fn layer(&mut self, name: &str, height: &Bound<'_, PyAny>, gds: u16, file: &str, line: u32) -> PyResult<()> {
        let spec = LayerSpec {
            name: name.to_owned(),
            height: length(height, "a layer's height")?,
            gds,
        };
        self.body()?.layer(spec, &Origin::new(file, line)).map_err(raised)
    }

    fn process(&mut self, name: &str, file: &str, line: u32) -> PyResult<()> {
        self.body()?.process(name, &Origin::new(file, line)).map_err(raised)
    }

    /// One of the design's own parameters; gives its value, or its override.
    #[allow(clippy::too_many_arguments)]
    fn param(
        &mut self,
        py: Python<'_>,
        name: &str,
        value: &Bound<'_, PyAny>,
        low: &Bound<'_, PyAny>,
        high: &Bound<'_, PyAny>,
        count: bool,
        file: &str,
        line: u32,
    ) -> PyResult<Py<PyAny>> {
        let range = if low.is_none() || high.is_none() {
            None
        } else {
            Some((quantity(low)?, quantity(high)?))
        };
        let settled = self
            .body()?
            .param(name, quantity(value)?, range, count, &Origin::new(file, line))
            .map_err(raised)?;
        object(py, settled)
    }

    /// How the chip is run. `drives` holds `(place, pressure or flow rate)`; `feeds` holds
    /// `(place, {species: fraction})`.
    fn operate(
        &mut self,
        viscosity: &Bound<'_, PyAny>,
        drives: Vec<(String, Bound<'_, PyAny>)>,
        feeds: Vec<(String, BTreeMap<String, f64>)>,
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let viscosity = si(viscosity, Dim::VISCOSITY, "the viscosity")?;
        let mut driven = Vec::new();
        for (place, value) in &drives {
            let value = quantity(value)?;
            let drive = match value.dim {
                Dim::PRESSURE => Drive::Pressure(value.si),
                Dim::FLOW => Drive::FlowRate(value.si),
                other => {
                    return Err(stated(format!(
                        "`{place}` is driven with a pressure or a flow rate, not {}",
                        other.name()
                    )));
                }
            };
            driven.push((place.clone(), drive));
        }
        self.body()?
            .operate(viscosity, &driven, &feeds, &Origin::new(file, line))
            .map_err(raised)
    }

    /// A requirement. `measure` is its kind and, for a fraction, a species; `bound` is its
    /// kind and its numbers in SI units.
    #[allow(clippy::too_many_arguments)]
    fn require(
        &mut self,
        name: &str,
        statement: &str,
        measure: (String, Option<String>),
        subjects: Vec<String>,
        bound: (String, f64, f64, bool),
        file: &str,
        line: u32,
    ) -> PyResult<()> {
        let measure = match (measure.0.as_str(), measure.1) {
            ("flow", _) => Measure::Flow,
            ("pressure", _) => Measure::Pressure,
            ("drop", _) => Measure::Drop,
            ("velocity", _) => Measure::Velocity,
            ("shear", _) => Measure::Shear,
            ("reynolds", _) => Measure::Reynolds,
            ("residence", _) => Measure::Residence,
            ("fraction", Some(species)) => Measure::Fraction { species },
            ("yield", _) => Measure::Yield,
            (other, _) => return Err(stated(format!("`{other}` is not something measured"))),
        };
        let (kind, value, within, relative) = bound;
        let bound = match kind.as_str() {
            "equal_within" => Limit::EqualWithin(value),
            "target" => Limit::Is {
                value,
                within,
                relative,
            },
            "at_most" => Limit::AtMost(value),
            "at_least" => Limit::AtLeast(value),
            other => return Err(stated(format!("`{other}` is not a bound"))),
        };
        let requirement = Requirement {
            name: name.to_owned(),
            statement: statement.to_owned(),
            measure,
            subjects,
            bound,
        };
        self.body()?
            .require(requirement, &Origin::new(file, line))
            .map_err(raised)
    }

    fn tune(&mut self, path: &str, file: &str, line: u32) -> PyResult<()> {
        self.body()?.tune(path, &Origin::new(file, line)).map_err(raised)
    }

    /// Whether the design's own operation and requirements take effect.
    fn own_conditions(&mut self, on: bool) -> PyResult<()> {
        self.body()?.own_conditions(on);
        Ok(())
    }

    /// A fault a region finds in itself, as the exception to raise, naming the instance.
    fn fault(&self, message: &str, file: &str, line: u32) -> PyResult<PyErr> {
        Ok(raised(self.held()?.fault(&Origin::new(file, line), message)))
    }

    /// The finished design.
    fn finish(&mut self) -> PyResult<PyElaborated> {
        let builder = self
            .inner
            .take()
            .ok_or_else(|| stated("this design has already been finished"))?;
        Ok(PyElaborated {
            inner: builder.finish().map_err(raised)?,
        })
    }
}

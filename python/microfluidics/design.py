"""Designs: a chip's slab, layers and processes, what is on it, how it is run and what it must do."""

from __future__ import annotations

import os
import traceback
from collections.abc import Callable, Mapping, Sequence
from typing import Any, ClassVar, Generic, Self, TypeVar, overload

from microfluidics._engine import Builder, DesignError, Elaborated, Overrides, Process, Quantity
from microfluidics.handles import Channel, Named, Species, Vertex
from microfluidics.model import Glass, Slab, Stack
from microfluidics.origin import Origin, authored, caller, reported
from microfluidics.region import Param, Region, _built, _coerced, _sources
from microfluidics.units import FlowRate, Pressure, Time, Viscosity

DimensionT = TypeVar("DimensionT")
DrivenT = TypeVar("DrivenT", bound=Region)
FedT = TypeVar("FedT", bound=Region)

Subject = Region | Channel | Vertex
"""What flow can be measured at or through."""


def _flat(subjects: Sequence[Subject | Sequence[Subject]]) -> tuple[Subject, ...]:
    flat: list[Subject] = []
    for subject in subjects:
        if isinstance(subject, (Region, Channel, Vertex)):
            flat.append(subject)
        else:
            flat.extend(subject)
    return tuple(flat)


class Measure(Generic[DimensionT]):
    """Something measured of the flow, whose values are of the dimension `DimensionT`."""

    def __init__(
        self, kind: str, subjects: Sequence[Subject | Sequence[Subject]], species: Species | None = None
    ) -> None:
        self.kind, self.subjects, self.species = kind, _flat(subjects), species


def flow(*subjects: Subject | Sequence[Subject]) -> Measure[FlowRate]:
    """The flow rate through each subject."""
    return Measure("flow", subjects)


def shear(*subjects: Subject | Sequence[Subject]) -> Measure[Pressure]:
    """The wall shear stress in the tightest duct of each subject."""
    return Measure("shear", subjects)


def pressure(*subjects: Subject | Sequence[Subject]) -> Measure[Pressure]:
    """The pressure at each subject."""
    return Measure("pressure", subjects)


def drop(start: Subject, end: Subject) -> Measure[Pressure]:
    """The pressure lost from one place to another."""
    return Measure("drop", (start, end))


def velocity(*subjects: Subject | Sequence[Subject]) -> Measure[Quantity]:
    """The mean speed in the tightest duct of each subject."""
    return Measure("velocity", subjects)


def reynolds(*subjects: Subject | Sequence[Subject]) -> Measure[float]:
    """The Reynolds number in the tightest duct of each subject."""
    return Measure("reynolds", subjects)


def residence(*subjects: Subject | Sequence[Subject]) -> Measure[Time]:
    """The time fluid takes to pass through each subject."""
    return Measure("residence", subjects)


def fraction(species: Species, at: Subject) -> Measure[float]:
    """How much of what arrives at a place is `species`."""
    return Measure("fraction", (at,), species)


def chip_yield() -> Measure[float]:
    """The share of chips, made within the process's tolerances, that meet every other requirement."""
    return Measure("yield", ())


class Bound(Generic[DimensionT]):
    """What a measure's values must be."""

    def __init__(self, kind: str, value: DimensionT, within: DimensionT | float = 0.0, relative: bool = False) -> None:
        self.kind, self.value, self.within, self.relative = kind, value, within, relative


def equal_within(share: float) -> Bound[Any]:
    """Every value within the fraction `share` of their mean. It holds for a measure of any
    dimension, since a share has none."""
    return Bound("equal_within", share)


def target(value: DimensionT, *, within: DimensionT | float, relative: bool = False) -> Bound[DimensionT]:
    """Every value within `within` of `value`; with `relative`, within that fraction of it."""
    return Bound("target", value, within, relative)


def at_most(value: DimensionT) -> Bound[DimensionT]:
    """Every value no more than `value`."""
    return Bound("at_most", value)


def at_least(value: DimensionT) -> Bound[DimensionT]:
    """Every value no less than `value`."""
    return Bound("at_least", value)


class Requirement(Named, Generic[DimensionT]):
    """Something a chip must do: what it means in words, what is measured, and what the values
    must be. A measure and a bound must be of one dimension."""

    kind = "requirement"

    def __init__(self, statement: str, measure: Measure[DimensionT], bound: Bound[DimensionT]) -> None:
        super().__init__(caller())
        self.statement, self.measure, self.bound = statement, measure, bound


class Conditions:
    """What a benchmark states in place of a candidate's own operation and requirements. A
    requirement is stated by assigning it: `run.balanced = mf.Requirement(...)`."""

    _design: Design

    def __init__(self, design: Design) -> None:
        object.__setattr__(self, "_design", design)

    def operate(
        self,
        *,
        viscosity: Viscosity,
        drive: Mapping[DrivenT, Pressure | FlowRate],
        feed: Mapping[FedT, Mapping[Species, float]] | None = None,
    ) -> None:
        """Say how the candidate is run."""
        self._design.operate(viscosity=viscosity, drive=drive, feed=feed)

    def __setattr__(self, name: str, value: Any) -> None:
        setattr(self._design, name, value)

    def __getattr__(self, name: str) -> Any:
        return getattr(self._design, name)


class Design(Region):
    """A chip. Subclass it, state its `processes`, `slab` and `layers` in the class body with any
    parameters of its own, and place what is on it in `build`."""

    processes: ClassVar[tuple[Process, ...]] = ()
    """The processes the chip is made in: it is held to the rules of every one."""
    slab: ClassVar[Slab]
    glass: ClassVar[Glass]
    layers: ClassVar[Stack]

    def operate(
        self,
        *,
        viscosity: Viscosity,
        drive: Mapping[DrivenT, Pressure | FlowRate],
        feed: Mapping[FedT, Mapping[Species, float]] | None = None,
    ) -> None:
        """Say how the chip is run: the viscosity of what flows, the pressure or flow rate at each
        region with an access point, and what is fed there."""
        origin = caller()
        drives = [(self._local(place, origin), value) for place, value in drive.items()]
        feeds = [
            (self._local(place, origin), {species.path: share for species, share in fed.items()})
            for place, fed in (feed or {}).items()
        ]
        self._body().operate(viscosity, drives, feeds, *origin)

    @overload
    def tune(self, *params: Param[Any]) -> None: ...

    @overload
    def tune(self, target: Region | Sequence[Region], /, *params: Param[Any]) -> None: ...

    def tune(self, *given: Any) -> None:
        """Allow parameters to be changed, within their ranges, to meet the requirements: the
        design's own, or those of a region placed in it."""
        origin = caller()
        if given and not isinstance(given[0], Param):
            targets = [given[0]] if isinstance(given[0], Region) else list(given[0])
            for region in targets:
                for held in given[1:]:
                    if held.owner is not None and not isinstance(region, held.owner):
                        message = (
                            f"`{held.name}` is a parameter of `{held.owner.__name__}`, not of `{type(region).__name__}`"
                        )
                        raise DesignError(message, *origin)
                    self._body().tune(f"{self._local(region, origin)}.{held.name}", *origin)
            return
        for held in given:
            self._body().tune(held.name, *origin)

    def _name_other(self, name: str, item: Named) -> None:
        if not isinstance(item, Requirement):
            super()._name_other(name, item)
            return
        origin = item._origin
        measure, bound = item.measure, item.bound
        subjects = [self._local(subject, origin) for subject in measure.subjects]
        species = measure.species.path if measure.species is not None else None
        limits = (bound.kind, bound.value, bound.within, bound.relative)
        self._body().require(name, item.statement, (measure.kind, species), subjects, limits, *origin)
        item._path = name

    @classmethod
    def elaborate(
        cls, overrides: Overrides | None = None, conditions: Callable[[Self, Conditions], None] | None = None
    ) -> Elaborated:
        """Build the design, with `overrides` replacing parameters by path. With `conditions`,
        they are stated in place of the design's own operation and requirements."""
        stated: Origin = (cls._source, cls._line)
        try:
            builder = Builder.design(cls.region_name(), overrides)
            # A tuple of processes: one written alone, or under the name `process`, would be held to
            # no rule at all without a word.
            if isinstance(cls.processes, Process) or "process" in vars(cls):
                message = "a design names its processes as a tuple: `processes = (A,)` or `processes = (A, B)`"
                raise DesignError(message, *stated)
            for process in cls.processes:
                builder.process(process.name, *stated)
            # A design that states no slab, or no glass, is told so by the engine.
            slab: Slab | None = getattr(cls, "slab", None)
            if slab is not None:
                builder.slab(slab.width, slab.depth, slab.thickness, *stated)
            glass: Glass | None = getattr(cls, "glass", None)
            if glass is not None:
                builder.glass(glass.width, glass.depth, glass.thickness, *stated)
            for layer, spec in cls.layers.layers.items():
                builder.layer(layer.name, spec.height, spec.gds, *stated)
            design = cls.__new__(cls)
            Region.__init__(design)
            design._origin, design._builder = stated, builder
            for name, held in cls._params.items():
                settled = builder.param(name, held.default, held.low, held.high, held.count, *stated)
                design._values[name] = _coerced(held, settled)
            builder.own_conditions(conditions is None)
            _sources.clear()
            with _built(design):
                design.build()
                design._finish()
                if conditions is not None:
                    builder.own_conditions(True)
                    conditions(design, Conditions(design))
            elaborated = builder.finish()
        except DesignError as error:
            # A fault with no line of its own, such as a sum of two dimensions, is given the line
            # of the design or region that it was raised through.
            if error.args[1] is not None:
                raise
            raise DesignError(error.args[0], *_raised_at(error)) from error
        except Exception as error:
            raise DesignError(f"{type(error).__name__}: {error}", *_raised_at(error)) from error
        for source in sorted({cls._source, *_sources}):
            if source and os.path.isfile(source):
                with open(source, "rb") as file:
                    elaborated.add_source(reported(source), file.read())
        return elaborated


def _raised_at(error: BaseException) -> tuple[str | None, int | None]:
    """The line of a design or a region that an exception was last raised through: the innermost
    one."""
    frames = traceback.extract_tb(error.__traceback__)
    written = [frame for frame in frames if authored(frame.filename)]
    for frame in reversed(written or frames):
        return (frame.filename, frame.lineno)
    return (None, None)

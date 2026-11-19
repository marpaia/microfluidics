"""What a region makes: each is known by a path once it is assigned to an attribute."""

from collections.abc import Sequence
from typing import ClassVar

from microfluidics._engine import DesignError
from microfluidics.model import Facing, Layer, PortKind
from microfluidics.origin import Origin
from microfluidics.units import Length, Point


class Named:
    """Something made in a region. It has no name until it is assigned to an attribute of the
    region, which names it after the attribute."""

    kind: ClassVar[str] = "thing"

    def __init__(self, origin: Origin) -> None:
        self._origin = origin
        self._path: str | None = None

    @property
    def path(self) -> str:
        """Its path from the design, as findings and tuned values name it."""
        if self._path is None:
            file, line = self._origin
            raise DesignError(
                f"this {self.kind} is used before it is named: assign it to an attribute first", file, line
            )
        return self._path

    def __repr__(self) -> str:
        return f"<{self.kind} {self._path or 'unnamed'}>"


class Rectangle(Named):
    """A closed rectangle on a layer, with sides along the axes."""

    kind = "rectangle"

    def __init__(self, origin: Origin, layer: Layer, start: Point, end: Point) -> None:
        super().__init__(origin)
        self.layer, self.start, self.end = layer, start, end


class Translates(Named):
    """Rectangles alike in a row: one rectangle and its images under repeated translation."""

    kind = "row of rectangles"

    def __init__(self, origin: Origin, layer: Layer, start: Point, end: Point, by: Point, count: int) -> None:
        super().__init__(origin)
        self.layer, self.start, self.end, self.by, self.count = layer, start, end, by, count


class Vertex(Named):
    """A place flow passes: a port, an access point, or a point inside a region."""

    kind = "vertex"


class Port(Vertex):
    """A segment of a region's boundary through which fluid passes."""

    kind = "port"

    def __init__(
        self, origin: Origin, at: Point, width: Length, layer: Layer, facing: Facing, kind_of: PortKind
    ) -> None:
        super().__init__(origin)
        self.at, self.width, self.layer, self.facing, self.kind_of = at, width, layer, facing, kind_of


class Access(Vertex):
    """Where a punched hole brings tubing to a region."""

    kind = "access point"


class Channel(Named):
    """A channel routed between two ports. It is an edge of the flow graph."""

    kind = "channel"

    def __init__(
        self, origin: Origin, layer: Layer, width: Length, start: Port, end: Port, via: Sequence[Point]
    ) -> None:
        super().__init__(origin)
        self.layer, self.width, self.start, self.end, self.via = layer, width, start, end, tuple(via)


class Stated(Named):
    """A number a region states about itself, such as how many places it has for observing cells."""

    kind = "stated number"

    def __init__(self, origin: Origin, value: float) -> None:
        super().__init__(origin)
        self.value = value


class Species(Named):
    """Something fed at an inlet, whose share of the flow is followed through the chip."""

    kind = "species"

    def __init__(self) -> None:
        super().__init__(("", 0))

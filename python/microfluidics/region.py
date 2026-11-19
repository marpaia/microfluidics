"""Regions: reusable, parametric pieces of a chip.

A region is a class. Its parameters and ports are typed attributes declared in the class body,
and its `build` draws it. Whatever `build` makes is named by assigning it to an attribute of
`self`: `self.inlet = self.port(...)` makes a port named `inlet`, and assigning a list names its
elements `name[0]`, `name[1]` and so on. Assigning a region to an attribute places it.
"""

from __future__ import annotations

import inspect
import re
from collections.abc import Callable, Iterator, Sequence
from contextlib import contextmanager
from typing import Any, ClassVar, Generic, Literal, Self, TypeVar, cast, dataclass_transform, overload

from microfluidics._engine import Builder, DesignError, Quantity
from microfluidics.handles import Access, Channel, Named, Port, Rectangle, Species, Stated, Translates, Vertex
from microfluidics.model import Duct, Facing, Layer, PortKind
from microfluidics.origin import Origin, caller
from microfluidics.units import Length, Point

T = TypeVar("T", Length, int, float)
N = TypeVar("N", bound=Named)


class Param(Generic[T]):
    """A parameter of a region or a design: its default, and the range it is held to.

    On the class it is the parameter itself, by which a design says which one it means:
    `self.tune(self.feed, Manifold.trunk_width)`. On an instance it is the parameter's value.
    """

    def __init__(self, default: T, low: T | None, high: T | None) -> None:
        self.default: T = default
        self.low: T | None = low
        self.high: T | None = high
        self.name = ""
        self.owner: type | None = None

    def __set_name__(self, owner: type, name: str) -> None:
        self.name, self.owner = name, owner

    @property
    def count(self) -> bool:
        """Whether it is a whole number of things."""
        return isinstance(self.default, int) and not isinstance(self.default, bool)

    @overload
    def __get__(self, instance: None, owner: type) -> Param[T]: ...

    @overload
    def __get__(self, instance: object, owner: type) -> T: ...

    def __get__(self, instance: object | None, owner: type) -> Param[T] | T:
        if instance is None:
            return self
        return cast(T, cast(Region, instance)._value(self))

    def __set__(self, instance: object, value: T) -> None:
        file, line = caller()
        raise DesignError(f"`{self.name}` is a parameter: it is given when its region is made, not set", file, line)


def param(*, default: T, low: T | None = None, high: T | None = None) -> Param[T]:
    """A parameter with its default and, when it may be tuned or must be held within limits, the
    range it stays in."""
    return Param(default, low, high)


class _Declared:
    """A port a region declares in its class body, to be made in `build`."""

    def __init__(self, row: bool) -> None:
        self.row = row


def port(*, init: Literal[False] = False) -> Any:
    """Declare a port: `inlet: mf.Port = mf.port()`. `build` makes it by assigning to it."""
    return _Declared(row=False)


def ports(*, init: Literal[False] = False) -> Any:
    """Declare a row of ports: `tap: list[mf.Port] = mf.ports()`."""
    return _Declared(row=True)


_building: list[Region] = []
"""The regions being built, the innermost last."""


@contextmanager
def _built(region: Region) -> Iterator[None]:
    _building.append(region)
    try:
        yield
    finally:
        _building.pop()


def ensure(holds: bool, message: str) -> None:
    """Stop with `message` unless `holds`: for parameters that do not fit together. The fault
    names the instance it was found in."""
    if holds:
        return
    file, line = caller()
    if _building:
        raise _building[-1]._body().fault(message, file, line)
    raise DesignError(message, file, line)


def _coerced(parameter: Param[Any], value: Quantity | float) -> Quantity | float | int:
    """A settled value as the parameter's own type: a count is a whole number."""
    return int(round(cast(float, value))) if parameter.count else value


@dataclass_transform(kw_only_default=True, eq_default=False, field_specifiers=(param, port, ports))
class Region:
    """A reusable, parametric piece of a chip. Subclass it, declare parameters and ports in the
    class body, and draw it in `build`."""

    _params: ClassVar[dict[str, Param[Any]]] = {}
    _port_fields: ClassVar[dict[str, bool]] = {}
    _source: ClassVar[str] = ""
    _line: ClassVar[int] = 0

    def __init_subclass__(cls, **kwargs: Any) -> None:
        super().__init_subclass__(**kwargs)
        params: dict[str, Param[Any]] = {}
        declared: dict[str, bool] = {}
        for base in reversed(cls.__mro__):
            for name, value in vars(base).items():
                if isinstance(value, Param):
                    params[name] = value
                elif isinstance(value, _Declared):
                    declared[name] = value.row
        cls._params, cls._port_fields = params, declared
        try:
            cls._source = inspect.getsourcefile(cls) or ""
            cls._line = inspect.getsourcelines(cls)[1]
        except (OSError, TypeError):
            cls._source, cls._line = "", 0

    def __init__(self, **given: Any) -> None:
        self._origin: Origin = caller()
        unknown = [name for name in given if name not in self._params]
        if unknown:
            known = ", ".join(self._params) or "none"
            message = f"`{type(self).__name__}` has no parameter `{unknown[0]}`; its parameters are {known}"
            raise DesignError(message, *self._origin)
        self._given: dict[str, Any] = dict(given)
        self._values: dict[str, Any] = {}
        self._builder: Builder | None = None
        self._prefix = ""
        self._path: str | None = None
        self._placement: tuple[Any, ...] | None = None
        self._made: list[Named | Region] = []
        if _building:
            _building[-1]._made.append(self)

    # -- what the region is called, and its parameters' values

    @classmethod
    def region_name(cls) -> str:
        """The name findings and benchmarks know the region by: `TrenchLane` is `trench_lane`."""
        return re.sub(r"(?<!^)(?=[A-Z])", "_", cls.__name__).lower()

    def _value(self, parameter: Param[Any]) -> Any:
        if parameter.name in self._values:
            return self._values[parameter.name]
        return self._given.get(parameter.name, parameter.default)

    @property
    def path(self) -> str:
        """The region's path from the design, once it is placed."""
        if self._path is None:
            message = f"this `{type(self).__name__}` is used before it is placed: assign it to an attribute first"
            raise DesignError(message, *self._origin)
        return self._path

    def _body(self) -> Builder:
        if self._builder is None:
            file, line = caller()
            raise DesignError(f"a `{type(self).__name__}` draws only while it is being built", file, line)
        return self._builder

    def _local(self, item: Named | Region, origin: Origin) -> str:
        """The name, within this region, of something it or a region placed in it made."""
        path = item.path
        if not path.startswith(self._prefix):
            raise DesignError(f"`{path}` belongs to another region, and cannot be used here", *origin)
        return path[len(self._prefix) :]

    # -- drawing: each gives something that is named when it is assigned to an attribute

    def build(self) -> None:
        """Draw the region: its rectangles, ports and edges, from its parameters."""

    def rectangle(self, layer: Layer, start: Point, end: Point) -> Rectangle:
        """A rectangle on `layer`, by two opposite corners."""
        return self._make(Rectangle(caller(), layer, start, end))

    def translates(self, layer: Layer, start: Point, end: Point, *, by: Point, count: int) -> Translates:
        """Rectangles alike on `layer`: the first by two opposite corners, each of the rest `by`
        from the last."""
        return self._make(Translates(caller(), layer, start, end, by, count))

    def port(self, *, at: Point, width: Length, layer: Layer, facing: Facing, kind: PortKind = PortKind.BOTH) -> Port:
        """A port at `at`, `width` wide, where a channel leaves the region the way `facing` says."""
        return self._make(Port(caller(), at, width, layer, facing, kind))

    def vertex(self) -> Vertex:
        """A place flow passes inside the region that is not a port."""
        return self._make(Vertex(caller()))

    def access(self, *, at: Point) -> Access:
        """Where a punched hole brings tubing to the region."""
        made = Access(caller())
        self._body().access(at, *made._origin)
        made._path = f"{self._prefix}access"
        return made

    def stated(self, value: float) -> Stated:
        """A number the region states about itself."""
        return self._make(Stated(caller(), value))

    def route(self, layer: Layer, width: Length, start: Port, end: Port, *, via: Sequence[Point] = ()) -> Channel:
        """A channel on `layer`, `width` wide, routed along X and Y from one port to another."""
        return self._make(Channel(caller(), layer, width, start, end, via))

    def edge(self, start: Vertex, end: Vertex, *ducts: Duct) -> None:
        """A way fluid passes through the region, between two of its ports or vertices, as ducts
        in series."""
        origin = caller()
        stretches = [(duct.layer.name, duct.width, duct.length) for duct in ducts]
        self._body().edge(self._local(start, origin), self._local(end, origin), stretches, *origin)

    def height(self, layer: Layer) -> Length:
        """The height the design gives `layer`."""
        found = self._body().height(layer.name)
        if found is None:
            file, line = caller()
            raise DesignError(f"the design has no layer `{layer.name}`", file, line)
        return found

    def _make(self, made: N) -> N:
        self._body()
        self._made.append(made)
        return made

    # -- placing this region in the one that makes it

    def at(self, x: Length, y: Length, *, turned: int = 0) -> Self:
        """With its origin at a point, turned counterclockwise by whole quarter turns."""
        self._placement = ("at", (x, y), turned)
        self._origin = caller()
        return self

    def attach(self, port: Callable[[Self], Port], *, to: Port, offset: Length | None = None) -> Self:
        """With the port `port` picks meeting the port `to`, facing it. With an `offset` it stands
        off that far, and a channel joins the two."""
        self._placement = ("attach", port, to, offset)
        self._origin = caller()
        return self

    # -- naming by assignment

    def __setattr__(self, name: str, value: Any) -> None:
        if not name.startswith("_"):
            # A name every region already uses for something it does cannot be given to a thing.
            if callable(getattr(type(self), name, None)):
                file, line = caller()
                message = f"`{name}` is something every region does, and cannot be the name of a thing: choose another"
                raise DesignError(message, file, line)
            if isinstance(value, (Named, Region)):
                self._name(name, value)
            elif (
                isinstance(value, (list, tuple)) and value and all(isinstance(each, (Named, Region)) for each in value)
            ):
                for index, each in enumerate(value):
                    self._name(f"{name}[{index}]", each)
        object.__setattr__(self, name, value)

    def _name(self, name: str, item: Named | Region) -> None:
        """Give `item` its name and send it to the engine. Something already named keeps its
        name: assigning it again only gives another way to reach it."""
        if item._path is not None:
            return
        builder, origin = self._body(), item._origin
        match item:
            case Region():
                item._place_into(self, name)
                return
            case Rectangle():
                builder.rectangle(name, item.layer.name, item.start, item.end, *origin)
            case Translates():
                builder.translates(name, item.layer.name, item.start, item.end, item.by, item.count, *origin)
            case Port():
                kind, faces = item.kind_of.value, item.facing.value
                builder.port(name, kind, item.at, item.width, item.layer.name, faces, *origin)
            case Vertex():
                builder.vertex(name, *origin)
            case Channel():
                start, end = self._local(item.start, origin), self._local(item.end, origin)
                builder.channel(name, item.layer.name, item.width, start, end, list(item.via), *origin)
            case Stated():
                builder.quantity(name, item.value, *origin)
            case Species():
                item._path = name
                return
            case _:
                self._name_other(name, item)
                return
        item._path = f"{self._prefix}{name}"

    def _name_other(self, name: str, item: Named) -> None:
        raise DesignError(f"a {item.kind} cannot be made here", *item._origin)

    # -- building

    def _param_rows(self) -> list[tuple[str, Any, Any, Any, bool]]:
        return [(name, held.default, held.low, held.high, held.count) for name, held in self._params.items()]

    def _place_into(self, parent: Region, name: str) -> None:
        """Build this region as the instance `name` of `parent`, and place it."""
        origin = self._origin
        if self._placement is None:
            message = f"`{name}` is not placed: give the region `.at(x, y)` or `.attach(port, to=...)`"
            raise DesignError(message, *origin)
        builder = parent._body()
        cls = type(self)
        child = builder.child(
            name,
            cls.region_name(),
            inspect.getdoc(cls) or "",
            cls._source,
            self._param_rows(),
            self._given,
            *origin,
        )
        self._builder, self._prefix = child, child.prefix()
        self._settle(child)
        with _built(self):
            self.build()
        self._finish()
        match self._placement:
            case ("at", point, turned):
                builder.place(child, *origin, at=point, turned=turned)
            case ("attach", pick, to, offset):
                own = pick(self)
                if not isinstance(own, Port) or not own.path.startswith(self._prefix):
                    raise DesignError(f"`{name}` is attached by something that is not one of its ports", *origin)
                attach = own.path[len(self._prefix) :]
                builder.place(child, *origin, attach=attach, to=parent._local(to, origin), offset=offset)
        self._builder = None
        self._path = f"{parent._prefix}{name}"
        _sources.add(type(self)._source)

    def _settle(self, child: Builder) -> None:
        for name, held in self._params.items():
            settled = child.settled(name)
            if settled is not None:
                self._values[name] = _coerced(held, settled)

    def _finish(self) -> None:
        """Hold the region to what it declared: everything made is named, every region placed,
        and every declared port made."""
        for made in self._made:
            if made._path is None:
                kind = f"`{type(made).__name__}`" if isinstance(made, Region) else made.kind
                does = "placed" if isinstance(made, Region) else "named"
                message = f"this {kind} was made and never {does}: assign it to an attribute of `self`"
                raise DesignError(message, *made._origin)
        for field, row in self._port_fields.items():
            held = vars(self).get(field)
            made_it = (
                isinstance(held, list) and all(isinstance(each, Port) for each in held)
                if row
                else isinstance(held, Port)
            )
            if not made_it:
                kind = "row of ports" if row else "port"
                message = f"`{type(self).__name__}` declares the {kind} `{field}` and `build` does not make it"
                raise DesignError(message, type(self)._source, type(self)._line)


_sources: set[str] = set()
"""The files of every region placed since the design being built began."""

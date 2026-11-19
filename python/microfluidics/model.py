"""The small fixed things a design is stated with: layers, directions, ducts, and a chip's slab."""

import enum
from collections.abc import Mapping
from dataclasses import dataclass

from microfluidics.units import Length


class Layer:
    """A layer rectangles are drawn on: one copy of the plane. A design gives each layer it uses a
    height and a GDSII number."""

    def __init__(self, name: str) -> None:
        self.name = name

    def __repr__(self) -> str:
        return f"Layer({self.name!r})"


class Facing(enum.Enum):
    """Which way a port faces: the direction a channel leaves the region by it."""

    UP = "up"
    DOWN = "down"
    LEFT = "left"
    RIGHT = "right"


class PortKind(enum.Enum):
    """Which way fluid passes a port, when that is fixed."""

    INLET = "inlet"
    OUTLET = "outlet"
    BOTH = "both"


@dataclass(frozen=True)
class Duct:
    """One straight stretch of an edge: fluid passes `length` of a duct `width` wide and as high as
    `layer`."""

    layer: Layer
    width: Length
    length: Length


@dataclass(frozen=True)
class LayerSpec:
    """What a design gives a layer: its height, and its number in the mask files."""

    height: Length
    gds: int


@dataclass(frozen=True)
class Slab:
    """The rectangle a chip is cut to, and how thick it is cast."""

    width: Length
    depth: Length
    thickness: Length


@dataclass(frozen=True)
class Glass:
    """The glass a slab is bonded to, at the glass's middle with their sides parallel."""

    width: Length
    depth: Length
    thickness: Length


class Stack:
    """A design's layers, each with its height and number."""

    def __init__(self, layers: Mapping[Layer, LayerSpec]) -> None:
        self.layers = dict(layers)

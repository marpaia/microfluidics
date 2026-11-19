"""Design microfluidic chips as typed Python.

A region is a class with typed parameters and ports that draws itself; a design is a class that
places regions on a chip, says how it is run and what it must do. `microfluidics.regions` holds
the standard regions, `microfluidics.units` the quantities, and `microfluidics.layers` the
layers they are drawn on.
"""

from microfluidics._engine import DesignError, Elaborated, Overrides, Process
from microfluidics.design import (
    Bound,
    Conditions,
    Design,
    Measure,
    Requirement,
    Subject,
    at_least,
    at_most,
    chip_yield,
    drop,
    equal_within,
    flow,
    fraction,
    pressure,
    residence,
    reynolds,
    shear,
    target,
    velocity,
)
from microfluidics.handles import Access, Channel, Port, Rectangle, Species, Stated, Translates, Vertex
from microfluidics.model import Duct, Facing, Glass, Layer, LayerSpec, PortKind, Slab, Stack
from microfluidics.region import Param, Region, ensure, param, port, ports

__all__ = [
    "Access",
    "Bound",
    "Channel",
    "Conditions",
    "Design",
    "DesignError",
    "Duct",
    "Elaborated",
    "Facing",
    "Glass",
    "Layer",
    "LayerSpec",
    "Measure",
    "Overrides",
    "Param",
    "Port",
    "PortKind",
    "Process",
    "Rectangle",
    "Region",
    "Requirement",
    "Slab",
    "Species",
    "Stack",
    "Stated",
    "Subject",
    "Translates",
    "Vertex",
    "at_least",
    "at_most",
    "chip_yield",
    "drop",
    "ensure",
    "equal_within",
    "flow",
    "fraction",
    "param",
    "port",
    "ports",
    "pressure",
    "residence",
    "reynolds",
    "shear",
    "target",
    "velocity",
]

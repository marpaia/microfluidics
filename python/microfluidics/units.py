"""Quantities and their units.

A length and a pressure are different types: `100 * um` is a `Length`, and passing it where a
`Pressure` is wanted is an error to a type checker and when the design is built. Sums and
differences keep their type, a quantity scales by a plain number, and the ratio of two of one
kind is a plain number. Any other product is a `Quantity`, checked when it is used.
"""

from typing import cast

from microfluidics._engine import FlowRate, Length, Pressure, Quantity, Time, Viscosity, unit

__all__ = [
    "FlowRate",
    "Length",
    "Point",
    "Pressure",
    "Quantity",
    "Time",
    "Viscosity",
    "Pa",
    "Pa_s",
    "h",
    "kPa",
    "mPa_s",
    "mbar",
    "minute",
    "mm",
    "nl_min",
    "nm",
    "pct",
    "psi",
    "s",
    "ul_h",
    "ul_min",
    "um",
]

Point = tuple[Length, Length]
"""A point: its distance along X and along Y from the origin of the region it is in."""

nm = cast(Length, unit("nm"))
um = cast(Length, unit("um"))
mm = cast(Length, unit("mm"))

Pa = cast(Pressure, unit("Pa"))
mbar = cast(Pressure, unit("mbar"))
kPa = cast(Pressure, unit("kPa"))
psi = cast(Pressure, unit("psi"))

ul_min = cast(FlowRate, unit("ul_min"))
nl_min = cast(FlowRate, unit("nl_min"))
ul_h = cast(FlowRate, unit("ul_h"))

mPa_s = cast(Viscosity, unit("mPa_s"))
Pa_s = cast(Viscosity, unit("Pa_s"))

s = cast(Time, unit("s"))
minute = cast(Time, unit("min"))
h = cast(Time, unit("h"))

pct: float = 0.01
"""One percent: `5 * pct` is 0.05."""

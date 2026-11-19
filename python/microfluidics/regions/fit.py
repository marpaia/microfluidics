"""How closely a region's parts must fit."""

from microfluidics.units import nm

SLACK = 0.001 * nm
"""How far short of fitting a region's parts may fall, to allow for arithmetic."""

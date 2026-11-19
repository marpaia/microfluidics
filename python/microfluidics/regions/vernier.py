"""A scale that shows how far two layers sit apart."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.units import Length, um


class Vernier(mf.Region):
    """Two rows of bars, one on each layer, at pitches a little apart. When the layers are aligned
    the middle bars are in line; the pair of bars that are in line instead tells how far the flow
    layer sits from the trench layer along the rows, in steps of `resolution`."""

    pitch: mf.Param[Length] = mf.param(default=60 * um, low=10 * um, high=500 * um)
    resolution: mf.Param[Length] = mf.param(default=0.5 * um, low=0.05 * um, high=10 * um)
    trench_bar: mf.Param[Length] = mf.param(default=4 * um, low=1 * um, high=50 * um)
    flow_bar: mf.Param[Length] = mf.param(default=20 * um, low=2 * um, high=200 * um)
    bar_length: mf.Param[Length] = mf.param(default=60 * um, low=10 * um, high=500 * um)

    def build(self) -> None:
        zero = 0 * um
        # Eleven bars a row: five either side of the middle one.
        middle = self.pitch * 5 + self.flow_bar / 2
        fine, coarse = self.pitch - self.resolution, self.pitch
        thin, wide = self.trench_bar / 2, self.flow_bar / 2
        below, above = self.bar_length + 10 * um, self.bar_length * 2 + 10 * um

        first = middle - fine * 5
        self.lower = self.translates(
            TRENCH, (first - thin, zero), (first + thin, self.bar_length), by=(fine, zero), count=11
        )
        first = middle - coarse * 5
        self.upper = self.translates(FLOW, (first - wide, below), (first + wide, above), by=(coarse, zero), count=11)

        self.reach = self.stated((self.resolution * 5).si)

"""A long resistance in a small area."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.regions.fit import SLACK
from microfluidics.units import Length, mm, um


class Serpentine(mf.Region):
    """A channel folded back and forth: a long resistance in a small area. Fluid enters the first
    run at the left and leaves the last at the right."""

    runs: mf.Param[int] = mf.param(default=5, low=1, high=199)
    run_length: mf.Param[Length] = mf.param(default=2 * mm, low=0.1 * mm, high=20 * mm)
    width: mf.Param[Length] = mf.param(default=100 * um, low=10 * um, high=500 * um)
    pitch: mf.Param[Length] = mf.param(default=300 * um, low=30 * um, high=2 * mm)

    inlet: mf.Port = mf.port()
    outlet: mf.Port = mf.port()

    def build(self) -> None:
        mf.ensure(self.runs % 2 == 1, "a serpentine has an odd number of runs, so that it leaves at its far end")
        mf.ensure(
            self.pitch >= self.width * 2 - SLACK, "the runs of a serpentine need at least their own width between them"
        )
        zero, width, pitch, length = 0 * um, self.width, self.pitch, self.run_length
        last = pitch * (self.runs - 1)

        self.run = self.translates(FLOW, (zero, zero), (length, width), by=(zero, pitch), count=self.runs)

        def link(i: int) -> mf.Rectangle:
            # Each run is linked to the next at the right end of even runs and the left end of odd ones.
            left = length - width if i % 2 == 0 else zero
            below = pitch * i
            return self.rectangle(FLOW, (left, below + width), (left + width, below + pitch))

        self.link = [link(i) for i in range(self.runs - 1)]

        self.inlet = self.port(at=(zero, width / 2), width=width, layer=FLOW, facing=mf.Facing.LEFT)
        self.outlet = self.port(at=(length, last + width / 2), width=width, layer=FLOW, facing=mf.Facing.RIGHT)
        self.edge(self.inlet, self.outlet, mf.Duct(FLOW, width, length * self.runs + last))

"""What is measured on a made master to learn the process."""

import microfluidics as mf
from microfluidics.layers import TRENCH
from microfluidics.regions.line_monitor import LineMonitor
from microfluidics.regions.vernier import Vernier
from microfluidics.units import Length, um


class ProcessMonitor(mf.Region):
    """What is measured on a made master to learn the process: lines on each layer stepping through
    widths from the narrowest drawn, and a vernier each way for how far the layers sit apart."""

    trench_first: mf.Param[Length] = mf.param(default=1 * um, low=0.2 * um, high=20 * um)
    trench_step: mf.Param[Length] = mf.param(default=0.2 * um, low=0.05 * um, high=5 * um)
    trench_gap: mf.Param[Length] = mf.param(default=3 * um, low=0.5 * um, high=50 * um)
    flow_first: mf.Param[Length] = mf.param(default=20 * um, low=1 * um, high=500 * um)
    flow_step: mf.Param[Length] = mf.param(default=5 * um, low=0.1 * um, high=100 * um)

    def build(self) -> None:
        zero = 0 * um
        widest = self.trench_first + self.trench_step * 4
        group = (widest + self.trench_gap) * 3 + 10 * um

        def fine(i: int) -> mf.Rectangle:
            # Line `i` is the `i % 3`th of the group of width `i // 3`.
            of_width, in_group = i // 3, i % 3
            width = self.trench_first + self.trench_step * of_width
            left = group * of_width + (width + self.trench_gap) * in_group
            return self.rectangle(TRENCH, (left, zero), (left + width, 200 * um))

        self.fine = [fine(i) for i in range(15)]

        self.coarse = LineMonitor(first=self.flow_first, step=self.flow_step).at(zero, 300 * um)
        self.along = Vernier().at(zero, 700 * um)
        self.up = Vernier().at(900 * um, 700 * um, turned=1)

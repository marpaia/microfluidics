"""Lines whose printed widths are measured on the master."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.units import Length, mm, um


class LineMonitor(mf.Region):
    """Lines on the flow layer stepping through five widths, three of each: measured on the master,
    they give the width a mask prints at against the width it was drawn."""

    first: mf.Param[Length] = mf.param(default=20 * um, low=1 * um, high=500 * um)
    step: mf.Param[Length] = mf.param(default=5 * um, low=0.1 * um, high=100 * um)
    gap: mf.Param[Length] = mf.param(default=40 * um, low=1 * um, high=500 * um)
    length: mf.Param[Length] = mf.param(default=300 * um, low=20 * um, high=5 * mm)

    def build(self) -> None:
        zero = 0 * um
        widest = self.first + self.step * 4
        group = (widest + self.gap) * 3

        def line(i: int) -> mf.Rectangle:
            # Line `i` is the `i % 3`th of the group of width `i // 3`.
            of_width, in_group = i // 3, i % 3
            width = self.first + self.step * of_width
            left = group * of_width + (width + self.gap) * in_group
            return self.rectangle(FLOW, (left, zero), (left + width, self.length))

        self.line = [line(i) for i in range(15)]
        self.widest = self.stated(widest.si)

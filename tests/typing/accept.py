"""What a type checker must accept, and the types it must find."""

from typing import assert_type

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.processes import COSINC_SU8_PDMS, FLOWJEM_SU8_PDMS
from microfluidics.regions import AccessPad, Manifold, TrenchLane
from microfluidics.units import FlowRate, Length, Pressure, mbar, mm, mPa_s, pct, ul_min, um

assert_type(100 * um, Length)
assert_type(100 * um + 2 * mm, Length)
assert_type((100 * um) / (2 * mm), float)
assert_type((12 * mm - 4 * um) / 2, Length)
assert_type(10 * mbar, Pressure)
assert_type(2 * ul_min, FlowRate)

lane = TrenchLane(trenches_per_side=100, channel_width=40 * um)
assert_type(lane.channel_width, Length)
assert_type(lane.trenches_per_side, int)
assert_type(TrenchLane.channel_width, mf.Param[Length])
assert_type(lane.inlet, mf.Port)
assert_type(Manifold().tap, list[mf.Port])
assert_type(Manifold(taps=4).at(1 * mm, 2 * mm, turned=1), Manifold)
assert_type(AccessPad().attach(lambda pad: pad.opening, to=lane.outlet), AccessPad)

assert_type(mf.flow(lane), mf.Measure[FlowRate])
assert_type(mf.at_most(1 * mbar), mf.Bound[Pressure])
mf.Requirement("Cells are not sheared off", mf.shear(lane), mf.at_most(1 * mbar))
mf.Requirement("Every lane gets the same flow", mf.flow([lane, lane]), mf.equal_within(5 * pct))
mf.Requirement("A tenth is dye", mf.fraction(mf.Species(), lane), mf.target(10 * pct, within=1 * pct, relative=True))


class Sent(mf.Design):
    """A design names every process it is made in."""

    processes = (FLOWJEM_SU8_PDMS, COSINC_SU8_PDMS)


class Spur(mf.Region):
    """A region written in Python."""

    length: mf.Param[Length] = mf.param(default=1 * mm, low=0.1 * mm, high=5 * mm)
    turns: mf.Param[int] = mf.param(default=3, low=1, high=9)

    inlet: mf.Port = mf.port()

    def build(self) -> None:
        assert_type(self.length, Length)
        assert_type(self.turns, int)
        self.body = self.rectangle(FLOW, (0 * um, 0 * um), (100 * um, self.length))
        self.inlet = self.port(at=(50 * um, 0 * um), width=100 * um, layer=FLOW, facing=mf.Facing.DOWN)


class Chip(mf.Design):
    slab = mf.Slab(10 * mm, 10 * mm, thickness=3 * mm)
    glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
    layers = mf.Stack({FLOW: mf.LayerSpec(height=20 * um, gds=1)})
    pitch: mf.Param[Length] = mf.param(default=1 * mm, low=0.5 * mm, high=2 * mm)

    def build(self) -> None:
        self.inlet = AccessPad().at(5 * mm, 2 * mm)
        self.spur = Spur(length=2 * mm).attach(lambda spur: spur.inlet, to=self.inlet.opening)
        self.operate(viscosity=1 * mPa_s, drive={self.inlet: 10 * mbar})
        self.tune(Chip.pitch)
        self.tune(self.spur, Spur.length)

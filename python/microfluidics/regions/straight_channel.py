"""A straight channel."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.units import Length, mm, um


class StraightChannel(mf.Region):
    """A straight channel of rectangular section."""

    length: mf.Param[Length] = mf.param(default=2 * mm, low=0.1 * mm, high=50 * mm)
    width: mf.Param[Length] = mf.param(default=100 * um, low=20 * um, high=500 * um)

    inlet: mf.Port = mf.port()
    outlet: mf.Port = mf.port()

    def build(self) -> None:
        zero, half = 0 * um, self.width / 2
        self.channel = self.rectangle(FLOW, (-half, zero), (half, self.length))
        self.inlet = self.port(
            at=(zero, zero), width=self.width, layer=FLOW, facing=mf.Facing.DOWN, kind=mf.PortKind.INLET
        )
        self.outlet = self.port(
            at=(zero, self.length), width=self.width, layer=FLOW, facing=mf.Facing.UP, kind=mf.PortKind.OUTLET
        )
        self.edge(self.inlet, self.outlet, mf.Duct(FLOW, self.width, self.length))

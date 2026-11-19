"""A chamber cells grow in as a single layer."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.regions.fit import SLACK
from microfluidics.units import Length, um


class TrapChamber(mf.Region):
    """A feed channel with one shallow chamber opening off its right side, centred along it. Cells
    held in the chamber grow in a single layer."""

    channel_length: mf.Param[Length] = mf.param(default=400 * um, low=100 * um, high=5000 * um)
    channel_width: mf.Param[Length] = mf.param(default=100 * um, low=20 * um, high=500 * um)
    trap_width: mf.Param[Length] = mf.param(default=200 * um, low=20 * um, high=2000 * um)
    """The chamber's extent along the feed channel."""
    trap_length: mf.Param[Length] = mf.param(default=100 * um, low=20 * um, high=500 * um)
    """The chamber's extent away from the feed channel."""

    inlet: mf.Port = mf.port()
    outlet: mf.Port = mf.port()

    def build(self) -> None:
        zero = 0 * um
        first = (self.channel_length - self.trap_width) / 2
        mf.ensure(first >= -SLACK, "the chamber is longer than the feed channel")

        half = self.channel_width / 2
        self.channel = self.rectangle(FLOW, (-half, zero), (half, self.channel_length))
        self.trap = self.rectangle(TRENCH, (half, first), (half + self.trap_length, first + self.trap_width))

        self.inlet = self.port(
            at=(zero, zero), width=self.channel_width, layer=FLOW, facing=mf.Facing.DOWN, kind=mf.PortKind.INLET
        )
        self.outlet = self.port(
            at=(zero, self.channel_length),
            width=self.channel_width,
            layer=FLOW,
            facing=mf.Facing.UP,
            kind=mf.PortKind.OUTLET,
        )
        self.edge(self.inlet, self.outlet, mf.Duct(FLOW, self.channel_width, self.channel_length))

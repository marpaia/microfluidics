"""Where tubing reaches a chip."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.units import Length, um


class AccessPad(mf.Region):
    """A square pad where a punched hole brings tubing to a channel. The channel leaves through the
    middle of the pad's upper edge."""

    pad_size: mf.Param[Length] = mf.param(default=900 * um, low=300 * um, high=3000 * um)
    channel_width: mf.Param[Length] = mf.param(default=100 * um, low=20 * um, high=500 * um)

    opening: mf.Port = mf.port()

    def build(self) -> None:
        zero, size, half = 0 * um, self.pad_size, self.pad_size / 2
        self.pad = self.rectangle(FLOW, (-half, zero), (half, size))
        self.opening = self.port(at=(zero, size), width=self.channel_width, layer=FLOW, facing=mf.Facing.UP)
        hole = self.access(at=(zero, half))
        # From the opening to the punched hole, across half the pad.
        self.edge(self.opening, hole, mf.Duct(FLOW, size, half))

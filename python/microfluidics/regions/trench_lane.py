"""The mother-machine layout."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.regions.fit import SLACK
from microfluidics.units import Length, mm, um


class TrenchLane(mf.Region):
    """A feed channel with dead-end growth trenches off both sides: the mother-machine layout.
    Cells loaded into a trench grow in single file, fed by diffusion from the channel, and their
    offspring are swept away at the trench mouth."""

    channel_length: mf.Param[Length] = mf.param(default=12 * mm, low=0.5 * mm, high=30 * mm)
    channel_width: mf.Param[Length] = mf.param(default=100 * um, low=20 * um, high=500 * um)
    trench_width: mf.Param[Length] = mf.param(default=1.2 * um, low=0.5 * um, high=10 * um)
    trench_length: mf.Param[Length] = mf.param(default=25 * um, low=5 * um, high=100 * um)
    trench_pitch: mf.Param[Length] = mf.param(default=4 * um, low=1.5 * um, high=100 * um)
    trenches_per_side: mf.Param[int] = mf.param(default=200, low=1, high=5000)

    inlet: mf.Port = mf.port()
    outlet: mf.Port = mf.port()

    def build(self) -> None:
        zero, count = 0 * um, self.trenches_per_side
        span = self.trench_pitch * (count - 1) + self.trench_width
        first = (self.channel_length - span) / 2
        mf.ensure(first >= -SLACK, "the trenches do not fit along the feed channel")
        mf.ensure(self.trench_pitch >= self.trench_width - SLACK, "the trench pitch is less than the trench width")

        half, reach = self.channel_width / 2, self.trench_length
        step = (zero, self.trench_pitch)
        self.channel = self.rectangle(FLOW, (-half, zero), (half, self.channel_length))
        self.left = self.translates(
            TRENCH, (-half - reach, first), (-half, first + self.trench_width), by=step, count=count
        )
        self.right = self.translates(
            TRENCH, (half, first), (half + reach, first + self.trench_width), by=step, count=count
        )

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

        # Places where one lineage can be observed.
        self.observation_sites = self.stated(2 * count)

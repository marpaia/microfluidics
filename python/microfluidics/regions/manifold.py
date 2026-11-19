"""One channel feeding many."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.units import Length, mm, um


class Manifold(mf.Region):
    """One channel feeding a row of taps. Fluid enters by a stem at the middle of the trunk and
    leaves by the taps; turned round, it gathers a row of channels into one."""

    taps: mf.Param[int] = mf.param(default=8, low=2, high=64)
    pitch: mf.Param[Length] = mf.param(default=2.6 * mm, low=0.2 * mm, high=10 * mm)
    trunk_width: mf.Param[Length] = mf.param(default=200 * um, low=20 * um, high=2000 * um)
    tap_width: mf.Param[Length] = mf.param(default=100 * um, low=20 * um, high=500 * um)
    tap_length: mf.Param[Length] = mf.param(default=300 * um, low=50 * um, high=5 * mm)
    stem_length: mf.Param[Length] = mf.param(default=500 * um, low=50 * um, high=5 * mm)

    inlet: mf.Port = mf.port()
    """The stem's open end."""
    tap: list[mf.Port] = mf.ports()

    def build(self) -> None:
        mf.ensure(self.taps % 2 == 0, "a manifold has an even number of taps, so that its stem sits between two")
        zero, pitch, taps = 0 * um, self.pitch, self.taps
        half = pitch * (taps - 1) / 2
        thin, top = self.tap_width / 2, self.trunk_width + self.tap_length

        self.trunk = self.rectangle(FLOW, (-half - thin, zero), (half + thin, self.trunk_width))
        self.stem = self.rectangle(FLOW, (-thin, -self.stem_length), (thin, zero))
        self.tap_row = self.translates(
            FLOW, (-half - thin, self.trunk_width), (-half + thin, top), by=(pitch, zero), count=taps
        )

        self.inlet = self.port(at=(zero, -self.stem_length), width=self.tap_width, layer=FLOW, facing=mf.Facing.DOWN)
        self.tap = [
            self.port(at=(-half + pitch * i, top), width=self.tap_width, layer=FLOW, facing=mf.Facing.UP)
            for i in range(taps)
        ]

        # The stem meets the trunk half-way between the two middle taps.
        self.hub = self.vertex()
        self.t = [self.vertex() for _ in range(taps)]
        self.edge(self.inlet, self.hub, mf.Duct(FLOW, self.tap_width, self.stem_length))
        self.edge(self.hub, self.t[taps // 2 - 1], mf.Duct(FLOW, self.trunk_width, pitch / 2))
        self.edge(self.hub, self.t[taps // 2], mf.Duct(FLOW, self.trunk_width, pitch / 2))
        for at in [*range(taps // 2 - 1), *range(taps // 2, taps - 1)]:
            self.edge(self.t[at], self.t[at + 1], mf.Duct(FLOW, self.trunk_width, pitch))
        for meets, tap in zip(self.t, self.tap):
            self.edge(meets, tap, mf.Duct(FLOW, self.tap_width, self.tap_length))

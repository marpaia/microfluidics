"""Several channels meeting in one."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.units import Length, mm, um


class Junction(mf.Region):
    """Several channels meeting in one: inputs enter the lower side of a collecting channel, which
    leaves to the right."""

    inputs: mf.Param[int] = mf.param(default=3, low=2, high=16)
    pitch: mf.Param[Length] = mf.param(default=1 * mm, low=0.1 * mm, high=10 * mm)
    collector_width: mf.Param[Length] = mf.param(default=200 * um, low=20 * um, high=2000 * um)
    input_width: mf.Param[Length] = mf.param(default=100 * um, low=10 * um, high=500 * um)
    stub_length: mf.Param[Length] = mf.param(default=200 * um, low=20 * um, high=5 * mm)

    input: list[mf.Port] = mf.ports()
    outlet: mf.Port = mf.port()

    def build(self) -> None:
        zero, pitch, half = 0 * um, self.pitch, self.input_width / 2
        span = pitch * (self.inputs - 1)
        end = span + pitch / 2

        self.collector = self.rectangle(FLOW, (-half, zero), (end, self.collector_width))
        self.stub = self.translates(FLOW, (-half, -self.stub_length), (half, zero), by=(pitch, zero), count=self.inputs)

        self.input = [
            self.port(at=(pitch * i, -self.stub_length), width=self.input_width, layer=FLOW, facing=mf.Facing.DOWN)
            for i in range(self.inputs)
        ]
        self.outlet = self.port(
            at=(end, self.collector_width / 2), width=self.collector_width, layer=FLOW, facing=mf.Facing.RIGHT
        )

        # Each input meets the collector at a vertex of its own; the collector runs vertex to vertex.
        self.c = [self.vertex() for _ in range(self.inputs)]
        for enters, meets in zip(self.input, self.c):
            self.edge(enters, meets, mf.Duct(FLOW, self.input_width, self.stub_length))
        for before, after in zip(self.c, self.c[1:]):
            self.edge(before, after, mf.Duct(FLOW, self.collector_width, pitch))
        self.edge(self.c[-1], self.outlet, mf.Duct(FLOW, self.collector_width, pitch / 2))

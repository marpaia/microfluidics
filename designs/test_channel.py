"""One straight channel, for measuring a process."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.processes import CONSERVATIVE_SU8_PDMS
from microfluidics.regions import AccessPad, ProcessMonitor, StraightChannel
from microfluidics.units import mbar, mm, mPa_s, um


class TestChannel(mf.Design):
    """One straight channel between two pads, with nothing else: its flow at a known pressure
    gives the layer's real height and how far the flow model is from a made chip."""

    __test__ = False  # A design, not a test: pytest collects classes named `Test*`.

    process = CONSERVATIVE_SU8_PDMS
    slab = mf.Slab(8 * mm, 16 * mm, thickness=4 * mm)
    glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
    layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

    def build(self) -> None:
        self.inlet = AccessPad().at(4 * mm, 2.5 * mm)
        self.duct = StraightChannel(length=8 * mm).attach(lambda duct: duct.inlet, to=self.inlet.opening)
        self.outlet = AccessPad().attach(lambda pad: pad.opening, to=self.duct.outlet)

        self.monitor = ProcessMonitor().at(0.6 * mm, 7 * mm)

        self.operate(viscosity=1 * mPa_s, drive={self.inlet: 10 * mbar, self.outlet: 0 * mbar})
        self.slow = mf.Requirement("Flow is slow enough for the model to hold", mf.reynolds(self.duct), mf.at_most(1.0))

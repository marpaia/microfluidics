"""One straight channel, for measuring a process."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.processes import COSINC_SU8_PDMS, FLOWJEM_SU8_PDMS
from microfluidics.regions import AccessPad, LineMonitor, StraightChannel
from microfluidics.units import mbar, mm, mPa_s, um


class TestChannel(mf.Design):
    """One straight channel between two pads, with nothing else: its flow at a known pressure
    gives the layer's real height and how far the flow model is from a made chip."""

    __test__ = False  # A design, not a test: pytest collects classes named `Test*`.

    processes = (FLOWJEM_SU8_PDMS, COSINC_SU8_PDMS)
    slab = mf.Slab(8 * mm, 16 * mm, thickness=4 * mm)
    glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
    layers = mf.Stack({FLOW: mf.LayerSpec(height=20 * um, gds=1)})

    def build(self) -> None:
        self.inlet = AccessPad().at(4 * mm, 2.5 * mm)
        self.duct = StraightChannel(length=8 * mm).attach(lambda duct: duct.inlet, to=self.inlet.opening)
        self.outlet = AccessPad().attach(lambda pad: pad.opening, to=self.duct.outlet)

        # Measured on the made master to learn what the process did.
        self.monitor = LineMonitor().at(0.6 * mm, 7 * mm)

        self.operate(viscosity=1 * mPa_s, drive={self.inlet: 10 * mbar, self.outlet: 0 * mbar})
        self.slow = mf.Requirement("Flow is slow enough for the model to hold", mf.reynolds(self.duct), mf.at_most(1.0))

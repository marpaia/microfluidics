"""A mother machine: three channels lined with growth trenches."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.processes import COSINC_SU8_PDMS
from microfluidics.regions import AccessPad, AlignmentMark, ProcessMonitor, TrenchLane
from microfluidics.units import Length, mbar, mm, mPa_s, s, ul_min, um


class MotherMachine(mf.Design):
    """A two-layer mother machine for Escherichia coli, laid out as the chip of James and Kempf et
    al. (2025) is in their Figure S1: three separate lanes on one slab, each a straight channel
    between its own inlet and outlet with growth trenches off both sides, on a 24 x 50 mm
    high-precision coverslip.

    Measured from that figure's photograph against the coverslip: the slab, the 4.2 mm between
    lanes, and the 13.2 mm from each inlet hole to its outlet hole. From the paper's text: about
    8,000 trenches in all, 400 of them in a 0.52 mm field of view, which over the two sides of one
    lane is a pitch of 2.6 um; holes punched at 0.75 mm; and medium flowing at 20 uL/min.

    The paper gives no cross-section. The trench's width, length and depth and the channel's width
    and depth are assumed, at values common for mother machines of this organism, and so is how the
    trenches are shared among the lanes."""

    processes = (COSINC_SU8_PDMS,)
    slab = mf.Slab(14.5 * mm, 20 * mm, thickness=4 * mm)
    glass = mf.Glass(24 * mm, 50 * mm, thickness=0.17 * mm)
    layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=25 * um, gds=2)})

    lanes: mf.Param[int] = mf.param(default=3)
    pitch: mf.Param[Length] = mf.param(default=4.2 * mm)

    def build(self) -> None:
        # Each inlet's hole is 13.2 mm from its outlet's: a pad's hole is at its middle.
        self.inlet = [AccessPad().at(3.05 * mm + self.pitch * i, 2.95 * mm) for i in range(self.lanes)]
        # A trench is as long as the region draws it by default: 25 um.
        self.lane = [
            TrenchLane(
                channel_length=12.3 * mm, trenches_per_side=1333, trench_width=1 * um, trench_pitch=2.6 * um
            ).attach(lambda lane: lane.inlet, to=pad.opening)
            for pad in self.inlet
        ]
        self.outlet = [AccessPad().attach(lambda pad: pad.opening, to=lane.outlet) for lane in self.lane]

        self.mark_west = AlignmentMark().at(1.2 * mm, 10 * mm)
        self.mark_east = AlignmentMark().at(13.3 * mm, 10 * mm)
        # Measured on the made master to learn what the process did.
        self.monitor = ProcessMonitor().at(4.3 * mm, 9 * mm)

        # M9 medium at 37 C, each lane held at a flow rate by a pressure controller.
        drive = {pad: 20 * ul_min for pad in self.inlet} | {pad: 0 * mbar for pad in self.outlet}
        self.operate(viscosity=0.7 * mPa_s, drive=drive)
        self.fresh = mf.Requirement(
            "Medium in each channel is replaced within a second", mf.residence(self.lane), mf.at_most(1 * s)
        )

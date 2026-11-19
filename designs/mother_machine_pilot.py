"""A mother machine of eight separate lanes."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.processes import CONSERVATIVE_SU8_PDMS
from microfluidics.regions import AccessPad, AlignmentMark, ProcessMonitor, TrenchLane
from microfluidics.units import Length, Pa, mbar, mm, mPa_s, um


class MotherMachinePilot(mf.Design):
    """A two-layer mother-machine chip: eight independent lanes, each a feed channel with growth
    trenches between two punched pads, and two alignment marks."""

    process = CONSERVATIVE_SU8_PDMS
    slab = mf.Slab(24.2 * mm, 18 * mm, thickness=4 * mm)
    glass = mf.Glass(60 * mm, 24 * mm, thickness=0.17 * mm)
    layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

    lanes: mf.Param[int] = mf.param(default=8)
    pitch: mf.Param[Length] = mf.param(default=2.6 * mm)

    def build(self) -> None:
        self.inlet = [AccessPad().at(3 * mm + self.pitch * i, 2 * mm) for i in range(self.lanes)]
        self.lane = [
            TrenchLane(trenches_per_side=100).attach(lambda lane: lane.inlet, to=pad.opening) for pad in self.inlet
        ]
        self.outlet = [AccessPad().attach(lambda pad: pad.opening, to=lane.outlet) for lane in self.lane]

        self.mark_west = AlignmentMark().at(1.2 * mm, 9 * mm)
        self.mark_east = AlignmentMark().at(23 * mm, 9 * mm)
        # Measured on the made master to learn what the process did.
        self.monitor = ProcessMonitor().at(0.5 * mm, 11 * mm)

        # Example values: each lane has its own inlet and outlet, all driven alike.
        drive = {pad: 10 * mbar for pad in self.inlet} | {pad: 0 * mbar for pad in self.outlet}
        self.operate(viscosity=0.7 * mPa_s, drive=drive)
        self.gentle = mf.Requirement(
            "Cells at the trench mouths are not sheared off", mf.shear(self.lane), mf.at_most(1 * Pa)
        )

"""A mother machine fed from one inlet."""

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.processes import COSINC_SU8_PDMS
from microfluidics.regions import AccessPad, AlignmentMark, Manifold, ProcessMonitor, TrenchLane
from microfluidics.units import Length, Pa, mbar, mm, mPa_s, pct, um


class MotherMachineManifold(mf.Design):
    """A two-layer mother-machine chip fed from one inlet: a manifold spreads the flow over eight
    lanes of growth trenches, and a second gathers it to one outlet."""

    processes = (COSINC_SU8_PDMS,)
    slab = mf.Slab(24.2 * mm, 21 * mm, thickness=4 * mm)
    glass = mf.Glass(60 * mm, 24 * mm, thickness=0.17 * mm)
    layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

    lanes: mf.Param[int] = mf.param(default=8)
    # How far apart the lanes are: closer, the manifolds' trunks are shorter and starve the outer
    # lanes less.
    pitch: mf.Param[Length] = mf.param(default=2.6 * mm, low=0.4 * mm, high=2.6 * mm)
    # How wide each lane's feed channel is: narrower, a lane resists more against the trunks.
    lane_width: mf.Param[Length] = mf.param(default=100 * um, low=40 * um, high=100 * um)

    def build(self) -> None:
        self.inlet = AccessPad().at(12.1 * mm, 2 * mm)
        self.feed = Manifold(taps=self.lanes, pitch=self.pitch).attach(lambda m: m.inlet, to=self.inlet.opening)
        self.lane = [
            TrenchLane(trenches_per_side=100, channel_width=self.lane_width).attach(
                lambda lane: lane.inlet, to=tap, offset=500 * um
            )
            for tap in self.feed.tap
        ]
        self.riser = [self.route(FLOW, 100 * um, tap, lane.inlet) for tap, lane in zip(self.feed.tap, self.lane)]

        # The gathering manifold is the feeding one turned round, so its taps run the other way.
        self.drain = Manifold(taps=self.lanes, pitch=self.pitch).attach(
            lambda m: m.tap[-1], to=self.lane[0].outlet, offset=500 * um
        )
        self.faller = [
            self.route(FLOW, 100 * um, lane.outlet, tap) for lane, tap in zip(self.lane, reversed(self.drain.tap))
        ]
        self.outlet = AccessPad().attach(lambda pad: pad.opening, to=self.drain.inlet)

        self.mark_west = AlignmentMark().at(1.2 * mm, 10.5 * mm)
        self.mark_east = AlignmentMark().at(23 * mm, 10.5 * mm)
        # Measured on the made master to learn what the process did.
        self.monitor = ProcessMonitor().at(0.5 * mm, 13 * mm)

        # Example values: the pressure and the shear the cells stand are to be set for the strain
        # and the medium.
        self.operate(viscosity=0.7 * mPa_s, drive={self.inlet: 10 * mbar, self.outlet: 0 * mbar})
        self.balanced = mf.Requirement("Every lane gets the same flow", mf.flow(self.lane), mf.equal_within(5 * pct))
        self.gentle = mf.Requirement(
            "Cells at the trench mouths are not sheared off", mf.shear(self.lane), mf.at_most(1 * Pa)
        )
        self.tune(MotherMachineManifold.pitch, MotherMachineManifold.lane_width)
        self.tune(self.feed, Manifold.trunk_width)
        self.tune(self.drain, Manifold.trunk_width)

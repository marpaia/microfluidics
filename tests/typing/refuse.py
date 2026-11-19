"""What a type checker must refuse: each line carries the kind of error it must raise."""

import microfluidics as mf
from microfluidics.layers import FLOW
from microfluidics.regions import AccessPad, Manifold, TrenchLane
from microfluidics.units import mbar, mm, ul_min, um

lane = TrenchLane()
feed = Manifold()

TrenchLane(trench_pich=4 * um)  # refused: call-arg
TrenchLane(trench_pitch=10 * mbar)  # refused: arg-type
TrenchLane(trenches_per_side=4 * um)  # refused: arg-type
TrenchLane(inlet=lane.inlet)  # refused: call-arg
feed.tapp  # refused: attr-defined
Manifold().attach(lambda m: m.stemm, to=lane.outlet)  # refused: attr-defined
AccessPad().attach(lambda pad: pad.opening, to=lane)  # refused: arg-type
AccessPad().at(1 * mm, 5)  # refused: arg-type
mf.Requirement("Gentle", mf.shear(lane), mf.at_most(1 * ul_min))  # refused: misc
mf.Duct(FLOW, 100 * um, 10 * mbar)  # refused: arg-type
mf.LayerSpec(height=20, gds=1)  # refused: arg-type
mf.Glass(75 * mm, 25 * mm, thickness=1 * mbar)  # refused: arg-type
lane.port(at=(0 * um, 0 * um), width=1 * um, layer=FLOW, facing="down")  # refused: arg-type
lane.route(FLOW, 100 * um, lane, lane.outlet)  # refused: arg-type
100 * um + 10 * mbar  # refused: operator

"""The conditions every candidate for this benchmark is run and judged under."""

from collections.abc import Sequence
from typing import Protocol

import microfluidics as mf
from microfluidics.regions import AccessPad, TrenchLane
from microfluidics.units import Pa, mbar, mPa_s, pct, ul_min


class Candidate(Protocol):
    """What a design must have for this benchmark to speak of it."""

    inlet: AccessPad
    outlet: AccessPad
    lane: Sequence[TrenchLane]


def conditions(chip: Candidate, run: mf.Conditions) -> None:
    run.operate(viscosity=0.7 * mPa_s, drive={chip.inlet: 10 * mbar, chip.outlet: 0 * mbar})
    run.eight = mf.Requirement("There are eight lanes", mf.flow(chip.lane[7]), mf.at_least(0 * ul_min))
    run.balanced = mf.Requirement("Every lane gets the same flow", mf.flow(chip.lane), mf.equal_within(5 * pct))
    run.gentle = mf.Requirement(
        "Cells at the trench mouths are not sheared off", mf.shear(chip.lane), mf.at_most(1 * Pa)
    )

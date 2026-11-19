"""The conditions every candidate for this benchmark is run and judged under."""

from typing import Protocol

import microfluidics as mf
from microfluidics.regions import AccessPad
from microfluidics.units import mbar, mPa_s, pct


class Candidate(Protocol):
    """What a design must have for this benchmark to speak of it."""

    buffer_in: AccessPad
    reagent_in: AccessPad
    sample_in: AccessPad
    outlet: AccessPad


def conditions(chip: Candidate, run: mf.Conditions) -> None:
    buffer, reagent, sample = mf.Species(), mf.Species(), mf.Species()
    run.buffer, run.reagent, run.sample = buffer, reagent, sample
    run.operate(
        viscosity=1 * mPa_s,
        drive={chip.buffer_in: 5 * mbar, chip.reagent_in: 5 * mbar, chip.sample_in: 5 * mbar, chip.outlet: 0 * mbar},
        feed={
            chip.buffer_in: {buffer: 100 * pct},
            chip.reagent_in: {reagent: 100 * pct},
            chip.sample_in: {sample: 100 * pct},
        },
    )
    near = mf.target
    run.buffer_share = mf.Requirement(
        "Buffer is 89.2 % of the mixture",
        mf.fraction(buffer, chip.outlet),
        near(89.2 * pct, within=1 * pct, relative=True),
    )
    run.reagent_share = mf.Requirement(
        "Reagent is 10.72 % of the mixture",
        mf.fraction(reagent, chip.outlet),
        near(10.72 * pct, within=1 * pct, relative=True),
    )
    run.sample_share = mf.Requirement(
        "Sample is 0.08 % of the mixture",
        mf.fraction(sample, chip.outlet),
        near(0.08 * pct, within=1 * pct, relative=True),
    )

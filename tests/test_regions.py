"""Regions and designs that are at fault: each fault says what is wrong, in the design's own terms,
and where."""

from pathlib import Path
from typing import Any

import pytest

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.regions import AccessPad, Manifold, TrenchLane
from microfluidics.units import Length, mbar, mm, mPa_s, um

HERE = Path(__file__)


def holder(make: Any) -> type[mf.Design]:
    """A 60 mm chip holding the one region `make` gives, at its middle."""

    class Holder(mf.Design):
        slab = mf.Slab(60 * mm, 60 * mm, thickness=3 * mm)
        glass = mf.Glass(80 * mm, 80 * mm, thickness=1 * mm)
        layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

        def build(self) -> None:
            self.it = make().at(30 * mm, 30 * mm)

    return Holder


def fault(build: Any, tuned: str = "") -> tuple[str, str | None, int | None]:
    """The fault a design with `build` as its body gives, built with the values `tuned` gives by
    path: its message, file and line."""

    class Faulty(mf.Design):
        slab = mf.Slab(12 * mm, 12 * mm, thickness=3 * mm)
        glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
        layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})
        pitch: mf.Param[Length] = mf.param(default=2.6 * mm)

    Faulty.build = build  # type: ignore[method-assign]
    with pytest.raises(mf.DesignError) as raised:
        Faulty.elaborate(mf.Overrides.parse("tuned", tuned))
    message, file, line = raised.value.args
    return message, file, line


def test_a_design_states_the_glass_its_slab_is_bonded_to_and_fits_on_it() -> None:
    def pad(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)

    class Bare(mf.Design):
        slab = mf.Slab(12 * mm, 12 * mm, thickness=3 * mm)
        layers = mf.Stack({FLOW: mf.LayerSpec(height=20 * um, gds=1)})
        build = pad

    with pytest.raises(mf.DesignError) as raised:
        Bare.elaborate()
    assert raised.value.args[0] == "a design states the glass its slab is bonded to: its width, depth and thickness"

    class Unsized(mf.Design):
        glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
        layers = mf.Stack({FLOW: mf.LayerSpec(height=20 * um, gds=1)})
        build = pad

    with pytest.raises(mf.DesignError) as raised:
        Unsized.elaborate()
    assert raised.value.args[0] == "a design states its slab: its width, depth and thickness"

    class Overhanging(Bare):
        glass = mf.Glass(10 * mm, 25 * mm, thickness=1 * mm)

    with pytest.raises(mf.DesignError) as raised:
        Overhanging.elaborate()
    assert raised.value.args[0] == "the slab, 12 x 12 mm, does not fit on the glass, 10 x 25 mm"


def line_of(text: str) -> int:
    return next(number for number, line in enumerate(HERE.read_text().splitlines(), 1) if text in line)


def test_a_fault_says_what_is_wrong_and_where() -> None:
    def out_of_range(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        self.lane = TrenchLane(trench_pitch=1 * um).attach(lambda lane: lane.inlet, to=self.pad.opening)

    message, file, line = fault(out_of_range)
    assert message == "`trench_pitch` of `trench_lane` is 1um, outside 1.5um..100um"
    assert (file, line) == (str(HERE), line_of("TrenchLane(trench_pitch=1 * um)"))

    def does_not_fit(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        self.lane = TrenchLane(channel_length=0.5 * mm).attach(lambda lane: lane.inlet, to=self.pad.opening)

    message, file, line = fault(does_not_fit)
    assert message == "the trenches do not fit along the feed channel (in lane)"
    # A region that finds its own parameters do not fit names the instance it is, at its own line.
    assert Path(str(file)).parts[-3:] == ("microfluidics", "regions", "trench_lane.py")
    assert "the trenches do not fit" in Path(str(file)).read_text().splitlines()[(line or 0) - 1]

    def fits(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        self.lane = TrenchLane(channel_length=6 * mm).attach(lambda lane: lane.inlet, to=self.pad.opening)

    # A value given by path is held to the parameter's range, and to its dimension.
    message, _, _ = fault(fits, '"lane.trench_pitch" = "1um"\n')
    assert message == "`trench_pitch` of `trench_lane` is 1um, outside 1.5um..100um"
    message, _, _ = fault(fits, '"pitch" = "5"\n')
    assert message == "the value given for `pitch` is a plain number, but `pitch` is a length"

    def off_the_chip(self: Any) -> None:
        self.pad = AccessPad().at(100 * um, 2 * mm)

    assert fault(off_the_chip)[0] == "`pad.pad` lies outside the chip's outline"

    def named_twice(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        self.pad = AccessPad().at(8 * mm, 2 * mm)

    assert fault(named_twice)[0] == "`pad` is already the name of something here"

    def never_named(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        self.rectangle(FLOW, (0 * um, 0 * um), (1 * mm, 1 * mm))  # made, and left without a name

    message, file, line = fault(never_named)
    assert message == "this rectangle was made and never named: assign it to an attribute of `self`"
    assert (file, line) == (str(HERE), line_of("made, and left without a name"))

    def never_placed(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        AccessPad().at(8 * mm, 2 * mm)  # made, and never placed

    message, file, line = fault(never_placed)
    assert message == "this `AccessPad` was made and never placed: assign it to an attribute of `self`"
    assert line == line_of("made, and never placed")

    def no_range(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        self.tune(type(self).pitch)

    assert fault(no_range)[0] == "`pitch` has no range to be tuned within; give it one"

    def wrong_owner(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        self.tune(self.pad, Manifold.trunk_width)

    assert fault(wrong_owner)[0] == "`trunk_width` is a parameter of `Manifold`, not of `AccessPad`"

    def adds_dimensions(self: Any) -> None:
        self.pad = AccessPad().at(3 * mm, 2 * mm)
        self.operate(viscosity=1 * mPa_s, drive={self.pad: 10 * mbar + 3 * um})  # type: ignore[operator]

    message, file, line = fault(adds_dimensions)
    assert message.endswith("this adds a pressure and a length") and line == line_of("10 * mbar + 3 * um")

    def unknown(self: Any) -> None:
        self.pad = AccessPad(pad_sise=1 * mm).at(3 * mm, 2 * mm)  # type: ignore[call-arg]

    message, _, line = fault(unknown)
    assert message == "`AccessPad` has no parameter `pad_sise`; its parameters are pad_size, channel_width"


def test_a_region_that_declares_a_port_must_make_it() -> None:
    class Forgetful(mf.Region):
        inlet: mf.Port = mf.port()

        def build(self) -> None:
            self.body = self.rectangle(FLOW, (0 * um, 0 * um), (1 * mm, 1 * mm))

    with pytest.raises(mf.DesignError) as raised:
        holder(Forgetful).elaborate()
    assert raised.value.args[0] == "`Forgetful` declares the port `inlet` and `build` does not make it"

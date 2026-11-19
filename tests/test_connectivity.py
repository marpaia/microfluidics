"""What the connectivity check finds when a design's features and its stated connections differ."""

from pathlib import Path
from typing import Any

from conftest import project

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.regions import AccessPad, Serpentine, StraightChannel
from microfluidics.units import mm, um


class Gap(mf.Region):
    """Two rectangles that do not meet, with an edge stated between their ports."""

    inlet: mf.Port = mf.port()
    outlet: mf.Port = mf.port()

    def build(self) -> None:
        zero = 0 * um
        self.lower = self.rectangle(FLOW, (-50 * um, zero), (50 * um, 1 * mm))
        self.upper = self.rectangle(FLOW, (-50 * um, 2 * mm), (50 * um, 3 * mm))
        self.inlet = self.port(at=(zero, zero), width=100 * um, layer=FLOW, facing=mf.Facing.DOWN)
        self.outlet = self.port(at=(zero, 3 * mm), width=100 * um, layer=FLOW, facing=mf.Facing.UP)
        self.edge(self.inlet, self.outlet, mf.Duct(FLOW, 100 * um, 3 * mm))


def found(directory: Path, build: Any) -> list[tuple[str, str]]:
    """What connectivity finds in a 12 x 12 mm chip with `build` as its body: each finding's rule
    and message."""

    class Arranged(mf.Design):
        slab = mf.Slab(12 * mm, 12 * mm, thickness=3 * mm)
        glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
        layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

    Arranged.build = build  # type: ignore[method-assign]
    check = project(directory, Arranged).verify().check("connectivity")
    assert check is not None
    return [(finding.rule, finding.message) for finding in check.findings]


def test_a_port_with_nothing_connected_is_dangling(tmp_path: Path) -> None:
    def leads_nowhere(self: Any) -> None:
        self.a = AccessPad().at(3 * mm, 2 * mm)
        self.line = StraightChannel(length=4 * mm).attach(lambda line: line.inlet, to=self.a.opening)

    hanging = found(tmp_path, leads_nowhere)
    assert [rule for rule, _ in hanging] == ["dangling", "closed"], hanging
    assert hanging[0][1] == "nothing is connected to the port line.outlet"
    assert "reach 1 access points" in hanging[1][1]


def test_an_edge_between_ports_that_do_not_meet_is_open(tmp_path: Path) -> None:
    def gapped(self: Any) -> None:
        self.a = AccessPad().at(3 * mm, 2 * mm)
        self.g = Gap().attach(lambda gap: gap.inlet, to=self.a.opening)
        self.b = AccessPad().attach(lambda pad: pad.opening, to=self.g.outlet)

    opened = found(tmp_path, gapped)
    assert any(
        rule == "open" and "g.inlet to g.outlet" in message and "no features join them" in message
        for rule, message in opened
    ), opened


def test_features_that_touch_without_being_connected_are_a_short(tmp_path: Path) -> None:
    def side_by_side(self: Any) -> None:
        # Two lines 100 um apart centre to centre: their channels share an edge.
        across = [3 * mm + 5 * mm * i for i in range(2)]
        along = [5.45 * mm + 100 * um * i for i in range(2)]
        self.a = [AccessPad(pad_size=300 * um).at(x, 2 * mm) for x in across]
        self.line = [StraightChannel(length=4 * mm).at(x, 5 * mm) for x in along]
        self.way_in = [
            self.route(FLOW, 100 * um, self.a[i].opening, self.line[i].inlet, via=[(across[i], 4 * mm + 400 * um * i)])
            for i in range(2)
        ]
        self.b = [AccessPad(pad_size=300 * um).at(x, 10.5 * mm, turned=2) for x in across]
        self.way_out = [
            self.route(
                FLOW, 100 * um, self.line[i].outlet, self.b[i].opening, via=[(along[i], 9.4 * mm + 300 * um * i)]
            )
            for i in range(2)
        ]

    shorted = found(tmp_path, side_by_side)
    assert any(
        rule == "short" and "are not connected in the design, but their features touch" in message
        for rule, message in shorted
    ), shorted


def test_a_region_lying_on_another_where_nothing_connects_them_is_a_short(tmp_path: Path) -> None:
    def overlaid(self: Any) -> None:
        # A serpentine attached straight to a pad's channel: its first runs lie on the pad itself,
        # so fluid passes from the pad into them without going the way stated.
        self.a = AccessPad().at(3 * mm, 2 * mm)
        self.s = Serpentine(runs=5, run_length=2 * mm, width=20 * um, pitch=100 * um).attach(
            lambda serpentine: serpentine.inlet, to=self.a.opening
        )
        self.b = AccessPad().at(9 * mm, 9 * mm, turned=2)
        self.out = self.route(FLOW, 20 * um, self.s.outlet, self.b.opening)

    shorted = found(tmp_path, overlaid)
    assert any(rule == "short" and "where the design connects nothing" in message for rule, message in shorted), shorted

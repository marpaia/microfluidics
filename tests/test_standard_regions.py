"""The standard regions: each builds the geometry, ports and edges captured for it."""

import json
from pathlib import Path
from typing import Any

import pytest
from conftest import ROOT, difference, golden

import microfluidics as mf
from microfluidics import _engine, regions
from microfluidics.layers import FLOW, TRENCH
from microfluidics.units import mm, um

NAMES = sorted(file.stem for file in (ROOT / "tests" / "golden" / "regions").glob("*.json"))


def standard(name: str) -> type[mf.Region]:
    """The standard region the captures know as `name`: `trench_lane` is `TrenchLane`."""
    held: type[mf.Region] = getattr(regions, "".join(part.capitalize() for part in name.split("_")))
    return held


def given(region: type[mf.Region], arguments: str) -> dict[str, Any]:
    """The values a captured set of arguments, written as `a = 1mm, b = 3`, gives."""
    held: dict[str, Any] = {}
    for pair in filter(None, (pair.strip() for pair in arguments.split(","))):
        name, value = (side.strip() for side in pair.split("="))
        held[name] = int(value) if region._params[name].count else _engine.parse(value)
    return held


def test_there_is_a_capture_for_every_standard_region() -> None:
    assert [standard(name).__name__ for name in NAMES] == sorted(regions.__all__) and len(NAMES) == 11


@pytest.mark.parametrize("name", NAMES)
def test_a_standard_region_builds_what_was_captured(name: str) -> None:
    region = standard(name)
    for captured in golden(f"regions/{name}.json"):
        values = given(region, captured["arguments"])

        class Holder(mf.Design):
            """A 60 mm chip holding the one region, at its middle."""

            slab = mf.Slab(60 * mm, 60 * mm, thickness=3 * mm)
            glass = mf.Glass(80 * mm, 80 * mm, thickness=1 * mm)
            layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

            def build(self) -> None:
                self.it = region(**values).at(30 * mm, 30 * mm)

        built = json.loads(Holder.elaborate().fingerprint())
        assert difference(built, captured["fingerprint"], name) is None, captured["arguments"]


def test_a_standard_region_states_its_parameters_ports_and_what_it_is() -> None:
    manifold = regions.Manifold
    assert (manifold.region_name(), list(manifold._port_fields.items())) == (
        "manifold",
        [("inlet", False), ("tap", True)],
    )
    assert (manifold.__doc__ or "").startswith("One channel feeding a row of taps.")
    assert Path(manifold._source).parts[-3:] == ("microfluidics", "regions", "manifold.py")
    assert list(manifold._params) == ["taps", "pitch", "trunk_width", "tap_width", "tap_length", "stem_length"]
    taps = manifold._params["taps"]
    assert taps.count and (taps.default, taps.low, taps.high) == (8, 2, 64)


def test_what_a_standard_region_draws_is_found_in_its_own_file() -> None:
    class Holder(mf.Design):
        slab = mf.Slab(12 * mm, 12 * mm, thickness=3 * mm)
        glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
        layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

        def build(self) -> None:
            self.monitor = regions.ProcessMonitor().at(1 * mm, 1 * mm)

    origins = Holder.elaborate().origins()
    here = Path(__file__)
    # The design's line placed the monitor; the monitor's own lines made what is in it.
    file, line = origins["monitor"]
    assert Path(file) == here and "regions.ProcessMonitor()" in here.read_text().splitlines()[line - 1]
    for path, source in {"monitor.fine[0]": "process_monitor.py", "monitor.along.lower": "vernier.py"}.items():
        file, line = origins[path]
        assert Path(file).parts[-3:] == ("microfluidics", "regions", source), path
        assert "self." in Path(file).read_text().splitlines()[line - 1], (path, line)

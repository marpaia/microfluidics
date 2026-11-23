"""KLayout's reading of the files this project writes, against this project's own.

KLayout is the layout tool cleanrooms and mask shops check files in. These tests have it read the
GDSII and DXF written here and compare what it finds with what was planned and checked. They need
a Python with the `klayout` module, named by `KLAYOUT_PYTHON`.
"""

import json
import os
import subprocess
from pathlib import Path
from typing import Any

import pytest
from conftest import ROOT, copied

from microfluidics.results import Report

pytestmark = [
    pytest.mark.klayout,
    pytest.mark.skipif("KLAYOUT_PYTHON" not in os.environ, reason="KLAYOUT_PYTHON names no Python with klayout"),
]

LAYERS = {"trench": 1, "flow": 2}
CELLS = ["CHIP", "flow_20000x60000", "trench_28000x1000", "trench_4000x60000"]


PROCESS = "cosinc-su8-pdms"


def measured(directory: Path, tuned: str = "") -> tuple[Report, dict[str, Any]]:
    """The mother machine, with `tuned` replacing values by path, exported into `directory`: its
    report, and what KLayout measures in the files written for the process it names."""
    report = copied(directory, "mother_machine", tuned).export(directory / "out")
    masks = directory / "out" / PROCESS / "masks"
    for id in ["gds-readback", "dxf-readback"]:
        check = report.check(id)
        assert check is not None and check.status == "satisfied", check
    drawings = [f"{number}={masks / f'mask_{number}_{name}.dxf'}" for name, number in LAYERS.items()]
    script = [os.environ["KLAYOUT_PYTHON"], str(ROOT / "scripts" / "klayout_check.py"), str(masks / "chip.gds")]
    ran = subprocess.run(
        [*script, *drawings, "--width", "1.0", "--space", "1.0"], capture_output=True, text=True, timeout=300
    )
    assert ran.returncode == 0, ran.stderr
    found: dict[str, Any] = json.loads(ran.stdout)
    return report, found


def broken(report: Report, rule: str) -> float:
    """How many times this project's own check finds the rule `rule` broken."""
    rules = report.check("rules")
    assert rules is not None
    listed = float(sum(1 for finding in rules.findings if finding.rule == rule))
    return rules.metrics.get(f"{rule}.violations", listed)


def test_klayout_reads_the_masks_as_they_were_planned(tmp_path: Path) -> None:
    report, found = measured(tmp_path)
    assert found["database_unit_um"] == 0.001
    assert found["cells"] == CELLS
    written, drawn = report.check("gds-readback"), report.check("dxf-readback")
    assert written is not None and drawn is not None
    for name, number in LAYERS.items():
        layer = found["layers"][str(number)]
        assert layer["outlines"] == written.metrics[f"{name}.outlines"], name
        assert abs(layer["area_um2"] / (drawn.metrics[f"{name}.area_mm2"] * 1e6) - 1.0) < 1e-9, (name, layer)
        # The flat DXF drawing and the hierarchical GDSII layer are the same shapes.
        assert layer["drawing_differs_um2"] == 0.0, name
        # The drawing holds each region once: as many outlines as were planned, none overlapping.
        assert layer["drawing_outlines"] == drawn.metrics[f"{name}.outlines"], name
        assert layer["drawing_overlap_um2"] == 0.0, name
        # Nothing narrower or closer than 1 um, as this project's own check found.
        assert (layer["narrow"], layer["close"]) == (0, 0), name
    # Each channel and the two pads it joins are one outline; the features they touch in the file
    # that holds them one by one are more.
    flow = found["layers"]["2"]
    assert flow["drawing_outlines"] < flow["outlines"]


def test_klayout_finds_the_violations_our_check_finds(tmp_path: Path) -> None:
    # One lane's trenches packed 1.8 um apart: walls of 0.8 um, under the 1 um the process needs.
    (tmp_path / "crowded").mkdir()
    report, found = measured(tmp_path / "crowded", '"lane[1].trench_pitch" = "1.8um"\n')
    assert broken(report, "feature.space") == 2664.0
    assert found["layers"]["1"]["close"] == 2664
    assert (found["layers"]["1"]["narrow"], found["layers"]["2"]["close"]) == (0, 0)

    # One lane's trenches 0.8 um wide: under the 1 um the process makes.
    (tmp_path / "thin").mkdir()
    report, found = measured(tmp_path / "thin", '"lane[1].trench_width" = "0.8um"\n')
    assert broken(report, "feature.width") == 2666.0
    assert found["layers"]["1"]["narrow"] == 2666
    assert found["layers"]["1"]["close"] == 0

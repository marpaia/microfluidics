"""What the engine says of designs: rules and waivers, scale, how masks print, wafers, verification,
closure, yield, benchmarks and the scene."""

import json
import shutil
from pathlib import Path
from typing import ClassVar

import pytest
from conftest import DESIGNS, FIXTURES, ROOT, copied, design, difference, golden, project, source

import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.project import Project, benchmark, export_wafer, score
from microfluidics.regions import TrenchLane
from microfluidics.results import Check, Report
from microfluidics.units import mm, um

MANIFOLD = (FIXTURES / "mother_machine_manifold.py").read_text()


def seen(checks: tuple[Check, ...]) -> list[dict[str, object]]:
    """Checks as the captures hold them."""
    return [
        {
            "id": check.id,
            "process": check.process,
            "status": check.status,
            "findings": len(check.findings),
            "metrics": check.metrics,
        }
        for check in checks
    ]


def test_verify_gives_the_captured_results(named: tuple[str, type[mf.Design]], tmp_path: Path) -> None:
    name, _ = named
    captured = golden(f"designs/{name}.json")
    # As written: the design alone, in a directory with nothing tuned beside it.
    shutil.copy(DESIGNS / f"{name}.py", tmp_path / f"{name}.py")
    report = Project(tmp_path / f"{name}.py").verify()
    assert difference(seen(report.checks), captured["untuned"]["checks"], name) is None
    assert report.passes == captured["untuned"]["passes"]
    # As tuned: with the tuned values kept beside it.
    if captured["tuned"] is not None:
        report = Project(DESIGNS / f"{name}.py").verify()
        assert difference(seen(report.checks), captured["tuned"]["checks"], f"{name} tuned") is None
        assert "tuned values" in report.inputs and f"designs/{name}.py" in report.inputs


@pytest.mark.parametrize("name", ["mother_machine_manifold", "goenner_mixer"])
def test_closure_tunes_a_design_to_the_captured_values(name: str, tmp_path: Path) -> None:
    captured = golden(f"closure/{name}.json")
    written = source(name).read_text()
    (tmp_path / f"{name}.py").write_text(written)
    closure = Project(tmp_path / f"{name}.py").close()
    assert closure.converged == captured["converged"]
    assert closure.tuned == captured["tuned_values"]
    # What was tuned is kept beside the design, and the design's source is as it was.
    assert (tmp_path / f"{name}.tuned.toml").read_text() == captured["tuned_values"]
    assert (tmp_path / f"{name}.py").read_text() == written


@pytest.mark.parametrize("name", ["metering_mixer"])
def test_a_benchmark_scores_its_candidate_as_captured(name: str) -> None:
    captured = golden(f"benchmarks/{name}.json")
    held = benchmark(ROOT / "benchmarks" / name)
    assert held.process == "flowjem-su8-pdms" and "access_pad" in held.regions
    scored = score(ROOT / "benchmarks" / name, DESIGNS / f"{captured['candidate']}.py")
    assert (scored.requirements_met, scored.requirements) == (captured["met"], captured["of"])
    assert scored.report.passes == captured["passes"]
    assert difference(seen(scored.report.checks), captured["checks"], name) is None


def test_a_benchmark_judges_a_candidate_by_its_own_conditions(tmp_path: Path) -> None:
    # A candidate that asks nothing of itself is judged by the benchmark's requirements all the
    # same: untuned, the mixer misses the shares the benchmark sets.
    loose = (DESIGNS / "goenner_mixer.py").read_text().replace("within=1 * pct", "within=1000 * pct")
    (tmp_path / "loose.py").write_text(loose)
    assert Project(tmp_path / "loose.py").verify().passes
    scored = score(ROOT / "benchmarks" / "metering_mixer", tmp_path / "loose.py")
    assert scored.requirements == 3 and scored.requirements_met < 3 and not scored.report.passes
    # A candidate without what the benchmark speaks of is told what it lacks.
    with pytest.raises(mf.DesignError) as fault:
        score(ROOT / "benchmarks" / "metering_mixer", DESIGNS / "test_channel.py")
    assert "buffer_in" in fault.value.args[0] and str(fault.value.args[1]).endswith("conditions.py")


def test_an_export_writes_each_process_its_masks_its_report_and_its_sheet(tmp_path: Path) -> None:
    shutil.copy(DESIGNS / "test_channel.py", tmp_path / "test_channel.py")
    report: Report = Project(tmp_path / "test_channel.py").export(tmp_path / "out")
    assert report.passes and report.processes == ("flowjem-su8-pdms", "cosinc-su8-pdms")
    assert all(report.check("dxf-readback", process) is not None for process in report.processes)
    for file in [
        "design.json",
        "report.json",
        "flowjem-su8-pdms/REPORT.md",
        "flowjem-su8-pdms/ORDER.md",
        "flowjem-su8-pdms/masks/mask_1_flow.dxf",
        "flowjem-su8-pdms/masks/README.md",
        "cosinc-su8-pdms/REPORT.md",
        "cosinc-su8-pdms/TRAVELER.md",
        "cosinc-su8-pdms/masks/chip.gds",
    ]:
        assert (tmp_path / "out" / file).is_file(), file
        assert file in report.files, file
    # Each process's report holds the checks that bear on it: the shared ones, and its own.
    flowjem = (tmp_path / "out" / "flowjem-su8-pdms" / "REPORT.md").read_text()
    assert "| `rules` at `flowjem-su8-pdms` | satisfied | 0 |" in flowjem and "cosinc" not in flowjem
    assert "| `graph` | satisfied | 0 |" in flowjem
    # The order names the drawing it sends by its hash, and what FlowJEM has not confirmed.
    order = (tmp_path / "out" / "flowjem-su8-pdms" / "ORDER.md").read_text()
    assert order.startswith("# Order: test_channel at FlowJEM\n")
    assert "| `masks/mask_1_flow.dxf` | flow | `" in order and "chip.gds" not in order
    assert "7 of 10 limits are assumed, not published by FlowJEM: layer.overlap" in order
    assert "2 holes, each made with a 0.75 mm punch" in order and "| `feature.width` | 20.000 | published |" in order
    # The traveler sets out each layer's steps, with room to write what was measured.
    traveler = (tmp_path / "out" / "cosinc-su8-pdms" / "TRAVELER.md").read_text()
    assert traveler.startswith("# Traveler: test_channel at COSINC\n")
    assert (
        "### 1. `flow`, 20.00 um" in traveler
        and "| Coat | Spin the resist and speed that give this layer's height." in traveler
    )
    assert "| Height of `flow` | 20.00 | |" in traveler and "| Width of `monitor.line[0]` | 20.000 | |" in traveler


def test_each_requirement_is_followed_across_the_corners_of_the_process(tmp_path: Path) -> None:
    # The mixer's channels are of one width, so its shares hold wherever fabrication carries them:
    # across the corners each stays within what is allowed, about the value as drawn.
    project = Project(DESIGNS / "goenner_mixer.py")
    report = project.verify()
    assert report.passes and report.problems == 0
    held = {each["name"]: each for each in project.scene()["requirements"]}
    assert set(held) == {"buffer_share", "reagent_share", "sample_share"}
    for requirement in held.values():
        across, bound = requirement["across"], requirement["bound"]
        drawn = next(iter(requirement["values"].values()))
        assert requirement["status"] == "satisfied" and across["holds"] is True
        assert bound["low"] <= across["low"] <= drawn <= across["high"] <= bound["high"]
        assert across["reached"] in (across["low"], across["high"])

    # Each process has its own corners: the mixer's shares hold at FlowJEM's and at COSINC's.
    assert [check.process for check in report.checks if check.id == "corners"] == list(report.processes)
    assert len(report.processes) == 2

    # The mother machine's requirement holds at every corner: the check carries the range it
    # covers, in the requirement's own unit.
    corners = Project(DESIGNS / "mother_machine.py").verify().check("corners")
    assert corners is not None and not corners.findings
    assert corners.metrics["fresh.low"] < corners.metrics["fresh.high"] <= 1.0

    # The manifold as written is unbalanced at every corner, and its report says what is reached.
    shutil.copy(FIXTURES / "mother_machine_manifold.py", tmp_path / "manifold.py")
    project = Project(tmp_path / "manifold.py")
    report = project.verify()
    corners = report.check("corners")
    assert corners is not None and not report.passes
    said = next(finding.message for finding in corners.findings if finding.rule == "balanced")
    assert said.startswith("`balanced` reaches ") and ", and must be within 5% of their mean, " in said
    balanced = next(each for each in project.scene()["requirements"] if each["name"] == "balanced")
    assert balanced["across"]["holds"] is False and balanced["across"]["corner"] in said
    assert report.problems == sum(len(check.findings) for check in report.checks)


def test_a_scene_holds_what_a_viewer_draws_with_python_lines_for_what_the_design_made(tmp_path: Path) -> None:
    project = Project(FIXTURES / "mother_machine_manifold.py")
    scene = project.scene()
    assert (scene["version"], scene["design"], scene["passes"]) == (5, "mother_machine_manifold", True)
    assert scene["problems"] == 0
    assert (scene["glass_um"], scene["punch_um"]) == ([60000.0, 24000.0, 170.0], 750.0)
    assert len(scene["translates"]) == 22 and len(scene["flows"]) == 60 and len(scene["tunables"]) == 4
    file, line = scene["origins"]["feed"]["file"], scene["origins"]["feed"]["line"]
    assert file.endswith("designs/mother_machine_manifold.py")
    assert "self.feed = Manifold" in Path(file).read_text().splitlines()[line - 1]
    # Every instance by its path, with its region, its box, its parameters and its line.
    feed = next(held for held in scene["instances"] if held["path"] == "feed")
    assert feed["region"] == "manifold" and feed["origin"]["line"] == line
    assert ["taps", "8"] in feed["parameters"] and feed["rect"][2] > feed["rect"][0]
    assert sum(1 for held in scene["instances"] if held["region"] == "trench_lane") == 8
    # What is applied where tubing reaches the chip, at the access point of the pad it names.
    applied = {held["instance"]: held["pressure_mbar"] for held in scene["drives"]}
    assert applied == {"inlet": 10.0, "outlet": 0.0}
    assert all(held["at"] in scene["access"] for held in scene["drives"])
    # A requirement with what it allows, in the unit of its values, and how it fares at the corners.
    gentle = next(held for held in scene["requirements"] if held["name"] == "gentle")
    assert (gentle["unit"], gentle["bound"]) == ("Pa", {"kind": "at_most", "low": None, "high": 1.0, "target": None})
    balanced = next(held for held in scene["requirements"] if held["name"] == "balanced")
    assert balanced["bound"]["kind"] == "equal_within"
    assert balanced["bound"]["low"] < min(balanced["values"].values()) and balanced["across"]["holds"]
    # Every check has a name a person reads.
    titles = {held["id"]: held["title"] for held in scene["checks"]}
    assert titles["corners"] == "Fabrication variation" and all(titles.values())
    # And says which question it answers; what the tool checks of its own work is apart.
    groups = {held["id"]: held["group"] for held in scene["checks"]}
    assert (groups["rules"], groups["graph"], groups["corners"]) == ("fabrication", "function", "variation")
    assert (groups["deterministic"], groups["gds-readback"]) == ("integrity", "integrity")
    # A check made against a process names it; one that holds whatever makes the chip names none.
    against = {held["id"]: held["process"] for held in scene["checks"]}
    assert (against["rules"], against["corners"], against["graph"]) == ("cosinc-su8-pdms", "cosinc-su8-pdms", None)
    # With trial values over the tuned ones, nothing is written and the result changes.
    before = (FIXTURES / "mother_machine_manifold.tuned.toml").read_text()
    trial = project.scene(trial='"pitch" = "2600um"\n')
    assert trial["passes"] is False and trial["problems"] > 0 and trial["tunables"][0]["value"] == 2600.0
    assert any(finding["check"] == "requirement/balanced" for finding in trial["findings"])
    assert (FIXTURES / "mother_machine_manifold.tuned.toml").read_text() == before


def held(report: Report, id: str) -> Check:
    check = report.check(id)
    assert check is not None, id
    return check


def test_a_chip_that_breaks_a_rule_names_the_rule_and_the_feature_and_a_waiver_is_recorded(tmp_path: Path) -> None:
    # One lane's trenches packed 1.8 um apart leave 0.8 um walls: under the 1 um the process needs.
    tuned = '"lane[1].trench_pitch" = "1.8um"\n'
    rules = held(copied(tmp_path, "mother_machine", tuned).verify(), "rules")
    assert rules.status == "violated" and rules.process == "cosinc-su8-pdms"
    spacing = next(finding for finding in rules.findings if finding.rule == "feature.space")
    assert spacing.subject.startswith("lane[1].") and "0.800 um apart" in spacing.message, spacing
    # 2,664 pairs of neighbours break the rule; twenty are listed and the rest counted.
    assert rules.metrics["feature.space.violations"] == 2664.0
    # The check says which of its limits the fab has not published.
    assert rules.note is not None and rules.note.startswith("10 of 10 limits are assumed, not published by COSINC")

    waiver = (
        f'[[waivers]]\nrule = "feature.space"\nsubject = "{spacing.subject}"\n'
        'reason = "Printed at this pitch before"\nby = "a researcher"\n'
    )
    waived = held(copied(tmp_path, "mother_machine", tuned, waiver).verify(), "rules")
    same = next(finding for finding in waived.findings if finding.subject == spacing.subject)
    assert same.waived == "Printed at this pitch before (a researcher)"
    assert waived.status == "violated"
    # A waiver that names a process accepts the finding there, and nowhere else.
    for process, accepted in [("cosinc-su8-pdms", True), ("flowjem-su8-pdms", False)]:
        elsewhere = waiver.replace("reason =", f'process = "{process}"\nreason =')
        found = held(copied(tmp_path, "mother_machine", tuned, elsewhere).verify(), "rules")
        same = next(finding for finding in found.findings if finding.subject == spacing.subject)
        assert (same.waived is not None) is accepted, process


class Still(mf.Design):
    """One lane, and nothing said of how it is run."""

    slab = mf.Slab(8 * mm, 16 * mm, thickness=4 * mm)
    glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
    layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

    def build(self) -> None:
        self.lane = TrenchLane().at(4 * mm, 2 * mm)


def test_the_mother_machines_trenches_print_and_a_grating_drawn_too_fine_does_not(tmp_path: Path) -> None:
    machine = Project(DESIGNS / "mother_machine.py")
    printed = held(machine.verify(), "print")
    assert printed.status == "satisfied", printed
    # 7,998 trenches and the monitor's fifteen lines, the narrowest of them 1 um.
    assert printed.metrics["trench.checked"] == 8013.0
    assert abs(printed.metrics["trench.narrowest_drawn_um"] - 1.0) < 1e-9
    # The process's print model is not fitted to a tool, and the check says so.
    assert printed.note is not None and "uncalibrated" in printed.note

    fine = '"monitor.trench_first" = "0.4um"\n"monitor.trench_step" = "0.1um"\n'
    printed = held(copied(tmp_path, "mother_machine", fine).verify(), "print")
    assert printed.status == "violated"
    assert all(finding.subject.startswith("monitor.fine[") for finding in printed.findings), printed
    assert any("does not print at all" in finding.message for finding in printed.findings)

    # At a threshold of a half a 1 um line prints a hair narrow; a few nanometres restore it.
    bias = {layer["layer"]: layer["bias_um"] for layer in machine.bias()}
    assert {layer["process"] for layer in machine.bias()} == {"cosinc-su8-pdms"}
    assert 0.0 < bias["trench"] < 0.05 and abs(bias["flow"]) < 0.002, bias
    # A process that states no print model is asked for no bias: FlowJEM's is not among them.
    assert {layer["process"] for layer in Project(DESIGNS / "mixer.py").bias()} == {"cosinc-su8-pdms"}


def test_a_wafer_holds_its_chips_named_and_refuses_one_that_does_not_fit(tmp_path: Path) -> None:
    wafer = (FIXTURES / "manifold_wafer.toml").read_text()
    (tmp_path / "manifold_wafer.toml").write_text(wafer)
    (tmp_path / "mother_machine_manifold.py").write_text(MANIFOLD)
    made = export_wafer(tmp_path / "manifold_wafer.toml", tmp_path / "out")
    labels = [chip["label"] for chip in made["chips"]]
    assert (len(labels), labels[0], labels[5]) == (6, "mother_machine_manifold-1", "mother_machine_manifold-6")
    assert made["alignment_marks"] == 2
    # What was written reads back from its files as it was planned.
    read_back = [check for check in made["checks"] if check["id"].endswith("-readback")]
    assert read_back and all(check["status"] == "satisfied" for check in read_back), made["checks"]
    assert set(made["files"]) >= {"chip.gds", "mask_1_trench.dxf", "mask_2_flow.dxf"}
    assert all((tmp_path / "out" / file).is_file() for file in made["files"])

    (tmp_path / "wide_wafer.toml").write_text(wafer.replace("columns = 3", "columns = 4"))
    with pytest.raises(mf.DesignError) as refused:
        export_wafer(tmp_path / "wide_wafer.toml", tmp_path / "wide")
    assert "mother_machine_manifold-1" in refused.value.args[0]

    # A wafer of a design that names two processes must say which it is made in.
    two = 'name = "mixers"\ndiameter_mm = 100.0\nedge_exclusion_mm = 5.0\nstreet_mm = 1.0\n'
    two += '[[chips]]\ndesign = "mixer.py"\ncolumns = 2\nrows = 2\n'
    shutil.copy(DESIGNS / "mixer.py", tmp_path / "mixer.py")
    (tmp_path / "mixers.toml").write_text(two)
    with pytest.raises(mf.DesignError) as unsaid:
        export_wafer(tmp_path / "mixers.toml", tmp_path / "mixers")
    assert "`mixer.py` names 2 processes; the wafer must say which" in unsaid.value.args[0]
    (tmp_path / "mixers.toml").write_text(two.replace("diameter_mm", 'process = "flowjem-su8-pdms"\ndiameter_mm'))
    assert len(export_wafer(tmp_path / "mixers.toml", tmp_path / "mixers")["chips"]) == 4
    (tmp_path / "mixers.toml").write_text(two.replace("diameter_mm", 'process = "absent"\ndiameter_mm'))
    with pytest.raises(mf.DesignError) as absent:
        export_wafer(tmp_path / "mixers.toml", tmp_path / "mixers")
    assert "the wafer is made in `absent`, which `mixer.py` does not name" in absent.value.args[0]


def test_verify_lists_what_it_checked_and_stands_on_every_check(tmp_path: Path) -> None:
    # First what holds whatever makes the chip, then what is checked against each process.
    report = Project(DESIGNS / "mother_machine.py").verify()
    assert [(check.id, check.process) for check in report.checks] == [
        ("deterministic", None),
        ("connectivity", None),
        ("graph", None),
        ("requirement/fresh", None),
        ("rules", "cosinc-su8-pdms"),
        ("mask-rules", "cosinc-su8-pdms"),
        ("gds-readback", "cosinc-su8-pdms"),
        ("print", "cosinc-su8-pdms"),
        ("corners", "cosinc-su8-pdms"),
    ]
    assert report.passes and report.problems == 0
    assert "process cosinc-su8-pdms" in report.inputs and len(report.inputs["elaborated design"]) == 64
    # A design that names two processes is checked against each in turn, FlowJEM's without a
    # print check, since FlowJEM's process states no print model.
    mixer = Project(DESIGNS / "mixer.py").verify()
    assert [check.id for check in mixer.checks if check.process == "flowjem-su8-pdms"] == [
        "rules",
        "mask-rules",
        "gds-readback",
        "corners",
    ]
    assert [check.id for check in mixer.checks if check.process == "cosinc-su8-pdms"] == [
        "rules",
        "mask-rules",
        "gds-readback",
        "print",
        "corners",
    ]
    assert {"process flowjem-su8-pdms", "process cosinc-su8-pdms"} <= set(mixer.inputs)
    # Each source file read is named: the design's, and the standard regions' by their place in the package.
    assert "microfluidics/regions/trench_lane.py" in report.inputs and "designs/mother_machine.py" in report.inputs

    # The manifold design as written is not balanced, and its report says so.
    report = copied(tmp_path, "mother_machine_manifold").export(tmp_path / "out")
    written = (tmp_path / "out" / "cosinc-su8-pdms" / "REPORT.md").read_text()
    assert written.startswith("# Verification report: mother_machine_manifold\n\n**Does not pass: ")
    assert "| `requirement/balanced` | VIOLATED | 6 |" in written
    # Its problems are its findings that are not waived, counted over every check.
    findings = sum(len(check.findings) for check in report.checks)
    assert report.problems == findings and findings >= 6

    # A design that does not say how it is run has no flow to check, and does not pass.
    report = project(tmp_path, Still).verify()
    assert held(report, "graph").status == "not_run"
    # A check that did not run is one problem, whatever it would have found.
    idle = sum(1 for check in report.checks if check.status == "not_run")
    found = sum(len(check.findings) for check in report.checks)
    assert not report.passes and report.problems == idle + found and idle >= 1


def test_closure_names_what_limits_a_requirement_beyond_reach_and_leaves_alone_what_has_nothing_to_tune(
    tmp_path: Path,
) -> None:
    strict = MANIFOLD.replace("mf.equal_within(5 * pct)", "mf.equal_within(0.5 * pct)")
    assert strict != MANIFOLD
    closure = copied(tmp_path, "mother_machine_manifold", text=strict).close()
    assert not closure.converged and closure.limiting is not None
    assert closure.limiting.rule == "balanced"
    # Each tunable is accounted for: at the end of its range, or stopped by a rule it names.
    assert "pitch cannot help: it is at the bottom of its range" in closure.limiting.message, closure.limiting
    assert "(rule `roof.span`)" in closure.limiting.message, closure.limiting

    nothing = copied(tmp_path, "mother_machine").close()
    assert nothing.converged and not nothing.changes and nothing.tuned == ""


def test_yield_is_found_when_a_design_asks_for_one(tmp_path: Path) -> None:
    tuned = (FIXTURES / "mother_machine_manifold.tuned.toml").read_text()
    asks = "        self.balanced = mf.Requirement("
    assert asks in MANIFOLD

    def asking(least: str) -> str:
        robust = f'        self.robust = mf.Requirement("Most chips work", mf.chip_yield(), mf.at_least({least}))\n'
        return MANIFOLD.replace(asks, robust + asks)

    report = copied(tmp_path, "mother_machine_manifold", tuned, text=asking("90 * pct")).verify()
    found = held(report, "yield")
    assert (found.status, found.metrics["chips"]) == ("satisfied", 500.0), found
    assert found.metrics["yield_percent"] >= 90.0 and report.passes
    # None can pass more often than every time.
    report = copied(tmp_path, "mother_machine_manifold", tuned, text=asking("100.5 * pct")).verify()
    assert held(report, "yield").status == "violated" and not report.passes


def test_a_scene_of_a_design_that_does_not_pass_says_where_each_finding_was_made(tmp_path: Path) -> None:
    # The manifold design as written: unbalanced, so its requirement is a finding.
    scene = copied(tmp_path, "mother_machine_manifold").scene()
    assert (scene["design"], scene["passes"]) == ("mother_machine_manifold", False) and scene["problems"] >= 6
    assert all(finding["group"] for finding in scene["findings"])
    assert scene["size_um"] == [24_200.0, 21_000.0]
    assert [layer["name"] for layer in scene["layers"]] == ["trench", "flow"]
    # Each instance with its region and the box about what it made, those within others too.
    lane = next(instance for instance in scene["instances"] if instance["path"] == "lane[3]")
    left, lower, right, upper = lane["rect"]
    assert lane["region"] == "trench_lane" and right - left > 100.0 and upper - lower > 10_000.0
    assert ["trenches_per_side", "100"] in lane["parameters"]
    assert all(instance["rect"] is not None and instance["region"] for instance in scene["instances"])
    # Sixteen rows of trenches stay rows, each of one outline stated once.
    rows = [row for row in scene["translates"] if row["outline"] == "trench_25000x1200"]
    assert (len(rows), len(scene["outlines"]["trench_25000x1200"])) == (16, 1)
    assert len(scene["access"]) == 2 and len(scene["drives"]) == 2
    # What flows: every lane's flow, and the inlet's pressure.
    flow = next(flow for flow in scene["flows"] if flow["element"] == "lane[3]")
    assert flow["flow_ul_min"] > 0.0 and flow["shear_pa"] > 0.0 and flow["reynolds"] < 1.0
    assert abs(scene["pressures_mbar"]["inlet.access"] - 10.0) < 1e-9
    assert [tunable["path"] for tunable in scene["tunables"]] == [
        "pitch",
        "lane_width",
        "feed.trunk_width",
        "drain.trunk_width",
    ]
    first = scene["tunables"][0]
    assert (first["value"], first["low"], first["written"]) == (2600.0, 400.0, "2600um")
    balanced = next(each for each in scene["requirements"] if each["name"] == "balanced")
    assert (balanced["status"], len(balanced["values"]), balanced["unit"]) == ("violated", 8, "ul_min")
    # A finding about a lane is found on the line of the design that placed the lanes.
    finding = next(finding for finding in scene["findings"] if finding["check"] == "requirement/balanced")
    assert finding["paths"][0].startswith("lane[")
    assert finding["origin"]["file"].endswith("mother_machine_manifold.py")
    assert len(json.dumps(scene)) < 200_000


def test_a_width_of_an_odd_number_of_nanometres_is_written_to_masks_as_it_is_checked(tmp_path: Path) -> None:
    # Half of 40.151 um falls between two grid points. Every edge drawn there goes to one of
    # them, so the lane's channel still meets its trenches and the file reads back as planned.
    for pitch in ["400um", "437.3um", "1000um"]:
        tuned = f'"lane_width" = "40.151um"\n"pitch" = "{pitch}"\n'
        report = copied(tmp_path, "mother_machine_manifold", tuned).verify()
        for id in ["rules", "connectivity", "gds-readback"]:
            check = held(report, id)
            assert check.status == "satisfied", (id, pitch, check.findings)


def test_a_value_given_for_a_parameter_the_design_does_not_have_is_a_fault() -> None:
    misspelt = mf.Overrides.parse("tuned", '"feed.trunk_wdth" = "240um"\n')
    with pytest.raises(mf.DesignError) as fault:
        design("mother_machine_manifold").elaborate(misspelt)
    assert "`feed.trunk_wdth`" in fault.value.args[0] and "feed.trunk_width" in fault.value.args[0]


class Fickle(mf.Design):
    """A design that draws a different chip each time it is built."""

    slab = mf.Slab(10 * mm, 10 * mm, thickness=3 * mm)
    glass = mf.Glass(75 * mm, 50 * mm, thickness=1 * mm)
    layers = mf.Stack({FLOW: mf.LayerSpec(height=20 * um, gds=1)})
    builds: ClassVar[int] = 0

    def build(self) -> None:
        self.strip = self.rectangle(FLOW, (1 * mm, 1 * mm), (2 * mm + 1 * um * Fickle.builds, 2 * mm))
        Fickle.builds += 1


def test_a_design_that_does_not_build_the_same_way_twice_does_not_pass(tmp_path: Path) -> None:
    report = project(tmp_path, Fickle).verify()
    found = held(report, "deterministic")
    assert found.status == "violated"
    assert "built twice with the same values is not the same design" in found.findings[0].message
    assert not report.passes


def test_a_design_that_names_a_process_alone_is_told_to_name_a_tuple(tmp_path: Path) -> None:
    # Held to no process, a design would pass every rule without a word.
    written = (DESIGNS / "test_channel.py").read_text()
    for wrong in ["    process = COSINC_SU8_PDMS\n", "    processes = COSINC_SU8_PDMS\n"]:
        alone = written.replace("    processes = (FLOWJEM_SU8_PDMS, COSINC_SU8_PDMS)\n", wrong)
        assert alone != written
        with pytest.raises(mf.DesignError) as fault:
            copied(tmp_path, "test_channel", text=alone).verify()
        assert "names its processes as a tuple" in fault.value.args[0]
    # The same process named twice is a fault too.
    twice = written.replace("(FLOWJEM_SU8_PDMS, COSINC_SU8_PDMS)", "(COSINC_SU8_PDMS, COSINC_SU8_PDMS)")
    with pytest.raises(mf.DesignError) as fault:
        copied(tmp_path, "test_channel", text=twice).verify()
    assert "the process `cosinc-su8-pdms` is named twice" in fault.value.args[0]

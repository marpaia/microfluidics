"""The commands: one table, from which the command line and the tools an agent sees are both made.

Each command takes its arguments by name and gives a result: `ok`, whether everything it checked
passed; `text`, for a person; and what was found, with every check that was run.
"""

from __future__ import annotations

import dataclasses
import inspect
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from importlib import resources
from pathlib import Path
from typing import Any

import microfluidics.regions as standard
from microfluidics import text
from microfluidics._engine import DesignError, Process
from microfluidics.project import Project, benchmark, export_wafer, load, score
from microfluidics.region import Param, Region
from microfluidics.results import Check, Report

Result = dict[str, Any]

MAKING = ("rules", "connectivity", "mask-rules", "gds-readback", "print")
"""The checks that say whether a design can be made, whatever it is for."""


@dataclass(frozen=True)
class Argument:
    """One argument of a command."""

    name: str
    about: str
    required: bool = True
    path: bool = False
    """Whether it names a file or directory."""


@dataclass(frozen=True)
class Command:
    name: str
    summary: str
    arguments: tuple[Argument, ...]
    run: Callable[[Mapping[str, str]], Result]

    def schema(self) -> dict[str, Any]:
        """The JSON Schema of the command's arguments."""
        properties = {held.name: {"type": "string", "description": held.about} for held in self.arguments}
        required = [held.name for held in self.arguments if held.required]
        return {"type": "object", "properties": properties, "required": required, "additionalProperties": False}


def _result(ok: bool, said: str, checks: Sequence[Check] = (), **more: Any) -> Result:
    return {"ok": ok, "text": said, "checks": [dataclasses.asdict(check) for check in checks], **more}


def _tuned(project: Project) -> str:
    """A line saying which tuned values a result was found with, when there are any."""
    held = project.beside()
    count = sum(1 for line in held["tuned"].splitlines() if line.strip())
    return f"with {count} tuned values from {held['tuned_path']}\n" if count else ""


def _regions(design: str | None) -> dict[str, type[Region]]:
    """Every region there is: the standard ones, and those a design's file brings."""
    found: dict[str, type[Region]] = {}
    spaces: list[Mapping[str, Any]] = [vars(standard)]
    if design is not None:
        spaces.append(vars(inspect.getmodule(load(design)) or standard))
    for space in spaces:
        for held in space.values():
            if isinstance(held, type) and issubclass(held, Region) and held is not Region and not _is_design(held):
                found[held.region_name()] = held
    return dict(sorted(found.items()))


def _is_design(held: type) -> bool:
    from microfluidics.design import Design

    return issubclass(held, Design)


def _about(region: type[Region]) -> str:
    return " ".join((inspect.getdoc(region) or "").split("\n\n")[0].split())


def guide(arguments: Mapping[str, str]) -> Result:
    return _result(True, resources.files("microfluidics").joinpath("guide.md").read_text())


def regions(arguments: Mapping[str, str]) -> Result:
    found = _regions(arguments.get("design"))
    lines, listed = [], []
    for name, region in found.items():
        lines.append(f"{region.__name__}({', '.join(region._params)})\n    {_about(region)}")
        listed.append(
            {"name": name, "class": region.__name__, "about": _about(region), "parameters": list(region._params)}
        )
    return _result(True, "\n".join(lines) + "\n", regions=listed)


def _written(value: object) -> str:
    return repr(value)


def describe(arguments: Mapping[str, str]) -> Result:
    found = _regions(arguments.get("design"))
    wanted = arguments["region"]
    region = found.get(wanted) or next((held for held in found.values() if held.__name__ == wanted), None)
    if region is None:
        known = ", ".join(held.__name__ for held in found.values())
        raise DesignError(f"there is no region `{wanted}`; the regions are {known}", None, None)
    lines = [f"{region.__name__}: {_about(region)}", "", "Parameters:"]
    params = []
    for name, held in region._params.items():
        limits = (
            f", from {_written(held.low)} to {_written(held.high)}"
            if held.low is not None and held.high is not None
            else ""
        )
        kind = "a whole number" if held.count else type(held.default).__name__
        lines.append(f"  {name}: {kind} = {_written(held.default)}{limits}")
        params.append(
            {"name": name, "default": _written(held.default), "low": _show(held, "low"), "high": _show(held, "high")}
        )
    lines += ["", "Ports:"] + [f"  {name}{' (a row)' if row else ''}" for name, row in region._port_fields.items()]
    if not region._port_fields:
        lines.append("  none")
    lines += ["", f"Source: {region._source}", "", inspect.getsource(region)]
    return _result(
        True, "\n".join(lines) + "\n", name=region.region_name(), parameters=params, ports=list(region._port_fields)
    )


def _show(held: Param[Any], which: str) -> str | None:
    value = getattr(held, which)
    return None if value is None else _written(value)


def process(arguments: Mapping[str, str]) -> Result:
    names = Process.names()
    name = arguments.get("name")
    if name is None:
        return _result(True, "".join(f"{held}\n" for held in names), processes=names)
    # The file itself says most: every rule with its limit, in the units a person reads.
    said = Process.text(name)
    if said is None and name.endswith(".toml"):
        said = Path(name).read_text()
    if said is None:
        raise DesignError(f"there is no process `{name}`; those built in are {', '.join(names)}", None, None)
    read = Process.parse(name, said)
    return _result(True, said, name=read.name, description=read.description)


def check(arguments: Mapping[str, str]) -> Result:
    project = Project(arguments["design"])
    report = project.verify()
    held = [found for found in report.checks if found.id in MAKING]
    ok = all(found.passes for found in held)
    heading = "can be made as drawn" if ok else "cannot be made as drawn"
    return _result(ok, f"{report.design}: {heading}\n{_tuned(project)}{text.checks(held)}", held, design=report.design)


def solve(arguments: Mapping[str, str]) -> Result:
    project = Project(arguments["design"])
    report = project.verify()
    # A flow solved on a design that breaks a rule or a connection is not that chip's flow, so
    # those checks are shown whenever they do not pass.
    unsound = [found for found in report.checks if found.id in ("rules", "connectivity") and not found.passes]
    flows = [found for found in report.checks if found.id == "graph" or found.id.startswith("requirement/")]
    held = unsound + flows
    ok = all(found.passes for found in held)
    heading = "every requirement holds" if ok else "NOT every requirement holds"
    if unsound:
        heading = "the design is NOT sound as drawn, so its flow is not to be relied on"
    said = f"{report.design}: {heading}\n{_tuned(project)}"
    for found in held:
        said += text.checks([found])
        if found.note and found.id.startswith("requirement/"):
            said += f"      {found.note}\n"
        said += text.metrics(found)
    return _result(ok, said, held, design=report.design)


def close(arguments: Mapping[str, str]) -> Result:
    project = Project(arguments["design"])
    closure = project.close()
    heading = "every requirement holds" if closure.converged else "NOT closed"
    said = f"{project.design.region_name()}: {heading} after {closure.iterations} moves\n"
    said += "".join(f"  {change.path}: {change.start} -> {change.end}\n" for change in closure.changes)
    for name, distance in closure.distances.items():
        said += f"  {name}: at {distance:.3f} of its tolerance; it holds at 1 or less\n"
    if closure.limiting is not None:
        said += f"  {closure.limiting.message}\n"
    said += f"written to {closure.tuned_path}\n"
    return _result(closure.converged, said, closure=dataclasses.asdict(closure))


def _one(project: Project, found: Check) -> Result:
    name = project.design.region_name()
    return _result(found.passes, f"{name}:\n{text.checks([found])}{text.metrics(found)}", [found], design=name)


def corners(arguments: Mapping[str, str]) -> Result:
    project = Project(arguments["design"])
    return _one(project, project.corners())


def _whole(arguments: Mapping[str, str], name: str, otherwise: int) -> int:
    given = arguments.get(name)
    if given is None:
        return otherwise
    if not given.isdigit():
        raise DesignError(f"`{name}` is {given!r}, and must be a whole number", None, None)
    return int(given)


def chip_yield(arguments: Mapping[str, str]) -> Result:
    project = Project(arguments["design"])
    return _one(project, project.chip_yield(_whole(arguments, "chips", 500), _whole(arguments, "seed", 1)))


def bias(arguments: Mapping[str, str]) -> Result:
    project = Project(arguments["design"])
    layers = project.bias()
    name = project.design.region_name()
    said = f"{name}: the bias at which each layer's narrowest feature prints as drawn\n"
    for layer in layers:
        if layer["bias_um"] is None:
            said += f"  {layer['layer']:<12} no bias makes its narrowest feature print\n"
        else:
            said += (
                f"  {layer['layer']:<12} {layer['bias_um']:+.3f} um (the process states {layer['stated_um']:+.3f} um)\n"
            )
    return _result(all(layer["bias_um"] is not None for layer in layers), said, design=name, layers=layers)


def _standing(report: Report) -> str:
    """Whether a design passes verification, and the checks that stand in the way when it does not."""
    if report.passes:
        return "passes"
    count = f"{report.problems} {'problem' if report.problems == 1 else 'problems'}"
    against = ", ".join(check.id for check in report.checks if not check.passes)
    return f"does NOT pass: {count}, in {against}" if against else f"does NOT pass: {count}"


def export(arguments: Mapping[str, str]) -> Result:
    project = Project(arguments["design"])
    report: Report = project.export(arguments["directory"])
    said = f"{report.design}: {_standing(report)}\n{_tuned(project)}{text.checks(report.checks)}"
    said += f"written to {arguments['directory']}: {', '.join(report.files)}\n"
    return _result(
        report.passes, said, report.checks, design=report.design, inputs=report.inputs, files=list(report.files)
    )


def wafer(arguments: Mapping[str, str]) -> Result:
    held = export_wafer(arguments["wafer"], arguments["directory"])
    said = f"{len(held['chips'])} chips on the wafer, and {held['alignment_marks']} crosses to align it by\n"
    said += "".join(f"  {chip['label']:<34} at ({chip['x_mm']:.2f}, {chip['y_mm']:.2f}) mm\n" for chip in held["chips"])
    for design, passes in held["passes"].items():
        said += f"  {design} {'passes' if passes else 'does NOT pass'}\n"
    found = [Check.read(each) for each in held["checks"]]
    said += f"{text.checks(found)}written to {arguments['directory']}: {', '.join(held['files'])}\n"
    ok = all(each.passes for each in found) and all(held["passes"].values())
    return _result(ok, said, found, chips=held["chips"], passes=held["passes"], files=held["files"])


def bench(arguments: Mapping[str, str]) -> Result:
    stated = benchmark(arguments["benchmark"])
    candidate = arguments.get("design")
    if candidate is None:
        allowed = ", ".join(stated.regions)
        said = f"{stated.brief.rstrip()}\n\nRegions a design may place: {allowed}\nProcess: {stated.process}\n"
        return _result(True, said, benchmark=dataclasses.asdict(stated))
    scored = score(arguments["benchmark"], candidate)
    report = scored.report
    heading = _standing(report)
    met = f"{scored.requirements_met} of {scored.requirements} requirements hold"
    said = f"{report.design} on {scored.benchmark}: {heading}; {met}\n"
    if scored.tuned_values:
        said += f"with {scored.tuned_values} tuned values from beside the design\n"
    said += text.checks(report.checks)
    return _result(
        report.passes,
        said,
        report.checks,
        benchmark=scored.benchmark,
        design=report.design,
        requirements_met=scored.requirements_met,
        requirements=scored.requirements,
    )


DESIGN = Argument("design", "The design's `.py` file", path=True)
BESIDE = Argument(
    "design", "A design's `.py` file, whose own regions are listed with the standard ones", required=False, path=True
)

COMMANDS: tuple[Command, ...] = (
    Command("guide", "How designs and regions are written: the API, with examples", (), guide),
    Command("regions", "Every region there is, with its parameters", (BESIDE,), regions),
    Command(
        "describe",
        "One region: its parameters with their ranges, its ports, and where it is drawn",
        (Argument("region", "The region's name"), BESIDE),
        describe,
    ),
    Command(
        "process",
        "The processes there are; with a name, that process's rules, tolerances, masks and print model",
        (Argument("name", "A built-in process's name, or a process's `.toml` file", required=False),),
        process,
    ),
    Command(
        "check", "Whether a design can be made: its rules, connections, masks, and how they print", (DESIGN,), check
    ),
    Command("solve", "A design's flow, and whether each of its requirements holds", (DESIGN,), solve),
    Command(
        "close", "Tune what a design allows until its requirements hold; writes <design>.tuned.toml", (DESIGN,), close
    ),
    Command("corners", "Whether a design's requirements hold at every corner of its process", (DESIGN,), corners),
    Command(
        "yield",
        "The share of chips, made within the process's tolerances, that meet every requirement",
        (
            DESIGN,
            Argument("chips", "How many chips to draw; 500 when not given", required=False),
            Argument("seed", "The seed they are drawn from; 1 when not given", required=False),
        ),
        chip_yield,
    ),
    Command("bias", "The mask bias at which each layer's narrowest feature prints at its drawn width", (DESIGN,), bias),
    Command(
        "export",
        "Verify a design and write its masks, report and elaborated design",
        (DESIGN, Argument("directory", "Where to write", path=True)),
        export,
    ),
    Command(
        "wafer",
        "Step designs across a wafer and write its masks",
        (Argument("wafer", "The wafer's `.toml` file", path=True), Argument("directory", "Where to write", path=True)),
        wafer,
    ),
    Command(
        "bench",
        "A benchmark's brief; with a design, that design scored under the benchmark's conditions",
        (
            Argument("benchmark", "The benchmark's directory", path=True),
            Argument("design", "A candidate design's `.py` file", required=False, path=True),
        ),
        bench,
    ),
)

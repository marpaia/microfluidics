"""The `microfluidics` command and its server, run as a person or an agent runs them."""

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest
from conftest import DESIGNS, ROOT

TEST_CHANNEL = (DESIGNS / "test_channel.py").read_text()
MANIFOLD = (DESIGNS / "mother_machine_manifold.py").read_text()


def run(directory: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    line = [sys.executable, "-m", "microfluidics", *arguments]
    return subprocess.run(line, cwd=directory, capture_output=True, text=True)


def served(directory: Path, requests: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Send `requests` to the server working in `directory`, and return its answers."""
    sent = "".join(json.dumps(request) + "\n" for request in requests)
    line = [sys.executable, "-m", "microfluidics", "mcp"]
    done = subprocess.run(line, cwd=directory, input=sent, capture_output=True, text=True)
    return [json.loads(answer) for answer in done.stdout.splitlines()]


def call(id: int, tool: str, **arguments: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": tool, "arguments": arguments}}


def test_the_command_names_what_it_can_do_and_refuses_what_it_cannot(tmp_path: Path) -> None:
    usage = run(tmp_path)
    assert usage.returncode == 2
    names = ["guide", "regions", "describe", "process", "check", "solve", "close", "corners", "yield", "bias"]
    for name in names + ["export", "wafer", "bench", "mcp"]:
        assert f"\n  {name} " in usage.stderr, name
    assert run(tmp_path, "tapeout").returncode == 2
    missing = run(tmp_path, "export", "a.py")
    assert missing.returncode == 2 and "`export` needs <directory>" in missing.stderr
    described = run(tmp_path, "describe", "Manifold").stdout
    assert described == run(tmp_path, "describe", "manifold").stdout
    assert "taps: a whole number = 8, from 2 to 64" in described and "tap (a row)" in described
    # A region is described with how it is drawn: its source.
    assert "regions/manifold.py" in described and "class Manifold(mf.Region):" in described
    assert "there is no region `spiral`; the regions are AccessPad" in run(tmp_path, "describe", "spiral").stderr
    assert "conservative-su8-pdms" in run(tmp_path, "process").stdout
    assert 'kind = "edge_margin"' in run(tmp_path, "process", "conservative-su8-pdms").stdout
    assert "TrenchLane(channel_length, channel_width" in run(tmp_path, "regions").stdout


def test_a_design_is_checked_exported_and_its_faults_point_at_its_source(tmp_path: Path) -> None:
    (tmp_path / "channel.py").write_text(TEST_CHANNEL)
    checked = run(tmp_path, "check", "channel.py")
    assert checked.returncode == 0 and checked.stdout.startswith("test_channel: can be made as drawn"), checked

    exported = run(tmp_path, "export", "channel.py", "out", "--json")
    result = json.loads(exported.stdout)
    assert exported.returncode == 0 and result["ok"] is True
    ids = [check["id"] for check in result["checks"]]
    expected = [
        "deterministic",
        "rules",
        "connectivity",
        "mask-rules",
        "gds-readback",
        "print",
        "graph",
        "requirement/slow",
        "corners",
    ]
    assert ids == expected + ["dxf-readback"]
    for file in ["REPORT.md", "report.json", "design.json", "masks/chip.gds", "masks/mask_1_trench.dxf"]:
        assert (tmp_path / "out" / file).is_file(), file

    # An inlet too near the chip's edge: violated, and the command says so by its exit.
    (tmp_path / "near.py").write_text(
        TEST_CHANNEL.replace("AccessPad().at(4 * mm, 2.5 * mm)", "AccessPad().at(4 * mm, 1.2 * mm)")
    )
    near = run(tmp_path, "check", "near.py")
    assert near.returncode == 1 and "rules                        VIOLATED" in near.stdout
    assert "the access point in inlet.pad is 1650 um from the chip's edge; the process needs 2000 um" in near.stdout

    # A fault in the design is reported as `file:line: message`, at the line that wrote it.
    wrong = TEST_CHANNEL.replace("AccessPad().at(4 * mm, 2.5 * mm)", "AccessPad(pad_size=10 * um).at(4 * mm, 2.5 * mm)")
    (tmp_path / "broken.py").write_text(wrong)
    broken = run(tmp_path, "check", "broken.py")
    line = wrong.splitlines().index(next(held for held in wrong.splitlines() if "pad_size=10" in held)) + 1
    assert broken.returncode == 2
    assert broken.stderr.strip() == f"broken.py:{line}: `pad_size` of `access_pad` is 10um, outside 300um..3000um"

    # A channel left hanging: its flow is solved, but the design is not sound, and `solve` says so.
    hanging = TEST_CHANNEL.replace(
        "        self.outlet = AccessPad().attach(lambda pad: pad.opening, to=self.duct.outlet)\n",
        "        self.outlet = AccessPad().at(4 * mm, 13.6 * mm, turned=2)\n"
        "        self.spur = StraightChannel(length=1 * mm).attach(lambda spur: spur.inlet, to=self.duct.outlet)\n",
    )
    (tmp_path / "hanging.py").write_text(hanging)
    solved = run(tmp_path, "solve", "hanging.py")
    assert solved.returncode == 1 and "is NOT sound as drawn" in solved.stdout, solved
    assert "connectivity                 VIOLATED" in solved.stdout


@pytest.mark.parametrize(
    ("measure", "bound", "message"),
    [
        ("reynolds", "at_most(1 * mbar)", "the upper bound must be a plain number, not a pressure"),
        ("flow", "at_least(1)", "the lower bound must be a flow rate, not a plain number"),
        ("shear", "target(1 * mbar, within=1 * mm)", "the target tolerance must be a pressure, not a length"),
        (
            "shear",
            "target(1 * mbar, within=1 * mbar, relative=True)",
            "the target tolerance must be a plain number, not a pressure",
        ),
        ("velocity", "at_most(1 * mm)", "the upper bound must be a velocity, not a length"),
        ("flow", "equal_within(1 * mbar)", "the allowed spread must be a plain number, not a pressure"),
    ],
)
def test_solve_reports_bound_dimension_errors_at_the_requirement(
    tmp_path: Path, measure: str, bound: str, message: str
) -> None:
    source = TEST_CHANNEL.replace("mf.reynolds(self.duct), mf.at_most(1.0)", f"mf.{measure}(self.duct), mf.{bound}")
    (tmp_path / "channel.py").write_text(source)
    line = next(n for n, text in enumerate(source.splitlines(), 1) if "self.slow =" in text)
    result = run(tmp_path, "solve", "channel.py")
    assert result.returncode == 2
    assert result.stderr.strip() == f"channel.py:{line}: {message}"


def test_closing_a_design_writes_what_was_tuned_beside_it(tmp_path: Path) -> None:
    (tmp_path / "manifold.py").write_text(MANIFOLD)
    assert run(tmp_path, "solve", "manifold.py").returncode == 1
    closed = run(tmp_path, "close", "manifold.py")
    assert closed.returncode == 0 and "every requirement holds" in closed.stdout, closed
    assert '"pitch" = ' in (tmp_path / "manifold.tuned.toml").read_text()
    assert (tmp_path / "manifold.py").read_text() == MANIFOLD
    assert run(tmp_path, "solve", "manifold.py").returncode == 0
    assert "corners                      satisfied" in run(tmp_path, "corners", "manifold.py").stdout


def test_a_benchmark_gives_its_brief_and_scores_a_design() -> None:
    brief = run(ROOT, "bench", "benchmarks/balanced_lanes").stdout
    assert brief.startswith("# Balanced lanes") and "Regions a design may place: access_pad, manifold" in brief
    scored = run(ROOT, "bench", "benchmarks/balanced_lanes", "designs/mother_machine_manifold.py")
    assert scored.returncode == 0 and "passes; 3 of 3 requirements hold" in scored.stdout, scored


def test_an_agent_writes_a_design_and_checks_it_through_the_server(tmp_path: Path) -> None:
    answers = served(
        tmp_path,
        [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
            call(3, "write_file", path="chips/channel.py", content=TEST_CHANNEL),
            call(4, "check", design="chips/channel.py"),
            call(5, "write_file", path="chips/channel.waivers.toml", content=""),
            call(6, "read_file", path="../outside.py"),
            call(7, "check", design="/etc/hosts"),
            call(8, "list_files", path="chips"),
            call(9, "check", design="chips/absent.py"),
            {"jsonrpc": "2.0", "id": 10, "method": "resources/list"},
        ],
    )
    assert len(answers) == 10
    assert answers[0]["result"]["protocolVersion"] == "2025-03-26"
    assert "You cannot waive a finding" in answers[0]["result"]["instructions"]
    tools = answers[1]["result"]["tools"]
    assert len(tools) == 16
    exports = next(tool for tool in tools if tool["name"] == "export")
    assert exports["inputSchema"]["required"] == ["design", "directory"]
    assert answers[2]["result"]["isError"] is False
    checked = answers[3]["result"]["structuredContent"]
    assert (
        checked["ok"] is True
        and checked["checks"][0]["id"] == "rules"
        and checked["checks"][0]["status"] == "satisfied"
    )
    assert "a waiver is a person's to write" in answers[4]["result"]["content"][0]["text"]
    assert answers[5]["result"]["isError"] is True and "is outside" in answers[5]["result"]["content"][0]["text"]
    assert "is outside" in answers[6]["result"]["content"][0]["text"]
    assert answers[7]["result"]["structuredContent"]["files"] == ["channel.py"]
    assert (
        answers[8]["result"]["isError"] is True
        and "there is no such file" in answers[8]["result"]["content"][0]["text"]
    )
    assert answers[9]["error"]["code"] == -32601

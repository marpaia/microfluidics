"""A session: a design held for a viewer, built again as it changes."""

import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from conftest import FIXTURES

MANIFOLD = (FIXTURES / "mother_machine_manifold.py").read_text()


class Viewer:
    """A session's other end: what a viewer sends, and what it is sent."""

    def __init__(self, design: Path) -> None:
        line = [sys.executable, "-m", "microfluidics", "session", str(design)]
        self.session = subprocess.Popen(
            line, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=design.parent
        )

    def ask(self, kind: str, **held: Any) -> None:
        assert self.session.stdin is not None
        self.session.stdin.write(json.dumps({"type": kind, **held}) + "\n")
        self.session.stdin.flush()

    def until(self, state: str) -> list[dict[str, Any]]:
        """Every message up to the next status of `state`."""
        assert self.session.stdout is not None
        seen = []
        while True:
            message = json.loads(self.session.stdout.readline())
            seen.append(message)
            if message["type"] == "status" and message["state"] == state:
                return seen

    def one(self, kind: str) -> dict[str, Any]:
        assert self.session.stdout is not None
        while True:
            message: dict[str, Any] = json.loads(self.session.stdout.readline())
            if message["type"] == kind:
                return message


def of(messages: list[dict[str, Any]], kind: str) -> dict[str, Any]:
    return next(message for message in messages if message["type"] == kind)


def test_a_session_builds_a_design_follows_its_edits_and_survives_its_faults(tmp_path: Path) -> None:
    design = tmp_path / "manifold.py"
    design.write_text(MANIFOLD)
    viewer = Viewer(design)
    try:
        # As written the design is unbalanced: a scene, and a finding on the line that made the lanes.
        first = viewer.until("ready")
        scene = of(first, "scene")["scene"]
        assert scene["design"] == "mother_machine_manifold" and not scene["passes"]
        found = of(first, "diagnostics")["files"][str(design.resolve())]
        lanes = MANIFOLD.splitlines().index("        self.lane = [") + 1
        balanced = [held for held in found if held["check"] == "requirement/balanced"]
        # The lanes are made by one statement of a few lines; each finding lands within it.
        assert balanced and all(lanes <= held["line"] <= lanes + 5 and held["severity"] == "error" for held in balanced)
        assert balanced[0]["paths"][0].startswith("lane[")

        # A slider: trial values are built with, and nothing is written beside the design.
        viewer.ask("set_overrides", values={"pitch": "400um", "lane_width": "40um"})
        tried = viewer.until("ready")
        assert of(tried, "scene")["trial"] is True and of(tried, "scene")["scene"]["tunables"][0]["value"] == 400.0
        assert not (tmp_path / "manifold.tuned.toml").exists()

        # An edit that breaks the design: the fault and its line, and the session lives on.
        time.sleep(0.05)
        design.write_text(
            MANIFOLD.replace("AccessPad().at(12.1 * mm, 2 * mm)", "AccessPad(pad_size=1 * um).at(12.1 * mm, 2 * mm)")
        )
        failed = viewer.until("failed")[-1]
        assert failed["message"] == "`pad_size` of `access_pad` is 1um, outside 300um..3000um"
        assert (
            failed["file"] == str(design.resolve())
            and "pad_size=1 * um" in design.read_text().splitlines()[failed["line"] - 1]
        )

        # The edit undone: it builds again without being asked.
        time.sleep(0.05)
        design.write_text(MANIFOLD)
        assert of(viewer.until("ready"), "scene")["scene"]["design"] == "mother_machine_manifold"

        # Keeping writes the trial values beside the design, and they are no longer a trial.
        viewer.ask("set_overrides", values={"pitch": "400um"})
        viewer.until("ready")
        viewer.ask("keep")
        kept = of(viewer.until("ready"), "scene")
        assert kept["trial"] is False and kept["scene"]["tunables"][0]["value"] == 400.0
        assert (tmp_path / "manifold.tuned.toml").read_text() == '"pitch" = "400um"\n'

        # A command the session does not run on a design is refused, and the session lives on.
        viewer.ask("run", command="export")
        refused = viewer.one("result")
        assert refused["ok"] is False and "`export` is not a command a session runs" in refused["text"]

        # Closing writes the tuned values, and the scene that follows passes.
        viewer.ask("run", command="close")
        result = viewer.one("result")
        assert result["ok"] is True and "every requirement holds" in result["text"]
        assert of(viewer.until("ready"), "scene")["scene"]["passes"] is True
        assert (tmp_path / "manifold.tuned.toml").exists()
        viewer.ask("shutdown")
        assert viewer.session.wait(timeout=10) == 0
    finally:
        viewer.session.kill()

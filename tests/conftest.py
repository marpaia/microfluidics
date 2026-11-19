"""What the tests share: where things are, and how a design file is loaded."""

import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

import pytest

import microfluidics as mf
from microfluidics.project import Project

ROOT = Path(__file__).resolve().parent.parent
DESIGNS = ROOT / "designs"


def golden(name: str) -> Any:
    """A captured file under `tests/golden`, read as JSON."""
    return json.loads((ROOT / "tests" / "golden" / name).read_text())


def difference(now: Any, then: Any, at: str = "") -> str | None:
    """Where two JSON values first differ, numbers compared to nine figures; `None` when they agree."""
    if isinstance(now, dict) and isinstance(then, dict):
        if now.keys() != then.keys():
            return f"{at}: keys {sorted(now)} now, {sorted(then)} captured"
        return next((found for key in now if (found := difference(now[key], then[key], f"{at}.{key}"))), None)
    if isinstance(now, list) and isinstance(then, list):
        if len(now) != len(then):
            return f"{at}: {len(now)} entries now, {len(then)} captured"
        pairs = enumerate(zip(now, then))
        return next((found for index, (a, b) in pairs if (found := difference(a, b, f"{at}[{index}]"))), None)
    if (
        isinstance(now, bool)
        or isinstance(then, bool)
        or not isinstance(now, (int, float))
        or not isinstance(then, (int, float))
    ):
        return None if now == then else f"{at}: {now!r} now, {then!r} captured"
    near = abs(now - then) <= 1e-9 * max(abs(now), abs(then)) + 1e-9
    return None if near else f"{at}: {now} now, {then} captured"


def design(name: str) -> type[mf.Design]:
    """The design in `designs/<name>.py`."""
    spec = importlib.util.spec_from_file_location(name, DESIGNS / f"{name}.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    found = [held for held in vars(module).values() if isinstance(held, type) and issubclass(held, mf.Design)]
    own = [held for held in found if held.__module__ == name]
    assert len(own) == 1, own
    return own[0]


def project(directory: Path, design: type[mf.Design], tuned: str = "", waivers: str = "") -> Project:
    """A design written in a test, as a project in `directory`, with the tuned values and the
    waivers given kept beside it."""
    held = Project.__new__(Project)
    held.path, held.design = directory / "design.py", design
    if tuned:
        (directory / "design.tuned.toml").write_text(tuned)
    if waivers:
        (directory / "design.waivers.toml").write_text(waivers)
    return held


def copied(directory: Path, name: str, tuned: str = "", waivers: str = "", source: str | None = None) -> Project:
    """The design `designs/<name>.py` as a project in `directory`, with nothing beside it but the
    tuned values and the waivers given. `source` is its text, when that is not as it is written."""
    (directory / f"{name}.py").write_text((DESIGNS / f"{name}.py").read_text() if source is None else source)
    if tuned:
        (directory / f"{name}.tuned.toml").write_text(tuned)
    if waivers:
        (directory / f"{name}.waivers.toml").write_text(waivers)
    return Project(directory / f"{name}.py")


@pytest.fixture(
    scope="session",
    params=["mother_machine_pilot", "mother_machine_manifold", "goenner_mixer", "mixer", "test_channel"],
)
def named(request: pytest.FixtureRequest) -> tuple[str, type[mf.Design]]:
    """Each design, with its name."""
    name: str = request.param
    return name, design(name)

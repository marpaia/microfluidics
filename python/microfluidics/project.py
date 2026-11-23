"""A design as it sits on disk: its file, and what the engine says of it."""

from __future__ import annotations

import importlib.util
import json
import os
import sys
from collections.abc import Callable
from pathlib import Path
from types import ModuleType
from typing import Any

from microfluidics import _engine
from microfluidics._engine import DesignError, Elaborated, Overrides
from microfluidics.design import Conditions, Design
from microfluidics.results import Benchmark, Check, Closure, Report, Score


def _module(path: Path) -> ModuleType:
    """The Python file at `path` as a module, with its directory importable so that a design can
    import regions written beside it."""
    if not path.is_file():
        raise DesignError(f"{path}: there is no such file", None, None)
    directory = str(path.parent.resolve())
    if directory not in sys.path:
        sys.path.insert(0, directory)
    spec = importlib.util.spec_from_file_location(path.stem, path)
    if spec is None or spec.loader is None:
        raise DesignError(f"{path} is not a Python file", None, None)
    module = importlib.util.module_from_spec(spec)
    sys.modules[path.stem] = module
    # A design's directory is left as it was found: no compiled files are written beside it.
    writes, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    try:
        spec.loader.exec_module(module)
    except DesignError:
        raise
    except SyntaxError as error:
        raise DesignError(f"{error.msg}", error.filename, error.lineno) from error
    except Exception as error:
        frames = [frame for frame in _frames(error) if os.path.samefile(frame[0], path)] or _frames(error)
        file, line = frames[-1] if frames else (str(path), None)
        raise DesignError(f"{type(error).__name__}: {error}", file, line) from error
    finally:
        sys.dont_write_bytecode = writes
    return module


def _frames(error: BaseException) -> list[tuple[str, int]]:
    import traceback

    return [
        (frame.filename, frame.lineno or 0)
        for frame in traceback.extract_tb(error.__traceback__)
        if os.path.exists(frame.filename)
    ]


def load(path: str | os.PathLike[str]) -> type[Design]:
    """The design a file holds: the one `Design` subclass defined in it."""
    file = Path(path)
    module = _module(file)
    found = [held for held in vars(module).values() if isinstance(held, type) and issubclass(held, Design)]
    own = [held for held in found if held.__module__ == module.__name__]
    if len(own) != 1:
        count = "no design" if not own else f"{len(own)} designs ({', '.join(held.__name__ for held in own)})"
        raise DesignError(
            f"{file} holds {count}; a design file holds one class that subclasses `mf.Design`", str(file), None
        )
    return own[0]


class Project:
    """A design file with what sits beside it: its tuned values, a person's waivers, and the
    processes it names."""

    def __init__(self, path: str | os.PathLike[str]) -> None:
        self.path = Path(path)
        self.design = load(self.path)

    def build(self, overrides: Overrides | None = None) -> Elaborated:
        """The design, built with `overrides` replacing parameters by path."""
        return self.design.elaborate(overrides)

    def beside(self) -> dict[str, Any]:
        """What sits beside the file: `tuned`, `tuned_path`, `waivers` and `processes`."""
        held: dict[str, Any] = json.loads(_engine.beside(str(self.path), self.build))
        return held

    def verify(self) -> Report:
        """Every check, with the design as it is tuned."""
        return Report.read(json.loads(_engine.verify(str(self.path), self.build)))

    def close(self) -> Closure:
        """Tune the design until its requirements hold, and keep what was tuned beside it."""
        return Closure.read(json.loads(_engine.close(str(self.path), self.build)))

    def export(self, directory: str | os.PathLike[str]) -> Report:
        """Verify the design and write its masks, report and elaborated design."""
        return Report.read(json.loads(_engine.export(str(self.path), self.build, str(directory))))

    def corners(self) -> list[Check]:
        """For each process, whether the requirements hold at every corner of it."""
        return [Check.read(held) for held in json.loads(_engine.corners(str(self.path), self.build))]

    def chip_yield(self, chips: int = 500, seed: int = 1) -> list[Check]:
        """For each process, the share of chips, made within its tolerances, that meet every
        requirement."""
        held = json.loads(_engine.chip_yield(str(self.path), self.build, chips, seed))
        return [Check.read(each) for each in held]

    def scene(self, trial: str | None = None) -> dict[str, Any]:
        """What a viewer draws: the chip, where each thing was made, what flows, and what every
        check found. With `trial`, the text of tuned values, those are used over what is kept
        beside the design, and nothing is written."""
        held: dict[str, Any] = json.loads(_engine.scene(str(self.path), self.build, trial))
        return held

    def keep(self, values: str) -> str:
        """Keep `values`, the text of tuned values, beside the design, over what is kept there
        already. Gives the text of what is now kept."""
        return _engine.keep(str(self.path), self.build, values)

    def bias(self) -> list[dict[str, Any]]:
        """For each process that states a print model, and each layer: the mask bias at which the
        layer's narrowest feature prints as drawn, and the bias the process states, in
        micrometres."""
        held: list[dict[str, Any]] = json.loads(_engine.solve_bias(str(self.path), self.build))
        return held


def benchmark(directory: str | os.PathLike[str]) -> Benchmark:
    """The benchmark in a directory."""
    held = json.loads(_engine.benchmark(str(directory)))
    return Benchmark(held["name"], held["brief"], held["process"], tuple(held["regions"]))


def score(directory: str | os.PathLike[str], candidate: str | os.PathLike[str]) -> Score:
    """A benchmark's score for the design in a file. The benchmark's `conditions.py` states how
    the candidate is run and judged, in place of what the candidate says of itself."""
    stated = _module(Path(directory) / "conditions.py")
    conditions: Callable[[Any, Conditions], None] | None = getattr(stated, "conditions", None)
    if conditions is None:
        raise DesignError(f"{Path(directory) / 'conditions.py'} defines no function `conditions`", None, None)
    design = load(candidate)

    def build(overrides: Overrides) -> Elaborated:
        return design.elaborate(overrides, conditions)

    held = json.loads(_engine.score(str(directory), str(candidate), build))
    return Score(
        held["benchmark"], Report.read(held), held["requirements_met"], held["requirements"], held["tuned_values"]
    )


def export_wafer(wafer: str | os.PathLike[str], directory: str | os.PathLike[str]) -> dict[str, Any]:
    """Step the designs a wafer file names across the wafer, and write its masks."""

    def opened(path: str) -> Callable[[Overrides], Elaborated]:
        return Project(path).build

    held: dict[str, Any] = json.loads(_engine.export_wafer(str(wafer), str(directory), opened))
    return held

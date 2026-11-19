"""A design kept alive for a viewer.

The engine's session holds one design file, builds its scene, and builds it again when the file
or anything beside it changes. It speaks JSON, a message to a line, on standard input and
output: what it finds goes out, and what a viewer asks comes in.

Out: `status` (building, ready, or failed with where), `scene`, `diagnostics` (each finding at
the line of the design that made what it is about), and `result` (what a command gave).

In: `rebuild`, `set_overrides` (trial values for tunables; nothing is written), `keep` (the trial
values are written beside the design), `run` (a command by name), and `shutdown`.

The host is here: it reads the design's file again, builds it, and runs a command on it, whose
result is what the command line prints for that command.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

from microfluidics import _engine
from microfluidics._engine import DesignError, Elaborated, Overrides
from microfluidics.commands import COMMANDS
from microfluidics.design import Design
from microfluidics.project import load


class Host:
    """A design's file as the engine's session holds it."""

    def __init__(self, path: Path) -> None:
        self.path = path
        self.design: type[Design] | None = None

    def reload(self) -> None:
        """Read the design again, and the regions written beside it."""
        beside = str(self.path.parent) + os.sep
        for name, module in list(sys.modules.items()):
            if str(getattr(module, "__file__", "") or "").startswith(beside):
                del sys.modules[name]
        self.design = None
        self.design = load(self.path)

    def build(self, overrides: Overrides) -> Elaborated:
        """The design as it was last read, built with `overrides` replacing parameters by path."""
        if self.design is None:
            raise DesignError(f"{self.path} holds no design that could be read", str(self.path), None)
        return self.design.elaborate(overrides)

    def run(self, name: str) -> tuple[bool, str]:
        """Whether a command on the design passed, and what it said."""
        command = next(
            (
                held
                for held in COMMANDS
                if held.name == name and held.arguments[:1] and held.arguments[0].name == "design"
            ),
            None,
        )
        if command is None or len([held for held in command.arguments if held.required]) != 1:
            return False, f"`{name}` is not a command a session runs on its design"
        try:
            result = command.run({"design": str(self.path)})
        except DesignError as error:
            message, file, line = (*error.args, None, None)[:3]
            return False, f"{file or self.path}:{line or 1}: {message}"
        return bool(result["ok"]), str(result["text"])


def serve(design: str | os.PathLike[str]) -> None:
    """Hold `design` for a viewer until standard input ends or the viewer asks the session to end."""
    path = Path(design).resolve()
    _engine.session(str(path), Host(path))

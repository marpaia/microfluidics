"""The package's types, held to what they must accept and what they must refuse."""

import re
import subprocess
import sys
from pathlib import Path

from conftest import ROOT

TYPING = ROOT / "tests" / "typing"


def mypy(file: Path) -> subprocess.CompletedProcess[str]:
    line = [sys.executable, "-m", "mypy", "--strict", "--no-error-summary", str(file)]
    return subprocess.run(line, cwd=ROOT, capture_output=True, text=True)


def test_what_must_type_check_does() -> None:
    checked = mypy(TYPING / "accept.py")
    assert checked.returncode == 0, checked.stdout


def test_every_mistake_is_refused_for_its_own_reason() -> None:
    lines = (TYPING / "refuse.py").read_text().splitlines()
    wanted = {
        number: found.group(1)
        for number, line in enumerate(lines, 1)
        if (found := re.search(r"# refused: ([\w-]+)", line))
    }
    assert len(wanted) >= 14
    checked = mypy(TYPING / "refuse.py")
    refused: dict[int, set[str]] = {}
    for found in re.finditer(r"refuse\.py:(\d+): error: .*\[([\w-]+)\]", checked.stdout):
        refused.setdefault(int(found.group(1)), set()).add(found.group(2))
    for number, kind in wanted.items():
        assert kind in refused.get(number, set()), f"line {number} ({lines[number - 1].strip()}): {refused.get(number)}"
    # Nothing else in the file is in error: every refusal is one that was asked for.
    assert set(refused) == set(wanted), sorted(set(refused) - set(wanted))

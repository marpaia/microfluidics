"""What the engine answers with: checks and their findings, reports, and closures."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class Finding:
    """One thing a check found: the rule broken, what broke it, and what was measured."""

    rule: str
    subject: str
    message: str
    measured: float | None = None
    limit: float | None = None
    waived: str | None = None

    @classmethod
    def read(cls, held: dict[str, Any]) -> Finding:
        return cls(
            held["rule"], held["subject"], held["message"], held.get("measured"), held.get("limit"), held.get("waived")
        )


@dataclass(frozen=True)
class Check:
    """The result of one verification: its status, what it found, and what it measured."""

    id: str
    status: str
    findings: tuple[Finding, ...] = ()
    metrics: dict[str, float] = field(default_factory=dict)
    note: str | None = None

    @property
    def passes(self) -> bool:
        """Whether the check stands in no one's way: satisfied, or waived by a person."""
        return self.status in ("satisfied", "waived")

    @classmethod
    def read(cls, held: dict[str, Any]) -> Check:
        findings = tuple(Finding.read(finding) for finding in held.get("findings", []))
        return cls(held["id"], held["status"], findings, dict(held.get("metrics", {})), held.get("note"))


@dataclass(frozen=True)
class Report:
    """Every check that was run on a design, and whether it passes them all."""

    design: str
    passes: bool
    checks: tuple[Check, ...]
    #: How many things stand in the way of passing: each finding not waived, each check not run.
    problems: int = 0
    inputs: dict[str, str] = field(default_factory=dict)
    files: tuple[str, ...] = ()

    def check(self, id: str) -> Check | None:
        """The check `id`, when it was run."""
        return next((check for check in self.checks if check.id == id), None)

    @classmethod
    def read(cls, held: dict[str, Any]) -> Report:
        checks = tuple(Check.read(check) for check in held["checks"])
        return cls(
            held["design"],
            held["passes"],
            checks,
            int(held.get("problems", 0)),
            dict(held.get("inputs", {})),
            tuple(held.get("files", ())),
        )


@dataclass(frozen=True)
class Change:
    """One parameter closure changed, as it is written."""

    path: str
    start: str
    end: str


@dataclass(frozen=True)
class Closure:
    """What tuning a design came to."""

    converged: bool
    iterations: int
    changes: tuple[Change, ...]
    distances: dict[str, float]
    """How far each requirement is from its bound, in units of its tolerance: it holds at 1 or less."""
    limiting: Finding | None
    tuned: str
    """The tuned values, as the text of the file they are kept in."""
    tuned_path: str

    @classmethod
    def read(cls, held: dict[str, Any]) -> Closure:
        changes = tuple(Change(change["path"], change["start"], change["end"]) for change in held["changes"])
        limiting = Finding.read(held["limiting"]) if held.get("limiting") else None
        return cls(
            held["converged"],
            held["iterations"],
            changes,
            dict(held["distances"]),
            limiting,
            held["tuned"],
            held["tuned_path"],
        )


@dataclass(frozen=True)
class Score:
    """How a candidate did on a benchmark."""

    benchmark: str
    report: Report
    requirements_met: int
    requirements: int
    tuned_values: int


@dataclass(frozen=True)
class Benchmark:
    """A task stated once, by which any design for it is scored."""

    name: str
    brief: str
    process: str
    regions: tuple[str, ...]

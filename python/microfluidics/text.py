"""Results set out for a person to read."""

from collections.abc import Sequence

from microfluidics.results import Check

SHOWN = 8
"""How many of a check's findings are set out before the rest are counted."""

_STATUS = {"satisfied": "satisfied", "violated": "VIOLATED", "waived": "waived", "not_run": "NOT RUN"}


def checks(held: Sequence[Check]) -> str:
    """Each check on a line, with what it found beneath it. Those made against a process follow
    its name, after those that hold whatever makes the chip."""
    lines: list[str] = []
    named: str | None = None
    # Processes in the order they first appear, each with every check made against it.
    order = list(dict.fromkeys(check.process for check in held if check.process is not None))
    for check in sorted(held, key=lambda check: -1 if check.process is None else order.index(check.process)):
        if check.process is not None and check.process != named:
            named = check.process
            lines.append(f"  at {named}")
        indent = "    " if check.process is not None else "  "
        lines.append(f"{indent}{check.id:<28} {_STATUS.get(check.status, check.status)}")
        for finding in check.findings[:SHOWN]:
            waived = f" (waived: {finding.waived})" if finding.waived else ""
            lines.append(f"      {finding.message}{waived}")
        if len(check.findings) > SHOWN:
            lines.append(f"      and {len(check.findings) - SHOWN} more; `--json` gives every one")
        if check.note and check.status == "not_run":
            lines.append(f"      {check.note}")
    return "".join(f"{line}\n" for line in lines)


def metrics(check: Check) -> str:
    """What a check measured, a line for each value."""
    return "".join(f"      {name} = {value:.6f}\n" for name, value in sorted(check.metrics.items()))

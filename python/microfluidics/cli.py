"""The `microfluidics` command.

microfluidics close designs/goenner_mixer.py
microfluidics export designs/goenner_mixer.py runs/goenner_mixer
microfluidics mcp
"""

from __future__ import annotations

import json
import os
import sys
import traceback
from collections.abc import Sequence

from microfluidics._engine import DesignError
from microfluidics.commands import COMMANDS, Command


def usage() -> str:
    lines = ["usage: microfluidics <command> [arguments] [--json] [--traceback]", ""]
    for command in COMMANDS:
        arguments = " ".join(f"<{held.name}>" if held.required else f"[{held.name}]" for held in command.arguments)
        lines.append(f"  {f'{command.name} {arguments}':<34} {command.summary}")
    lines.append(f"  {'session <design>':<34} Hold a design for a viewer: its scene, built again whenever it changes")
    lines.append(f"  {'mcp':<34} Serve every command above as a tool, over standard input and output")
    return "\n".join(lines) + "\n"


def named(command: Command, given: Sequence[str]) -> dict[str, str]:
    """The arguments a command line gives a command, by name."""
    if len(given) > len(command.arguments):
        raise DesignError(
            f"`{command.name}` takes {len(command.arguments)} arguments, and {len(given)} were given", None, None
        )
    for missing in command.arguments[len(given) :]:
        if missing.required:
            raise DesignError(f"`{command.name}` needs <{missing.name}>: {missing.about}", None, None)
    return {held.name: value for held, value in zip(command.arguments, given)}


def fault(error: DesignError) -> str:
    """A fault as `file:line: message`, with the file as it is reached from here."""
    message, file, line = (*error.args, None, None)[:3]
    if file is None:
        return str(message)
    shown = os.path.relpath(file) if os.path.isabs(file) and not os.path.relpath(file).startswith("..") else file
    return f"{shown}:{line}: {message}" if line is not None else f"{shown}: {message}"


def main(argv: Sequence[str] | None = None) -> int:
    given = list(sys.argv[1:] if argv is None else argv)
    as_json, whole = "--json" in given, "--traceback" in given
    given = [argument for argument in given if argument not in ("--json", "--traceback")]
    if not given:
        sys.stderr.write(usage())
        return 2
    name, rest = given[0], given[1:]
    if name == "mcp":
        from microfluidics import mcp

        mcp.serve(sys.stdin, sys.stdout, os.getcwd())
        return 0
    if name == "session":
        from microfluidics import session

        if len(rest) != 1:
            sys.stderr.write("`session` needs <design>: the design's `.py` file\n")
            return 2
        session.serve(rest[0])
        return 0
    command = next((held for held in COMMANDS if held.name == name), None)
    if command is None:
        sys.stderr.write(f"there is no command `{name}`\n\n{usage()}")
        return 2
    try:
        result = command.run(named(command, rest))
    except DesignError as error:
        if whole:
            traceback.print_exc()
        sys.stderr.write(f"{fault(error)}\n")
        return 2
    print(json.dumps(result, indent=2) if as_json else result["text"].rstrip())
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())

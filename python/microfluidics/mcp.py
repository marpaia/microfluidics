"""The commands as tools for an agent: the Model Context Protocol over standard input and
output, a JSON-RPC message to a line.

Each command tool runs the command in a process of its own, with a time limit, so that a design
that hangs or crashes does not take the server with it. A design is code, and is run as such:
this contains faults, it does not confine what a design does.
"""

from __future__ import annotations

import json
import subprocess
import sys
from importlib.metadata import version
from typing import IO, Any

from microfluidics import files
from microfluidics._engine import DesignError
from microfluidics.cli import fault
from microfluidics.commands import COMMANDS, Command

VERSIONS = ("2025-06-18", "2025-03-26", "2024-11-05")
"""The versions of the protocol this server speaks, newest first."""

TIME_LIMIT = 60.0
"""How long a command may run, in seconds."""

INSTRUCTIONS = (
    "Design microfluidic chips as typed Python. A design is a `.py` file holding one class that subclasses "
    "`mf.Design`: read `guide` first, then `regions` and `describe` for what there is to place, and `process` for "
    "the limits a design is made within. Write a design with `write_file`, then run `check` (can it be made), "
    "`solve` (does it do what it must), `close` (tune it until it does), `corners` and `yield` (does it survive the "
    "process), and `export` (verification, masks and report).\n\n"
    "Every result carries the checks that were run. Say a requirement is met, or a design can be made, only when "
    "its check says `satisfied`; a check that is `violated` or `not_run` stands in the way, and its findings say "
    "why. Never report a number that no check or metric gave you.\n\n"
    "A design leaves things open that are not yours to choose: the pressures it is run at, the limits it must "
    "meet, the processes it is made in. When the brief does not give one, ask for it.\n\n"
    "You cannot waive a finding. A waiver is a person's decision, written by them in `<design>.waivers.toml`; say "
    "what you would waive and why. Tuned values are written only by `close`."
)


def _tool(name: str, description: str, schema: dict[str, Any]) -> dict[str, Any]:
    return {"name": name, "description": description, "inputSchema": schema}


def _text(about: str) -> dict[str, str]:
    return {"type": "string", "description": about}


def tools() -> list[dict[str, Any]]:
    listed = [_tool(command.name, command.summary, command.schema()) for command in COMMANDS]
    one = {
        "type": "object",
        "properties": {"path": _text("The file")},
        "required": ["path"],
        "additionalProperties": False,
    }
    both = {"path": _text("The file"), "content": _text("Its whole text")}
    two = {"type": "object", "properties": both, "required": ["path", "content"], "additionalProperties": False}
    where = {"path": _text("The directory; the working directory when not given")}
    listed.append(_tool("read_file", "Read a file within the working directory", one))
    written = (
        "Write a file within the working directory, such as a design; waivers and tuned values are not written this way"
    )
    listed.append(_tool("write_file", written, two))
    listed.append(
        _tool(
            "list_files",
            "The files and directories in a directory within the working directory",
            {"type": "object", "properties": where, "additionalProperties": False},
        )
    )
    return listed


def _run(command: Command, arguments: dict[str, Any], root: str) -> dict[str, Any]:
    """Run a command in a process of its own, every file it names held within `root`."""
    given: list[str] = []
    for held in command.arguments:
        value = arguments.get(held.name)
        if value is None:
            if held.required:
                raise DesignError(f"`{command.name}` needs `{held.name}`: {held.about}", None, None)
            break
        given.append(str(files.confined(root, str(value))) if held.path else str(value))
    line = [sys.executable, "-m", "microfluidics", command.name, *given, "--json"]
    try:
        done = subprocess.run(line, cwd=root, capture_output=True, text=True, timeout=TIME_LIMIT)
    except subprocess.TimeoutExpired as error:
        raise DesignError(
            f"`{command.name}` did not finish within {TIME_LIMIT:.0f} seconds, and was stopped", None, None
        ) from error
    if done.returncode == 2 or not done.stdout.strip():
        raise DesignError(done.stderr.strip() or f"`{command.name}` failed", None, None)
    result: dict[str, Any] = json.loads(done.stdout)
    return result


def _call(params: dict[str, Any], root: str) -> dict[str, Any]:
    name, arguments = params.get("name"), params.get("arguments") or {}
    if name == "read_file":
        return {"ok": True, "text": files.read(root, str(arguments["path"]))}
    if name == "write_file":
        files.write(root, str(arguments["path"]), str(arguments["content"]))
        return {"ok": True, "text": f"wrote {arguments['path']}"}
    if name == "list_files":
        names = files.listing(root, str(arguments.get("path", ".")))
        return {"ok": True, "text": "\n".join(names), "files": names}
    command = next((held for held in COMMANDS if held.name == name), None)
    if command is None:
        raise DesignError(f"there is no tool `{name}`", None, None)
    return _run(command, arguments, root)


def answer(message: dict[str, Any], root: str) -> dict[str, Any] | None:
    """The answer to one request; `None` for a notification, which is not answered."""
    if "id" not in message:
        return None
    method = message.get("method", "")
    result: dict[str, Any]
    if method == "initialize":
        asked = (message.get("params") or {}).get("protocolVersion")
        protocol_version = asked if asked in VERSIONS else VERSIONS[0]
        info = {"name": "microfluidics", "version": version("microfluidics")}
        result = {
            "protocolVersion": protocol_version,
            "capabilities": {"tools": {}},
            "serverInfo": info,
            "instructions": INSTRUCTIONS,
        }
    elif method == "ping":
        result = {}
    elif method == "tools/list":
        result = {"tools": tools()}
    elif method == "tools/call":
        try:
            held = _call(message.get("params") or {}, root)
            said = {"type": "text", "text": json.dumps(held, indent=2)}
            result = {"content": [said], "structuredContent": held, "isError": False}
        except (DesignError, KeyError) as error:
            why = fault(error) if isinstance(error, DesignError) else f"`{error.args[0]}` is missing"
            result = {"content": [{"type": "text", "text": why}], "isError": True}
    else:
        failed = {"code": -32601, "message": f"there is no method `{method}`"}
        return {"jsonrpc": "2.0", "id": message["id"], "error": failed}
    return {"jsonrpc": "2.0", "id": message["id"], "result": result}


def serve(source: IO[str], sink: IO[str], root: str) -> None:
    """Answer requests from `source` on `sink` until `source` ends. Files are read and written
    only within `root`."""
    for line in source:
        if not line.strip():
            continue
        try:
            reply = answer(json.loads(line), root)
        except json.JSONDecodeError as error:
            reply = {"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": str(error)}}
        if reply is not None:
            sink.write(json.dumps(reply) + "\n")
            sink.flush()

# Command reference

Every command of the `microfluidics` command-line tool. The same commands are the tools of the [MCP server](../agents-and-benchmarks.md) and what a [session](../protocols/session.md) runs when a viewer asks.

```
microfluidics <command> [arguments] [--json] [--traceback]
```

## Commands

| Command | Does |
| --- | --- |
| `guide` | Prints the API guide for writing designs and regions |
| `regions [design]` | Lists every region with its parameters; with a design, includes the regions that design's file defines |
| `describe <region> [design]` | Prints one region's parameters, ranges, ports and source |
| `process [name]` | Lists the available processes; with a name, prints that process's rules, tolerances, masks, print model, fab and steps |
| `check <design>` | Runs the fabrication checks: connectivity, and against each process the rules, mask rules, GDSII read-back and print |
| `solve <design>` | Solves the flow and evaluates each requirement |
| `close <design>` | Tunes the tunable parameters until the requirements hold, and writes `<design>.tuned.toml` |
| `corners <design>` | Evaluates the requirements at every corner of each process's tolerances |
| `yield <design> [chips] [seed]` | Estimates, for each process, the share of chips made within its tolerances that meet every requirement |
| `bias <design>` | Finds, for each process with a print model and each layer, the mask bias at which the layer's narrowest feature prints at its drawn width |
| `export <design> <directory>` | Runs every check and writes `design.json`, `report.json`, and for each process a folder with its masks, its `REPORT.md` and the sheet for its fab |
| `wafer <wafer> <directory>` | Steps the designs a wafer file names across the wafer and writes its masks |
| `bench <benchmark> [design]` | Prints a benchmark's brief; with a design, scores that design under the benchmark's conditions |
| `session <design>` | Keeps a design's scene current for a viewer, over standard input and output |
| `mcp` | Serves the commands above as Model Context Protocol tools |

## Exit codes and output

| Exit code | Meaning |
| --- | --- |
| 0 | Everything the command checked passed |
| 1 | Something did not pass |
| 2 | The command could not run |

A command that checks a design against its processes sets out what holds whatever makes the chip first, then the checks made against each process under `at <process>`. In `--json` each of those checks carries its `process`.

A fault in a design is reported as `file:line: message`. `--json` prints the whole result: every check that ran, with its findings and what it measured. `--traceback` prints the Python traceback of a fault.

## Files read beside a design

A command that takes a design reads these files from the design's directory when they exist:

| File | Purpose |
| --- | --- |
| Python files the design imports | Regions written beside the design |
| `<design>.tuned.toml` | Values that replace parameters by path. `close` writes it, and so does the viewer's **Keep** |
| `<design>.waivers.toml` | Findings a person has waived, each with a reason, at one process or at all |
| `<process>.toml` | A process the design names. Without this file, the bundled process of that name is used |

A tuned value is addressed by path, such as `pitch` or `lane[2].trench_length`, and is held to the parameter's dimension and range.

# microfluidics documentation

`microfluidics` takes a chip written as a Python class through fabrication checks, a flow solve, parameter tuning, process variation and mask export. These pages cover writing a design, what each check does, the files the tool writes, and how the tool itself is built.

## Start here

| If you want to… | Read… |
| --- | --- |
| Install the tool and take a design from source to masks | [Getting started](getting-started.md) |
| Write a design or a region of your own | [Designing](designing.md) |
| Understand what a check verifies and what the flow model assumes | [Verification](verification.md) |
| Tune a design until its requirements hold | [Closure](verification.md#closure) |
| Write a process file, waive a finding, or order masks | [Fabrication](fabrication.md) |
| Order a chip from FlowJEM, or make one at COSINC | [FlowJEM](fabs/flowjem.md), [COSINC](fabs/cosinc.md) |
| See the chip beside its source | [Viewer](viewer.md) |
| Drive the tool from an agent, or score a design against a task | [Agents and benchmarks](agents-and-benchmarks.md) |
| Know what the tool does not do | [Limitations](limitations.md) |

## Guides

- [Getting started](getting-started.md) installs the package and runs `check`, `solve`, `close` and `export` on one design.
- [Designing](designing.md) covers regions, designs, the standard regions, and what the type checker catches. The packaged API guide is [`python/microfluidics/guide.md`](../python/microfluidics/guide.md), which `microfluidics guide` prints.
- [Verification](verification.md) covers checks, connectivity, the flow model, requirements, closure, corners, yield and the report.
- [Fabrication](fabrication.md) covers processes and the fabs they describe, design rules, waivers, mask preparation, hand-off sheets, the print model, process monitors, wafers and agreement with KLayout.
- [FlowJEM](fabs/flowjem.md) and [COSINC](fabs/cosinc.md) cover the two fabs the examples are made at: what each publishes, what is assumed, and how a design reaches it.
- [Viewer](viewer.md) covers the VS Code extension.
- [Agents and benchmarks](agents-and-benchmarks.md) covers the MCP server and the benchmark format.
- [Examples](examples.md) describes the four designs in [`designs/`](../designs/), which fab each can be made at, and their results.
- [Limitations](limitations.md) lists what is not modelled, not validated, or not built.

## Reference

- [Commands](reference/commands.md) lists every command, its exit codes, and the files it reads beside a design.
- [Terminology](reference/terminology.md) defines the words the project uses and gives the source of each.

## Formats and protocols

- [Scene format](formats/scene.md) defines the document a viewer draws.
- [Session protocol](protocols/session.md) defines the messages between a viewer and the process that keeps a design's scene current.

## Development

- [Architecture](development/architecture.md) describes the crates, the Python package and the extension, and how a design passes through them.
- [Testing](development/testing.md) describes the test layers, the captured results the tests compare against, and the commands CI runs.

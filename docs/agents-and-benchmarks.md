# Agents and benchmarks

This page covers driving the tool from an AI agent through the Model Context Protocol, and benchmarks: tasks stated once, against which any design can be scored.

## MCP server

`microfluidics mcp` speaks the Model Context Protocol over standard input and output.

```console
claude mcp add microfluidics -- microfluidics mcp
```

Its tools are the [commands](reference/commands.md) and three file tools, `read_file`, `write_file` and `list_files`, which work only within the directory the server was started in. There are no tools for editing a design piece by piece. An agent writes Python, as a person does, and runs `check`.

- **Every result carries its checks.** The server's instructions tell an agent to call a requirement met only when its check says so, and to ask about anything a brief leaves open.
- **An agent cannot waive a finding.** `write_file` refuses `*.waivers.toml`, which only a person writes, and `*.tuned.toml`, which only `close` writes.
- **A design is code.** Each command runs in its own process with a time limit, so a design that hangs or crashes does not take the server down. This contains faults. It does not sandbox what a design's code can do.

## Benchmarks

A benchmark is a directory under [`benchmarks/`](../benchmarks/):

| File | Contents |
| --- | --- |
| `brief.md` | The task in plain words |
| `benchmark.toml` | The process, and the regions a candidate may place |
| `conditions.py` | How every candidate is run and judged |

`conditions.py` states what a candidate must provide as a typed `Protocol`, and the operating conditions and requirements applied to it:

```python
class Candidate(Protocol):
    buffer_in: AccessPad
    reagent_in: AccessPad
    sample_in: AccessPad
    outlet: AccessPad


def conditions(chip: Candidate, run: mf.Conditions) -> None:
    buffer, reagent, sample = mf.Species(), mf.Species(), mf.Species()
    run.buffer, run.reagent, run.sample = buffer, reagent, sample
    run.operate(
        viscosity=1 * mPa_s,
        drive={chip.buffer_in: 5 * mbar, chip.reagent_in: 5 * mbar, chip.sample_in: 5 * mbar, chip.outlet: 0 * mbar},
        feed={chip.buffer_in: {buffer: 100 * pct}, chip.reagent_in: {reagent: 100 * pct}, chip.sample_in: {sample: 100 * pct}},
    )
    run.buffer_share = mf.Requirement(
        "Buffer is 89.2 % of the mixture", mf.fraction(buffer, chip.outlet), mf.target(89.2 * pct, within=1 * pct, relative=True)
    )
```

Scoring sets a candidate's own operating conditions and requirements aside and applies the benchmark's, so no design is judged by requirements it chose. Waivers do not count.

`microfluidics bench <benchmark>` prints the brief, and `microfluidics bench <benchmark> <design>` scores a design.

| Benchmark | Task | Reference design |
| --- | --- | --- |
| [`metering_mixer`](../benchmarks/metering_mixer/brief.md) | Three liquids metered to 89.2 %, 10.72 % and 0.08 % from one pressure | `goenner_mixer`: 3 of 3 requirements hold; passes |

The benchmark's process is `flowjem-su8-pdms`, so a candidate is held to what FlowJEM makes.

## Evidence so far

The MCP server has been driven only by tests that speak its protocol.

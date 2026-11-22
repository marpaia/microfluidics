# Microfluidics

`microfluidics` is a design tool for microfluidic chips made by soft lithography. A chip is written as a typed Python class that places parametric regions, states how the chip is run, and states what it must do. The tool checks the design against a fabrication process, solves its flow, tunes its parameters until the requirements hold, checks it across process variation, and writes photomasks with a verification report.

A soft-lithography chip is a few flat patterns, each built to one height. The tool works from that description: layout in the plane, and flow as a network of rectangular ducts. It has no solid modeller and no field solver.

<p align="center">
  <img src="docs/assets/screenshot-mixer-3d-speed.png" alt="The mixer design in VS Code: its Python source beside a 3D chip coloured by flow speed, with passing verification checks" width="100%">
</p>

## What it does

- **Typed designs:** regions and designs are Python classes whose parameters and ports are typed fields. `100 * um` is a `Length`, and a type checker rejects it where a `Pressure` is expected. See [designing](docs/designing.md).
- **Fabrication checks:** a process file holds design rules, mask preparation and a print model. The tool checks the drawn chip and the prepared masks against it, and compares stated connections with drawn geometry. See [fabrication](docs/fabrication.md).
- **Flow and requirements:** the flow network is extracted from the design and solved for pressure, flow rate, shear and mixing fractions. Each requirement is a measure with a bound. See [verification](docs/verification.md).
- **Closure:** `close` tunes the parameters a design marks as tunable until every requirement holds, rejecting any trial that breaks a design rule. See [closure](docs/verification.md#closure).
- **Process variation:** requirements are re-evaluated at every corner of the process tolerances, and over a Monte Carlo sample when a design asks for a yield. See [corners and yield](docs/verification.md#corners-and-yield).
- **Masks and wafers:** `export` writes GDSII and DXF masks, reads them back to confirm they match, and writes a report in which every claim is a named check. `wafer` steps chips across a wafer. See [masks](docs/fabrication.md#masks).

## Quick start

Install a published release with CPython 3.11 or newer.

```console
python -m pip install microfluidics
microfluidics guide
```

To build from a checkout and run the example designs, use a Rust toolchain (`rustup` reads the pinned version from `rust-toolchain.toml`) and [uv](https://docs.astral.sh/uv/).

```console
git clone https://github.com/marpaia/microfluidics.git
cd microfluidics
uv venv && source .venv/bin/activate
uv pip install maturin && maturin develop
microfluidics solve designs/mixer.py
microfluidics export designs/mixer.py runs/mixer
```

[`designs/mixer.py`](designs/mixer.py) meters three liquids into one stream in shares of 50 %, 30 % and 20 %, set by the length of each liquid's path to a junction. `solve` extracts its flow network and reports each share against its target. `export` runs every check and writes `REPORT.md`, `report.json` and the masks under `runs/mixer/`.

The design marks two serpentine lengths as tunable. `microfluidics close designs/mixer.py` adjusts them until the shares sit exactly on target and writes the values to `designs/mixer.tuned.toml`, leaving the source unchanged.

[Getting started](docs/getting-started.md) walks through checking, solving, closing and exporting a design, and how to read the report.

## A design

This is [`designs/test_channel.py`](designs/test_channel.py) without its process monitor:

```python
class TestChannel(mf.Design):
    """One straight channel between two pads."""

    process = CONSERVATIVE_SU8_PDMS
    slab = mf.Slab(8 * mm, 16 * mm, thickness=4 * mm)
    glass = mf.Glass(75 * mm, 25 * mm, thickness=1 * mm)
    layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

    def build(self) -> None:
        self.inlet = AccessPad().at(4 * mm, 2.5 * mm)
        self.duct = StraightChannel(length=8 * mm).attach(lambda duct: duct.inlet, to=self.inlet.opening)
        self.outlet = AccessPad().attach(lambda pad: pad.opening, to=self.duct.outlet)

        self.operate(viscosity=1 * mPa_s, drive={self.inlet: 10 * mbar, self.outlet: 0 * mbar})
        self.slow = mf.Requirement("Flow is slow enough for the model to hold", mf.reynolds(self.duct), mf.at_most(1.0))
```

Assigning to an attribute names the thing: `self.inlet` is the pad `inlet`, and findings and tuned values refer to it by that path. Every rectangle, port and region also records the source line that made it, so a finding points at the line that caused it.

## Examples

| Design | Shows |
| --- | --- |
| [`test_channel.py`](designs/test_channel.py) | The smallest design: one channel between two pads |
| [`mixer.py`](designs/mixer.py) | Three liquids metered 50 / 30 / 20 % by channel length alone |
| [`goenner_mixer.py`](designs/goenner_mixer.py) | The calcium-assay mixer of Goenner et al. (2025), closed to 89.18 / 10.74 / 0.07995 % |
| [`mother_machine_pilot.py`](designs/mother_machine_pilot.py) | Eight independent lanes of growth trenches on two layers |
| [`mother_machine_manifold.py`](designs/mother_machine_manifold.py) | One inlet feeding eight lanes, balanced by closure |

[Examples](docs/examples.md) gives each design's results and what they demonstrate.

## Viewer and agents

The VS Code extension in [`editors/vscode`](editors/vscode) shows a design's chip beside its source and rebuilds it on save. It lists problems by the check that found them, plots each requirement against its bound and its range across process corners, and gives each tunable parameter a slider. See the [viewer guide](docs/viewer.md).

`microfluidics mcp` serves every command as a Model Context Protocol tool, so an agent can write a design and check it the way a person does. See [agents and benchmarks](docs/agents-and-benchmarks.md).

## Status

The tool has not yet produced a fabricated chip. The bundled process, `conservative-su8-pdms`, is a stand-in with rule-of-thumb limits and guessed tolerances, not a cleanroom's design rules. The flow model has not been compared with a field solver or a measurement, and the print model is not fitted to a process. KLayout reads the mask files and agrees with the tool's own checks, but no file has been sent to a mask shop. [Limitations](docs/limitations.md) lists what the tool does not do.

## Documentation

| Topic | Entry point |
| --- | --- |
| First run | [Getting started](docs/getting-started.md) |
| Writing designs and regions | [Designing](docs/designing.md) |
| Checks, flow, closure, variation | [Verification](docs/verification.md) |
| Process rules, masks, wafers | [Fabrication](docs/fabrication.md) |
| VS Code extension | [Viewer](docs/viewer.md) |
| MCP server and benchmarks | [Agents and benchmarks](docs/agents-and-benchmarks.md) |
| Commands | [Command reference](docs/reference/commands.md) |
| Development | [Architecture](docs/development/architecture.md) and [testing](docs/development/testing.md) |
| Python releases | [Release process](docs/development/releasing.md) |

The complete index is at [`docs/README.md`](docs/README.md).

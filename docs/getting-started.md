# Getting started

This page installs the package and takes one design, a mother machine with eight lanes fed from one inlet, from source to masks. It uses `check`, `solve`, `close` and `export`, which are the commands most work goes through.

## Install

The package needs Python 3.11 or newer, a Rust toolchain, and [uv](https://docs.astral.sh/uv/). `rustup` installs the toolchain version pinned in `rust-toolchain.toml` on first use.

```console
git clone https://github.com/marpaia/microfluidics.git
cd microfluidics
uv venv && source .venv/bin/activate
uv pip install maturin && maturin develop
```

`maturin develop` compiles the engine and installs the `microfluidics` package and command into the virtual environment. Run it again after changing anything under `crates/`.

## Work on a copy

`close` writes a file beside the design it tunes, and the repository already holds tuned values for this design. Copy the design somewhere else to see it as written:

```console
mkdir -p runs/tour && cp designs/mother_machine_manifold.py runs/tour/
cd runs/tour
```

## Check that it can be made

```console
$ microfluidics check mother_machine_manifold.py
mother_machine_manifold: can be made as drawn
  rules                        satisfied
  connectivity                 satisfied
  mask-rules                   satisfied
  gds-readback                 satisfied
  print                        satisfied
```

`check` runs the fabrication checks: the design rules on the chip as drawn and on the prepared masks, the comparison of stated connections with drawn geometry, and the print model. [Fabrication](fabrication.md) describes each.

## Solve its flow

```console
$ microfluidics solve mother_machine_manifold.py
mother_machine_manifold: NOT every requirement holds
  graph                        satisfied
  requirement/balanced         VIOLATED
      Every lane gets the same flow: lane[0] is 0.09858ul_min, and must be within 5% of their mean, 0.1474ul_min
      ...
  requirement/gentle           satisfied
```

The design asks that every lane get the same flow within 5 %. As written, the two middle lanes get more than twice the flow of the outer ones, because the outer lanes sit at the far ends of the manifold's trunk. The command exits with 1 when a requirement does not hold.

## Close it

The design marks four parameters as tunable: the lane pitch, the lane width, and the trunk width of each manifold. `close` searches those ranges for values at which every requirement holds.

```console
$ microfluidics close mother_machine_manifold.py
mother_machine_manifold: every requirement holds after 23 moves
  pitch: 2600um -> 400um
  lane_width: 100um -> 40.338um
  feed.trunk_width: 200um -> 239.724um
  drain.trunk_width: 200um -> 239.724um
  balanced: at 0.541 of its tolerance; it holds at 1 or less
  gentle: at 0.650 of its tolerance; it holds at 1 or less
written to mother_machine_manifold.tuned.toml
```

The source file is unchanged. The tuned values are in `mother_machine_manifold.tuned.toml`, and every later command reads them. [Closure](verification.md#closure) explains the search and what it reports when the requirements cannot all be met.

## Export masks and a report

```console
$ microfluidics export mother_machine_manifold.py out
mother_machine_manifold: passes
with 4 tuned values from mother_machine_manifold.tuned.toml
  deterministic                satisfied
  rules                        satisfied
  ...
  corners                      satisfied
  dxf-readback                 satisfied
written to out: REPORT.md, report.json, design.json, masks/chip.gds, masks/mask_1_trench.dxf, masks/mask_2_flow.dxf, masks/preview.svg, masks/README.md
```

`export` runs every check, including the process corners, and writes:

| File | Contents |
| --- | --- |
| `REPORT.md`, `report.json` | Every check with its status, its findings and what it measured, and a hash of every input |
| `design.json` | The elaborated design |
| `masks/chip.gds` | Every layer in GDSII |
| `masks/mask_<n>_<layer>.dxf` | One flat drawing per layer |
| `masks/preview.svg` | All layers together, for viewing in a browser |
| `masks/README.md` | Each mask's tone, bias and mirroring, for whoever makes the masks |

Open `out/REPORT.md` first. Its opening line says whether the design passes, and the table under it lists each check. A design passes only when every check is satisfied or waived.

## Next steps

- Open the design in VS Code with the [viewer](viewer.md) to see the chip, drag a tunable parameter, and watch the requirements respond.
- Read [designing](designing.md) and `microfluidics guide` before writing a design of your own. [`designs/mixer.py`](../designs/mixer.py) is a small one to start from.
- Run `microfluidics` with no arguments for the list of commands, or see the [command reference](reference/commands.md).

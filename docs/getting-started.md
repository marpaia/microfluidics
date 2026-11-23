# Getting started

This page installs the package and takes one design, a mixer that meters three liquids into one stream, from source to the files a fab is sent. It uses `check`, `solve`, `close` and `export`, which are the commands most work goes through.

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
mkdir -p runs/tour && cp designs/goenner_mixer.py runs/tour/
cd runs/tour
```

## Check that it can be made

The design names two processes, one for each fab it can be made at:

```python
processes = (FLOWJEM_SU8_PDMS, COSINC_SU8_PDMS)
```

```console
$ microfluidics check goenner_mixer.py
goenner_mixer: can be made as drawn
  connectivity                 satisfied
  at flowjem-su8-pdms
    rules                        satisfied
    mask-rules                   satisfied
    gds-readback                 satisfied
  at cosinc-su8-pdms
    rules                        satisfied
    mask-rules                   satisfied
    gds-readback                 satisfied
    print                        satisfied
```

`check` runs the fabrication checks: the comparison of stated connections with drawn geometry, and then, against each process, the design rules on the chip as drawn and on the prepared masks, and the print model when the process states one. [Fabrication](fabrication.md) describes each.

## Solve its flow

```console
$ microfluidics solve goenner_mixer.py
goenner_mixer: NOT every requirement holds
  graph                        satisfied
  requirement/buffer_share     satisfied
  requirement/reagent_share    VIOLATED
      Reagent is 10.72 % of the mixture: outlet is 11.16%, and must be 10.72% within 1%
  requirement/sample_share     satisfied
```

The design asks for 89.2 % buffer, 10.72 % reagent and 0.08 % sample, each within 1 % of its value. As written, the reagent's share is 11.16 %. The command exits with 1 when a requirement does not hold.

## Close it

The design marks two parameters as tunable: the run length of the reagent's serpentine and of the sample's. `close` searches their ranges for values at which every requirement holds. A trial that breaks a rule of either process is never moved to.

```console
$ microfluidics close goenner_mixer.py
goenner_mixer: every requirement holds after 5 moves
  reagent_resistor.run_length: 1000um -> 1176.674um
  sample_resistor.run_length: 5600um -> 5700.237um
  buffer_share: at 0.024 of its tolerance; it holds at 1 or less
  reagent_share: at 0.196 of its tolerance; it holds at 1 or less
  sample_share: at 0.058 of its tolerance; it holds at 1 or less
written to goenner_mixer.tuned.toml
```

The source file is unchanged. The tuned values are in `goenner_mixer.tuned.toml`, and every later command reads them. [Closure](verification.md#closure) explains the search and what it reports when the requirements cannot all be met.

## Export masks, a report and a sheet for each fab

```console
$ microfluidics export goenner_mixer.py out
goenner_mixer: passes
with 2 tuned values from goenner_mixer.tuned.toml
  deterministic                satisfied
  connectivity                 satisfied
  graph                        satisfied
  requirement/buffer_share     satisfied
  requirement/reagent_share    satisfied
  requirement/sample_share     satisfied
  at flowjem-su8-pdms
    rules                        satisfied
    ...
    dxf-readback                 satisfied
  at cosinc-su8-pdms
    rules                        satisfied
    ...
    dxf-readback                 satisfied
written to out: design.json, report.json, flowjem-su8-pdms/REPORT.md, flowjem-su8-pdms/ORDER.md, flowjem-su8-pdms/masks/chip.gds, ...
```

`export` runs every check, including each process's corners, and writes a folder for each process:

| File | Contents |
| --- | --- |
| `design.json` | The elaborated design |
| `report.json` | Every check at every process, with its status, its findings and what it measured, and a hash of every input |
| `<process>/REPORT.md` | The checks that bear on that process, for a person to read |
| `<process>/ORDER.md` or `TRAVELER.md` | The sheet for the fab: an order for one that is sent files, a traveler for a chip made by hand |
| `<process>/masks/chip.gds` | Every layer in GDSII |
| `<process>/masks/mask_<n>_<layer>.dxf` | One flat drawing per layer |
| `<process>/masks/preview.svg` | All layers together, for viewing in a browser |
| `<process>/masks/README.md` | Each mask's tone, bias and mirroring |

Open `out/flowjem-su8-pdms/REPORT.md` first. Its opening line says whether the design passes at FlowJEM, and the table under it lists each check. A design passes only when every check is satisfied or waived, at every process it names.

Then open `out/flowjem-su8-pdms/ORDER.md`. It says which limits FlowJEM has not published and what to ask before ordering. [FlowJEM](fabs/flowjem.md) and [COSINC](fabs/cosinc.md) describe how a design reaches each fab.

## Next steps

- Open the design in VS Code with the [viewer](viewer.md) to see the chip, drag a tunable parameter, and watch the requirements respond.
- Read [designing](designing.md) and `microfluidics guide` before writing a design of your own. [`designs/mixer.py`](../designs/mixer.py) is a small one to start from.
- Run `microfluidics` with no arguments for the list of commands, or see the [command reference](reference/commands.md).

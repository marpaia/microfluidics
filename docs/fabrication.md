# Fabrication

This page covers what stands between a drawn chip and a made one: processes and the fabs they describe, design rules, waivers, mask preparation, the sheet that goes to the fab, the print model, the structures measured on a finished master, wafers, and how the output compares with KLayout.

## Processes

A process is a TOML file. It holds a deck of design rules, as a fab's design rule manual does, and may also state fabrication tolerances, how each layer's mask is prepared from its drawing, a print model, the fab it describes, and the steps by which a chip is made there. `microfluidics process` lists the available processes, and `microfluidics process <name>` prints one.

A design names its processes as a tuple, and is held to the rules of every one:

```python
class GoennerMixer(mf.Design):
    processes = (FLOWJEM_SU8_PDMS, COSINC_SU8_PDMS)
```

For each name the tool looks for `<name>.toml` beside the design and falls back to the bundled process of that name. Two processes are bundled, one for each fab the examples are made at:

| Process | Fab | How a design reaches it | Smallest feature and gap |
| --- | --- | --- | --- |
| [`flowjem-su8-pdms`](../processes/flowjem-su8-pdms.toml) | [FlowJEM](fabs/flowjem.md), by mail order | An order, with a DXF drawing for each layer | 5 um, published |
| [`cosinc-su8-pdms`](../processes/cosinc-su8-pdms.toml) | [COSINC](fabs/cosinc.md), by hand | A traveler, with the GDSII file | 1 um, assumed |

### What a fab has not published

A process file names its fab in a `[fab]` table:

```toml
[fab]
name = "FlowJEM"
handoff = "order"
drawing = "dxf"
source = "https://www.flowjem.com/placing-an-order.html"
read = "2026-11-23"
polarity = "Right reading, emulsion side down. Channels are transparent on the mask."
assumed = ["layer.overlap", "resist.aspect", "roof.span", "port.pitch", "port.edge", "chip.size", "layer.count"]
questions = ["How far may one layer sit from another after alignment?"]
```

| Field | Says |
| --- | --- |
| `handoff` | `order` for a fab that is sent files, `traveler` for a chip a person makes there |
| `drawing` | The mask file the fab works from: `dxf` or `gds` |
| `source`, `read` | Where the limits were read, and on what date |
| `assumed` | The rules, by name, that the fab has not published, and any of the tables `tolerances`, `masks` and `print` |
| `polarity` | How the fab wants the tone of a mask stated |
| `questions` | What to ask the fab before relying on what is assumed |

A name under `assumed` that is neither a rule of the deck nor one of those tables is refused when the file is read.

The `rules` and `mask-rules` checks carry what is assumed in their note, and so does every sheet written for the fab. FlowJEM's process assumes 7 of its 10 limits. COSINC has published no limit on what its tools make in SU-8, so its process assumes all 10, its tolerances and its print model. A design that passes at COSINC passes against assumptions until COSINC's staff or a test wafer confirm them.

## Design rules

Each rule has a name and one of ten kinds. The process file decides which rules exist, on which layers, and with what limits.

| Kind | Limits |
| --- | --- |
| `width`, `space` | The narrowest feature, and the smallest gap between two features of one layer that do not touch |
| `overlap` | How far a feature that opens into a deeper one is drawn into it on the mask |
| `aspect` | The tallest feature, or wall between features, relative to its width |
| `roof_span` | The widest roof relative to its height, less what a punched hole removes |
| `port_pitch`, `edge_margin` | The spacing of access points, and their distance from the chip's edge |
| `extent` | The chip's size, in either orientation |
| `layer_height`, `layer_count` | The heights a layer may have, and how many layers |

A rule that names no layer applies to every layer. One function interprets the deck and runs twice: on the chip as drawn (`rules`), and on the prepared masks (`mask-rules`), where overlap rules also apply. Regions are taken by their bounding rectangles, and arrays are checked flattened. A rule reports its twenty worst violations and counts the rest.

## Waivers

A person may waive a finding by rule and subject, with a reason, in `<design>.waivers.toml` beside the design:

```toml
[[waivers]]
rule = "feature.space"
subject = "lane[2].left[17]"
reason = "Printed at this pitch before"
by = "a researcher"
```

A waiver with `process = "<name>"` accepts the finding at that process alone; without one it accepts it at every process the design names. A waived finding is reported as waived. It is never dropped from the report.

## Masks

Mask preparation turns the chip as drawn into what each mask exposes, in three steps. It is done once for each process, since each has its own overlap, bias and mirroring:

1. **Overlap.** A shallow feature that opens into a deeper one is extended into it by the distance the process's overlap rules give, so the two still meet when the layers are misaligned.
2. **Bias.** Every edge moves outward or inward by the bias the process gives the mask.
3. **Mirror.** A mask the process mirrors is reflected about the chip's vertical centre line as it is written. Rules and the print check run before this step, on unmirrored masks.

Rectangles repeated at a constant step are stored as translates of one outline, and stay that way through all three steps when every element is drawn alike. The mother machine's 7,998 trenches are six entries, one for each side of each lane. Each mask also carries its tone: clear inside the outlines, for a negative resist, or dark. Lengths are metres inside the engine and whole nanometres in the masks.

`export` writes a folder for each process, then reads the mask files back from disk and compares them with the plan:

| File | Contents |
| --- | --- |
| `design.json`, `report.json` | The elaborated design, and every check at every process |
| `<process>/REPORT.md` | The checks that bear on the process: those that hold whatever makes the chip, and its own |
| `<process>/ORDER.md` or `TRAVELER.md` | The sheet for the fab, when the process names one |
| `<process>/masks/chip.gds` | Every layer in GDSII, with repeated features as structures placed by arrays; coordinates in nanometres |
| `<process>/masks/mask_<n>_<layer>.dxf` | One drawing for each layer, flat, in micrometres |
| `<process>/masks/preview.svg` | Every layer together, for viewing in a browser |
| `<process>/masks/README.md` | Each mask's tone, bias and mirroring |

A design that names no process has its masks and report written into the directory itself.

### Drawings hold each region once

A DXF drawing holds closed outlines of no width, none of which overlaps another. Features that touch or overlap, such as a channel and the pad it enters, are drawn as one outline: the boundary of them together. A region with a hole, such as two trunks joined by several lanes, is one outline that runs in to the hole along a cut and back out the same way, so that it touches itself and crosses nothing. Mask shops ask for drawings of this kind; FlowJEM's rules for a design file forbid overlapping boundaries and allow one that touches itself.

The GDSII file keeps each feature as it is drawn, which is what keeps repeated features as arrays. For the mother machine, `chip.gds` is 4.3 kB and holds 8,041 outlines on the trench layer and 48 on the flow layer. Its drawings hold 8,037 and 42, and the trench layer's is 1.8 MB.

## Hand-off sheets

For a process that names a fab, `export` writes one sheet beside the masks. Both kinds open with the fab, the date its limits were read, what is assumed, and the questions to ask first.

| `ORDER.md`, for a fab that is sent files | `TRAVELER.md`, for a chip made by hand |
| --- | --- |
| The files to send, each with its SHA-256 | The file each layer is exposed from, with its SHA-256 |
| Each layer's height, narrowest feature, tone, mirroring and bias | The same, in the order the layers are exposed |
| Mask polarity in the fab's own words | For each layer: the process's steps, with columns to mark each done |
| Each limit with what the chip measures against it, and whether the fab published it | For each layer: its height and its five narrowest widths as drawn, with a column for what was measured |
| The slab and the glass, and a blank for the quantity | The centre of each alignment mark |
| The punch diameter, and the position of every hole | The steps after the master, the slab and the glass, and the position of every hole |

A process states its steps as `[[steps]]`, each with a title, the layer it makes, and what to do; a step that names no layer comes after the master. COSINC's steps are those of the Jun lab's mother machine handbook (version 2.00, 2010), which was written for another cleanroom's tools: each resist, spin speed, bake and exposure in them is a starting point.

## Alignment marks

`AlignmentMark` draws a cross on the trench layer inside a square window on the flow layer, and records its centre as a fiducial. With a clear tone a mask is opaque outside its features, so the cross made by the first mask is seen only through a feature of the second: the window. The second mask is aligned by centring the window on the cross, and the gap between them is the largest misalignment through which the whole cross is still seen.

A feature may lie wholly within one of a taller layer, as the cross does within the window: the cast chip has the taller feature there. Any other overlap of two features is a fault in the design.

A region records a fiducial with `self.fiducial(at=...)`. A traveler lists every fiducial's position, which is what a direct-write tool is given to align a layer by.

## Print model

The print check models the exposure a mask gives as the mask blurred by a Gaussian, and the resist as remaining wherever the exposure passes a threshold. For rectangles this has a closed form, so each narrow feature's printed width is found along a line across it, with the light from its neighbours included.

A feature fails when it prints further from its drawn width than the process's tolerance, does not print, or prints merged with a neighbour. Only features narrower than eight blur widths are examined, and identical features with identical neighbours are computed once.

`microfluidics bias <design>` finds, for each layer, the bias at which its narrowest feature prints at its drawn width.

The model is first-order. COSINC's process states a blur and a threshold that are guesses, and FlowJEM's states no print model, so the check does not run against it. The check says in its note that a model is not fitted until a process sets `calibrated = true`.

## Process monitors

Three structures exist to be measured on a finished master:

- **`ProcessMonitor`** places lines on each layer that step through five widths from the narrowest drawn, and a vernier in each direction that shows the offset between the flow and trench layers in steps of 0.5 um. The mother machine carries one.
- **`LineMonitor`** is the single-layer part of it, for a one-layer design. `goenner_mixer` and the test channel carry one.
- **[`designs/test_channel.py`](../designs/test_channel.py)** is one straight channel between two pads. Its flow at a known pressure gives the layer's real height.

A traveler lists each layer's five narrowest widths, which on a chip with a monitor are the monitor's lines, with a column for what was measured.

Measuring these is how a process's blur, bias and tolerances, and the flow model's error, would be found. The tool does not yet read such measurements back into a process file.

## Wafers

A wafer file names designs and how many columns and rows of each to place:

```toml
name = "mother-machine-wafer"
diameter_mm = 100.0
edge_exclusion_mm = 5.0
street_mm = 1.0

[[chips]]
design = "mother_machine.py"
columns = 4
rows = 2
```

`microfluidics wafer designs/mother_machine_wafer.toml runs/wafer` steps each design's masks across the wafer, using its tuned values. A wafer is made in one process: when a design on it names more than one, the wafer file says which with `process = "<name>"`.

Each chip is labelled below its outline in the deepest layer, with ticks at its corners for dicing. Two marks align the wafer, one either side of it: a cross on the shallowest layer, which is exposed first, and on every other layer a window over the cross. Trenches remain arrays: the eight-chip wafer's `chip.gds` is 116 kB, against 14 MB for the drawing of its trench layer. A chip that would extend into the edge exclusion is rejected by name.

## Agreement with KLayout

KLayout is the layout tool that cleanrooms and mask shops use to check files. The tool does not depend on it, but tests have KLayout read the exported files and compare:

- It reads the GDSII as the same structures, outline counts and areas.
- Its difference between each DXF drawing and the corresponding GDSII layer is empty.
- It finds as many outlines in each drawing as were planned, and no area where two of them overlap.
- On a design seeded with features that are too narrow or too close, its width and space checks find exactly as many violations as the rule check here.

```console
python3 -m pip install klayout
KLAYOUT_PYTHON=python3 pytest -m klayout
```

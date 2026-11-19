# Fabrication

This page covers what stands between a drawn chip and a set of masks: the process file and its design rules, waivers, mask preparation, the print model, the structures measured on a finished master, wafers, and how the output compares with KLayout.

## Processes

A process is a TOML file. It holds a deck of design rules, as a fab's design rule manual does, and may also state fabrication tolerances, how each layer's mask is prepared from its drawing, and a print model. `microfluidics process` lists the available processes, and `microfluidics process <name>` prints one.

A design names its process. The tool looks for `<name>.toml` beside the design and falls back to the bundled process of that name.

The one bundled process, [`processes/conservative-su8-pdms.toml`](../processes/conservative-su8-pdms.toml), is a stand-in for a two-layer SU-8 master and PDMS cast. Its limits are cautious rules of thumb, and its tolerances and print model are guesses. None of its numbers are measurements, and it is not a cleanroom's or a vendor's design rule manual. Replace it with the real process before ordering masks.

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

A waived finding is reported as waived. It is never dropped from the report.

## Masks

Mask preparation turns the chip as drawn into what each mask exposes, in three steps:

1. **Overlap.** A shallow feature that opens into a deeper one is extended into it by the distance the process's overlap rules give, so the two still meet when the layers are misaligned.
2. **Bias.** Every edge moves outward or inward by the bias the process gives the mask.
3. **Mirror.** A mask the process mirrors is reflected about the chip's vertical centre line as it is written. Rules and the print check run before this step, on unmirrored masks.

Rectangles repeated at a constant step are stored as translates of one outline, and stay that way through all three steps when every element is drawn alike. The pilot's 1,600 trenches are sixteen entries. Each mask also carries its tone: clear inside the outlines, for a negative resist, or dark. Lengths are metres inside the engine and whole nanometres in the masks.

`export` writes these files, then reads them back from disk and compares them with the plan:

| File | Contents |
| --- | --- |
| `chip.gds` | Every layer in GDSII, with repeated features as structures placed by arrays; coordinates in nanometres |
| `mask_<n>_<layer>.dxf` | The same outlines, one layer per file, flat, in micrometres |
| `preview.svg` | Every layer together, for viewing in a browser |
| `README.md` | Each mask's tone, bias and mirroring, for whoever makes the masks |

For the pilot, `chip.gds` is 6 kB and holds 1,643 outlines on the trench layer and 69 on the flow layer. The flat DXF of the trench layer is 357 kB.

## Print model

The print check models the exposure a mask gives as the mask blurred by a Gaussian, and the resist as remaining wherever the exposure passes a threshold. For rectangles this has a closed form, so each narrow feature's printed width is found along a line across it, with the light from its neighbours included.

A feature fails when it prints further from its drawn width than the process's tolerance, does not print, or prints merged with a neighbour. Only features narrower than eight blur widths are examined, and identical features with identical neighbours are computed once.

`microfluidics bias <design>` finds, for each layer, the bias at which its narrowest feature prints at its drawn width.

The model is first-order, and the stand-in process's blur and threshold are guesses. The check says so in its note until a process sets `calibrated = true`.

## Process monitors

Three structures exist to be measured on a finished master:

- **`ProcessMonitor`** places lines on each layer that step through five widths from the narrowest drawn, and a vernier in each direction that shows the offset between the flow and trench layers in steps of 0.5 um. Both mother-machine designs and the test channel carry one.
- **`LineMonitor`** is the single-layer part of it, for a one-layer design. `goenner_mixer` carries one.
- **[`designs/test_channel.py`](../designs/test_channel.py)** is one straight channel between two pads. Its flow at a known pressure gives the layer's real height.

Measuring these is how a process's blur, bias and tolerances, and the flow model's error, would be found. The tool does not yet read such measurements back into a process file.

## Wafers

A wafer file names designs and how many columns and rows of each to place:

```toml
name = "pilot-wafer"
diameter_mm = 100.0
edge_exclusion_mm = 5.0
street_mm = 1.0

[[chips]]
design = "mother_machine_manifold.py"
columns = 3
rows = 2
```

`microfluidics wafer designs/pilot_wafer.toml runs/pilot_wafer` steps each design's masks across the wafer, using its tuned values. Each chip is labelled below its outline in the deepest layer, with ticks at its corners for dicing, and two crosses on the shallowest layer align the wafer. Trenches remain arrays: the six-chip wafer's `chip.gds` is 138 kB, against 2.2 MB for the flat drawing of its trench layer. A chip that would extend into the edge exclusion is rejected by name.

## Agreement with KLayout

KLayout is the layout tool that cleanrooms and mask shops use to check files. The tool does not depend on it, but tests have KLayout read the exported files and compare:

- It reads the GDSII as the same structures, outline counts and areas.
- Its difference between each DXF drawing and the corresponding GDSII layer is empty.
- On a design seeded with features that are too narrow or too close, its width and space checks find exactly as many violations as the rule check here.

```console
python3 -m pip install klayout
KLAYOUT_PYTHON=python3 pytest -m klayout
```

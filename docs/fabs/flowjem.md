# FlowJEM

FlowJEM, in Toronto, makes SU-8 masters and PDMS chips to order. A design reaches it by email, with a price request form. This page covers what FlowJEM publishes, what the process [`flowjem-su8-pdms`](../../processes/flowjem-su8-pdms.toml) assumes beyond that, and how to send an example. Everything here was read from FlowJEM's site on 2026-11-23; nothing has been ordered.

## What FlowJEM publishes

| Limit | Published | In the process as |
| --- | --- | --- |
| Smallest feature and gap | "≥ ~5 um" | `feature.width` and `feature.space`, 5 um |
| Feature height | "A few to 500 um" | `layer.height`, 2 to 500 um; the 2 is a reading of "a few" |
| Height tolerance | Within about 15 % of the channel height, or 10 % with its premium service | `height_sigma_fraction = 0.05`, taking 15 % as three standard deviations |
| Wafer | 4 inch or 6 inch, with a usable circle of about 3.5 or 5.2 inches | Not a rule; a wafer file's `edge_exclusion_mm` holds it |
| Punched holes | 0.75 mm, or 0.90 mm, which it says is usually preferred | `roof.span`'s `punch_um = 750` |
| Layers | Multi-layer SU-8 is offered | `layer.count`, 2, assumed |

Its rules for a design file, and how the exported drawings meet them:

| FlowJEM asks for | The drawing |
| --- | --- |
| DXF or DWG | DXF, one file for each layer |
| Closed polylines of zero width, no hatching | Every outline is a closed polyline |
| Boundaries that do not overlap or cross themselves; one that touches itself is allowed | Features that touch are one outline, and a hole is reached along a cut |
| Millimetres or micrometres | Micrometres |
| Data near the origin | The chip's lower left corner is the origin |
| Masks "right reading, emulsion side down", with channels transparent | `tone = "clear"`, unmirrored; `ORDER.md` states the polarity in those words |

## What the process assumes

FlowJEM states nothing of these, so the process holds cautious values and lists each under `assumed`:

| Limit | Assumed | Why |
| --- | --- | --- |
| `layer.overlap` | 30 um | The alignment tolerance another foundry publishes for printed film masks |
| `resist.aspect` | 2 | A common limit for SU-8 walls |
| `roof.span` | 12 | A common limit before a PDMS roof sags |
| `port.pitch`, `port.edge` | 2.5 mm, 2 mm | Room for tubing and for the slab's edge |
| `chip.size` | 50 x 24 mm | A cover glass |
| `layer.count` | 2 | Multi-layer is offered without a stated limit |

Width tolerances are assumed too. The process states no print model, so the `print` check does not run against it.

## Which examples it can make

| Example | At FlowJEM |
| --- | --- |
| [`test_channel`](../../designs/test_channel.py), [`mixer`](../../designs/mixer.py), [`goenner_mixer`](../../designs/goenner_mixer.py) | Within every limit |
| [`mother_machine`](../../designs/mother_machine.py) | Not declared for it: its trenches are 1 um wide, under the 5 um FlowJEM makes |

## Sending a design

```console
$ microfluidics export designs/goenner_mixer.py out
```

1. Read `out/flowjem-su8-pdms/REPORT.md`. Send nothing that does not pass.
2. Read `out/flowjem-su8-pdms/ORDER.md`. It lists the drawing to send with its hash, the layer's height, the mask polarity, the slab and the glass, and the position of every hole.
3. Fill in FlowJEM's [price request form](https://www.flowjem.com/price-request-form.html) from the order sheet, and add the quantity.
4. Email the form and `out/flowjem-su8-pdms/masks/mask_<n>_<layer>.dxf`, with the questions the order sheet lists. FlowJEM's [ordering page](https://www.flowjem.com/placing-an-order.html) gives the address.
5. When FlowJEM answers a question, put the answer into a copy of the process file beside the design and take the limit out of `assumed`.

FlowJEM publishes no price, lead time, layer-to-layer alignment tolerance, glass thickness or shipping terms.

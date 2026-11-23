# Examples

The four designs in [`designs/`](../designs/), what each demonstrates, which fab each can be made at, and what verification reports for it. The operating values and requirements in them are examples, not values chosen for a strain or an assay.

| Design | [FlowJEM](fabs/flowjem.md) | [COSINC](fabs/cosinc.md) | As written | After closure |
| --- | --- | --- | --- | --- |
| [`test_channel`](../designs/test_channel.py) | Yes | Yes | Passes | Nothing to tune |
| [`mixer`](../designs/mixer.py) | Yes | Yes | Passes: three liquids in shares of 50 %, 30 % and 20 % | Unchanged |
| [`goenner_mixer`](../designs/goenner_mixer.py) | Yes | Yes | Does not pass: shares of 88.76 %, 11.16 % and 0.0798 % | Passes, corners included: shares of 89.18 %, 10.74 % and 0.07995 % |
| [`mother_machine`](../designs/mother_machine.py) | No: its trenches are under 5 um | Yes, on assumed limits | Passes | Nothing to tune |

A design is declared for a fab by naming the fab's process in its `processes`, and passes only when it passes at every one. COSINC has published none of its limits, so passing there means passing against assumptions; each report and each [hand-off sheet](fabrication.md#hand-off-sheets) says so.

The tuned values for the closed design are kept beside it as `goenner_mixer.tuned.toml`. [`designs/mother_machine_wafer.toml`](../designs/mother_machine_wafer.toml) places eight copies of the mother machine on a 100 mm wafer.

## Test channel

One straight channel between two pads in one 20 um layer, with a line monitor. Its flow at a known pressure gives the layer's real height, which makes it the design to fabricate first when characterising a process.

## Mixer

The small design to start from: three inlets, each with a serpentine of three runs, a junction, and a longer serpentine to mix in. Every channel has the same width, so the length of each path to the junction sets its share: 50 %, 30 % and 20 %. Because the three paths differ only in length, the shares hold however the chip is fabricated.

## Goenner mixer

The calcium-assay mixer of Goenner et al. (2025), whose targets are 89.2 % buffer, 10.72 % reagent and 0.08 % sample from one inlet pressure. Their automated flow reported mixing errors of 0.17 %, 1.21 % and 8.08 % after routing, with a length added by hand to compensate.

Here closure changes two serpentine lengths, and the routed design's shares are 89.18 %, 10.74 % and 0.07995 %: within 0.02 %, 0.2 % and 0.06 % of their targets, with no manual step. Both sets of figures come from a network model. Theirs measured 9.2 % error on a printed chip against 2.24 % simulated.

The process corners show what holds those shares on a fabricated chip. Every channel from a pad to the junction's outlet has one width in one layer, so the three resistances differ only by length: the buffer's path is a millimetre, and the sample's is about a thousand times that, folded into a serpentine. A layer that comes out thick or thin, or features that come out wide or narrow, changes all three alike. Across the corners the sample's share stays between 0.0799 % and 0.0801 %, against the 0.0792 % to 0.0808 % allowed.

A mixer that meters with channels of different widths does not have this property, because a narrow channel responds to a width error far more than a wide one. With a 27 um channel for the sample and a 500 um channel for the buffer, the same targets are met as drawn, and the sample's share ranges from 0.057 % to 0.112 % across the corners.

## Mother machine

A two-layer mother machine for *Escherichia coli*, laid out as the chip of James and Kempf et al. (2025) is in their Figure S1: three separate lanes on one slab, each a straight channel between its own inlet and outlet with growth trenches off both sides, on a 24 x 50 mm coverslip 0.17 mm thick. It carries two alignment marks and a process monitor. Its 7,998 trenches are stored as six rows of 1,333.

| Property | Value | From |
| --- | --- | --- |
| Three lanes, each with its own inlet and outlet | | Figure S1 of the paper |
| Slab, lane spacing, hole-to-hole distance | 14.5 x 20 mm, 4.2 mm, 13.2 mm | Measured from Figure S1's photograph against the 24 x 50 mm coverslip |
| Trenches | About 8,000 in all | The paper's text |
| Trench pitch | 2.6 um | The paper's 400 trenches in a 0.52 mm field of view, over the two sides of one lane |
| Holes, coverslip | 0.75 mm punch; No. 1.5H | The paper's methods |
| Trench | 1 um wide, 25 um long, 1.1 um deep | Assumed |
| Channel | 100 um wide, 25 um deep | Assumed |
| Trenches per lane | 1,333 a side | Assumed: the 8,000 shared equally among the lanes |
| Flow | 20 uL/min in each lane | The paper's methods |

The paper gives no cross-section of a trench or of a channel, and its master was a gift from another laboratory. The assumed values are common ones for mother machines of this organism, and are not that chip's.

At 20 uL/min the medium in each channel is replaced in 0.09 s. The Reynolds number in a channel of the assumed section is 7.6, and inertia loses 0.03 % of the pressure that drives the chip, so the flow model stands. Across COSINC's assumed corners the time to replace the medium ranges from 0.08 s to 0.11 s.

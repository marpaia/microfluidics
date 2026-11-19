# Examples

The five designs in [`designs/`](../designs/), what each demonstrates, and what verification reports for it. The operating values and requirements in them are examples, not values chosen for a strain or an assay.

| Design | As written | After closure |
| --- | --- | --- |
| [`test_channel`](../designs/test_channel.py) | Passes | Nothing to tune |
| [`mixer`](../designs/mixer.py) | Passes: three liquids in shares of 50 %, 30 % and 20 % | Unchanged |
| [`goenner_mixer`](../designs/goenner_mixer.py) | Does not pass: shares of 88.76 %, 11.16 % and 0.0798 % | Passes, corners included: shares of 89.18 %, 10.74 % and 0.07995 % |
| [`mother_machine_pilot`](../designs/mother_machine_pilot.py) | Passes | Nothing to tune |
| [`mother_machine_manifold`](../designs/mother_machine_manifold.py) | Does not pass: the middle lanes get more than twice the flow of the outer ones | Passes, corners included: lane flows within 2.7 % of their mean |

The tuned values for the two closed designs are kept beside them as `<design>.tuned.toml`. [`designs/pilot_wafer.toml`](../designs/pilot_wafer.toml) places six copies of the manifold design on a 100 mm wafer.

## Test channel

One straight channel between two pads, with a process monitor. Its flow at a known pressure gives the flow layer's real height, which makes it the design to fabricate first when characterising a process.

## Mixer

The small design to start from: three inlets, each with a serpentine of three runs, a junction, and a longer serpentine to mix in. Every channel has the same width, so the length of each path to the junction sets its share: 50 %, 30 % and 20 %. Because the three paths differ only in length, the shares hold however the chip is fabricated.

## Goenner mixer

The calcium-assay mixer of Goenner et al. (2025), whose targets are 89.2 % buffer, 10.72 % reagent and 0.08 % sample from one inlet pressure. Their automated flow reported mixing errors of 0.17 %, 1.21 % and 8.08 % after routing, with a length added by hand to compensate.

Here closure changes two serpentine lengths, and the routed design's shares are 89.18 %, 10.74 % and 0.07995 %: within 0.02 %, 0.2 % and 0.06 % of their targets, with no manual step. Both sets of figures come from a network model. Theirs measured 9.2 % error on a printed chip against 2.24 % simulated.

The process corners show what holds those shares on a fabricated chip. Every channel from a pad to the junction's outlet has one width in one layer, so the three resistances differ only by length: the buffer's path is a millimetre, and the sample's is about a thousand times that, folded into a serpentine. A layer that comes out thick or thin, or features that come out wide or narrow, changes all three alike. Across the corners the sample's share stays between 0.0799 % and 0.0801 %, against the 0.0792 % to 0.0808 % allowed.

A mixer that meters with channels of different widths does not have this property, because a narrow channel responds to a width error far more than a wide one. With a 27 um channel for the sample and a 500 um channel for the buffer, the same targets are met as drawn, and the sample's share ranges from 0.057 % to 0.112 % across the corners.

## Mother-machine pilot

A two-layer mother machine: eight independent lanes, each a feed channel with 100 growth trenches per side between its own inlet and outlet pads, with two alignment marks and a process monitor. Its 1,600 trenches are stored as sixteen rows. The trench and channel sizes are the `TrenchLane` region's defaults, not values tuned to a strain or a process.

## Mother-machine manifold

The same eight lanes fed from one inlet through a manifold and gathered by a second. As written, the lanes are unbalanced. Closure balances them by bringing the lanes to a 0.4 mm pitch, narrowing each lane's feed channel to 40 um and widening the manifold trunks to 240 um. The pitch and the lane width end at the limits of their ranges, so the design has no margin left in those two parameters.

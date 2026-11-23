# COSINC

COSINC (Colorado Shared Instrumentation in Nanofabrication and Characterization) is the shared cleanroom of the University of Colorado Boulder, in SEEL 168 at 4001 Discovery Drive. A person makes a chip there by hand, on tools booked by the hour. This page covers what COSINC publishes, what the process [`cosinc-su8-pdms`](../../processes/cosinc-su8-pdms.toml) assumes, and how to take an example there. Everything here was read from COSINC's site on 2026-11-23; no chip has been made there.

## What COSINC publishes

COSINC publishes its tools and its rates, and no limit on what the tools make in SU-8.

| Step | Tool | Stated |
| --- | --- | --- |
| Exposure without a mask | Heidelberg DWL 66FS laser writer | "Sub-micron precision"; slated to be replaced in 2026 |
| Exposure without a mask | Raith Picomaster XF200 laser writer | 0.6 um resolution; listed as "coming 2026" |
| Exposure with a mask | OAI 806 mask aligner | Wafers up to 6 inches; "submicron alignment accuracy", with no figure |
| Coating and baking | Laurell WS-650MZ spin coaters, programmable hotplates | 100 to 6,000 rpm; one coater for general use |
| Measuring | Stylus and optical profilometers, inspection microscopes | |
| Bonding | Plasma cleaner | Whether PDMS may be bonded in it is not stated |

Its safety manual (the 2019 version) lists SU8-10 and SU8-3050 among the chemicals it stocks, and lets a user bring a chemical after telling staff.

Rates are by the hour, in four columns. Training and staff assistance are charged on top of a tool's rate.

| Item | CU Boulder | External non-profit | External industry |
| --- | --- | --- | --- |
| Cleanroom induction | $45.70 | $50.27 | $114.25 |
| Training | $28.65 | $31.52 | $71.63 |
| Staff assistance | $52.00 | $57.20 | $130.00 |
| Spin coater | $36.50 | $40.15 | $91.25 |
| Solvent bench | $25.30 | $27.83 | $63.25 |
| Plasma cleaner | $35.00 | $38.50 | $87.50 |
| Stylus profilometer | $45.90 | $50.49 | $114.75 |

The laser writers and the mask aligner have no published rate.

## What the process assumes

Every limit in the process is assumed: all ten rules, the tolerances and the print model. The values are cautious rules of thumb for a two-layer SU-8 master, with the smallest feature and gap at 1 um. Two things in particular are not confirmed:

- **1 um features in SU-8.** The tools are stated as sub-micron. No one states a 1 um line in a 1.1 um negative resist, and the stocked SU-8 grades are for layers of about 10 um and 50 um.
- **Aligning one layer to another within 3 um.** No overlay figure is published.

The cheapest proof of both is a test exposure of a `ProcessMonitor`: its lines step from 1.0 um to 1.8 um, and its verniers read the offset between the layers in steps of 0.5 um.

## Which examples it can make

All four examples are declared for COSINC. The three one-layer chips ask nothing unusual of it. [`mother_machine`](../../designs/mother_machine.py) needs both of the unconfirmed things above.

## Making a chip there

```console
$ microfluidics export designs/mother_machine.py out
```

1. Ask the questions `out/cosinc-su8-pdms/TRAVELER.md` lists, at `cosinc-fab@colorado.edu`, before booking anything.
2. Open an account in COSINC's scheduler, linked from its [rates page](https://www.colorado.edu/facility/cosinc/resource-scheduler-rates), and take the cleanroom induction and the training for each tool.
3. Take `out/cosinc-su8-pdms/masks/chip.gds` and a printed `TRAVELER.md`. The traveler gives each layer's steps in the order they are done, the centre of each alignment mark, and the position of every hole.
4. Write what was measured into the traveler: each layer's height, and the widths it asks for.
5. Put what was learned into a copy of the process file beside the design, and take each confirmed limit out of `assumed`.

The traveler's steps are those of the Jun lab's mother machine handbook (version 2.00, 2010), written for another cleanroom's tools. Each resist, spin speed, bake and exposure is a starting point to be found again on COSINC's tools.

The tool does not yet read measurements back into a process file.

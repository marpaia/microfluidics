# Verification

This page describes how the tool decides whether a design passes: the checks it runs, how it compares stated connections with drawn geometry, the flow model, requirements, closure, process variation, and the report. Fabrication checks (design rules, masks and printing) are covered in [fabrication](fabrication.md).

## Checks

Everything that verifies a design returns the same thing, a check. A check has a status (satisfied, violated, waived or not run), the findings it made, and the quantities it measured. Nothing else decides whether a design passes.

Verification runs the checks in a fixed order. Each answers one of four questions:

| Question | Check | Verifies |
| --- | --- | --- |
| **Fabrication:** can it be made? | `rules` | The chip as drawn is within the process's design rules |
| | `mask-rules` | The prepared masks are within the rules, overlap rules included |
| | `print` | Every narrow feature prints at its drawn width, without vanishing or merging with a neighbour |
| **Function:** does it do what is asked? | `connectivity` | The connections the design states are the connections its geometry makes |
| | `graph` | Flow is conserved at every vertex and is slow enough for the model |
| | `requirement/<name>` | Each requirement holds |
| **Variation:** does it still, as fabrication varies? | `corners` | Every requirement holds at every corner of the process, with the range of values reached |
| | `yield` | At least the requested share of chips meets every requirement |
| **Integrity:** is the tool's own output sound? | `deterministic` | The design builds the same chip twice from the same values |
| | `gds-readback`, `dxf-readback` | The mask files, read back from disk, hold exactly the outlines planned |

A design passes only when every check is satisfied or waived. A check that did not run counts against it. A report's problem count is the number of findings that are not waived plus the number of checks that did not run.

## Connectivity

A region states how fluid passes through it as edges between its ports and internal vertices. Each edge is a series of ducts, and each duct is a layer, a width and a length. `route` draws a channel between two ports along X and Y, through any points given, and that channel is an edge with the length it was actually routed to.

The connectivity check compares what the design states with what its geometry does, as layout-versus-schematic does for an integrated circuit:

| Finding | Raised when |
| --- | --- |
| `open` | Ports placed together, or the two ends of an edge, lie in features that do not touch |
| `short` | Features touch that the design does not connect: two nets, or two regions anywhere other than a port that joins them |
| `dangling` | A port has nothing connected to it |
| `closed` | A body of fluid reaches fewer than two access points |

The check also identifies which ports are placed together. Those become the vertices of the flow graph.

## Flow model

A design states how it is run with `operate`: the viscosity, a pressure or a flow rate at each access point, and optionally what is fed at each inlet. The [API guide](../python/microfluidics/guide.md) lists the arguments.

The flow graph is extracted from what the design states, never from its outlines. Every region's edges and every routed channel become edges of the graph, and ports placed together become one vertex. The connectivity check is what justifies this. Each edge's resistance is the sum of its ducts' resistances, computed from the exact series solution for a rectangular duct. The graph is solved for the pressure at every vertex and the flow in every edge, and fed species mix at each vertex in proportion to flow.

The model assumes steady, slow, fully developed flow. Bends, junctions and entrance lengths add no resistance. The `graph` check reports a finding when any edge's Reynolds number exceeds 1.

## Requirements

A requirement is a statement, a measure and a bound:

```python
self.balanced = mf.Requirement("Every lane gets the same flow", mf.flow(self.lane), mf.equal_within(5 * pct))
```

Measures include flow rate, wall shear, pressure, pressure drop, velocity, Reynolds number, residence time, the fraction of a fed species, and chip yield. Bounds are `at_most`, `at_least`, `target` with a tolerance, and `equal_within`. A bound must have the dimension of its measure, which the type checker enforces. The [API guide](../python/microfluidics/guide.md) lists each.

## Closure

A design marks parameters as tunable with `tune`, each within its declared range. `close` changes those parameters until every requirement holds.

Each trial is the whole design built again: checked against the design rules and its connections, with its network extracted and solved. The search never moves to a trial that breaks a rule or a connection, so the result is a design that can still be made.

The search is bounded least squares by the Levenberg-Marquardt method, with derivatives taken by stepping each parameter. A target value pulls all the way to the target. A limit or a spread pulls only until it holds with half its tolerance to spare. The search prefers to stay near its starting point. Tuned lengths lie on the masks' 1 nm grid, and a parameter that is a count moves in whole numbers.

Closure never rewrites source. It writes `<design>.tuned.toml` beside the design, and every later command reads that file.

When the requirements cannot all be met, closure reports which requirement is furthest from holding and what each tunable parameter does to it: which direction would help, which is at the end of its range, and which cannot move without breaking a named rule.

## Corners and yield

A process may state how far a fabricated chip departs from its drawing, as standard deviations of each layer's height, of every width on a mask together, and of each feature's width alone. The flow graph is extracted and solved again under each variation. The layout is not redrawn.

- **Corners** take each layer at its thickest and thinnest and every feature at its widest and narrowest, three standard deviations out, in every combination. Every requirement must hold at every corner.
- **Yield** draws 500 chips at random from a fixed seed, with every tolerance varying, and reports the share that meets every requirement. It runs when a design has a requirement on `mf.chip_yield()`.

`microfluidics corners` and `microfluidics yield` run each on its own.

## The report

`export` writes `REPORT.md` and `report.json`. Each lists every check with its status, its findings and what it measured. The report also carries a hash of every input: each source file read (the design, any regions beside it, and the standard regions it places), the process, the tuned values, and the elaborated design.

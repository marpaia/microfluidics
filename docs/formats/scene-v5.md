# Scene format v5

A scene is one JSON document that holds everything a viewer draws for a design: its geometry, its flow, its checks and where each thing was written. `Project(path).scene()` returns one, and a [session](../protocols/session.md) sends one after every build. The format carries its own `version`, currently 5.

## Contents

| Field | Contents |
| --- | --- |
| `version`, `design`, `passes`, `problems` | The format version, the design's name, whether it passes, and how many problems it has |
| `size_um`, `thickness_um`, `glass_um`, `punch_um` | The slab, the glass it is bonded to, and the diameter of a punched hole when the first process the design names states one |
| `layers` | Each layer with its name, height, GDSII number and colour |
| `shapes`, `translates`, `outlines` | Every rectangle. A row of identical rectangles is one entry in `translates` that places an outline stated once in `outlines` |
| `ports`, `access` | Where channels leave regions, and where tubing reaches the chip |
| `flows`, `pressures_mbar` | The flow through each edge of the graph, and the pressure at each vertex |
| `instances` | Every placed instance of a region by path: its region, the bounding box of what it made, its parameters as written, and the line that placed it |
| `drives` | What is applied at each access point, and where |
| `tunables` | Each parameter a slider can move, with its range |
| `requirements` | Each requirement: what was measured, the unit, what the bound allows in that unit, and how far it is from holding at the worst corner of the first process the design names |
| `checks`, `findings` | Every check by id and title, with the `process` it was made against or `null` when it holds whatever makes the chip; and each finding with its check's id and process, the paths it concerns and the line that made the first of them |
| `origins` | For each named thing, the file and line that made it |

## Conventions

- Lengths are micrometres.
- A design that names two processes has two checks of one id, such as `rules`. The id and the process together tell one check from another, and a finding belongs to the check that has both.
- Because rows of identical rectangles stay rows, a chip with a hundred thousand trenches has twenty `translates` entries.
- No layer colour is red. A viewer reserves red for what is wrong.
- Every named thing carries the line that made it: the line that called `rectangle`, `port`, `route`, `at` or `attach`. What a standard region draws points into that region's own file under `python/microfluidics/regions/`.

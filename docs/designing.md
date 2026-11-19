# Designing

Designs and regions are typed Python classes. This page summarises how they are written, describes the standard regions, and lists the mistakes the type checker catches. Everything a design makes is named after the attribute it is assigned to, and every design is assembled by one builder in the engine.

- **Guide:** [`python/microfluidics/guide.md`](../python/microfluidics/guide.md), which `microfluidics guide` prints.
- **Designs:** those in [`designs/`](../designs/) are worked examples. [`designs/mixer.py`](../designs/mixer.py) is a small one to start from.
- **Regions:** [`python/microfluidics/regions/`](../python/microfluidics/regions/) holds the standard regions, one per file. They are written the same way as a region of your own, so they double as examples. `microfluidics describe <region>` prints a region's parameters, ports and source.
- **Terminology:** [`reference/terminology.md`](reference/terminology.md) defines each word the project uses and gives its source.

## Syntax at a glance

| Thing | Written |
| - | - |
| A region | `class TrenchLane(mf.Region)` with `build(self)` |
| A parameter | `width: mf.Param[Length] = mf.param(default=100 * um, low=20 * um, high=500 * um)` |
| Its ports | `inlet: mf.Port = mf.port()` in the class body; a row is `tap: list[mf.Port] = mf.ports()` |
| Naming | `self.inlet = self.port(...)` |
| A row | `self.lane = [... for ...]` |
| A rectangle | `self.rectangle(FLOW, a, b)` |
| Translates | `self.translates(TRENCH, a, b, by=v, count=n)` |
| An edge | `self.edge(a, b, mf.Duct(FLOW, w, l))` |
| Placing | `Manifold().at(x, y, turned=1)` |
| Attaching | `Manifold().attach(lambda m: m.inlet, to=port, offset=d)` |
| A channel | `self.route(FLOW, w, a, b)` |
| Running it | `self.operate(viscosity=v, drive={self.inlet: p})` |
| A requirement | `self.gentle = mf.Requirement("...", mf.shear(self.lane), mf.at_most(1 * Pa))` |
| Tuning | `self.tune(self.feed, Manifold.trunk_width)` |
| A fault of its own | `mf.ensure(holds, "why")` |

## The standard regions

Eleven regions come with the package: `AccessPad`, `AlignmentMark`, `Junction`, `LineMonitor`, `Manifold`, `ProcessMonitor`, `Serpentine`, `StraightChannel`, `TrapChamber`, `TrenchLane` and `Vernier`. Each is an ordinary `mf.Region`: the engine treats it exactly as it treats a region written beside a design. They draw on two layers, `TRENCH` and `FLOW`, which a design that places them must have.

[`tests/golden/regions/`](../tests/golden/regions/) holds the geometry each region builds for several parameter sets, and a test compares each region against it.

## What the types catch

`tests/typing/refuse.py` holds each of these with the error it must raise, and a test fails if the checker accepts any: a misspelt parameter, a pressure given for a length, a count given a length, a port that does not exist, a region given where a port is wanted, a bound of another dimension than its measure, a string where a direction is wanted, and a sum of a length and a pressure. Everything the types catch is also checked when a design is built.

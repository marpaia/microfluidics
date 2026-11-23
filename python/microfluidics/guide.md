# Designing with microfluidics

A design is a Python file holding one class. This is what it is written with; `docs/reference/terminology.md` in the repository says what each word means and where it comes from.

## A design

```python
import microfluidics as mf
from microfluidics.layers import FLOW, TRENCH
from microfluidics.processes import COSINC_SU8_PDMS
from microfluidics.regions import AccessPad, Manifold, TrenchLane
from microfluidics.units import Length, Pa, mbar, mm, mPa_s, pct, um


class Lanes(mf.Design):
    """One inlet feeding eight lanes of growth trenches."""

    processes = (COSINC_SU8_PDMS,)
    slab = mf.Slab(24.2 * mm, 21 * mm, thickness=4 * mm)
    glass = mf.Glass(60 * mm, 24 * mm, thickness=0.17 * mm)
    layers = mf.Stack({TRENCH: mf.LayerSpec(height=1.1 * um, gds=1), FLOW: mf.LayerSpec(height=20 * um, gds=2)})

    pitch: mf.Param[Length] = mf.param(default=2.6 * mm, low=0.4 * mm, high=2.6 * mm)

    def build(self) -> None:
        self.inlet = AccessPad().at(12.1 * mm, 2 * mm)
        self.feed = Manifold(taps=8, pitch=self.pitch).attach(lambda m: m.inlet, to=self.inlet.opening)
        self.lane = [
            TrenchLane(trenches_per_side=100).attach(lambda lane: lane.inlet, to=tap, offset=500 * um)
            for tap in self.feed.tap
        ]
        self.riser = [self.route(FLOW, 100 * um, tap, lane.inlet) for tap, lane in zip(self.feed.tap, self.lane)]
        self.drain = Manifold(taps=8, pitch=self.pitch).attach(
            lambda m: m.tap[-1], to=self.lane[0].outlet, offset=500 * um
        )
        self.faller = [
            self.route(FLOW, 100 * um, lane.outlet, tap) for lane, tap in zip(self.lane, reversed(self.drain.tap))
        ]
        self.outlet = AccessPad().attach(lambda pad: pad.opening, to=self.drain.inlet)

        self.operate(viscosity=0.7 * mPa_s, drive={self.inlet: 10 * mbar, self.outlet: 0 * mbar})
        self.balanced = mf.Requirement("Every lane gets the same flow", mf.flow(self.lane), mf.equal_within(5 * pct))
        self.gentle = mf.Requirement("Cells are not sheared off", mf.shear(self.lane), mf.at_most(1 * Pa))
        self.tune(Lanes.pitch)
        self.tune(self.feed, Manifold.trunk_width)
```

- **The class body states the chip:** its `processes` (a tuple: the chip is held to the rules of every process it names), its `slab` (width, depth, thickness), the `glass` the slab is bonded to (width, depth, thickness; the slab sits at its middle and must fit on it), its `layers` (each with a height and a GDSII number), and any parameters of its own.
- **`build` places what is on it.** A design file holds exactly one `mf.Design` subclass.

## Names come from attributes

Whatever is made is named by assigning it to an attribute of `self`.

- `self.feed = Manifold(...)...` places a region named `feed`.
- Assigning a list names its elements: `self.lane = [...]` makes `lane[0]`, `lane[1]`, and so on.
- Something made and never assigned is a fault, on the line that made it.
- A name that every region already uses for something it does (`route`, `port`, `edge`, `rectangle`, `at`, `attach`, `height`, `build`, `path`) cannot be given to a thing.

These names are the paths findings use (`lane[2].left`), and the ones tuned values are kept under (`feed.trunk_width`).

## Quantities

`100 * um` is a `Length`; `10 * mbar` is a `Pressure`. They are different types, to a type checker and when the design is built.

| Units | Type |
| - | - |
| `nm`, `um`, `mm` | `Length` |
| `Pa`, `mbar`, `kPa`, `psi` | `Pressure` |
| `ul_min`, `nl_min`, `ul_h` | `FlowRate` |
| `mPa_s`, `Pa_s` | `Viscosity` |
| `s`, `minute`, `h` | `Time` |
| `pct` | a plain number: `5 * pct` is 0.05 |

Everything drawn lies on a grid of one nanometre, the unit masks are written in: a corner given between two grid points is moved to one. Sums and differences keep their type, a quantity scales by a plain number, and the ratio of two of one kind is a plain number. A point is a pair of lengths: `(12.1 * mm, 2 * mm)`.

## Placing a region

A region is placed by where its origin goes, or by one of its ports meeting another.

| Written | Does |
| - | - |
| `AccessPad().at(x, y)` | Puts its origin at a point. `turned=1` turns it a quarter turn counterclockwise |
| `Manifold().attach(lambda m: m.inlet, to=port)` | Places it so that its port meets `port`, facing it, and joins the two |
| `.attach(..., to=port, offset=500 * um)` | Stands it off by a gap; a `route` then joins the two ports |

An `attach` joins one pair of ports. Where one region meets many, such as a manifold gathering a row of lanes, attach it by one port and `route` a channel to each of the others. A gathering manifold is the feeding one turned round, so its taps run the other way: lane `i` meets `drain.tap[-1 - i]`.

`self.route(layer, width, start, end, via=[...])` routes a channel between two ports along X and Y, through any points given.

## Parameters

```python
pitch: mf.Param[Length] = mf.param(default=2.6 * mm, low=0.4 * mm, high=2.6 * mm)
lanes: mf.Param[int] = mf.param(default=8)
```

- On an instance, `self.pitch` is the value. On the class, `Lanes.pitch` is the parameter itself.
- A value outside its range is a fault on the line that gave it.
- A parameter with a whole-number default is a count.

## How it is run, and what it must do

```python
self.operate(viscosity=0.7 * mPa_s, drive={self.inlet: 10 * mbar, self.outlet: 0 * mbar}, feed={self.inlet: {self.dye: 100 * pct}})
self.share = mf.Requirement("A tenth is dye", mf.fraction(self.dye, self.outlet), mf.target(10 * pct, within=1 * pct, relative=True))
```

- **`drive`** gives each region with an access point a pressure or a flow rate. At least one is a pressure.
- **`feed`** says what enters: `self.dye = mf.Species()` names a species.
- **A requirement** is a statement in words, a measure, and a bound of the same dimension.

| Measure | Of | Is |
| - | - | - |
| `mf.flow`, `mf.velocity`, `mf.shear`, `mf.reynolds`, `mf.residence` | Regions and channels flow passes through | Flow rate; speed, wall shear and Reynolds number in the tightest duct; time to pass |
| `mf.pressure` | A port, or a region with an access point | Pressure there |
| `mf.drop(a, b)` | Two places | Pressure lost from one to the other |
| `mf.fraction(species, at)` | A species and a place | How much of what arrives is that species |
| `mf.chip_yield()` | The whole chip | The share of chips, made within the process's tolerances, that meet every other requirement |

| Bound | Holds when |
| - | - |
| `mf.equal_within(share)` | Every value is within that fraction of their mean |
| `mf.target(value, within=..., relative=False)` | Every value is that near the target |
| `mf.at_most(value)`, `mf.at_least(value)` | Every value is no more, or no less |

## What may be tuned

```python
self.tune(Lanes.pitch)                       # the design's own
self.tune(self.feed, Manifold.trunk_width)   # a placed region's
```

Each must have a range. The second form also takes a list of regions, each then tuned on its own: `self.tune(self.lane, TrenchLane.channel_width)`. To hold several regions to one value, give them a parameter of the design and tune that.

`microfluidics close` changes them until the requirements hold and writes the result beside the design as `<design>.tuned.toml`; the design's source is never rewritten.

- **Every command reads the tuned values** when that file is there, and says so. `close` starts from the design as written, not from the file.
- **A tuned value whose parameter is gone** is a fault naming it; delete the file or run `close` again.
- **When closure stops short** it says which tunable is at the end of its range or which rule stops it. What else to change is the designer's call: a requirement that every branch gets the same flow is met by making the branches resist far more than what feeds them.

## Writing a region

```python
class Spur(mf.Region):
    """A short channel off a port."""

    length: mf.Param[Length] = mf.param(default=1 * mm, low=0.1 * mm, high=5 * mm)
    width: mf.Param[Length] = mf.param(default=100 * um, low=20 * um, high=500 * um)

    inlet: mf.Port = mf.port()
    outlet: mf.Port = mf.port()

    def build(self) -> None:
        half, zero = self.width / 2, 0 * um
        mf.ensure(self.length >= self.width, "a spur is at least as long as it is wide")
        self.body = self.rectangle(FLOW, (-half, zero), (half, self.length))
        self.inlet = self.port(at=(zero, zero), width=self.width, layer=FLOW, facing=mf.Facing.DOWN)
        self.outlet = self.port(at=(zero, self.length), width=self.width, layer=FLOW, facing=mf.Facing.UP)
        self.edge(self.inlet, self.outlet, mf.Duct(FLOW, self.width, self.length))
```

| Made with | Is |
| - | - |
| `self.rectangle(layer, start, end)` | A rectangle, by two opposite corners |
| `self.translates(layer, start, end, by=(dx, dy), count=n)` | A row of rectangles alike, held once however many there are |
| `self.port(at=, width=, layer=, facing=)` | Where a channel leaves; declare it in the class body with `mf.port()`, or a row with `mf.ports()` |
| `self.vertex()` | A place flow passes inside the region that is not a port |
| `self.access(at=...)` | Where a punched hole brings tubing |
| `self.edge(start, end, *ducts)` | How fluid passes between two ports or vertices; each `mf.Duct` is a layer, a width and a length |
| `self.stated(number)` | Something the region states about itself |

A region written in a file beside a design is imported as any module is: `from my_regions import Spur`.

## Faults and checks

A fault in a design is reported as `file:line: message`. Every command's result carries the checks it ran; a design passes only when every one is satisfied or waived.

| Command | Asks |
| - | - |
| `microfluidics regions`, `describe <region>`, `process [name]` | What there is to place, and the limits a design is made within |
| `microfluidics check <design>` | Can it be made: rules, connections, masks, how they print |
| `microfluidics solve <design>` | Its flow, and whether each requirement holds |
| `microfluidics close <design>` | Tune it until its requirements hold |
| `microfluidics corners <design>`, `yield <design>` | Does it survive the process |
| `microfluidics export <design> <directory>` | Verification, masks, report |

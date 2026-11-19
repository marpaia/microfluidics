# Terminology

The words this project uses, each with its meaning and where the meaning comes from. Mathematical words are used in their mathematical sense, following do Carmo and Lee; where engineering has a word and mathematics has none, the engineering word is kept and defined mathematically.

Sources are cited as:

- **do Carmo**: Manfredo P. do Carmo, *Differential Geometry of Curves and Surfaces*, revised edition.
- **Lee TM**: John M. Lee, *Introduction to Topological Manifolds*, second edition.
- **Lee SM**: John M. Lee, *Introduction to Smooth Manifolds*, second edition.
- **Lee RM**: John M. Lee, *Introduction to Riemannian Manifolds*, second edition.

The word *cell* is not used for anything a design is made of. In this project a cell is a living cell.

## Geometry

Everything is drawn in the plane ℝ², one copy of it for each layer. Lengths are held in metres and written to masks in whole nanometres, so every vertex of everything drawn lies on the lattice (1 nm)·ℤ².

| Word | Meaning | Source |
| - | - | - |
| **Region** | A reusable, parametric piece of a chip: a trench lane, a manifold. For each value of its parameters it is, on each layer, a finite union of regular regions. | do Carmo §2-5: "a region of S is the union of a domain with its boundary"; §4-5: a *regular region* is connected and compact with boundary "the finite union of (simple) closed piecewise regular curves which do not intersect". Lee TM ch. 6: *polygonal region* |
| **Rectangle** | A closed rectangle with sides along the axes: the one thing regions are drawn from. It is a simple region. | do Carmo §4-5: a *simple region* is homeomorphic to a disk, with boundary the trace of a simple closed piecewise regular curve. Lee SM Example 16.19 |
| **Boundary**, **interior** | Of a rectangle or a region: ∂R and Int R. | Lee TM ch. 2 |
| **Vertex** (of a boundary) | A corner: a point where the boundary curve is not regular. | do Carmo §4-5 |
| **Translates** | Finitely many images of one rectangle under repeated translation by one vector: a row of trenches. They are part of the rectangle's orbit under the group that vector generates. | Lee TM ch. 3 (orbits of a group action) |
| **Rigid motion** | A rotation followed by a translation, x ↦ b + Ax. Placing a region applies one. Here A is a whole number of quarter turns. | do Carmo §1-5; Lee RM ch. 3, the action of the Euclidean group, (b, A)·x = b + Ax |
| **J** | The quarter turn counterclockwise, J(x, y) = (−y, x). | The letter is the one the `DifferentialGeometry` Lean library uses for it |
| **Reflection** | The motion a mirrored mask applies: x ↦ (w − x₁, x₂). An isometry that is not a rigid motion. | Lee RM ch. 3 |
| **Attach** | To place one region so that a port of it meets a port of another. The two are then one space, glued along the ports. | Lee TM ch. 3 (attaching one space to another along a map; adjunction spaces) |
| **Component** | A maximal connected subset of what is drawn on the layers taken together: a body of fluid. | Lee TM ch. 4 |

## Flow

| Word | Meaning | Source |
| - | - | - |
| **Graph** | What flow is solved on: vertices joined by edges. | Lee TM ch. 5: "a CW complex of dimension less than or equal to 1 is called a graph"; ch. 10 |
| **Vertex** (of the graph) | A place flow passes: a port, an access point, or a point inside a region. Ports placed together are one vertex. | Lee TM ch. 5 |
| **Edge** | A way fluid passes between two vertices, with the ducts it passes through. A routed channel is an edge. | Lee TM ch. 5 |

*Path* is not used for an edge: in Lee TM ch. 4 a path is a continuous map from the unit interval.

## Words kept from engineering

Each has no counterpart in the sources, and is defined here in their terms.

| Word | Meaning |
| - | - |
| **Port** | A segment of a region's boundary through which fluid passes, with its width and its outward normal. |
| **Access point** | A point in the interior of a region where a punched hole reaches it from outside the plane. |
| **Layer** | One copy of the plane, with a height h. What is drawn on it stands for the product of that set with [0, h]. |
| **Design** | A region with the rectangle that contains it (its slab), the glass the slab is bonded to, how it is run, and what it must do. |
| **Glass** | The rectangle the slab is bonded to, with a thickness. The slab lies at its middle, their sides parallel, and within it. |
| **Placed** | A region placed in another: its image under a rigid motion. |
| **Duct** | One straight stretch of an edge: a layer, a width and a length. Its resistance is that of a rectangular duct of that width, the layer's height, and that length. |
| **Structure** | Rectangles stated once and placed many times: what a row of translates repeats, and what a wafer steps. The word is GDSII's own for a named group of outlines. |
| **Bias** | Moving every edge of a rectangle outward by a signed distance: passing to the parallel curve at that distance (do Carmo §1-7, in the exercises, defines the parallel curve to a plane curve). |
| **Parameter** | A coordinate of the space a region is a family over: a length, a number, or a whole number, with the interval it is held to. |
| **Process**, **mask**, **requirement**, **check** | As in fabrication and in this project's verification; they have no geometric meaning of their own. |

## As they are written

| Thing | Written |
| - | - |
| A region | `class TrenchLane(mf.Region)` |
| A rectangle | `self.channel = self.rectangle(FLOW, a, b)` |
| Translates | `self.left = self.translates(TRENCH, a, b, by=v, count=n)` |
| A vertex | `self.hub = self.vertex()` |
| An edge | `self.edge(a, b, mf.Duct(...))` |
| A routed channel | `self.riser = self.route(FLOW, w, a, b)` |
| A rigid motion | `Manifold().at(x, y, turned=1)` |
| Attaching | `Manifold().attach(lambda m: m.inlet, to=port)` |
| The standard regions | `microfluidics.regions` |

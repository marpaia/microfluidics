# Limitations

What the tool does not model, has not validated, or does not yet do. Read this before relying on a result.

## Not yet validated

- **No fabricated chip.** The flow model has not been compared with a field solver or with a measurement on a fabricated chip.
- **No mask shop.** KLayout's Python module has read the exported files in tests. Nobody has yet opened them in a layout viewer or sent them to a mask shop.
- **No fab has confirmed a process.** `flowjem-su8-pdms` holds the three limits FlowJEM publishes and assumes seven more. `cosinc-su8-pdms` assumes every limit, its tolerances and its print model, since COSINC publishes none. Nothing has been ordered from FlowJEM or made at COSINC. See [processes](fabrication.md#processes).
- **1 um features at COSINC are unconfirmed.** The mother machine passes against assumed limits. See [COSINC](fabs/cosinc.md#what-the-process-assumes).
- **The print model is not fitted.** It uses one blur and one threshold for every layer, whatever its height. Corners are not rounded, and only rectangles are biased.
- **The example designs carry example values.** Their operating conditions and requirements are not chosen for a strain or an assay.

## Flow model

- The model is a graph of straight rectangular ducts in steady, laminar, fully developed flow. Bends, junctions and entrance lengths add no resistance.
- The flow check estimates what inertia loses as one dynamic pressure for each duct an edge is stated as. A region that states a folded path as one duct, as `Serpentine` does, is counted as one, so the bends of a fast serpentine are not counted.
- There are no valves and no control layer.

## Layout

- Geometry is rectangles with sides along the axes.
- A channel is routed where the design says, through the points it gives. Nothing places regions or finds routes automatically.
- The standard regions draw on two fixed layers, `TRENCH` and `FLOW`. A design that places them must have those layers.
- An alignment mark's cross is drawn in the shallow layer's resist, about 1 um high, and may be faint under a thick second coat. The mother machine handbook's answer is a metal layer for the marks, which is not modelled.
- A hand-off sheet's steps are the process file's text. They do not follow from a layer's height: no resist, spin speed or dose is worked out.

## Closure and variation

- Closure searches from the design's starting values and can stop short of a better answer elsewhere.
- Closure tunes the chip as drawn. It does not tune for the corners or for yield.
- Nothing reads measurements of the process monitors back into a process file.

## Wafers

- A wafer's chips are stepped in blocks stacked up its middle. Nothing packs a round wafer efficiently.
- A wafer's rules are not checked beyond its chips' own.

## Tooling

- **A design is code.** The MCP server and the session run it as such, with a time limit on each command and no sandbox.
- **The MCP server** has been driven only by tests that speak its protocol. See [evidence so far](agents-and-benchmarks.md#evidence-so-far).
- **The VS Code extension** is tested in a browser harness and in an isolated VS Code instance. See [testing](development/testing.md#vs-code-extension) for what those tests cover.

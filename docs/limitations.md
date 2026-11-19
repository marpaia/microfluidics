# Limitations

What the tool does not model, has not validated, or does not yet do. Read this before relying on a result.

## Not yet validated

- **No fabricated chip.** The flow model has not been compared with a field solver or with a measurement on a fabricated chip.
- **No mask shop.** KLayout's Python module has read the exported files in tests. Nobody has yet opened them in a layout viewer or sent them to a mask shop.
- **The bundled process is a stand-in.** `conservative-su8-pdms` has rule-of-thumb limits, guessed tolerances and a guessed print model. See [processes](fabrication.md#processes).
- **The print model is not fitted.** It uses one blur and one threshold for every layer, whatever its height. Corners are not rounded, and only rectangles are biased.
- **The example designs carry example values.** Their operating conditions and requirements are not chosen for a strain or an assay.

## Flow model

- The model is a graph of straight rectangular ducts in steady, slow, fully developed flow. Bends, junctions and entrance lengths add no resistance.
- There are no valves and no control layer.

## Layout

- Geometry is rectangles with sides along the axes.
- A channel is routed where the design says, through the points it gives. Nothing places regions or finds routes automatically.
- The standard regions draw on two fixed layers, `TRENCH` and `FLOW`. A design that places them must have those layers.
- Alignment marks are drawn into the shallow layer. A separate metal layer for them is not modelled.

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

# Testing

The test layers, what each covers, and the commands that run them. CI runs them on pushes to `master` and on pull requests.

## Commands

```console
cargo test --workspace
maturin develop && pytest
mypy && pyright
(cd editors/vscode && npm install && npm run check && npm test)
```

Formatting, linting and the Python type check also run as commit hooks:

```console
pre-commit install
pre-commit run --all-files
```

The hooks run the Rust import-layout check, `cargo fmt`, `cargo clippy`, `isort`, `black`, `flake8` and `mypy`.

## Layers

| Layer | Location | Covers |
| --- | --- | --- |
| Engine unit tests | `crates/*/src` | Quantities, hashes, the rule check, routing, the print model, GDSII, the optimiser, duct resistance |
| Engine integration tests | `crates/microfluidics-network/tests`, `crates/microfluidics-layout/tests` | The solver on networks with known answers, the session's messages, and masks at scale (100,000 trenches, a wafer) |
| Python tests | `tests/` | Whole designs: captured geometry and results, closure, benchmarks, faults, connectivity, the command line, the MCP server and the session |
| Typing tests | `tests/typing/` | What a type checker must accept, and each mistake it must reject |
| Extension tests | `editors/vscode/test` | The viewer's geometry, formatting, check summaries and layout |

The engine tests build their fixtures directly on the builder, without Python. Anything that needs a whole design is tested from Python, so `maturin develop` must run before `pytest`.

## Captured results

`tests/golden/` holds results captured from the tool, which the Python tests compare against to nine significant figures:

| Directory | Contents |
| --- | --- |
| `regions/` | The geometry, ports and edges each standard region builds, for several parameter sets |
| `designs/` | Each design's geometry and every check's status and measurements, as written and as tuned |
| `closure/` | The tuned values closure reaches |
| `benchmarks/` | Each benchmark's score for its reference design |

A change that alters geometry or a measured value fails these tests. Re-capture a file only when the change is intended, and review its diff.

`tests/designs/` holds a design kept for the tests alone: eight lanes of trenches fed from one inlet through a manifold, with its tuned values and a wafer of it. It exercises what the examples do not: closure stopped by a rule, yield, and requirements that fail as written.

## Typing tests

`tests/typing/accept.py` must type-check. `tests/typing/refuse.py` holds one mistake per line with the error it must raise, and a test fails if the checker accepts any of them. [Designing](../designing.md#what-the-types-catch) lists the mistakes.

## KLayout

`tests/test_klayout.py` has KLayout read the exported mask files and compares its measurements with the tool's own, using `scripts/klayout_check.py`. The tests are skipped unless `KLAYOUT_PYTHON` names a Python that has the `klayout` module:

```console
python3 -m pip install klayout
KLAYOUT_PYTHON=python3 pytest -m klayout
```

[Agreement with KLayout](../fabrication.md#agreement-with-klayout) describes what is compared.

## VS Code extension

- `npm test` runs the unit tests of the viewer's geometry, formatting, check summaries, layout at each size, the camera, and the device's 3D solids. None of them needs anything drawn.
- `npm run test:vscode` starts a second instance of the installed VS Code with its own profile and exercises the preview there: what it shows for the designs, selection, trial values and **Keep**, a design that does not build, a window reload, and its layout at each size, zoom and theme.
- `node test/harness.mjs scene.json page.html` writes a page that shows a scene in a browser, outside VS Code, with a stand-in for the editor that answers trial values and **Keep**.

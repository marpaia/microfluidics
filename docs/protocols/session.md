# Session protocol

A session keeps one design's [scene](../formats/scene-v5.md) current for a viewer. `microfluidics session <design.py>` starts one. It exchanges JSON messages, one per line, on standard input and output, and rebuilds the scene whenever a Python or TOML file beside the design is written.

## Messages from the session

| `type` | Carries |
| --- | --- |
| `status` | `state` is `building`, `ready` (with `passes`, `problems` and `build_ms`), or `failed` (with `message`, `file` and `line`) |
| `scene` | A whole scene, and `trial`: whether it was built with trial values |
| `diagnostics` | For each source file, a list of entries with line, severity, message, check and paths |
| `result` | What a command gave: `command`, `ok` and `text` |

After each successful build the session sends `status` (building), `scene`, `diagnostics`, then `status` (ready).

## Messages to the session

| `type` | Effect |
| --- | --- |
| `rebuild` | Builds again immediately |
| `set_overrides` with `values` | Builds with those tunable parameters at trial values, over the tuned values kept beside the design. Nothing is written. `values` maps each parameter's path to its value with a unit, such as `{"pitch": "400um"}` |
| `keep` | Writes the trial values into the tuned-values file beside the design, then builds again |
| `run` with `command` | Runs `check`, `solve`, `close` or `corners` on the design. The result's `text` is what the command line prints for that command |
| `shutdown` | Ends the session |

## Failures

A design that fails to build leaves the last scene in place. The session reports what is wrong and where, and builds again when a watched file is next written. A message that is not valid JSON, or whose type the session does not know, gets a `status` of `failed` and changes nothing else.

## Implementation

The session loop is in the engine (`crates/microfluidics-network/src/session.rs`). The Python side supplies a host that reloads the design's file, builds it, and runs commands (`python/microfluidics/session.py`).

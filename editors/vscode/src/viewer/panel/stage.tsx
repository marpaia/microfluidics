// What lies over the chip: how it is shown, where the pointer is, what the colours mean, and
// what stands in the way of showing it.

import type { JSX } from "preact";

import type { Pinned } from "../arrangement";
import { number, place, unitName } from "../format";
import { legendTicks, ramp, share } from "../geometry";
import { host } from "../host";
import { legend } from "../paint";
import { MODES, fitAll, look, panelShown, pin, setDepth, setDevice, setMode, toggleLayer, toggleNames, togglePanel, zoomBy } from "../state";
import { IconButton } from "../ui/icon";
import { Choice, Menu, MenuHeading, Toggle } from "../ui/menu";
import { useSteps } from "../ui/overflow";
import { useStore } from "../ui/store";
import { Mark } from "./panel";
import { Said } from "./said";

/** What the chip's colours stand for. */
function Colours(): JSX.Element {
  const { scene, mode } = useStore();
  const flows = (scene?.flows.length ?? 0) > 0;
  return (
    <>
      <MenuHeading>Colour by</MenuHeading>
      {MODES.map((held) => (
        <Choice chosen={mode === held.mode} disabled={held.mode !== "layout" && !flows} title={held.mode !== "layout" && !flows ? "The design does not say how it is run, so there is no flow to show" : held.what} onChoose={() => setMode(held.mode)}>
          {held.mode === "layout" ? "Layer" : held.name}
        </Choice>
      ))}
    </>
  );
}

/** What is drawn, and where the panel is. */
function Viewing(): JSX.Element | null {
  const { scene, hidden, names, device, depth, pinned, arrangement } = useStore();
  if (!scene) return null;
  const placed = (to: Pinned | undefined, name: string) => <Choice chosen={pinned === to} onChoose={() => pin(to)}>{name}</Choice>;
  return (
    <>
      <MenuHeading>Layers</MenuHeading>
      {scene.layers.map((layer) => (
        <Toggle on={!hidden.has(layer.name)} onToggle={() => toggleLayer(layer.name)}>
          <span class="swatch" style={{ background: layer.colour }} />
          {layer.name}
          <span class="quiet end">{number(layer.height_um)} µm</span>
        </Toggle>
      ))}
      <Toggle on={names} onToggle={toggleNames}>Names on the chip</Toggle>
      {device && (
        <>
          <MenuHeading>Depth {depth > 1 && <span class="warn">not to scale</span>}</MenuHeading>
          <label class="menu-range" title="How many times their true height channels stand. Nothing else is changed.">
            <input type="range" aria-label="How many times their true height channels stand" min="0" max="2" step="0.01" value={Math.log10(depth)} onInput={(event) => setDepth(Math.round(10 ** Number(event.currentTarget.value)))} />
            <span>{depth}×</span>
          </label>
        </>
      )}
      {arrangement !== "sheet" && (
        <>
          <MenuHeading>Panel</MenuHeading>
          {placed(undefined, "Where it fits")}
          {placed("right", "Right")}
          {placed("below", "Below")}
          {placed("hidden", "Hidden")}
        </>
      )}
    </>
  );
}

/** The controls over the chip's upper edge. What does not fit gives way: first the name of what
 * colours the chip, then the two menus become one. */
function Toolbar(): JSX.Element {
  const state = useStore();
  const { mode, device, spaceless, refusal, depth, arrangement } = state;
  const { held, steps } = useSteps(3);
  const shown = panelShown(state);
  const coloured = MODES.find((each) => each.mode === mode)!;
  return (
    <div class="toolbar" ref={held}>
      <div class="segmented" role="radiogroup" aria-label="How the chip is drawn">
        <button role="radio" aria-checked={device} disabled={spaceless} title={spaceless ? `Three dimensions cannot be drawn here: ${refusal ?? "it did not start"}` : "The device, in three dimensions: drag to turn it, Shift-drag to carry it"} onClick={() => setDevice(true)}>3D</button>
        <button role="radio" aria-checked={!device} title="The layout, flat" onClick={() => setDevice(false)}>Flat</button>
      </div>
      {steps < 3 ? (
        <>
          <Menu icon="symbol-color" label={mode === "layout" ? "Layer" : coloured.name} compact={steps >= 1} title={`Colour by: ${mode === "layout" ? "layer" : coloured.name.toLowerCase()}`}><Colours /></Menu>
          <Menu icon="eye" label="View" compact={steps >= 2} title="Layers, names, depth and where the panel is"><Viewing /></Menu>
        </>
      ) : (
        <Menu icon="ellipsis" title="Colour, layers, names, depth and where the panel is"><Colours /><Viewing /></Menu>
      )}
      {device && depth > 1 && steps < 2 && <span class="chip-note warn" title={`Channels stand ${depth} times their true height`}>depth {depth}×</span>}
      <span class="spacer" />
      <IconButton icon={arrangement === "stacked" ? "layout-panel" : arrangement === "sheet" ? "list-flat" : shown ? "layout-sidebar-right" : "layout-sidebar-right-off"} title={shown ? "Hide the panel" : "Show the panel"} pressed={shown} onClick={togglePanel} />
    </div>
  );
}

function Legend(): JSX.Element | null {
  const state = useStore();
  const held = state.scene ? legend(state.scene, state) : undefined;
  if (!held || state.failure) return null;
  const stops = Array.from({ length: 9 }, (_, at) => ramp(at / 8)).join(", ");
  const ticks = legendTicks(held.low, held.high, held.log);
  return (
    <div class="legend" title={`${MODES.find((each) => each.mode === state.mode)?.what ?? ""}, in ${unitName(held.unit)}${held.log ? ", on a logarithmic scale" : ""}`}>
      <span class="bar" style={{ background: `linear-gradient(to right, ${stops})` }} />
      <span class="ticks">
        {ticks.map((tick, at) => <span class={at > 0 && at < ticks.length - 1 ? "mid" : ""} style={{ left: `${(share(tick, held.low, held.high, held.log) * 100).toFixed(1)}%` }}>{number(tick, 2)}</span>)}
      </span>
      <span class="unit">{unitName(held.unit)}{held.log ? ", logarithmic" : ""}</span>
    </div>
  );
}

function Notice(): JSX.Element | null {
  const { failure, scene, building, outdated } = useStore();
  if (outdated) {
    return (
      <div class="notice" role="alert">
        <strong>The extension was updated.</strong>
        <span>This preview was opened by the version before. Reload the window to finish the update.</span>
        <span class="buttons"><button class="primary" onClick={() => host.postMessage({ type: "reload" })}>Reload the window</button></span>
      </div>
    );
  }
  if (failure) {
    const { file, line, message } = failure;
    return (
      <div class="notice bad" role="alert">
        <strong>{scene ? "The design does not build. Showing the last one that did." : "The design does not build."}</strong>
        <pre><Said text={message} /></pre>
        <span class="buttons">
          {file && <button class="primary" onClick={() => host.postMessage({ type: "open", file, line })}>Go to {place(file, line ?? 1)}</button>}
          <button onClick={() => host.postMessage({ type: "log" })}>Show the log</button>
        </span>
      </div>
    );
  }
  if (!scene) return <div class="notice quiet-notice">{building ? "Building the design…" : "Nothing to show yet."}</div>;
  return null;
}

/** Where the pointer is; or, until something has been selected, how to select. */
function Readout(): JSX.Element | null {
  const { pointer, hover, hinted, scene, device } = useStore();
  if (!scene) return null;
  if (!pointer) return hinted ? null : <div class="readout hint">Click to select. {device ? "Drag to turn, scroll to zoom." : "Drag to move, scroll to zoom."}</div>;
  return (
    <div class="readout">
      <span>{number(pointer[0] / 1000, 5)}, {number(pointer[1] / 1000, 5)} mm</span>
      {hover !== undefined && <code>{hover}</code>}
    </div>
  );
}

/** The controls over the chip's lower edge: from where it is seen, and how near. */
function Seeing(): JSX.Element {
  const { device } = useStore();
  const { held, steps } = useSteps(1);
  return (
    <div class="seeing" ref={held}>
      <span class="spacer" />
      {device && steps === 0 && (
        <div class="segmented" role="group" aria-label="From where the chip is seen">
          <button title="From straight above" onClick={() => look("top")}>Top</button>
          <button title="Tilted, from the front" onClick={() => look("tilted")}>Tilted</button>
          <button title="From beneath, through the glass" onClick={() => look("under")}>Under</button>
        </div>
      )}
      {device && steps > 0 && (
        <Menu icon="compass" title="From where the chip is seen" up>
          <Choice chosen={false} onChoose={() => look("top")}>From straight above</Choice>
          <Choice chosen={false} onChoose={() => look("tilted")}>Tilted, from the front</Choice>
          <Choice chosen={false} onChoose={() => look("under")}>From beneath</Choice>
        </Menu>
      )}
      <div class="group">
        <IconButton icon="zoom-out" title="Zoom out (−)" onClick={() => zoomBy(1 / 1.4)} />
        <IconButton icon="zoom-in" title="Zoom in (+)" onClick={() => zoomBy(1.4)} />
        <IconButton icon="screen-full" title="Show the whole chip (0)" onClick={fitAll} />
      </div>
    </div>
  );
}

export function Stage(): JSX.Element {
  const { building, arrangement } = useStore();
  if (arrangement === "chip") {
    return (
      <>
        {building && <div class="progress" />}
        <Mark />
      </>
    );
  }
  return (
    <>
      <Toolbar />
      {building && <div class="progress" />}
      <Notice />
      <Readout />
      <Legend />
      <Seeing />
    </>
  );
}

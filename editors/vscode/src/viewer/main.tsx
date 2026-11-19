// The viewer: a design drawn as the device it is or as its flat layout, with how it stands, what
// is wrong, what it must do and what may be tried beside it. It knows a scene and nothing else
// of chips.

import { render } from "preact";

import type { ToViewer } from "../scene";
import { drawFlat, drawOver, drawScale, drawSides, theme } from "./canvas";
import { shownFlat } from "./geometry";
import { host } from "./host";
import { listen } from "./input";
import { Foot, Panel } from "./panel/panel";
import { Stage } from "./panel/stage";
import { drawnBy, get, laidOut, openSheet, outdated, panelShown, receive, resized, subscribe } from "./state";
import { Space } from "./three/view";
import { splitter } from "./ui/splitter";

declare const BUILD: string;

// Two canvases, one on the other: the device in three dimensions beneath, and over it the flat
// layout, or else only the names and marks that lie over the device.
const canvas = document.querySelector<HTMLCanvasElement>("#chip")!;
const context = canvas.getContext("2d")!;

let queued = false;
function redraw(): void {
  if (queued) return;
  queued = true;
  requestAnimationFrame(() => {
    queued = false;
    draw();
  });
}

let space: Space | undefined;
try {
  space = new Space(document.querySelector<HTMLCanvasElement>("#space")!, redraw);
  drawnBy(space);
} catch (refused) {
  // Three dimensions cannot be drawn here: the chip is drawn flat, and the reason is shown.
  console.error(refused);
  drawnBy(undefined, refused instanceof Error ? `${refused.name}: ${refused.message}` : String(refused));
}

function draw(): void {
  const state = get();
  const ratio = window.devicePixelRatio || 1;
  const [width, height] = [canvas.clientWidth, canvas.clientHeight];
  if (canvas.width !== Math.round(width * ratio) || canvas.height !== Math.round(height * ratio)) [canvas.width, canvas.height] = [Math.round(width * ratio), Math.round(height * ratio)];
  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  context.clearRect(0, 0, width, height);
  const { scene, view, index } = state;
  if (!scene || !index || width <= 0 || height <= 0) return;
  const tones = theme();
  // Where there is room for the chip alone, nothing is written over it.
  const bare = state.arrangement === "chip";
  if (state.device && space) {
    space.draw(state, tones);
    const shown = space.shown();
    const taken = drawOver(context, shown, scene, bare ? { ...state, names: false } : state, tones);
    if (bare) return;
    // Straight above or below, one scale holds; otherwise the glass and the slab say how large they are.
    const scale = space.flatScale();
    if (scale !== undefined) return drawScale(context, scale, height, tones);
    // The slab's sides first: where the glass's would lie on them, or on a name, they are left out.
    drawSides(context, shown, [0, 0, scene.size_um[0], scene.size_um[1]], tones, taken);
    const glass = space.glass();
    if (glass) drawSides(context, shown, glass, tones, taken);
  } else if (view) {
    drawFlat(context, view, scene, state, tones);
    drawOver(context, shownFlat(view), scene, bare ? { ...state, names: false } : state, tones);
    if (!bare) drawScale(context, view.scale, height, tones);
  }
}

/** Carry the state to the page: how it is arranged, and what the styles go by. */
function shown(): void {
  const state = get();
  const body = document.body;
  body.dataset.arrangement = state.arrangement;
  body.dataset.panel = panelShown(state) ? "shown" : "hidden";
  body.dataset.stale = String(state.failure !== undefined);
  body.dataset.device = String(state.device);
  body.dataset.mode = state.mode;
  body.style.setProperty("--panel", `${state.panelPx}px`);
  const line = document.querySelector<HTMLElement>("#splitter")!;
  line.setAttribute("aria-orientation", state.arrangement === "side" ? "vertical" : "horizontal");
  line.setAttribute("aria-valuenow", String(state.panelPx));
  space?.clear(state.arrangement === "chip");
  redraw();
}
subscribe(shown);

/** Measure the preview and arrange it. */
function arrange(): void {
  const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 13;
  laidOut(document.documentElement.clientWidth, document.documentElement.clientHeight, rem);
}
arrange();
shown();
new ResizeObserver(arrange).observe(document.documentElement);
new ResizeObserver(() => {
  resized(canvas.clientWidth, canvas.clientHeight);
  space?.resized(canvas.clientWidth, canvas.clientHeight);
  redraw();
}).observe(canvas);
// The editor changes its theme by changing the page's own attributes: the chip is drawn again in it.
new MutationObserver(redraw).observe(document.documentElement, { attributes: true, attributeFilter: ["style", "class"] });
new MutationObserver(redraw).observe(document.body, { attributes: true, attributeFilter: ["class", "data-vscode-theme-kind", "data-vscode-theme-id"] });
// Moved to another screen, or zoomed, the canvases are drawn again at the new density.
function dense(): void {
  matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`).addEventListener("change", () => { space?.resized(canvas.clientWidth, canvas.clientHeight); arrange(); redraw(); dense(); }, { once: true });
}
dense();
listen(canvas, () => space && ((px, py) => space!.under(px, py)));
splitter(document.querySelector<HTMLElement>("#splitter")!);
// Escape closes a panel that lies over the chip.
document.querySelector("#panel")!.addEventListener("keydown", (event) => {
  if ((event as KeyboardEvent).key === "Escape" && get().arrangement === "sheet") openSheet(false);
});

render(<Stage />, document.querySelector("#over")!);
render(<Panel />, document.querySelector("#panel")!);
render(<Foot />, document.querySelector("#foot")!);

if (document.documentElement.dataset.build !== BUILD) outdated();
window.addEventListener("message", (event: MessageEvent<ToViewer>) => receive(event.data));
host.postMessage({ type: "ready" });

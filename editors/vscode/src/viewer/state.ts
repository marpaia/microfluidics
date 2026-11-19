// What the viewer holds: the scene it was last sent, how it is looked at, what is selected, and
// the values being tried. Everything that changes it is here, and the canvas and the panel are
// drawn from it.

import type { Scene, SceneFinding, SceneFlow, SceneInstance, ScenePort, ToViewer } from "../scene";
import { type Arrangement, type Pinned, arrangement, panelSize } from "./arrangement";
import { type Rect, type View, ancestors, bounds, fit, within, zoomed } from "./geometry";
import { host } from "./host";
import type { Look } from "./three/camera";

export type Mode = "layout" | "flow" | "shear" | "speed" | "pressure";

export const MODES: { mode: Mode; name: string; unit: string; what: string }[] = [
  { mode: "layout", name: "Layout", unit: "", what: "The chip as it is drawn, by layer" },
  { mode: "flow", name: "Flow", unit: "ul_min", what: "Flow rate through each channel" },
  { mode: "shear", name: "Shear", unit: "Pa", what: "Shear stress on the wall of each channel's narrowest part" },
  { mode: "speed", name: "Speed", unit: "mm_s", what: "Mean speed in each channel's narrowest part" },
  { mode: "pressure", name: "Pressure", unit: "mbar", what: "Pressure along each channel: the mean of its two ends" },
];

/** What a scene is asked for again and again, worked out once. */
export interface Index {
  where: Map<string, Rect>;
  instances: Map<string, SceneInstance>;
  ports: Map<string, ScenePort>;
  flows: Map<string, SceneFlow>;
  /** The findings that are about something on the chip, in the order they are numbered. */
  pinned: SceneFinding[];
}

export interface State {
  scene?: Scene;
  index?: Index;
  view?: View;
  size: [number, number];
  mode: Mode;
  selection?: string;
  hover?: string;
  /** What the line under the editor's cursor made. */
  highlighted: string[];
  hidden: Set<string>;
  names: boolean;
  /** Whether the chip is drawn as the device it is, in three dimensions, or as its flat layout. */
  device: boolean;
  /** Whether three dimensions can be drawn here at all. */
  spaceless: boolean;
  /** Why it cannot, in the words of what refused. */
  refusal?: string;
  /** How many times their true height features stand, in three dimensions. */
  depth: number;
  /** The values being tried, by the tunable's path; and what each tunable is kept at. */
  trial: Map<string, number>;
  kept: Map<string, number>;
  /** Whether the scene shown was built with values that are not kept. */
  tried: boolean;
  building: boolean;
  failure?: { message: string; file?: string; line?: number };
  buildMs?: number;
  result?: { command: string; ok: boolean; text: string };
  running?: string;
  /** Which sections of the panel are open. */
  open: Record<string, boolean>;
  /** How the stage and the panel share the preview, and where the reader asked the panel to be. */
  arrangement: Arrangement;
  pinned?: Pinned;
  /** The panel's width beside the stage, or its height beneath it, in pixels. */
  panelPx: number;
  /** What the reader dragged the panel to, in each arrangement. */
  held: { side?: number; stacked?: number };
  /** Whether the panel lies over the stage, where there is room for only one of them. */
  sheet: boolean;
  /** Whether the reader has selected something, and no longer needs telling how. */
  hinted: boolean;
  /** The checks whose problems are all listed, however many. */
  unfolded: Set<string>;
  /** Whether the view of the flat chip is the fitted one, to be fitted again when its room changes. */
  fitted: boolean;
  /** Whether this page was made by another build of the extension than the one that runs in it. */
  outdated: boolean;
  /** Where the pointer is on the chip, in micrometres. */
  pointer?: [number, number];
}

interface Remembered {
  mode?: Mode;
  names?: boolean;
  device?: boolean;
  depth?: number;
  open?: Record<string, boolean>;
  pinned?: Pinned;
  held?: { side?: number; stacked?: number };
  hinted?: boolean;
  /** The design this preview is of, for the editor to open it again after a reload. */
  design?: string;
}

const remembered = (host.getState() ?? {}) as Remembered;

let state: State = {
  size: [0, 0],
  mode: remembered.mode ?? "layout",
  highlighted: [],
  hidden: new Set(),
  names: remembered.names ?? true,
  device: remembered.device ?? true,
  spaceless: false,
  depth: remembered.depth ?? 1,
  trial: new Map(),
  kept: new Map(),
  tried: false,
  building: true,
  open: { selected: true, checks: true, requirements: true, parameters: true, ...remembered.open },
  arrangement: "side",
  pinned: remembered.pinned,
  panelPx: 0,
  held: remembered.held ?? {},
  sheet: false,
  hinted: remembered.hinted ?? false,
  unfolded: new Set(),
  fitted: true,
  outdated: false,
};

/** Whether the panel is on the screen: beside the stage, beneath it, or over it. */
export function panelShown(held: State = state): boolean {
  if (held.arrangement === "chip" || held.pinned === "hidden") return false;
  return held.arrangement !== "sheet" || held.sheet;
}

/** The view of a whole chip, clear of the controls that lie over the canvas's upper and lower edges. */
function whole(size: [number, number], width: number, height: number): View {
  return state.arrangement === "chip" ? fit(size, width, height, 8) : fit(size, width, height, 24, 52, 64);
}

/** What moves the eye in three dimensions, when the chip is drawn so. */
export interface Eyed {
  frame(rect?: Rect): void;
  look(look: Look): void;
  orbitBy(dx: number, dy: number): void;
  panBy(dx: number, dy: number): void;
  zoomBy(factor: number, px?: number, py?: number): void;
}

let space: Eyed | undefined;

/** Say what draws the chip in three dimensions; with nothing, that it cannot be drawn so here. */
export function drawnBy(eyed: Eyed | undefined, refusal?: string): void {
  space = eyed;
  if (!eyed) set({ device: false, spaceless: true, refusal });
}

/** What moves the eye, while the chip is drawn in three dimensions. */
function eyed(): Eyed | undefined {
  return state.device ? space : undefined;
}

const listeners = new Set<() => void>();

export function get(): State {
  return state;
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function set(patch: Partial<State>): void {
  state = { ...state, ...patch };
  host.setState({ mode: state.mode, names: state.names, device: state.device, depth: state.depth, open: state.open, pinned: state.pinned, held: state.held, hinted: state.hinted, design: document.documentElement.dataset.design } satisfies Remembered);
  for (const listener of listeners) listener();
}

function indexed(scene: Scene): Index {
  const where = bounds(scene);
  for (const instance of scene.instances) if (instance.rect) where.set(instance.path, instance.rect);
  return {
    where,
    instances: new Map(scene.instances.map((held) => [held.path, held])),
    ports: new Map(scene.ports.map((held) => [held.path, held])),
    flows: new Map(scene.flows.map((held) => [held.element, held])),
    pinned: scene.findings.filter((finding) => !finding.waived && finding.paths.some((path) => where.has(path))),
  };
}

/** The box about a path: its own, or that of the nearest thing that holds it. */
export function boxOf(path: string): Rect | undefined {
  const where = state.index?.where;
  for (const held of ancestors(path).reverse()) {
    const rect = where?.get(held);
    if (rect) return rect;
  }
  return undefined;
}

/** What the session or the editor said. */
export function receive(message: ToViewer): void {
  if (message.type === "scene") {
    const scene = message.scene;
    const index = indexed(scene);
    const same = state.scene && state.scene.size_um[0] === scene.size_um[0] && state.scene.size_um[1] === scene.size_um[1];
    const kept = message.trial ? state.kept : new Map(scene.tunables.map((held) => [held.path, held.value]));
    // What was selected stays selected while it, or something that held it, is still there.
    const selection = state.selection === undefined ? undefined : ancestors(state.selection).reverse().find((path) => index.where.has(path));
    set({
      scene,
      index,
      kept,
      selection,
      hover: undefined,
      tried: message.trial,
      trial: message.trial ? state.trial : new Map(),
      failure: undefined,
      view: same && state.view ? state.view : state.size[0] > 0 ? whole(scene.size_um, ...state.size) : undefined,
      fitted: same && state.view ? state.fitted : true,
    });
  } else if (message.type === "status") {
    if (message.state === "failed") set({ building: false, failure: { message: message.message ?? "The design could not be built.", file: message.file, line: message.line } });
    else if (message.state === "building") set({ building: true });
    else set({ building: false, buildMs: message.build_ms, failure: undefined });
  } else if (message.type === "highlight") {
    set({ highlighted: message.paths });
  } else if (message.type === "result") {
    set({ result: message, running: undefined });
  }
}

export function resized(width: number, height: number): void {
  if (width === state.size[0] && height === state.size[1]) return;
  // The fitted view is fitted again; any other keeps the point of the chip that was at its middle.
  const kept = (view: View): View => ({ ...view, width, height, x: view.x + (view.width - width) / view.scale / 2, y: view.y + (view.height - height) / view.scale / 2 });
  const view = state.scene && width > 0 && (state.fitted || !state.view) ? whole(state.scene.size_um, width, height) : state.view ? kept(state.view) : undefined;
  set({ size: [width, height], view });
}

export function setView(view: View): void {
  set({ view });
}

export function zoomBy(factor: number, px?: number, py?: number): void {
  const held = eyed();
  if (held) held.zoomBy(factor, px, py);
  else if (state.view) set({ fitted: false, view: zoomed(state.view, factor, px ?? state.view.width / 2, py ?? state.view.height / 2) });
}

export function panBy(dx: number, dy: number): void {
  const held = eyed();
  if (held) held.panBy(-dx, dy);
  else if (state.view) set({ fitted: false, view: { ...state.view, x: state.view.x + dx / state.view.scale, y: state.view.y + dy / state.view.scale } });
}

/** Bring a box to the middle of the canvas with room about it. */
export function fitTo(rect: Rect): void {
  const held = eyed();
  if (held) return held.frame(rect);
  if (!state.view) return;
  const { width, height } = state.view;
  const [wide, high] = [Math.max(rect[2] - rect[0], 20), Math.max(rect[3] - rect[1], 20)];
  const scale = Math.min(width / (wide * 1.6), height / (high * 1.6), 2000);
  set({ fitted: false, view: { ...state.view, scale, x: (rect[0] + rect[2]) / 2 - width / scale / 2, y: (rect[1] + rect[3]) / 2 - height / scale / 2 } });
}

/** Show the whole chip; or, with something selected or under the editor's cursor, that. */
export function fitAll(): void {
  const held = eyed();
  if (held) held.frame();
  else if (state.scene && state.size[0] > 0) set({ fitted: true, view: whole(state.scene.size_um, ...state.size) });
}

export function fitSelection(): void {
  const path = state.selection ?? state.highlighted[0];
  const rect = path === undefined ? undefined : boxOf(path);
  if (rect) fitTo(rect);
  else fitAll();
}

/** Select something, or nothing. The editor is shown the line that made it. */
export function select(path: string | undefined, shown = false): void {
  // What is chosen from a panel that lies over the chip is shown on the chip, so the panel gives way.
  set({ selection: path, open: path === undefined ? state.open : { ...state.open, selected: true }, hinted: state.hinted || path !== undefined, sheet: shown ? false : state.sheet });
  if (path === undefined) return;
  host.postMessage({ type: "reveal", path });
  const rect = boxOf(path);
  if (shown && rect) fitTo(rect);
}

export function hover(path: string | undefined, pointer?: [number, number]): void {
  if (path === state.hover && pointer?.[0] === state.pointer?.[0] && pointer?.[1] === state.pointer?.[1]) return;
  set({ hover: path, pointer });
}

export function setMode(mode: Mode): void {
  set({ mode });
}

export function toggleLayer(name: string): void {
  const hidden = new Set(state.hidden);
  if (!hidden.delete(name)) hidden.add(name);
  set({ hidden });
}

/** Draw the chip in three dimensions, or flat. */
export function setDevice(device: boolean): void {
  if (!state.spaceless) set({ device });
}

/** Turn the eye about the chip, in three dimensions. */
export function orbitBy(dx: number, dy: number): void {
  eyed()?.orbitBy(dx, dy);
}

/** See the chip from straight above, tilted, or from beneath. */
export function look(to: Look): void {
  eyed()?.look(to);
}

export function setDepth(depth: number): void {
  set({ depth });
}

export function toggleNames(): void {
  set({ names: !state.names });
}

/** Hide the panel, or show it again where the preview's shape puts it. */
export function togglePanel(): void {
  if (state.arrangement === "sheet") set({ sheet: !state.sheet, pinned: state.pinned === "hidden" ? undefined : state.pinned });
  else set({ pinned: state.pinned === "hidden" ? undefined : "hidden" });
}

/** Put the panel where the reader asks, or with `undefined` where the preview's shape puts it. */
export function pin(pinned: Pinned | undefined): void {
  set({ pinned });
  laidOut(...measured);
}

export function openSheet(sheet: boolean): void {
  set({ sheet });
}

let measured: [number, number, number] = [0, 0, 13];

/** The preview is `width` by `height` pixels, with `rem` pixels to the rem: arrange it. */
export function laidOut(width: number, height: number, rem: number): void {
  measured = [width, height, rem];
  if (width <= 0 || height <= 0) return;
  const how = arrangement(width, height, rem, state.pinned);
  const panelPx = how === "side" || how === "stacked" ? panelSize(how, width, height, rem, state.held[how]) : 0;
  if (how !== state.arrangement || panelPx !== state.panelPx) set({ arrangement: how, panelPx, sheet: how === "sheet" ? state.sheet : false });
}

/** Hold the panel at `size` pixels in the arrangement it is in, or with `undefined` let it be
 * what the preview's shape gives. */
export function resizePanel(size: number | undefined): void {
  const how = state.arrangement;
  if (how !== "side" && how !== "stacked") return;
  const [width, height, rem] = measured;
  const held = { ...state.held, [how]: size === undefined ? undefined : panelSize(how, width, height, rem, size) };
  set({ held, panelPx: panelSize(how, width, height, rem, held[how]) });
}

/** List every problem of a check, however many. */
export function unfold(check: string): void {
  set({ unfolded: new Set([...state.unfolded, check]) });
}

/** Say that this page is of another build than the script that runs in it. */
export function outdated(): void {
  set({ outdated: true });
}

export function toggleSection(name: string, open?: boolean): void {
  set({ open: { ...state.open, [name]: open ?? !state.open[name] } });
}

let sending: number | undefined;

function sendTrial(): void {
  const values: Record<string, string> = {};
  for (const tunable of state.scene?.tunables ?? []) {
    const value = state.trial.get(tunable.path);
    if (value !== undefined) values[tunable.path] = `${Number(value.toFixed(3))}${tunable.written.replace(/^[-+0-9.eE]+/, "")}`;
  }
  host.postMessage({ type: "set_overrides", values });
}

/** Try a value for a tunable, or with `undefined` go back to what is kept for it. */
export function tryValue(path: string, value: number | undefined): void {
  const trial = new Map(state.trial);
  if (value === undefined || value === state.kept.get(path)) trial.delete(path);
  else trial.set(path, value);
  set({ trial });
  window.clearTimeout(sending);
  sending = window.setTimeout(sendTrial, 120);
}

export function discard(): void {
  window.clearTimeout(sending);
  set({ trial: new Map() });
  host.postMessage({ type: "set_overrides", values: {} });
}

export function keep(): void {
  window.clearTimeout(sending);
  sendTrial();
  host.postMessage({ type: "keep" });
}

export function run(command: string): void {
  window.clearTimeout(sending);
  set({ trial: new Map(), running: command, result: undefined });
  host.postMessage({ type: "run", command });
}

export function dismissResult(): void {
  set({ result: undefined });
}

/** The value each edge of the flow graph has in a mode, by the element it belongs to. */
export function valuesIn(scene: Scene, mode: Mode): Map<string, number> {
  const of = (flow: SceneFlow): number => {
    if (mode === "flow") return Math.abs(flow.flow_ul_min);
    if (mode === "shear") return Math.abs(flow.shear_pa);
    if (mode === "speed") return Math.abs(flow.velocity_mm_s);
    return ((scene.pressures_mbar[flow.start] ?? 0) + (scene.pressures_mbar[flow.end] ?? 0)) / 2;
  };
  return mode === "layout" ? new Map() : new Map(scene.flows.map((flow) => [flow.element, of(flow)]));
}

/** The flows through a thing: its own, or those of everything within it. */
export function flowsThrough(scene: Scene, path: string): SceneFlow[] {
  const own = scene.flows.filter((flow) => flow.element === path);
  return own.length ? own : scene.flows.filter((flow) => within(flow.element, path));
}

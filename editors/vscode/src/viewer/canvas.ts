// What is drawn on the 2D canvas: the flat layout, and, over whichever way the chip is drawn,
// the names of what is on it, what is selected, what was found and how large things are.

import type { Scene, SceneTranslates, Shape } from "../scene";
import { length, number, unitName } from "./format";
import { type Rect, type Shown, type View, extent, inView, labels, nameable, overlaps, placed, scaleBar, toCanvas, visible, within } from "./geometry";
import { painter } from "./paint";
import type { State } from "./state";

type Box = [number, number, number, number];

/** The editor's colours, read from the page each time the chip is drawn. */
export interface Theme {
  text: string;
  quiet: string;
  ground: string;
  slab: string;
  line: string;
  focus: string;
  wrong: string;
  font: string;
}

export function theme(): Theme {
  const style = getComputedStyle(document.body);
  const read = (name: string, otherwise: string) => style.getPropertyValue(name).trim() || otherwise;
  return {
    text: read("--vscode-foreground", "#cccccc"),
    quiet: read("--vscode-descriptionForeground", "#9d9d9d"),
    ground: read("--vscode-editor-background", "#1f1f1f"),
    slab: read("--vscode-editorWidget-background", "#252526"),
    line: read("--vscode-editorWidget-border", "#454545"),
    focus: read("--vscode-focusBorder", "#0078d4"),
    wrong: read("--vscode-editorError-foreground", "#f14c4c"),
    font: style.fontFamily || "sans-serif",
  };
}

function box(view: View, rect: Rect): Box {
  const [x0, y1] = toCanvas(view, rect[0], rect[1]);
  const [x1, y0] = toCanvas(view, rect[2], rect[3]);
  return [x0, y0, Math.max(x1 - x0, 0.75), Math.max(y1 - y0, 0.75)];
}

function drawRow(context: CanvasRenderingContext2D, view: View, row: SceneTranslates, outline: Shape[], seen: Rect, paint: (path: string, layer: string) => [string, number]): void {
  const whole = extent(row, outline);
  if (!overlaps(whole, seen)) return;
  const first = placed(row, outline);
  const smallest = Math.min(...first.map((rect) => Math.min(rect[2] - rect[0], rect[3] - rect[1]))) * view.scale;
  const [fill, strength] = paint(row.path, outline[0]?.layer ?? "");
  context.fillStyle = fill;
  if (smallest < 0.6) {
    // Too small to tell apart: the row is shown as one faint block.
    context.globalAlpha = strength * 0.45;
    context.fillRect(...box(view, whole));
    return;
  }
  context.globalAlpha = strength;
  const [from, to] = inView(row, outline, seen);
  for (const rect of first) {
    for (let index = from; index < to; index++) {
      context.fillRect(...box(view, [rect[0] + index * row.step[0], rect[1] + index * row.step[1], rect[2] + index * row.step[0], rect[3] + index * row.step[1]]));
    }
  }
}

/** An arrowhead at each edge's downstream end, pointing the way fluid goes. */
function drawDirections(context: CanvasRenderingContext2D, view: View, scene: Scene, state: State, tones: Theme): void {
  const index = state.index!;
  context.fillStyle = tones.text;
  for (const flow of scene.flows) {
    if (flow.flow_ul_min === 0) continue;
    const rect = index.where.get(flow.element);
    if (!rect || Math.max(rect[2] - rect[0], rect[3] - rect[1]) * view.scale < 28) continue;
    const port = index.ports.get(flow.flow_ul_min > 0 ? flow.end : flow.start);
    if (!port) continue;
    // A port faces the way a channel leaves by it: outward for the edge's own, inward for another's.
    const outward = within(port.path, flow.element) ? 1 : -1;
    const [dx, dy] = [port.facing[0] * outward, -port.facing[1] * outward];
    const [px, py] = toCanvas(view, port.at[0], port.at[1]);
    if (px < -10 || py < -10 || px > view.width + 10 || py > view.height + 10) continue;
    const size = Math.min(Math.max(port.width_um * view.scale * 0.9, 5), 9);
    const [cx, cy] = [px - dx * size * 1.4, py - dy * size * 1.4];
    context.globalAlpha = 0.95;
    context.beginPath();
    context.moveTo(cx + dx * size, cy + dy * size);
    context.lineTo(cx - dx * size * 0.6 - dy * size * 0.75, cy - dy * size * 0.6 + dx * size * 0.75);
    context.lineTo(cx - dx * size * 0.6 + dy * size * 0.75, cy - dy * size * 0.6 - dx * size * 0.75);
    context.closePath();
    context.fill();
  }
  context.globalAlpha = 1;
}

/** The flat layout: the slab's outline, each layer, where tubing reaches the chip, and which
 * way fluid goes. */
export function drawFlat(context: CanvasRenderingContext2D, view: View, scene: Scene, state: State, tones: Theme): void {
  const seen = visible(view);
  context.fillStyle = tones.slab;
  context.strokeStyle = tones.line;
  context.lineWidth = 1;
  const slab = box(view, [0, 0, scene.size_um[0], scene.size_um[1]]);
  context.fillRect(...slab);
  context.strokeRect(slab[0] + 0.5, slab[1] + 0.5, slab[2], slab[3]);
  // Each layer, the deepest first so that the shallow one lies on top.
  const paint = painter(scene, state);
  for (const layer of [...scene.layers].sort((a, b) => b.height_um - a.height_um)) {
    if (state.hidden.has(layer.name)) continue;
    for (const shape of scene.shapes) {
      if (shape.layer !== layer.name || !overlaps(shape.rect, seen)) continue;
      [context.fillStyle, context.globalAlpha] = paint(shape.path, shape.layer);
      context.fillRect(...box(view, shape.rect));
    }
    for (const row of scene.translates) {
      const outline = scene.outlines[row.outline] ?? [];
      if (outline[0]?.layer === layer.name) drawRow(context, view, row, outline, seen, paint);
    }
  }
  context.globalAlpha = state.failure ? 0.35 : 1;
  context.strokeStyle = tones.text;
  context.lineWidth = 1.25;
  const radius = (scene.punch_um ?? 750) / 2;
  for (const [x, y] of scene.access) {
    const [px, py] = toCanvas(view, x, y);
    context.beginPath();
    context.arc(px, py, Math.max(radius * view.scale, 3), 0, 2 * Math.PI);
    context.stroke();
  }
  if (state.mode !== "layout" && !state.failure) drawDirections(context, view, scene, state, tones);
  context.globalAlpha = 1;
}

/** Write the names, and give the box each took on the canvas. */
function drawNames(context: CanvasRenderingContext2D, shown: Shown, scene: Scene, state: State, tones: Theme): Rect[] {
  const taken: Rect[] = [];
  context.font = `500 11px ${tones.font}`;
  context.textBaseline = "alphabetic";
  context.lineJoin = "round";
  context.lineWidth = 3.5;
  context.strokeStyle = tones.ground;
  const width = (text: string) => context.measureText(text).width;
  const applied = new Map(scene.drives.map((drive) => [drive.instance, drive.pressure_mbar !== null ? `${number(drive.pressure_mbar)} mbar` : `${number(drive.flow_ul_min ?? 0)} ${unitName("ul_min")}`]));
  const names = nameable(scene.instances, shown, width).map((name) => ({ ...name, note: applied.get(name.path) }));
  // What is selected or pointed at is named first, so that its name is never the one left out.
  const first = [...new Set([state.selection, state.hover])].filter((path): path is string => path !== undefined && !names.some((name) => name.path === path));
  const chosen = first.flatMap((path) => {
    const rect = state.index!.where.get(path);
    return rect ? [{ path, text: path, rect, note: applied.get(path) }] : [];
  });
  const marked = new Set([state.selection, state.hover]);
  const ordered = [...chosen, ...names.filter((name) => marked.has(name.path)), ...names.filter((name) => !marked.has(name.path))];
  // Names stay clear of the controls along the canvas's upper edge.
  for (const label of labels(ordered, shown, width, 14, 50)) {
    // Each name is edged in the ground's colour, so that it reads over whatever lies under it.
    const written = (text: string, x: number, fill: string) => {
      context.globalAlpha = 0.9;
      context.strokeText(text, x, label.y);
      context.globalAlpha = 1;
      context.fillStyle = fill;
      context.fillText(text, x, label.y);
    };
    written(label.text, label.x, label.path === state.selection ? tones.focus : tones.text);
    if (label.note) written(label.note, label.x + width(`${label.text} `) + 4, tones.quiet);
    taken.push([label.x - 2, label.y - 12, label.x + label.width + 2, label.y + 4]);
  }
  return taken;
}

/** A scale bar, where one length on the canvas is one length on the chip. */
export function drawScale(context: CanvasRenderingContext2D, scale: number, height: number, tones: Theme): void {
  const bar = scaleBar(scale);
  const [x, y] = [14, height - 16];
  context.strokeStyle = tones.text;
  context.fillStyle = tones.text;
  context.lineWidth = 1;
  context.beginPath();
  context.moveTo(x + 0.5, y - 5);
  context.lineTo(x + 0.5, y + 0.5);
  context.lineTo(x + bar.px + 0.5, y + 0.5);
  context.lineTo(x + bar.px + 0.5, y - 5);
  context.stroke();
  context.font = `11px ${tones.font}`;
  context.textAlign = "start";
  context.fillText(bar.label, x + bar.px + 8, y + 1);
}

/** The width and depth of a rectangle, written at the middle of two of its sides: for a view in
 * which no one bar is true. A length is left out where it would lie on something in `taken`,
 * and what is written is added to `taken`. */
export function drawSides(context: CanvasRenderingContext2D, shown: Shown, rect: Rect, tones: Theme, taken: Rect[] = []): void {
  const corners = shown.corners(rect);
  if (!corners) return;
  // The side nearest the foot of the canvas, and of the two that meet it the nearer.
  const low = (side: number) => corners[side % 4]![1] + corners[(side + 1) % 4]![1];
  const lowest = [0, 1, 2, 3].reduce((best, side) => (low(side) > low(best) ? side : best), 0);
  const beside = low(lowest + 1) > low(lowest + 3) ? (lowest + 1) % 4 : (lowest + 3) % 4;
  context.font = `500 11px ${tones.font}`;
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.lineJoin = "round";
  context.lineWidth = 3.5;
  context.strokeStyle = tones.ground;
  context.fillStyle = tones.quiet;
  for (const side of [lowest, beside]) {
    const [a, b] = [corners[side]!, corners[(side + 1) % 4]!];
    if (Math.hypot(a[0] - b[0], a[1] - b[1]) < 60) continue;
    // Sides 0 and 2 run along the rectangle's width; 1 and 3 along its depth.
    const text = length(side % 2 === 0 ? rect[2] - rect[0] : rect[3] - rect[1]);
    const [x, y] = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2 + 10];
    const half = context.measureText(text).width / 2 + 3;
    const box: Rect = [x - half, y - 8, x + half, y + 8];
    if (taken.some((held) => overlaps(held, box))) continue;
    taken.push(box);
    context.strokeText(text, x, y);
    context.fillText(text, x, y);
  }
  context.textAlign = "start";
  context.textBaseline = "alphabetic";
}

/** What lies over the chip however it is drawn: what the editor's cursor is on, what the
 * pointer is on, what is selected, the names, and each finding that is about something here.
 * Gives the boxes its names took, for what is written after to keep clear of. */
export function drawOver(context: CanvasRenderingContext2D, shown: Shown, scene: Scene, state: State, tones: Theme): Rect[] {
  const index = state.index!;
  const outline = (rect: Rect | undefined) => {
    const corners = rect && shown.corners(rect);
    if (!corners) return;
    context.beginPath();
    corners.forEach(([x, y], at) => (at ? context.lineTo(x, y) : context.moveTo(x, y)));
    context.closePath();
    context.stroke();
  };
  context.lineJoin = "round";
  context.strokeStyle = tones.focus;
  context.lineWidth = 1;
  context.setLineDash([4, 3]);
  for (const path of state.highlighted) if (path !== state.selection) outline(index.where.get(path));
  context.setLineDash([]);
  if (state.hover !== undefined && state.hover !== state.selection) {
    context.strokeStyle = tones.text;
    context.globalAlpha = 0.6;
    outline(index.where.get(state.hover));
    context.globalAlpha = 1;
  }
  if (state.selection !== undefined) {
    context.strokeStyle = tones.focus;
    context.lineWidth = 2;
    outline(index.where.get(state.selection));
  }

  const taken = state.names && !state.failure ? drawNames(context, shown, scene, state, tones) : [];

  // Each finding that is about something here, numbered as the panel numbers it.
  context.font = `600 10px ${tones.font}`;
  context.textAlign = "center";
  context.textBaseline = "middle";
  index.pinned.forEach((finding, at) => {
    const rect = finding.paths.map((path) => index.where.get(path)).find((held) => held !== undefined);
    const held = rect && shown.box(rect);
    if (!rect || !held || held[2] < 0 || held[0] > shown.width || held[3] < 0 || held[1] > shown.height) return;
    context.strokeStyle = tones.wrong;
    context.lineWidth = 1.5;
    outline(rect);
    if (at >= 40) return;
    // Beside its upper right corner, clear of the name hung from its upper left.
    const corner = shown.corners(rect)?.[1] ?? [held[2], held[1]];
    const [cx, cy] = [corner[0] + 11, corner[1] + 12];
    context.fillStyle = tones.wrong;
    context.beginPath();
    context.arc(cx, cy, 8, 0, 2 * Math.PI);
    context.fill();
    context.fillStyle = tones.ground;
    context.fillText(String(at + 1), cx, cy + 0.5);
  });
  context.textAlign = "start";
  context.textBaseline = "alphabetic";
  return taken;
}

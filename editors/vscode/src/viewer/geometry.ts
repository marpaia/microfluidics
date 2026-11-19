// What the viewer works out from a scene before it draws: where things are, what is in view,
// what is under the pointer, and what colour a value is. Nothing here touches the page.

import type { Scene, SceneInstance, SceneTranslates, Shape } from "../scene";

export type Rect = [number, number, number, number];

/** A view of the plane: `scale` pixels to a micrometre, with the point `(x, y)` at the canvas's
 * lower left. Y runs upward in a design and downward on a canvas. */
export interface View {
  scale: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

/** The view that shows the whole chip with a margin: `margin` pixels at its sides, and `above`
 * and `below` it, where a canvas has controls lying over it. */
export function fit(size: [number, number], width: number, height: number, margin = 24, above = margin, below = margin): View {
  const scale = Math.max(Math.min((width - 2 * margin) / size[0], (height - above - below) / size[1]), 1e-4);
  const spare = (height - above - below) / scale - size[1];
  return { scale, x: (size[0] - width / scale) / 2, y: -below / scale - spare / 2, width, height };
}

/** A point of the design on the canvas. */
export function toCanvas(view: View, x: number, y: number): [number, number] {
  return [(x - view.x) * view.scale, view.height - (y - view.y) * view.scale];
}

/** A point of the canvas in the design. */
export function toDesign(view: View, px: number, py: number): [number, number] {
  return [view.x + px / view.scale, view.y + (view.height - py) / view.scale];
}

/** The view after zooming by `factor` about a point of the canvas, which stays where it is. */
export function zoomed(view: View, factor: number, px: number, py: number): View {
  const [x, y] = toDesign(view, px, py);
  const scale = Math.min(Math.max(view.scale * factor, 1e-4), 2000);
  return { ...view, scale, x: x - px / scale, y: y - (view.height - py) / scale };
}

/** The part of the design a view shows. */
export function visible(view: View): Rect {
  return [view.x, view.y, view.x + view.width / view.scale, view.y + view.height / view.scale];
}

export function overlaps(a: Rect, b: Rect): boolean {
  return a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3];
}

function turned(rect: Rect, quarterTurns: number): Rect {
  let [x0, y0, x1, y1] = rect;
  for (let turn = 0; turn < ((quarterTurns % 4) + 4) % 4; turn++) {
    [x0, y0, x1, y1] = [-y1, x0, -y0, x1];
  }
  return [Math.min(x0, x1), Math.min(y0, y1), Math.max(x0, x1), Math.max(y0, y1)];
}

/** The rectangles of one outline as an row places them, before any step: turned, at the
 * row's origin. */
export function placed(row: SceneTranslates, outline: Shape[]): Rect[] {
  return outline.map((shape) => {
    const [x0, y0, x1, y1] = turned(shape.rect, row.quarter_turns);
    return [x0 + row.origin[0], y0 + row.origin[1], x1 + row.origin[0], y1 + row.origin[1]];
  });
}

/** The rectangle that holds every element of an row. */
export function extent(row: SceneTranslates, outline: Shape[]): Rect {
  const first = placed(row, outline);
  const last = Math.max(row.count - 1, 0);
  const xs = first.flatMap((rect) => [rect[0], rect[2], rect[0] + last * row.step[0], rect[2] + last * row.step[0]]);
  const ys = first.flatMap((rect) => [rect[1], rect[3], rect[1] + last * row.step[1], rect[3] + last * row.step[1]]);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

/** The elements of an row that may be in view: the first and one past the last, by index. An
 * row whose elements step along one axis is cut to the part the view holds. */
export function inView(row: SceneTranslates, outline: Shape[], view: Rect): [number, number] {
  const box = extent({ ...row, count: 1 }, outline);
  let [from, to] = [0, row.count];
  for (const axis of [0, 1] as const) {
    const step = row.step[axis];
    if (step === 0) continue;
    const low = (view[axis] - box[axis + 2]!) / step;
    const high = (view[axis + 2]! - box[axis]!) / step;
    from = Math.max(from, Math.floor(Math.min(low, high)));
    to = Math.min(to, Math.ceil(Math.max(low, high)) + 1);
  }
  return [Math.max(from, 0), Math.max(Math.min(to, row.count), 0)];
}

/** Where everything named is: each path's rectangle. An row is known by its whole extent, and
 * an instance by everything made under it. */
export function bounds(scene: Scene): Map<string, Rect> {
  const found = new Map<string, Rect>();
  const grow = (path: string, rect: Rect) => {
    const held = found.get(path);
    found.set(path, held ? [Math.min(held[0], rect[0]), Math.min(held[1], rect[1]), Math.max(held[2], rect[2]), Math.max(held[3], rect[3])] : rect);
  };
  const named = (path: string, rect: Rect) => {
    grow(path, rect);
    // Every instance that holds it: `lane[3].left` is in `lane[3]`.
    for (let at = path.lastIndexOf("."); at > 0; at = path.lastIndexOf(".", at - 1)) grow(path.slice(0, at), rect);
  };
  for (const shape of scene.shapes) named(shape.path, shape.rect);
  for (const row of scene.translates) named(row.path, extent(row, scene.outlines[row.outline] ?? []));
  for (const port of scene.ports) {
    const half = port.width_um / 2;
    grow(port.path, [port.at[0] - half, port.at[1] - half, port.at[0] + half, port.at[1] + half]);
  }
  return found;
}

/** The path of what is drawn at a point, or within `slack` of it: the smallest rectangle that
 * holds it, an element of a row by the row's path. `undefined` where nothing is drawn. */
export function at(scene: Scene, hidden: Set<string>, x: number, y: number, slack = 0): string | undefined {
  let best: { path: string; area: number } | undefined;
  const consider = (path: string, rect: Rect) => {
    if (x < rect[0] - slack || x > rect[2] + slack || y < rect[1] - slack || y > rect[3] + slack) return;
    const area = (rect[2] - rect[0]) * (rect[3] - rect[1]);
    if (!best || area < best.area) best = { path, area };
  };
  for (const shape of scene.shapes) if (!hidden.has(shape.layer)) consider(shape.path, shape.rect);
  for (const row of scene.translates) {
    const outline = scene.outlines[row.outline] ?? [];
    if (outline.some((shape) => hidden.has(shape.layer))) continue;
    const box = extent(row, outline);
    if (x < box[0] - slack || x > box[2] + slack || y < box[1] - slack || y > box[3] + slack) continue;
    const [from, to] = inView(row, outline, [x - slack, y - slack, x + slack, y + slack]);
    for (const rect of placed(row, outline)) {
      for (let index = from; index < to; index++) {
        consider(row.path, [rect[0] + index * row.step[0], rect[1] + index * row.step[1], rect[2] + index * row.step[0], rect[3] + index * row.step[1]]);
      }
    }
  }
  return best?.path;
}

/** The edge of the flow graph a path belongs to: `riser[2].0` belongs to `riser[2]`, and
 * `lane[3].left` to `lane[3]`. */
export function owner(path: string, elements: Set<string>): string | undefined {
  for (let held = path; held; ) {
    if (elements.has(held)) return held;
    const at = held.lastIndexOf(".");
    if (at < 0) return undefined;
    held = held.slice(0, at);
  }
  return undefined;
}

/** Where a value lies between `low` and `high`, from 0 to 1: by its logarithm when `log`. */
export function share(value: number, low: number, high: number, log: boolean): number {
  if (!(high > low)) return 0.5;
  const place = log && low > 0 ? Math.log(Math.max(value, low) / low) / Math.log(high / low) : (value - low) / (high - low);
  return Math.min(Math.max(place, 0), 1);
}

/** Whether values from `low` to `high` are better told apart by their logarithm: when they span
 * more than fifty times. */
export function spansDecades(low: number, high: number): boolean {
  return low > 0 && high / low > 50;
}

const RAMP: [number, number, number][] = [[59, 111, 212], [43, 179, 163], [140, 200, 70], [240, 190, 50], [238, 119, 51]];

/** The colour of a share from 0 to 1: one run of hues from blue, the least, to orange, the most. */
export function ramp(place: number): string {
  const along = Math.min(Math.max(place, 0), 1) * (RAMP.length - 1);
  const [a, b] = [RAMP[Math.floor(along)]!, RAMP[Math.min(Math.floor(along) + 1, RAMP.length - 1)]!];
  const mix = along - Math.floor(along);
  return `rgb(${a.map((channel, index) => Math.round(channel + (b[index]! - channel) * mix)).join(", ")})`;
}

/** The values a legend marks between `low` and `high`: the ends, and between them the middle, or
 * when `log` the powers of ten that are not so near an end as to be written over it. */
export function legendTicks(low: number, high: number, log: boolean): number[] {
  if (!(high > low)) return [low];
  if (!log || low <= 0) return [low, (low + high) / 2, high];
  const between: number[] = [];
  const room = 0.14 * Math.log10(high / low);
  for (let power = Math.ceil(Math.log10(low) + room); power <= Math.floor(Math.log10(high) - room); power++) between.push(10 ** power);
  // At most three between the ends, evenly taken.
  const taken = between.length <= 3 ? between : [between[0]!, between[Math.floor(between.length / 2)]!, between[between.length - 1]!];
  return [low, ...taken, high];
}

/** A scale bar for a view: a round length, 1, 2 or 5 times a power of ten, about `wanted` pixels
 * long. */
export function scaleBar(scale: number, wanted = 110): { um: number; px: number; label: string } {
  const most = wanted / scale;
  const power = 10 ** Math.floor(Math.log10(most));
  const um = ([5, 2, 1].find((step) => step * power <= most) ?? 1) * power;
  const shown = (value: number) => String(Number(value.toPrecision(3)));
  const label = um >= 1000 ? `${shown(um / 1000)} mm` : um >= 1 ? `${shown(um)} µm` : `${shown(um * 1000)} nm`;
  return { um, px: um * scale, label };
}

/** Whether `path` is `holder` or something within it. */
export function within(path: string, holder: string): boolean {
  return path === holder || (path.startsWith(holder) && (path[holder.length] === "." || path[holder.length] === "["));
}

/** What holds a path, outermost first, ending with the path itself: `lane[2].left` is in
 * `lane[2]`. */
export function ancestors(path: string): string[] {
  const held: string[] = [];
  for (let at = path.indexOf("."); at >= 0; at = path.indexOf(".", at + 1)) held.push(path.slice(0, at));
  return [...held, path];
}

/** The smallest instance whose box holds a point. */
export function holding(instances: SceneInstance[], x: number, y: number): string | undefined {
  let best: { path: string; area: number } | undefined;
  for (const { path, rect } of instances) {
    if (!rect || x < rect[0] || x > rect[2] || y < rect[1] || y > rect[3]) continue;
    const area = (rect[2] - rect[0]) * (rect[3] - rect[1]);
    if (!best || area < best.area) best = { path, area };
  }
  return best?.path;
}

/** What a click selects, given what is under the pointer and what is selected: the outermost
 * thing that holds it, or, when that or something within it is selected already, one step
 * further in. */
export function pick(under: string | undefined, selection: string | undefined): string | undefined {
  if (under === undefined) return undefined;
  const chain = ancestors(under);
  const held = selection === undefined ? -1 : chain.indexOf(selection);
  return held < 0 ? chain[0] : chain[Math.min(held + 1, chain.length - 1)];
}

export interface Named {
  path: string;
  text: string;
  /** Said after the name, such as what is applied there. */
  note?: string;
  rect: Rect;
}

export interface Label extends Named {
  /** The left end and the baseline of its text, on the canvas. */
  x: number;
  y: number;
  width: number;
}

/** How a renderer shows the chip: the size of what it draws on, and where a rectangle of the
 * design lies there, as the box about it with its left, top, right and bottom in pixels.
 * `undefined` for a rectangle that cannot be seen, such as one behind the eye. */
export interface Shown {
  width: number;
  height: number;
  box(rect: Rect): Rect | undefined;
  /** The rectangle's four corners in pixels, in order about it from its upper left in the
   * design: the first is where its name is hung. */
  corners(rect: Rect): [number, number][] | undefined;
}

/** A flat view as a renderer shows it. */
export function shownFlat(view: View): Shown {
  return {
    width: view.width,
    height: view.height,
    box(rect) {
      const [x0, top] = toCanvas(view, rect[0], rect[3]);
      const [x1, bottom] = toCanvas(view, rect[2], rect[1]);
      return [x0, top, x1, bottom];
    },
    corners(rect) {
      const [x0, top] = toCanvas(view, rect[0], rect[3]);
      const [x1, bottom] = toCanvas(view, rect[2], rect[1]);
      return [[x0, top], [x1, top], [x1, bottom], [x0, bottom]];
    },
  };
}

/** What may be named on the chip in a view, the most important first: the design's own
 * instances by size, a row of them as one (`lane[0..7]`) when there is no room to name each,
 * and what is inside an instance once it is large on the canvas. */
export function nameable(instances: SceneInstance[], shown: Shown, width: (text: string) => number): Named[] {
  const boxed = instances.filter((held): held is SceneInstance & { rect: Rect } => held.rect !== null);
  const size = (rect: Rect) => (rect[2] - rect[0]) * (rect[3] - rect[1]);
  const own = boxed.filter((held) => !held.path.includes("."));
  const rows = new Map<string, (SceneInstance & { rect: Rect })[]>();
  const named: Named[] = [];
  for (const held of own) {
    const row = /^(.*)\[(\d+)\]$/.exec(held.path);
    if (row) rows.set(row[1]!, [...(rows.get(row[1]!) ?? []), held]);
    else named.push({ path: held.path, text: held.path, rect: held.rect });
  }
  for (const [name, members] of rows) {
    // Each is named when no two of their names, each above its own upper left corner, would meet.
    const apart = (a: Rect, b: Rect, wide: number) => {
      const [first, second] = [shown.box(a), shown.box(b)];
      return first !== undefined && second !== undefined && (Math.abs(first[0] - second[0]) >= wide * 1.5 + 16 || Math.abs(first[1] - second[1]) >= 28);
    };
    const clear = members.every((held, at) => members.slice(at + 1).every((other) => apart(held.rect, other.rect, Math.max(width(held.path), width(other.path)))));
    if (members.length === 1 || clear) {
      named.push(...members.map((held) => ({ path: held.path, text: held.path, rect: held.rect })));
      continue;
    }
    const indices = members.map((held) => Number(/\[(\d+)\]$/.exec(held.path)![1]));
    const rect: Rect = [Math.min(...members.map((held) => held.rect[0])), Math.min(...members.map((held) => held.rect[1])), Math.max(...members.map((held) => held.rect[2])), Math.max(...members.map((held) => held.rect[3]))];
    named.push({ path: members[0]!.path, text: `${name}[${Math.min(...indices)}..${Math.max(...indices)}]`, rect });
  }
  named.sort((a, b) => size(b.rect) - size(a.rect));
  // What is inside an instance, once the instance is large on the canvas.
  const large = (path: string) => {
    const holder = boxed.find((held) => held.path === path);
    const seen = holder && shown.box(holder.rect);
    return seen !== undefined && Math.min(seen[2] - seen[0], seen[3] - seen[1]) > 260;
  };
  const inner = boxed.filter((held) => held.path.includes(".") && large(held.path.slice(0, held.path.lastIndexOf("."))));
  inner.sort((a, b) => size(b.rect) - size(a.rect));
  return [...named, ...inner.map((held) => ({ path: held.path, text: held.path.slice(held.path.lastIndexOf(".") + 1), rect: held.rect }))];
}

/** Where each name goes: above its box's upper left corner, or below its lower left when that
 * place is taken. A box that runs off the canvas's upper or left edge is named where it comes
 * into view, no higher than `ceiling`. A name whose box is too small to point at, or that would
 * lie on one already placed, is left out; `names` are placed in the order given. */
export function labels(names: Named[], shown: Shown, width: (text: string) => number, height = 14, ceiling = height + 2): Label[] {
  const placed: Label[] = [];
  const taken: Rect[] = [];
  for (const name of names) {
    const [seen, hung] = [shown.box(name.rect), shown.corners(name.rect)?.[0]];
    if (!seen || !hung) continue;
    // The name hangs from the rectangle's own upper left corner, wherever the eye puts it.
    const [[x0, top], [, , x1, bottom]] = [hung, seen];
    if (x1 < 0 || x0 > shown.width || bottom < 0 || top > shown.height) continue;
    if (Math.max(x1 - x0, bottom - top) < 14) continue;
    const wide = width(name.text) + (name.note ? width(` ${name.note}`) + 4 : 0);
    const x = Math.min(Math.max(x0, 4), Math.max(shown.width - wide - 4, 4));
    for (const y of [Math.min(Math.max(top - 5, ceiling), bottom), bottom + height]) {
      if (y - height < 0 || y > shown.height) continue;
      const box: Rect = [x - 2, y - height, x + wide + 2, y + 3];
      if (taken.some((held) => overlaps(held, box))) continue;
      taken.push(box);
      placed.push({ ...name, x, y, width: wide });
      break;
    }
  }
  return placed;
}

/** The paths made on one line of a file. */
export function madeOn(scene: Scene, file: string, line: number): string[] {
  return Object.entries(scene.origins).filter(([, origin]) => origin.file === file && origin.line === line).map(([path]) => path);
}

/** A tunable's value as it is written: with the unit its value came with. */
export function written(value: number, example: string): string {
  const unit = example.replace(/^[-+0-9.eE]+/, "");
  return `${Number(value.toFixed(3))}${unit}`;
}

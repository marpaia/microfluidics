// The device as boxes: every rectangle of every layer as a solid of its layer's height, where the
// glass and the slab are, and where each hole is punched. Lengths here are millimetres, with the
// slab's lower left corner at the origin and its bonded face at z = 0. Nothing here draws.

import type { Scene } from "../../scene";
import { type Rect, placed } from "../geometry";
import type { Vector } from "./camera";

const MM = 1e-3;

/** One rectangle of a layer as a solid: the path it is known by, and its box. */
export interface Solid {
  path: string;
  min: Vector;
  max: Vector;
}

/** The rectangles of each layer that is shown, each as a solid that stands `depth` times its
 * layer's height from the bonded face. An element of a row is known by the row's path. */
export function solids(scene: Scene, hidden: Set<string>, depth: number): Map<string, Solid[]> {
  const found = new Map<string, Solid[]>();
  const heights = new Map(scene.layers.map((layer) => [layer.name, layer.height_um]));
  const add = (path: string, layer: string, rect: Rect) => {
    if (hidden.has(layer)) return;
    const high = (heights.get(layer) ?? 0) * depth * MM;
    const held = found.get(layer) ?? [];
    held.push({ path, min: [rect[0] * MM, rect[1] * MM, 0], max: [rect[2] * MM, rect[3] * MM, high] });
    found.set(layer, held);
  };
  for (const shape of scene.shapes) add(shape.path, shape.layer, shape.rect);
  for (const row of scene.translates) {
    const outline = scene.outlines[row.outline] ?? [];
    placed(row, outline).forEach((rect, at) => {
      for (let index = 0; index < row.count; index++) {
        add(row.path, outline[at]!.layer, [rect[0] + index * row.step[0], rect[1] + index * row.step[1], rect[2] + index * row.step[0], rect[3] + index * row.step[1]]);
      }
    });
  }
  return found;
}

/** The glass in the design's plane: the slab lies at its middle. */
export function glassRect(scene: Scene): Rect {
  const [x, y] = [(scene.size_um[0] - scene.glass_um[0]) / 2, (scene.size_um[1] - scene.glass_um[1]) / 2];
  return [x, y, x + scene.glass_um[0], y + scene.glass_um[1]];
}

/** The slab and the glass under it, as boxes. */
export function bodies(scene: Scene): { slab: { min: Vector; max: Vector }; glass: { min: Vector; max: Vector } } {
  const pane = glassRect(scene);
  return {
    slab: { min: [0, 0, 0], max: [scene.size_um[0] * MM, scene.size_um[1] * MM, scene.thickness_um * MM] },
    glass: { min: [pane[0] * MM, pane[1] * MM, -scene.glass_um[2] * MM], max: [pane[2] * MM, pane[3] * MM, 0] },
  };
}

/** The radius of a punched hole, in millimetres. */
export function holeRadius(scene: Scene): number {
  return ((scene.punch_um ?? 750) / 2) * MM;
}

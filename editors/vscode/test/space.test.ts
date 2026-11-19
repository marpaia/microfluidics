// The eye in three dimensions and the device as boxes: tested without anything being drawn.

import assert from "node:assert/strict";
import { test } from "node:test";

import type { Scene } from "../src/scene";
import { type Eye, type Vector, LOOKS, axes, framed, neared, onLevel, orbited, panned, position, projected } from "../src/viewer/three/camera";
import { bodies, glassRect, holeRadius, solids } from "../src/viewer/three/device";

const [WIDTH, HEIGHT] = [900, 600];
const near = (a: number, b: number, within = 1e-6) => Math.abs(a - b) < within;

test("an eye looks straight down with the chip upright, and tilts toward its lower edge", () => {
  const top: Eye = { target: [10, 5, 0], distance: 100, ...LOOKS.top };
  assert.deepEqual(position(top), [10, 5, 100]);
  assert.deepEqual(axes(top).right.map((held) => Math.round(held) + 0), [1, 0, 0]);
  assert.deepEqual(axes(top).up.map((held) => Math.round(held) + 0), [0, 1, 0]);
  // A point to the right and above in the design is to the right and above on the screen.
  const [px, py] = projected(top, [20, 10, 0], WIDTH, HEIGHT)!;
  assert.ok(px > WIDTH / 2 && py < HEIGHT / 2);
  // Tilted with no turn, the eye is on the side of the chip's lower edge, and above it.
  const tilted = position({ ...top, tilt: 1 });
  assert.ok(tilted[1] < 5 && tilted[2] > 0 && near(tilted[0], 10));
  // From beneath, the eye is under the glass.
  assert.ok(position({ ...top, ...LOOKS.under })[2] < 0);
  // Nothing behind the eye is anywhere on the screen.
  assert.equal(projected(top, [10, 5, 200], WIDTH, HEIGHT), undefined);
});

test("a point seen through a pixel is found again by the line of sight through that pixel", () => {
  for (const look of Object.values(LOOKS)) {
    const eye: Eye = { target: [12, 8, 1], distance: 80, ...look };
    const point: Vector = [20, 3, 0];
    const [px, py] = projected(eye, point, WIDTH, HEIGHT)!;
    const found = onLevel(eye, px, py, WIDTH, HEIGHT)!;
    assert.ok(near(found[0], point[0], 1e-6) && near(found[1], point[1], 1e-6), JSON.stringify(look));
  }
});

test("a box is framed whole, within the margins, from every side", () => {
  const [min, max]: [Vector, Vector] = [[-22, -1, -1], [52, 23, 4]];
  for (const look of Object.values(LOOKS)) {
    const eye = framed({ target: [0, 0, 0], distance: 1, ...look }, min, max, WIDTH, HEIGHT, 24, 52, 64);
    let [left, right, top, bottom] = [Infinity, -Infinity, Infinity, -Infinity];
    for (const x of [min[0], max[0]]) for (const y of [min[1], max[1]]) for (const z of [min[2], max[2]]) {
      const [px, py] = projected(eye, [x, y, z], WIDTH, HEIGHT)!;
      [left, right, top, bottom] = [Math.min(left, px), Math.max(right, px), Math.min(top, py), Math.max(bottom, py)];
    }
    assert.ok(left >= 24 - 1e-6 && right <= WIDTH - 24 + 1e-6 && top >= 52 - 1 && bottom <= HEIGHT - 64 + 1, `${JSON.stringify(look)}: ${[left, right, top, bottom]}`);
    // And it is not framed loosely: it reaches a margin one way or the other.
    assert.ok(left < 24 + 2 || top < 52 + 40, `${JSON.stringify(look)}: ${[left, right, top, bottom]}`);
  }
});

test("dragging turns the eye, carries the chip, and zooming keeps a point where it is", () => {
  const eye: Eye = { target: [10, 5, 0], distance: 100, tilt: 0.8, turn: 0.3 };
  // The tilt stops straight above and straight below.
  assert.equal(orbited(eye, 0, 10_000).tilt, 0);
  assert.equal(orbited(eye, 0, -10_000).tilt, Math.PI);
  assert.ok(orbited(eye, 50, 0).turn !== eye.turn && orbited(eye, 50, 0).distance === eye.distance);
  // A drag of thirty pixels right and twenty down carries what was looked at by as much.
  const [px, py] = projected(panned(eye, 30, 20, HEIGHT), eye.target, WIDTH, HEIGHT)!;
  assert.ok(near(px, WIDTH / 2 + 30, 1e-6) && near(py, HEIGHT / 2 + 20, 1e-6));
  // Twice as near, about a point: the point stays under its pixel.
  const about: Vector = [18, 9, 0];
  const before = projected(eye, about, WIDTH, HEIGHT)!;
  const closer = neared(eye, 2, about);
  const after = projected(closer, about, WIDTH, HEIGHT)!;
  assert.equal(closer.distance, 50);
  assert.ok(near(before[0], after[0], 1.5) && near(before[1], after[1], 1.5));
});

const scene = {
  size_um: [30_000, 22_000],
  thickness_um: 4000,
  glass_um: [75_000, 25_000, 1000],
  punch_um: 750,
  layers: [{ name: "trench", height_um: 1.1, gds: 1, colour: "#e0a030" }, { name: "flow", height_um: 20, gds: 2, colour: "#4c9be8" }],
  outlines: { tooth: [{ path: "tooth", layer: "trench", rect: [0, 0, 25, 1.2] }] },
  shapes: [{ path: "lane.channel", layer: "flow", rect: [1000, 500, 1100, 12_500] }],
  translates: [{ path: "lane.left", outline: "tooth", origin: [975, 1000], step: [0, 4], count: 50, quarter_turns: 0 }],
} as unknown as Scene;

test("every rectangle is a solid of its layer's height, times the depth asked for", () => {
  const true_ = solids(scene, new Set(), 1);
  assert.deepEqual([true_.get("flow")!.length, true_.get("trench")!.length], [1, 50]);
  const channel = true_.get("flow")![0]!;
  assert.deepEqual([channel.path, channel.min, channel.max.map((held) => Number(held.toFixed(6)))], ["lane.channel", [1, 0.5, 0], [1.1, 12.5, 0.02]]);
  // An element of a row is known by the row, and stands where the row puts it.
  const last = true_.get("trench")![49]!;
  assert.ok(last.path === "lane.left" && near(last.min[1], 1.196) && near(last.max[2], 0.0011));
  // Twenty times deeper, in height alone.
  const deep = solids(scene, new Set(), 20).get("flow")![0]!;
  assert.ok(near(deep.max[2], 0.4) && deep.max[0] === channel.max[0]);
  // A hidden layer is not made.
  assert.deepEqual([...solids(scene, new Set(["trench"]), 1).keys()], ["flow"]);
});

test("the slab lies at the middle of its glass, bonded face on the glass", () => {
  assert.deepEqual(glassRect(scene), [-22_500, -1500, 52_500, 23_500]);
  const { slab, glass } = bodies(scene);
  assert.deepEqual([slab.min, slab.max], [[0, 0, 0], [30, 22, 4]]);
  assert.deepEqual([glass.min, glass.max], [[-22.5, -1.5, -1], [52.5, 23.5, 0]]);
  assert.equal(holeRadius(scene), 0.375);
});

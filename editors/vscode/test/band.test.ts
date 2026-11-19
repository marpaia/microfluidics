import assert from "node:assert/strict";
import { test } from "node:test";

import type { SceneRequirement } from "../src/scene";
import { band } from "../src/viewer/panel/band";

const requirement = (values: number[], low: number | null, high: number | null, across?: [number, number]): SceneRequirement => ({
  name: "held",
  statement: "",
  status: "satisfied",
  values: Object.fromEntries(values.map((value, at) => [`s${at}`, value])),
  unit: "%",
  bound: { kind: "target", low, high, target: low !== null && high !== null ? (low + high) / 2 : null },
  across: across ? { low: across[0], high: across[1], low_corner: "thin", high_corner: "thick", reached: across[1], corner: "thick", holds: (low === null || across[0] >= low) && (high === null || across[1] <= high) } : null,
  note: null,
});

test("a value within what is allowed, and a range across fabrication that stays within it", () => {
  const held = band(requirement([10], 9, 11, [9.5, 10.5]))!;
  assert.ok(held.allowed[0] < held.values[0]!.at && held.values[0]!.at < held.allowed[1]);
  assert.equal(held.values[0]!.out, false);
  assert.ok(held.across && held.allowed[0] < held.across.from && held.across.to < held.allowed[1]);
  assert.deepEqual(held.across.out, []);
  assert.ok(held.target !== undefined && Math.abs(held.target - held.values[0]!.at) < 1e-9);
});

test("a range across fabrication that leaves what is allowed is marked where it is outside", () => {
  const held = band(requirement([10], 9.9, 10.1, [9.5, 10.8]))!;
  const [below, above] = held.across!.out;
  assert.ok(below && above);
  assert.ok(Math.abs(below[0] - held.across!.from) < 1e-9 && Math.abs(below[1] - held.allowed[0]) < 1e-9);
  assert.ok(Math.abs(above[0] - held.allowed[1]) < 1e-9 && Math.abs(above[1] - held.across!.to) < 1e-9);
  // Everything lies on the line.
  for (const place of [held.across!.from, held.across!.to, ...held.allowed, held.values[0]!.at]) assert.ok(place >= 0 && place <= 1);
});

test("a limit on one side runs to the line's end on the other, and values past it are out", () => {
  const held = band(requirement([0.2, 0.5, 1.4], null, 1, [0.1, 1.6]))!;
  assert.equal(held.allowed[0], 0);
  assert.deepEqual(held.values.map((value) => value.out), [false, false, true]);
  assert.equal(held.across!.out.length, 1);
});

test("nothing measured is nothing drawn", () => {
  assert.equal(band(requirement([], 0, 1)), undefined);
});

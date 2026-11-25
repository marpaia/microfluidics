// What the viewer works out from a scene: tested against a small scene of its own.

import assert from "node:assert/strict";
import { test } from "node:test";

import type { Scene } from "../src/scene";
import { ancestors, at, bounds, extent, fit, holding, inView, labels, legendTicks, madeOn, nameable, owner, pick, placed, ramp, scaleBar, share, shownFlat, spansDecades, toCanvas, toDesign, visible, within, written, zoomed } from "../src/viewer/geometry";

const scene: Scene = {
  version: "0.6.0",
  design: "demo",
  passes: false,
  problems: 0,
  size_um: [1000, 500],
  thickness_um: 4000,
  glass_um: [1200, 700, 1000],
  punch_um: 750,
  layers: [{ name: "trench", height_um: 1.1, gds: 1, colour: "#d1495b" }, { name: "flow", height_um: 20, gds: 2, colour: "#30638e" }],
  outlines: { tooth: [{ path: "tooth", layer: "trench", rect: [0, 0, 25, 1.2] }] },
  shapes: [{ path: "lane.channel", layer: "flow", rect: [100, 50, 200, 450] }, { path: "riser.0", layer: "flow", rect: [140, 0, 160, 50] }],
  translates: [{ path: "lane.left", outline: "tooth", origin: [75, 100], step: [0, 4], count: 50, quarter_turns: 0 }],
  ports: [{ path: "lane.inlet", at: [150, 50], width_um: 100, layer: "flow", facing: [0, -1] }],
  access: [],
  flows: [{ element: "lane", start: "a", end: "b", flow_ul_min: 0.2, velocity_mm_s: 1, shear_pa: 0.5, reynolds: 0.1 }],
  pressures_mbar: {},
  viscosity_mpa_s: 1,
  fractions: {},
  tunables: [],
  requirements: [],
  instances: [],
  drives: [],
  checks: [],
  findings: [],
  origins: { "lane": { file: "/d/demo.py", line: 12 }, "lane.left": { file: "/r/lane.py", line: 30 }, "riser": { file: "/d/demo.py", line: 14 } },
};

test("a view fits the chip, and a point goes to the canvas and back", () => {
  const view = fit(scene.size_um, 600, 400);
  assert.ok(Math.abs(view.scale - 0.552) < 1e-9);
  const [px, py] = toCanvas(view, 250, 125);
  const [x, y] = toDesign(view, px, py);
  assert.ok(Math.abs(x - 250) < 1e-9 && Math.abs(y - 125) < 1e-9);
  // Y runs upward in a design: the chip's lower edge is lower on the canvas than its upper.
  assert.ok(toCanvas(view, 0, 0)[1] > toCanvas(view, 0, 500)[1]);
  // With controls over the canvas's upper and lower edges, the chip is kept clear of them.
  const clear = fit(scene.size_um, 600, 400, 24, 60, 40);
  // The chip is 276 pixels high in the 300 left to it: twelve spare above and below.
  assert.ok(Math.abs(toCanvas(clear, 0, 500)[1] - 72) < 1e-9 && Math.abs(toCanvas(clear, 0, 0)[1] - 348) < 1e-9);
  const seen = visible(view);
  assert.ok(seen[0] < 0 && seen[2] > 1000 && seen[1] < 0 && seen[3] > 500);
});

test("zooming keeps the point under the pointer where it is", () => {
  const view = fit(scene.size_um, 600, 400);
  const before = toDesign(view, 200, 150);
  const after = toDesign(zoomed(view, 4, 200, 150), 200, 150);
  assert.ok(Math.abs(before[0] - after[0]) < 1e-9 && Math.abs(before[1] - after[1]) < 1e-9);
});

test("a row of rectangles has an extent, and only the part in view is drawn", () => {
  const row = scene.translates[0]!;
  const outline = scene.outlines.tooth!;
  assert.deepEqual(placed(row, outline), [[75, 100, 100, 101.2]]);
  assert.deepEqual(extent(row, outline), [75, 100, 100, 297.2]);
  assert.deepEqual(inView(row, outline, [0, 0, 1000, 500]), [0, 50]);
  const [from, to] = inView(row, outline, [0, 140, 1000, 160]);
  assert.ok(from <= 10 && from >= 8 && to >= 16 && to <= 17, `${from}..${to}`);
  assert.deepEqual(inView(row, outline, [0, 400, 1000, 500]).reduce((a, b) => b - a) <= 0, true);
  // Turned a quarter turn, a tooth stands upright to the left of its origin.
  assert.deepEqual(placed({ ...row, quarter_turns: 1 }, outline), [[73.8, 100, 75, 125]]);
});

test("everything named has a place, and an instance holds what was made under it", () => {
  const where = bounds(scene);
  assert.deepEqual(where.get("lane.channel"), [100, 50, 200, 450]);
  assert.deepEqual(where.get("lane.left"), [75, 100, 100, 297.2]);
  assert.deepEqual(where.get("lane"), [75, 50, 200, 450]);
  assert.deepEqual(where.get("lane.inlet"), [100, 0, 200, 100]);
});

test("what is under a point is the smallest thing there, and a hidden layer is passed through", () => {
  assert.equal(at(scene, new Set(), 150, 200), "lane.channel");
  assert.equal(at(scene, new Set(), 80, 100.5), "lane.left");
  assert.equal(at(scene, new Set(), 80, 103), undefined);
  assert.equal(at(scene, new Set(["trench"]), 80, 100.5), undefined);
  assert.equal(at(scene, new Set(), 900, 400), undefined);
});

test("a rectangle belongs to the edge its path is under", () => {
  const elements = new Set(["lane", "riser"]);
  assert.equal(owner("lane.left", elements), "lane");
  assert.equal(owner("riser.0", elements), "riser");
  assert.equal(owner("mark.cross", elements), undefined);
});

test("a value has a colour, a line has what it made, and a trial value keeps its unit", () => {
  assert.equal(ramp(0), "rgb(59, 111, 212)");
  assert.equal(ramp(1), "rgb(238, 119, 51)");
  assert.equal(ramp(share(5, 5, 5, false)), ramp(0.5));
  assert.deepEqual(madeOn(scene, "/d/demo.py", 12), ["lane"]);
  assert.deepEqual(madeOn(scene, "/d/demo.py", 13), []);
  assert.equal(written(412.6, "2600um"), "412.6um");
  assert.equal(written(8, "8"), "8");
});

test("values that span many times are told apart by their logarithm, and a legend marks round ones", () => {
  assert.equal(spansDecades(0.0014, 1.6), true);
  assert.equal(spansDecades(0.14, 0.16), false);
  assert.equal(spansDecades(0, 5), false);
  // Linearly, a hundredth of the way is nearly the least; by logarithm it is half way.
  assert.ok(share(0.1, 0.01, 1, false) < 0.1);
  assert.ok(Math.abs(share(0.1, 0.01, 1, true) - 0.5) < 1e-9);
  assert.deepEqual(legendTicks(0, 10, false), [0, 5, 10]);
  // A power of ten that would be written over an end is left to the end.
  assert.deepEqual(legendTicks(0.0014, 1.6, true), [0.0014, 0.01, 0.1, 1.6]);
  assert.deepEqual(legendTicks(0.001, 1000, true), [0.001, 0.01, 1, 100, 1000]);
});

test("a scale bar is a round length near the width asked for", () => {
  assert.deepEqual(scaleBar(0.04), { um: 2000, px: 80, label: "2 mm" });
  assert.equal(scaleBar(0.4).label, "200 µm");
  assert.equal(scaleBar(11).label, "10 µm");
  assert.equal(scaleBar(300).label, "200 nm");
  for (const scale of [0.003, 0.07, 1.3, 55, 900]) assert.ok(scaleBar(scale).px <= 110 && scaleBar(scale).px >= 40);
});

test("a click selects the outermost thing, then one step further in each time", () => {
  assert.deepEqual(ancestors("monitor.coarse.line[3]"), ["monitor", "monitor.coarse", "monitor.coarse.line[3]"]);
  assert.deepEqual(ancestors("faller[2]"), ["faller[2]"]);
  assert.ok(within("lane[1].left", "lane[1]") && within("lane[1]", "lane[1]") && within("riser[2].0", "riser[2]"));
  assert.ok(!within("lane[10].left", "lane[1]") && !within("lanes", "lane"));
  assert.equal(pick("lane[2].left", undefined), "lane[2]");
  assert.equal(pick("lane[2].left", "lane[2]"), "lane[2].left");
  assert.equal(pick("lane[2].left", "lane[2].left"), "lane[2].left");
  // Something else was selected: the click starts again from the outside.
  assert.equal(pick("lane[2].left", "feed"), "lane[2]");
  assert.equal(pick(undefined, "feed"), undefined);
  // Thin features are picked from just beside them.
  assert.equal(at(scene, new Set(), 139, 20), undefined);
  assert.equal(at(scene, new Set(), 139, 20, 2), "riser.0");
});

test("names go above what they name, a row is named as one when there is no room, and none overlap", () => {
  const instance = (path: string, rect: [number, number, number, number]) => ({ path, region: "r", rect, parameters: [], origin: null });
  const lanes = [0, 1, 2, 3].map((at) => instance(`lane[${at}]`, [100 + at * 40, 100, 110 + at * 40, 400]));
  const instances = [instance("feed", [100, 420, 300, 440]), ...lanes, instance("feed.trunk", [100, 420, 300, 430]), { ...instance("ghost", [0, 0, 0, 0]), rect: null }];
  const width = (text: string) => text.length * 6;
  assert.equal(holding(instances, 105, 200), "lane[0]");
  assert.equal(holding(instances, 50, 50), undefined);

  // Far out, forty micrometres apart is too close for `lane[0]`: the lanes are named as one.
  const far = fit([400, 500], 400, 500, 0);
  const named = nameable(instances, shownFlat(far), width);
  assert.deepEqual(named.map((name) => name.text).sort(), ["feed", "lane[0..3]"]);
  assert.deepEqual(named.find((name) => name.text === "lane[0..3]")!.rect, [100, 100, 230, 400]);
  // Close in, each has room, and what is inside the feed is named too: the largest first.
  const near = { ...far, scale: 14 };
  assert.deepEqual(nameable(instances, shownFlat(near), width).map((name) => name.text), ["feed", "lane[0]", "lane[1]", "lane[2]", "lane[3]", "trunk"]);

  const placed = labels(named, shownFlat(far), width);
  assert.deepEqual(placed.map((label) => label.text).sort(), ["feed", "lane[0..3]"]);
  const feed = placed.find((label) => label.text === "feed")!;
  assert.ok(Math.abs(feed.x - 100) < 1e-9 && Math.abs(feed.y - (500 - 440 - 5)) < 1e-9);
  // Two names for one place: the first keeps it, the second goes below, and a third is left out.
  const same = [0, 1, 2].map((at) => ({ path: `p${at}`, text: `p${at}`, rect: [100, 100, 300, 300] as [number, number, number, number] }));
  const stacked = labels(same, shownFlat(far), width);
  assert.deepEqual(stacked.map((label) => [label.path, label.y < 250]), [["p0", true], ["p1", false]]);
  // A box that runs off the upper edge is named where it comes into view.
  const tall = labels([{ path: "tall", text: "tall", rect: [100, 100, 300, 5000] }], shownFlat(far), width);
  assert.equal(tall[0]!.y, 16);
  // Something too small to point at is not named.
  assert.deepEqual(labels([{ path: "dot", text: "dot", rect: [10, 10, 12, 12] }], shownFlat(far), width), []);
});

import assert from "node:assert/strict";
import { test } from "node:test";

import type { Scene, SceneCheck, SceneFinding } from "../src/scene";
import { type Arrangement, arrangement, panelSize } from "../src/viewer/arrangement";
import { headline, listed, problems, shortened, standings, waived } from "../src/viewer/verification";

const check = (id: string, group: SceneCheck["group"], status = "satisfied", note: string | null = null): SceneCheck => ({ id, title: id, group, status, note, metrics: {} });
const finding = (id: string, group: SceneFinding["group"], message: string, waivedBy: string | null = null): SceneFinding => ({ check: id, group, rule: id, message, waived: waivedBy, paths: [], origin: null });
const scene = (checks: SceneCheck[], findings: SceneFinding[]): Scene => ({ checks, findings } as Scene);

test("with nothing wrong there are no problems, and the tool's own checks are not shown", () => {
  const held = scene([check("deterministic", "integrity"), check("rules", "fabrication"), check("graph", "function"), check("corners", "variation")], []);
  assert.deepEqual(problems(held), []);
  assert.deepEqual(standings(held).map((each) => [each.group, each.problems, each.ran]), [["fabrication", 0, true], ["function", 0, true], ["variation", 0, true]]);
  assert.equal(headline(0), "No problems");
});

test("problems are read by question, in the order the checks ran, whatever order they were found in", () => {
  const held = scene(
    [check("deterministic", "integrity", "violated"), check("rules", "fabrication", "violated"), check("requirement/a", "function", "violated"), check("corners", "variation", "violated")],
    [finding("corners", "variation", "c"), finding("deterministic", "integrity", "d"), finding("requirement/a", "function", "a"), finding("rules", "fabrication", "r1"), finding("rules", "fabrication", "r2")],
  );
  assert.deepEqual(problems(held).map((each) => each.message), ["r1", "r2", "a", "c", "d"]);
  assert.deepEqual(standings(held).map((each) => [each.group, each.problems]), [["fabrication", 2], ["function", 1], ["variation", 1], ["integrity", 1]]);
  assert.equal(headline(5), "5 problems");
  assert.equal(headline(1), "1 problem");
});

test("a check that did not run is one problem, and a waived finding is none", () => {
  const held = scene([check("graph", "function", "not_run", "the design has no `operate` block"), check("rules", "fabrication", "waived")], [finding("rules", "fabrication", "narrow", "accepted by a person")]);
  const found = problems(held);
  assert.deepEqual(found.map((each) => [each.check.id, each.message, each.finding === undefined]), [["graph", "the design has no `operate` block", true]]);
  assert.equal(waived(held).length, 1);
  assert.deepEqual(standings(held).map((each) => [each.group, each.problems, each.ran]), [["fabrication", 0, true], ["function", 1, false]]);
});

test("many problems of one check are folded until they are asked for", () => {
  const rules = check("rules", "fabrication", "violated");
  const held = scene([rules, check("corners", "variation", "violated")], [...Array.from({ length: 9 }, (_, at) => finding("rules", "fabrication", `r${at}`)), finding("corners", "variation", "c")]);
  const folded = listed(problems(held), 4, new Set());
  assert.deepEqual(folded.shown.map((each) => each.message), ["r0", "r1", "r2", "r3", "c"]);
  assert.deepEqual(folded.folded.map((each) => [each.check.id, each.more]), [["rules", 5]]);
  const opened = listed(problems(held), 4, new Set(["rules"]));
  assert.equal(opened.shown.length, 10);
  assert.deepEqual(opened.folded, []);
});

test("a long path keeps both its ends", () => {
  assert.equal(shortened("mixer.turn[3]", 20), "mixer.turn[3]");
  const short = shortened("feed.manifold.trunk.tap[12].mouth", 16);
  assert.equal(short.length, 16);
  assert.ok(short.startsWith("feed.") && short.endsWith("2].mouth") && short.includes("…"), short);
});

test("the arrangement follows the preview's width and height, in rem", () => {
  const rem = 13;
  const cases: [number, number, Arrangement][] = [
    [1600, 1000, "side"],
    [900, 1000, "side"],
    [520, 900, "stacked"],
    [423, 849, "stacked"],
    [340, 800, "stacked"],
    [1400, 300, "side"],
    [480, 320, "sheet"],
    [250, 700, "sheet"],
    [200, 140, "chip"],
    [1400, 120, "chip"],
  ];
  for (const [width, height, expected] of cases) assert.equal(arrangement(width, height, rem), expected, `${width} x ${height}`);
  // Zoomed in, the same pixels hold less: a preview that was side by side is stacked.
  assert.equal(arrangement(900, 1000, 13), "side");
  assert.equal(arrangement(900, 1000, 20), "stacked");
  // What the reader pins is kept wherever there is room for it, and given up where there is not.
  assert.equal(arrangement(600, 900, rem, "right"), "side");
  assert.equal(arrangement(1600, 1000, rem, "below"), "stacked");
  assert.equal(arrangement(300, 900, rem, "right"), "stacked");
  assert.equal(arrangement(1400, 250, rem, "below"), "side");
});

test("the panel is never smaller than it can be read at, nor larger than leaves the stage room", () => {
  const rem = 13;
  assert.equal(panelSize("side", 1600, 1000, rem), 32 * rem);
  assert.equal(panelSize("side", 900, 1000, rem), 21 * rem);
  assert.equal(panelSize("side", 900, 1000, rem, 50), 18 * rem);
  assert.equal(panelSize("side", 900, 1000, rem, 5000), 450);
  assert.equal(panelSize("stacked", 423, 849, rem, 5000), 849 - 14 * rem);
  assert.ok(panelSize("stacked", 423, 849, rem) >= 12 * rem);
});

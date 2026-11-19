// How the viewer writes numbers, units and names.

import assert from "node:assert/strict";
import { test } from "node:test";

import { length, number, parts, place, quantity, spans, unitOf, units, written } from "../src/viewer/format";

test("numbers are written short, and very small ones as a power of ten", () => {
  assert.equal(number(89.19993699975169), "89.2");
  assert.equal(number(0.0014405240475489456), "0.001441");
  assert.equal(number(0.000066346), "6.6×10⁻⁵");
  assert.equal(number(2613.463, 2), "2600");
  assert.equal(number(0), "0");
});

test("values carry their units as a person reads them", () => {
  assert.equal(quantity(0.152, "ul_min"), "0.152 µL/min");
  assert.equal(quantity(0.081, ""), "0.081");
  assert.equal(length(8672.154), "8.672 mm");
  assert.equal(length(27.377), "27.38 µm");
  assert.equal(length(0.5), "500 nm");
  assert.equal(written("27.377um"), "27.377 µm");
  assert.equal(written("41"), "41");
  assert.equal(unitOf("2600um"), "um");
  assert.equal(units("lane[0] is 0.0986ul_min, within 5.0% of 0.1474ul_min"), "lane[0] is 0.0986 µL/min, within 5.0% of 0.1474 µL/min");
});

test("paths, messages and places are cut where a person reads them", () => {
  assert.deepEqual(parts("sample_resistor.width"), { holder: "sample_resistor", leaf: "width" });
  assert.deepEqual(parts("pitch"), { holder: "", leaf: "pitch" });
  assert.deepEqual(spans("`pad_size` of `access_pad` is 1um"), [{ text: "pad_size", code: true }, { text: " of ", code: false }, { text: "access_pad", code: true }, { text: " is 1um", code: false }]);
  assert.equal(place("/work/designs/demo.py", 27), "demo.py:27");
});

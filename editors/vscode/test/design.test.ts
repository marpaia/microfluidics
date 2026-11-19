import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { designAt } from "../src/design";

test("a design is recognised by its class, and a file without one is not", () => {
  const source = readFileSync(new URL("../../../designs/mixer.py", import.meta.url), "utf8");
  const found = designAt(source);
  assert.equal(typeof found, "number");
  assert.ok(source.slice(found).startsWith("class Mixer(mf.Design):"));
  assert.equal(designAt("class Unrelated: pass"), undefined);
});

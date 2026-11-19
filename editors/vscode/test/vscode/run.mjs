// The preview, held to what it must do inside VS Code itself.
//
//   node test/vscode/run.mjs [directory for pictures]
//
// It starts a VS Code of its own (see `driver.mjs`), opens designs, and checks what the preview
// shows, how it is laid out at every size and zoom it can be given, and that selecting, trying
// and keeping values, a design that does not build, and a reload of the window all behave.
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { LAID_OUT, SHOWN } from "./checks.mjs";
import { Code, until } from "./driver.mjs";

const out = process.argv[2] ?? mkdtempSync(join(tmpdir(), "mf-vscode-pictures-"));
mkdirSync(out, { recursive: true });
const pause = (ms) => new Promise((done) => setTimeout(done, ms));

const failed = [];
let passed = 0;
function hold(what, ok, detail = "") {
  if (ok) passed += 1;
  else failed.push(`${what}${detail ? `: ${detail}` : ""}`);
  console.log(`${ok ? "ok  " : "FAIL"} ${what}${ok || !detail ? "" : `\n       ${detail}`}`);
}

/** Each size the preview is tried at, in its own pixels, and how it must be arranged there. */
const SIZES = [
  ["whole editor area", 1600, 1000, "side"],
  ["beside the code", 900, 1000, "side"],
  ["one of three columns", 520, 900, "stacked"],
  ["narrow column", 340, 800, "stacked"],
  ["strip under the code", 1400, 300, "side"],
  ["small corner", 480, 320, "sheet"],
  ["sliver", 200, 140, "chip"],
];
const ZOOMS = [-2, -1, 0, 2];

const code = await Code.start();
try {
  // The mixer: it meets its requirements as drawn and across fabrication, so nothing stands in its way.
  let frame = await code.preview("goenner_mixer.py");
  const scene = () => until("a scene", () => frame.evaluate("document.querySelector('.summary .standing') !== null && document.body.dataset.stale === 'false'"), 60000);
  await scene();
  await pause(800);
  let shown = await frame.evaluate(SHOWN);
  hold("the preview opens beside the code with the design's name", shown.design === "goenner_mixer", JSON.stringify(shown.design));
  hold("the chip is drawn in three dimensions", shown.device === "true");
  hold("the mixer has no problems, and every question is answered well", shown.standing === "No problems" && shown.marks.join() === "Fabrication,Function,Variation" && shown.problems.length === 0, `${shown.standing} ${shown.marks}`);
  hold("beside the code, in a window of the usual size, the panel is beneath the chip", shown.arrangement === "stacked" && shown.panel === "shown", `${shown.arrangement} at ${shown.size}`);
  hold("nothing is wrong with its layout as it opens", (await frame.evaluate(LAID_OUT)).length === 0, (await frame.evaluate(LAID_OUT)).join("; "));
  await code.picture(join(out, "window.png"), { whole: true });

  hold("the editor's Problems panel holds none for it", (await code.ask({ diagnostics: join(code.workspace, "goenner_mixer.py") })).diagnostics.length === 0);

  // Trying a value builds a trial and writes nothing; keeping it writes the tuned file.
  const tuned = join(code.workspace, "goenner_mixer.tuned.toml");
  const before = readFileSync(tuned, "utf8");
  await frame.evaluate("(() => { const slider = document.querySelector('.parameter input[type=range]'); slider.value = String(Number(slider.min) + (Number(slider.max) - Number(slider.min)) * 0.37); slider.dispatchEvent(new Event('input', { bubbles: true })); })()");
  shown = await until("the trial to be shown", async () => {
    const held = await frame.evaluate(SHOWN);
    return held.unsaved ? held : undefined;
  });
  hold("a value that is tried is said to be unsaved, and is not written", /1 unsaved value/.test(shown.unsaved) && readFileSync(tuned, "utf8") === before, shown.unsaved);
  hold("the bar to keep or discard it lies within the panel", (await frame.evaluate(LAID_OUT)).length === 0, (await frame.evaluate(LAID_OUT)).join("; "));
  await code.picture(join(out, "trying.png"));
  await frame.evaluate("document.querySelector('.trying .primary').click()");
  await until("the tuned file to change", async () => readFileSync(tuned, "utf8") !== before);
  shown = await until("the trial to be kept", async () => {
    const held = await frame.evaluate(SHOWN);
    return held.unsaved ? undefined : held;
  });
  hold("keeping it writes the tuned file and leaves nothing unsaved", shown.unsaved === null && readFileSync(tuned, "utf8") !== before);
  writeFileSync(tuned, before);
  await pause(1200);

  // A design that does not build says so, keeps the last chip, and says where.
  const design = join(code.workspace, "goenner_mixer.py");
  const source = readFileSync(design, "utf8");
  writeFileSync(design, source.replace("def build(self) -> None:", "def build(self) -> None:\n        raise ValueError('not today')"));
  shown = await until("the failure to be shown", async () => {
    const held = await frame.evaluate(SHOWN);
    return held.notice ? held : undefined;
  });
  hold("a design that does not build says so and keeps the last chip", /does not build/i.test(shown.standing ?? "") && /not today/.test(shown.notice) && shown.design === "goenner_mixer", `${shown.standing} | ${shown.notice}`);
  hold("the notice lies clear of the controls", (await frame.evaluate(LAID_OUT)).length === 0, (await frame.evaluate(LAID_OUT)).join("; "));
  await code.picture(join(out, "does-not-build.png"));
  writeFileSync(design, source);
  await until("the design to build again", async () => (await frame.evaluate(SHOWN)).notice === null);

  // The manifold as written: its lanes are unequal, and each lane that is out is on the chip.
  frame.close();
  await code.ask({ command: "workbench.action.closeAllEditors" });
  frame = await code.preview("untuned_manifold.py");
  await scene();
  await pause(800);
  shown = await frame.evaluate(SHOWN);
  hold("the manifold as written has problems of function and of variation", /^\d+ problems$/.test(shown.standing) && shown.marks[1] !== "Function" && shown.marks[2] !== "Variation", `${shown.standing} ${shown.marks}`);
  hold("a problem of variation says what is reached and what is allowed", shown.problems.some((said) => /^balanced reaches [\d.]+ ?\S+ with .*, and must be within 5 ?% of their mean, /.test(said)), shown.problems.join(" | "));
  // The editor's own places for problems say the same.
  const problems = await until("the Problems panel", async () => {
    const found = (await code.ask({ diagnostics: join(code.workspace, "untuned_manifold.py") })).diagnostics;
    return found.length ? found : undefined;
  });
  hold("the Problems panel holds them as errors, each under its question", problems.length >= 2 && problems.every((held) => held.severity === 0 && /^microfluidics: (function|variation)$/.test(held.source)), JSON.stringify(problems).slice(0, 300));
  await frame.evaluate("document.querySelector('#pane-problems .row.acts').click()");
  shown = await until("a selection", async () => {
    const held = await frame.evaluate(SHOWN);
    return held.selection ? held : undefined;
  });
  const editor = await code.ask({ editor: true });
  hold("choosing a problem selects what it is about, on the chip and in the panel", /^lane\[\d\]/.test(shown.selection), shown.selection);
  hold("and leaves the source open beside it", editor.visible.some((file) => file.endsWith("untuned_manifold.py")), JSON.stringify(editor));
  await code.picture(join(out, "selected.png"));
  // By the keyboard: the arrows move through the problems, and Enter chooses one.
  await frame.evaluate("document.querySelector('#pane-problems .row').focus()");
  const moved = await frame.evaluate("(() => { const list = document.querySelector('#pane-problems .list'); const key = (name) => document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key: name, bubbles: true })); key('ArrowDown'); key('ArrowDown'); const at = [...list.children].indexOf(document.activeElement); key('Enter'); return at; })()");
  hold("the arrow keys move through the problems", moved === 2, String(moved));
  await frame.evaluate("document.querySelector('#pane-selected [title^=\"Clear\"]').click()");
  hold("clearing the selection takes its section away", (await frame.evaluate(SHOWN)).selection === null);

  // Every size, at every zoom at which the test's own window can give it.
  await code.ask({ command: "workbench.action.closeSidebar" });
  await code.ask({ command: "workbench.action.closeAuxiliaryBar" });
  await code.ask({ command: "workbench.action.closePanel" });
  await code.ask({ command: "workbench.action.focusSecondEditorGroup" });
  await code.ask({ command: "workbench.action.joinAllGroups" });
  await frame.evaluate("document.querySelector('[title^=\"Show the whole chip\"]').click()");
  let ran = 0;
  for (const zoom of ZOOMS) {
    await code.ask({ settings: { "window.zoomLevel": zoom } });
    await pause(900);
    await frame.sized();
    const [wide, high] = await frame.room();
    for (const [name, width, height, expected] of SIZES) {
      if (width > wide || height > high) continue;
      ran += 1;
      const size = await frame.sized(width, height);
      // A window that lies behind another is drawn late: the preview is given time to settle.
      await until("the preview to settle", async () => (await frame.evaluate(SHOWN)).arrangement === expected, 6000).catch(() => undefined);
      await pause(250);
      const held = await frame.evaluate(SHOWN);
      const wrong = await frame.evaluate(LAID_OUT);
      const said = `${name}, ${width} x ${height} at zoom ${zoom}`;
      hold(`${said}: ${expected}`, held.arrangement === expected && size[0] === width && size[1] === height, `${held.arrangement} at ${size}, rem ${held.rem}, root ${held.root}`);
      hold(`${said}: laid out with nothing wrong`, wrong.length === 0, wrong.join("; "));
      if (zoom === 0 || (zoom === -2 && width > 1000)) await code.picture(join(out, `size-${width}x${height}-zoom${zoom}.png`), { size: [width, height] });
      // Where the panel lies over the chip, it opens from the foot and closes again.
      if (expected === "sheet") {
        await frame.evaluate("document.querySelector('.foot-bar').click()");
        await pause(200);
        const open = await frame.evaluate(SHOWN);
        hold(`${said}: the panel opens over the chip`, open.panel === "shown" && (await frame.evaluate(LAID_OUT)).length === 0, (await frame.evaluate(LAID_OUT)).join("; "));
        if (zoom === 0) await code.picture(join(out, `size-${width}x${height}-sheet.png`), { size: [width, height] });
        await frame.evaluate("document.querySelector('.sheet-close button').click()");
        hold(`${said}: and closes`, (await frame.evaluate(SHOWN)).panel === "hidden");
      }
    }
  }
  hold("the sizes were tried at more than one zoom", ran >= SIZES.length + 6, `${ran} ran`);
  // The splitter, from one end to the other, never lets either side be too small to use.
  await code.ask({ settings: { "window.zoomLevel": 0 } });
  await pause(900);
  await frame.sized(900, 640);
  for (const key of ["Home", "End", "Enter"]) {
    await frame.evaluate(`document.querySelector('#splitter').dispatchEvent(new KeyboardEvent('keydown', { key: '${key}', bubbles: true }))`);
    await pause(200);
    const wrong = await frame.evaluate(LAID_OUT);
    const panel = await frame.evaluate("document.querySelector('#panel').getBoundingClientRect().width");
    hold(`the splitter at ${key}: the panel is ${Math.round(panel)} px, and nothing is wrong`, wrong.length === 0 && panel >= 230 && panel <= 452, wrong.join("; "));
  }
  await frame.sized();

  // Light, and both high-contrast themes.
  for (const [theme, file] of [["Default Light Modern", "light"], ["Default High Contrast", "high-contrast"], ["Default High Contrast Light", "high-contrast-light"]]) {
    await code.ask({ settings: { "workbench.colorTheme": theme } });
    await pause(1500);
    hold(`in ${theme}, nothing is wrong`, (await frame.evaluate(LAID_OUT)).length === 0, (await frame.evaluate(LAID_OUT)).join("; "));
    await code.picture(join(out, `theme-${file}.png`));
  }
  await code.ask({ settings: { "workbench.colorTheme": "Default Dark Modern" } });

  hold("nothing was written to the console as an error", frame.console.filter((said) => said.kind === "error").length === 0, JSON.stringify(frame.console.filter((said) => said.kind === "error").slice(0, 3)));
} catch (error) {
  hold("the test ran to its end", false, String(error?.stack ?? error));
} finally {
  await code.stop();
}
// An editor as a person starts one, worked by its keys: the preview is opened from the Command
// Palette, and comes back on its design when the window is reloaded.
const plain = await Code.start({ hooked: false, open: "goenner_mixer.py" });
try {
  await pause(4000);
  await plain.command("Microfluidics: Open Preview");
  let frame = await until("the preview", () => plain.frame(), 40000);
  let shown = await until("its scene", async () => {
    const held = await frame.evaluate(SHOWN);
    return held.standing ? held : undefined;
  }, 60000);
  hold("the preview opens from the Command Palette", shown.design === "goenner_mixer" && shown.standing === "No problems", `${shown.design} ${shown.standing}`);
  frame.close();
  await plain.command("Developer: Reload Window");
  await pause(6000);
  frame = await until("the preview to come back", () => plain.frame(), 40000);
  shown = await until("its scene", async () => {
    const held = await frame.evaluate(SHOWN);
    return held.standing ? held : undefined;
  }, 60000);
  hold("after the window is reloaded, the preview is back on its design", shown.design === "goenner_mixer" && shown.standing === "No problems", `${shown.design} ${shown.standing}`);
  hold("and its page is of the build that runs in it", shown.notice === null, shown.notice ?? "");
} catch (error) {
  hold("the reload ran to its end", false, String(error?.stack ?? error));
} finally {
  await plain.stop();
}

console.log(`\n${passed} held, ${failed.length} did not. Pictures are in ${out}`);
for (const said of failed) console.log(`  FAIL ${said}`);
process.exit(failed.length ? 1 : 0);

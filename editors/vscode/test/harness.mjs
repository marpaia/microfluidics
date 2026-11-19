// Write a page that shows the viewer outside VS Code, with a scene given as a JSON file and a
// stand-in for the editor that answers what the viewer asks: for looking the viewer over in a
// browser.
//
//   node test/harness.mjs scene.json out/harness.html
//
// The page takes `?theme=light`, and `?state=failed` or `?state=building`.
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const [scenePath, target] = process.argv.slice(2);
const scene = readFileSync(scenePath, "utf8");
const viewer = readFileSync(new URL("../out/viewer.js", import.meta.url), "utf8");
const { page } = await import("../out/page.mjs");

const LIGHT = {
  foreground: "#3b3b3b", descriptionForeground: "#717171", "editor-background": "#ffffff", "editorWidget-background": "#f3f3f3", "editorWidget-border": "#c8c8c8",
  focusBorder: "#005fb8", "editorError-foreground": "#e51400", "editorWarning-foreground": "#bf8803", "testing-iconPassed": "#388a34",
  "button-background": "#005fb8", "button-foreground": "#ffffff", "button-secondaryBackground": "#e5e5e5", "button-secondaryForeground": "#3b3b3b", "button-secondaryHoverBackground": "#cccccc",
  "input-background": "#ffffff", "input-foreground": "#3b3b3b", "input-border": "#cecece", "textLink-foreground": "#005fb8", "badge-background": "#cccccc", "badge-foreground": "#3b3b3b",
  "inputValidation-errorBackground": "#f2dede", "inputValidation-errorBorder": "#be1100", "textCodeBlock-background": "rgba(0, 0, 0, 0.08)", "list-hoverBackground": "#f2f2f2", "sideBar-background": "#f8f8f8",
  "progressBar-background": "#005fb8",
};

const host = `
  const wanted = new URLSearchParams(location.search);
  if (wanted.get("theme") === "light") for (const [name, value] of Object.entries(${JSON.stringify(LIGHT)})) document.body.style.setProperty("--vscode-" + name, value);
  const kept = ${scene};
  let shown = kept, remembered;
  const tell = (message) => window.postMessage(message, "*");
  const built = (scene, trial) => { tell({ type: "status", state: "building" }); setTimeout(() => { tell({ type: "scene", scene, trial }); tell({ type: "status", state: "ready", build_ms: 33 }); }, 60); };
  const asked = [];
  // The editor's part: trial values come back as the scene built with them, and keeping them
  // makes that scene the one that is kept.
  window.acquireVsCodeApi = () => ({
    getState: () => remembered,
    setState: (state) => { remembered = state; },
    postMessage(message) {
      asked.push(message);
      document.title = "asked " + JSON.stringify(message);
      if (message.type === "set_overrides") {
        const values = message.values;
        shown = { ...kept, tunables: kept.tunables.map((held) => (held.path in values ? { ...held, value: parseFloat(values[held.path]), written: values[held.path] } : held)) };
        built(shown, Object.keys(values).length > 0);
      } else if (message.type === "keep") {
        Object.assign(kept, shown);
        built(kept, false);
      } else if (message.type === "run") {
        setTimeout(() => tell({ type: "result", command: message.command, ok: true, text: kept.design + ": every requirement holds\\n  3 values tuned and kept" }), 400);
      }
    },
  });
`;
const after = `
  if (wanted.get("state") !== "building") { tell({ type: "scene", scene: kept, trial: false }); tell({ type: "status", state: "ready", build_ms: 33 }); }
  if (wanted.get("state") === "failed") tell({ type: "status", state: "failed", file: "/work/designs/demo.py", line: 27, message: "\`pad_size\` of \`access_pad\` is 1um, outside 300um..3000um" });
  if (wanted.get("highlight")) tell({ type: "highlight", paths: wanted.get("highlight").split(",") });
`;
// VS Code's icons are served from beside the page.
for (const file of ["codicon.css", "codicon.ttf"]) copyFileSync(new URL(`../out/${file}`, import.meta.url), join(dirname(target), file));
const html = page("viewer.js", "codicon.css", "'self'", "harness", "/work/designs/demo.py")
  .replace(/<meta http-equiv="Content-Security-Policy"[^>]*>/, "")
  .replace(/<script nonce="harness" src="viewer.js"><\/script>/, () => `<script>${host}</script>\n<script>${viewer}</script>\n<script>${after}</script>`);
writeFileSync(target, html);

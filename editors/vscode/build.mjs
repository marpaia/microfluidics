// Bundle the extension, which runs in VS Code's Node, and the viewer, which runs in a webview.
// The viewer's page is built on its own as well, for the harness that shows it in a browser.
import { copyFileSync, mkdirSync } from "node:fs";

import { build } from "esbuild";

// The page and the viewer's script carry the mark of the build that made them, so that a page
// made by one build knows when the script it was given is of another.
const shared = { bundle: true, sourcemap: true, logLevel: "info", define: { BUILD: JSON.stringify(Date.now().toString(36)) } };
await build({ ...shared, entryPoints: ["src/extension.ts"], outfile: "out/extension.js", platform: "node", format: "cjs", external: ["vscode"] });
await build({ ...shared, entryPoints: ["src/viewer/main.tsx"], outfile: "out/viewer.js", platform: "browser", format: "iife", jsx: "automatic", jsxImportSource: "preact", minify: true });
await build({ ...shared, sourcemap: false, entryPoints: ["src/viewer/page.ts"], outfile: "out/page.mjs", platform: "node", format: "esm" });

// VS Code's icons, as the font and the stylesheet that names its glyphs.
mkdirSync("out", { recursive: true });
for (const file of ["codicon.css", "codicon.ttf"]) copyFileSync(`node_modules/@vscode/codicons/dist/${file}`, `out/${file}`);

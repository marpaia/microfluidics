// The page the viewer runs in: its frame and its styles, which take their colours from the
// editor's theme.

import { PANEL } from "./styles/panel";
import { STAGE } from "./styles/stage";
import { TOKENS } from "./styles/tokens";

declare const BUILD: string;

/** What marks a page and a viewer script as of one build of the extension. */
export const build: string = BUILD;

/** The page. `script` and `icons` are where the viewer's script and the icon font's stylesheet
 * are served from, `source` is what the page may load from, and `design` is the file it shows. */
export function page(script: string, icons: string, source: string, nonce: string, design = ""): string {
  const quoted = design.replace(/&/g, "&amp;").replace(/"/g, "&quot;");
  return `<!DOCTYPE html>
<html lang="en" data-build="${BUILD}" data-design="${quoted}">
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${source} 'unsafe-inline'; font-src ${source}; script-src 'nonce-${nonce}';">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Chip</title>
<link rel="stylesheet" href="${icons}">
<style>${TOKENS}${STAGE}${PANEL}</style>
</head>
<body data-arrangement="side" data-panel="hidden">
<div id="stage"><canvas id="space" aria-hidden="true"></canvas><canvas id="chip" tabindex="0" aria-label="The chip. Arrow keys move it, plus and minus zoom, F shows what is selected, Escape clears the selection."></canvas><div id="over"></div></div>
<div id="splitter" role="separator" tabindex="0" aria-label="The size of the panel. Arrow keys change it; Enter gives it its usual size."></div>
<aside id="panel" aria-label="Verification, requirements and parameters"></aside>
<div id="foot"></div>
<script nonce="${nonce}" src="${script}"></script>
</body>
</html>`;
}

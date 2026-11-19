// What every part of the preview is drawn with: the editor's own colours, by what they are for,
// and the few sizes everything is a multiple of. One rem is the editor's font size, so the
// editor's zoom and font scale the whole preview together.

export const TOKENS = `
  html { font-size: var(--vscode-font-size, 13px); height: 100%; }
  :root {
    --row: 1.7rem;
    --gap: 0.6rem;
    --edge: 0.75rem;
    --fg: var(--vscode-foreground, #cccccc);
    --quiet: var(--vscode-descriptionForeground, #9d9d9d);
    --ground: var(--vscode-editor-background, #1f1f1f);
    --panel-ground: var(--vscode-sideBar-background, #181818);
    --line: var(--vscode-contrastBorder, var(--vscode-sideBarSectionHeader-border, var(--vscode-editorWidget-border, #2b2b2b)));
    --widget: var(--vscode-editorWidget-background, #202020);
    --widget-line: var(--vscode-contrastBorder, var(--vscode-editorWidget-border, #313131));
    --shadow: var(--vscode-widget-shadow, rgba(0, 0, 0, 0.36));
    --hover: var(--vscode-list-hoverBackground, rgba(128, 128, 128, 0.12));
    --tool-hover: var(--vscode-toolbar-hoverBackground, rgba(90, 93, 94, 0.31));
    --focus: var(--vscode-focusBorder, #0078d4);
    --active: var(--vscode-contrastActiveBorder, transparent);
    --bad: var(--vscode-editorError-foreground, #f14c4c);
    --warn: var(--vscode-editorWarning-foreground, #cca700);
    --good: var(--vscode-testing-iconPassed, #73c991);
    --link: var(--vscode-textLink-foreground, #4daafc);
    --accent: var(--vscode-button-background, #0078d4);
    --fill: var(--vscode-contrastActiveBorder, var(--vscode-button-background, #0078d4));
    --on-accent: var(--vscode-button-foreground, #ffffff);
    --badge: var(--vscode-badge-background, #616161);
    --on-badge: var(--vscode-badge-foreground, #f8f8f8);
    --input: var(--vscode-input-background, #313131);
    --input-line: var(--vscode-contrastBorder, var(--vscode-input-border, #3c3c3c));
    --code: var(--vscode-textCodeBlock-background, rgba(128, 128, 128, 0.18));
  }
  * { box-sizing: border-box; }
  body { height: 100%; margin: 0; padding: 0; font: 1rem/1.4 var(--vscode-font-family, system-ui, sans-serif); color: var(--fg); background: var(--ground); overflow: hidden; }
  p, h1, h3, ul, dl, dd { margin: 0; padding: 0; }
  ul { list-style: none; }
  code, pre { font-family: var(--vscode-editor-font-family, ui-monospace, monospace); font-size: 0.92em; }
  code { background: var(--code); padding: 0 0.25em; border-radius: 3px; overflow-wrap: anywhere; }
  .quiet { color: var(--quiet); }
  .good { color: var(--good); }
  .bad { color: var(--bad); }
  .warn { color: var(--warn); }
  .codicon { font-size: 1.23rem; line-height: 1; flex: none; }
  .codicon-blank::before { content: ""; display: inline-block; width: 1em; }
  .status { display: inline-flex; flex: none; }

  button { font: inherit; color: inherit; cursor: pointer; margin: 0; }
  button:disabled { opacity: 0.5; cursor: default; }
  button:not([class]), button.primary { min-height: var(--row); background: var(--vscode-button-secondaryBackground, #313131); color: var(--vscode-button-secondaryForeground, #cccccc); border: 1px solid var(--vscode-contrastBorder, var(--vscode-button-border, transparent)); border-radius: 4px; padding: 0.15rem 0.85rem; display: inline-flex; align-items: center; justify-content: center; gap: 0.35rem; }
  button:not([class]):hover:not(:disabled) { background: var(--vscode-button-secondaryHoverBackground, #3c3c3c); }
  button.primary { background: var(--accent); color: var(--on-accent); }
  button.primary:hover:not(:disabled) { background: var(--vscode-button-hoverBackground, #026ec1); }
  button.link { background: none; border: 0; padding: 0; color: var(--link); text-align: left; display: inline-flex; align-items: center; gap: 0.3rem; min-height: var(--row); }
  button.link:hover:not(:disabled) { text-decoration: underline; }
  .icon-button { width: var(--row); height: var(--row); flex: none; display: inline-grid; place-items: center; padding: 0; background: none; border: 1px solid transparent; border-radius: 5px; color: var(--vscode-icon-foreground, var(--fg)); }
  .icon-button:hover:not(:disabled) { background: var(--tool-hover); outline: 1px dashed var(--active); outline-offset: -1px; }
  .icon-button[aria-pressed="true"] { background: var(--vscode-inputOption-activeBackground, rgba(36, 137, 219, 0.3)); border-color: var(--vscode-inputOption-activeBorder, transparent); color: var(--vscode-inputOption-activeForeground, var(--fg)); }
  :focus { outline: none; }
  :focus-visible { outline: 1px solid var(--focus); outline-offset: -1px; }

  .badge { flex: none; min-width: 1.4rem; height: 1.4rem; padding: 0 0.45rem; border-radius: 0.7rem; display: inline-grid; place-items: center; background: var(--badge); color: var(--on-badge); font-size: 0.85rem; font-weight: 400; font-variant-numeric: tabular-nums; white-space: nowrap; border: 1px solid var(--vscode-contrastBorder, transparent); }
  .badge.bad { background: var(--bad); color: var(--ground); font-weight: 600; }
  .badge.warn { background: var(--warn); color: var(--ground); font-weight: 600; }
  .chip-note { font-size: 0.85rem; padding: 0 0.45rem; border: 1px solid currentColor; border-radius: 0.7rem; white-space: nowrap; font-weight: 400; }

  input[type="number"] { font: inherit; font-variant-numeric: tabular-nums; color: var(--vscode-input-foreground, var(--fg)); background: var(--input); border: 1px solid var(--input-line); border-radius: 2px; padding: 0.1rem 0.3rem; height: var(--row); min-width: 0; text-align: right; }
  input[type="number"]::-webkit-inner-spin-button { display: none; }
  input[type="number"]:focus { outline: 1px solid var(--focus); outline-offset: -1px; }
  input[type="range"] { appearance: none; -webkit-appearance: none; background: none; margin: 0; height: var(--row); min-width: 0; cursor: pointer; }
  input[type="range"]::-webkit-slider-runnable-track { height: 2px; border-radius: 1px; background: linear-gradient(to right, var(--fill) var(--at, 0%), var(--vscode-scrollbarSlider-activeBackground, rgba(128, 128, 128, 0.5)) var(--at, 0%)); }
  input[type="range"]::-webkit-slider-thumb { -webkit-appearance: none; width: 0.92rem; height: 0.92rem; margin-top: calc(1px - 0.46rem); border-radius: 50%; background: var(--fg); border: 2px solid var(--panel-ground); box-shadow: 0 0 0 1px var(--vscode-contrastBorder, transparent); }
  input[type="range"]:focus-visible { outline: none; }
  input[type="range"]:focus-visible::-webkit-slider-thumb { background: var(--focus); }
  ::-webkit-scrollbar { width: 10px; height: 10px; }
  ::-webkit-scrollbar-thumb { background: var(--vscode-scrollbarSlider-background, rgba(121, 121, 121, 0.4)); }
  ::-webkit-scrollbar-thumb:hover { background: var(--vscode-scrollbarSlider-hoverBackground, rgba(100, 100, 100, 0.7)); }
  ::-webkit-scrollbar-corner { background: transparent; }
`;

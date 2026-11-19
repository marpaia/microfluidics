// The stage: the chip, and the controls that lie over its edges. The preview's arrangement is
// set on the body; the controls answer to the stage's own width.

export const STAGE = `
  body { display: grid; grid-template: "stage" minmax(0, 1fr) / minmax(0, 1fr); }
  #stage { grid-area: stage; position: relative; min-width: 0; min-height: 0; container: stage / size; overflow: hidden; }
  #splitter, #panel, #foot { display: none; }
  body[data-arrangement="side"][data-panel="shown"] { grid-template: "stage splitter panel" minmax(0, 1fr) / minmax(0, 1fr) 0 var(--panel, 22rem); }
  body[data-arrangement="stacked"][data-panel="shown"] { grid-template: "stage" minmax(0, 1fr) "splitter" 0 "panel" var(--panel, 40%) / minmax(0, 1fr); }
  body[data-arrangement="sheet"] { grid-template: "stage" minmax(0, 1fr) "foot" auto / minmax(0, 1fr); }
  body[data-panel="shown"] #panel { display: flex; }
  body:is([data-arrangement="side"], [data-arrangement="stacked"])[data-panel="shown"] #splitter { display: block; }
  body[data-arrangement="sheet"] #foot { display: block; grid-area: foot; }
  body[data-arrangement="sheet"][data-panel="shown"] #panel { grid-area: 1 / 1 / -1 / -1; z-index: 5; }
  body[data-arrangement="sheet"][data-panel="shown"] #foot { display: none; }

  #splitter { grid-area: splitter; position: relative; z-index: 4; touch-action: none; }
  #splitter::before { content: ""; position: absolute; background: transparent; transition: background 0.1s 0.15s; }
  body[data-arrangement="side"] #splitter { cursor: col-resize; }
  body[data-arrangement="side"] #splitter::before { inset: 0 -2px 0 -2px; width: 4px; }
  body[data-arrangement="stacked"] #splitter { cursor: row-resize; }
  body[data-arrangement="stacked"] #splitter::before { inset: -2px 0 -2px 0; height: 4px; }
  #splitter:hover::before, #splitter:focus-visible::before, body[data-dragging] #splitter::before { background: var(--vscode-sash-hoverBorder, var(--focus)); }
  #splitter:focus-visible { outline: none; }
  body[data-dragging] { user-select: none; }
  body[data-dragging] #stage, body[data-dragging] #panel { pointer-events: none; }

  #chip, #space { position: absolute; inset: 0; width: 100%; height: 100%; display: block; }
  #chip { cursor: default; touch-action: none; }
  #chip:focus-visible { outline: 1px solid var(--focus); outline-offset: -1px; }
  body[data-device="false"] #space { visibility: hidden; }
  #over > * { position: absolute; }

  .toolbar, .seeing { left: 0.5rem; right: 0.5rem; display: flex; gap: 0.4rem; align-items: center; pointer-events: none; overflow: hidden; padding: 2px; margin: -2px; }
  .toolbar { top: 0.5rem; }
  .seeing { bottom: 0.5rem; }
  .toolbar > *, .seeing > * { pointer-events: auto; flex: none; }
  .toolbar:has(.menu.open), .seeing:has(.menu.open) { overflow: visible; }
  .spacer { flex: 1 1 0 !important; min-width: 0; pointer-events: none !important; }
  .toolbar > .icon-button, .seeing .icon-button, .toolbar .menu > button, .seeing .menu > button { background: var(--widget); border: 1px solid var(--widget-line); }
  .toolbar > .icon-button:hover:not(:disabled), .seeing .icon-button:hover:not(:disabled), .menu > button:hover { background: color-mix(in srgb, var(--widget) 80%, var(--fg)); }
  .toolbar .chip-note { background: var(--widget); }
  .group { display: inline-flex; gap: 0; border-radius: 5px; }
  .group .icon-button { border-radius: 0; }
  .group .icon-button + .icon-button { border-left: 0; }
  .group .icon-button:first-child { border-radius: 5px 0 0 5px; }
  .group .icon-button:last-child { border-radius: 0 5px 5px 0; }

  .segmented { display: inline-flex; border: 1px solid var(--widget-line); border-radius: 5px; overflow: hidden; background: var(--widget); height: var(--row); }
  .segmented button { border: 0; border-radius: 0; background: none; color: var(--fg); padding: 0 0.7rem; min-width: 2.2rem; }
  .segmented button + button { border-left: 1px solid var(--widget-line); }
  .segmented button:hover:not(:disabled):not([aria-checked="true"]) { background: var(--tool-hover); }
  .segmented button[aria-checked="true"] { background: var(--accent); color: var(--on-accent); box-shadow: inset 0 0 0 2px var(--vscode-contrastActiveBorder, transparent); }
  .segmented button:focus-visible { outline-offset: -2px; }

  .menu { position: relative; }
  .menu-button { height: var(--row); display: inline-flex; align-items: center; gap: 0.3rem; padding: 0 0.3rem 0 0.45rem; border-radius: 5px; color: var(--fg); white-space: nowrap; }
  .menu-button .codicon-chevron-down { font-size: 0.9rem; color: var(--quiet); }
  .menu-sheet { position: absolute; top: calc(100% + 3px); left: 0; z-index: 10; min-width: 13rem; max-width: min(22rem, calc(100cqw - 1rem)); max-height: min(28rem, calc(100cqh - 4rem)); overflow-y: auto; padding: 0.3rem; display: grid; background: var(--vscode-menu-background, var(--widget)); color: var(--vscode-menu-foreground, var(--fg)); border: 1px solid var(--vscode-contrastBorder, var(--vscode-menu-border, var(--widget-line))); border-radius: 6px; box-shadow: 0 2px 8px var(--shadow); }
  .menu-sheet.end { left: auto; right: 0; }
  .menu-sheet.up { top: auto; bottom: calc(100% + 3px); }
  .menu-heading { padding: 0.35rem 0.5rem 0.15rem; font-size: 0.85rem; color: var(--quiet); display: flex; gap: 0.5rem; justify-content: space-between; }
  .menu-heading:not(:first-child) { border-top: 1px solid var(--vscode-menu-separatorBackground, var(--widget-line)); margin-top: 0.3rem; padding-top: 0.5rem; }
  .menu-item { all: unset; box-sizing: border-box; min-height: var(--row); display: flex; align-items: center; gap: 0.4rem; padding: 0 0.6rem 0 0.3rem; border-radius: 4px; cursor: pointer; white-space: nowrap; }
  .menu-item:disabled { opacity: 0.5; cursor: default; }
  .menu-item:hover:not(:disabled), .menu-item:focus-visible { background: var(--vscode-menu-selectionBackground, var(--accent)); color: var(--vscode-menu-selectionForeground, var(--on-accent)); outline: 1px solid var(--active); outline-offset: -1px; }
  .menu-item:hover .quiet, .menu-item:focus-visible .quiet { color: inherit; }
  .menu-item .codicon { font-size: 1.1rem; }
  .menu-item .end { margin-left: auto; padding-left: 1rem; font-variant-numeric: tabular-nums; }
  .menu-range { display: flex; align-items: center; gap: 0.5rem; padding: 0 0.6rem 0 0.5rem; }
  .menu-range input { flex: 1; }
  .menu-range span { min-width: 2.6rem; text-align: right; font-variant-numeric: tabular-nums; }
  .swatch { width: 0.75rem; height: 0.75rem; border-radius: 2px; flex: none; border: 1px solid var(--vscode-contrastBorder, transparent); }

  .readout { left: 0.9rem; bottom: 2.9rem; max-width: calc(100% - 1.8rem); font-size: 0.85rem; color: var(--quiet); pointer-events: none; font-variant-numeric: tabular-nums; display: flex; gap: 0.5rem; align-items: baseline; white-space: nowrap; overflow: hidden; }
  .readout code { overflow: hidden; text-overflow: ellipsis; }
  .legend { left: 50%; bottom: 0.5rem; transform: translateX(-50%); width: min(17rem, calc(100cqw - 20rem)); display: grid; gap: 1px; padding: 0.35rem 0.6rem 0.25rem; background: var(--widget); border: 1px solid var(--widget-line); border-radius: 5px; font-size: 0.85rem; }
  .legend .bar { height: 0.55rem; border-radius: 2px; }
  .legend .ticks { position: relative; height: 1.25em; font-variant-numeric: tabular-nums; }
  .legend .ticks span { position: absolute; transform: translateX(-50%); white-space: nowrap; }
  .legend .ticks span:first-child { transform: none; }
  .legend .ticks span:last-child { transform: translateX(-100%); }
  .legend .unit { color: var(--quiet); text-align: center; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  /* Where the legend cannot sit between the readout and the controls, it has a line of its own
     above them, and gives up its middle marks before it is too narrow to read. */
  @container stage (max-width: 44rem) {
    .legend { bottom: 2.8rem; width: min(17rem, calc(100cqw - 1.2rem)); }
    .readout { bottom: 7rem; }
    body[data-mode="layout"] .readout { bottom: 2.9rem; }
  }
  @container stage (max-width: 20rem) { .legend .ticks .mid, .legend .unit { display: none; } }
  @container stage (max-height: 18rem) { .legend { display: none; } .readout { bottom: 2.9rem; } }

  .notice { top: 3rem; left: 50%; transform: translateX(-50%); width: min(40rem, calc(100% - 1.2rem)); max-height: calc(100% - 6.5rem); overflow: auto; padding: 0.7rem 0.85rem; background: var(--vscode-notifications-background, var(--widget)); color: var(--vscode-notifications-foreground, var(--fg)); border: 1px solid var(--vscode-contrastBorder, var(--vscode-notifications-border, var(--widget-line))); border-radius: 6px; box-shadow: 0 2px 8px var(--shadow); display: grid; gap: 0.5rem; z-index: 3; }
  .notice.bad { border-left: 3px solid var(--bad); }
  .notice.quiet-notice { width: auto; box-shadow: none; background: none; border: 0; color: var(--quiet); top: 50%; }
  .notice pre { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; max-height: 14rem; overflow: auto; }
  .notice .buttons { display: flex; flex-wrap: wrap; gap: 0.4rem; }
  .progress { inset: 0 0 auto 0; height: 2px; overflow: hidden; z-index: 6; }
  .progress::after { content: ""; position: absolute; inset: 0 60% 0 0; background: var(--vscode-progressBar-background, #0078d4); animation: passing 1.1s linear infinite; }
  @keyframes passing { from { transform: translateX(-100%); } to { transform: translateX(260%); } }
  @media (prefers-reduced-motion: reduce) { .progress::after { animation: none; inset: 0; } .codicon-modifier-spin { animation: none !important; } }
  .chip-mark { top: 0.3rem; left: 0.3rem; display: inline-flex; padding: 0.15rem; border-radius: 50%; background: var(--widget); }

  .foot-bar { all: unset; box-sizing: border-box; width: 100%; min-height: calc(var(--row) + 0.3rem); padding: 0 0.6rem; display: flex; align-items: center; gap: 0.45rem; cursor: pointer; border-top: 1px solid var(--line); background: var(--panel-ground); white-space: nowrap; }
  .foot-bar:hover { background: color-mix(in srgb, var(--panel-ground) 90%, var(--fg)); }
  .foot-bar:focus-visible { outline: 1px solid var(--focus); outline-offset: -1px; }
  .foot-bar > span { display: inline-flex; }
  .foot-bar .said { font-weight: 600; }
  .foot-bar .name { flex: 1 1 0; min-width: 0; overflow: hidden; text-overflow: ellipsis; display: block; }
`;

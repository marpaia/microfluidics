// The panel: sections as the sidebar's own views are, and rows that answer to the panel's own
// width, so that they are right however the panel came to be narrow.

export const PANEL = `
  #panel { grid-area: panel; flex-direction: column; min-width: 0; min-height: 0; background: var(--panel-ground); color: var(--vscode-sideBar-foreground, var(--fg)); container: panel / inline-size; position: relative; }
  body[data-arrangement="side"] #panel { border-left: 1px solid var(--line); }
  body[data-arrangement="stacked"] #panel { border-top: 1px solid var(--line); }
  .panes { flex: 1 1 0; min-height: 0; overflow-y: auto; overflow-x: hidden; }
  .sheet-close { position: absolute; top: 0.4rem; right: 0.4rem; z-index: 2; }

  .summary { flex: none; padding: 0.6rem var(--edge) 0.65rem; display: grid; gap: 0.3rem; border-bottom: 1px solid var(--line); }
  .summary h1 { font-size: 1rem; font-weight: 600; display: flex; align-items: center; gap: 0.4rem; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  body[data-arrangement="sheet"] .summary h1 { padding-right: 2rem; }
  .summary h1 .codicon { font-size: 1rem; color: var(--quiet); }
  .standing { display: flex; align-items: center; flex-wrap: wrap; gap: 0.2rem 0.4rem; font-size: 1.15rem; font-weight: 600; line-height: 1.3; }
  .standing .codicon { font-size: 1.3rem; }
  .standing .built { margin-left: auto; font-size: 0.85rem; font-weight: 400; color: var(--quiet); white-space: nowrap; }
  .standings { display: flex; flex-wrap: wrap; gap: 0.15rem 0.2rem; margin: 0.05rem -0.3rem -0.15rem; }
  .standing-mark { all: unset; box-sizing: border-box; cursor: pointer; display: inline-flex; align-items: center; gap: 0.3rem; min-height: var(--row); padding: 0 0.4rem 0 0.3rem; border-radius: 4px; color: var(--fg); white-space: nowrap; }
  .standing-mark .codicon { font-size: 1.1rem; }
  .standing-mark.good .codicon { color: var(--good); }
  .standing-mark.bad .codicon, .standing-mark.bad .n { color: var(--bad); }
  .standing-mark.warn .codicon, .standing-mark.warn .n { color: var(--warn); }
  .standing-mark .n { font-variant-numeric: tabular-nums; font-weight: 600; }
  .standing-mark:hover { background: var(--hover); outline: 1px dashed var(--active); outline-offset: -1px; }
  .standing-mark:focus-visible { outline: 1px solid var(--focus); outline-offset: -1px; }
  .result { display: flex; align-items: flex-start; gap: 0.3rem; margin-top: 0.2rem; border: 1px solid var(--widget-line); border-left: 3px solid var(--good); border-radius: 3px; padding: 0.3rem 0.2rem 0.3rem 0.55rem; }
  .result.bad { border-left-color: var(--bad); }
  .result pre { flex: 1; margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; max-height: 11rem; overflow: auto; }
  body[data-stale="true"] .panes { opacity: 0.55; }

  .pane { border-bottom: 1px solid var(--line); }
  .pane-head { position: sticky; top: 0; z-index: 1; display: flex; align-items: center; gap: 0.2rem; height: var(--row); padding-right: 0.5rem; background: var(--vscode-sideBarSectionHeader-background, var(--panel-ground)); color: var(--vscode-sideBarSectionHeader-foreground, var(--fg)); }
  .pane-title { all: unset; box-sizing: border-box; flex: 1 1 0; min-width: 0; height: 100%; display: flex; align-items: center; gap: 0.15rem; padding-left: 0.2rem; cursor: pointer; }
  .pane-title .codicon { font-size: 1.23rem; }
  .pane-title .name { font-size: 0.85rem; font-weight: 700; text-transform: uppercase; white-space: nowrap; }
  .pane-title .note { margin-left: 0.5rem; color: var(--quiet); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .pane-title:focus-visible { outline: 1px solid var(--focus); outline-offset: -1px; }
  .pane-actions { display: inline-flex; opacity: 0; }
  .pane:hover .pane-actions, .pane:focus-within .pane-actions { opacity: 1; }
  .pane-body { padding: 0.15rem 0 0.6rem; }
  .pane-body > p, .pane-body > .tune, .pane-body > .fold, h3.in-pane { margin: 0.3rem var(--edge) 0; }
  .empty { display: flex; align-items: center; gap: 0.4rem; min-height: var(--row); color: var(--fg); }
  .empty .codicon { color: var(--good); }
  h3 { font-size: 0.85rem; font-weight: 600; color: var(--quiet); }
  h3.in-pane { margin-top: 0.6rem; }

  .list { display: grid; }
  .row { position: relative; display: flex; align-items: flex-start; gap: 0.4rem; min-height: var(--row); padding: 0.2rem var(--edge) 0.2rem 0.6rem; }
  .row.acts { cursor: pointer; }
  .row:hover { background: var(--hover); outline: 1px dashed var(--active); outline-offset: -1px; }
  .row:focus-visible { outline: 1px solid var(--vscode-list-focusOutline, var(--focus)); outline-offset: -1px; background: var(--vscode-list-inactiveSelectionBackground, var(--hover)); }
  .row .mark { flex: none; width: 1.3rem; height: 1.3rem; display: grid; place-items: center; margin-top: 0.02rem; }
  .row.tone-bad .mark { color: var(--bad); }
  .row.tone-warn .mark { color: var(--warn); }
  .pin { width: 1.3rem; height: 1.3rem; border-radius: 50%; background: var(--bad); color: var(--ground); font-size: 0.78rem; font-weight: 700; display: grid; place-items: center; font-variant-numeric: tabular-nums; }
  .row .text { flex: 1 1 0; min-width: 0; display: grid; gap: 0.05rem; }
  .row .message { overflow-wrap: anywhere; }
  .row .where { display: flex; flex-wrap: wrap; gap: 0 0.45rem; color: var(--quiet); font-size: 0.85rem; }
  .row .where > * + *::before { content: "·"; margin-right: 0.45rem; }
  .row .where code { background: none; padding: 0; }
  .row-actions { position: absolute; top: 0.05rem; right: 0.35rem; display: none; background: var(--panel-ground); border-radius: 5px; box-shadow: -0.5rem 0 0.4rem var(--panel-ground); }
  .row:hover .row-actions, .row:focus-within .row-actions { display: inline-flex; }
  .fold { min-height: var(--row); }

  .more { margin-top: 0.25rem; }
  .more > summary { list-style: none; cursor: pointer; display: flex; align-items: center; gap: 0.15rem; min-height: var(--row); padding: 0 var(--edge) 0 0.2rem; color: var(--fg); }
  .more > summary::-webkit-details-marker { display: none; }
  .more > summary > span { display: flex; gap: 0.5rem; align-items: baseline; flex-wrap: wrap; }
  .more > summary:hover { background: var(--hover); }
  .more > summary:focus-visible { outline: 1px solid var(--focus); outline-offset: -1px; }
  .more[open] > summary .codicon-chevron-right { transform: rotate(90deg); }
  .plain-list { display: grid; gap: 0.1rem; padding: 0 var(--edge) 0 1.6rem; }
  .verified { display: grid; gap: 0.5rem; padding-top: 0.1rem; }
  .verified h3 { padding: 0 var(--edge) 0.1rem 1.6rem; display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: baseline; color: var(--fg); }
  .verified h3 .quiet { font-weight: 400; }
  .verified-check { display: flex; align-items: flex-start; gap: 0.4rem; min-height: 1.5rem; }
  .verified-check .codicon { font-size: 1.1rem; margin-top: 0.1rem; }

  .requirements { display: grid; gap: 0.75rem; padding: 0.3rem var(--edge) 0; }
  .requirement { display: grid; grid-template-columns: 1.3rem minmax(0, 1fr); gap: 0.15rem 0.4rem; align-items: start; }
  .requirement > :not(.status) { grid-column: 2; }
  .requirement .status { grid-column: 1; grid-row: 1; margin-top: 0.05rem; }
  .requirement .statement { grid-row: 1; overflow-wrap: anywhere; }
  .figures { display: flex; flex-wrap: wrap; align-items: baseline; gap: 0 0.75rem; font-variant-numeric: tabular-nums; }
  .figures .measured { flex: 1 1 auto; }
  .figures .allowed-words { flex: 0 1 auto; text-align: right; }
  .figures .band { flex: 1 0 100%; }
  .requirement .detail { font-size: 0.92rem; overflow-wrap: anywhere; }
  .band { position: relative; height: 1.1rem; min-width: 0; }
  .band::before { content: ""; position: absolute; inset: calc(50% - 1px) 0 auto 0; height: 2px; border-radius: 1px; background: var(--vscode-scrollbarSlider-activeBackground, rgba(128, 128, 128, 0.5)); }
  .band .allowed { position: absolute; top: calc(50% - 3px); height: 6px; border-radius: 1px; background: var(--good); opacity: 0.6; min-width: 2px; border: 1px solid var(--vscode-contrastBorder, transparent); }
  .band .across { position: absolute; top: calc(50% - 0.5px); height: 1px; min-width: 1px; background: var(--fg); }
  .band .across::before, .band .across::after { content: ""; position: absolute; top: -3px; width: 1px; height: 7px; background: inherit; }
  .band .across::before { left: 0; }
  .band .across::after { right: 0; }
  .band .across.out { top: calc(50% - 1.5px); height: 3px; background: var(--bad); }
  .band .across.out::before, .band .across.out::after { top: -2px; }
  .band .target { position: absolute; top: calc(50% - 5px); width: 1px; height: 10px; background: var(--fg); opacity: 0.45; }
  .band .value { position: absolute; top: calc(50% - 0.45rem); width: 3px; height: 0.9rem; margin-left: -1.5px; border-radius: 1px; background: var(--fg); box-shadow: 0 0 0 1px var(--panel-ground); }
  .band .value.out { background: var(--bad); }
  /* Very narrow: what is allowed is told by the line alone. */
  @container panel (max-width: 16rem) { .figures .allowed-words { display: none; } }

  .parameters { display: grid; gap: 0.1rem; padding: 0.15rem 0.3rem 0 var(--edge); }
  .parameter { display: grid; grid-template-columns: minmax(9rem, 1fr) minmax(6rem, 1.2fr) auto var(--row); align-items: center; gap: 0 0.5rem; min-height: var(--row); }
  .parameter label { display: flex; gap: 0 0.3rem; min-width: 0; white-space: nowrap; align-items: baseline; }
  .parameter .holder { overflow: hidden; text-overflow: ellipsis; min-width: 0; flex: 0 1 auto; }
  .parameter .name { flex: 0 1 auto; min-width: 2rem; overflow: hidden; text-overflow: ellipsis; }
  .parameter.changed .name { color: var(--warn); }
  .parameter.changed .name::after { content: ""; display: inline-block; width: 0.4rem; height: 0.4rem; border-radius: 50%; background: var(--warn); margin-left: 0.3rem; vertical-align: 0.08em; }
  .parameter.changed .entry input { border-color: var(--warn); }
  .track { position: relative; display: flex; min-width: 0; }
  .track input { width: 100%; }
  .track .kept { position: absolute; left: var(--kept); bottom: 0.1rem; width: 1px; height: 0.35rem; background: var(--quiet); pointer-events: none; }
  .entry { display: inline-flex; align-items: center; gap: 0.3rem; }
  .entry input { width: 5.2rem; }
  .entry .unit { min-width: 1.6rem; color: var(--quiet); white-space: nowrap; }
  .reset { display: inline-flex; width: var(--row); }
  .hint { font-size: 0.92rem; }
  .tune { display: flex; }
  @container panel (max-width: 30rem) {
    .parameter { grid-template-columns: minmax(0, 1fr) auto var(--row); padding: 0.15rem 0; }
    .parameter .track { grid-column: 1 / -1; grid-row: 2; margin-right: 0.45rem; }
  }
  @container panel (max-width: 16rem) {
    .parameter .track { display: none; }
    .parameter:focus-within .track { display: flex; }
    .entry input { width: 4.2rem; }
  }
  .trying { flex: none; display: flex; flex-wrap: wrap; gap: 0.4rem; align-items: center; padding: 0.4rem var(--edge); border-top: 1px solid var(--line); background: var(--panel-ground); }
  .trying > span { flex: 1 1 7rem; display: inline-flex; align-items: center; gap: 0.4rem; white-space: nowrap; }
  .trying .dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: var(--warn); }

  .selection { display: grid; gap: 0.35rem; padding: 0.25rem var(--edge) 0; }
  .selection h3 { margin-top: 0.3rem; }
  .crumbs { display: flex; flex-wrap: wrap; gap: 0.1rem 0.35rem; align-items: center; font-size: 0.92rem; }
  .crumbs > * + *::before { content: "›"; margin-right: 0.35rem; color: var(--quiet); display: inline-block; }
  .what { display: grid; gap: 0.1rem; }
  .what code { justify-self: start; font-size: 1em; }
  .source { justify-self: start; }
  .source .codicon { font-size: 1.1rem; }
  .inside { display: flex; flex-wrap: wrap; gap: 0.2rem 0.7rem; }
  dl { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 0.1rem 0.9rem; }
  dt { color: var(--quiet); }
  dd { text-align: right; font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
`;

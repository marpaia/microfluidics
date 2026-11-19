// A list worked as VS Code's are: one stop for Tab, the arrows move through it, Enter acts on a
// row, and what can be done to a row is reached from it.

import type { ComponentChildren, JSX } from "preact";
import { useRef } from "preact/hooks";

/** The rows of a list, to the keyboard: the one that has it, or else the first. */
function rows(list: HTMLElement): HTMLElement[] {
  return Array.from(list.querySelectorAll<HTMLElement>(":scope > .row"));
}

export function List({ label, children }: { label: string; children: ComponentChildren }): JSX.Element {
  const list = useRef<HTMLDivElement>(null);
  const moved = (event: KeyboardEvent) => {
    const held = list.current ? rows(list.current) : [];
    const at = held.findIndex((row) => row === document.activeElement || row.contains(document.activeElement));
    const to = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: held.length - 1 }[event.key];
    if (to === undefined || at < 0) return;
    event.preventDefault();
    const next = held[Math.min(Math.max(to, 0), held.length - 1)];
    for (const row of held) row.tabIndex = row === next ? 0 : -1;
    next?.focus();
  };
  return (
    <div class="list" role="list" aria-label={label} ref={list} onKeyDown={moved}>
      {children}
    </div>
  );
}

/** One row. `onAct` is what a click or Enter on it does; `actions` are what else can be done to
 * it, shown while the pointer or the keyboard is on it. */
export function Row({ first, onAct, title, actions, tone, group, children }: { first?: boolean; onAct?: () => void; title?: string; actions?: ComponentChildren; tone?: string; group?: string; children: ComponentChildren }): JSX.Element {
  const keyed = (event: KeyboardEvent) => {
    if (event.target !== event.currentTarget || !onAct) return;
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onAct();
    }
  };
  return (
    <div class={`row ${onAct ? "acts" : ""} ${tone ? `tone-${tone}` : ""}`} role="listitem" tabIndex={first ? 0 : -1} title={title} data-group={group} onClick={onAct} onKeyDown={keyed}>
      {children}
      {actions && <span class="row-actions" onClick={(event) => event.stopPropagation()}>{actions}</span>}
    </div>
  );
}

// A section of the panel, as the sidebar's own views are: a heading that opens and closes it,
// a count, and what can be done to it, shown while the pointer or the keyboard is on it.

import type { ComponentChildren, JSX } from "preact";

import { toggleSection } from "../state";
import { useStore } from "./store";
import { Icon } from "./icon";

export function Pane({ name, title, count, tone, note, actions, children }: { name: string; title: string; count?: string | number; tone?: string; note?: string; actions?: ComponentChildren; children: ComponentChildren }): JSX.Element {
  const open = useStore().open[name] ?? true;
  return (
    <section class="pane" id={`pane-${name}`} data-open={open}>
      <header class="pane-head">
        <button class="pane-title" aria-expanded={open} aria-controls={`pane-body-${name}`} onClick={() => toggleSection(name)}>
          <Icon name={open ? "chevron-down" : "chevron-right"} />
          <span class="name">{title}</span>
          {note && <span class="note">{note}</span>}
        </button>
        {actions && <span class="pane-actions">{actions}</span>}
        {count !== undefined && count !== "" && <span class={`badge ${tone ?? ""}`}>{count}</span>}
      </header>
      {open && <div class="pane-body" id={`pane-body-${name}`}>{children}</div>}
    </section>
  );
}

/** Something that opens to show more, within a section. */
export function More({ summary, children, open }: { summary: ComponentChildren; children: ComponentChildren; open?: boolean }): JSX.Element {
  return (
    <details class="more" open={open}>
      <summary>
        <Icon name="chevron-right" />
        {summary}
      </summary>
      {children}
    </details>
  );
}

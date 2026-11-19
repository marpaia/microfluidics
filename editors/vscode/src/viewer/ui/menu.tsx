// A button that opens a menu beneath it. The menu closes on Escape, on a click outside it, and
// when the keyboard leaves it; it opens toward whichever side has room.

import type { ComponentChildren, JSX } from "preact";
import { useEffect, useRef, useState } from "preact/hooks";

import { Icon } from "./icon";

export function Menu({ icon, label, title, compact, up, children }: { icon?: string; label?: string; title: string; compact?: boolean; up?: boolean; children: ComponentChildren }): JSX.Element {
  const [open, setOpen] = useState(false);
  const [end, setEnd] = useState(false);
  const held = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return undefined;
    const box = held.current?.getBoundingClientRect();
    const room = held.current?.closest("#stage, #panel")?.getBoundingClientRect();
    // It opens from the button's left side unless that would carry it past the edge.
    if (box && room) setEnd(box.left - room.left > room.right - box.right);
    const away = (event: Event) => {
      if (!held.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", away, true);
    document.addEventListener("focusin", away, true);
    return () => {
      document.removeEventListener("pointerdown", away, true);
      document.removeEventListener("focusin", away, true);
    };
  }, [open]);
  const keyed = (event: KeyboardEvent) => {
    if (event.key === "Escape" && open) {
      event.stopPropagation();
      setOpen(false);
      held.current?.querySelector<HTMLElement>(":scope > button")?.focus();
      return;
    }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    if ((event.target as HTMLElement).matches("input[type=range]")) return;
    const items = Array.from(held.current?.querySelectorAll<HTMLElement>(".menu-sheet button, .menu-sheet input") ?? []);
    if (!items.length) return;
    event.preventDefault();
    if (!open) setOpen(true);
    const at = items.indexOf(document.activeElement as HTMLElement);
    items[(at + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length]?.focus();
  };
  return (
    <div class={`menu ${open ? "open" : ""}`} ref={held} onKeyDown={keyed}>
      <button class={compact || !label ? "icon-button" : "menu-button"} title={title} aria-label={compact || !label ? title : undefined} aria-haspopup="true" aria-expanded={open} onClick={() => setOpen(!open)}>
        {icon && <Icon name={icon} />}
        {!compact && label && <span>{label}</span>}
        {!compact && label && <Icon name="chevron-down" />}
      </button>
      {open && <div class={`menu-sheet ${end ? "end" : ""} ${up ? "up" : ""}`} role="menu">{children}</div>}
    </div>
  );
}

/** One of several things of which one is chosen. */
export function Choice({ chosen, disabled, title, onChoose, children }: { chosen: boolean; disabled?: boolean; title?: string; onChoose: () => void; children: ComponentChildren }): JSX.Element {
  return (
    <button class="menu-item" role="menuitemradio" aria-checked={chosen} disabled={disabled} title={title} onClick={onChoose}>
      <Icon name={chosen ? "check" : "blank"} />
      {children}
    </button>
  );
}

/** Something that is on or off. */
export function Toggle({ on, onToggle, children }: { on: boolean; onToggle: () => void; children: ComponentChildren }): JSX.Element {
  return (
    <button class="menu-item" role="menuitemcheckbox" aria-checked={on} onClick={onToggle}>
      <Icon name={on ? "check" : "blank"} />
      {children}
    </button>
  );
}

export function MenuHeading({ children }: { children: ComponentChildren }): JSX.Element {
  return <div class="menu-heading" role="presentation">{children}</div>;
}

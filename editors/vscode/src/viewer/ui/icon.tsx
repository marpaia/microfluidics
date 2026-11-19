// VS Code's own icons, by name, and the buttons that are only an icon.

import type { JSX } from "preact";

export function Icon({ name, label, spin }: { name: string; label?: string; spin?: boolean }): JSX.Element {
  return <i class={`codicon codicon-${name}${spin ? " codicon-modifier-spin" : ""}`} role={label ? "img" : undefined} aria-label={label} aria-hidden={label ? undefined : "true"} />;
}

const STATUS: Record<string, { icon: string; tone: string; label: string }> = {
  satisfied: { icon: "pass", tone: "good", label: "passes" },
  waived: { icon: "pass", tone: "warn", label: "waived" },
  violated: { icon: "error", tone: "bad", label: "fails" },
  not_run: { icon: "circle-slash", tone: "warn", label: "could not be verified" },
};

/** The mark of how a check or a requirement came out. */
export function Status({ status }: { status: string }): JSX.Element {
  const held = STATUS[status] ?? STATUS.not_run!;
  return <span class={`status ${held.tone}`}><Icon name={held.icon} label={held.label} /></span>;
}

/** A button that is an icon, with what it does as its name and its tooltip. */
export function IconButton({ icon, title, onClick, pressed, disabled, tabIndex }: { icon: string; title: string; onClick: (event: MouseEvent) => void; pressed?: boolean; disabled?: boolean; tabIndex?: number }): JSX.Element {
  return (
    <button class="icon-button" title={title} aria-label={title} aria-pressed={pressed} disabled={disabled} tabIndex={tabIndex} onClick={onClick}>
      <Icon name={icon} />
    </button>
  );
}

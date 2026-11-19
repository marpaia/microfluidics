// The panel: how the design stands, then what is selected, what is wrong, what it must do and
// what may be tried; and, where the panel lies over the chip, the way back to the chip.

import type { JSX } from "preact";

import { openSheet } from "../state";
import { Icon, IconButton } from "../ui/icon";
import { useStore } from "../ui/store";
import { headline } from "../verification";
import { Parameters, Trying } from "./parameters";
import { Problems } from "./problems";
import { Requirements } from "./requirements";
import { Selected } from "./selected";
import { Summary } from "./summary";

export function Panel(): JSX.Element {
  const { arrangement } = useStore();
  return (
    <>
      {arrangement === "sheet" && <span class="sheet-close"><IconButton icon="chevron-down" title="Back to the chip (Escape)" onClick={() => openSheet(false)} /></span>}
      <Summary />
      <div class="panes">
        <Selected />
        <Problems />
        <Requirements />
        <Parameters />
      </div>
      <Trying />
    </>
  );
}

/** Where there is room for the chip alone: the design's standing on one line, which opens the panel. */
export function Foot(): JSX.Element | null {
  const { scene, failure, trial, sheet } = useStore();
  if (!scene || sheet) return null;
  const count = scene.problems;
  return (
    <button class="foot-bar" aria-haspopup="dialog" onClick={() => openSheet(true)} title="Show problems, requirements and parameters">
      <span class={failure || count ? "bad" : "good"}><Icon name={failure || count ? "error" : "pass"} /></span>
      <span class="said">{failure ? "Does not build" : headline(count)}</span>
      <span class="quiet name">{scene.design}</span>
      {trial.size > 0 && <span class="warn">{trial.size} unsaved</span>}
      <Icon name="chevron-up" />
    </button>
  );
}

/** Where there is not room for a single control: a mark of how the design stands. */
export function Mark(): JSX.Element | null {
  const { scene, failure } = useStore();
  if (!scene) return null;
  const bad = failure !== undefined || scene.problems > 0;
  const said = failure ? "Does not build" : headline(scene.problems);
  return <span class={`chip-mark ${bad ? "bad" : "good"}`} title={`${scene.design}: ${said}`}><Icon name={bad ? "error" : "pass"} label={said} /></span>;
}

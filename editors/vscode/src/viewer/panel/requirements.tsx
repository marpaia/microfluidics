// What the design must do: each requirement with what was measured as drawn, what is allowed,
// and how far fabrication carries it.

import type { JSX } from "preact";

import type { SceneRequirement } from "../../scene";
import { number, quantity } from "../format";
import { Status } from "../ui/icon";
import { Pane } from "../ui/pane";
import { useStore } from "../ui/store";
import { band } from "./band";

/** What a bound allows, in words. */
function allowed(requirement: SceneRequirement): string {
  const { bound, unit } = requirement;
  const [low, high] = [bound.low, bound.high];
  if (low !== null && high !== null) return `${number(low)} to ${quantity(high, unit)}`;
  if (high !== null) return `at most ${quantity(high, unit)}`;
  if (low !== null) return `at least ${quantity(low, unit)}`;
  return "";
}

/** What was measured, in words: one value, or the least and greatest of several. */
function measured(requirement: SceneRequirement): string {
  const values = Object.values(requirement.values);
  if (!values.length) return "Not measured";
  const [low, high] = [Math.min(...values), Math.max(...values)];
  if (values.length === 1 || low === high) return quantity(low, requirement.unit);
  return `${number(low)} to ${quantity(high, requirement.unit)}`;
}

const share = (place: number) => `${(place * 100).toFixed(2)}%`;

/** The requirement on a line: allowed, measured, and across fabrication. */
function Band({ requirement }: { requirement: SceneRequirement }): JSX.Element | null {
  const held = band(requirement);
  if (!held) return null;
  const across = requirement.across;
  const said = `${measured(requirement)} as drawn; ${allowed(requirement)} allowed${across ? `; ${number(across.low)} to ${quantity(across.high, requirement.unit)} as fabrication varies` : ""}`;
  return (
    <div class="band" role="img" aria-label={said} title={said}>
      <span class="allowed" style={{ left: share(held.allowed[0]), width: share(held.allowed[1] - held.allowed[0]) }} />
      {held.across && <span class="across" style={{ left: share(held.across.from), width: share(held.across.to - held.across.from) }} />}
      {held.across?.out.map(([from, to]) => <span class="across out" style={{ left: share(from), width: share(to - from) }} />)}
      {held.target !== undefined && <span class="target" style={{ left: share(held.target) }} />}
      {held.values.map((value) => <span class={`value ${value.out ? "out" : ""}`} style={{ left: share(value.at) }} />)}
    </div>
  );
}

function Requirement({ requirement }: { requirement: SceneRequirement }): JSX.Element {
  const across = requirement.across;
  const drifts = across !== null && !across.holds;
  const count = Object.keys(requirement.values).length;
  const out = Object.values(requirement.values).filter((value) => (requirement.bound.low !== null && value < requirement.bound.low) || (requirement.bound.high !== null && value > requirement.bound.high)).length;
  // As drawn it may hold and still not hold across fabrication: the mark is of both.
  const status = requirement.status === "satisfied" && drifts ? "violated" : requirement.status;
  return (
    <div class="requirement" data-status={status}>
      <Status status={status} />
      <span class="statement">{requirement.statement}</span>
      <span class="figures">
        <span class="measured"><strong>{measured(requirement)}</strong>{count > 1 && <span class="quiet"> over {count}</span>}</span>
        <span class="quiet allowed-words">{allowed(requirement)}</span>
        <Band requirement={requirement} />
      </span>
      {requirement.status === "not_run" && <span class="detail warn">{requirement.note ?? "It could not be measured."}</span>}
      {requirement.status === "violated" && <span class="detail bad">{count > 1 ? `${out} of ${count} are outside what is allowed, as drawn.` : "Outside what is allowed, as drawn."}</span>}
      {drifts && across && (
        <span class="detail bad">
          {requirement.status === "satisfied" ? "Holds as drawn. " : ""}Reaches {quantity(across.reached, requirement.unit)} as fabrication varies <span class="quiet">({across.corner})</span>.
        </span>
      )}
    </div>
  );
}

export function Requirements(): JSX.Element | null {
  const { scene } = useStore();
  if (!scene || !scene.requirements.length) return null;
  const holding = scene.requirements.filter((held) => (held.status === "satisfied" || held.status === "waived") && (held.across === null || held.across.holds)).length;
  const all = holding === scene.requirements.length;
  return (
    <Pane name="requirements" title="Requirements" count={all ? String(holding) : `${holding} of ${scene.requirements.length}`} tone={all ? "" : "bad"}>
      <div class="requirements">
        {scene.requirements.map((requirement) => <Requirement key={requirement.name} requirement={requirement} />)}
      </div>
    </Pane>
  );
}

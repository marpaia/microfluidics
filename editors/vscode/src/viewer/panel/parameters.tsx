// The parameters the design allows to be changed: each tried within its range, and what is
// tried kept or discarded on purpose.

import type { JSX } from "preact";

import type { SceneTunable } from "../../scene";
import { number, parts, unitName, unitOf } from "../format";
import { discard, keep, run, tryValue } from "../state";
import { IconButton } from "../ui/icon";
import { Pane } from "../ui/pane";
import { useStore } from "../ui/store";

function Parameter({ tunable, value, kept }: { tunable: SceneTunable; value: number; kept: number }): JSX.Element {
  const { holder, leaf } = parts(tunable.path);
  const unit = unitName(unitOf(tunable.written));
  const step = tunable.count ? 1 : (tunable.high - tunable.low) / 400;
  const changed = value !== kept;
  const tried = (text: string) => {
    const asked = Number(text);
    if (text.trim() === "" || !Number.isFinite(asked)) return;
    tryValue(tunable.path, Math.min(Math.max(asked, tunable.low), tunable.high));
  };
  const along = (held: number) => `${(((held - tunable.low) / (tunable.high - tunable.low || 1)) * 100).toFixed(2)}%`;
  const range = `${number(tunable.low)} to ${number(tunable.high)} ${unit}`.trim();
  return (
    <div class={`parameter ${changed ? "changed" : ""}`}>
      <label for={`try-${tunable.path}`} title={`${tunable.path}, ${range}`}>
        {holder && <span class="quiet holder">{holder}</span>}
        <span class="name">{leaf}</span>
      </label>
      <span class="track" style={{ "--at": along(value), "--kept": along(kept) }}>
        <input type="range" aria-label={`${tunable.path}, ${range}`} title={range} min={tunable.low} max={tunable.high} step={step} value={value} onInput={(event) => tryValue(tunable.path, Number(event.currentTarget.value))} />
        <span class="kept" title={`Saved: ${number(kept)} ${unit}`} />
      </span>
      <span class="entry">
        <input id={`try-${tunable.path}`} type="number" min={tunable.low} max={tunable.high} step={tunable.count ? 1 : "any"} value={Number(value.toFixed(3))} onChange={(event) => tried(event.currentTarget.value)} />
        <span class="unit">{unit}</span>
      </span>
      <span class="reset">{changed && <IconButton icon="discard" title={`Back to ${number(kept)} ${unit}`} onClick={() => tryValue(tunable.path, undefined)} />}</span>
    </div>
  );
}

export function Parameters(): JSX.Element | null {
  const { scene, trial, kept, running } = useStore();
  if (!scene || !scene.tunables.length) return null;
  const count = trial.size;
  const tuning = running === "close";
  return (
    <Pane
      name="parameters"
      title="Parameters"
      count={count ? `${count} unsaved` : String(scene.tunables.length)}
      tone={count ? "warn" : ""}
      actions={<IconButton icon="wand" title="Tune automatically: find values that meet the requirements, and save them" disabled={running !== undefined} onClick={() => run("close")} />}
    >
      <div class="parameters">
        {scene.tunables.map((tunable) => {
          const saved = kept.get(tunable.path) ?? tunable.value;
          return <Parameter key={tunable.path} tunable={tunable} value={trial.get(tunable.path) ?? saved} kept={saved} />;
        })}
      </div>
      <p class="quiet hint">Drag to try a value. Nothing is saved until you keep it.</p>
      <div class="tune">
        <button disabled={running !== undefined} onClick={() => run("close")}>{tuning ? "Tuning…" : "Tune automatically"}</button>
      </div>
    </Pane>
  );
}

/** What is being tried, and the two things to do with it. It stays at the panel's foot. */
export function Trying(): JSX.Element | null {
  const { trial, scene } = useStore();
  if (!scene || trial.size === 0) return null;
  return (
    <div class="trying" role="status">
      <span><span class="dot" /> {trial.size} unsaved {trial.size === 1 ? "value" : "values"}</span>
      <button class="primary" onClick={keep}>Keep</button>
      <button onClick={discard}>Discard</button>
    </div>
  );
}

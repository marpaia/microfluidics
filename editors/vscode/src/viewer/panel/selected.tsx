// What is selected on the chip: what it is, where it was made, how large it is, what it was
// built with, and what flows through it.

import type { JSX } from "preact";

import type { Scene, SceneFlow } from "../../scene";
import { length, number, parts, place, quantity, written } from "../format";
import { ancestors, within } from "../geometry";
import { host } from "../host";
import { type Index, boxOf, fitTo, flowsThrough, select } from "../state";
import { Icon, IconButton } from "../ui/icon";
import { List } from "../ui/list";
import { Pane } from "../ui/pane";
import { useStore } from "../ui/store";
import { problems } from "../verification";
import { ProblemRow } from "./problems";

/** What kind of thing a path is, in a few words. */
function kind(scene: Scene, index: Index, path: string): string {
  const instance = index.instances.get(path);
  if (instance) return `An instance of ${instance.region}`;
  const row = scene.translates.find((held) => held.path === path);
  if (row) return `A row of ${row.count}, each ${length(Math.hypot(...row.step))} from the last`;
  const shape = scene.shapes.find((held) => held.path === path);
  if (shape) return `A rectangle on the ${shape.layer} layer`;
  if (index.flows.has(path)) return "A channel";
  return "Drawn by the design";
}

function range(flows: SceneFlow[], of: (flow: SceneFlow) => number, unit: string): string {
  const values = flows.map((flow) => Math.abs(of(flow)));
  const [low, high] = [Math.min(...values), Math.max(...values)];
  return low === high ? quantity(low, unit) : `${number(low)} to ${quantity(high, unit)}`;
}

function Flow({ scene, flows }: { scene: Scene; flows: SceneFlow[] }): JSX.Element {
  const drop = (flow: SceneFlow) => (scene.pressures_mbar[flow.start] ?? 0) - (scene.pressures_mbar[flow.end] ?? 0);
  return (
    <>
      <h3>{flows.length === 1 ? "What flows through it" : `What flows through its ${flows.length} channels`}</h3>
      <dl>
        <dt>Flow rate</dt><dd>{range(flows, (flow) => flow.flow_ul_min, "ul_min")}</dd>
        <dt>Speed</dt><dd>{range(flows, (flow) => flow.velocity_mm_s, "mm_s")}</dd>
        <dt>Wall shear</dt><dd>{range(flows, (flow) => flow.shear_pa, "Pa")}</dd>
        <dt>Pressure drop</dt><dd>{range(flows, drop, "mbar")}</dd>
        <dt>Reynolds number</dt><dd>{range(flows, (flow) => flow.reynolds, "")}</dd>
      </dl>
    </>
  );
}

/** The selection, shown only while there is one. */
export function Selected(): JSX.Element | null {
  const { scene, index, selection } = useStore();
  if (!scene || !index || selection === undefined) return null;
  const rect = boxOf(selection);
  const instance = index.instances.get(selection);
  const origin = scene.origins[selection];
  const chain = ancestors(selection);
  const flows = flowsThrough(scene, selection);
  const drive = scene.drives.find((held) => held.instance === selection);
  const found = problems(scene).filter((problem) => problem.finding?.paths.some((path) => within(path, selection)));
  const inside = scene.instances.filter((held) => parts(held.path).holder === selection);
  return (
    <Pane
      name="selected"
      title="Selection"
      note={parts(selection).leaf}
      actions={
        <>
          {rect && <IconButton icon="target" title="Zoom to it (F)" onClick={() => fitTo(rect)} />}
          <IconButton icon="close" title="Clear the selection (Escape)" onClick={() => select(undefined)} />
        </>
      }
    >
      <div class="selection">
        {chain.length > 1 && (
          <nav class="crumbs" aria-label="What holds it">
            {chain.slice(0, -1).map((path) => <button class="link" onClick={() => select(path)}>{parts(path).leaf}</button>)}
            <span>{parts(selection).leaf}</span>
          </nav>
        )}
        <p class="what">
          <code>{selection}</code>
          <span class="quiet">{kind(scene, index, selection)}</span>
        </p>
        {origin && (
          <button class="link source" onClick={() => host.postMessage({ type: "open", path: selection })} title="Go to the line that made it">
            <Icon name="go-to-file" /> {place(origin.file, origin.line)}
          </button>
        )}
        <dl>
          {rect && <><dt>Size</dt><dd>{length(rect[2] - rect[0])} × {length(rect[3] - rect[1])}</dd></>}
          {rect && <><dt>Lower left corner</dt><dd>{number(rect[0] / 1000)}, {number(rect[1] / 1000)} mm</dd></>}
          {drive && <><dt>Applied here</dt><dd>{drive.pressure_mbar !== null ? quantity(drive.pressure_mbar, "mbar") : quantity(drive.flow_ul_min ?? 0, "ul_min")}</dd></>}
        </dl>
        {instance && instance.parameters.length > 0 && (
          <>
            <h3>Built with</h3>
            <dl>{instance.parameters.map(([name, value]) => <><dt>{name}</dt><dd>{written(value)}</dd></>)}</dl>
          </>
        )}
        {flows.length > 0 && <Flow scene={scene} flows={flows} />}
        {inside.length > 0 && (
          <>
            <h3>Inside it</h3>
            <p class="inside">{inside.map((held) => <button class="link" onClick={() => select(held.path)}>{parts(held.path).leaf}</button>)}</p>
          </>
        )}
      </div>
      {found.length > 0 && (
        <>
          <h3 class="in-pane">Problems here</h3>
          <List label="Problems here">{found.slice(0, 8).map((problem, at) => <ProblemRow problem={problem} index={index} first={at === 0} />)}</List>
        </>
      )}
    </Pane>
  );
}

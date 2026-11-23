// What stands in the way of the design passing, listed as an editor lists problems; and, for the
// record, every check that was run.

import type { JSX } from "preact";

import type { SceneFinding } from "../../scene";
import { host } from "../host";
import { type Index, boxOf, fitTo, select, unfold } from "../state";
import { Icon, IconButton, Status } from "../ui/icon";
import { List, Row } from "../ui/list";
import { More, Pane } from "../ui/pane";
import { useStore } from "../ui/store";
import { GROUPS, type Problem, keyOf, listed, named, problems, shortened, waived } from "../verification";
import { Said } from "./said";

/** How many problems of one check are listed before the rest are folded away. */
const MOST = 4;

const NAMES = Object.fromEntries(GROUPS.map((held) => [held.group, held.name]));

/** The first thing on the chip a finding is about. */
function about(finding: SceneFinding | undefined, index: Index): string | undefined {
  return finding?.paths.find((path) => index.where.has(path));
}

/** One problem as a row: what is wrong, which question it belongs to, and where it is. */
export function ProblemRow({ problem, index, first }: { problem: Problem; index: Index; first?: boolean }): JSX.Element {
  const { finding, check } = problem;
  const path = about(finding, index);
  const pin = finding ? index.pinned.indexOf(finding) : -1;
  const origin = finding?.origin;
  const act = path !== undefined ? () => select(path, true) : origin ? () => host.postMessage({ type: "open", file: origin.file, line: origin.line }) : undefined;
  const rect = path === undefined ? undefined : boxOf(path);
  return (
    <Row
      first={first}
      group={problem.group}
      onAct={act}
      tone={finding ? "bad" : "warn"}
      title={path !== undefined ? "Show it on the chip and in the source" : origin ? "Go to the line it is about" : undefined}
      actions={
        <>
          {origin && <IconButton icon="go-to-file" title="Go to source" tabIndex={-1} onClick={() => host.postMessage({ type: "open", file: origin.file, line: origin.line })} />}
          {rect && <IconButton icon="target" title="Zoom to it" tabIndex={-1} onClick={() => fitTo(rect)} />}
        </>
      }
    >
      <span class="mark">
        {pin >= 0 && pin < 40 ? <span class="pin">{pin + 1}</span> : <Icon name={finding ? "error" : "circle-slash"} label={finding ? "problem" : "could not be verified"} />}
      </span>
      <span class="text">
        <span class="message"><Said text={problem.message} /></span>
        <span class="where">
          <span>{NAMES[problem.group]}</span>
          <span>{check.id.startsWith("requirement/") ? "Requirement" : named(check)}</span>
          {path !== undefined ? <code title={path}>{shortened(path, 28)}</code> : <span>whole design</span>}
        </span>
      </span>
    </Row>
  );
}

export function Problems(): JSX.Element | null {
  const { scene, index, unfolded } = useStore();
  if (!scene || !index) return null;
  const all = problems(scene);
  const { shown, folded } = listed(all, MOST, unfolded);
  const accepted = waived(scene);
  const shownChecks = scene.checks.filter((check) => check.group !== "integrity" || check.status !== "satisfied");
  return (
    <Pane name="problems" title="Problems" count={all.length || ""} tone={all.length ? "bad" : ""}>
      {all.length === 0 ? (
        <p class="empty good"><Icon name="pass" /> No problems. Every check passes.</p>
      ) : (
        <List label="Problems">
          {shown.map((problem, at) => (
            <ProblemRow key={`${keyOf(problem.check)}-${at}`} problem={problem} index={index} first={at === 0} />
          ))}
        </List>
      )}
      {folded.map(({ check, more }) => (
        <button class="link fold" onClick={() => unfold(keyOf(check))}>Show {more} more of {check.id.startsWith("requirement/") ? "this requirement" : named(check).toLowerCase()}</button>
      ))}
      {accepted.length > 0 && (
        <More summary={<span>{accepted.length} accepted by waiver</span>}>
          <ul class="plain-list">
            {accepted.map((finding) => (
              <li>
                <Said text={finding.message} />
                <span class="quiet"> Waived: {finding.waived}</span>
              </li>
            ))}
          </ul>
        </More>
      )}
      <More summary={<span>What was verified <span class="quiet">{shownChecks.length} checks</span></span>}>
        <div class="verified">
          {GROUPS.map(({ group, name, question }) => {
            const held = shownChecks.filter((check) => check.group === group);
            if (!held.length) return null;
            return (
              <div class="verified-group">
                <h3 title={question}>{name} <span class="quiet">{question}</span></h3>
                <ul class="plain-list">
                  {held.map((check) => (
                    <li class="verified-check" title={check.note ?? undefined}>
                      <Status status={check.status} />
                      <span>{check.id.startsWith("requirement/") ? <Said text={check.title} /> : named(check)}</span>
                    </li>
                  ))}
                </ul>
              </div>
            );
          })}
        </div>
      </More>
    </Pane>
  );
}

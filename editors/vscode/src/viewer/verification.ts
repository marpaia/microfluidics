// What a scene's checks come to for a reader: the problems that stand in the way, in the order
// they are read, and how each question stands.

import type { Group, Scene, SceneCheck, SceneFinding } from "../scene";

/** The questions, in the order they are asked. */
export const GROUPS: { group: Group; name: string; question: string }[] = [
  { group: "fabrication", name: "Fabrication", question: "Can it be made?" },
  { group: "function", name: "Function", question: "Does it do what is asked?" },
  { group: "variation", name: "Variation", question: "Does it still, as fabrication varies?" },
  { group: "integrity", name: "Integrity", question: "Is the tool's own work sound?" },
];

/** One thing that stands in the way: a finding, or a check that did not run. */
export interface Problem {
  check: SceneCheck;
  group: Group;
  message: string;
  /** The finding, when the problem is one. A check that did not run has none. */
  finding?: SceneFinding;
}

/** Every problem of a scene: by question, then by check, as the checks ran. */
export function problems(scene: Scene): Problem[] {
  const found: Problem[] = [];
  for (const { group } of GROUPS) {
    for (const check of scene.checks) {
      if (check.group !== group) continue;
      if (check.status === "not_run") found.push({ check, group, message: check.note ?? "It could not be verified." });
      for (const finding of scene.findings) if (finding.check === check.id && !finding.waived) found.push({ check, group, message: finding.message, finding });
    }
  }
  return found;
}

/** The findings a person accepted, with the reason each gave. */
export function waived(scene: Scene): SceneFinding[] {
  return scene.findings.filter((finding) => finding.waived);
}

export interface Standing {
  group: Group;
  name: string;
  question: string;
  checks: number;
  problems: number;
  /** Whether every check of it ran. */
  ran: boolean;
}

/** How each question stands. The tool's own checks are left out while they find nothing. */
export function standings(scene: Scene): Standing[] {
  const all = problems(scene);
  return GROUPS.map(({ group, name, question }) => {
    const checks = scene.checks.filter((check) => check.group === group);
    return { group, name, question, checks: checks.length, problems: all.filter((held) => held.group === group).length, ran: checks.every((check) => check.status !== "not_run") };
  }).filter((held) => held.checks > 0 && (held.group !== "integrity" || held.problems > 0));
}

/** The scene's standing in a few words. */
export function headline(count: number): string {
  return count === 0 ? "No problems" : count === 1 ? "1 problem" : `${count} problems`;
}

/** Problems as they are listed: those of one check beyond `most` are folded into a count. */
export function listed(all: Problem[], most: number, opened: Set<string>): { shown: Problem[]; folded: { check: SceneCheck; more: number }[] } {
  const [shown, folded]: [Problem[], { check: SceneCheck; more: number }[]] = [[], []];
  const seen = new Map<string, number>();
  for (const problem of all) {
    const count = (seen.get(problem.check.id) ?? 0) + 1;
    seen.set(problem.check.id, count);
    if (count <= most || opened.has(problem.check.id)) shown.push(problem);
  }
  for (const [id, count] of seen) {
    if (count > most && !opened.has(id)) folded.push({ check: all.find((held) => held.check.id === id)!.check, more: count - most });
  }
  return { shown, folded };
}

/** A path with its middle left out when it is longer than `most`, so that both ends are read. */
export function shortened(path: string, most: number): string {
  if (path.length <= most || most < 5) return path;
  const tail = Math.ceil((most - 1) * 0.6);
  return `${path.slice(0, most - 1 - tail)}…${path.slice(path.length - tail)}`;
}

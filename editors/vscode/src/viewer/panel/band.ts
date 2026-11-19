// A requirement on a line: what is allowed, what was measured as drawn, and how far fabrication
// carries it. Every place is a share of the line's length, from 0 to 1.

import type { SceneRequirement } from "../../scene";

export interface Band {
  /** The allowed stretch. A side with no limit runs to the line's end. */
  allowed: [number, number];
  target?: number;
  /** Each value measured as drawn, and whether it is outside what is allowed. */
  values: { at: number; out: boolean }[];
  /** The stretch its values cover across the corners of the process, and the parts of it that
   * lie outside what is allowed. */
  across?: { from: number; to: number; out: [number, number][] };
}

export function band(requirement: SceneRequirement): Band | undefined {
  const values = Object.values(requirement.values);
  const { low, high, target } = requirement.bound;
  const across = requirement.across;
  const points = [...values, low, high, target, across?.low, across?.high].filter((value): value is number => value !== null && value !== undefined && Number.isFinite(value));
  if (!values.length || !points.length) return undefined;
  const [least, most] = [Math.min(...points), Math.max(...points)];
  const span = most - least || Math.abs(most) * 0.1 || 1;
  const [from, to] = [least - span * 0.12, most + span * 0.12];
  const place = (value: number) => Math.min(Math.max((value - from) / (to - from), 0), 1);
  const outside = (value: number) => (low !== null && value < low - 1e-12) || (high !== null && value > high + 1e-12);
  const out: [number, number][] = [];
  if (across) {
    if (low !== null && across.low < low - 1e-12) out.push([place(across.low), place(Math.min(low, across.high))]);
    if (high !== null && across.high > high + 1e-12) out.push([place(Math.max(high, across.low)), place(across.high)]);
  }
  return {
    allowed: [low === null ? 0 : place(low), high === null ? 1 : place(high)],
    target: target === null ? undefined : place(target),
    values: values.slice(0, 200).map((value) => ({ at: place(value), out: outside(value) })),
    across: across ? { from: place(across.low), to: place(across.high), out } : undefined,
  };
}

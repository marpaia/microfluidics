// The scene a session sends: what a design draws, where each thing was made, what flows, and
// what every check found. Lengths are micrometres.

export interface Origin {
  file: string;
  line: number;
}

export interface Shape {
  path: string;
  layer: string;
  /** `[x0, y0, x1, y1]`. */
  rect: [number, number, number, number];
}

export interface SceneTranslates {
  path: string;
  outline: string;
  origin: [number, number];
  step: [number, number];
  count: number;
  quarter_turns: number;
}

export interface SceneLayer {
  name: string;
  height_um: number;
  gds: number;
  colour: string;
}

export interface ScenePort {
  path: string;
  at: [number, number];
  width_um: number;
  layer: string;
  facing: [number, number];
}

export interface SceneFlow {
  element: string;
  start: string;
  end: string;
  flow_ul_min: number;
  velocity_mm_s: number;
  shear_pa: number;
  reynolds: number;
}

export interface SceneTunable {
  path: string;
  value: number;
  low: number;
  high: number;
  count: boolean;
  written: string;
}

/** What a requirement allows, in the unit of its values. A side with no limit is `null`. */
export interface SceneBound {
  kind: "equal_within" | "target" | "at_most" | "at_least";
  low: number | null;
  high: number | null;
  target: number | null;
}

/** The question a check answers. */
export type Group = "fabrication" | "function" | "variation" | "integrity";

/** A requirement across the corners of a process, in the requirement's unit. */
export interface SceneAcross {
  low: number;
  low_corner: string;
  high: number;
  high_corner: string;
  /** The value furthest from holding, and the corner it is at. */
  reached: number;
  corner: string;
  holds: boolean;
}

export interface SceneRequirement {
  name: string;
  statement: string;
  status: string;
  values: Record<string, number>;
  unit: string;
  bound: SceneBound;
  /** How it fares across the corners of the process, when that was found. */
  across: SceneAcross | null;
  note: string | null;
}

/** One placed instance of a region. */
export interface SceneInstance {
  path: string;
  region: string;
  rect: [number, number, number, number] | null;
  /** Each parameter's name and its value as written. */
  parameters: [string, string][];
  origin: Origin | null;
}

/** What is applied where tubing reaches the chip. */
export interface SceneDrive {
  instance: string;
  vertex: string;
  at: [number, number];
  pressure_mbar: number | null;
  flow_ul_min: number | null;
}

export interface SceneFinding {
  check: string;
  group: Group;
  rule: string;
  message: string;
  waived: string | null;
  paths: string[];
  origin: Origin | null;
}

export interface SceneCheck {
  id: string;
  title: string;
  group: Group;
  status: string;
  note: string | null;
  metrics: Record<string, number>;
}

export interface Scene {
  version: number;
  design: string;
  passes: boolean;
  /** How many things stand in the way of it passing. */
  problems: number;
  size_um: [number, number];
  thickness_um: number;
  /** The width, depth and thickness of the glass the slab is bonded to, at the glass's middle. */
  glass_um: [number, number, number];
  /** The diameter of the hole punched at each access point, when the process states it. */
  punch_um: number | null;
  layers: SceneLayer[];
  outlines: Record<string, Shape[]>;
  shapes: Shape[];
  translates: SceneTranslates[];
  ports: ScenePort[];
  access: [number, number][];
  flows: SceneFlow[];
  pressures_mbar: Record<string, number>;
  tunables: SceneTunable[];
  requirements: SceneRequirement[];
  instances: SceneInstance[];
  drives: SceneDrive[];
  checks: SceneCheck[];
  findings: SceneFinding[];
  origins: Record<string, Origin>;
}

/** One finding as an editor shows it. */
export interface Diagnostic {
  line: number;
  severity: "error" | "warning" | "information";
  message: string;
  check: string;
  group: Group;
  rule: string;
  paths: string[];
}

/** What a session says. */
export type FromSession =
  | { type: "status"; state: "building" | "ready" | "failed"; message?: string; file?: string; line?: number; passes?: boolean; problems?: number; build_ms?: number }
  | { type: "scene"; scene: Scene; trial: boolean }
  | { type: "diagnostics"; files: Record<string, Diagnostic[]> }
  | { type: "result"; command: string; ok: boolean; text: string };

/** What the viewer asks of the extension. */
export type FromViewer =
  | { type: "ready" }
  // Show the line that made something, leaving the keyboard where it is.
  | { type: "reveal"; path: string }
  // Open a place in the source and go to it.
  | { type: "open"; path?: string; file?: string; line?: number }
  | { type: "keep" }
  // Reload the editor's window, and show the session's log.
  | { type: "reload" }
  | { type: "log" }
  | { type: "set_overrides"; values: Record<string, string> }
  | { type: "run"; command: string };

/** What the extension tells the viewer. */
export type ToViewer = FromSession | { type: "highlight"; paths: string[] };

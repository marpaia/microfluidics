// Where the eye is: a point looked at, how far from it, how far tilted from straight above and
// how far turned about the upright. Lengths are millimetres; the chip lies in the plane of x and
// y with z upward. Nothing here draws.

export type Vector = [number, number, number];

export interface Eye {
  target: Vector;
  distance: number;
  /** From straight above, 0, through level with the chip, a quarter turn, to straight below. */
  tilt: number;
  /** About the upright; at 0 the chip's lower edge is nearest the eye. */
  turn: number;
}

/** The angle the view takes in from top to bottom, in degrees: narrow, so that straight above
 * the chip is seen nearly as a drawing of it. */
export const FIELD = 28;

const HALF = Math.tan((FIELD * Math.PI) / 360);

/** The directions of an eye: from the point looked at toward the eye, and to the right and
 * upward on the screen. */
export function axes(eye: Eye): { back: Vector; right: Vector; up: Vector } {
  const [st, ct, sp, cp] = [Math.sin(eye.turn), Math.cos(eye.turn), Math.sin(eye.tilt), Math.cos(eye.tilt)];
  const back: Vector = [sp * st, -sp * ct, cp];
  const up: Vector = [-cp * st, cp * ct, sp];
  const right: Vector = [up[1] * back[2] - up[2] * back[1], up[2] * back[0] - up[0] * back[2], up[0] * back[1] - up[1] * back[0]];
  return { back, right, up };
}

export function position(eye: Eye): Vector {
  const { back } = axes(eye);
  return [eye.target[0] + back[0] * eye.distance, eye.target[1] + back[1] * eye.distance, eye.target[2] + back[2] * eye.distance];
}

const dot = (a: Vector, b: Vector) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

/** How many pixels a millimetre is, at the distance of the point looked at, or at `far`. */
export function pixelsPerMm(eye: Eye, height: number, far = eye.distance): number {
  return height / (2 * far * HALF);
}

/** Where a point is on a screen of `width` by `height` pixels, and how far before the eye;
 * `undefined` for a point behind the eye. */
export function projected(eye: Eye, point: Vector, width: number, height: number): [number, number] | undefined {
  const { back, right, up } = axes(eye);
  const from: Vector = [point[0] - eye.target[0], point[1] - eye.target[1], point[2] - eye.target[2]];
  const before = eye.distance - dot(from, back);
  if (before <= 1e-6) return undefined;
  const scale = height / (2 * before * HALF);
  return [width / 2 + dot(from, right) * scale, height / 2 - dot(from, up) * scale];
}

/** The line of sight through a pixel: where it starts, and its direction. */
export function sight(eye: Eye, px: number, py: number, width: number, height: number): { from: Vector; along: Vector } {
  const { back, right, up } = axes(eye);
  const [x, y] = [((px - width / 2) / (height / 2)) * HALF, ((height / 2 - py) / (height / 2)) * HALF];
  const along: Vector = [right[0] * x + up[0] * y - back[0], right[1] * x + up[1] * y - back[1], right[2] * x + up[2] * y - back[2]];
  const size = Math.hypot(...along);
  return { from: position(eye), along: [along[0] / size, along[1] / size, along[2] / size] };
}

/** Where the line of sight through a pixel meets the level `z`; `undefined` when it never does. */
export function onLevel(eye: Eye, px: number, py: number, width: number, height: number, z = 0): Vector | undefined {
  const { from, along } = sight(eye, px, py, width, height);
  if (Math.abs(along[2]) < 1e-9) return undefined;
  const far = (z - from[2]) / along[2];
  return far <= 0 ? undefined : [from[0] + along[0] * far, from[1] + along[1] * far, z];
}

/** The eye, at the same angles, from which a box is seen whole: within a screen of `width` by
 * `height` pixels less `margin` at its sides, `above` and `below`. */
export function framed(eye: Eye, min: Vector, max: Vector, width: number, height: number, margin = 24, above = margin, below = margin): Eye {
  const centre: Vector = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2];
  const { back, right, up } = axes(eye);
  const [across, upright] = [Math.max(width - 2 * margin, 1) / width, Math.max(height - above - below, 1) / height];
  let distance = 1e-3;
  for (const x of [min[0], max[0]]) {
    for (const y of [min[1], max[1]]) {
      for (const z of [min[2], max[2]]) {
        const from: Vector = [x - centre[0], y - centre[1], z - centre[2]];
        const near = dot(from, back);
        distance = Math.max(distance, Math.abs(dot(from, right)) / (HALF * (width / height) * across) + near, Math.abs(dot(from, up)) / (HALF * upright) + near);
      }
    }
  }
  // The box sits midway between what lies over the screen's upper edge and its lower.
  const lift = ((below - above) / 2) * ((2 * distance * HALF) / height);
  return { ...eye, distance, target: [centre[0] - up[0] * lift, centre[1] - up[1] * lift, centre[2] - up[2] * lift] };
}

/** The eye after a drag that turns it about the point looked at: across the screen about the
 * upright, and up the screen toward straight above. */
export function orbited(eye: Eye, dx: number, dy: number): Eye {
  return { ...eye, turn: eye.turn - dx * 0.006, tilt: Math.min(Math.max(eye.tilt - dy * 0.006, 0), Math.PI) };
}

/** The eye after a drag that carries the chip with it across the screen. */
export function panned(eye: Eye, dx: number, dy: number, height: number): Eye {
  const { right, up } = axes(eye);
  const mm = 1 / pixelsPerMm(eye, height);
  const target: Vector = [0, 1, 2].map((at) => eye.target[at]! - right[at]! * dx * mm + up[at]! * dy * mm) as Vector;
  return { ...eye, target };
}

/** The eye brought `factor` times nearer, with the point `about` staying where it is on the
 * screen. */
export function neared(eye: Eye, factor: number, about: Vector | undefined, least = 0.02, most = 5000): Eye {
  const distance = Math.min(Math.max(eye.distance / factor, least), most);
  const kept = distance / eye.distance;
  const hold = about ?? eye.target;
  return { ...eye, distance, target: [0, 1, 2].map((at) => hold[at]! + (eye.target[at]! - hold[at]!) * kept) as Vector };
}

export const LOOKS = {
  top: { tilt: 0, turn: 0 },
  tilted: { tilt: 1.02, turn: -0.5 },
  under: { tilt: Math.PI, turn: Math.PI },
} as const;

export type Look = keyof typeof LOOKS;

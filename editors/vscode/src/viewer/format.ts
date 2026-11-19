// How the viewer writes numbers, units and names for a person to read.

const UNITS: Record<string, string> = { ul_min: "µL/min", mm_s: "mm/s", mbar: "mbar", Pa: "Pa", "%": "%", s: "s", um: "µm", mPa_s: "mPa·s" };

/** A unit as a person reads it: `ul_min` is `µL/min`. */
export function unitName(unit: string): string {
  return UNITS[unit] ?? unit;
}

const RAISED = "⁰¹²³⁴⁵⁶⁷⁸⁹";

/** A number to `digits` significant figures, as a power of ten only when it is very small:
 * `6.6×10⁻⁵`. */
export function number(value: number, digits = 4): string {
  if (value === 0 || !Number.isFinite(value)) return String(value);
  if (Math.abs(value) >= 1e-4) return String(Number(value.toPrecision(digits)));
  const [mantissa, power] = value.toExponential(1).split("e-");
  return `${mantissa}×10⁻${[...(power ?? "")].map((digit) => RAISED[Number(digit)]).join("")}`;
}

/** A value with its unit: `0.152 µL/min`, `89.2 %`. */
export function quantity(value: number, unit: string, digits = 4): string {
  const name = unitName(unit);
  return name ? `${number(value, digits)} ${name}` : number(value, digits);
}

/** A length given in micrometres, in the unit that reads best. */
export function length(um: number): string {
  const size = Math.abs(um);
  if (size >= 1000) return `${number(um / 1000)} mm`;
  if (size >= 1 || size === 0) return `${number(um)} µm`;
  return `${number(um * 1000)} nm`;
}

/** A value as a design writes it, set apart from its unit: `27.377um` is `27.377 µm`. */
export function written(text: string): string {
  const [, value, unit] = /^([-+0-9.eE]+)(.*)$/.exec(text) ?? [];
  return value === undefined ? text : unit ? `${value} ${unitName(unit)}` : value;
}

/** The unit a written value carries: `um` of `27.377um`. */
export function unitOf(text: string): string {
  return text.replace(/^[-+0-9.eE]+/, "");
}

/** The last part of a path, and what holds it: `sample_resistor.width` is `width` of `sample_resistor`. */
export function parts(path: string): { holder: string; leaf: string } {
  const at = path.lastIndexOf(".");
  return at < 0 ? { holder: "", leaf: path } : { holder: path.slice(0, at), leaf: path.slice(at + 1) };
}

/** A message cut where it quotes code: the pieces between backticks are code. */
export function spans(text: string): { text: string; code: boolean }[] {
  return text.split("`").map((piece, index) => ({ text: piece, code: index % 2 === 1 })).filter((piece) => piece.text !== "");
}

/** A message with the units it writes after numbers as a person reads them: `0.15ul_min` is
 * `0.15 µL/min`. */
export function units(text: string): string {
  return text.replace(/(\d)(ul_min|mm_s|mPa_s|mbar|um|Pa)\b/g, (_, digit: string, unit: string) => `${digit} ${unitName(unit)}`);
}

/** A file and line as a link reads: the file's name without its folders. */
export function place(file: string, line: number): string {
  return `${file.slice(file.lastIndexOf("/") + 1)}:${line}`;
}

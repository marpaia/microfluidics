// Recognising a design in a Python file.

const DESIGN = /^class\s+\w+\s*\([^)]*\bDesign\b[^)]*\)\s*:/m;

/** The start of the class that is a design, when `source` holds one. */
export function designAt(source: string): number | undefined {
  return DESIGN.exec(source)?.index;
}

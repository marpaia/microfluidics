// Text that quotes code.

import type { JSX } from "preact";

import { spans, units } from "../format";

/** A message with what it quotes between backticks set as code. */
export function Said({ text }: { text: string }): JSX.Element {
  return <>{spans(text).map((piece) => (piece.code ? <code>{piece.text}</code> : units(piece.text)))}</>;
}

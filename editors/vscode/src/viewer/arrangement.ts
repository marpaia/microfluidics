// How the stage and the panel share the preview, chosen from the preview's own width and height
// in rem, so that the editor's zoom and font size count as its pixels do.

export type Arrangement = "side" | "stacked" | "sheet" | "chip";

/** Where the reader has asked the panel to be, when they have. */
export type Pinned = "right" | "below" | "hidden";

/** The least a preview must be, in rem, for each thing it can hold. */
export const LEAST = { chip: [16, 11], side: [52, 16], stacked: 36, sheet: 22, panel: 18, stage: 14 } as const;

/** The arrangement for a preview `width` by `height` pixels, with `rem` pixels to the rem. */
export function arrangement(width: number, height: number, rem: number, pinned?: Pinned): Arrangement {
  const [wide, high] = [width / rem, height / rem];
  if (wide < LEAST.chip[0] || high < LEAST.chip[1]) return "chip";
  const room = { side: wide >= LEAST.panel + LEAST.stage + 2 && high >= LEAST.side[1], stacked: high >= LEAST.stacked - 8 && wide >= LEAST.sheet };
  if (pinned === "right" && room.side) return "side";
  if (pinned === "below" && room.stacked) return "stacked";
  if (wide < LEAST.sheet) return "sheet";
  if (wide >= LEAST.side[0]) return high >= LEAST.side[1] ? "side" : "sheet";
  return high >= LEAST.stacked ? "stacked" : "sheet";
}

/** The panel's width beside the stage, or its height beneath it, in pixels: what is `held` when
 * it fits, between the least a panel can be and what leaves the stage its own least. */
export function panelSize(how: "side" | "stacked", width: number, height: number, rem: number, held?: number): number {
  const along = how === "side" ? width : height;
  const least = (how === "side" ? LEAST.panel : 12) * rem;
  const most = Math.max(least, how === "side" ? Math.min(along / 2, along - LEAST.stage * rem) : along - LEAST.stage * rem);
  const wanted = held ?? (how === "side" ? Math.min(Math.max(along * 0.3, 21 * rem), 32 * rem) : along * 0.42);
  return Math.round(Math.min(Math.max(wanted, least), most));
}

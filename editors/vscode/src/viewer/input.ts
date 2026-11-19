// The pointer and the keys on the chip, turned into what they mean: moving the view or the eye,
// pointing at something, selecting it.

import { at, holding, pick, toDesign } from "./geometry";
import { fitAll, fitSelection, get, hover, orbitBy, panBy, select, zoomBy } from "./state";

/** What a renderer in three dimensions says is under a pixel. */
export type Under = (px: number, py: number) => { path: string | undefined; point: [number, number] | undefined };

/** Listen on the canvas that lies over the chip. `space` says what is under a pixel while the
 * chip is drawn in three dimensions. */
export function listen(canvas: HTMLCanvasElement, space: () => Under | undefined): void {
  /** What is under a point of the canvas: what is drawn there or just beside it, or else the
   * smallest instance whose box holds it. */
  const under = (px: number, py: number): { path: string | undefined; point: [number, number] | undefined } => {
    const { scene, view, hidden, device } = get();
    if (!scene) return { path: undefined, point: undefined };
    const seen = device ? space()?.(px, py) : undefined;
    if (seen) return { path: seen.path ?? (seen.point && holding(scene.instances, ...seen.point)), point: seen.point };
    if (!view) return { path: undefined, point: undefined };
    const point = toDesign(view, px, py);
    return { path: at(scene, hidden, point[0], point[1], 3 / view.scale) ?? holding(scene.instances, point[0], point[1]), point };
  };

  canvas.addEventListener("wheel", (event) => {
    event.preventDefault();
    zoomBy(Math.exp(-event.deltaY * 0.0015), event.offsetX, event.offsetY);
  }, { passive: false });
  canvas.addEventListener("contextmenu", (event) => event.preventDefault());

  let dragging: { x: number; y: number; moved: boolean; carry: boolean } | undefined;
  canvas.addEventListener("pointerdown", (event) => {
    // In three dimensions a drag turns the eye about the chip; with Shift, or the other button,
    // it carries the chip across the screen, as every drag does when the chip is drawn flat.
    dragging = { x: event.offsetX, y: event.offsetY, moved: false, carry: !get().device || event.shiftKey || event.button !== 0 };
    canvas.setPointerCapture(event.pointerId);
  });
  canvas.addEventListener("pointermove", (event) => {
    if (dragging) {
      const [dx, dy] = [event.offsetX - dragging.x, event.offsetY - dragging.y];
      if (dragging.moved || Math.abs(dx) + Math.abs(dy) > 3) {
        dragging.moved = true;
        if (dragging.carry) panBy(-dx, dy);
        else orbitBy(dx, dy);
        [dragging.x, dragging.y] = [event.offsetX, event.offsetY];
      }
      return;
    }
    const held = under(event.offsetX, event.offsetY);
    hover(pick(held.path, get().selection), held.point);
  });
  canvas.addEventListener("pointerleave", () => hover(undefined, undefined));
  canvas.addEventListener("pointerup", (event) => {
    const moved = dragging?.moved;
    dragging = undefined;
    if (moved || event.button !== 0) return;
    select(pick(under(event.offsetX, event.offsetY).path, get().selection));
  });

  canvas.addEventListener("keydown", (event) => {
    const step = event.shiftKey ? 200 : 60;
    // The arrows carry the flat chip; in three dimensions they turn the eye, and with Shift carry.
    const arrow = (dx: number, dy: number) => () => (get().device && !event.shiftKey ? orbitBy(-dx * 0.6, dy * 0.6) : panBy(dx, dy));
    const keys: Record<string, () => void> = {
      "+": () => zoomBy(1.4),
      "=": () => zoomBy(1.4),
      "-": () => zoomBy(1 / 1.4),
      f: fitSelection,
      F: fitSelection,
      "0": fitAll,
      Escape: () => select(undefined),
      ArrowLeft: arrow(-step, 0),
      ArrowRight: arrow(step, 0),
      ArrowUp: arrow(0, step),
      ArrowDown: arrow(0, -step),
    };
    const does = keys[event.key];
    if (!does || event.metaKey || event.ctrlKey || event.altKey) return;
    event.preventDefault();
    does();
  });
}

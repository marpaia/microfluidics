// The line between the stage and the panel: dragged, or moved with the arrow keys, it changes
// how much of the preview the panel has. A double click gives the panel back its usual size.

import { get, resizePanel } from "../state";

export function splitter(line: HTMLElement): void {
  /** The panel's size were its edge at a point of the page. */
  const sized = (event: PointerEvent): number => (get().arrangement === "side" ? document.body.clientWidth - event.clientX : document.body.clientHeight - event.clientY);
  let dragging = false;
  line.addEventListener("pointerdown", (event) => {
    dragging = true;
    line.setPointerCapture(event.pointerId);
    document.body.dataset.dragging = "true";
    event.preventDefault();
  });
  line.addEventListener("pointermove", (event) => {
    if (dragging) resizePanel(sized(event));
  });
  const ended = () => {
    dragging = false;
    delete document.body.dataset.dragging;
  };
  line.addEventListener("pointerup", ended);
  line.addEventListener("pointercancel", ended);
  line.addEventListener("dblclick", () => resizePanel(undefined));
  line.addEventListener("keydown", (event) => {
    const { arrangement, panelPx } = get();
    const [more, less] = arrangement === "side" ? ["ArrowLeft", "ArrowRight"] : ["ArrowUp", "ArrowDown"];
    const step = event.shiftKey ? 64 : 16;
    if (event.key === more) resizePanel(panelPx + step);
    else if (event.key === less) resizePanel(panelPx - step);
    else if (event.key === "Home") resizePanel(0);
    else if (event.key === "End") resizePanel(1e6);
    else if (event.key === "Enter") resizePanel(undefined);
    else return;
    event.preventDefault();
  });
}

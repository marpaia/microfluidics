// What is asked of the preview's page at every size: run in the page, it gives each thing that
// is wrong with how it is laid out, as a sentence. Nothing given means nothing is wrong.

export const LAID_OUT = `(() => {
  const wrong = [];
  const seen = (element) => {
    const box = element.getBoundingClientRect();
    if (box.width <= 0 || box.height <= 0) return undefined;
    const style = getComputedStyle(element);
    if (style.visibility === "hidden" || style.display === "none" || Number(style.opacity) === 0) return undefined;
    // What an ancestor has scrolled or clipped out of sight is not on the screen.
    for (let held = element.parentElement; held; held = held.parentElement) {
      const about = held.getBoundingClientRect();
      const clips = getComputedStyle(held).overflow !== "visible";
      if (clips && (box.bottom <= about.top + 1 || box.top >= about.bottom - 1 || box.right <= about.left + 1 || box.left >= about.right - 1)) return undefined;
    }
    return box;
  };
  const named = (element) => (element.className && typeof element.className === "string" ? "." + element.className.trim().split(/\\s+/).join(".") : element.tagName.toLowerCase()) + (element.title ? " [" + element.title.slice(0, 30) + "]" : "") + (element.textContent ? " '" + element.textContent.trim().slice(0, 24) + "'" : "");

  // Nothing scrolls sideways.
  for (const held of [document.documentElement, document.body, ...document.querySelectorAll("#panel, .panes, .pane-body, .summary")]) {
    if (held.scrollWidth > held.clientWidth + 1) wrong.push(named(held) + " scrolls sideways: " + held.scrollWidth + " in " + held.clientWidth);
  }
  // No row of the panel is wider than the panel.
  const panel = document.querySelector("#panel");
  const about = panel && seen(panel);
  if (about) {
    for (const row of panel.querySelectorAll(".row, .requirement, .figures > *, .parameter > *, .pane-head > *, .summary > *, .trying > *, .standing-mark, .more > summary")) {
      const box = seen(row);
      if (box && (box.right > about.right + 1 || box.left < about.left - 1)) wrong.push(named(row) + " is wider than the panel: " + Math.round(box.left) + " to " + Math.round(box.right) + " in " + Math.round(about.left) + " to " + Math.round(about.right));
    }
  }
  // Nothing over the stage lies on anything else over it, or off it.
  const stage = document.querySelector("#stage").getBoundingClientRect();
  const over = [...document.querySelectorAll("#over .segmented, #over .menu > button, #over .toolbar > .icon-button, #over .seeing .group, #over .legend, #over .readout, #over .chip-note, #over .chip-mark")].map((element) => [element, seen(element)]).filter(([, box]) => box);
  for (const [element, box] of over) {
    if (box.left < stage.left - 1 || box.right > stage.right + 1 || box.top < stage.top - 1 || box.bottom > stage.bottom + 1) wrong.push(named(element) + " is off the stage");
  }
  for (let a = 0; a < over.length; a += 1) {
    for (let b = a + 1; b < over.length; b += 1) {
      const [one, other] = [over[a][1], over[b][1]];
      if (one.left < other.right - 1 && other.left < one.right - 1 && one.top < other.bottom - 1 && other.top < one.bottom - 1) wrong.push(named(over[a][0]) + " lies on " + named(over[b][0]));
    }
  }
  // Everything clicked is large enough to click.
  const least = parseFloat(getComputedStyle(document.documentElement).fontSize) * 1.65;
  for (const held of document.querySelectorAll("button, summary, input, .row.acts")) {
    const box = seen(held);
    if (!box || held.closest(".menu-sheet")) continue;
    if (box.height < least - 0.5 && !(held.type === "range")) wrong.push(named(held) + " is only " + box.height.toFixed(1) + " px high");
  }
  return wrong;
})()`;

/** What the page shows, for a test to hold it to. */
export const SHOWN = `(() => ({
  size: [innerWidth, innerHeight],
  rem: parseFloat(getComputedStyle(document.documentElement).fontSize),
  root: [document.documentElement.clientWidth, document.documentElement.clientHeight],
  arrangement: document.body.dataset.arrangement,
  panel: document.body.dataset.panel,
  device: document.body.dataset.device,
  design: document.querySelector(".summary h1")?.textContent ?? null,
  standing: document.querySelector(".summary .standing")?.childNodes[1]?.textContent ?? null,
  marks: [...document.querySelectorAll(".standing-mark")].map((held) => held.textContent),
  problems: [...document.querySelectorAll("#pane-problems .row .message")].map((held) => held.textContent),
  selection: document.querySelector("#pane-selected .what code")?.textContent ?? null,
  toolbar: [...document.querySelectorAll(".toolbar > *")].filter((held) => held.getBoundingClientRect().width > 0 && !held.classList.contains("spacer")).map((held) => (held.className + " " + (held.getAttribute("aria-label") ?? held.querySelector("button")?.title ?? held.textContent)).trim().slice(0, 60)),
  notice: document.querySelector(".notice")?.textContent ?? null,
  foot: document.querySelector(".foot-bar")?.textContent ?? null,
  unsaved: document.querySelector(".trying")?.textContent ?? null,
}))()`;

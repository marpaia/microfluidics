// The head of the panel: the design, how it stands in a few words, and how each question stands.

import type { JSX } from "preact";

import { place } from "../format";
import { host } from "../host";
import { dismissResult, toggleSection } from "../state";
import { Icon, IconButton } from "../ui/icon";
import { useStore } from "../ui/store";
import { headline, standings } from "../verification";

/** Bring the problems of a question into view, and put the keyboard on the first of them. */
function show(group: string): void {
  toggleSection("problems", true);
  requestAnimationFrame(() => {
    const row = document.querySelector<HTMLElement>(`#pane-problems .row[data-group="${group}"]`) ?? document.querySelector<HTMLElement>("#pane-problems .pane-title");
    row?.scrollIntoView({ block: "nearest" });
    row?.focus();
  });
}

export function Summary(): JSX.Element | null {
  const { scene, tried, result, running, failure, building, buildMs } = useStore();
  if (!scene) return null;
  const count = scene.problems;
  return (
    <header class="summary" data-stale={failure !== undefined}>
      <h1 title={scene.design}>
        {scene.design}
        {building && <Icon name="loading" spin label="building" />}
      </h1>
      {failure ? (
        <>
          <p class="standing bad"><Icon name="error" />Does not build</p>
          <p class="quiet">
            Showing the last design that built.
            {failure.file && <> <button class="link" onClick={() => host.postMessage({ type: "open", file: failure.file, line: failure.line })}>Go to {place(failure.file, failure.line ?? 1)}</button></>}
          </p>
        </>
      ) : (
        <>
          <p class={`standing ${count ? "bad" : "good"}`}>
            <Icon name={count ? "error" : "pass"} />
            {headline(count)}
            {tried && <span class="chip-note warn" title="The chip is shown with values that are tried and not saved">unsaved values</span>}
            {buildMs !== undefined && !building && <span class="built">Built in {buildMs} ms</span>}
          </p>
          <ul class="standings" aria-label="How each question stands">
            {standings(scene).map((held) => {
              const tone = held.problems ? (held.ran ? "bad" : "warn") : "good";
              const said = held.problems ? `${held.name}: ${headline(held.problems).toLowerCase()}. ${held.question}` : `${held.name}: ${held.checks} ${held.checks === 1 ? "check passes" : "checks pass"}. ${held.question}`;
              return (
                <li>
                  <button class={`standing-mark ${tone}`} title={said} aria-label={said} onClick={() => show(held.group)}>
                    <Icon name={held.problems ? (held.ran ? "error" : "circle-slash") : "pass"} />
                    <span>{held.name}</span>
                    {held.problems > 0 && <span class="n">{held.problems}</span>}
                  </button>
                </li>
              );
            })}
          </ul>
        </>
      )}
      {running && <p class="quiet"><Icon name="loading" spin /> {running === "close" ? "Tuning the design…" : `Running ${running}…`}</p>}
      {result && (
        <div class={`result ${result.ok ? "" : "bad"}`} role="status">
          <pre>{result.text}</pre>
          <IconButton icon="close" title="Dismiss" onClick={dismissResult} />
        </div>
      )}
    </header>
  );
}

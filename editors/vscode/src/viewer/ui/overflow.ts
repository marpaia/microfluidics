// A row of controls that gives way in steps when it does not fit: each step is one thing made
// smaller or moved into the row's menu, and the row takes as few steps as let it fit.

import { useLayoutEffect, useRef, useState } from "preact/hooks";
import type { RefObject } from "preact";

/** How many steps the row `held` must give way by, of `most`, to fit its own width. */
export function useSteps(most: number): { held: RefObject<HTMLDivElement>; steps: number } {
  const held = useRef<HTMLDivElement>(null);
  const [steps, setSteps] = useState(0);
  const width = useRef(0);
  // After each drawing: one more step while the row is wider than its room.
  useLayoutEffect(() => {
    const row = held.current;
    if (row && steps < most && row.scrollWidth > row.clientWidth + 1) setSteps(steps + 1);
  });
  // When its room grows, it starts again from nothing given up.
  useLayoutEffect(() => {
    const row = held.current;
    if (!row) return undefined;
    const watching = new ResizeObserver(() => {
      const now = row.clientWidth;
      if (now > width.current + 1) setSteps(0);
      else if (now < width.current - 1) setSteps((given) => given);
      if (Math.abs(now - width.current) > 1 && row.scrollWidth > row.clientWidth + 1) setSteps((given) => Math.min(given + 1, most));
      width.current = now;
    });
    watching.observe(row);
    return () => watching.disconnect();
  }, [most]);
  return { held, steps };
}

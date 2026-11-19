// What colour each thing is, whichever way the chip is drawn: by its layer, or by what flows
// through the edge it belongs to.

import type { Scene } from "../scene";
import { owner, ramp, share, spansDecades, within } from "./geometry";
import { type State, MODES, valuesIn } from "./state";

/** What a legend says of a mode: the least and greatest value, and whether they are told apart by
 * their logarithm. `undefined` when the mode shows nothing measured. */
export function legend(scene: Scene, state: State): { low: number; high: number; log: boolean; unit: string } | undefined {
  const values = [...valuesIn(scene, state.mode).values()];
  if (!values.length) return undefined;
  const [least, most] = [Math.min(...values), Math.max(...values)];
  // Values that differ by less than a part in a thousand are one value, and one colour.
  const [low, high] = most - least < 1e-3 * Math.abs(most) ? [most, most] : [least, most];
  // Pressure falls evenly along a channel, so it is never shown by its logarithm.
  return { low, high, log: state.mode !== "pressure" && spansDecades(low, high), unit: MODES.find((held) => held.mode === state.mode)?.unit ?? "" };
}

/** How to colour and how strongly to draw each thing: by layer or by what flows, and fainter
 * when something else is selected. */
export function painter(scene: Scene, state: State): (path: string, layer: string) => [string, number] {
  const layers = new Map(scene.layers.map((layer) => [layer.name, layer.colour]));
  const values = valuesIn(scene, state.mode);
  const elements = new Set(values.keys());
  const { low, high, log } = legend(scene, state) ?? { low: 0, high: 0, log: false };
  const strong = state.failure ? 0.3 : 0.92;
  return (path, layer) => {
    if (state.mode !== "layout") {
      // A colour that stands for a value is drawn at full strength, or it would read as another value.
      const edge = owner(path, elements);
      return edge === undefined ? ["rgb(128, 128, 128)", strong * 0.4] : [ramp(share(values.get(edge)!, low, high, log)), strong];
    }
    const apart = state.selection !== undefined && !within(path, state.selection) && !within(state.selection, path);
    return [layers.get(layer) ?? "#888", apart ? strong * 0.45 : strong];
  };
}

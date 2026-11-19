// The viewer's state as a component reads it.

import { useEffect, useState } from "preact/hooks";

import { type State, get, subscribe } from "../state";

/** The viewer's state, read again whenever it changes. */
export function useStore(): State {
  const [state, setState] = useState(get());
  useEffect(() => {
    // What changed between the first drawing and this is caught up on here.
    setState(get());
    return subscribe(() => setState(get()));
  }, []);
  return state;
}

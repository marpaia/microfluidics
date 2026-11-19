// The editor the viewer runs in: where it sends what it asks, and where it keeps what a person
// chose so that a panel shown again is as it was left.

import type { FromViewer } from "../scene";

interface Host {
  postMessage(message: FromViewer): void;
  getState(): unknown;
  setState(state: unknown): void;
}

declare function acquireVsCodeApi(): Host;

export const host: Host = acquireVsCodeApi();

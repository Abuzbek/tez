import { withoutTracking } from "./graph";

export function untrack<T>(fn: () => T): T {
  return withoutTracking(fn);
}

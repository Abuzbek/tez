import type { Observer } from "./graph";

export function markObserversStale(observers: Iterable<Observer>): void {
  for (const observer of observers) {
    observer.notify();
  }
}

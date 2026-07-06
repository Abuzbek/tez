import {
  trackAccess,
  withTracking,
  type Observer,
  type Source,
  type TrackingObserver,
} from "./graph";
import { markObserversStale } from "./propagate";

type ComputedState = "clean" | "stale" | "computing";

export interface ComputedOptions<T> {
  equals?: (a: T, b: T) => boolean;
}

export class Computed<T> implements Source, TrackingObserver {
  private cachedValue!: T;
  private hasValue = false;
  private state: ComputedState = "stale";
  private readonly sources = new Set<Source>();
  private readonly observers = new Set<Observer>();
  private readonly compute: () => T;
  private readonly equals: (a: T, b: T) => boolean;

  constructor(compute: () => T, options?: ComputedOptions<T>) {
    this.compute = compute;
    this.equals = options?.equals ?? Object.is;
  }

  get(): T {
    if (this.state !== "clean") {
      this.recompute();
    }
    trackAccess(this);
    return this.cachedValue;
  }

  private recompute(): void {
    if (this.state === "computing") {
      throw new Error(
        "Signal.Computed cycle detected: computation read its own value while computing",
      );
    }
    this.state = "computing";
    this.unsubscribeFromSources();

    const newValue = withTracking(this, () => this.compute());

    this.state = "clean";
    if (!this.hasValue || !this.equals(this.cachedValue, newValue)) {
      this.cachedValue = newValue;
      this.hasValue = true;
    }
  }

  private unsubscribeFromSources(): void {
    for (const source of this.sources) {
      source.removeObserver(this);
    }
    this.sources.clear();
  }

  addSource(source: Source): void {
    this.sources.add(source);
  }

  notify(): void {
    if (this.state === "clean") {
      this.state = "stale";
      markObserversStale(this.observers);
    }
  }

  addObserver(observer: Observer): void {
    this.observers.add(observer);
  }

  removeObserver(observer: Observer): void {
    this.observers.delete(observer);
  }

  dispose(): void {
    this.unsubscribeFromSources();
  }
}

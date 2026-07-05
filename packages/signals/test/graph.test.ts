import { describe, expect, it, vi } from "vitest";
import {
  getCurrentConsumer,
  trackAccess,
  withTracking,
  withoutTracking,
  type Source,
  type TrackingObserver,
} from "../src/graph";

function makeSource(): Source & { observers: Set<unknown> } {
  const observers = new Set<unknown>();
  return {
    observers,
    addObserver: vi.fn((observer) => observers.add(observer)),
    removeObserver: vi.fn((observer) => observers.delete(observer)),
  };
}

function makeConsumer(): TrackingObserver & { sources: Set<Source> } {
  const sources = new Set<Source>();
  return {
    sources,
    notify: vi.fn(),
    addSource: vi.fn((source) => sources.add(source)),
  };
}

describe("graph tracking primitives", () => {
  it("has no current consumer outside of withTracking", () => {
    expect(getCurrentConsumer()).toBeNull();
  });

  it("trackAccess is a no-op when there is no current consumer", () => {
    const source = makeSource();
    trackAccess(source);
    expect(source.addObserver).not.toHaveBeenCalled();
  });

  it("withTracking makes the consumer current for the duration of fn", () => {
    const consumer = makeConsumer();
    const source = makeSource();
    let observedDuring: TrackingObserver | null = null;

    withTracking(consumer, () => {
      observedDuring = getCurrentConsumer();
      trackAccess(source);
    });

    expect(observedDuring).toBe(consumer);
    expect(getCurrentConsumer()).toBeNull();
    expect(consumer.addSource).toHaveBeenCalledWith(source);
  });

  it("withTracking restores the previous consumer after fn returns", () => {
    const outer = makeConsumer();
    const inner = makeConsumer();
    let observedInner: TrackingObserver | null = null;

    withTracking(outer, () => {
      withTracking(inner, () => {
        observedInner = getCurrentConsumer();
      });
      expect(getCurrentConsumer()).toBe(outer);
    });

    expect(observedInner).toBe(inner);
  });

  it("withTracking restores the previous consumer even if fn throws", () => {
    const consumer = makeConsumer();
    expect(() =>
      withTracking(consumer, () => {
        throw new Error("boom");
      }),
    ).toThrow("boom");
    expect(getCurrentConsumer()).toBeNull();
  });

  it("withoutTracking suspends the current consumer for the duration of fn", () => {
    const consumer = makeConsumer();
    const source = makeSource();
    let observedDuring: TrackingObserver | null = consumer;

    withTracking(consumer, () => {
      withoutTracking(() => {
        observedDuring = getCurrentConsumer();
        trackAccess(source);
      });
      expect(getCurrentConsumer()).toBe(consumer);
    });

    expect(observedDuring).toBeNull();
    expect(consumer.addSource).not.toHaveBeenCalled();
  });

  it("withoutTracking restores the previous consumer even if fn throws", () => {
    const consumer = makeConsumer();
    withTracking(consumer, () => {
      expect(() =>
        withoutTracking(() => {
          throw new Error("boom");
        }),
      ).toThrow("boom");
      expect(getCurrentConsumer()).toBe(consumer);
    });
  });
});

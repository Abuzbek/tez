import { describe, expect, it } from "vitest";
import { Signal } from "../src/index";

// Adapted from the TC39 proposal-signals reference polyfill test suite
// (github.com/proposal-signals/signal-polyfill), trimmed to the behaviors
// this package implements: State, Computed, and subtle.Watcher.
describe("adapted TC39 signal-polyfill test subset", () => {
  it("Signal.State: get returns the set value", () => {
    const s = new Signal.State(1);
    expect(s.get()).toBe(1);
    s.set(2);
    expect(s.get()).toBe(2);
  });

  it("Signal.Computed: recomputes only when read after a dependency changes", () => {
    const s = new Signal.State(1);
    let calls = 0;
    const c = new Signal.Computed(() => {
      calls++;
      return s.get() * 2;
    });
    expect(calls).toBe(0);
    expect(c.get()).toBe(2);
    expect(calls).toBe(1);
    expect(c.get()).toBe(2);
    expect(calls).toBe(1);
    s.set(2);
    expect(calls).toBe(1);
    expect(c.get()).toBe(4);
    expect(calls).toBe(2);
  });

  it("Signal.subtle.Watcher: notify fires once per change, then must be re-armed", () => {
    const s = new Signal.State(1);
    const notifications: number[] = [];
    const w = new Signal.subtle.Watcher(() => notifications.push(s.get()));
    w.watch(s);
    s.set(2);
    s.set(3);
    expect(notifications).toEqual([2]);
    w.watch(s);
    s.set(4);
    expect(notifications).toEqual([2, 4]);
  });

  it("diamond dependency graph resolves to a consistent final value", () => {
    const s = new Signal.State(2);
    const double = new Signal.Computed(() => s.get() * 2);
    const triple = new Signal.Computed(() => s.get() * 3);
    const sum = new Signal.Computed(() => double.get() + triple.get());

    expect(sum.get()).toBe(10);
    s.set(3);
    expect(sum.get()).toBe(15);
  });

  it("Watcher.getPending reflects signals disarmed by a notification", () => {
    const a = new Signal.State(1);
    const b = new Signal.State(2);
    const w = new Signal.subtle.Watcher(() => {});
    w.watch(a, b);
    expect(w.getPending()).toEqual([]);
    a.set(10);
    expect(w.getPending()).toEqual([a, b]);
  });
});

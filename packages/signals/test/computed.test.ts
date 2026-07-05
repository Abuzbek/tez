import { describe, expect, it, vi } from "vitest";
import { State } from "../src/state";
import { Computed } from "../src/computed";

describe("Computed", () => {
  it("computes its value lazily from a compute function", () => {
    const a = new State(2);
    const b = new State(3);
    const sum = new Computed(() => a.get() + b.get());
    expect(sum.get()).toBe(5);
  });

  it("does not call compute until get() is called", () => {
    const compute = vi.fn(() => 42);
    new Computed(compute);
    expect(compute).not.toHaveBeenCalled();
  });

  it("caches the value and does not recompute on repeated get() with no source change", () => {
    const compute = vi.fn(() => 42);
    const c = new Computed(compute);
    c.get();
    c.get();
    c.get();
    expect(compute).toHaveBeenCalledOnce();
  });

  it("recomputes after a source changes", () => {
    const a = new State(1);
    const compute = vi.fn(() => a.get() * 10);
    const c = new Computed(compute);
    expect(c.get()).toBe(10);
    a.set(2);
    expect(c.get()).toBe(20);
    expect(compute).toHaveBeenCalledTimes(2);
  });

  it("propagates staleness to computeds that depend on it", () => {
    const a = new State(1);
    const b = new Computed(() => a.get() + 1);
    const c = new Computed(() => b.get() + 1);
    expect(c.get()).toBe(3);
    a.set(10);
    expect(c.get()).toBe(12);
  });

  it("diamond dependency: each computed recomputes exactly once per source change", () => {
    const a = new State(1);
    const bCompute = vi.fn(() => a.get() + 1);
    const cCompute = vi.fn(() => a.get() + 2);
    const b = new Computed(bCompute);
    const c = new Computed(cCompute);
    const dCompute = vi.fn(() => b.get() + c.get());
    const d = new Computed(dCompute);

    expect(d.get()).toBe(1 + 1 + 1 + 2);
    a.set(5);
    expect(d.get()).toBe(5 + 1 + 5 + 2);

    expect(bCompute).toHaveBeenCalledTimes(2);
    expect(cCompute).toHaveBeenCalledTimes(2);
    expect(dCompute).toHaveBeenCalledTimes(2);
  });

  it("throws when a computation reads its own value (cycle)", () => {
    const c: Computed<number> = new Computed(() => c.get() + 1);
    expect(() => c.get()).toThrow(/cycle/i);
  });

  it("supports a custom equals option to keep the cached reference stable", () => {
    const a = new State(1);
    const c = new Computed(() => ({ n: a.get() % 2 }), {
      equals: (x, y) => x.n === y.n,
    });
    const first = c.get();
    a.set(3);
    const second = c.get();
    expect(second).toBe(first);
  });

  it("notify() is idempotent while already stale (no duplicate propagation)", () => {
    const a = new State(1);
    const b2 = new State(2);
    const c = new Computed(() => a.get() + b2.get());
    c.get();
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    c.addObserver(observer);
    a.set(10);
    b2.set(20);
    expect(observer.notify).toHaveBeenCalledOnce();
  });

  it("removeObserver stops further staleness propagation", () => {
    const a = new State(1);
    const c = new Computed(() => a.get());
    c.get();
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    c.addObserver(observer);
    c.removeObserver(observer);
    a.set(2);
    expect(observer.notify).not.toHaveBeenCalled();
  });
});

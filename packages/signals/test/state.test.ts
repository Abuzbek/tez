import { describe, expect, it, vi } from "vitest";
import { State } from "../src/state";
import { withTracking } from "../src/graph";

describe("State", () => {
  it("get() returns the initial value", () => {
    const s = new State(1);
    expect(s.get()).toBe(1);
  });

  it("set() updates the value observed by a subsequent get()", () => {
    const s = new State(1);
    s.set(2);
    expect(s.get()).toBe(2);
  });

  it("set() with an Object.is-equal value does not notify observers", () => {
    const s = new State(1);
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    withTracking(observer, () => s.get());
    s.set(1);
    expect(observer.notify).not.toHaveBeenCalled();
  });

  it("set() with a changed value notifies observers", () => {
    const s = new State(1);
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    withTracking(observer, () => s.get());
    s.set(2);
    expect(observer.notify).toHaveBeenCalledOnce();
  });

  it("get() inside withTracking registers the state as a source of the current consumer", () => {
    const s = new State(1);
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    withTracking(observer, () => s.get());
    expect(observer.addSource).toHaveBeenCalledWith(s);
  });

  it("removeObserver stops future notifications", () => {
    const s = new State(1);
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    withTracking(observer, () => s.get());
    s.removeObserver(observer);
    s.set(2);
    expect(observer.notify).not.toHaveBeenCalled();
  });

  it("supports a custom equals option", () => {
    const s = new State({ n: 1 }, { equals: (a, b) => a.n === b.n });
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    withTracking(observer, () => s.get());
    s.set({ n: 1 });
    expect(observer.notify).not.toHaveBeenCalled();
    s.set({ n: 2 });
    expect(observer.notify).toHaveBeenCalledOnce();
  });
});

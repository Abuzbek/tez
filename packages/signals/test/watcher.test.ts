import { describe, expect, it, vi } from "vitest";
import { State } from "../src/state";
import { Computed } from "../src/computed";
import { Watcher } from "../src/watcher";

describe("Watcher", () => {
  it("does not call onNotify until a watched signal changes", () => {
    const onNotify = vi.fn();
    const s = new State(1);
    const w = new Watcher(onNotify);
    w.watch(s);
    expect(onNotify).not.toHaveBeenCalled();
  });

  it("calls onNotify once when a watched State changes", () => {
    const onNotify = vi.fn();
    const s = new State(1);
    const w = new Watcher(onNotify);
    w.watch(s);
    s.set(2);
    expect(onNotify).toHaveBeenCalledOnce();
  });

  it("calls onNotify when a watched Computed's dependency changes", () => {
    const onNotify = vi.fn();
    const a = new State(1);
    const c = new Computed(() => a.get() * 2);
    c.get();
    const w = new Watcher(onNotify);
    w.watch(c);
    a.set(5);
    expect(onNotify).toHaveBeenCalledOnce();
  });

  it("does not fire again for the same signal until re-armed via watch()", () => {
    const onNotify = vi.fn();
    const s = new State(1);
    const w = new Watcher(onNotify);
    w.watch(s);
    s.set(2);
    s.set(3);
    expect(onNotify).toHaveBeenCalledOnce();
  });

  it("fires again after being re-armed with watch()", () => {
    const onNotify = vi.fn();
    const s = new State(1);
    const w = new Watcher(onNotify);
    w.watch(s);
    s.set(2);
    w.watch(s);
    s.set(3);
    expect(onNotify).toHaveBeenCalledTimes(2);
  });

  it("unwatch() stops future notifications for that signal", () => {
    const onNotify = vi.fn();
    const s = new State(1);
    const w = new Watcher(onNotify);
    w.watch(s);
    w.unwatch(s);
    s.set(2);
    expect(onNotify).not.toHaveBeenCalled();
  });

  it("getPending() is empty before any change", () => {
    const s = new State(1);
    const w = new Watcher(() => {});
    w.watch(s);
    expect(w.getPending()).toEqual([]);
  });

  it("getPending() lists signals disarmed by a notification", () => {
    const s1 = new State(1);
    const s2 = new State(2);
    const w = new Watcher(() => {});
    w.watch(s1, s2);
    s1.set(10);
    expect(w.getPending()).toEqual([s1, s2]);
  });

  it("watching the same signal twice before any notification does not double-register", () => {
    const onNotify = vi.fn();
    const s = new State(1);
    const w = new Watcher(onNotify);
    w.watch(s);
    w.watch(s);
    s.set(2);
    expect(onNotify).toHaveBeenCalledOnce();
  });
});

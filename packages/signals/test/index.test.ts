import { describe, expect, it, vi } from "vitest";
import { signal, computed, effect, batch, untrack, Signal } from "../src/index";

describe("public API", () => {
  it("signal() creates a Signal.State instance", () => {
    const s = signal(1);
    expect(s).toBeInstanceOf(Signal.State);
    expect(s.get()).toBe(1);
  });

  it("computed() creates a Signal.Computed instance", () => {
    const s = signal(2);
    const c = computed(() => s.get() * 2);
    expect(c).toBeInstanceOf(Signal.Computed);
    expect(c.get()).toBe(4);
  });

  it("Signal.subtle.Watcher observes a signal() and a computed()", () => {
    const s = signal(1);
    const c = computed(() => s.get() + 1);
    c.get();
    const onNotify = vi.fn();
    const w = new Signal.subtle.Watcher(onNotify);
    w.watch(s, c);
    s.set(2);
    expect(onNotify).toHaveBeenCalledOnce();
  });

  it("effect() reacts to a signal() write", async () => {
    const s = signal(1);
    const seen: number[] = [];
    effect(() => {
      seen.push(s.get());
    });
    s.set(2);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(seen).toEqual([1, 2]);
  });

  it("batch() and untrack() are re-exported and usable together", () => {
    const a = signal(1);
    const b = signal(2);
    let reads = 0;
    const c = computed(() => {
      reads++;
      return a.get() + untrack(() => b.get());
    });
    expect(c.get()).toBe(3);
    batch(() => {
      a.set(10);
      b.set(20);
    });
    expect(c.get()).toBe(30);
    expect(reads).toBe(2);
  });
});

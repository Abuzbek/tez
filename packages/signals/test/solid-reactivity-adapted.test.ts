import { describe, expect, it } from "vitest";
import { signal, computed, effect, batch, untrack } from "../src/index";

function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

// Adapted from Solid.js's reactivity test suite (solidjs/solid,
// packages/solid/test/effects.spec.ts and signals.spec.ts), trimmed to the
// behaviors this package implements: signal/computed/effect/batch/untrack.
describe("adapted Solid reactivity test subset", () => {
  it("createSignal-equivalent: a computed only updates after its source changes", () => {
    const count = signal(0);
    const double = computed(() => count.get() * 2);
    expect(double.get()).toBe(0);
    count.set(5);
    expect(double.get()).toBe(10);
  });

  it("createMemo-equivalent: does not recompute when set to an equal value", () => {
    const s = signal(1);
    let calls = 0;
    const c = computed(() => {
      calls++;
      return s.get();
    });
    c.get();
    s.set(1);
    c.get();
    expect(calls).toBe(1);
  });

  it("createEffect-equivalent: batches multiple writes into a single effect run", async () => {
    const a = signal(1);
    const b = signal(2);
    let runs = 0;
    effect(() => {
      a.get();
      b.get();
      runs++;
    });
    runs = 0;
    batch(() => {
      a.set(10);
      b.set(20);
    });
    await flush();
    expect(runs).toBe(1);
  });

  it("untrack prevents a read inside an effect from becoming a dependency", async () => {
    const tracked = signal(1);
    const silent = signal(100);
    const runs: number[] = [];
    effect(() => {
      runs.push(tracked.get() + untrack(() => silent.get()));
    });
    silent.set(999);
    await flush();
    expect(runs).toEqual([101]);
    tracked.set(2);
    await flush();
    expect(runs).toEqual([101, 2 + 999]);
  });

  it("nested effects (owner tree) dispose children when the parent reruns", async () => {
    const outer = signal(0);
    const inner = signal("x");
    let innerDisposed = 0;
    let innerRuns = 0;

    effect(() => {
      outer.get();
      effect(() => {
        inner.get();
        innerRuns++;
        return () => {
          innerDisposed++;
        };
      });
    });

    expect(innerRuns).toBe(1);
    outer.set(1);
    await flush();
    expect(innerDisposed).toBe(1);
    expect(innerRuns).toBe(2);
  });

  it("disposing the root effect tears down the whole subtree", () => {
    const inner = signal("x");
    let disposed = 0;
    const dispose = effect(() => {
      effect(() => {
        inner.get();
        return () => {
          disposed++;
        };
      });
    });
    dispose();
    expect(disposed).toBe(1);
  });
});

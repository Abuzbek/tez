import { describe, expect, it, vi } from "vitest";
import { State } from "../src/state";
import { batch } from "../src/batch";
import { effect } from "../src/effect";

function flushMicrotasks(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

describe("effect", () => {
  it("runs fn immediately upon creation", () => {
    const fn = vi.fn();
    effect(fn);
    expect(fn).toHaveBeenCalledOnce();
  });

  it("re-runs when a signal it read changes", async () => {
    const s = new State(1);
    const seen: number[] = [];
    effect(() => {
      seen.push(s.get());
    });
    s.set(2);
    await flushMicrotasks();
    expect(seen).toEqual([1, 2]);
  });

  it("does not re-run when an unrelated signal changes", async () => {
    const a = new State(1);
    const b = new State(100);
    const seen: number[] = [];
    effect(() => {
      seen.push(a.get());
    });
    b.set(999);
    await flushMicrotasks();
    expect(seen).toEqual([1]);
  });

  it("calls the previous cleanup before re-running", async () => {
    const s = new State(1);
    const cleanup = vi.fn();
    effect(() => {
      s.get();
      return cleanup;
    });
    s.set(2);
    await flushMicrotasks();
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it("calls cleanup on dispose", () => {
    const cleanup = vi.fn();
    const dispose = effect(() => cleanup);
    dispose();
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it("does not run again after dispose", async () => {
    const s = new State(1);
    const fn = vi.fn(() => {
      s.get();
    });
    const dispose = effect(fn);
    dispose();
    fn.mockClear();
    s.set(2);
    await flushMicrotasks();
    expect(fn).not.toHaveBeenCalled();
  });

  it("dispose is idempotent", () => {
    const cleanup = vi.fn();
    const dispose = effect(() => cleanup);
    dispose();
    dispose();
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it("coalesces multiple synchronous writes inside batch() into a single re-run", async () => {
    const a = new State(1);
    const b = new State(2);
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
    expect(runs).toBe(1);
  });

  it("auto-disposes a nested effect when the parent re-runs", async () => {
    const outer = new State(0);
    const inner = new State("a");
    const innerCleanup = vi.fn();
    const innerRuns: string[] = [];

    effect(() => {
      outer.get();
      effect(() => {
        innerRuns.push(inner.get());
        return innerCleanup;
      });
    });

    expect(innerRuns).toEqual(["a"]);
    outer.set(1);
    await flushMicrotasks();
    expect(innerCleanup).toHaveBeenCalledOnce();
    expect(innerRuns).toEqual(["a", "a"]);

    inner.set("b");
    await flushMicrotasks();
    expect(innerRuns).toEqual(["a", "a", "b"]);
  });

  it("does not run again if disposed after being scheduled but before the flush executes", () => {
    const s = new State(1);
    const fn = vi.fn(() => {
      s.get();
    });
    batch(() => {
      const dispose = effect(fn);
      fn.mockClear();
      s.set(2); // schedules the effect (batchDepth > 0, so no immediate flush)
      dispose(); // disposes before flushEffects() runs at the end of batch()
    });
    expect(fn).not.toHaveBeenCalled();
  });

  it("auto-disposes a nested effect when the parent is disposed", () => {
    const inner = new State("a");
    const innerCleanup = vi.fn();

    const disposeOuter = effect(() => {
      effect(() => {
        inner.get();
        return innerCleanup;
      });
    });

    disposeOuter();
    expect(innerCleanup).toHaveBeenCalledOnce();
  });
});

import { describe, expect, it, vi } from "vitest";
import { batch, scheduleEffect } from "../src/batch";

describe("batch", () => {
  it("returns the value produced by fn", () => {
    expect(batch(() => 42)).toBe(42);
  });

  it("runs a scheduled effect immediately when not batching", () => {
    const run = vi.fn();
    scheduleEffect({ run });
    expect(run).toHaveBeenCalledOnce();
  });

  it("defers scheduled effects until the outermost batch completes", () => {
    const run = vi.fn();
    batch(() => {
      scheduleEffect({ run });
      expect(run).not.toHaveBeenCalled();
    });
    expect(run).toHaveBeenCalledOnce();
  });

  it("coalesces multiple schedules of the same effect into a single run", () => {
    const run = vi.fn();
    const effect = { run };
    batch(() => {
      scheduleEffect(effect);
      scheduleEffect(effect);
      scheduleEffect(effect);
    });
    expect(run).toHaveBeenCalledOnce();
  });

  it("supports nested batch() calls, flushing only when the outermost one exits", () => {
    const run = vi.fn();
    batch(() => {
      batch(() => {
        scheduleEffect({ run });
      });
      expect(run).not.toHaveBeenCalled();
    });
    expect(run).toHaveBeenCalledOnce();
  });

  it("still flushes pending effects if fn throws", () => {
    const run = vi.fn();
    expect(() =>
      batch(() => {
        scheduleEffect({ run });
        throw new Error("boom");
      }),
    ).toThrow("boom");
    expect(run).toHaveBeenCalledOnce();
  });
});

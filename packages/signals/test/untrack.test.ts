import { describe, expect, it } from "vitest";
import { State } from "../src/state";
import { Computed } from "../src/computed";
import { untrack } from "../src/untrack";

describe("untrack", () => {
  it("returns the value produced by fn", () => {
    expect(untrack(() => 7)).toBe(7);
  });

  it("prevents a computed from registering a dependency read inside it", () => {
    const a = new State(1);
    const b = new State(100);
    let computeCount = 0;
    const c = new Computed(() => {
      computeCount++;
      return a.get() + untrack(() => b.get());
    });

    expect(c.get()).toBe(101);
    b.set(999);
    expect(c.get()).toBe(101);
    expect(computeCount).toBe(1);

    a.set(2);
    expect(c.get()).toBe(2 + 999);
    expect(computeCount).toBe(2);
  });

  it("restores tracking after fn returns even if fn throws", () => {
    const a = new State(1);
    const c = new Computed(() => {
      let caught = false;
      try {
        untrack(() => {
          throw new Error("boom");
        });
      } catch {
        caught = true;
      }
      return caught ? a.get() : -1;
    });
    expect(c.get()).toBe(1);
    a.set(2);
    expect(c.get()).toBe(2);
  });
});

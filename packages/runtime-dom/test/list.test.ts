import { describe, expect, it } from "vitest";
import { signal, effect } from "@tez/signals";
import { mapArray } from "../src/list";
import { mount } from "../src/mount";

interface Item {
  id: number;
  label: string;
}

describe("mapArray", () => {
  it("renders one node per item, in order", () => {
    const items = signal<Item[]>([
      { id: 1, label: "a" },
      { id: 2, label: "b" },
    ]);
    const getNodes = mapArray(
      () => items.get(),
      (item) => item.id,
      (item) => {
        const el = document.createElement("li");
        el.textContent = item().label;
        return el;
      },
    );
    expect(getNodes().map((n) => n.textContent)).toEqual(["a", "b"]);
  });

  it("reuses the existing DOM node for a persisting key instead of re-rendering", () => {
    const items = signal<Item[]>([{ id: 1, label: "a" }]);
    let renderCount = 0;
    const getNodes = mapArray(
      () => items.get(),
      (item) => item.id,
      (item) => {
        renderCount++;
        const el = document.createElement("li");
        el.textContent = item().label;
        return el;
      },
    );
    const [firstNode] = getNodes();
    items.set([{ id: 1, label: "a-renamed" }]);
    const [secondNode] = getNodes();
    expect(secondNode).toBe(firstNode);
    expect(renderCount).toBe(1);
  });

  it("does not update the DOM automatically unless renderItem wires an explicit binding", () => {
    // mapArray only updates the per-item signal; whether the rendered node's
    // content reacts depends on renderItem reading the accessor inside its
    // own effect/binding (as insert()/setAttr() do). A one-time read at
    // render time does not see later updates.
    const items = signal<Item[]>([{ id: 1, label: "a" }]);
    const getNodes = mapArray(
      () => items.get(),
      (item) => item.id,
      (item) => {
        const el = document.createElement("li");
        el.textContent = item().label;
        return el;
      },
    );
    const [node] = getNodes();
    items.set([{ id: 1, label: "a-renamed" }]);
    expect(node.textContent).toBe("a");
  });

  it("disposes a removed item's effect scope", () => {
    const items = signal<Item[]>([
      { id: 1, label: "a" },
      { id: 2, label: "b" },
    ]);
    const disposedIds: number[] = [];
    mapArray(
      () => items.get(),
      (item) => item.id,
      (item) => {
        const el = document.createElement("li");
        const id = item().id;
        // No reactive read inside this probe effect on purpose: it exists to
        // observe *permanent* disposal on key removal, not the ordinary
        // cleanup-then-rerun cycle a genuinely tracked dependency would also
        // produce on every persisting-item update.
        effect(() => {
          return () => disposedIds.push(id);
        });
        return el;
      },
    );
    items.set([{ id: 1, label: "a" }]);
    expect(disposedIds).toEqual([2]);
  });

  it("adds a new node for a new key without re-rendering existing entries", () => {
    const items = signal<Item[]>([{ id: 1, label: "a" }]);
    let renderCount = 0;
    const getNodes = mapArray(
      () => items.get(),
      (item) => item.id,
      (item) => {
        renderCount++;
        const el = document.createElement("li");
        el.textContent = item().label;
        return el;
      },
    );
    const [firstNode] = getNodes();
    items.set([
      { id: 1, label: "a" },
      { id: 2, label: "b" },
    ]);
    const nodes = getNodes();
    expect(nodes.length).toBe(2);
    expect(nodes[0]).toBe(firstNode);
    expect(renderCount).toBe(2);
  });

  it("the returned accessor is reactive: an effect wrapping it re-runs when the list changes", () => {
    const items = signal<Item[]>([{ id: 1, label: "a" }]);
    const getNodes = mapArray(
      () => items.get(),
      (item) => item.id,
      (item) => {
        const el = document.createElement("li");
        el.textContent = item().label;
        return el;
      },
    );
    const container = document.createElement("ul");
    let runs = 0;
    effect(() => {
      container.replaceChildren(...getNodes());
      runs++;
    });
    expect(runs).toBe(1);
    expect(container.children.length).toBe(1);
    items.set([
      { id: 1, label: "a" },
      { id: 2, label: "b" },
    ]);
    expect(runs).toBe(2);
    expect(container.children.length).toBe(2);
  });

  it("does not leak items() as a dependency of an enclosing effect (e.g. mount())", () => {
    const items = signal<Item[]>([{ id: 1, label: "a" }]);
    let mountEffectRuns = 0;
    const target = document.createElement("div");

    mount(() => {
      mountEffectRuns++;
      const el = document.createElement("ul");
      const getNodes = mapArray(
        () => items.get(),
        (item) => item.id,
        (item) => {
          const li = document.createElement("li");
          li.textContent = item().label;
          return li;
        },
      );
      effect(() => {
        el.replaceChildren(...getNodes());
      });
      return el;
    }, target);

    expect(mountEffectRuns).toBe(1);
    items.set([{ id: 1, label: "a-renamed" }]);
    expect(mountEffectRuns).toBe(1);
  });
});

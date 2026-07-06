# DOM Runtime Primitives Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `packages/runtime-dom`'s reactive DOM primitives (`template`, `insert`/`setAttr`/`toggleClass`, `listen`, `mapArray`, `mount`) and prove them with a hand-written counter + keyed todo-list demo in `apps/playground`, verified both by unit tests and a real-browser Playwright suite.

**Architecture:** Five single-purpose modules in `packages/runtime-dom/src/`, each wrapping DOM mutations in `effect()` from `@tez/signals` so updates are fine-grained (no VDOM, no component re-execution). `mapArray` does keyed reconciliation via per-item `signal`s, reusing DOM nodes across updates. `mount()` establishes a disposal scope by wrapping the one-time component call in `effect()`, reusing Phase 0's already-tested owner-scope machinery rather than adding a new primitive to `packages/signals`.

**Tech Stack:** TypeScript, `@tez/signals` (workspace dependency), Vitest + happy-dom (unit tests), Vite (playground dev/build), Playwright (browser verification).

## Global Constraints

- `packages/runtime-dom` may import only from `packages/signals` (repo layout rule, main spec §1).
- `effect()` flushes synchronously outside `batch()` (established in Phase 0) — do not write tests that `await` a delay expecting async scheduling; assert immediately after a `.set()` call.
- Out of scope this cycle (see design doc §5): SSR, resumability (`restoreSignal`, QRLs), the compiler, public `<Show>`/`<For>`/`<Switch>`/`<Dynamic>` components, TEZ10x error codes, the ≤4 KB gzip budget CI gate, and any change to `packages/signals`' public API.
- Design reference: `docs/superpowers/specs/2026-07-06-runtime-dom-primitives-design.md`.

---

## Task 1: `packages/runtime-dom` package setup + `template()`

**Files:**
- Modify: `packages/runtime-dom/package.json`
- Modify: `packages/runtime-dom/tsconfig.json`
- Create: `packages/runtime-dom/vitest.config.ts`
- Modify: `packages/runtime-dom/src/index.ts` (currently a stub `export {};`)
- Create: `packages/runtime-dom/src/template.ts`
- Test: `packages/runtime-dom/test/template.test.ts`

**Interfaces:**
- Produces: `template(html: string): () => Node` — clones a cached `<template>`'s single root child on each factory call; throws if `html` has no root element. Tasks 2–7 assume this exists but don't directly depend on its output type beyond `Node`.

- [ ] **Step 1: Update the package manifest**

`packages/runtime-dom/package.json`:
```json
{
  "name": "@tez/runtime-dom",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "build": "echo 'no build yet'",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "@tez/signals": "workspace:*"
  },
  "devDependencies": {
    "vitest": "^2.1.8",
    "@vitest/coverage-v8": "^2.1.8",
    "happy-dom": "^15.0.0",
    "typescript": "^5.6.0"
  }
}
```

- [ ] **Step 2: Add DOM lib to the package's tsconfig**

`packages/runtime-dom/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src",
    "lib": ["ES2022", "DOM", "DOM.Iterable"]
  },
  "include": ["src"]
}
```

(The root `tsconfig.base.json` sets `"lib": ["ES2022"]` only, with no DOM types — `packages/signals` doesn't need them, but `runtime-dom` does.)

- [ ] **Step 3: Add a vitest config with the happy-dom environment**

`packages/runtime-dom/vitest.config.ts`:
```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "happy-dom",
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/**/*.ts"],
    },
  },
});
```

- [ ] **Step 4: Install dependencies**

Run: `pnpm install`
Expected: resolves `@tez/signals` via the workspace symlink and installs vitest/happy-dom/typescript with no errors.

- [ ] **Step 5: Write the failing test for `template()`**

`packages/runtime-dom/test/template.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { template } from "../src/template";

describe("template", () => {
  it("clones the template's root element on each call", () => {
    const factory = template("<button>Click me</button>");
    const a = factory();
    const b = factory();
    expect(a).not.toBe(b);
    expect(a).toBeInstanceOf(HTMLButtonElement);
    expect((a as HTMLButtonElement).textContent).toBe("Click me");
    expect((b as HTMLButtonElement).textContent).toBe("Click me");
  });

  it("preserves nested structure and marker comment nodes", () => {
    const factory = template("<div><span>Count: <!></span></div>");
    const node = factory() as HTMLDivElement;
    const span = node.querySelector("span");
    expect(span?.childNodes.length).toBe(2);
    expect(span?.childNodes[1]?.nodeType).toBe(Node.COMMENT_NODE);
  });

  it("returns independent clones that can be mutated without affecting each other", () => {
    const factory = template("<p>Hello</p>");
    const a = factory() as HTMLParagraphElement;
    const b = factory() as HTMLParagraphElement;
    a.textContent = "Changed";
    expect(b.textContent).toBe("Hello");
  });

  it("throws when the html has no root element", () => {
    expect(() => template("")).toThrow(/no root element/i);
  });
});
```

- [ ] **Step 6: Run test to verify it fails**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: FAIL — `Cannot find module '../src/template'`.

- [ ] **Step 7: Implement `template.ts`**

`packages/runtime-dom/src/template.ts`:
```ts
export function template(html: string): () => Node {
  const templateEl = document.createElement("template");
  templateEl.innerHTML = html;
  const root = templateEl.content.firstChild;

  if (!root) {
    throw new Error(`template(): no root element found in: ${html}`);
  }

  return () => root.cloneNode(true) as Node;
}
```

- [ ] **Step 8: Replace the stub `index.ts` to re-export it**

`packages/runtime-dom/src/index.ts`:
```ts
export { template } from "./template";
```

- [ ] **Step 9: Run test to verify it passes**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: PASS — 4 tests in `template.test.ts`.

- [ ] **Step 10: Commit**

```bash
git add packages/runtime-dom
git commit -m "Add runtime-dom package setup and template()"
```

---

## Task 2: `insert()`, `setAttr()`, `toggleClass()`

**Files:**
- Create: `packages/runtime-dom/src/bindings.ts`
- Test: `packages/runtime-dom/test/bindings.test.ts`

**Interfaces:**
- Consumes: `effect` from `@tez/signals`.
- Produces: `insert(parent: Node, accessor: () => unknown, marker: Node): void`, `setAttr(el: Element, name: string, accessor: () => unknown): void`, `toggleClass(el: Element, className: string, accessor: () => boolean): void`. Task 6 (playground demo) and Task 5 (mount tests) both use `insert`.

- [ ] **Step 1: Write the failing tests**

`packages/runtime-dom/test/bindings.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { signal } from "@tez/signals";
import { insert, setAttr, toggleClass } from "../src/bindings";

describe("insert", () => {
  it("inserts the accessor's initial value as text before the marker", () => {
    const parent = document.createElement("div");
    const marker = document.createComment("");
    parent.appendChild(marker);
    insert(parent, () => "hello", marker);
    expect(parent.textContent).toBe("hello");
    expect(parent.lastChild).toBe(marker);
  });

  it("updates the DOM when the accessor's signal changes", () => {
    const parent = document.createElement("div");
    const marker = document.createComment("");
    parent.appendChild(marker);
    const count = signal(1);
    insert(parent, () => count.get(), marker);
    expect(parent.textContent).toBe("1");
    count.set(2);
    expect(parent.textContent).toBe("2");
    expect(parent.childNodes.length).toBe(2);
  });

  it("inserts a real Node value directly instead of stringifying it", () => {
    const parent = document.createElement("div");
    const marker = document.createComment("");
    parent.appendChild(marker);
    const span = document.createElement("span");
    span.textContent = "child";
    insert(parent, () => span, marker);
    expect(parent.querySelector("span")).toBe(span);
  });

  it("renders null and undefined as empty text", () => {
    const parent = document.createElement("div");
    const marker = document.createComment("");
    parent.appendChild(marker);
    insert(parent, () => null, marker);
    expect(parent.textContent).toBe("");
  });
});

describe("setAttr", () => {
  it("sets the attribute to the accessor's initial value", () => {
    const el = document.createElement("input");
    setAttr(el, "placeholder", () => "name");
    expect(el.getAttribute("placeholder")).toBe("name");
  });

  it("updates the attribute reactively", () => {
    const el = document.createElement("input");
    const value = signal("a");
    setAttr(el, "placeholder", () => value.get());
    value.set("b");
    expect(el.getAttribute("placeholder")).toBe("b");
  });

  it("removes the attribute when the value is null or false", () => {
    const el = document.createElement("input");
    const disabled = signal(true);
    setAttr(el, "disabled", () => disabled.get());
    expect(el.getAttribute("disabled")).toBe("true");
    disabled.set(false);
    expect(el.hasAttribute("disabled")).toBe(false);
  });
});

describe("toggleClass", () => {
  it("adds the class when the accessor is true", () => {
    const el = document.createElement("li");
    toggleClass(el, "done", () => true);
    expect(el.classList.contains("done")).toBe(true);
  });

  it("removes the class reactively when the accessor becomes false", () => {
    const el = document.createElement("li");
    const done = signal(true);
    toggleClass(el, "done", () => done.get());
    done.set(false);
    expect(el.classList.contains("done")).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: FAIL — `Cannot find module '../src/bindings'`.

- [ ] **Step 3: Implement `bindings.ts`**

`packages/runtime-dom/src/bindings.ts`:
```ts
import { effect } from "@tez/signals";

function toNode(value: unknown): Node {
  if (value instanceof Node) return value;
  return document.createTextNode(value == null ? "" : String(value));
}

export function insert(parent: Node, accessor: () => unknown, marker: Node): void {
  let current: Node | null = null;

  effect(() => {
    const next = toNode(accessor());

    if (current) {
      parent.replaceChild(next, current);
    } else {
      parent.insertBefore(next, marker);
    }
    current = next;
  });
}

export function setAttr(el: Element, name: string, accessor: () => unknown): void {
  effect(() => {
    const value = accessor();
    if (value == null || value === false) {
      el.removeAttribute(name);
    } else {
      el.setAttribute(name, String(value));
    }
  });
}

export function toggleClass(el: Element, className: string, accessor: () => boolean): void {
  effect(() => {
    el.classList.toggle(className, Boolean(accessor()));
  });
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: PASS — 9 tests in `bindings.test.ts` (13 total with Task 1).

- [ ] **Step 5: Commit**

```bash
git add packages/runtime-dom/src/bindings.ts packages/runtime-dom/test/bindings.test.ts
git commit -m "Add insert(), setAttr(), and toggleClass() bindings"
```

---

## Task 3: `listen()` — delegated events

**Files:**
- Create: `packages/runtime-dom/src/events.ts`
- Test: `packages/runtime-dom/test/events.test.ts`

**Interfaces:**
- Produces: `listen(el: Element, eventName: string, handler: (ev: Event) => void): void`. Task 6 (playground demo) uses this for click/change handlers.

- [ ] **Step 1: Write the failing tests**

`packages/runtime-dom/test/events.test.ts`:
```ts
import { describe, expect, it, vi } from "vitest";
import { listen } from "../src/events";

describe("listen", () => {
  it("invokes the handler when the element is clicked", () => {
    const button = document.createElement("button");
    document.body.appendChild(button);
    const handler = vi.fn();
    listen(button, "click", handler);
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(handler).toHaveBeenCalledOnce();
    button.remove();
  });

  it("delegates from a descendant of the listening element up to the handler", () => {
    const button = document.createElement("button");
    const span = document.createElement("span");
    button.appendChild(span);
    document.body.appendChild(button);
    const handler = vi.fn();
    listen(button, "click", handler);
    span.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(handler).toHaveBeenCalledOnce();
    button.remove();
  });

  it("does not invoke a handler on an unrelated element", () => {
    const button = document.createElement("button");
    const other = document.createElement("div");
    document.body.appendChild(button);
    document.body.appendChild(other);
    const handler = vi.fn();
    listen(button, "click", handler);
    other.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(handler).not.toHaveBeenCalled();
    button.remove();
    other.remove();
  });

  it("supports arbitrary event names via the same $$<name> convention", () => {
    const input = document.createElement("input");
    document.body.appendChild(input);
    const handler = vi.fn();
    listen(input, "input", handler);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    expect(handler).toHaveBeenCalledOnce();
    input.remove();
  });

  it("only registers one document listener per event type across multiple listen() calls", () => {
    const addEventListenerSpy = vi.spyOn(document, "addEventListener");
    const a = document.createElement("button");
    const b = document.createElement("button");
    listen(a, "dblclick", vi.fn());
    listen(b, "dblclick", vi.fn());
    const dblclickCalls = addEventListenerSpy.mock.calls.filter(([type]) => type === "dblclick");
    expect(dblclickCalls.length).toBe(1);
    addEventListenerSpy.mockRestore();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: FAIL — `Cannot find module '../src/events'`.

- [ ] **Step 3: Implement `events.ts`**

`packages/runtime-dom/src/events.ts`:
```ts
const registeredEventTypes = new Set<string>();

export function listen(el: Element, eventName: string, handler: (ev: Event) => void): void {
  (el as unknown as Record<string, unknown>)[`$$${eventName}`] = handler;

  if (registeredEventTypes.has(eventName)) return;
  registeredEventTypes.add(eventName);

  document.addEventListener(eventName, (event) => {
    const prop = `$$${eventName}`;
    let node: Node | null = event.target as Node | null;

    while (node) {
      const handlerOnNode = (node as unknown as Record<string, unknown>)[prop];
      if (typeof handlerOnNode === "function") {
        (handlerOnNode as (ev: Event) => void)(event);
        return;
      }
      node = node.parentNode;
    }
  });
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: PASS — 5 tests in `events.test.ts` (18 total).

- [ ] **Step 5: Commit**

```bash
git add packages/runtime-dom/src/events.ts packages/runtime-dom/test/events.test.ts
git commit -m "Add listen() delegated event handling"
```

---

## Task 4: `mapArray()` — keyed list reconciliation

**Files:**
- Create: `packages/runtime-dom/src/list.ts`
- Test: `packages/runtime-dom/test/list.test.ts`

**Interfaces:**
- Consumes: `signal`, `computed`, `effect`, `untrack`, `Signal` from `@tez/signals`.
- Produces: `mapArray<T, U extends Node>(items: () => T[], keyFn: (item: T) => unknown, renderItem: (item: () => T, index: () => number) => U): () => U[]`. The returned accessor is itself reactive (backed by an internal `signal`), so a caller can do `effect(() => container.replaceChildren(...getNodes()))` and have it re-run whenever the list changes. Task 6 (playground demo) uses this exact pattern for the todo list.

- [ ] **Step 1: Write the failing tests**

`packages/runtime-dom/test/list.test.ts`:
```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: FAIL — `Cannot find module '../src/list'`.

- [ ] **Step 3: Implement `list.ts`**

`packages/runtime-dom/src/list.ts`:
```ts
import { signal, computed, effect, untrack, Signal } from "@tez/signals";

interface ListEntry<T, U extends Node> {
  itemSignal: ReturnType<typeof signal<T>>;
  indexSignal: ReturnType<typeof signal<number>>;
  node: U;
  dispose: () => void;
}

export function mapArray<T, U extends Node>(
  items: () => T[],
  keyFn: (item: T) => unknown,
  renderItem: (item: () => T, index: () => number) => U,
): () => U[] {
  const entries = new Map<unknown, ListEntry<T, U>>();
  const ordered = signal<U[]>([]);
  const itemsView = computed(() => items());

  function reconcile(): void {
    const currentItems = untrack(() => itemsView.get());
    const nextKeys = new Set(currentItems.map(keyFn));

    for (const [key, entry] of entries) {
      if (!nextKeys.has(key)) {
        entry.dispose();
        entries.delete(key);
      }
    }

    const nextOrdered = currentItems.map((item, index) => {
      const key = keyFn(item);
      let entry = entries.get(key);

      if (!entry) {
        const itemSignal = signal(item);
        const indexSignal = signal(index);
        let node!: U;
        const dispose = effect(() => {
          // untrack(): renderItem may read item()/index() synchronously at
          // its own top level (e.g. capturing an id for a closure) — without
          // untrack, that read would make THIS wrapping effect itself track
          // itemSignal/indexSignal, so updating a persisting entry's signal
          // (below, in the reuse branch) would reentrantly re-run renderItem
          // instead of just updating whatever nested binding (insert/setAttr/
          // etc.) explicitly subscribed to it. Nested effect() calls made
          // inside renderItem (e.g. insert()'s own internal effect) are
          // unaffected by this — they establish their own tracking scope
          // regardless of the ambient untrack.
          untrack(() => {
            node = renderItem(() => itemSignal.get(), () => indexSignal.get());
          });
        });
        entry = { itemSignal, indexSignal, node, dispose };
        entries.set(key, entry);
      } else {
        entry.itemSignal.set(item);
        entry.indexSignal.set(index);
      }

      return entry.node;
    });

    ordered.set(nextOrdered);
  }

  reconcile();

  const watcher = new Signal.subtle.Watcher(() => {
    reconcile();
    watcher.watch(itemsView);
  });
  watcher.watch(itemsView);

  return () => ordered.get();
}
```

> **Correction (post-Task-4 review):** the reconciliation loop runs from a
> `Signal.subtle.Watcher` callback, not from `effect()`. `@tez/signals`'
> `Effect` unconditionally calls `disposeChildren()` at the start of every
> re-run (see `packages/signals/src/effect.ts`) — if the reconciliation loop
> itself were an `effect()`, every per-item `effect()` call inside
> `renderItem` would become its child, and *every* list update would
> silently dispose *all* per-item effects (including ones for keys that
> persist), since the reuse branch never recreates them. `Watcher` isn't
> part of the owner/child-disposal tree at all, so per-item effects are only
> ever disposed by this function's own explicit `entry.dispose()` call when
> a key is actually removed. `itemsView` (a `Computed`) exists because
> `Watcher.watch()` needs a real `Source` object to subscribe to, not a bare
> accessor function — wrapping `items` in `computed()` gives it one while
> preserving whatever `items()` itself reads internally as the tracked
> dependency.
>
> **Known residual gap, out of scope for this cycle:** per-item effects
> created during the *initial* `mapArray()` call are parented under
> whatever effect is synchronously active at that moment (typically
> `mount()`'s root effect, if `mapArray` is called during initial component
> construction) and so get disposed automatically if that ancestor is later
> torn down. Per-item effects created by *later* reconciliations (e.g. an
> item added via a subsequent `items.set()` from an event handler) are not
> parented to anything — they are only ever cleaned up by this function's
> own key-removal logic, not by an ancestor's disposal. A full fix would
> require `mapArray` to expose its own disposal handle to the caller,
> changing its approved return type from `() => U[]` to something larger —
> a public API change outside this cycle's scope. Not exercised by this
> cycle's test suite or demo (the playground never unmounts the todo list).
>
> **Second correction (same review round):** the `Watcher` swap above fixed
> the *outer* sweep, but surfaced a second, independent bug: `renderItem`
> commonly reads `item()`/`index()` synchronously at its own top level (e.g.
> `const id = item().id;`, used later in a closure) — a completely normal
> pattern, and exactly what the earlier "disposes a removed item" test does.
> Without `untrack()`, that read makes the *wrapping* per-item `effect()`
> itself track `itemSignal`/`indexSignal` as its own dependency. Then
> `entry.itemSignal.set(item)` in the reuse branch (for a *persisting* key)
> reentrantly re-triggers that same wrapping effect mid-`reconcile()` —
> re-running `renderItem` and producing a brand-new node/DOM identity for an
> entry that was supposed to be reused untouched. Wrapping the `renderItem`
> call itself in `untrack()` fixes this: the wrapping effect ends up with
> zero tracked dependencies (so it never self-triggers), while any *nested*
> `effect()` call renderItem makes on its own behalf (`insert`, `setAttr`,
> `toggleClass`, or a hand-written `effect()` like the probe below) is
> unaffected — a nested `effect()` always establishes its own tracking scope
> regardless of an ambient `untrack()`, because `currentOwner` (ownership,
> in `effect.ts`) and `currentConsumer` (dependency tracking, in `graph.ts`)
> are independent mechanisms; `untrack()` only suspends the latter. This is
> also the *correct* alignment with §2.4's own documented contract (a plain
> top-level read inside `renderItem` is a one-time read, not a reactive
> binding) — the original code accidentally violated that contract by
> making the wrapping effect react to it. The "disposes a removed item's
> effect scope" test's own probe effect was also fixed alongside this (see
> its updated code above): it previously called `item()` reactively inside
> its own body, which — correctly, per normal effect semantics — fires its
> cleanup on every value change, not only on final disposal, conflating
> "reused with an updated value" with "permanently torn down." The fixed
> probe reads nothing reactive, so its cleanup fires only when its owning
> per-item effect is actually disposed.
>
> **Third correction (discovered during Task 8, via the playground demo running
> in a real browser):** the *initial* `reconcile()` call's `itemsView.get()`
> read (first line of `reconcile()`) was never wrapped in `untrack()` — every
> other read in this file got the treatment (the per-item `renderItem` call
> above), but this one was missed. Since `reconcile()`'s first call runs
> directly at `mapArray()`'s own top level (not inside any effect `mapArray`
> creates itself), whatever `effect()` happens to be synchronously executing
> `mapArray()`'s caller — in practice, `mount()`'s wrapping effect, whenever a
> component calls `mapArray()` during its own construction — becomes the
> ambient `currentConsumer` at the moment `itemsView.get()` returns, and gets
> spuriously registered as `itemsView`'s dependent. Any later signal write
> that changes the list then incorrectly re-runs `mount()`'s *entire* effect:
> it disposes every nested binding from the first render (including
> `mapArray`'s own per-item effects) and reconstructs a second, disconnected
> component instance that is never attached to the page — the original DOM
> goes permanently inert. This was invisible to every prior unit test because
> none of them exercise a `mapArray`-using component through `mount()`'s
> wrapping effect while also mutating the source signal afterward (`list.test.ts`
> calls `mapArray()` directly with no `mount()`; `mount.test.ts` mounts
> components that don't use `mapArray()`) — it only surfaced once the
> playground demo (Task 7, `TodoList`, mounted via `mount()`) was driven in an
> actual browser (Task 8). Fixed by wrapping the read:
> `const currentItems = untrack(() => itemsView.get());` — the same treatment
> already given to every other read in this file, just missed on this one
> line. A dedicated regression test (mounting a `mapArray`-using component via
> `mount()`, then mutating its source signal) was added to `list.test.ts`
> to close the gap that let this ship unnoticed.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: PASS — 6 tests in `list.test.ts` (24 total).

- [ ] **Step 5: Commit**

```bash
git add packages/runtime-dom/src/list.ts packages/runtime-dom/test/list.test.ts
git commit -m "Add mapArray() keyed list reconciliation"
```

---

## Task 5: `mount()`

**Files:**
- Create: `packages/runtime-dom/src/mount.ts`
- Test: `packages/runtime-dom/test/mount.test.ts`

**Interfaces:**
- Consumes: `effect` from `@tez/signals`; `insert` from `../src/bindings` (test only).
- Produces: `mount<P>(Component: (props: P) => Node, target: Element, props?: P): () => void`. Task 6 (playground `main.ts`) calls this to mount the counter and todo-list components.

- [ ] **Step 1: Write the failing tests**

`packages/runtime-dom/test/mount.test.ts`:
```ts
import { describe, expect, it, vi } from "vitest";
import { signal } from "@tez/signals";
import { mount } from "../src/mount";
import { insert } from "../src/bindings";

describe("mount", () => {
  it("calls the component once and appends its returned node to the target", () => {
    const componentFn = vi.fn(() => document.createElement("div"));
    const target = document.createElement("main");
    mount(componentFn, target);
    expect(componentFn).toHaveBeenCalledOnce();
    expect(target.children.length).toBe(1);
  });

  it("passes props through to the component", () => {
    const target = document.createElement("main");
    mount(
      (props: { label: string }) => {
        const el = document.createElement("span");
        el.textContent = props.label;
        return el;
      },
      target,
      { label: "hi" },
    );
    expect(target.textContent).toBe("hi");
  });

  it("supports nested reactive bindings created during the component call", () => {
    const count = signal(1);
    const target = document.createElement("main");
    mount(() => {
      const el = document.createElement("span");
      const marker = document.createComment("");
      el.appendChild(marker);
      insert(el, () => count.get(), marker);
      return el;
    }, target);
    expect(target.textContent).toBe("1");
    count.set(2);
    expect(target.textContent).toBe("2");
  });

  it("dispose() removes the mounted node from the DOM", () => {
    const target = document.createElement("main");
    const dispose = mount(() => document.createElement("div"), target);
    expect(target.children.length).toBe(1);
    dispose();
    expect(target.children.length).toBe(0);
  });

  it("dispose() tears down nested bindings created inside the component", () => {
    const count = signal(1);
    const removeObserverSpy = vi.spyOn(count, "removeObserver");
    const target = document.createElement("main");
    const dispose = mount(() => {
      const el = document.createElement("span");
      const marker = document.createComment("");
      el.appendChild(marker);
      insert(el, () => count.get(), marker);
      return el;
    }, target);
    dispose();
    expect(removeObserverSpy).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: FAIL — `Cannot find module '../src/mount'`.

- [ ] **Step 3: Implement `mount.ts`**

`packages/runtime-dom/src/mount.ts`:
```ts
import { effect } from "@tez/signals";

export function mount<P>(
  Component: (props: P) => Node,
  target: Element,
  props?: P,
): () => void {
  let node!: Node;

  // Wrapping in effect() is purely for its owner-scope disposal mechanics,
  // not because the component call is meant to be reactive: insert()/
  // setAttr()/toggleClass()/mapArray() calls made during Component(props)
  // create their own nested effects, which need a parent scope to be
  // disposed under when dispose() below is called. Well-formed components
  // don't call .get() directly outside those helpers, so in practice this
  // effect has no tracked dependencies and never re-runs.
  const dispose = effect(() => {
    node = Component(props as P);
  });

  target.appendChild(node);

  return () => {
    dispose();
    node.remove();
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: PASS — 5 tests in `mount.test.ts` (29 total).

- [ ] **Step 5: Commit**

```bash
git add packages/runtime-dom/src/mount.ts packages/runtime-dom/test/mount.test.ts
git commit -m "Add mount()"
```

---

## Task 6: Public API surface + integration test

**Files:**
- Modify: `packages/runtime-dom/src/index.ts`
- Test: `packages/runtime-dom/test/index.test.ts`

**Interfaces:**
- Consumes: `template`, `insert`/`setAttr`/`toggleClass`, `listen`, `mapArray`, `mount` from their respective modules.
- Produces: the single public entry point `@tez/runtime-dom` re-exporting all five primitives. This is the only surface `apps/playground` (Task 7) may import from.

- [ ] **Step 1: Write the failing integration test**

`packages/runtime-dom/test/index.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { signal } from "@tez/signals";
import { template, insert, setAttr, toggleClass, listen, mapArray, mount } from "../src/index";

describe("public API", () => {
  it("re-exports every primitive from a single entry point", () => {
    expect(typeof template).toBe("function");
    expect(typeof insert).toBe("function");
    expect(typeof setAttr).toBe("function");
    expect(typeof toggleClass).toBe("function");
    expect(typeof listen).toBe("function");
    expect(typeof mapArray).toBe("function");
    expect(typeof mount).toBe("function");
  });

  it("composes template + insert + listen + mount into a working counter", () => {
    const makeCounter = template("<button>Count: <!></button>");

    function Counter() {
      const count = signal(0);
      const el = makeCounter() as HTMLButtonElement;
      const marker = el.childNodes[1] as Node;
      insert(el, () => count.get(), marker);
      listen(el, "click", () => count.set(count.get() + 1));
      return el;
    }

    const target = document.createElement("main");
    document.body.appendChild(target);
    mount(Counter, target);

    const button = target.querySelector("button")!;
    expect(button.textContent).toBe("Count: 0");
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(button.textContent).toBe("Count: 1");
    target.remove();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: FAIL — `template`/etc. are `undefined` (only re-exported so far is `template`, per Task 1 Step 8).

- [ ] **Step 3: Update `index.ts` to re-export everything**

`packages/runtime-dom/src/index.ts`:
```ts
export { template } from "./template";
export { insert, setAttr, toggleClass } from "./bindings";
export { listen } from "./events";
export { mapArray } from "./list";
export { mount } from "./mount";
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/runtime-dom test`
Expected: PASS — 2 tests in `index.test.ts` (31 total across the package).

- [ ] **Step 5: Run typecheck**

Run: `pnpm --filter @tez/runtime-dom typecheck`
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add packages/runtime-dom/src/index.ts packages/runtime-dom/test/index.test.ts
git commit -m "Re-export all runtime-dom primitives from index.ts"
```

---

## Task 7: Playground demo (counter + keyed todo list)

**Files:**
- Modify: `apps/playground/package.json`
- Create: `apps/playground/vite.config.ts`
- Create: `apps/playground/index.html`
- Create: `apps/playground/src/counter.ts`
- Create: `apps/playground/src/todo-list.ts`
- Create: `apps/playground/src/main.ts`

**Interfaces:**
- Consumes: `template`, `insert`, `setAttr`, `toggleClass`, `listen`, `mapArray`, `mount` from `@tez/runtime-dom`; `signal`, `effect` from `@tez/signals`.
- Produces: a real, servable playground app at `apps/playground`. Task 8 (Playwright) drives this app in a browser.

- [ ] **Step 1: Update the playground package manifest**

`apps/playground/package.json`:
```json
{
  "name": "playground",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@tez/runtime-dom": "workspace:*",
    "@tez/signals": "workspace:*"
  },
  "devDependencies": {
    "vite": "^6.0.0"
  }
}
```

- [ ] **Step 2: Add a minimal Vite config**

`apps/playground/vite.config.ts`:
```ts
import { defineConfig } from "vite";

export default defineConfig({
  server: {
    // Allow serving files from workspace-linked packages outside this app's
    // own directory (packages/runtime-dom, packages/signals).
    fs: {
      allow: [".."],
    },
  },
});
```

- [ ] **Step 3: Add the HTML entry point**

`apps/playground/index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>Tez Playground</title>
  </head>
  <body>
    <div id="counter-demo"></div>
    <div id="todo-demo"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 4: Write the counter component**

`apps/playground/src/counter.ts`:
```ts
import { signal } from "@tez/signals";
import { template, insert, listen } from "@tez/runtime-dom";

const makeCounterTemplate = template(`<button>Count: <!></button>`);

export function Counter() {
  const count = signal(0);
  const el = makeCounterTemplate() as HTMLButtonElement;
  const marker = el.childNodes[1] as Node;

  insert(el, () => count.get(), marker);
  listen(el, "click", () => count.set(count.get() + 1));

  return el;
}
```

- [ ] **Step 5: Write the todo-list component**

`apps/playground/src/todo-list.ts`:
```ts
import { signal, effect } from "@tez/signals";
import { template, insert, setAttr, toggleClass, listen, mapArray } from "@tez/runtime-dom";

interface Todo {
  id: number;
  text: string;
  done: boolean;
}

let nextId = 1;

const makeRootTemplate = template(`<div><ul></ul><button>Add</button></div>`);
const makeItemTemplate = template(
  `<li><input type="checkbox" /><span><!></span><button>Remove</button></li>`,
);

export function TodoList() {
  const todos = signal<Todo[]>([
    { id: nextId++, text: "Write the compiler", done: false },
    { id: nextId++, text: "Ship Phase 1", done: false },
  ]);

  const root = makeRootTemplate() as HTMLDivElement;
  const list = root.querySelector("ul")!;
  const addButton = root.querySelector("button")!;

  const getNodes = mapArray(
    () => todos.get(),
    (todo) => todo.id,
    (todo) => {
      const item = makeItemTemplate() as HTMLLIElement;
      const checkbox = item.querySelector("input")!;
      const label = item.querySelector("span")!;
      const marker = label.childNodes[0] as Node;
      const removeButton = item.querySelectorAll("button")[0]!;

      insert(label, () => todo().text, marker);
      toggleClass(item, "done", () => todo().done);
      setAttr(removeButton, "aria-label", () => `Remove ${todo().text}`);

      // checkbox.checked must be set as a DOM property, not an attribute:
      // once a user has interacted with a checkbox, browsers stop syncing
      // its live checked state from the "checked" attribute.
      effect(() => {
        checkbox.checked = todo().done;
      });

      listen(checkbox, "change", () => {
        const id = todo().id;
        todos.set(todos.get().map((t) => (t.id === id ? { ...t, done: !t.done } : t)));
      });

      listen(removeButton, "click", () => {
        const id = todo().id;
        todos.set(todos.get().filter((t) => t.id !== id));
      });

      return item;
    },
  );

  effect(() => {
    list.replaceChildren(...getNodes());
  });

  listen(addButton, "click", () => {
    const id = nextId++;
    todos.set([...todos.get(), { id, text: `New todo ${id}`, done: false }]);
  });

  return root;
}
```

- [ ] **Step 6: Write the entry point that mounts both demos**

`apps/playground/src/main.ts`:
```ts
import { mount } from "@tez/runtime-dom";
import { Counter } from "./counter";
import { TodoList } from "./todo-list";

mount(Counter, document.getElementById("counter-demo")!);
mount(TodoList, document.getElementById("todo-demo")!);
```

- [ ] **Step 7: Install dependencies**

Run: `pnpm install`
Expected: resolves `vite`, `@tez/runtime-dom`, `@tez/signals` with no errors.

- [ ] **Step 8: Verify the app builds cleanly**

Run: `pnpm --filter playground build`
Expected: Vite build completes with no TypeScript or bundling errors, producing `apps/playground/dist/`.

- [ ] **Step 9: Commit**

```bash
git add apps/playground
git commit -m "Add playground counter + todo-list demo"
```

---

## Task 8: Playwright browser verification

**Files:**
- Modify: `pnpm-workspace.yaml`
- Create: `e2e/package.json`
- Create: `e2e/playwright.config.ts`
- Create: `e2e/tests/playground.spec.ts`

**Interfaces:**
- Consumes: the running playground dev server (`pnpm --filter playground dev`), started automatically by Playwright's `webServer` config.
- Produces: an automated real-browser verification of Task 7's demo. This is the final task of this cycle — no later task depends on it.

- [ ] **Step 1: Add `e2e` as a workspace member**

`pnpm-workspace.yaml`:
```yaml
packages:
  - "packages/*"
  - "apps/*"
  - "e2e"
```

- [ ] **Step 2: Add the e2e package manifest**

`e2e/package.json`:
```json
{
  "name": "e2e",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "test": "playwright test"
  },
  "devDependencies": {
    "@playwright/test": "^1.49.0"
  }
}
```

- [ ] **Step 3: Add the Playwright config**

`e2e/playwright.config.ts`:
```ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  webServer: {
    command: "pnpm --filter playground dev --port 4173",
    url: "http://localhost:4173",
    reuseExistingServer: !process.env.CI,
  },
  use: {
    baseURL: "http://localhost:4173",
  },
});
```

> **Correction (discovered during Task 8):** the `command` has no `--`
> separator before `--port 4173`. `apps/playground`'s `dev` script is bare
> `"vite"`; pnpm forwards args after `--` to the underlying script, so
> `pnpm --filter playground dev -- --port 4173` would invoke `vite -- --port
> 4173` — a redundant `--` that Vite's CLI treats as an end-of-options
> marker, silently starting on its default port 5173 instead of 4173 and
> causing Playwright's `webServer.url` readiness check to time out.

- [ ] **Step 4: Install dependencies and the Playwright browser binary**

Run: `pnpm install`
Expected: resolves `@playwright/test` with no errors.

Run: `pnpm --filter e2e exec playwright install --with-deps chromium`
Expected: downloads and installs a Chromium build for Playwright (may take a minute; requires network access).

- [ ] **Step 5: Write the browser test**

`e2e/tests/playground.spec.ts`:
```ts
import { test, expect } from "@playwright/test";

test.describe("playground demo", () => {
  test("counter increments when clicked", async ({ page }) => {
    await page.goto("/");
    const button = page.locator("#counter-demo button");
    await expect(button).toHaveText("Count: 0");
    await button.click();
    await expect(button).toHaveText("Count: 1");
    await button.click();
    await expect(button).toHaveText("Count: 2");
  });

  test("todo list: toggle, add, and remove items", async ({ page }) => {
    await page.goto("/");
    const list = page.locator("#todo-demo ul");
    await expect(list.locator("li")).toHaveCount(2);

    const firstItem = list.locator("li").first();
    await firstItem.locator("input[type=checkbox]").check();
    await expect(firstItem).toHaveClass(/done/);

    await page.locator("#todo-demo button", { hasText: "Add" }).click();
    await expect(list.locator("li")).toHaveCount(3);

    const secondItemText = await list.locator("li").nth(1).locator("span").textContent();
    await list.locator("li").nth(1).locator("button", { hasText: "Remove" }).click();
    await expect(list.locator("li")).toHaveCount(2);
    const remainingTexts = await list.locator("li span").allTextContents();
    expect(remainingTexts).not.toContain(secondItemText);
  });

  test("toggling an item preserves the list container's DOM node identity (keyed reconciliation, not a full re-render)", async ({
    page,
  }) => {
    await page.goto("/");
    const list = page.locator("#todo-demo ul");
    const before = await list.elementHandle();
    await list.locator("li").first().locator("input[type=checkbox]").check();
    const after = await list.elementHandle();
    const sameNode = await page.evaluate(([a, b]) => a === b, [before, after]);
    expect(sameNode).toBe(true);
  });
});
```

- [ ] **Step 6: Run the Playwright suite**

Run: `pnpm --filter e2e test`
Expected: PASS — 3 tests, Playwright starts the playground dev server automatically, runs against Chromium, and tears the server down afterward.

- [ ] **Step 7: Commit**

```bash
git add pnpm-workspace.yaml e2e
git commit -m "Add Playwright browser verification for the playground demo"
```

---

## Cycle Gate Checklist

- [ ] `template`, `insert`, `setAttr`, `toggleClass`, `listen`, `mapArray`, `mount` implemented and unit-tested (Tasks 1–6).
- [ ] All reactive DOM writes happen inside `effect()` — no component re-execution, no VDOM (Tasks 2, 4, 5).
- [ ] `mapArray` reuses DOM nodes for persisting keys and disposes removed items' effect scopes (Task 4).
- [ ] Hand-written counter + todo-list demo runs in `apps/playground`, mounted via `mount()` (Task 7).
- [ ] Real-browser Playwright verification passes: click interactions, keyed list add/toggle/remove, DOM node identity preserved across updates (Task 8).

# DOM Runtime Primitives — Design (Phase 1, Cycle 1 of 4)

> **Status:** Approved by user 2026-07-06. Scope: `packages/runtime-dom` reactive DOM primitives only.
> **Precedes:** Phase 1 cycles 2–4 (compiler MVP, control-flow components, gate demonstration — see decomposition note below).
> **Depends on:** `packages/signals` (Phase 0, implemented on branch `worktree-phase0-signals`, PR open, not yet merged into `main` — this branch stacks on that branch's tip; rebase onto `main` once that PR merges).

---

## 0. Why this is its own cycle

The architecture spec's Phase 1 ("Compiler MVP + DOM runtime", §8) bundles several independent subsystems: a Rust/oxc compiler with napi-rs bindings, a TypeScript DOM runtime, control-flow components (`Show`/`For`/`Switch`/`Dynamic`), and a TodoMVC + js-framework-benchmark gate demonstration. These were decomposed into four ordered sub-cycles, each with its own brainstorm → spec → plan → implementation pass:

1. **DOM runtime primitives** (this document) — pure TypeScript, no compiler dependency.
2. Compiler MVP (Rust/oxc + napi-rs) — JSX parse, reactivity analysis, DOM codegen targeting cycle 1's runtime, error codes TEZ101–103.
3. Control-flow components (`Show`/`For`/`Switch`/`Dynamic`) — built on cycle 1's runtime primitives (`mapArray` in particular); compiler enforcement of the "static JSX tag" rule ties into cycle 2.
4. Gate demonstration — TodoMVC in `apps/playground` + js-framework-benchmark keyed suite, plus the ≤4 KB gzip `runtime-dom` budget CI gate (deferred here, wired up in this cycle).

This document covers cycle 1 only.

---

## 1. Package structure

`packages/runtime-dom/src/`, five single-purpose files, importing only from `packages/signals` (repo layout rule, §1 of the main spec):

- `template.ts` — cached `<template>` cloning.
- `bindings.ts` — reactive text/attribute/class bindings.
- `events.ts` — delegated event listening.
- `list.ts` — keyed list reconciliation.
- `mount.ts` — top-level component instantiation + disposal.
- `index.ts` — public re-exports.

---

## 2. API surface

### 2.1 `template.ts`

```ts
function template(html: string): () => Node;
```

Each call creates one `<template>` element (set once via `innerHTML = html`) and returns a factory function closing over it; the factory clones `templateEl.content.firstChild` on every invocation. No internal cache is needed: callers are expected to invoke `template(html)` once per component definition (assigned to a module-level `const`, as in the compiler's own codegen contract, e.g. `const _t1 = template(...)`), so the "create once, clone many times" property falls out of ordinary JS module evaluation rather than requiring a lookup structure. Assumes exactly one root element in `html` for this cycle — multi-root/fragment templates are not needed until the compiler (cycle 2) emits them, if ever.

### 2.2 `bindings.ts`

```ts
function insert(parent: Node, accessor: () => unknown, marker: Node): void;
function setAttr(el: Element, name: string, accessor: () => unknown): void;
function toggleClass(el: Element, className: string, accessor: () => boolean): void;
```

- `insert`: wraps `effect(() => {...})`. Reads `accessor()`; if the value differs from the last-rendered value (primitive string/number, `null`/`undefined`, or a single `Node`), updates the DOM immediately before `marker` (a comment placeholder node, matching the compiler contract's `<!>` convention). Arrays of children are explicitly out of scope for `insert` — `mapArray` (2.4) owns list rendering and is responsible for its own DOM insertion.
- `setAttr`: wraps `effect(() => {...})`. Reads `accessor()`; `null`/`false`/`undefined` removes the attribute (`el.removeAttribute(name)`), otherwise `el.setAttribute(name, String(value))`.
- `toggleClass`: wraps `effect(() => {...})`. Reads `accessor()`; calls `el.classList.toggle(className, Boolean(value))`.

Each of these creates its own `effect()` — disposal is automatic once the owning scope (ultimately `mount()`'s root effect, or a `mapArray` item's nested effect) disposes.

### 2.3 `events.ts`

```ts
function listen(el: Element, eventName: string, handler: (ev: Event) => void): void;
```

Sets `(el as any)[`$$${eventName}`] = handler` and, if no delegated listener for `eventName` has been registered yet (tracked in a module-level `Set<string>`), adds one `document.addEventListener(eventName, dispatch)` where `dispatch` walks up from `event.target` via ancestor traversal (not `Element.closest`, since the property isn't a CSS-selectable attribute — a manual `while (node) { if (node[$$prop]) {...}; node = node.parentNode; }` loop) looking for the first ancestor carrying `$$<eventName>`, and invokes it with `(event, node)`.

No hardcoded event list — any event name works via this convention, registered lazily on first `listen()` call for that name.

### 2.4 `list.ts`

```ts
function mapArray<T, U extends Node>(
  items: () => T[],
  keyFn: (item: T) => unknown,
  renderItem: (item: () => T, index: () => number) => U,
): () => U[];
```

Wrapped in its own `effect()` that tracks `items()`. Maintains a `Map<unknown, { itemSignal: State<T>; indexSignal: State<number>; node: U; dispose: () => void }>` keyed by `keyFn(item)`. On each run (initial or reactive):

1. Compute the new key list from `items()`.
2. For keys present in the map but absent from the new list: call that entry's `dispose()` (disposes its nested effect scope) and remove it from the map.
3. For keys present in both: `itemSignal.set(newItem)` and `indexSignal.set(newIndex)` (updates the existing node's bindings in place — no re-render, no new DOM node).
4. For keys new to the map: create `itemSignal = signal(item)`, `indexSignal = signal(index)`, call `renderItem(() => itemSignal.get(), () => indexSignal.get())` inside a nested `effect()` (auto-parented as a child of the outer `mapArray` effect, so it disposes automatically when the list itself is torn down or an item is removed), store the resulting node + that nested effect's dispose function.
5. Return the `U[]` in the new order. The caller (e.g. the playground demo) is responsible for reconciling actual DOM child order from this array (e.g. via repeated `insertBefore`/`Node.before` calls, or a `replaceChildren(...nodes)` full pass — no optimized "minimal DOM move" algorithm required this cycle; only the *identity* of nodes must be preserved across re-renders, not the specific reordering algorithm).

This is genuine engine groundwork for `<For>` (cycle 3), not demo-only throwaway code.

### 2.5 `mount.ts`

```ts
function mount<P>(Component: (props: P) => Node, target: Element, props?: P): () => void;
```

```ts
function mount(Component, target, props) {
  let node;
  const dispose = effect(() => {
    node = Component(props);
  });
  target.appendChild(node);
  return () => {
    dispose();
    node.remove();
  };
}
```

Wraps the one-time `Component(props)` call in `effect()` purely for its owner-scope disposal side effect — `insert`/`setAttr`/`toggleClass`/`mapArray` calls made during `Component`'s execution create their own nested effects, which get parented under this root effect and all dispose together when `mount()`'s returned function is called.

**Known simplification, documented deliberately:** this root effect *would* incorrectly re-run the whole `Component(props)` call if the component ever read a signal's `.get()` directly at its top level outside of the sanctioned binding helpers (`insert`/`setAttr`/`toggleClass`/`mapArray`/`listen` handlers) — violating the "components run once" invariant. Well-formed Tez components don't do this by convention (they create new signals from props, e.g. `signal(props.start)`, and only read existing signals through the binding helpers), so in practice this root effect has no real reactive dependencies and never re-runs. This is a deliberate reuse of Phase 0's existing, already-tested `effect()` machinery rather than adding a new non-reactive "root" primitive to `packages/signals` in this cycle. If this convention proves too fragile once the compiler (cycle 2) starts enforcing/generating component bodies, revisit with a real `createRoot()`-style primitive as a `packages/signals` spec amendment.

---

## 3. Demo

`apps/playground` gets two hand-written components — written in the shape the compiler will eventually emit (direct primitive calls, no JSX):

- **Counter**: a `<button>` + reactive text span, proving `template` + `insert` + `listen`.
- **Todo list**: add / toggle-done / remove, proving `mapArray` + `setAttr`/`toggleClass` + keyed reordering (removing a middle item must not rebuild the other items' DOM nodes).

Both mounted via `mount()` into the playground's HTML shell.

---

## 4. Testing

- **Unit tests** (vitest + happy-dom, matching `packages/signals`' existing setup): one test file per module — `template.test.ts`, `bindings.test.ts`, `events.test.ts`, `list.test.ts`, `mount.test.ts`.
- **Browser verification** (Playwright, using the existing `e2e/` placeholder): a single spec driving the playground demo in a real browser — click the counter, add/toggle/remove todos, assert on rendered DOM state. Satisfies the project rule to verify frontend changes in an actual browser, not simulated DOM only.

---

## 5. Explicitly out of scope this cycle

- SSR, resumability (`restoreSignal`, QRL chunk extraction/loading) — Phase 2.
- The compiler itself — cycle 2.
- Public control-flow components `<Show>`/`<For>`/`<Switch>`/`<Dynamic>` — cycle 3 (wraps `mapArray` etc.; this cycle only builds the underlying primitive).
- TEZ10x compiler error codes — cycle 2.
- The ≤4 KB gzip `runtime-dom` budget CI gate — deferred to cycle 4, once there's a real bundler/build step to measure against.
- Any change to `packages/signals`' public API (e.g. a `createRoot()` primitive) — noted as a possible future revisit in §2.5, not undertaken now.

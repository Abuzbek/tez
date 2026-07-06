# Tez Phase 0 — Monorepo Scaffold + Signals Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the Tez monorepo skeleton (pnpm + turborepo, all packages/apps per the architecture spec) and fully implement `packages/signals` — a zero-dependency, TC39-signals-shaped reactive core — to its Phase 0 gate.

**Architecture:** A push-then-pull reactive graph. `Signal.State` (leaf) and `Signal.Computed` (derived) both implement a `Source`/`Observer` interface. Writes eagerly mark the transitive observer graph `stale` (once per node, so no exponential re-marking); reads lazily recompute only what's actually stale. `Signal.subtle.Watcher` is a low-level notify-once-then-must-rearm primitive; `effect()` is sugar built on top of a `Computed` + a `Watcher`, with nested effects auto-disposed via an implicit owner stack. `batch()` coalesces effect flushes; `untrack()` suspends dependency collection.

**Tech Stack:** TypeScript 5, pnpm workspaces, Turborepo, Vitest + `@vitest/coverage-v8`. `packages/signals` has zero runtime dependencies (dev tooling only).

## Global Constraints

- `packages/signals` must have zero runtime dependencies (spec §1).
- `packages/signals` internals must be TC39-Signals-shaped: `Signal.State`, `Signal.Computed`, `Signal.subtle.Watcher` (spec §2.2).
- Phase 0 gate (spec §8): ≥99% branch coverage on `packages/signals`, passes an adapted subset of the TC39 signal-polyfill test suite plus adapted Solid reactivity test cases.
- `runtime-dom` may import only from `signals` (spec §1) — not built this cycle, but stub must not violate this later.
- `compiler` never imports runtime code (spec §1) — stub only this cycle.
- Circular package deps are a CI failure (spec §1) — n/a until more packages have real code, but keep import graph acyclic within `signals` itself.
- Per §12 of the design spec, this cycle covers: full monorepo skeleton (stub packages/apps) + full `packages/signals` implementation. All other packages stay stubs.

---

## Task 1: Root monorepo scaffold

**Files:**
- Create: `pnpm-workspace.yaml`
- Create: `package.json`
- Create: `turbo.json`
- Create: `tsconfig.base.json`
- Create: `.gitignore`

**Interfaces:**
- Produces: root workspace covering `packages/*` and `apps/*`; root scripts `build`, `test`, `test:coverage`, `typecheck`, `lint` fan out via turbo; `tsconfig.base.json` is extended by every package's own `tsconfig.json`.

- [ ] **Step 1: Create `pnpm-workspace.yaml`**

```yaml
packages:
  - "packages/*"
  - "apps/*"
```

- [ ] **Step 2: Create root `package.json`**

```json
{
  "name": "tez",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "packageManager": "pnpm@10.32.1",
  "engines": {
    "node": ">=20"
  },
  "scripts": {
    "build": "turbo run build",
    "test": "turbo run test",
    "test:coverage": "turbo run test:coverage",
    "typecheck": "turbo run typecheck",
    "lint": "turbo run lint"
  },
  "devDependencies": {
    "turbo": "^2.3.0",
    "typescript": "^5.6.0"
  }
}
```

- [ ] **Step 3: Create `turbo.json`**

```json
{
  "$schema": "https://turbo.build/schema.json",
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**"]
    },
    "test": {
      "dependsOn": ["^build"],
      "outputs": []
    },
    "test:coverage": {
      "dependsOn": ["^build"],
      "outputs": ["coverage/**"]
    },
    "typecheck": {
      "dependsOn": ["^build"],
      "outputs": []
    },
    "lint": {
      "outputs": []
    }
  }
}
```

- [ ] **Step 4: Create `tsconfig.base.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "isolatedModules": true,
    "noUncheckedIndexedAccess": true
  }
}
```

- [ ] **Step 5: Create `.gitignore`**

```
node_modules/
dist/
coverage/
.turbo/
*.log
.DS_Store
```

- [ ] **Step 6: Verify pnpm recognizes the workspace**

Run: `pnpm install`
Expected: completes with no error (zero packages matched yet is fine — `packages/` and `apps/` don't exist until Task 2).

- [ ] **Step 7: Commit**

```bash
git add pnpm-workspace.yaml package.json turbo.json tsconfig.base.json .gitignore
git commit -m "Scaffold root pnpm/turborepo workspace"
```

---

## Task 2: Stub packages and apps

**Files:**
- Create: `packages/compiler/package.json`, `packages/compiler/README.md`
- Create: `packages/runtime-dom/package.json`, `packages/runtime-dom/src/index.ts`, `packages/runtime-dom/tsconfig.json`
- Create: `packages/runtime-server/package.json`, `packages/runtime-server/src/index.ts`, `packages/runtime-server/tsconfig.json`
- Create: `packages/server/package.json`, `packages/server/src/index.ts`, `packages/server/tsconfig.json`
- Create: `packages/vite-plugin/package.json`, `packages/vite-plugin/src/index.ts`, `packages/vite-plugin/tsconfig.json`
- Create: `packages/interop/package.json`, `packages/interop/src/index.ts`, `packages/interop/tsconfig.json`
- Create: `packages/create-tez/package.json`, `packages/create-tez/src/index.ts`, `packages/create-tez/tsconfig.json`
- Create: `packages/devtools/package.json`, `packages/devtools/README.md`
- Create: `apps/playground/package.json`, `apps/playground/README.md`
- Create: `apps/docs/package.json`, `apps/docs/README.md`
- Create: `apps/bench/package.json`, `apps/bench/README.md`
- Create: `e2e/README.md`
- Create: `rfcs/README.md`

**Interfaces:**
- Produces: every package name referenced in spec §1 exists as an installable workspace member (`@tez/<name>`), so Task 3+ can add `"@tez/signals": "workspace:*"` as a dependency from any of them later without restructuring.

`packages/compiler` is a future Rust/napi-rs crate (spec §3), not a TS package — it gets a `package.json` (so `pnpm -F @tez/compiler` resolves) plus a `README.md` noting its future shape, no `src/`.

`packages/devtools` (phase 4, browser extension) and the three `apps/*` get the same package.json + README treatment — no runtime code belongs in them yet.

The five TS-runtime stub packages (`runtime-dom`, `runtime-server`, `server`, `vite-plugin`, `interop`, `create-tez`) get a real (empty-ish) `src/index.ts` and `tsconfig.json` so `typecheck`/`build` turbo tasks have something valid to run against.

- [ ] **Step 1: `packages/compiler` stub**

`packages/compiler/package.json`:
```json
{
  "name": "@tez/compiler",
  "version": "0.0.0",
  "private": true,
  "description": "Rust/oxc-based compiler, exposed to JS via napi-rs. Not yet implemented (Phase 1)."
}
```

`packages/compiler/README.md`:
```markdown
# @tez/compiler

Phase 1 deliverable. Rust crate (oxc-based parser/semantic analysis), exposed to
the Vite plugin via napi-rs bindings. See `docs/superpowers/specs/2026-07-05-tez-architecture-design.md` §3.

Not yet implemented.
```

- [ ] **Step 2: `packages/runtime-dom` stub**

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
    "test": "echo 'no tests yet'",
    "test:coverage": "echo 'no tests yet'",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {}
}
```

`packages/runtime-dom/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

`packages/runtime-dom/src/index.ts`:
```ts
// Client runtime: signals, effects, template cloning, event delegation,
// resume-loader. Phase 1 deliverable — see spec §1 and §3.2.
export {};
```

- [ ] **Step 3: `packages/runtime-server` stub**

`packages/runtime-server/package.json`:
```json
{
  "name": "@tez/runtime-server",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "build": "echo 'no build yet'",
    "test": "echo 'no tests yet'",
    "test:coverage": "echo 'no tests yet'",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {}
}
```

`packages/runtime-server/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

`packages/runtime-server/src/index.ts`:
```ts
// SSR string-stream renderer + signal serializer. Phase 2 deliverable —
// see spec §3.2 and §4.3.
export {};
```

- [ ] **Step 4: `packages/server` stub**

`packages/server/package.json`:
```json
{
  "name": "@tez/server",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "build": "echo 'no build yet'",
    "test": "echo 'no tests yet'",
    "test:coverage": "echo 'no tests yet'",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {}
}
```

`packages/server/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

`packages/server/src/index.ts`:
```ts
// Nitro integration: routing, routeRules, ISR/SWR cache, server$ RPC,
// revalidateTag. Phase 3 deliverable — see spec §5.
export {};
```

- [ ] **Step 5: `packages/vite-plugin` stub**

`packages/vite-plugin/package.json`:
```json
{
  "name": "@tez/vite-plugin",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "build": "echo 'no build yet'",
    "test": "echo 'no tests yet'",
    "test:coverage": "echo 'no tests yet'",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {}
}
```

`packages/vite-plugin/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

`packages/vite-plugin/src/index.ts`:
```ts
// Dev server, HMR, build orchestration on Vite 6+ Environment API.
// Phase 1+ deliverable — see spec §1.
export {};
```

- [ ] **Step 6: `packages/interop` stub**

`packages/interop/package.json`:
```json
{
  "name": "@tez/interop",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "build": "echo 'no build yet'",
    "test": "echo 'no tests yet'",
    "test:coverage": "echo 'no tests yet'",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {}
}
```

`packages/interop/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

`packages/interop/src/index.ts`:
```ts
// customElement(), fromReact(), fromVue(), mount(). Phase 4 deliverable —
// see spec §6.
export {};
```

- [ ] **Step 7: `packages/create-tez` stub**

`packages/create-tez/package.json`:
```json
{
  "name": "create-tez",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "bin": {
    "create-tez": "./src/index.ts"
  },
  "scripts": {
    "build": "echo 'no build yet'",
    "test": "echo 'no tests yet'",
    "test:coverage": "echo 'no tests yet'",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {}
}
```

`packages/create-tez/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

`packages/create-tez/src/index.ts`:
```ts
// Scaffolding CLI. Phase 4 deliverable — see spec §8.
export {};
```

- [ ] **Step 8: `packages/devtools` stub**

`packages/devtools/package.json`:
```json
{
  "name": "@tez/devtools",
  "version": "0.0.0",
  "private": true,
  "description": "Browser extension: signal graph inspector. Not yet implemented (Phase 4)."
}
```

`packages/devtools/README.md`:
```markdown
# @tez/devtools

Phase 4 deliverable. Browser extension exposing a signal graph inspector.
See `docs/superpowers/specs/2026-07-05-tez-architecture-design.md` §1.

Not yet implemented.
```

- [ ] **Step 9: `apps/playground`, `apps/docs`, `apps/bench` stubs**

`apps/playground/package.json`:
```json
{
  "name": "playground",
  "version": "0.0.0",
  "private": true,
  "description": "Kitchen-sink app exercising every Tez feature. Not yet implemented (Phase 1+)."
}
```

`apps/playground/README.md`:
```markdown
# playground

Kitchen-sink app exercising every Tez feature as phases land.
See `docs/superpowers/specs/2026-07-05-tez-architecture-design.md` §1.

Not yet implemented.
```

`apps/docs/package.json`:
```json
{
  "name": "docs",
  "version": "0.0.0",
  "private": true,
  "description": "Docs site, built with Tez itself. Not yet implemented (dogfood gate, Phase 4)."
}
```

`apps/docs/README.md`:
```markdown
# docs

Docs site, to be built with Tez itself once Phase 4's dogfood gate is reached.
See `docs/superpowers/specs/2026-07-05-tez-architecture-design.md` §8.

Not yet implemented.
```

`apps/bench/package.json`:
```json
{
  "name": "bench",
  "version": "0.0.0",
  "private": true,
  "description": "js-framework-benchmark harness + SSR throughput bench. Not yet implemented."
}
```

`apps/bench/README.md`:
```markdown
# bench

js-framework-benchmark harness (keyed suite) and SSR throughput bench.
See `docs/superpowers/specs/2026-07-05-tez-architecture-design.md` §9.

Not yet implemented.
```

- [ ] **Step 10: `e2e/` and `rfcs/` placeholders**

`e2e/README.md`:
```markdown
# e2e

Playwright suites: resume behavior, no-JS baseline, prefetch.
See `docs/superpowers/specs/2026-07-05-tez-architecture-design.md` §9.

Not yet implemented.
```

`rfcs/README.md`:
```markdown
# rfcs

One markdown file per accepted architectural design change that isn't
already covered by `docs/superpowers/specs/2026-07-05-tez-architecture-design.md`.
See that spec's §10 (Decision Log) for decisions already made.
```

- [ ] **Step 11: Verify workspace install picks up all stub packages**

Run: `pnpm install && pnpm -r list --depth -1`
Expected: lists `tez` (root) plus `@tez/compiler`, `@tez/runtime-dom`, `@tez/runtime-server`, `@tez/server`, `@tez/vite-plugin`, `@tez/interop`, `create-tez`, `@tez/devtools`, `playground`, `docs`, `bench` — 11 workspace packages, no errors.

- [ ] **Step 12: Commit**

```bash
git add packages apps e2e rfcs
git commit -m "Add stub packages and apps per monorepo layout"
```

---

## Task 3: `packages/signals` scaffolding + dependency-tracking core

**Files:**
- Create: `packages/signals/package.json`
- Create: `packages/signals/tsconfig.json`
- Create: `packages/signals/vitest.config.ts`
- Create: `packages/signals/src/graph.ts`
- Test: `packages/signals/test/graph.test.ts`

**Interfaces:**
- Produces: `Source` interface (`addObserver(observer): void`, `removeObserver(observer): void`), `Observer` interface (`notify(): void`), `TrackingObserver extends Observer` (adds `addSource(source: Source): void`), `getCurrentConsumer(): TrackingObserver | null`, `withTracking<T>(consumer: TrackingObserver, fn: () => T): T`, `withoutTracking<T>(fn: () => T): T`, `trackAccess(source: Source): void`. Everything in Tasks 4–8 is built on these five exports.

- [ ] **Step 1: Package manifest**

`packages/signals/package.json`:
```json
{
  "name": "@tez/signals",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "description": "TC39-Signals-shaped reactive core. Zero dependencies, publishable alone.",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "scripts": {
    "build": "tsc --noEmit false --emitDeclarationOnly false -p tsconfig.json --outDir dist",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {},
  "devDependencies": {
    "vitest": "^2.1.8",
    "@vitest/coverage-v8": "^2.1.8",
    "typescript": "^5.6.0"
  }
}
```

- [ ] **Step 2: TS config**

`packages/signals/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src"]
}
```

- [ ] **Step 3: Vitest config with the 99% coverage gate**

`packages/signals/vitest.config.ts`:
```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/**/*.ts"],
      thresholds: {
        branches: 99,
        functions: 99,
        lines: 99,
        statements: 99,
      },
    },
  },
});
```

- [ ] **Step 4: Install dependencies**

Run: `pnpm install`
Expected: `packages/signals/node_modules` (or the hoisted root store) resolves vitest with no errors.

- [ ] **Step 5: Write the failing test for tracking primitives**

`packages/signals/test/graph.test.ts`:
```ts
import { describe, expect, it, vi } from "vitest";
import {
  getCurrentConsumer,
  trackAccess,
  withTracking,
  withoutTracking,
  type Source,
  type TrackingObserver,
} from "../src/graph";

function makeSource(): Source & { observers: Set<unknown> } {
  const observers = new Set<unknown>();
  return {
    observers,
    addObserver: vi.fn((observer) => observers.add(observer)),
    removeObserver: vi.fn((observer) => observers.delete(observer)),
  };
}

function makeConsumer(): TrackingObserver & { sources: Set<Source> } {
  const sources = new Set<Source>();
  return {
    sources,
    notify: vi.fn(),
    addSource: vi.fn((source) => sources.add(source)),
  };
}

describe("graph tracking primitives", () => {
  it("has no current consumer outside of withTracking", () => {
    expect(getCurrentConsumer()).toBeNull();
  });

  it("trackAccess is a no-op when there is no current consumer", () => {
    const source = makeSource();
    trackAccess(source);
    expect(source.addObserver).not.toHaveBeenCalled();
  });

  it("withTracking makes the consumer current for the duration of fn", () => {
    const consumer = makeConsumer();
    const source = makeSource();
    let observedDuring: TrackingObserver | null = null;

    withTracking(consumer, () => {
      observedDuring = getCurrentConsumer();
      trackAccess(source);
    });

    expect(observedDuring).toBe(consumer);
    expect(getCurrentConsumer()).toBeNull();
    expect(consumer.addSource).toHaveBeenCalledWith(source);
  });

  it("withTracking restores the previous consumer after fn returns", () => {
    const outer = makeConsumer();
    const inner = makeConsumer();
    let observedInner: TrackingObserver | null = null;

    withTracking(outer, () => {
      withTracking(inner, () => {
        observedInner = getCurrentConsumer();
      });
      expect(getCurrentConsumer()).toBe(outer);
    });

    expect(observedInner).toBe(inner);
  });

  it("withTracking restores the previous consumer even if fn throws", () => {
    const consumer = makeConsumer();
    expect(() =>
      withTracking(consumer, () => {
        throw new Error("boom");
      }),
    ).toThrow("boom");
    expect(getCurrentConsumer()).toBeNull();
  });

  it("withoutTracking suspends the current consumer for the duration of fn", () => {
    const consumer = makeConsumer();
    const source = makeSource();
    let observedDuring: TrackingObserver | null = consumer;

    withTracking(consumer, () => {
      withoutTracking(() => {
        observedDuring = getCurrentConsumer();
        trackAccess(source);
      });
      expect(getCurrentConsumer()).toBe(consumer);
    });

    expect(observedDuring).toBeNull();
    expect(consumer.addSource).not.toHaveBeenCalled();
  });

  it("withoutTracking restores the previous consumer even if fn throws", () => {
    const consumer = makeConsumer();
    withTracking(consumer, () => {
      expect(() =>
        withoutTracking(() => {
          throw new Error("boom");
        }),
      ).toThrow("boom");
      expect(getCurrentConsumer()).toBe(consumer);
    });
  });
});
```

- [ ] **Step 6: Run test to verify it fails**

Run: `pnpm --filter @tez/signals test`
Expected: FAIL — `Cannot find module '../src/graph'` (file doesn't exist yet).

- [ ] **Step 7: Implement `graph.ts`**

`packages/signals/src/graph.ts`:
```ts
export interface Observer {
  notify(): void;
}

export interface Source {
  addObserver(observer: Observer): void;
  removeObserver(observer: Observer): void;
}

export interface TrackingObserver extends Observer {
  addSource(source: Source): void;
}

let currentConsumer: TrackingObserver | null = null;

export function getCurrentConsumer(): TrackingObserver | null {
  return currentConsumer;
}

export function withTracking<T>(consumer: TrackingObserver, fn: () => T): T {
  const previous = currentConsumer;
  currentConsumer = consumer;
  try {
    return fn();
  } finally {
    currentConsumer = previous;
  }
}

export function withoutTracking<T>(fn: () => T): T {
  const previous = currentConsumer;
  currentConsumer = null;
  try {
    return fn();
  } finally {
    currentConsumer = previous;
  }
}

export function trackAccess(source: Source): void {
  if (currentConsumer) {
    currentConsumer.addSource(source);
    source.addObserver(currentConsumer);
  }
}
```

> **Correction (post-Task-4 review):** `trackAccess` is the single place that wires
> both directions of a dependency edge — the consumer records the source (via
> `addSource`) and the source records the consumer as an observer (via
> `addObserver`) — in the same call. No other file may call `source.addObserver`
> to establish a tracked dependency; `addSource` implementations (e.g.
> `Computed.addSource` in Task 5) only need to update their own bookkeeping.

- [ ] **Step 8: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 7 tests in `graph.test.ts`.

- [ ] **Step 9: Commit**

```bash
git add packages/signals
git commit -m "Scaffold @tez/signals with dependency-tracking core"
```

---

## Task 4: `Signal.State`

**Files:**
- Create: `packages/signals/src/propagate.ts`
- Create: `packages/signals/src/state.ts`
- Test: `packages/signals/test/state.test.ts`

**Interfaces:**
- Consumes: `Source`, `Observer`, `trackAccess` from `../src/graph` (Task 3).
- Produces: `markObserversStale(observers: Iterable<Observer>): void` (from `propagate.ts`, reused by Task 5's `Computed`). `State<T>` class with `constructor(initialValue: T, options?: { equals?: (a: T, b: T) => boolean })`, `get(): T`, `set(newValue: T): void`, plus `Source` methods `addObserver`/`removeObserver`. This is the class Task 9's `Signal.State` re-exports directly.

- [ ] **Step 1: Write the failing tests**

`packages/signals/test/state.test.ts`:
```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/signals test`
Expected: FAIL — `Cannot find module '../src/state'`.

- [ ] **Step 3: Implement `propagate.ts`**

`packages/signals/src/propagate.ts`:
```ts
import type { Observer } from "./graph";

export function markObserversStale(observers: Iterable<Observer>): void {
  for (const observer of Array.from(observers)) {
    observer.notify();
  }
}
```

> **Correction (post-Task-8 review):** iterates a snapshot (`Array.from(observers)`), not the live
> `Set`, because `Observer.notify()` can synchronously mutate that same `Set` before the loop
> finishes — specifically, `Watcher.notify()` removes itself as an observer and then, via a
> synchronous `scheduleEffect`/`flushEffects` flush outside `batch()`, immediately re-adds itself
> (`Effect.run()` re-arming via `watcher.watch()`). Per the `Set` iteration spec, deleting and
> re-adding the same value mid-iteration causes that live iterator to revisit it forever. Snapshotting
> up front sidesteps this regardless of what a given `Observer.notify()` does to the set during the
> call.

- [ ] **Step 4: Implement `state.ts`**

`packages/signals/src/state.ts`:
```ts
import { trackAccess, type Observer, type Source } from "./graph";
import { markObserversStale } from "./propagate";

export interface StateOptions<T> {
  equals?: (a: T, b: T) => boolean;
}

export class State<T> implements Source {
  private value: T;
  private readonly observers = new Set<Observer>();
  private readonly equals: (a: T, b: T) => boolean;

  constructor(initialValue: T, options?: StateOptions<T>) {
    this.value = initialValue;
    this.equals = options?.equals ?? Object.is;
  }

  get(): T {
    trackAccess(this);
    return this.value;
  }

  set(newValue: T): void {
    if (this.equals(this.value, newValue)) return;
    this.value = newValue;
    markObserversStale(this.observers);
  }

  addObserver(observer: Observer): void {
    this.observers.add(observer);
  }

  removeObserver(observer: Observer): void {
    this.observers.delete(observer);
  }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 7 tests in `state.test.ts` (plus the 7 from Task 3, 14 total).

- [ ] **Step 6: Commit**

```bash
git add packages/signals/src/propagate.ts packages/signals/src/state.ts packages/signals/test/state.test.ts
git commit -m "Implement Signal.State"
```

---

## Task 5: `Signal.Computed`

**Files:**
- Create: `packages/signals/src/computed.ts`
- Test: `packages/signals/test/computed.test.ts`

**Interfaces:**
- Consumes: `Source`, `Observer`, `TrackingObserver`, `withTracking`, `trackAccess` from `../src/graph` (Task 3); `markObserversStale` from `../src/propagate` (Task 4); `State` from `../src/state` (Task 4) for tests only.
- Produces: `Computed<T>` class with `constructor(compute: () => T, options?: { equals?: (a: T, b: T) => boolean })`, `get(): T`, `Source` methods `addObserver`/`removeObserver`, and (as a `TrackingObserver`) `notify(): void` / `addSource(source: Source): void`. Task 8's `Effect` wraps its function body in a `Computed<void>`; Task 9 re-exports this class as `Signal.Computed`.

- [ ] **Step 1: Write the failing tests**

`packages/signals/test/computed.test.ts`:
```ts
import { describe, expect, it, vi } from "vitest";
import { State } from "../src/state";
import { Computed } from "../src/computed";

describe("Computed", () => {
  it("computes its value lazily from a compute function", () => {
    const a = new State(2);
    const b = new State(3);
    const sum = new Computed(() => a.get() + b.get());
    expect(sum.get()).toBe(5);
  });

  it("does not call compute until get() is called", () => {
    const compute = vi.fn(() => 42);
    new Computed(compute);
    expect(compute).not.toHaveBeenCalled();
  });

  it("caches the value and does not recompute on repeated get() with no source change", () => {
    const compute = vi.fn(() => 42);
    const c = new Computed(compute);
    c.get();
    c.get();
    c.get();
    expect(compute).toHaveBeenCalledOnce();
  });

  it("recomputes after a source changes", () => {
    const a = new State(1);
    const compute = vi.fn(() => a.get() * 10);
    const c = new Computed(compute);
    expect(c.get()).toBe(10);
    a.set(2);
    expect(c.get()).toBe(20);
    expect(compute).toHaveBeenCalledTimes(2);
  });

  it("propagates staleness to computeds that depend on it", () => {
    const a = new State(1);
    const b = new Computed(() => a.get() + 1);
    const c = new Computed(() => b.get() + 1);
    expect(c.get()).toBe(3);
    a.set(10);
    expect(c.get()).toBe(12);
  });

  it("diamond dependency: each computed recomputes exactly once per source change", () => {
    const a = new State(1);
    const bCompute = vi.fn(() => a.get() + 1);
    const cCompute = vi.fn(() => a.get() + 2);
    const b = new Computed(bCompute);
    const c = new Computed(cCompute);
    const dCompute = vi.fn(() => b.get() + c.get());
    const d = new Computed(dCompute);

    expect(d.get()).toBe(1 + 1 + 1 + 2);
    a.set(5);
    expect(d.get()).toBe(5 + 1 + 5 + 2);

    expect(bCompute).toHaveBeenCalledTimes(2);
    expect(cCompute).toHaveBeenCalledTimes(2);
    expect(dCompute).toHaveBeenCalledTimes(2);
  });

  it("throws when a computation reads its own value (cycle)", () => {
    const c: Computed<number> = new Computed(() => c.get() + 1);
    expect(() => c.get()).toThrow(/cycle/i);
  });

  it("supports a custom equals option to keep the cached reference stable", () => {
    const a = new State(1);
    const c = new Computed(() => ({ n: a.get() % 2 }), {
      equals: (x, y) => x.n === y.n,
    });
    const first = c.get();
    a.set(3);
    const second = c.get();
    expect(second).toBe(first);
  });

  it("notify() is idempotent while already stale (no duplicate propagation)", () => {
    const a = new State(1);
    const b2 = new State(2);
    const c = new Computed(() => a.get() + b2.get());
    c.get();
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    c.addObserver(observer);
    a.set(10);
    b2.set(20);
    expect(observer.notify).toHaveBeenCalledOnce();
  });

  it("removeObserver stops further staleness propagation", () => {
    const a = new State(1);
    const c = new Computed(() => a.get());
    c.get();
    const observer = { notify: vi.fn(), addSource: vi.fn() };
    c.addObserver(observer);
    c.removeObserver(observer);
    a.set(2);
    expect(observer.notify).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/signals test`
Expected: FAIL — `Cannot find module '../src/computed'`.

- [ ] **Step 3: Implement `computed.ts`**

`packages/signals/src/computed.ts`:
```ts
import {
  trackAccess,
  withTracking,
  type Observer,
  type Source,
  type TrackingObserver,
} from "./graph";
import { markObserversStale } from "./propagate";

type ComputedState = "clean" | "stale" | "computing";

export interface ComputedOptions<T> {
  equals?: (a: T, b: T) => boolean;
}

export class Computed<T> implements Source, TrackingObserver {
  private cachedValue!: T;
  private hasValue = false;
  private state: ComputedState = "stale";
  private readonly sources = new Set<Source>();
  private readonly observers = new Set<Observer>();
  private readonly compute: () => T;
  private readonly equals: (a: T, b: T) => boolean;

  constructor(compute: () => T, options?: ComputedOptions<T>) {
    this.compute = compute;
    this.equals = options?.equals ?? Object.is;
  }

  get(): T {
    if (this.state !== "clean") {
      this.recompute();
    }
    trackAccess(this);
    return this.cachedValue;
  }

  private recompute(): void {
    if (this.state === "computing") {
      throw new Error(
        "Signal.Computed cycle detected: computation read its own value while computing",
      );
    }
    this.state = "computing";
    this.unsubscribeFromSources();

    const newValue = withTracking(this, () => this.compute());

    this.state = "clean";
    if (!this.hasValue || !this.equals(this.cachedValue, newValue)) {
      this.cachedValue = newValue;
      this.hasValue = true;
    }
  }

  private unsubscribeFromSources(): void {
    for (const source of this.sources) {
      source.removeObserver(this);
    }
    this.sources.clear();
  }

  addSource(source: Source): void {
    this.sources.add(source);
  }

  notify(): void {
    if (this.state === "clean") {
      this.state = "stale";
      markObserversStale(this.observers);
    }
  }

  addObserver(observer: Observer): void {
    this.observers.add(observer);
  }

  removeObserver(observer: Observer): void {
    this.observers.delete(observer);
  }

  dispose(): void {
    this.unsubscribeFromSources();
  }
}
```

> **Correction (post-final-review):** added `dispose()` (and factored its shared logic
> into `unsubscribeFromSources()`, also used by `recompute()`). Without it, nothing ever
> removed a `Computed` from its source signals' observer sets except a *subsequent*
> `recompute()` — so a `Computed` that stops being recomputed (e.g. the internal computed
> inside a disposed `Effect`, see Task 8's correction) stayed permanently reachable from
> its source signals, along with everything its compute closure captured. `Effect.dispose()`
> now calls `this.computed.dispose()` to release this.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 10 tests in `computed.test.ts` (24 total across the suite).

- [ ] **Step 5: Commit**

```bash
git add packages/signals/src/computed.ts packages/signals/test/computed.test.ts
git commit -m "Implement Signal.Computed with glitch-free lazy recomputation"
```

---

## Task 6: `batch()` and `untrack()`

**Files:**
- Create: `packages/signals/src/batch.ts`
- Create: `packages/signals/src/untrack.ts`
- Test: `packages/signals/test/batch.test.ts`
- Test: `packages/signals/test/untrack.test.ts`

**Interfaces:**
- Consumes: `withoutTracking` from `../src/graph` (Task 3); `State` from `../src/state` and `Computed` from `../src/computed` (Tasks 4–5) for tests only.
- Produces: `batch<T>(fn: () => T): T` and `scheduleEffect(effect: { run(): void }): void` (from `batch.ts` — Task 8's `Effect` calls `scheduleEffect(this)` from its `Watcher` notify callback). `untrack<T>(fn: () => T): T` (from `untrack.ts`).

`scheduleEffect` takes a minimal structural type (`{ run(): void }`) rather than importing `Effect` from `./effect`, so `batch.ts` has no dependency on `effect.ts` — `effect.ts` depends on `batch.ts`, not the other way around, keeping the import graph acyclic (Global Constraints).

- [ ] **Step 1: Write the failing tests for `batch`**

`packages/signals/test/batch.test.ts`:
```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/signals test`
Expected: FAIL — `Cannot find module '../src/batch'`.

- [ ] **Step 3: Implement `batch.ts`**

`packages/signals/src/batch.ts`:
```ts
export interface Runnable {
  run(): void;
}

let batchDepth = 0;
const pendingEffects = new Set<Runnable>();

export function batch<T>(fn: () => T): T {
  batchDepth++;
  try {
    return fn();
  } finally {
    batchDepth--;
    if (batchDepth === 0) flushEffects();
  }
}

export function scheduleEffect(effect: Runnable): void {
  pendingEffects.add(effect);
  if (batchDepth === 0) flushEffects();
}

function flushEffects(): void {
  if (pendingEffects.size === 0) return;
  const effects = Array.from(pendingEffects);
  pendingEffects.clear();
  for (const effect of effects) {
    effect.run();
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 6 tests in `batch.test.ts`.

- [ ] **Step 5: Write the failing tests for `untrack`**

`packages/signals/test/untrack.test.ts`:
```ts
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
```

- [ ] **Step 6: Run test to verify it fails**

Run: `pnpm --filter @tez/signals test`
Expected: FAIL — `Cannot find module '../src/untrack'`.

- [ ] **Step 7: Implement `untrack.ts`**

`packages/signals/src/untrack.ts`:
```ts
import { withoutTracking } from "./graph";

export function untrack<T>(fn: () => T): T {
  return withoutTracking(fn);
}
```

- [ ] **Step 8: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 3 tests in `untrack.test.ts` (33 total across the suite).

- [ ] **Step 9: Commit**

```bash
git add packages/signals/src/batch.ts packages/signals/src/untrack.ts packages/signals/test/batch.test.ts packages/signals/test/untrack.test.ts
git commit -m "Implement batch() and untrack()"
```

---

## Task 7: `Signal.subtle.Watcher`

**Files:**
- Create: `packages/signals/src/watcher.ts`
- Test: `packages/signals/test/watcher.test.ts`

**Interfaces:**
- Consumes: `Observer`, `Source` from `../src/graph` (Task 3); `State` from `../src/state`, `Computed` from `../src/computed` (Tasks 4–5) for tests only.
- Produces: `Watcher` class with `constructor(onNotify: () => void)`, `watch(...signals: Source[]): void`, `unwatch(...signals: Source[]): void`, `notify(): void`, `getPending(): Source[]`. Task 8's `Effect` constructs one `Watcher` per effect instance.

Per the real TC39 proposal, a `Watcher` fires its callback **at most once** per watched signal until that signal is explicitly re-armed via `watch()` again — this prevents the notify callback itself from synchronously observing a half-updated graph. This implementation adapts that rule: `notify()` disarms every currently-watched signal (removing itself as an observer of all of them, not just the one that fired), so the callback is expected to read `getPending()` and/or call `watch()` again to resume observation.

- [ ] **Step 1: Write the failing tests**

`packages/signals/test/watcher.test.ts`:
```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/signals test`
Expected: FAIL — `Cannot find module '../src/watcher'`.

- [ ] **Step 3: Implement `watcher.ts`**

`packages/signals/src/watcher.ts`:
```ts
import type { Observer, Source } from "./graph";

export class Watcher implements Observer {
  private readonly watched = new Set<Source>();
  private readonly armed = new Set<Source>();

  constructor(private readonly onNotify: () => void) {}

  watch(...signals: Source[]): void {
    for (const signal of signals) {
      this.watched.add(signal);
      if (!this.armed.has(signal)) {
        signal.addObserver(this);
        this.armed.add(signal);
      }
    }
  }

  unwatch(...signals: Source[]): void {
    for (const signal of signals) {
      this.watched.delete(signal);
      if (this.armed.has(signal)) {
        signal.removeObserver(this);
        this.armed.delete(signal);
      }
    }
  }

  notify(): void {
    if (this.armed.size === 0) return;
    for (const signal of this.watched) {
      signal.removeObserver(this);
    }
    this.armed.clear();
    this.onNotify();
  }

  getPending(): Source[] {
    return Array.from(this.watched).filter((signal) => !this.armed.has(signal));
  }
}
```

> **Correction (post-Task-9 review):** `notify()` now returns immediately if `armed` is
> already empty. Without this guard, a `Watcher` that watches both a source and something
> downstream of that source (e.g. `watch(state, computedThatReadsState)`) gets notified
> twice for one change: once transitively (the computed's own propagation reaches the
> watcher first and disarms it) and once directly (the watcher was also a direct entry in
> the source's own observer snapshot, captured before the transitive call ran). Since a
> disarmed watcher (`armed.size === 0`) has nothing left to report until it's re-armed via
> `watch()`, a second `notify()` call arriving before that happens is stale and must be a
> no-op.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 9 tests in `watcher.test.ts` (42 total across the suite).

- [ ] **Step 5: Commit**

```bash
git add packages/signals/src/watcher.ts packages/signals/test/watcher.test.ts
git commit -m "Implement Signal.subtle.Watcher"
```

---

## Task 8: `effect()`

**Files:**
- Create: `packages/signals/src/effect.ts`
- Test: `packages/signals/test/effect.test.ts`

**Interfaces:**
- Consumes: `Computed` from `../src/computed` (Task 5); `Watcher` from `../src/watcher` (Task 7); `scheduleEffect` from `../src/batch` (Task 6); `State` from `../src/state`, `batch` from `../src/batch` for tests only.
- Produces: `EffectCleanup = () => void`, `EffectFn = () => void | EffectCleanup`, `effect(fn: EffectFn): () => void` (the returned function disposes the effect). Task 9 re-exports `effect` directly as the public API.

Nested `effect()` calls made from inside a running effect's `fn` are tracked as children of that effect via a module-level `currentOwner` stack, and are disposed automatically whenever the parent re-runs (before re-invoking `fn`) or is itself disposed — this is the "auto-disposed with owning scope" behavior from spec §2.2.

- [ ] **Step 1: Write the failing tests**

`packages/signals/test/effect.test.ts`:
```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/signals test`
Expected: FAIL — `Cannot find module '../src/effect'`.

- [ ] **Step 3: Implement `effect.ts`**

`packages/signals/src/effect.ts`:
```ts
import { Computed } from "./computed";
import { Watcher } from "./watcher";
import { scheduleEffect } from "./batch";

export type EffectCleanup = () => void;
export type EffectFn = () => void | EffectCleanup;

let currentOwner: Effect | null = null;

export class Effect {
  private readonly computed: Computed<void>;
  private readonly watcher: Watcher;
  private readonly children = new Set<Effect>();
  private cleanup: EffectCleanup | undefined;
  private disposed = false;

  constructor(private readonly fn: EffectFn) {
    if (currentOwner) {
      currentOwner.children.add(this);
    }
    this.computed = new Computed(() => {
      this.disposeChildren();
      this.cleanup?.();
      this.cleanup = undefined;
      const previousOwner = currentOwner;
      currentOwner = this;
      try {
        this.cleanup = this.fn() ?? undefined;
      } finally {
        currentOwner = previousOwner;
      }
    });
    this.watcher = new Watcher(() => scheduleEffect(this));
    this.run();
  }

  private disposeChildren(): void {
    for (const child of this.children) {
      child.dispose();
    }
    this.children.clear();
  }

  run(): void {
    if (this.disposed) return;
    this.computed.get();
    this.watcher.watch(this.computed);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.disposeChildren();
    this.watcher.unwatch(this.computed);
    this.computed.dispose();
    this.cleanup?.();
    this.cleanup = undefined;
  }
}

export function effect(fn: EffectFn): () => void {
  const instance = new Effect(fn);
  return () => instance.dispose();
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 10 tests in `effect.test.ts` (52 total across the suite).

- [ ] **Step 5: Commit**

```bash
git add packages/signals/src/effect.ts packages/signals/test/effect.test.ts
git commit -m "Implement effect() with owner-scoped auto-disposal"
```

---

## Task 9: Public API surface + adapted TC39/Solid test subsets

**Files:**
- Create: `packages/signals/src/index.ts`
- Test: `packages/signals/test/index.test.ts`
- Test: `packages/signals/test/tc39-polyfill-adapted.test.ts`
- Test: `packages/signals/test/solid-reactivity-adapted.test.ts`

**Interfaces:**
- Consumes: `State`, `StateOptions` from `../src/state`; `Computed`, `ComputedOptions` from `../src/computed`; `Watcher` from `../src/watcher`; `batch` from `../src/batch`; `untrack` from `../src/untrack`; `effect`, `EffectCleanup`, `EffectFn` from `../src/effect`.
- Produces: `signal<T>(initialValue: T, options?: StateOptions<T>): State<T>`, `computed<T>(fn: () => T, options?: ComputedOptions<T>): Computed<T>`, re-exported `effect`, `batch`, `untrack`, and `Signal = { State, Computed, subtle: { Watcher } }`. This is the module `@tez/signals` resolves to and the only surface later packages (`runtime-dom`, `compiler`) may import from.

- [ ] **Step 1: Write the failing test for the public API surface**

`packages/signals/test/index.test.ts`:
```ts
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @tez/signals test`
Expected: FAIL — `Cannot find module '../src/index'`.

- [ ] **Step 3: Implement `index.ts`**

`packages/signals/src/index.ts`:
```ts
import { State, type StateOptions } from "./state";
import { Computed, type ComputedOptions } from "./computed";
import { Watcher } from "./watcher";

export { batch } from "./batch";
export { untrack } from "./untrack";
export { effect } from "./effect";
export type { EffectCleanup, EffectFn } from "./effect";
export type { StateOptions, ComputedOptions };

export function signal<T>(initialValue: T, options?: StateOptions<T>): State<T> {
  return new State(initialValue, options);
}

export function computed<T>(fn: () => T, options?: ComputedOptions<T>): Computed<T> {
  return new Computed(fn, options);
}

export const Signal = {
  State,
  Computed,
  subtle: {
    Watcher,
  },
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 5 tests in `index.test.ts` (57 total across the suite).

- [ ] **Step 5: Write the adapted TC39 signal-polyfill test subset**

`packages/signals/test/tc39-polyfill-adapted.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { Signal } from "../src/index";

// Adapted from the TC39 proposal-signals reference polyfill test suite
// (github.com/proposal-signals/signal-polyfill), trimmed to the behaviors
// this package implements: State, Computed, and subtle.Watcher.
describe("adapted TC39 signal-polyfill test subset", () => {
  it("Signal.State: get returns the set value", () => {
    const s = new Signal.State(1);
    expect(s.get()).toBe(1);
    s.set(2);
    expect(s.get()).toBe(2);
  });

  it("Signal.Computed: recomputes only when read after a dependency changes", () => {
    const s = new Signal.State(1);
    let calls = 0;
    const c = new Signal.Computed(() => {
      calls++;
      return s.get() * 2;
    });
    expect(calls).toBe(0);
    expect(c.get()).toBe(2);
    expect(calls).toBe(1);
    expect(c.get()).toBe(2);
    expect(calls).toBe(1);
    s.set(2);
    expect(calls).toBe(1);
    expect(c.get()).toBe(4);
    expect(calls).toBe(2);
  });

  it("Signal.subtle.Watcher: notify fires once per change, then must be re-armed", () => {
    const s = new Signal.State(1);
    const notifications: number[] = [];
    const w = new Signal.subtle.Watcher(() => notifications.push(s.get()));
    w.watch(s);
    s.set(2);
    s.set(3);
    expect(notifications).toEqual([2]);
    w.watch(s);
    s.set(4);
    expect(notifications).toEqual([2, 4]);
  });

  it("diamond dependency graph resolves to a consistent final value", () => {
    const s = new Signal.State(2);
    const double = new Signal.Computed(() => s.get() * 2);
    const triple = new Signal.Computed(() => s.get() * 3);
    const sum = new Signal.Computed(() => double.get() + triple.get());

    expect(sum.get()).toBe(10);
    s.set(3);
    expect(sum.get()).toBe(15);
  });

  it("Watcher.getPending reflects signals disarmed by a notification", () => {
    const a = new Signal.State(1);
    const b = new Signal.State(2);
    const w = new Signal.subtle.Watcher(() => {});
    w.watch(a, b);
    expect(w.getPending()).toEqual([]);
    a.set(10);
    expect(w.getPending()).toEqual([a, b]);
  });
});
```

- [ ] **Step 6: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 5 tests in `tc39-polyfill-adapted.test.ts` (62 total). No implementation changes are expected in this step — these tests exercise existing code through the public `Signal` namespace.

- [ ] **Step 7: Write the adapted Solid reactivity test subset**

`packages/signals/test/solid-reactivity-adapted.test.ts`:
```ts
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
```

- [ ] **Step 8: Run test to verify it passes**

Run: `pnpm --filter @tez/signals test`
Expected: PASS — 6 tests in `solid-reactivity-adapted.test.ts` (68 total across the suite, 0 failures).

- [ ] **Step 9: Commit**

```bash
git add packages/signals/src/index.ts packages/signals/test/index.test.ts packages/signals/test/tc39-polyfill-adapted.test.ts packages/signals/test/solid-reactivity-adapted.test.ts
git commit -m "Add public API surface and adapted TC39/Solid test subsets"
```

---

## Task 10: Coverage gate + root wiring verification

**Files:**
- Modify: `packages/signals/test/watcher.test.ts`
- Modify: `packages/signals/package.json` (only if the coverage report demands a source change, not expected)

**Interfaces:**
- Consumes: everything built in Tasks 3–9.
- Produces: nothing new — this task closes out the Phase 0 gate from spec §8 (≥99% branch coverage, zero deps, adapted test subsets passing) and confirms the root `turbo` pipeline actually reaches `packages/signals`.

- [ ] **Step 1: Add the one branch this plan's tests don't yet exercise**

`unwatch()` on a signal that was added to `watched` but whose `armed` entry was already cleared by a prior `notify()` hits the `if (this.armed.has(signal))` false branch — no test in Task 7 exercises calling `unwatch()` after a `notify()` has already disarmed the signal. Add it to `packages/signals/test/watcher.test.ts`:

```ts
  it("unwatch() after notify() has already disarmed the signal is a no-op", () => {
    const onNotify = vi.fn();
    const s = new State(1);
    const w = new Watcher(onNotify);
    w.watch(s);
    s.set(2);
    expect(() => w.unwatch(s)).not.toThrow();
    expect(w.getPending()).toEqual([]);
  });
```

Add this `it(...)` block inside the existing `describe("Watcher", ...)` block, after the `"unwatch() stops future notifications for that signal"` test.

- [ ] **Step 2: Run the full suite with coverage**

Run: `pnpm --filter @tez/signals test:coverage`
Expected: all test files pass; the v8 coverage summary reports branches/functions/lines/statements each ≥99% for every file under `src/`. Vitest exits non-zero automatically if any threshold in `vitest.config.ts` isn't met.

- [ ] **Step 3: If any file is still below threshold, close the gap**

If Step 2 fails, the text reporter prints the uncovered line numbers per file, e.g.:
```
src/computed.ts | 97.5 | 90 | 100 | 97.5 | 42-43
```
For each reported file:
1. Open the file at the listed line numbers.
2. Identify which `if`/`??`/ternary branch has no test reaching it.
3. Add one `it(...)` case to that file's existing test file (e.g. `computed.test.ts` for `computed.ts`) that drives execution through the missing branch, following the same TDD style as the earlier steps (write the test, confirm it exercises new code, confirm the suite still passes).
4. Re-run `pnpm --filter @tez/signals test:coverage` and repeat until every file is ≥99% on all four metrics.

- [ ] **Step 4: Confirm zero runtime dependencies**

Run: `node -e "const p = require('./packages/signals/package.json'); if (Object.keys(p.dependencies ?? {}).length) { console.error('non-zero deps:', p.dependencies); process.exit(1); } console.log('zero runtime deps OK')"`
Expected: prints `zero runtime deps OK`.

- [ ] **Step 5: Confirm the root turbo pipeline reaches `@tez/signals`**

Run: `pnpm test` (from repo root)
Expected: turbo runs the `test` task across all workspace packages; `@tez/signals`'s vitest suite (68+ tests) shows as passing in the turbo output, other packages show their `echo 'no tests yet'` stub output with exit code 0.

Run: `pnpm test:coverage` (from repo root)
Expected: same, with `@tez/signals` producing a coverage report and the other stub packages no-op'ing successfully.

- [ ] **Step 6: Typecheck the whole workspace**

Run: `pnpm typecheck` (from repo root)
Expected: `tsc --noEmit` succeeds with zero errors across `@tez/signals` and every stub package.

- [ ] **Step 7: Commit**

```bash
git add packages/signals/test/watcher.test.ts
git commit -m "Close coverage gap and verify Phase 0 gate end-to-end"
```

---

## Phase 0 Gate Checklist (spec §8)

- [ ] `packages/signals`: state/computed/effect/batch/untrack implemented (Tasks 4, 5, 6, 8).
- [ ] TC39-shaped internals: `Signal.State`, `Signal.Computed`, `Signal.subtle.Watcher` (Tasks 4, 5, 7, 9).
- [ ] Zero dependencies (Task 10, Step 4).
- [ ] ≥99% branch coverage (Task 10, Steps 2–3).
- [ ] Passes an adapted subset of the TC39 signal-polyfill test suite (Task 9).
- [ ] Passes an adapted subset of Solid's reactivity test cases (Task 9).

# TEZ — Framework Architecture & Implementation Directive

> **Audience:** Claude Code, acting as principal engineer on this codebase.
> **Status:** Authoritative. Every implementation decision must trace back to a rule in this document. If a task conflicts with this document, stop and flag the conflict instead of improvising.

---

## 0. Mission

Tez is a compiled, resumable, signals-based web framework. One sentence of physics: **the compiler decides what runs where; the browser executes only what the user touches.**

Non-negotiable outcomes:

1. **Zero hydration.** No component function ever re-executes on the client to "boot" the page. State resumes from serialized HTML.
2. **O(1) startup JS.** Initial interactive payload is the resume-loader (< 1.5 KB gzip) regardless of app size.
3. **No VDOM.** Compiler emits direct DOM bindings driven by fine-grained signals.
4. **Host-agnostic server.** Runs identically on Node, Bun, Deno, Cloudflare Workers, Lambda. No feature may exist that only works on one host.
5. **Errors teach.** Every compiler and runtime error carries: code, location, cause, and a suggested fix. An error without a suggested fix is a bug.

Anti-goals (reject PRs/ideas that introduce these):

- No hydration fallback mode "for compatibility."
- No class components, no lifecycle methods, no `this`.
- No proprietary cloud features. No telemetry.
- No runtime scheduler/concurrent-mode complexity — fine-grained reactivity makes it unnecessary.
- No config option whose only purpose is to opt out of a core guarantee.

---

## 1. Repository Layout (pnpm + turborepo monorepo)

```
tez/
├── packages/
│   ├── compiler/          # Rust (oxc-based). JSX transform, reactivity analysis,
│   │                      # closure extraction, QRL emission, SSR codegen.
│   │                      # Exposed to JS via napi-rs. THE core asset.
│   ├── runtime-dom/       # Client runtime: signals, effects, template cloning,
│   │                      # event delegation, resume-loader. Budget: 4 KB gzip full,
│   │                      # 1.5 KB loader-only.
│   ├── runtime-server/    # SSR string-stream renderer + signal serializer.
│   ├── signals/           # TC39-signals-compatible core. Zero deps. Publishable alone.
│   ├── server/            # Nitro integration: routing, routeRules, ISR/SWR cache,
│   │                      # server$ RPC, revalidateTag.
│   ├── vite-plugin/       # Dev server, HMR, build orchestration. Vite 6+ Environment API.
│   ├── interop/           # customElement(), fromReact(), fromVue() adapters.
│   ├── create-tez/        # Scaffolding CLI.
│   └── devtools/          # Browser extension: signal graph inspector (phase 4).
├── apps/
│   ├── playground/        # Kitchen-sink app, exercises every feature.
│   ├── docs/              # Docs site, built with Tez itself (dogfood gate for v0.3+).
│   └── bench/             # js-framework-benchmark harness + SSR throughput bench.
├── e2e/                   # Playwright suites.
└── rfcs/                  # One markdown file per accepted design change.
```

Rules:
- `signals` has zero dependencies and zero imports from other packages.
- `runtime-dom` may import only from `signals`.
- `compiler` never imports runtime code; it emits references to it.
- Circular package deps are a CI failure.

---

## 2. Language Surface (what users write)

TypeScript + JSX. No custom file format, no SFCs. `.tsx` in, standard tooling works.

### 2.1 Components

```tsx
// A component is a plain function. It runs ONCE per instance, EVER
// (on the server during SSR, or on the client for client-created instances).
// It never "re-renders". There is no render loop.
export function Counter(props: { start: number }) {
  let count = signal(props.start);
  let double = computed(() => count * 2);

  return (
    <div>
      <button onClick={() => count++}>+</button>
      <span>{count} / {double}</span>
    </div>
  );
}
```

### 2.2 Signal ergonomics — compiler-unwrapped

- `signal(v)` declares reactive state. In compiler-processed `.tsx`, reads/writes use plain variable syntax (`count`, `count++`, `count = 5`). The compiler rewrites to `.get()/.set()`.
- In plain `.ts` files (no compiler), users interact with the raw API: `count.get()`, `count.set(5)`. Document both forms.
- `computed(fn)` — lazy, cached, auto-tracked.
- `effect(fn)` — side effects; auto-disposed with owning scope.
- `untrack(fn)`, `batch(fn)` — escape hatches.
- The signal core MUST track the TC39 Signals proposal API shape (`Signal.State`, `Signal.Computed`, `Signal.subtle.Watcher`) internally, with our sugar as a thin layer. When the proposal ships natively, we alias, not migrate.

### 2.3 Control flow — components, not re-execution

Because components run once, JSX expressions can't "re-run" for lists/branches. Provide:

```tsx
<Show when={user}>{u => <Profile user={u} />}</Show>
<For each={items}>{(item, i) => <Row item={item} index={i} />}</For>
<Switch>...</Switch>
<Dynamic component={comp} />   // ONLY sanctioned dynamic-type escape hatch
```

Compiler enforces: a lowercase JSX tag must be statically known HTML; a component reference must be statically resolvable or wrapped in `<Dynamic>`. Violation = compile error `TEZ103` with fix suggestion.

### 2.4 Restrictions that make analysis possible (enforced, with error codes)

- `TEZ101` — signal write during component body execution (must be inside handler/effect).
- `TEZ102` — spreading unknown props onto native elements blocks static template extraction; require `<Dynamic>` or explicit props.
- `TEZ103` — dynamic component type outside `<Dynamic>`.
- `TEZ104` — closure crossing a serialization boundary captures a non-serializable value (see §4.3). Error must name the variable, its inferred type, and suggest: move into handler, wrap in `server$`, or mark `transient`.

---

## 3. Compiler (packages/compiler) — the core asset

Rust, built on **oxc** (parser/semantic analysis). Exposed via napi-rs bindings to the Vite plugin. All transforms operate on oxc AST; no string manipulation.

### 3.1 Pipeline

```
.tsx source
  → oxc parse + semantic analysis
  → PASS 1: reactivity analysis
      - classify every JSX expression: static | signal-driven | server-only
      - build the signal dependency graph per component
  → PASS 2: boundary inference (THE differentiator — see §4)
      - decide per-expression: server-only / resumable-client / shared
      - extract every event handler & client effect into its own chunk (QRL)
  → PASS 3: codegen, dual output per component:
      a) DOM build:  cloneable <template> strings + binding effects
      b) SSR build:  string-stream writer function
  → source maps end-to-end (both outputs map to original .tsx)
```

### 3.2 DOM codegen contract

```tsx
// input
<button onClick={() => count++}>Count: {count}</button>
```

```js
// output (DOM build)
const _t1 = template(`<button>Count: <!></button>`);
function Counter_client(props) {
  const count = restoreSignal("s0", props);       // resumed, not recreated
  const el = _t1();
  insert(el, () => count.get(), el.childNodes[1]); // fine-grained text binding
  return el;
}
// handler chunk: counter-inc.a1f2.js
export const h = (ev, scope) => scope.s0.set(scope.s0.get() + 1);
```

```js
// output (SSR build)
function Counter_ssr(props, out, ctx) {
  const count = ctx.signal("s0", props.start);
  out.write(`<button on:click="`);
  out.write(qrl("counter-inc.a1f2.js#h", ["s0"]));
  out.write(`">Count: `);
  out.write(esc(count.get()));
  out.write(`</button>`);
}
```

SSR is string writes only. No element objects, no tree allocation. This is a hard rule; benchmark-gate it (§9).

### 3.3 Performance requirements

- Incremental: re-compiling one changed file in a 3,000-file app < 50 ms.
- Whole-program boundary inference runs on the module graph incrementally; cache analysis per-module keyed by content hash + import signatures.
- Compiler panics are bugs: always emit structured diagnostics.

---

## 4. Resumability & Automatic Boundaries (the thesis)

### 4.1 Model

- Server executes components once, producing HTML + a serialized state map.
- Every event handler / client effect is compiled to a **QRL**: a URL reference to a lazily-loadable chunk plus the IDs of the scope signals it captures.
- HTML carries QRLs as attributes: `on:click="/c/cart-toggle.a1f2.js#h[s0,s4]"`.
- A single global **resume-loader** (< 1.5 KB gzip, inlined into HTML) installs delegated listeners for all events at document root. On first matching interaction: fetch chunk → deserialize referenced signals → execute handler. Synchronous-feeling via speculative prefetch (§4.4).

### 4.2 Boundary inference (replaces Qwik's `$`, RSC's `'use client'`, Astro's `client:`)

Per closure/expression, the compiler classifies:

1. Referenced only during initial render, no signal deps → **server-only**: rendered to HTML, code never shipped.
2. Event handler or effect depending on signals → **client chunk** (QRL).
3. Shared pure utilities → duplicated into both builds; warn `TEZ301` if a shared chunk exceeds 8 KB.

Never ask the user to annotate. If inference is genuinely ambiguous, the error must present the ambiguity and the two explicit fixes — it must never silently guess.

### 4.3 Serialization contract

Serializable across the boundary: JSON scalars/objects/arrays, `Date`, `Map`, `Set`, `URL`, signals (by ID), QRL references, cyclic graphs (via reference table). NOT serializable: functions (unless compiled to QRL), class instances (unless `@serializable` with codec), DOM nodes, promises in flight (represented as resumable placeholders), streams.

`TEZ104` fires at compile time when a client-bound closure captures a non-serializable. This error's quality is a top-3 project priority — it is Qwik's biggest DX failure and our biggest opportunity.

Serialization format: single `<script type="tez/state">` JSON blob with a reference table (flat array + pointer indices, like Qwik's, but versioned with a schema byte).

### 4.4 Prefetch

Emit a per-page manifest of QRLs. Resume-loader prefetches chunks for visible interactive elements via `IntersectionObserver` + `<link rel="modulepreload">`, priority-ordered by document position. No service worker in v0 (keep the loader budget).

---

## 5. Server Layer (packages/server) — Nitro, adopted not rebuilt

- Built on **Nitro** (same engine as Nuxt/SolidStart). We write a Nitro preset + renderer, not a server.
- File-based routing: `src/routes/**` → pages; `src/routes/api/**` → endpoints.

### 5.1 Rendering modes — per-route config, all portable

```ts
// tez.config.ts
export default defineConfig({
  routes: {
    "/":             { prerender: true },
    "/blog/**":      { isr: 3600 },
    "/dashboard/**": { ssr: true, cache: false },
    "/app/**":       { spa: true },
  },
  cache: process.env.REDIS_URL
    ? redisDriver(process.env.REDIS_URL)
    : fsDriver(".tez/cache"),
});
```

- Modes: `prerender` (SSG), `ssr` (streaming, out-of-order via Suspense placeholders), `isr: seconds` (stale-while-revalidate semantics), `spa` (shell + client render — the ONE place full client execution is allowed, still no hydration since there's no server HTML to hydrate).
- ISR/SWR MUST work identically on a bare VPS with the fs/redis driver and on Workers with KV. Any mode that only works on one host is rejected (Mission rule 4).
- `revalidateTag(tag)` invalidates across all instances via the shared cache driver.

### 5.2 server$ — typed RPC

```tsx
const getOrders = server$(async (userId: string) => {
  return db.orders.where({ userId });          // this code never ships to client
});

// in a component:
const orders = query(getOrders, { key: ["orders", uid], swr: 60 });
```

- Compiler strips `server$` bodies from client builds, emits a typed fetch stub (POST `/​_tez/rpc/:hash`, body = serialized args).
- `query()` returns a signal; SWR revalidation reuses the same tag/cache machinery as ISR. One caching system, not two.
- CSRF: same-origin + double-submit token, on by default.

---

## 6. Interop (packages/interop)

1. `customElement("tez-cart", Cart)` — compile any component to a web component (shadow-DOM optional, attributes→props reflection, typed events). This is the primary embed/adoption story.
2. `fromReact(loader)` / `fromVue(loader)` — mount foreign components inside Tez subtrees; foreign runtime loads lazily on first render of that subtree; props bridged from signals via effects.
3. `mount(Component, element, props?)` — attach to any existing DOM node on any page. Core runtime ≤ 4 KB gzip so this is a realistic "sprinkle" story.
4. State interop is free by design: signals follow the TC39 shape (§2.2).

---

## 7. Error System (cross-cutting, phase-1 deliverable, not polish)

- Every diagnostic: `TEZ###` code + doc URL + primary span + cause + ≥1 concrete fix.
- Ranges: 1xx compile/authoring, 2xx reactivity runtime, 3xx bundling/budgets, 4xx server/rpc/cache, 5xx serialization/resume.
- Runtime errors in dev overlay show: signal graph slice that triggered the effect, serialized state snapshot, chunk identity.
- Hydration mismatch errors do not exist (no hydration). Resume errors (`TEZ5xx`) must state which QRL failed, which signal IDs it wanted, and what was actually in the state blob.
- CI gate: an error-message snapshot test suite. Changing an error message requires updating the snapshot — reviews weigh message quality.

---

## 8. Phased Delivery (strict order; do not start N+1 before N's gate passes)

**Phase 0 — Signals core.** `packages/signals`: state/computed/effect/batch/untrack, TC39-shaped internals, ≥ 99% branch coverage, zero deps. *Gate:* passes an adapted subset of the TC39 signals polyfill test suite + Solid's reactivity test cases.

**Phase 1 — Compiler MVP + DOM runtime.** JSX→template codegen, signal unwrapping, control-flow components, event delegation. Client-only apps work via `mount()`. *Gate:* js-framework-benchmark keyed suite runs; TodoMVC in playground; error codes TEZ101–104 implemented with fixes.

**Phase 2 — SSR + resumability.** String-stream SSR build, state serializer, QRL extraction, resume-loader, boundary inference v1 (conservative: when unsure, client chunk + `TEZ302` info diagnostic). *Gate:* playground page is interactive with loader-only JS; Playwright proves zero component re-execution on load; serialization round-trip fuzz tests.

**Phase 3 — Server layer.** Nitro preset, routing, per-route modes, ISR with fs/redis/KV drivers, `server$` + `query()`, `revalidateTag`. *Gate:* same app deploys to Node VPS + Cloudflare Workers from one codebase, ISR verified on both; SSR throughput bench ≥ 5x Next.js 16 on identical 33-route fixture.

**Phase 4 — DX completion.** create-tez, docs site built with Tez, devtools signal inspector, interop adapters, boundary inference v2 (aggressive server-only elimination). *Gate:* docs dogfood; a React dev completes the tutorial in < 30 min without prior signals experience (user-test this).

---

## 9. Quality Gates (CI, every PR)

- **Size budgets (hard fail):** resume-loader ≤ 1.5 KB gzip; runtime-dom full ≤ 4 KB; hello-world total JS ≤ 2 KB; TodoMVC total ≤ 12 KB.
- **Perf budgets:** js-framework-benchmark keyed geometric mean within 1.10 of vanilla-js reference build (tracked, alert on regress > 3%); SSR: ≥ 50k simple-component renders/sec/core on the bench fixture.
- **Compile speed:** cold 1,000-component fixture < 3 s; incremental single-file < 50 ms.
- **Tests:** unit (vitest + Rust tests), E2E (Playwright: resume behavior, no-JS baseline, prefetch), fuzz (serializer round-trips), error-snapshot suite (§7).
- **No-JS test:** every playground page must render meaningful HTML with JS disabled.

---

## 10. Decision Log (ADRs — why, so future-you doesn't relitigate)

| # | Decision | Over | Because |
|---|---|---|---|
| 1 | Signals, TC39-shaped | VDOM, proprietary signals | Ecosystem convergence; future-native; interop for free |
| 2 | Resumability | Hydration, partial hydration | Double render is the root cost; islands still hydrate |
| 3 | Compiler-inferred boundaries | `$`, `'use client'`, `client:*` | Humans drawing the line is the #1 DX failure of Qwik/RSC/Astro |
| 4 | JSX/TSX surface | SFC/custom format | Free TS tooling, editors, prettier, eslint day one |
| 5 | Rust/oxc compiler | Babel/SWC-plugin | Whole-program analysis speed; incremental at scale |
| 6 | Nitro server | Custom server | Solved problem; host portability is a mission rule |
| 7 | SSR = string streams | Element-tree SSR | 5–10x less allocation; the anti-Next.js RAM story |
| 8 | One cache system (ISR+query tags) | Separate data/page caches | Next.js's dual cache confusion is a known failure mode |
| 9 | No hydration fallback | "Compat mode" | A fallback becomes the default; kills the thesis |
| 10 | WC output as primary interop | Framework bridges first | Embeds anywhere incl. server-rendered backends (Laravel/Django) |

New architectural decisions require an RFC in `rfcs/` referencing the mission rules they uphold.

---

## 11. Working Agreements for Claude Code

- Read this file at session start. Cite rule numbers when making tradeoffs.
- TDD for runtime + serializer: failing test first, then implementation.
- Any PR touching compiler output updates codegen snapshot tests in the same PR.
- Never weaken a budget or gate to make a task pass; flag instead.
- Prefer boring code in runtime packages (readability > cleverness); cleverness lives in the compiler, documented.
- Public API changes: update playground + docs in the same PR.
- When blocked on an ambiguous design point: write a short RFC draft with 2–3 options and a recommendation; do not pick silently.

---

## 12. Initial Implementation Scope (this cycle)

Per §8's strict phase ordering, this implementation cycle covers:

- Full monorepo skeleton per §1 (pnpm workspace, turborepo pipeline, empty/stub packages and apps so the layout and tooling exist).
- Full implementation of `packages/signals` (Phase 0) to its gate: state/computed/effect/batch/untrack, TC39-shaped internals (`Signal.State`, `Signal.Computed`, `Signal.subtle.Watcher`), zero dependencies, ≥99% branch coverage, passing an adapted subset of the TC39 signals polyfill test suite plus Solid's reactivity test cases.
- All other packages (`compiler`, `runtime-dom`, `runtime-server`, `server`, `vite-plugin`, `interop`, `create-tez`, `devtools`) remain stubs (package.json + empty entrypoint) until their phase begins.

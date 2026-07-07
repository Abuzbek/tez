# Compiler TEZ101 Enforcement — Design (Phase 1, Cycle 2, Sub-cycle 2, Piece 3 of 3)

> **Status:** Approved by user 2026-07-07. Scope: `packages/compiler` — detect a signal write in a component's synchronous body and emit the `TEZ101` diagnostic; establish the reusable `Diagnostic` type the later codes (TEZ102–104) will share.
> **Completes:** the reactivity-analysis sub-cycle (piece 1: semantic signal detection; piece 2: JSX expression classification; piece 3: this document).
> **Depends on:** `find_reactive_bindings()` (piece 1) for the signal-binding map, and piece 2's component-discovery / reference-resolution patterns, which this piece mirrors.

---

## 0. What TEZ101 is

From the architecture spec (§7 Error System):

> `TEZ101` — signal write during component body execution (must be inside handler/effect).

A component body runs during render. A `count.set(...)` placed directly in it executes on every render, which at best does redundant work and at worst re-triggers the render that ran it. The legal homes for a write are event handlers and `effect()` callbacks — both are nested functions whose execution is deferred past render.

The spec's per-diagnostic contract (§7) binds this piece: every diagnostic carries a `TEZ###` code, a doc URL, a primary span, a cause, and at least one concrete fix; and error messages are locked by a snapshot test suite.

---

## 1. Module structure

Two new files, no new crate dependencies.

### `packages/compiler/src/diagnostics.rs` — the reusable type

```rust
pub struct Diagnostic {
    pub code: &'static str,  // "TEZ101"
    pub span: Span,          // primary span: the offending `.set(...)` call expression
    pub message: String,     // what happened, naming the signal and the component
    pub cause: String,       // why it is an error
    pub help: String,        // >= 1 concrete fix
    pub docs_url: String,    // https://tez.dev/errors/TEZ101
}

impl Diagnostic {
    /// Stable plain-text rendering -- the surface the error-message
    /// snapshot suite asserts against.
    pub fn render(&self, source: &str) -> String;
}
```

`render()` output shape (exact wording finalized in the plan; field order is fixed here):

```
error[TEZ101]: signal `count` is written during `Counter`'s body execution
  --> 4:3
cause: a component body runs on every render; this write executes each time and can re-trigger the render that ran it
help: move the write into an event handler or an effect() callback
docs: https://tez.dev/errors/TEZ101
```

Line/column are computed from `span.start` against `source`. Deliberately **not** built now: a code registry, severity levels, multi-span labels, JSON output. `Diagnostic` gets its second producer when TEZ102 arrives; extend it then.

### `packages/compiler/src/tez101.rs` — the checker

```rust
pub fn check_body_signal_writes(
    program: &Program,
    semantic: &Semantic,
    reactive_bindings: &HashMap<SymbolId, ReactiveKind>,
) -> Vec<Diagnostic>;
```

(No `source` parameter: the signal and component names come from AST identifiers, and line/col is computed by `render()`, which is the only place that needs the source text.)

---

## 2. What counts as a component

Same boundary as piece 2, restated: a component is a **named function declaration whose body contains at least one JSX element**. Arrow/const-assigned components are out of scope (piece 2's documented boundary), and — new refinement for this piece — a named function *without* JSX is a plain helper, not a component, and is **not** checked: `function reset() { count.set(0) }` at module scope is legal code, not a TEZ101.

Nested named function declarations with JSX are independent components (piece 2's rule) and get checked as their own bodies wherever they appear.

---

## 3. What counts as a write in the body

A write is a `CallExpression` whose callee is a static member expression `x.set` where `x` is an `IdentifierReference` resolving (via `Semantic`, the same reference→symbol path as pieces 1–2) to a `SymbolId` present in `reactive_bindings` with `ReactiveKind::Signal`. `Computed` bindings have no `.set` in the runtime's type surface and are never flagged.

The body walk covers the component function's **entire synchronous extent**:

- Included: direct statements, and statements inside `if`/`else`, loops, `try`/`catch`, `switch` — all of it runs during render.
- Included: JSX expression positions in the returned tree — `<div>{count.set(1)}</div>` executes during render and is flagged. (`onClick={() => count.set(1)}` is not — the arrow rule below catches it first.)
- Excluded: **everything inside any nested function** — arrow functions, function expressions, and nested function declarations alike. Any nested function defers execution past render; handlers and `effect()` callbacks fall out of this rule with no special-casing.

Note this skip rule is deliberately broader than piece 2's JSX collector (which descends into *anonymous* nested functions to attribute their JSX to the enclosing component). For write-checking, attribution is irrelevant — only execution timing matters — so the walker skips every nested function uniformly.

### Documented out of scope (direct-only philosophy, consistent with pieces 1–2)

- Transitive writes: the body calling a local helper that writes. Needs call-graph analysis; build it when a concrete need arises.
- IIFEs: `(() => count.set(1))()` in the body does execute during render but is skipped by the nested-function rule. Accepted false negative.
- `batch(() => …)` / `untrack(() => …)` called in the body: same shape, same accepted false negative.
- Writes through aliases (`const c = count; c.set(1)`): the alias is not in `reactive_bindings`; same dataflow boundary piece 2 drew for reads.

TEZ101 is a **compile error** (spec §7 ranges: 1xx compile/authoring). The checker reports every violation it finds (no first-error bail); how errors halt the pipeline is the caller's concern and is outside this piece.

---

## 4. Message content

The checker composes, per violation:

- `message`: names the signal variable and the enclosing component — "signal \`count\` is written during \`Counter\`'s body execution".
- `cause`: the render-loop explanation (fixed text, shared across TEZ101 instances).
- `help`: "move the write into an event handler or an effect() callback" (fixed text).
- `docs_url`: `https://tez.dev/errors/TEZ101` (URL convention established here for all future codes: `https://tez.dev/errors/<CODE>`).
- `span`: the whole `count.set(...)` call expression.

---

## 5. Test cases

Unit tests in `tez101.rs` (mirroring the fixture style of pieces 1–2):

1. Basic: write directly in a component body → one diagnostic, correct span, signal + component names in message.
2. Write inside an `if` block in the body → flagged.
3. Write inside an `onClick` arrow → not flagged.
4. Write inside an `effect(() => …)` callback in the body → not flagged.
5. `.set()` on a non-signal binding (e.g. a `Set` instance or plain object) → not flagged.
6. Aliased import (`import { signal as sig }`) producing the binding → still flagged (falls out of piece 1's map, verified anyway).
7. Write on a `computed` binding symbol → not flagged.
8. Named helper function without JSX containing a write → not flagged.
9. Two writes in one body → two diagnostics.

Snapshot: one test asserting the full `render()` output of case 1 verbatim — the seed of the spec's error-message snapshot CI gate. Changing message wording thereafter requires touching the snapshot.

---

## 6. Explicitly not in this piece

- TEZ102–104 (each is its own future piece; `Diagnostic` is the only shared artifact).
- Wiring the checker into any driver/pipeline entry point — there is no compile pipeline yet; `check_body_signal_writes` is a public library function like `classify_jsx_expressions`.
- Any change to piece 1's or piece 2's public APIs.
- Dev-overlay/terminal pretty-printing beyond the plain-text `render()`.

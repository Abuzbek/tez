# Compiler Semantic Signal Detection — Design (Phase 1, Cycle 2, Sub-cycle 2, Piece 1 of 3)

> **Status:** Approved by user 2026-07-06. Scope: `packages/compiler` — `oxc_semantic` integration + real (import-resolved) `signal()`/`computed()` binding detection. No JSX classification, no dependency graph, no `TEZ101`.
> **Precedes:** Reactivity analysis pieces 2–3 (JSX expression classification + dependency graph, `TEZ101` enforcement — see decomposition note below).
> **Depends on:** `packages/compiler`'s `parse()` (sub-cycle 1, complete) — this piece consumes the `Program` it returns.

---

## 0. Why this is its own piece

"Reactivity analysis" (Phase 1, cycle 2's sub-cycle 2) was decomposed into three ordered pieces:

1. **Semantic model + signal-binding detection** (this document) — integrate `oxc_semantic`, resolve symbol declarations/references over sub-cycle 1's `Program`, and identify which bindings are `signal()`/`computed()`-declared via real reference resolution (confirming the import source is `@tez/signals`), not a spelling heuristic.
2. JSX expression classification + per-component signal dependency graph — for each JSX expression container/attribute value, determine whether it reads a reactive binding (directly or transitively) and classify static vs. signal-driven.
3. `TEZ101` enforcement — detect a signal write occurring in a component's synchronous top-level body (not inside a nested closure/handler/effect), emit the diagnostic with span, cause, and fix suggestion.

This document covers piece 1 only.

---

## 1. Module structure

New file `packages/compiler/src/semantic.rs`. `Cargo.toml` gains one dependency: `oxc_semantic`, pinned to `0.116.0` — the same toolchain constraint from sub-cycle 1 applies (this environment's `rustc 1.91.1` cannot build newer oxc releases; see that sub-cycle's `Cargo.toml` comment).

```rust
pub enum ReactiveKind {
    Signal,
    Computed,
}

pub fn find_reactive_bindings(
    program: &Program,
    semantic: &Semantic,
) -> HashMap<SymbolId, ReactiveKind>;
```

(`SymbolId` and `Semantic` come from `oxc_semantic`; exact construction of a `Semantic` from a `Program` — e.g. via a builder — will be verified against the real crate during plan-writing, the same way sub-cycle 1's `parse()` signature was.)

This stays a separate module and return type from sub-cycle 1's `extract_structure()`/`StructuralSummary` — that function's contract is "purely syntactic facts, no semantic analysis" (stated explicitly in its own design doc), and this piece's whole point is doing real semantic resolution. Mixing the two would blur a distinction the prior design deliberately drew.

---

## 2. Detection logic

For every variable declarator whose initializer is a call expression (`let x = <call>(...)`):

1. Resolve the call's callee identifier through `Semantic`'s scope/reference graph back to its declaration.
2. If the declaration is an import binding, walk to the enclosing import declaration and check two things: the module specifier is exactly `"@tez/signals"`, and the imported name is `signal` or `computed` — following any local alias (e.g. `import { signal as sig } from "@tez/signals"` — the imported name is `signal`, the local binding name is `sig`; resolution must follow the alias correctly).
3. If both hold, record `x`'s `SymbolId` in the result map as `ReactiveKind::Signal` or `ReactiveKind::Computed` respectively.
4. Anything else is not reactive: a same-named local function, an import from a different module, a call to an unresolved/undeclared identifier, or a declarator whose initializer isn't a call at all.

This is real import-source verification, not a spelling match — the upgrade this piece exists to make over sub-cycle 1's syntactic `signal_call_sites` heuristic (which stays as-is; this piece doesn't modify or replace it, it adds a new, more rigorous fact alongside it for future pieces to use).

---

## 3. Fixtures and tests

Four fixtures drive the test suite (in `packages/compiler/tests/fixtures/`):

1. **`counter.tsx`** (reused from sub-cycle 1) — `count` (declared via `let count = signal(props.start)`) should now resolve as `ReactiveKind::Signal`.
2. **`aliased_signal.tsx`** (new) — `import { signal as sig } from "@tez/signals"; let count = sig(0);` — proves alias resolution: the local binding `count` resolves as `Signal` even though the call is spelled `sig`, not `signal`.
3. **`shadowed_signal.tsx`** (new) — a locally-declared `function signal(x: number) { return x; }` (no import from `@tez/signals` at all) used as `let count = signal(5);` — proves the import-source check correctly does NOT classify `count` as reactive. This is the sharpest test of "real semantic fact vs. spelling heuristic": sub-cycle 1's syntactic heuristic would have flagged this call as a "signal call site" purely by spelling, but this piece's import-resolved detector must not.
4. **`computed_binding.tsx`** (new) — both `signal` and `computed` imported and used (e.g. `let count = signal(1); let double = computed(() => count * 2);`) — proves both `ReactiveKind` variants resolve correctly in the same file.

Tests assert on binding *names* resolved to their `ReactiveKind`, not raw `SymbolId`s (which aren't human-legible in a test) — each fixture is small enough that a helper resolving "the symbol named `count`" to its entry in the result map is unambiguous. The exact mechanism for that name lookup (e.g. via `Semantic`'s symbol table) is an implementation detail to be nailed down against the real crate during plan-writing, not fixed here.

---

## 4. Explicitly out of scope this piece

- JSX expression classification (static vs. signal-driven) and the per-component dependency graph — piece 2.
- `TEZ101` or any other diagnostic code — piece 3.
- Any DOM/SSR codegen, napi-rs bindings, Vite integration — later sub-cycles.
- Modifying or replacing sub-cycle 1's `extract_structure()`/`StructuralSummary`/`signal_call_sites` heuristic — it stays as-is; this piece adds a new, separate, more rigorous fact.
- Detecting reactive bindings created any way other than a direct `let x = signal(...)`/`let x = computed(...)` call expression (e.g. destructuring, reassignment tracking, signals stored in objects/arrays) — out of scope until a concrete need arises.

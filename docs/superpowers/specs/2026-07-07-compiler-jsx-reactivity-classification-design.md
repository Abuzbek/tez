# Compiler JSX Reactivity Classification — Design (Phase 1, Cycle 2, Sub-cycle 2, Piece 2 of 3)

> **Status:** Approved by user 2026-07-07. Scope: `packages/compiler` — classify each JSX expression (container or attribute value) as static or signal-driven, per component. No transitive dataflow, no server-only classification, no codegen, no `TEZ101`.
> **Precedes:** Reactivity analysis piece 3 (`TEZ101` enforcement — see decomposition note below).
> **Depends on:** `packages/compiler`'s `parse()` (sub-cycle 1) and `find_reactive_bindings()` (piece 1, complete) — this piece consumes both.

---

## 0. Why this is its own piece

"Reactivity analysis" (Phase 1, cycle 2's sub-cycle 2) was decomposed into three ordered pieces:

1. Semantic model + signal-binding detection (complete) — real, import-resolved `signal()`/`computed()` binding detection via `oxc_semantic`.
2. **JSX expression classification + per-component dependency graph** (this document) — for each JSX expression (container or attribute value), determine whether it reads a reactive binding and classify static vs. signal-driven.
3. `TEZ101` enforcement — detect a signal write occurring in a component's synchronous top-level body, emit the diagnostic.

This document covers piece 2 only.

---

## 1. Module structure

New file `packages/compiler/src/reactivity.rs`. No new crate dependencies — reuses `oxc_ast`, `oxc_ast_visit`, `oxc_semantic` already present from sub-cycle 1 and piece 1.

```rust
pub enum JsxExpressionKind {
    Static,
    SignalDriven,
}

pub struct ClassifiedExpression {
    pub span: Span,
    pub kind: JsxExpressionKind,
    pub dependencies: Vec<SymbolId>,
}

pub struct ComponentReactivity {
    pub component_name: String,
    pub expressions: Vec<ClassifiedExpression>,
}

pub fn classify_jsx_expressions(
    program: &Program,
    semantic: &Semantic,
    reactive_bindings: &HashMap<SymbolId, ReactiveKind>,
) -> Vec<ComponentReactivity>;
```

`JsxExpressionKind` is deliberately two variants only — `Static`/`SignalDriven`. The spec's broader model also has a `server-only` category, but nothing in the language surface can produce it yet (`server$` doesn't exist until Phase 3). Adding a third variant no code path can ever populate would be a dead placeholder; extend this enum when Phase 3 introduces `server$`.

---

## 2. Component identification

A "component," for this piece's purposes, is any top-level function declaration whose body contains at least one JSX element — the same shape sub-cycle 1's `extract_structure` already walks via `visit_function`. `classify_jsx_expressions` walks the `Program` once, and for each such function, runs the per-component JSX walk described below, collecting results keyed by the function's name into one `ComponentReactivity` entry.

---

## 3. Per-expression classification

Within a component's body, every JSX expression container (`{expr}`, whether a direct child of an element or an attribute value, e.g. `attr={expr}`) is collected. For each one:

1. Run a sub-walk over just that expression's subtree, collecting every `IdentifierReference` inside it.
2. Resolve each collected reference through `Semantic` (the same reference→symbol resolution piece 1 uses) and check whether the resolved `SymbolId` is a key in `reactive_bindings`.
3. If the expression itself is a function or arrow-function expression (i.e., an event handler like `onClick={() => count++}`), classify it `Static` unconditionally — the closure object itself never needs a live re-binding; whatever it reads only matters when it's *called*, a separate, later concern (closure/QRL serialization boundaries, Phase 2). This is not a special case in the code — it falls out of the fact that a function expression's own top-level identifiers (the parameter list, its own body) aren't meaningfully "read" by the containing JSX position the way a value expression's identifiers are; the classifier only inspects non-function expression values for this piece.
4. Otherwise: if any collected reference resolves to a reactive binding, the expression is `SignalDriven` with `dependencies` set to those resolved `SymbolId`s (deduplicated); if none do, it's `Static` with an empty `dependencies` list.

This is **direct-reference-only** classification. An expression like `{doubled}` where `doubled` is a plain (non-`signal()`/`computed()`) local variable assigned from a signal read (`let doubled = count * 2`) is classified `Static` by this piece — tracing reactivity transitively through intermediate plain-variable dataflow is explicitly out of scope (see §5). This matches direct usage of a signal/computed value in JSX, which is the common and currently-supported case; the transitive case would need real dataflow/alias analysis, a substantially larger undertaking to build only when a concrete need arises.

---

## 4. Testing

Fixtures (new, in `packages/compiler/tests/fixtures/`), covering:

1. **The mission's `Counter` example** (reused from earlier pieces) — `{count}` classifies `SignalDriven`, depending on `count`'s `SymbolId`.
2. **A static component** — no signal reads at all; every expression (if any) classifies `Static` with empty dependencies.
3. **Mixed expressions in one component** — a component whose JSX has both a signal-reading expression and a non-reading expression as siblings, proving classification is per-expression, not per-component (e.g. a static label alongside a live counter value).
4. **A reactive attribute alongside a handler** — e.g. `<button disabled={isDisabled} onClick={() => count++}>` — proves `disabled={isDisabled}` classifies `SignalDriven` (if `isDisabled` is a signal) while `onClick={...}` classifies `Static`, using the same general rule with no handler-specific special-casing in the code.

---

## 5. Explicitly out of scope this piece

- Transitive dataflow through intermediate plain variables (direct references only, per §3).
- `server-only` classification (two-variant enum only, per §1).
- Any DOM/SSR codegen.
- `TEZ101` or any other diagnostic code — piece 3.
- Any change to piece 1's `find_reactive_bindings()`/`ReactiveKind`, or sub-cycle 1's `extract_structure()`/`StructuralSummary` — this piece only adds a new, separate module consuming their existing outputs.

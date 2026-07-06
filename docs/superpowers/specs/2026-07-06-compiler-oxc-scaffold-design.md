# Compiler oxc Scaffold — Design (Phase 1, Cycle 2, Sub-cycle 1 of 4)

> **Status:** Approved by user 2026-07-06. Scope: `packages/compiler` Rust crate — oxc-based TSX/JSX parsing + a purely-syntactic structural visitor. No semantic analysis, no codegen, no Node exposure.
> **Precedes:** Compiler MVP sub-cycles 2–4 (reactivity analysis, napi-rs + Vite integration, DOM codegen — see decomposition note below).
> **Depends on:** Nothing from earlier cycles at the code level (this is a standalone Rust crate) — sequenced after `packages/runtime-dom` (Phase 1 cycle 1, complete) per the overall Phase 1 sub-cycle ordering, since later compiler sub-cycles target that runtime.

---

## 0. Why this is its own sub-cycle

"Compiler MVP" (Phase 1, cycle 2 of the overall Phase 1 decomposition) is itself a multi-subsystem effort: standing up a Rust crate with oxc, a semantic-analysis pass, napi-rs bindings, a Vite plugin, and DOM codegen. These were decomposed into four ordered sub-cycles:

1. **Rust/oxc parsing scaffold** (this document) — parse `.tsx` via oxc, walk the AST, extract purely syntactic structure. No semantics, no codegen, no Node exposure.
2. Reactivity analysis (spec §3.1 Pass 1) — classify JSX expressions (static/signal-driven/server-only), build the per-component signal dependency graph, enforce `TEZ101` (signal write during component body).
3. napi-rs bindings + minimal Vite plugin integration — expose the Rust compiler to Node, wire a Vite plugin that intercepts `.tsx` imports (even with a passthrough/stub codegen) to prove the whole pipeline before investing in real codegen.
4. DOM codegen — emit `template()`/`insert()`/`setAttr()`/`toggleClass()`/`listen()`/`mount()` calls targeting `packages/runtime-dom` (Phase 1 cycle 1), plus control-flow/dynamic-component enforcement (`TEZ103`).

This document covers sub-cycle 1 only.

---

## 1. Crate structure

Single Rust crate at `packages/compiler/` (no Cargo workspace — YAGNI; a workspace split can happen later if a real reason emerges). `Cargo.toml` depends on:

- `oxc_allocator` — arena allocator oxc's AST is built on.
- `oxc_ast` — AST node types.
- `oxc_ast_visit` — the `Visit` trait and generated `walk_*` functions used to traverse the AST.
- `oxc_parser` — the parser itself (TSX/JSX-aware).
- `oxc_span` — source position/span types.
- `oxc_syntax` — needed for `ScopeFlags`, a parameter type on `Visit::visit_function`'s generated signature (not otherwise used by this sub-cycle).

> **Correction (found during plan-writing, verified against the actual crate):** the original draft of this section listed only `oxc_allocator`/`oxc_ast`/`oxc_parser`/`oxc_span`. Writing and compiling the plan's implementation against the real oxc `0.116.0` API (the newest version this environment's Rust toolchain can build — see the plan's Tech Stack note) surfaced two more required dependencies: `oxc_ast_visit` (the crate the `Visit` trait actually lives in) and `oxc_syntax` (for `ScopeFlags`, needed only because it appears in `visit_function`'s parameter list).

No `oxc_semantic` in this sub-cycle — scope/symbol resolution belongs to sub-cycle 2's reactivity analysis, which needs to know whether an identifier refers to a `signal()`-declared binding; that requires semantic analysis this sub-cycle deliberately does not build.

---

## 2. Public API

```rust
pub struct ParseError {
    pub message: String,
    pub span: (u32, u32), // byte offsets into the source
}

// oxc's AST is arena-allocated: `Program<'a>` borrows from an `oxc_allocator::Allocator`
// that must outlive it. The exact signature (an allocator the caller owns and passes
// in, vs. a callback-based `with_parsed<R>(source, f: impl FnOnce(&Program) -> R) -> R`
// API) depends on oxc's actual current API shape and is a plan/implementation-time
// decision, not fixed here — the intent is: parse `source`, get back either a `Program`
// (or a way to run code against one) plus its arena, or a `Vec<ParseError>`.
pub fn parse(source: &str) -> Result</* Program + owning allocator, or equivalent */, Vec<ParseError>>;

pub struct StructuralSummary {
    pub function_declarations: Vec<String>,       // names
    pub jsx_elements: Vec<JsxElementInfo>,
    pub jsx_expression_containers: usize,          // count; position not needed yet
    pub signal_call_sites: usize,                  // count of call expressions whose callee is `signal`
}

pub struct JsxElementInfo {
    pub tag_name: String,
    pub is_native: bool,       // true: lowercase (native HTML), false: uppercase (component reference)
    pub attribute_names: Vec<String>,
}

pub fn extract_structure(program: &Program) -> StructuralSummary;
```

`extract_structure` walks the AST via oxc's visitor pattern and collects purely syntactic facts — it does not resolve identifiers, does not know whether a `signal(...)` call site is actually the `signal` from `@tez/signals` (that requires import resolution, deferred to sub-cycle 2), and does not classify any expression as "reactive." A call expression counts toward `signal_call_sites` purely because its callee identifier is spelled `signal` — a heuristic, not a semantic fact, and documented as such in the code.

---

## 3. Fixtures and tests

`packages/compiler/tests/fixtures/`, four `.tsx` files:

1. `static.tsx` — a component with no signals, no JSX expression containers, only static JSX (e.g., `<div>Hello</div>`).
2. `counter.tsx` — the mission's own example: `signal()` call, a JSX expression container (`{count}`), an event handler attribute.
3. `mixed_tags.tsx` — a component whose JSX includes both a lowercase (native) and an uppercase (component-reference) tag, e.g. `<div><Profile /></div>`.
4. `malformed.tsx` — deliberately invalid syntax (e.g., an unclosed JSX tag).

`packages/compiler/tests/parse_test.rs` (or similarly named): for fixtures 1–3, calls `parse()` then `extract_structure()`, asserting on the exact expected `StructuralSummary` (function names, JSX tag names + `is_native` + attribute names, expression container count, signal call-site count). For fixture 4, asserts `parse()` returns `Err(..)` with at least one `ParseError` — not a panic.

---

## 4. Error handling

`parse()` surfaces oxc's own native syntax errors via `Result<ParseOutcome, Vec<ParseError>>` (oxc's parser is error-recovering and can report multiple syntax errors per file). No custom `TEZ###` diagnostic codes are introduced in this sub-cycle — the mission's error-code ranges start at `TEZ101` for a semantic rule (signal write during component body) that doesn't exist until sub-cycle 2's reactivity analysis. Inventing error codes before there's a rule to attach them to would be premature.

---

## 5. Explicitly out of scope this sub-cycle

- Semantic/scope analysis (`oxc_semantic`), import resolution, or any determination of whether `signal` actually refers to `@tez/signals`' export.
- Reactivity classification (static vs. signal-driven vs. server-only) — sub-cycle 2.
- Any DOM or SSR codegen — sub-cycle 4.
- napi-rs bindings or any Node/JS exposure — sub-cycle 3.
- A CLI binary for manual AST inspection (per user choice — `cargo test` only this sub-cycle).
- Any `TEZ###` error code.

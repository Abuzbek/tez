# Compiler DOM Codegen — Cycle Design & Sub-cycle 1 Detail (Phase 1, Cycle 3)

> **Status:** Approved by user 2026-07-07. Scope: `packages/compiler` — the JSX→template DOM codegen backbone (cycle 3), plus the language-surface decisions for the Vue-style directives layer (cycle 4) that codegen must reserve room for.
> **Depends on:** cycle 2's analyses (`parse()`, `extract_structure()`, `find_reactive_bindings()`, `classify_jsx_expressions()`, `check_body_signal_writes()`) and the browser-verified `@tez/runtime-dom` primitives (`template`, `insert`, `setAttr`, `toggleClass`, `listen`, `mapArray`, `mount`).

---

## 0. Cycle 3 decomposition (vertical slices, snapshot-first)

Codegen is built as a Rust pass that constructs output JavaScript as an oxc AST (`AstBuilder` + in-place `VisitMut` mutation) and prints it with `oxc_codegen` — no string-pasted JS transforms (architecture spec §3). Four sub-cycles, each its own design→plan→branch:

1. **Codegen skeleton (this document, §2):** static components only. JSX tree → hoisted `const _tN = template("…")` + clone call; runtime import injection; `compile_dom()` entry point; snapshot tests of emitted JS.
2. **Dynamic bindings + signal unwrapping:** `template_html` grows `<!>` hole markers + a hole map; classification-driven emission of `insert()` / `setAttr()` / `listen()`; rewrite signal reads (`count` → `count.get()`) and writes (`count++`, `count = x` → `.set(...)`) per spec §2.2. Extends TEZ101 to flag assignment-form writes in component bodies (today it only knows `.set()` calls — an interaction unwrapping creates). Gate: the spec §2.1 Counter compiles and runs.
3. **Control-flow + enforcement:** `<Show>` / `<For>` / `<Switch>` / `<Dynamic>` runtime components in `@tez/runtime-dom` (over `mapArray`); codegen for component-typed tags with lazily-evaluated children; `TEZ102` (spread onto native elements) and `TEZ103` (dynamic component type outside `<Dynamic>`).
4. **napi-rs + Vite plugin:** the compiler transforms `.tsx` in the playground for real; TodoMVC rewritten in JSX; Playwright e2e; js-framework-benchmark harness. Carries the cycle's main toolchain risk (napi build plumbing) — isolated last on purpose.

**Phase-1 simplification, stated once:** codegen emits direct inline handlers and plain component functions returning DOM nodes. QRL extraction, `restoreSignal`, and the SSR string-writer output are Phase 2's passes (spec §3.1 PASS 2 / output b); nothing in cycle 3 emits them.

---

## 1. Cycle 4 pre-commitment: Vue-style directives (decided now, built after cycle 3)

Decided with the user 2026-07-07 so codegen can reserve syntax space. All directives are **typed, compiler-recognized JSX attributes** that desugar into exactly the constructs cycle 3 emits — a pure compiler pass plus small runtime additions, never a new file format.

| Directive | Meaning | Desugars to |
|---|---|---|
| `v-model={sig}` | two-way binding on form elements | `value` binding + `onInput` writing the signal |
| `v-show={cond}` | visibility without unmount | a `style.display` binding effect |
| `v-if={cond}` / `v-else-if` / `v-else` | conditional regions (sibling analysis) | the `<Show>`/`<Switch>` codegen paths |
| `use:action={arg}` | custom element behaviors (mount → cleanup) | runtime `use` contract call |
| `v-for={items}` + callback children | repeat the element itself | the `<For>`/`mapArray` keyed codegen path |

**Callback `v-for` form (decided):** `<li v-for={todos}>{(todo: Todo, i: () => number) => …}</li>`. The loop parameter is annotated manually — TypeScript has no mechanism to infer a variable introduced by an attribute on an intrinsic element (generic inference only flows through component props, which is why `<For>` keeps full inference and remains available).

**Rejected: Vue's string micro-syntax** (`v-for="item in items"`). The loop variable is undefined to `tsc`; making it work requires shipping a TypeScript language-service plugin and an ESLint plugin — a large standalone project that breaks the spec's pillar 4 ("free TS tooling day one") for everyone not running the plugin. `<Show>`/`<For>` stay the spec-blessed forms; directives are sugar over the same output.

**What cycle 3 does about this:** nothing except reservation — the codegen treats any attribute named `v-*` or `use:*` as a compile error ("reserved for the directives layer, not yet supported") rather than emitting it as an HTML attribute. One error path, one test.

---

## 2. Sub-cycle 1: codegen skeleton (static components)

### 2.1 Entry point

```rust
pub fn compile_dom(source: &str) -> Result<String, CompileError>

pub enum CompileError {
    /// Source failed to parse (wraps the existing ParseError list).
    Parse(Vec<ParseError>),
    /// A construct sub-cycle 1 cannot compile yet. Temporary by design:
    /// each later sub-cycle deletes the variants it implements.
    Unsupported { span: Span, what: String },
}
```

`compile_dom` runs parse → `SemanticBuilder` → existing analyses → emit, and is the exact function the napi layer wraps in sub-cycle 4. Explicit `Unsupported` errors (never silent wrong output) for everything deferred: JSX expression containers (sub-cycle 2), component-typed tags and spread attributes (sub-cycle 3), fragment roots (multi-node templates, sub-cycle 3), `v-*`/`use:*` attributes (cycle 4, per §1).

### 2.2 Output contract (snapshot-locked)

```tsx
// input
import { something } from "./helpers";
export function Static() {
  return <div class="box">hello</div>;
}
```

```js
// output
import { template } from "@tez/runtime-dom";
import { something } from "./helpers";
const _t1 = template('<div class="box">hello</div>');
export function Static() {
  return _t1();
}
```

The HTML argument is a plain **string literal AST node** (not a template literal): `oxc_codegen` then owns JS-level escaping, so backticks or `${` inside static text can never corrupt the output. (The architecture spec's §3.2 backtick example is illustrative, not contractual; snapshots lock whatever quote style `oxc_codegen` actually prints.)

- One `_tN` const per component in source order, hoisted to module top after imports.
- Exactly one `import { template } from "@tez/runtime-dom"` injected (first statement).
- All non-component code passes through byte-faithful modulo `oxc_codegen` printing.
- Component boundary: identical to pieces 2–3 (named function whose own body — excluding nested named functions — contains a JSX element or fragment).

### 2.3 Module structure

- **`packages/compiler/src/template_html.rs`** — pure function: JSX element → static HTML string. Owns:
  - *Escaping:* text children escape `&` `<` `>`; attribute values escape `&` `"`. Emitted attributes always double-quoted.
  - *Void elements* (`area base br col embed hr img input link meta source track wbr`): no closing tag, children on them are an `Unsupported` error.
  - *Boolean attributes:* a bare JSX attribute (`<input disabled />`) emits the bare HTML attribute.
  - Returns `Result<String, CompileError>` so unsupported shapes surface with spans.
- **`packages/compiler/src/codegen.rs`** — the transform: walks the program, and for each component replaces the JSX expression with a call to its `_tN`, records the template HTML, then splices the hoisted consts + runtime import into the program and prints with `oxc_codegen`.

### 2.4 New dependency

`oxc_codegen = "0.116.0"` — same pinned family and toolchain (rustc 1.91.1) as the existing oxc crates. No other additions.

### 2.5 Test cases

Snapshot tests (full emitted-JS string equality) in `lib.rs` per crate convention:
1. `static.tsx` (existing fixture) → the §2.2 output shape.
2. Nested static elements + multiple attributes.
3. Two components in one module → `_t1`, `_t2` in source order.
4. Non-component code (imports, helpers, exports) passes through unchanged around the transform.
5. Void element (`<img src="x.png" />`) and boolean attribute (`<input disabled />`).

Unit tests in `template_html.rs` scope: escaping (`&`, `<`, `>` in text; `"`, `&` in attribute values), void-element closing rules.

Error tests: expression container → `Unsupported`; component tag → `Unsupported`; spread attribute → `Unsupported`; fragment root → `Unsupported`; `v-model` attribute → `Unsupported` (reserved wording, per §1).

### 2.6 Explicitly not in sub-cycle 1

- Holes/markers, `insert`/`setAttr`/`listen` emission, signal unwrapping — sub-cycle 2.
- Control-flow components, TEZ102/103, fragment/multi-root templates — sub-cycle 3.
- napi bindings, Vite plugin, source maps — sub-cycle 4 (source maps ride the napi integration where `oxc_codegen`'s map output gets wired through).
- Any directive behavior beyond the reserved-namespace error — cycle 4.

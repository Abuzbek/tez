# Compiler TEZ101 Enforcement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect signal writes in a component's synchronous body and emit the `TEZ101` compile diagnostic, introducing the reusable `Diagnostic` type that TEZ102–104 will share.

**Architecture:** Two new modules in the existing `tez-compiler` crate. `diagnostics.rs` defines a `Diagnostic` struct (code + span + message + cause + help + docs URL) with a stable plain-text `render()` for snapshot tests. `tez101.rs` walks the program with `oxc_ast_visit` visitors: an outer `ComponentFinder` locates named function declarations containing JSX, and an inner `WriteFinder` walks each component body — skipping every nested function — flagging `x.set(...)` calls where `x` resolves via `oxc_semantic` to a `ReactiveKind::Signal` binding from piece 1's `find_reactive_bindings()`.

**Tech Stack:** Rust (edition 2024, rustc 1.91.1), oxc 0.116.0 (pinned — do NOT `cargo add`/`cargo update`; newer oxc needs rustc 1.94+).

**Design spec:** `docs/superpowers/specs/2026-07-07-compiler-tez101-enforcement-design.md` (approved 2026-07-07).

## Global Constraints

- All work happens in the `phase1-compiler-tez101-enforcement` worktree at `.worktrees/phase1-compiler-tez101-enforcement/` (repo-relative paths below are relative to that worktree root).
- No new crate dependencies. oxc crates stay pinned at `0.116.0`.
- Tests live in `#[cfg(test)]` modules inside `packages/compiler/src/lib.rs` (crate convention — `semantic.rs`/`reactivity.rs` code is tested there too). Fixtures are `.tsx` files in `packages/compiler/tests/fixtures/` loaded with `include_str!`.
- Run tests with: `cargo test` from `packages/compiler/`.
- Diagnostic contract (architecture spec §7): code + doc URL + primary span + cause + ≥1 concrete fix. Doc URL convention: `https://tez.dev/errors/<CODE>`.
- Commit messages: imperative mood matching repo history (e.g. "Add …", "Detect …"). **Never add a `Co-Authored-By` trailer.**
- `TEZ101` message wording is fixed by the design doc §4 and locked by a snapshot test in Task 5 — copy strings exactly as written in the tasks below.

---

### Task 1: `Diagnostic` type with plain-text rendering

**Files:**
- Create: `packages/compiler/src/diagnostics.rs`
- Modify: `packages/compiler/src/lib.rs` (add module declaration + test module)

**Interfaces:**
- Consumes: `oxc_span::Span` (already a dependency).
- Produces: `pub struct Diagnostic { pub code: &'static str, pub span: Span, pub message: String, pub cause: String, pub help: String, pub docs_url: String }` with `pub fn render(&self, source: &str) -> String`. Task 2's checker constructs these; Task 5 snapshots `render()`.

- [ ] **Step 1: Write the failing tests**

Add to the top of `packages/compiler/src/lib.rs` (alongside the existing `pub mod` lines):

```rust
pub mod diagnostics;
```

Add at the bottom of `packages/compiler/src/lib.rs` (after `reactivity_tests`):

```rust
#[cfg(test)]
mod diagnostics_tests {
    use oxc_span::Span;

    use crate::diagnostics::Diagnostic;

    fn example(span: Span) -> Diagnostic {
        Diagnostic {
            code: "TEZ999",
            span,
            message: "example message".to_string(),
            cause: "example cause".to_string(),
            help: "example help".to_string(),
            docs_url: "https://tez.dev/errors/TEZ999".to_string(),
        }
    }

    #[test]
    fn render_produces_stable_plain_text_form() {
        // Offset 11 is the start of "x.set(2)" -- line 2, column 1.
        let source = "let x = 1;\nx.set(2);\n";
        let expected = "\
error[TEZ999]: example message
  --> 2:1
cause: example cause
help: example help
docs: https://tez.dev/errors/TEZ999";
        assert_eq!(example(Span::new(11, 19)).render(source), expected);
    }

    #[test]
    fn render_locates_offset_on_first_line() {
        // Offset 4 is "x" -- line 1, column 5 (columns are 1-based).
        let source = "let x = 1;";
        assert!(example(Span::new(4, 5)).render(source).contains("--> 1:5"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test diagnostics_tests` from `packages/compiler/`
Expected: COMPILE ERROR — `file not found for module diagnostics`.

- [ ] **Step 3: Write the implementation**

Create `packages/compiler/src/diagnostics.rs`:

```rust
use oxc_span::Span;

/// A compiler diagnostic satisfying the architecture spec's per-diagnostic
/// contract (spec §7): TEZ### code + doc URL + primary span + cause +
/// at least one concrete fix. `TEZ101` (tez101.rs) is the first producer;
/// TEZ102-104 will reuse this type. Deliberately NOT built yet: a code
/// registry, severity levels, multi-span labels, JSON output -- extend when
/// a second producer needs them.
#[derive(Debug)]
pub struct Diagnostic {
    pub code: &'static str,
    /// Primary span, byte offsets into the source text.
    pub span: Span,
    /// What happened, naming the specific bindings involved.
    pub message: String,
    /// Why it is an error.
    pub cause: String,
    /// At least one concrete fix.
    pub help: String,
    pub docs_url: String,
}

impl Diagnostic {
    /// Stable plain-text rendering -- the surface the error-message
    /// snapshot suite asserts against (spec §7's CI gate). Changing this
    /// format or any producer's wording requires updating snapshots.
    pub fn render(&self, source: &str) -> String {
        let (line, col) = line_col(source, self.span.start);
        format!(
            "error[{}]: {}\n  --> {}:{}\ncause: {}\nhelp: {}\ndocs: {}",
            self.code, self.message, line, col, self.cause, self.help, self.docs_url
        )
    }
}

/// 1-based (line, column) of a byte offset. Column counts chars, which
/// matches byte positions for the ASCII fixtures; full Unicode column
/// semantics are a rendering concern deferred until a real terminal
/// reporter exists.
fn line_col(source: &str, offset: u32) -> (usize, usize) {
    let prefix = &source[..offset as usize];
    let line = prefix.matches('\n').count() + 1;
    let col = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
    (line, col)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test diagnostics_tests` from `packages/compiler/`
Expected: 2 passed.

Also run the full suite to confirm nothing broke: `cargo test`
Expected: all tests pass (16 pre-existing + 2 new).

- [ ] **Step 5: Commit**

```bash
git add packages/compiler/src/diagnostics.rs packages/compiler/src/lib.rs
git commit -m "Add Diagnostic type with stable plain-text rendering"
```

---

### Task 2: TEZ101 checker — flag writes in a component body

**Files:**
- Create: `packages/compiler/src/tez101.rs`
- Create: `packages/compiler/tests/fixtures/tez101_body_write.tsx`
- Create: `packages/compiler/tests/fixtures/tez101_conditional_write.tsx`
- Create: `packages/compiler/tests/fixtures/tez101_double_write.tsx`
- Modify: `packages/compiler/src/lib.rs` (add module declaration + test module)

**Interfaces:**
- Consumes: `crate::diagnostics::Diagnostic` (Task 1); `crate::semantic::{find_reactive_bindings, ReactiveKind}` (existing, piece 1) — `find_reactive_bindings(&Program, &Semantic) -> HashMap<SymbolId, ReactiveKind>` where `ReactiveKind` is `Signal | Computed`.
- Produces: `pub fn check_body_signal_writes(program: &Program, semantic: &Semantic, reactive_bindings: &HashMap<SymbolId, ReactiveKind>) -> Vec<Diagnostic>` in `crate::tez101`, plus `pub const TEZ101_DOCS_URL: &str`. Tasks 3–5 add tests against this exact signature.

- [ ] **Step 1: Create the fixtures**

Create `packages/compiler/tests/fixtures/tez101_body_write.tsx` with EXACTLY this content (the snapshot test in Task 5 depends on `count.set(1)` sitting at line 5, column 3):

```tsx
import { signal } from "@tez/signals";

export function Counter() {
  let count = signal(0);
  count.set(1);
  return <span>{count}</span>;
}
```

Create `packages/compiler/tests/fixtures/tez101_conditional_write.tsx`:

```tsx
import { signal } from "@tez/signals";

export function Gate(props: { reset: boolean }) {
  let count = signal(0);
  if (props.reset) {
    count.set(0);
  }
  return <span>{count}</span>;
}
```

Create `packages/compiler/tests/fixtures/tez101_double_write.tsx`:

```tsx
import { signal } from "@tez/signals";

export function Doubled() {
  let a = signal(0);
  let b = signal(0);
  a.set(1);
  b.set(2);
  return (
    <span>
      {a}
      {b}
    </span>
  );
}
```

- [ ] **Step 2: Write the failing tests**

Add to the module declarations at the top of `packages/compiler/src/lib.rs`:

```rust
pub mod tez101;
```

Add at the bottom of `packages/compiler/src/lib.rs`:

```rust
#[cfg(test)]
mod tez101_tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;

    use crate::diagnostics::Diagnostic;
    use crate::semantic::find_reactive_bindings;
    use crate::tez101::check_body_signal_writes;

    /// Parses `source`, builds the semantic model and reactive-bindings map,
    /// and runs the TEZ101 checker. `Diagnostic` owns all its data (Span is
    /// Copy), so returning it past the allocator's lifetime is fine.
    fn analyze(source: &str) -> Vec<Diagnostic> {
        let allocator = Allocator::default();
        let source_type = SourceType::tsx();
        let parser_ret = Parser::new(&allocator, source, source_type).parse();
        assert!(parser_ret.errors.is_empty(), "unexpected parse errors");

        let semantic_ret = SemanticBuilder::new().build(&parser_ret.program);
        assert!(semantic_ret.errors.is_empty(), "unexpected semantic errors");
        let semantic = semantic_ret.semantic;

        let reactive_bindings = find_reactive_bindings(&parser_ret.program, &semantic);
        check_body_signal_writes(&parser_ret.program, &semantic, &reactive_bindings)
    }

    /// The source text a diagnostic's primary span points at.
    fn span_text<'a>(source: &'a str, diagnostic: &Diagnostic) -> &'a str {
        &source[diagnostic.span.start as usize..diagnostic.span.end as usize]
    }

    #[test]
    fn body_write_is_flagged() {
        let source = include_str!("../tests/fixtures/tez101_body_write.tsx");
        let diagnostics = analyze(source);
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.code, "TEZ101");
        assert!(diagnostic.message.contains("`count`"), "message must name the signal");
        assert!(diagnostic.message.contains("`Counter`"), "message must name the component");
        assert_eq!(span_text(source, diagnostic), "count.set(1)");
        assert_eq!(diagnostic.docs_url, "https://tez.dev/errors/TEZ101");
        assert!(!diagnostic.cause.is_empty());
        assert!(!diagnostic.help.is_empty());
    }

    #[test]
    fn write_inside_if_block_is_flagged() {
        let source = include_str!("../tests/fixtures/tez101_conditional_write.tsx");
        let diagnostics = analyze(source);
        assert_eq!(diagnostics.len(), 1, "an if block still runs during render");
        assert_eq!(span_text(source, &diagnostics[0]), "count.set(0)");
    }

    #[test]
    fn two_body_writes_produce_two_diagnostics() {
        let source = include_str!("../tests/fixtures/tez101_double_write.tsx");
        let diagnostics = analyze(source);
        assert_eq!(diagnostics.len(), 2, "the checker must not bail after the first violation");
        assert!(diagnostics[0].message.contains("`a`"));
        assert!(diagnostics[1].message.contains("`b`"));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test tez101_tests` from `packages/compiler/`
Expected: COMPILE ERROR — `file not found for module tez101`.

- [ ] **Step 4: Write the implementation**

Create `packages/compiler/src/tez101.rs`:

```rust
use std::collections::HashMap;

use oxc_ast::ast::{CallExpression, Expression, Function, Program};
use oxc_ast_visit::Visit;
use oxc_semantic::{Semantic, SymbolId};
use oxc_syntax::scope::ScopeFlags;

use crate::diagnostics::Diagnostic;
use crate::semantic::ReactiveKind;

pub const TEZ101_DOCS_URL: &str = "https://tez.dev/errors/TEZ101";

// Fixed per-code copy (design doc §4). Locked by the snapshot test in
// lib.rs's tez101_tests -- changing either string requires updating it.
const TEZ101_CAUSE: &str = "a component body runs on every render; this write executes each time and can re-trigger the render that ran it";
const TEZ101_HELP: &str = "move the write into an event handler or an effect() callback";

/// TEZ101: signal write during component body execution (spec §7).
///
/// A component is a named function declaration whose subtree contains at
/// least one JSX element (piece 2's boundary; named helpers without JSX are
/// legal write sites and are not checked). Within a component, every
/// statement of the synchronous body is checked -- if/loops/try included --
/// but nothing inside any nested function: a nested function defers
/// execution past render, which is exactly what makes handler and effect()
/// writes legal.
///
/// Direct-only, consistent with pieces 1-2: transitive writes via helper
/// calls, IIFEs, batch()/untrack() callbacks, and writes through aliases
/// (`const c = count; c.set(1)`) are accepted false negatives (design §3).
pub fn check_body_signal_writes(
    program: &Program<'_>,
    semantic: &Semantic<'_>,
    reactive_bindings: &HashMap<SymbolId, ReactiveKind>,
) -> Vec<Diagnostic> {
    let mut finder = ComponentFinder { semantic, reactive_bindings, diagnostics: Vec::new() };
    finder.visit_program(program);
    finder.diagnostics
}

/// Sets `found` on the first JSX element in the walked subtree. Not walking
/// past a found element is fine -- one is enough.
struct ContainsJsx {
    found: bool,
}

impl<'a> Visit<'a> for ContainsJsx {
    fn visit_jsx_element(&mut self, _it: &oxc_ast::ast::JSXElement<'a>) {
        self.found = true;
    }
}

struct ComponentFinder<'s, 'a> {
    semantic: &'s Semantic<'a>,
    reactive_bindings: &'s HashMap<SymbolId, ReactiveKind>,
    diagnostics: Vec<Diagnostic>,
}

impl<'s, 'a> Visit<'a> for ComponentFinder<'s, 'a> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            let mut probe = ContainsJsx { found: false };
            oxc_ast_visit::walk::walk_function(&mut probe, it, flags);
            if probe.found {
                let mut writes = WriteFinder {
                    semantic: self.semantic,
                    reactive_bindings: self.reactive_bindings,
                    component_name: id.name.as_str().to_string(),
                    diagnostics: Vec::new(),
                };
                // `walk_function` (not `writes.visit_function`) so the
                // component's own body is walked rather than immediately
                // hitting WriteFinder's nested-function skip.
                oxc_ast_visit::walk::walk_function(&mut writes, it, flags);
                self.diagnostics.extend(writes.diagnostics);
            }
        }
        // Always recurse: a nested named function with JSX is an independent
        // component and gets its own body check wherever it appears (its
        // body was skipped by the enclosing component's WriteFinder).
        oxc_ast_visit::walk::walk_function(self, it, flags);
    }
}

struct WriteFinder<'s, 'a> {
    semantic: &'s Semantic<'a>,
    reactive_bindings: &'s HashMap<SymbolId, ReactiveKind>,
    component_name: String,
    diagnostics: Vec<Diagnostic>,
}

impl<'s, 'a> Visit<'a> for WriteFinder<'s, 'a> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(diagnostic) = self.check_call(it) {
            self.diagnostics.push(diagnostic);
        }
        oxc_ast_visit::walk::walk_call_expression(self, it);
    }
}

impl WriteFinder<'_, '_> {
    /// `x.set(...)` where `x` resolves to a `ReactiveKind::Signal` binding.
    /// Computed bindings have no `.set` in the runtime's type surface and
    /// are never flagged.
    fn check_call(&self, call: &CallExpression) -> Option<Diagnostic> {
        let Expression::StaticMemberExpression(member) = &call.callee else { return None };
        if member.property.name.as_str() != "set" {
            return None;
        }
        let Expression::Identifier(ident) = &member.object else { return None };
        let reference_id = ident.reference_id.get()?;
        let reference = self.semantic.scoping().get_reference(reference_id);
        let symbol_id = reference.symbol_id()?;
        if self.reactive_bindings.get(&symbol_id) != Some(&ReactiveKind::Signal) {
            return None;
        }
        Some(Diagnostic {
            code: "TEZ101",
            span: call.span,
            message: format!(
                "signal `{}` is written during `{}`'s body execution",
                ident.name, self.component_name
            ),
            cause: TEZ101_CAUSE.to_string(),
            help: TEZ101_HELP.to_string(),
            docs_url: TEZ101_DOCS_URL.to_string(),
        })
    }
}
```

NOTE: this task deliberately does NOT yet skip nested functions — `WriteFinder` has no `visit_function`/`visit_arrow_function_expression` overrides. Task 3 adds them test-first. The doc comment above already describes the final behavior; the code catches up in Task 3.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test tez101_tests` from `packages/compiler/`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add packages/compiler/src/tez101.rs packages/compiler/src/lib.rs packages/compiler/tests/fixtures/tez101_body_write.tsx packages/compiler/tests/fixtures/tez101_conditional_write.tsx packages/compiler/tests/fixtures/tez101_double_write.tsx
git commit -m "Detect signal writes in component bodies and emit TEZ101"
```

---

### Task 3: Skip nested functions — handler and effect writes are legal

**Files:**
- Create: `packages/compiler/tests/fixtures/tez101_handler_write.tsx`
- Create: `packages/compiler/tests/fixtures/tez101_effect_write.tsx`
- Modify: `packages/compiler/src/tez101.rs` (add two visitor overrides to `WriteFinder`)
- Modify: `packages/compiler/src/lib.rs` (add tests to `tez101_tests`)

**Interfaces:**
- Consumes: `check_body_signal_writes` and the `analyze`/`span_text` test helpers exactly as defined in Task 2.
- Produces: no API change — behavior refinement only.

- [ ] **Step 1: Create the fixtures**

Create `packages/compiler/tests/fixtures/tez101_handler_write.tsx`:

```tsx
import { signal } from "@tez/signals";

export function Clicker() {
  let count = signal(0);
  return <button onClick={() => count.set(count.get() + 1)}>{count}</button>;
}
```

Create `packages/compiler/tests/fixtures/tez101_effect_write.tsx`:

```tsx
import { signal, effect } from "@tez/signals";

export function Logger() {
  let count = signal(0);
  let last = signal(-1);
  effect(() => {
    last.set(count.get());
  });
  return <span>{last}</span>;
}
```

- [ ] **Step 2: Write the failing tests**

Add inside `mod tez101_tests` in `packages/compiler/src/lib.rs`:

```rust
    #[test]
    fn handler_write_is_not_flagged() {
        let source = include_str!("../tests/fixtures/tez101_handler_write.tsx");
        let diagnostics = analyze(source);
        assert!(
            diagnostics.is_empty(),
            "a write inside an event-handler arrow runs after render, not during it: {diagnostics:?}"
        );
    }

    #[test]
    fn effect_callback_write_is_not_flagged() {
        let source = include_str!("../tests/fixtures/tez101_effect_write.tsx");
        let diagnostics = analyze(source);
        assert!(
            diagnostics.is_empty(),
            "a write inside an effect() callback is the documented legal home for writes: {diagnostics:?}"
        );
    }
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test tez101_tests` from `packages/compiler/`
Expected: `handler_write_is_not_flagged` and `effect_callback_write_is_not_flagged` FAIL (the walker currently descends into arrows and flags both writes); the 3 Task-2 tests still pass.

- [ ] **Step 4: Add the skip overrides**

In `packages/compiler/src/tez101.rs`, add two methods to `impl<'s, 'a> Visit<'a> for WriteFinder<'s, 'a>` (above `visit_call_expression`):

```rust
    // Any nested function -- named or anonymous, declaration or expression --
    // defers execution past render, so writes inside it are legal. This skip
    // is deliberately broader than piece 2's JSX collector (which descends
    // into anonymous callbacks to attribute their JSX): for write-checking,
    // only execution timing matters, so every nested function is skipped
    // uniformly. Nested named components are still checked -- by
    // ComponentFinder, as their own bodies.
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(
        &mut self,
        _it: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test` from `packages/compiler/`
Expected: full suite passes (all pre-existing tests + 5 tez101 tests + 2 diagnostics tests).

- [ ] **Step 6: Commit**

```bash
git add packages/compiler/src/tez101.rs packages/compiler/src/lib.rs packages/compiler/tests/fixtures/tez101_handler_write.tsx packages/compiler/tests/fixtures/tez101_effect_write.tsx
git commit -m "Skip nested functions in TEZ101: handler and effect writes are legal"
```

---

### Task 4: Regression tests — binding discrimination and component boundary

These tests are expected to pass immediately (the Task 2 implementation already checks `ReactiveKind::Signal` and JSX presence) — they lock in behavior the earlier tasks' tests don't cover, following the same "add regression test" pattern as pieces 1–2.

**Files:**
- Create: `packages/compiler/tests/fixtures/tez101_non_signal_set.tsx`
- Create: `packages/compiler/tests/fixtures/tez101_aliased_write.tsx`
- Create: `packages/compiler/tests/fixtures/tez101_computed_set.tsx`
- Create: `packages/compiler/tests/fixtures/tez101_helper_write.tsx`
- Modify: `packages/compiler/src/lib.rs` (add tests to `tez101_tests`)

**Interfaces:**
- Consumes: `analyze`/`span_text` helpers from Task 2. No production-code change expected; if any of these tests fails, the checker (Task 2/3 code in `packages/compiler/src/tez101.rs`) has a bug — fix it there, don't loosen the test.
- Produces: nothing new.

- [ ] **Step 1: Create the fixtures**

Create `packages/compiler/tests/fixtures/tez101_non_signal_set.tsx`:

```tsx
export function Tags() {
  let tags = new Map<string, string>();
  tags.set("color", "red");
  return <span>{tags.size}</span>;
}
```

Create `packages/compiler/tests/fixtures/tez101_aliased_write.tsx`:

```tsx
import { signal as sig } from "@tez/signals";

export function Aliased() {
  let count = sig(0);
  count.set(1);
  return <span>{count}</span>;
}
```

Create `packages/compiler/tests/fixtures/tez101_computed_set.tsx` (nonsensical at the type level — `Computed` has no `.set` — but the checker sees untyped source and must discriminate by `ReactiveKind`):

```tsx
import { signal, computed } from "@tez/signals";

export function Doubler() {
  let count = signal(1);
  let double = computed(() => count.get() * 2);
  double.set(4);
  return <span>{double}</span>;
}
```

Create `packages/compiler/tests/fixtures/tez101_helper_write.tsx`:

```tsx
import { signal } from "@tez/signals";

let count = signal(0);

function reset() {
  count.set(0);
}

export function Display() {
  return <span>{count}</span>;
}
```

- [ ] **Step 2: Write the tests**

Add inside `mod tez101_tests` in `packages/compiler/src/lib.rs`:

```rust
    #[test]
    fn set_call_on_non_signal_is_not_flagged() {
        let source = include_str!("../tests/fixtures/tez101_non_signal_set.tsx");
        let diagnostics = analyze(source);
        assert!(
            diagnostics.is_empty(),
            "Map.set (any non-signal .set) must not be flagged: {diagnostics:?}"
        );
    }

    #[test]
    fn aliased_signal_import_write_is_flagged() {
        let source = include_str!("../tests/fixtures/tez101_aliased_write.tsx");
        let diagnostics = analyze(source);
        assert_eq!(diagnostics.len(), 1, "aliased import must resolve via piece 1's binding map");
        assert_eq!(span_text(source, &diagnostics[0]), "count.set(1)");
    }

    #[test]
    fn set_call_on_computed_binding_is_not_flagged() {
        let source = include_str!("../tests/fixtures/tez101_computed_set.tsx");
        let diagnostics = analyze(source);
        assert!(
            diagnostics.is_empty(),
            "only ReactiveKind::Signal bindings are TEZ101 write targets: {diagnostics:?}"
        );
    }

    #[test]
    fn named_helper_without_jsx_is_not_checked() {
        let source = include_str!("../tests/fixtures/tez101_helper_write.tsx");
        let diagnostics = analyze(source);
        assert!(
            diagnostics.is_empty(),
            "a named function without JSX is a plain helper, not a component: {diagnostics:?}"
        );
    }
```

- [ ] **Step 3: Run the tests**

Run: `cargo test tez101_tests` from `packages/compiler/`
Expected: all 9 tez101 tests PASS. If any of the four new ones fails, the checker has a real bug — debug `packages/compiler/src/tez101.rs` (most likely suspects: `check_call`'s `ReactiveKind::Signal` comparison, or `ComponentFinder`'s `ContainsJsx` probe) rather than adjusting the assertions.

- [ ] **Step 4: Commit**

```bash
git add packages/compiler/src/lib.rs packages/compiler/tests/fixtures/tez101_non_signal_set.tsx packages/compiler/tests/fixtures/tez101_aliased_write.tsx packages/compiler/tests/fixtures/tez101_computed_set.tsx packages/compiler/tests/fixtures/tez101_helper_write.tsx
git commit -m "Add TEZ101 regression tests for binding discrimination and component boundary"
```

---

### Task 5: Message snapshot test and README documentation

**Files:**
- Modify: `packages/compiler/src/lib.rs` (add snapshot test to `tez101_tests`)
- Modify: `packages/compiler/README.md`

**Interfaces:**
- Consumes: `analyze` helper (Task 2), `Diagnostic::render` (Task 1), fixture `tez101_body_write.tsx` (Task 2 — the expected line 5, column 3 depends on its exact content).
- Produces: the first entry in the spec §7 error-message snapshot suite. From now on, changing TEZ101 wording or `render()` format requires updating this expected string — that friction is the point (reviews weigh message quality).

- [ ] **Step 1: Write the snapshot test**

Add inside `mod tez101_tests` in `packages/compiler/src/lib.rs`:

```rust
    /// Error-message snapshot (spec §7 CI gate): the full rendered TEZ101
    /// text, asserted verbatim. Changing the message wording or the render
    /// format is allowed -- but only deliberately, by updating this string.
    #[test]
    fn rendered_tez101_message_snapshot() {
        let source = include_str!("../tests/fixtures/tez101_body_write.tsx");
        let diagnostics = analyze(source);
        assert_eq!(diagnostics.len(), 1);
        let expected = "\
error[TEZ101]: signal `count` is written during `Counter`'s body execution
  --> 5:3
cause: a component body runs on every render; this write executes each time and can re-trigger the render that ran it
help: move the write into an event handler or an effect() callback
docs: https://tez.dev/errors/TEZ101";
        assert_eq!(diagnostics[0].render(source), expected);
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test rendered_tez101_message_snapshot` from `packages/compiler/`
Expected: PASS. (If it fails on `5:3`, the fixture drifted from Task 2's exact content; if it fails on wording, the implementation drifted from the design doc §4 copy.)

- [ ] **Step 3: Update the README**

Replace the entire content of `packages/compiler/README.md` with:

```markdown
# @tez/compiler

Phase 1 deliverable. Rust crate (oxc-based parser/semantic analysis), exposed to
the Vite plugin via napi-rs bindings. See `docs/superpowers/specs/2026-07-05-tez-architecture-design.md` §3.

Implemented so far (no pipeline/driver yet — each is a public library function):

- `parse()` — oxc-based TSX parsing (`src/lib.rs`).
- `extract_structure()` — structural summary: functions, JSX tags/attributes,
  expression containers (`src/lib.rs`).
- `find_reactive_bindings()` — import-resolved `signal()`/`computed()` binding
  detection via `oxc_semantic` (`src/semantic.rs`).
- `classify_jsx_expressions()` — per-component static vs. signal-driven
  classification of JSX expressions (`src/reactivity.rs`).
- `check_body_signal_writes()` — `TEZ101`: signal write during component body
  execution (`src/tez101.rs`), producing `Diagnostic` values (`src/diagnostics.rs`)
  that carry code + span + cause + fix + docs URL and render to a snapshot-tested
  plain-text form. Docs URL convention: `https://tez.dev/errors/<CODE>`.

All oxc crates are pinned to 0.116.0 (rustc 1.91.1 compatibility) — see
`Cargo.toml` before touching dependencies.
```

- [ ] **Step 4: Run the full suite one last time**

Run: `cargo test` from `packages/compiler/`
Expected: all 28 tests pass (16 pre-existing + 2 diagnostics + 10 tez101).

- [ ] **Step 5: Commit**

```bash
git add packages/compiler/src/lib.rs packages/compiler/README.md
git commit -m "Add TEZ101 message snapshot test and document the compiler's public surface"
```

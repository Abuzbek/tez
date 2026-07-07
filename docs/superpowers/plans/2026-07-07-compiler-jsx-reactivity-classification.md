# Compiler JSX Reactivity Classification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add per-expression JSX reactivity classification (static vs. signal-driven) and a per-component dependency list to `packages/compiler`, consuming piece 1's `find_reactive_bindings()`.

**Architecture:** A new module, `src/reactivity.rs`. One pass identifies top-level function declarations ("components") and, for each, walks its JSX tree collecting every expression container (both JSX children `{expr}` and attribute values `attr={expr}` — both route through the same `oxc_ast_visit::Visit::visit_jsx_expression_container` method). For each collected expression: if it's a function/arrow-function expression (an event handler), it's `Static` unconditionally; otherwise, every identifier reference inside it is resolved through `Semantic` and checked against the reactive-bindings map — any hit makes it `SignalDriven` with those bindings recorded as dependencies.

**Tech Stack:** Rust 2024 edition, oxc `0.116.0` (no new crate dependencies — reuses `oxc_ast`, `oxc_ast_visit`, `oxc_semantic`, `oxc_span`, `oxc_syntax` already present).

## Global Constraints

- No new oxc crate dependencies this plan — everything needed is already in `Cargo.toml` from sub-cycle 1 and piece 1.
- `JsxExpressionKind` is two variants only (`Static`, `SignalDriven`) — no `ServerOnly` placeholder; `server$` doesn't exist until Phase 3.
- Classification is direct-reference-only — no transitive dataflow through intermediate plain variables (explicitly out of scope per the design doc).
- Piece 1's `find_reactive_bindings()`/`ReactiveKind`, and sub-cycle 1's `extract_structure()`/`StructuralSummary`, stay unmodified — this plan only adds a new, separate module consuming their existing outputs.
- **Both JSX children expression containers and attribute-value expression containers route through the same `visit_jsx_expression_container` override** — do not also override `visit_jsx_attribute_value` to classify expressions, since `walk_jsx_attribute_value`'s `ExpressionContainer` arm already calls `visit_jsx_expression_container` internally; overriding both would double-classify every attribute expression. (Discovered and fixed during plan-writing verification — the first draft made exactly this mistake.)

---

## Task 1: `classify_jsx_expressions()` core implementation

**Files:**
- Create: `packages/compiler/src/reactivity.rs`
- Modify: `packages/compiler/src/lib.rs` (add `pub mod reactivity;` and the `reactivity_tests` module)

**Interfaces:**
- Consumes: `find_reactive_bindings`, `ReactiveKind` (piece 1, from `crate::semantic`).
- Produces: `pub enum JsxExpressionKind { Static, SignalDriven }` (derives `Debug, PartialEq, Eq`), `pub struct ClassifiedExpression { pub span: Span, pub kind: JsxExpressionKind, pub dependencies: Vec<SymbolId> }` (derives `Debug`), `pub struct ComponentReactivity { pub component_name: String, pub expressions: Vec<ClassifiedExpression> }` (derives `Debug, Default`), `pub fn classify_jsx_expressions<'a>(program: &Program<'a>, semantic: &Semantic<'a>, reactive_bindings: &HashMap<SymbolId, ReactiveKind>) -> Vec<ComponentReactivity>`. Tasks 2–4 add fixtures/tests exercising this SAME function — no further code changes to `reactivity.rs` are expected in Tasks 2–4 (verified during plan-writing against the real oxc `0.116.0` API, including catching and fixing a double-classification bug and a test-expectation bug before writing this plan).

- [ ] **Step 1: Write the failing test**

Add to `packages/compiler/src/lib.rs` (append at the end of the file), a new test module:
```rust
#[cfg(test)]
mod reactivity_tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;

    use crate::reactivity::{classify_jsx_expressions, ComponentReactivity, JsxExpressionKind};
    use crate::semantic::find_reactive_bindings;

    /// Parses and analyzes `source`, returning each component's name paired
    /// with a summary (kind, dependency count) per classified expression, in
    /// traversal order -- easier to assert on than raw `SymbolId`s or spans.
    fn analyze(source: &str) -> Vec<(String, Vec<(JsxExpressionKind, usize)>)> {
        let allocator = Allocator::default();
        let source_type = SourceType::tsx();
        let parser_ret = Parser::new(&allocator, source, source_type).parse();
        assert!(parser_ret.errors.is_empty(), "unexpected parse errors");

        let semantic_ret = SemanticBuilder::new().build(&parser_ret.program);
        assert!(semantic_ret.errors.is_empty(), "unexpected semantic errors");
        let semantic = semantic_ret.semantic;

        let reactive_bindings = find_reactive_bindings(&parser_ret.program, &semantic);
        let components = classify_jsx_expressions(&parser_ret.program, &semantic, &reactive_bindings);

        components
            .into_iter()
            .map(|ComponentReactivity { component_name, expressions }| {
                let summarized =
                    expressions.into_iter().map(|e| (e.kind, e.dependencies.len())).collect();
                (component_name, summarized)
            })
            .collect()
    }

    #[test]
    fn counter_expressions_classify_correctly() {
        let source = include_str!("../tests/fixtures/counter.tsx");
        let components = analyze(source);
        assert_eq!(components.len(), 1);
        let (name, expressions) = &components[0];
        assert_eq!(name, "Counter");
        // Attributes are visited before children: onClick={() => count++}
        // (Static, a handler) comes before {count} (SignalDriven).
        assert_eq!(expressions.len(), 2);
        assert_eq!(expressions[0], (JsxExpressionKind::Static, 0));
        assert_eq!(expressions[1], (JsxExpressionKind::SignalDriven, 1));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/compiler && cargo test`
Expected: FAIL — compile error, `unresolved module 'reactivity'` (or similar), since `src/reactivity.rs` doesn't exist yet.

- [ ] **Step 3: Implement `src/reactivity.rs`**

`packages/compiler/src/reactivity.rs`:
```rust
use std::collections::HashMap;

use oxc_ast::ast::{JSXExpression, Program};
use oxc_ast_visit::Visit;
use oxc_semantic::{Semantic, SymbolId};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;

use crate::semantic::ReactiveKind;

#[derive(Debug, PartialEq, Eq)]
pub enum JsxExpressionKind {
    Static,
    SignalDriven,
}

#[derive(Debug)]
pub struct ClassifiedExpression {
    pub span: Span,
    pub kind: JsxExpressionKind,
    pub dependencies: Vec<SymbolId>,
}

#[derive(Debug, Default)]
pub struct ComponentReactivity {
    pub component_name: String,
    pub expressions: Vec<ClassifiedExpression>,
}

/// Collects every `IdentifierReference` inside a single JSX expression's
/// subtree, so the caller can check each against the reactive-bindings map.
struct IdentifierCollector<'s, 'a> {
    semantic: &'s Semantic<'a>,
    symbol_ids: Vec<SymbolId>,
}

impl<'s, 'a> Visit<'a> for IdentifierCollector<'s, 'a> {
    fn visit_identifier_reference(&mut self, it: &oxc_ast::ast::IdentifierReference<'a>) {
        if let Some(reference_id) = it.reference_id.get() {
            let reference = self.semantic.scoping().get_reference(reference_id);
            if let Some(symbol_id) = reference.symbol_id() {
                self.symbol_ids.push(symbol_id);
            }
        }
    }
}

fn classify_expression(
    expr: &JSXExpression,
    semantic: &Semantic,
    reactive_bindings: &HashMap<SymbolId, ReactiveKind>,
) -> ClassifiedExpression {
    let span = expr.span();

    // Event handlers (function/arrow-function expression values) are always
    // static: the closure object itself never needs a live re-binding.
    if matches!(
        expr,
        JSXExpression::ArrowFunctionExpression(_) | JSXExpression::FunctionExpression(_)
    ) {
        return ClassifiedExpression { span, kind: JsxExpressionKind::Static, dependencies: Vec::new() };
    }

    let mut collector = IdentifierCollector { semantic, symbol_ids: Vec::new() };
    collector.visit_jsx_expression(expr);

    let mut dependencies: Vec<SymbolId> = collector
        .symbol_ids
        .into_iter()
        .filter(|symbol_id| reactive_bindings.contains_key(symbol_id))
        .collect();
    dependencies.sort();
    dependencies.dedup();

    let kind =
        if dependencies.is_empty() { JsxExpressionKind::Static } else { JsxExpressionKind::SignalDriven };

    ClassifiedExpression { span, kind, dependencies }
}

struct ComponentCollector<'s, 'a> {
    semantic: &'s Semantic<'a>,
    reactive_bindings: &'s HashMap<SymbolId, ReactiveKind>,
    components: Vec<ComponentReactivity>,
}

impl<'s, 'a> Visit<'a> for ComponentCollector<'s, 'a> {
    fn visit_function(&mut self, it: &oxc_ast::ast::Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            let mut jsx_collector = JsxExpressionCollector {
                semantic: self.semantic,
                reactive_bindings: self.reactive_bindings,
                expressions: Vec::new(),
            };
            jsx_collector.visit_function(it, flags);
            self.components.push(ComponentReactivity {
                component_name: id.name.as_str().to_string(),
                expressions: jsx_collector.expressions,
            });
            return;
        }
        oxc_ast_visit::walk::walk_function(self, it, flags);
    }
}

struct JsxExpressionCollector<'s, 'a> {
    semantic: &'s Semantic<'a>,
    reactive_bindings: &'s HashMap<SymbolId, ReactiveKind>,
    expressions: Vec<ClassifiedExpression>,
}

impl<'s, 'a> Visit<'a> for JsxExpressionCollector<'s, 'a> {
    // Both JSX children expression containers (`{expr}`) and JSX attribute
    // value expression containers (`attr={expr}`) route through this same
    // method -- `walk_jsx_attribute_value`'s `ExpressionContainer` arm calls
    // `visit_jsx_expression_container` internally, so overriding it here
    // alone covers both cases without double-classifying attribute values.
    fn visit_jsx_expression_container(&mut self, it: &oxc_ast::ast::JSXExpressionContainer<'a>) {
        self.expressions.push(classify_expression(&it.expression, self.semantic, self.reactive_bindings));
        oxc_ast_visit::walk::walk_jsx_expression_container(self, it);
    }
}

pub fn classify_jsx_expressions<'a>(
    program: &Program<'a>,
    semantic: &Semantic<'a>,
    reactive_bindings: &HashMap<SymbolId, ReactiveKind>,
) -> Vec<ComponentReactivity> {
    let mut collector = ComponentCollector { semantic, reactive_bindings, components: Vec::new() };
    collector.visit_program(program);
    collector.components
}
```

- [ ] **Step 4: Add the module declaration to `lib.rs`**

Add this line near the top of `packages/compiler/src/lib.rs`, alongside the existing `pub mod semantic;`:
```rust
pub mod reactivity;
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS — the new `reactivity_tests::counter_expressions_classify_correctly` test passes, plus all 10 pre-existing tests still pass (11 total).

- [ ] **Step 6: Commit**

```bash
git add packages/compiler
git commit -m "Add classify_jsx_expressions() for per-expression JSX reactivity classification"
```

---

## Task 2: Static-component confirmation

**Files:**
- Modify: `packages/compiler/src/lib.rs` (add one test)

**Interfaces:**
- Consumes: `classify_jsx_expressions`, `ComponentReactivity`, `analyze` (Task 1). No changes to `reactivity.rs` expected — this task proves that a component with no expression containers at all produces an empty expression list, reusing the existing `static.tsx` fixture from sub-cycle 1.

- [ ] **Step 1: Write the test**

Add to `reactivity_tests` in `packages/compiler/src/lib.rs`:
```rust
    #[test]
    fn static_component_has_no_expressions() {
        let source = include_str!("../tests/fixtures/static.tsx");
        let components = analyze(source);
        assert_eq!(components.len(), 1);
        let (name, expressions) = &components[0];
        assert_eq!(name, "Static");
        assert!(expressions.is_empty());
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS immediately — no code changes needed. `static.tsx` (`export function Static() { return <div>Hello</div>; }`) has no JSX expression containers at all (`Hello` is plain text, not `{...}`), so `classify_jsx_expressions` correctly produces a `ComponentReactivity` with an empty `expressions` list. If this test fails, report BLOCKED rather than modifying the test or `reactivity.rs`.

- [ ] **Step 3: Commit**

```bash
git add packages/compiler
git commit -m "Add regression test confirming a static component has no classified expressions"
```

---

## Task 3: Mixed static/signal-driven expressions in one component

**Files:**
- Create: `packages/compiler/tests/fixtures/mixed_expressions.tsx`
- Modify: `packages/compiler/src/lib.rs` (add one test)

**Interfaces:**
- Consumes: `classify_jsx_expressions`, `ComponentReactivity`, `JsxExpressionKind`, `analyze` (Task 1). No changes to `reactivity.rs` expected — this task proves classification is per-expression, not per-component.

- [ ] **Step 1: Create the mixed-expressions fixture**

`packages/compiler/tests/fixtures/mixed_expressions.tsx`:
```tsx
import { signal } from "@tez/signals";

export function Labeled() {
  let count = signal(0);
  let label = "Count:";
  return (
    <div>
      <span>{label}</span>
      <span>{count}</span>
    </div>
  );
}
```

- [ ] **Step 2: Write the test**

Add to `reactivity_tests` in `packages/compiler/src/lib.rs`:
```rust
    #[test]
    fn mixed_expressions_classify_independently() {
        let source = include_str!("../tests/fixtures/mixed_expressions.tsx");
        let components = analyze(source);
        assert_eq!(components.len(), 1);
        let (name, expressions) = &components[0];
        assert_eq!(name, "Labeled");
        assert_eq!(expressions.len(), 2);
        assert_eq!(expressions[0], (JsxExpressionKind::Static, 0));
        assert_eq!(expressions[1], (JsxExpressionKind::SignalDriven, 1));
    }
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS immediately — no code changes needed. `{label}` reads a plain (non-signal) variable, so it classifies `Static`; `{count}` reads the `signal()`-declared `count`, so it classifies `SignalDriven` — both within the same `Labeled` component, proving per-expression granularity. If this test fails, report BLOCKED rather than modifying the test or `reactivity.rs`.

- [ ] **Step 4: Commit**

```bash
git add packages/compiler
git commit -m "Add regression test for per-expression classification within one component"
```

---

## Task 4: Reactive attribute alongside an event handler

**Files:**
- Create: `packages/compiler/tests/fixtures/reactive_attribute.tsx`
- Modify: `packages/compiler/src/lib.rs` (add one test)

**Interfaces:**
- Consumes: `classify_jsx_expressions`, `ComponentReactivity`, `JsxExpressionKind`, `analyze` (Task 1). No changes to `reactivity.rs` expected — this task proves a reactive attribute value classifies `SignalDriven` while a sibling event-handler attribute classifies `Static`, via the same general rule with no handler-specific special-casing in the code.

- [ ] **Step 1: Create the reactive-attribute fixture**

`packages/compiler/tests/fixtures/reactive_attribute.tsx`:
```tsx
import { signal } from "@tez/signals";

export function ToggleButton() {
  let count = signal(0);
  let isDisabled = signal(false);
  return (
    <button disabled={isDisabled} onClick={() => count++}>
      {count}
    </button>
  );
}
```

- [ ] **Step 2: Write the test**

Add to `reactivity_tests` in `packages/compiler/src/lib.rs`:
```rust
    #[test]
    fn reactive_attribute_and_handler_classify_correctly() {
        let source = include_str!("../tests/fixtures/reactive_attribute.tsx");
        let components = analyze(source);
        assert_eq!(components.len(), 1);
        let (name, expressions) = &components[0];
        assert_eq!(name, "ToggleButton");
        assert_eq!(expressions.len(), 3);
        // disabled={isDisabled}
        assert_eq!(expressions[0], (JsxExpressionKind::SignalDriven, 1));
        // onClick={() => count++}
        assert_eq!(expressions[1], (JsxExpressionKind::Static, 0));
        // {count}
        assert_eq!(expressions[2], (JsxExpressionKind::SignalDriven, 1));
    }
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS — 14 tests total (10 pre-existing from sub-cycle 1 + piece 1, plus this plan's 4 new tests). No code changes needed in `reactivity.rs` — `disabled={isDisabled}` reads the `signal()`-declared `isDisabled` directly (classifies `SignalDriven`), `onClick={() => count++}` is an arrow-function expression (classifies `Static` via the unconditional handler rule, even though its body reads `count`), and `{count}` classifies `SignalDriven` — all via the same general rule.

- [ ] **Step 4: Commit**

```bash
git add packages/compiler
git commit -m "Add regression test for reactive attribute alongside an event handler"
```

---

## Piece Gate Checklist

- [ ] `classify_jsx_expressions()` correctly identifies components (top-level function declarations) and classifies every JSX expression container (children and attribute values alike) as `Static` or `SignalDriven` (Task 1).
- [ ] Event-handler attribute values (function/arrow-function expressions) classify `Static` unconditionally, via the same general rule, with no handler-specific special-casing (Task 1, confirmed by Task 4).
- [ ] A component with no expression containers produces an empty classification list (Task 2).
- [ ] Classification is per-expression, not per-component — a static and a signal-driven expression can coexist in one component (Task 3).
- [ ] A reactive attribute value and a sibling event handler classify correctly and independently (Task 4).
- [ ] Piece 1's `find_reactive_bindings()`/`ReactiveKind` and sub-cycle 1's `extract_structure()`/`StructuralSummary` remain untouched; all 10 of their original tests still pass alongside this plan's 4 new tests (14 total).
- [ ] `JsxExpressionKind` stays two variants (`Static`, `SignalDriven`) — no `ServerOnly` placeholder.

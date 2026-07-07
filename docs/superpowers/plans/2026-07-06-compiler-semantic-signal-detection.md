# Compiler Semantic Signal Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real (import-resolved) `signal()`/`computed()` binding detection to `packages/compiler`, via `oxc_semantic`, replacing nothing from sub-cycle 1 but adding a new, more rigorous semantic fact alongside its syntactic heuristic.

**Architecture:** A new module, `src/semantic.rs`, builds an `oxc_semantic::Semantic` model over the `Program` from sub-cycle 1's `parse()`, then walks every variable declarator via `oxc_ast_visit::Visit`. For each `let x = <call>(...)`, it resolves the call's callee through the semantic model's reference/symbol graph back to its declaration; if that declaration is an `ImportSpecifier` whose enclosing `ImportDeclaration`'s source is exactly `"@tez/signals"` and whose imported name is `signal` or `computed` (alias-resolved), `x` is recorded as reactive.

**Tech Stack:** Rust 2024 edition, oxc `0.116.0` (adding `oxc_semantic` to the existing pinned set from sub-cycle 1).

## Global Constraints

- All oxc crates (including the new `oxc_semantic`) pinned to exactly `0.116.0` — this environment's `rustc 1.91.1` cannot build newer oxc releases (they require `rustc 1.94.0+`). This is already documented in `packages/compiler/Cargo.toml`'s comment from sub-cycle 1; extend that pin to `oxc_semantic`, don't let `cargo add` upgrade any of them.
- Sub-cycle 1's `extract_structure()`/`StructuralSummary`/`signal_call_sites` stay unmodified — this plan adds a new, separate module and function, it does not touch `extract_structure`'s existing behavior.
- No JSX expression classification, no per-component dependency graph, no `TEZ101` diagnostic — those are pieces 2–3 of this sub-cycle, not this plan.
- Detection only covers `let x = signal(...)`/`let x = computed(...)` direct call-expression initializers — no destructuring, no reassignment tracking, no signals stored in objects/arrays (design doc §4, explicitly out of scope).

---

## Task 1: `oxc_semantic` integration + `find_reactive_bindings()` core implementation

**Files:**
- Modify: `packages/compiler/Cargo.toml`
- Create: `packages/compiler/src/semantic.rs`
- Modify: `packages/compiler/src/lib.rs` (add `pub mod semantic;`)
- Modify: `packages/compiler/tests/fixtures/counter.tsx` (add the missing `import` statement — see below)

**Interfaces:**
- Consumes: `Program` from `crate::parse()` (sub-cycle 1).
- Produces: `pub enum ReactiveKind { Signal, Computed }` (derives `Debug, PartialEq, Eq`), `pub fn find_reactive_bindings<'a>(program: &Program<'a>, semantic: &Semantic<'a>) -> HashMap<SymbolId, ReactiveKind>`. Tasks 2–4 add fixtures/tests exercising this SAME function — no further code changes to `semantic.rs` are expected in Tasks 2–4, since this task's implementation already correctly handles aliasing, false-positive rejection, and `computed()` (verified during plan-writing against the real oxc `0.116.0` API in a standalone scratch project covering all four planned fixture patterns).

- [ ] **Step 1: Extend the oxc version pin to include `oxc_semantic`**

`packages/compiler/Cargo.toml` (full file):
```toml
[package]
name = "tez-compiler"
version = "0.0.0"
edition = "2024"
publish = false

# All oxc crates pinned to 0.116.0 -- the newest version buildable with this
# environment's Rust toolchain (rustc 1.91.1); newer oxc releases require
# rustc 1.94.0+. Do not `cargo add`/`cargo update` these without also
# upgrading the Rust toolchain, or the build will silently try to resolve
# an incompatible version.
[dependencies]
oxc_allocator = "0.116.0"
oxc_ast = "0.116.0"
oxc_ast_visit = "0.116.0"
oxc_parser = "0.116.0"
oxc_semantic = "0.116.0"
oxc_span = "0.116.0"
oxc_syntax = "0.116.0"
```

- [ ] **Step 2: Add the missing import to the reused `counter.tsx` fixture**

The existing `packages/compiler/tests/fixtures/counter.tsx` (from sub-cycle 1) has no `import` statement — sub-cycle 1 only needed a syntactic spelling match, so it didn't matter. This sub-cycle needs a real import to resolve. Update the fixture:

`packages/compiler/tests/fixtures/counter.tsx`:
```tsx
import { signal } from "@tez/signals";

export function Counter(props: { start: number }) {
  let count = signal(props.start);
  return (
    <button onClick={() => count++}>{count}</button>
  );
}
```

This is safe: sub-cycle 1's own `counter_component_full_structure` test (in `src/lib.rs`) only asserts on `function_names`, `jsx_elements`, `jsx_expression_containers`, and `signal_call_sites` — none of which change by adding a top-level import statement before the function.

- [ ] **Step 3: Write the failing test**

Add to `packages/compiler/src/lib.rs`, a new test module (append at the end of the file):
```rust
#[cfg(test)]
mod semantic_tests {
    use std::collections::HashMap;

    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;

    use crate::semantic::{find_reactive_bindings, ReactiveKind};

    /// Parses `source`, builds a semantic model, and returns reactive bindings
    /// keyed by binding NAME (not `SymbolId`, which isn't meaningful across
    /// the allocator's lifetime once this function returns).
    fn analyze_reactive_bindings(source: &str) -> HashMap<String, ReactiveKind> {
        let allocator = Allocator::default();
        let source_type = SourceType::tsx();
        let parser_ret = Parser::new(&allocator, source, source_type).parse();
        assert!(parser_ret.errors.is_empty(), "unexpected parse errors");

        let semantic_ret = SemanticBuilder::new().build(&parser_ret.program);
        assert!(semantic_ret.errors.is_empty(), "unexpected semantic errors");
        let semantic = semantic_ret.semantic;

        let bindings = find_reactive_bindings(&parser_ret.program, &semantic);

        bindings
            .into_iter()
            .map(|(symbol_id, kind)| (semantic.scoping().symbol_name(symbol_id).to_string(), kind))
            .collect()
    }

    #[test]
    fn counter_signal_binding_is_detected() {
        let source = include_str!("../tests/fixtures/counter.tsx");
        let bindings = analyze_reactive_bindings(source);
        assert_eq!(bindings.get("count"), Some(&ReactiveKind::Signal));
    }
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cd packages/compiler && cargo test`
Expected: FAIL — compile error, `use of undeclared crate or module 'semantic'` (or similar), since `src/semantic.rs` doesn't exist yet.

- [ ] **Step 5: Implement `src/semantic.rs`**

`packages/compiler/src/semantic.rs`:
```rust
use std::collections::HashMap;

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, Program, VariableDeclarator};
use oxc_ast_visit::Visit;
use oxc_semantic::{Semantic, SymbolId};

#[derive(Debug, PartialEq, Eq)]
pub enum ReactiveKind {
    Signal,
    Computed,
}

struct ReactiveBindingCollector<'s, 'a> {
    semantic: &'s Semantic<'a>,
    result: HashMap<SymbolId, ReactiveKind>,
}

impl<'s, 'a> Visit<'a> for ReactiveBindingCollector<'s, 'a> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        check_declarator(it, self.semantic, &mut self.result);
        oxc_ast_visit::walk::walk_variable_declarator(self, it);
    }
}

pub fn find_reactive_bindings<'a>(
    program: &Program<'a>,
    semantic: &Semantic<'a>,
) -> HashMap<SymbolId, ReactiveKind> {
    let mut collector = ReactiveBindingCollector { semantic, result: HashMap::new() };
    collector.visit_program(program);
    collector.result
}

fn check_declarator(
    declarator: &VariableDeclarator,
    semantic: &Semantic,
    result: &mut HashMap<SymbolId, ReactiveKind>,
) {
    let Some(Expression::CallExpression(call)) = &declarator.init else { return };
    let Expression::Identifier(callee) = &call.callee else { return };
    let Some(reference_id) = callee.reference_id.get() else { return };
    let reference = semantic.scoping().get_reference(reference_id);
    let Some(symbol_id) = reference.symbol_id() else { return };

    let decl_node_id = semantic.scoping().symbol_declaration(symbol_id);
    let AstKind::ImportSpecifier(spec) = semantic.nodes().kind(decl_node_id) else { return };
    let imported_name = spec.imported.name();

    let kind = match imported_name.as_str() {
        "signal" => ReactiveKind::Signal,
        "computed" => ReactiveKind::Computed,
        _ => return,
    };

    // Confirm the enclosing ImportDeclaration's source is "@tez/signals" --
    // this is the real semantic check, not just a spelling match.
    let import_decl_node_id = semantic.nodes().parent_id(decl_node_id);
    let AstKind::ImportDeclaration(import_decl) = semantic.nodes().kind(import_decl_node_id) else {
        return;
    };
    if import_decl.source.value.as_str() != "@tez/signals" {
        return;
    }

    if let Some(binding_id) = declarator.id.get_binding_identifier() {
        if let Some(binding_symbol_id) = binding_id.symbol_id.get() {
            result.insert(binding_symbol_id, kind);
        }
    }
}
```

- [ ] **Step 6: Add the module declaration to `lib.rs`**

Add this line near the top of `packages/compiler/src/lib.rs` (alongside the other `use`/`mod` statements, before the existing `use` lines):
```rust
pub mod semantic;
```

- [ ] **Step 7: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS — the new `semantic_tests::counter_signal_binding_is_detected` test passes, plus all 6 pre-existing tests still pass (7 total).

- [ ] **Step 8: Commit**

```bash
git add packages/compiler
git commit -m "Add oxc_semantic integration and real signal()/computed() binding detection"
```

---

## Task 2: Aliased import resolution

**Files:**
- Create: `packages/compiler/tests/fixtures/aliased_signal.tsx`
- Modify: `packages/compiler/src/lib.rs` (add one test)

**Interfaces:**
- Consumes: `find_reactive_bindings`, `ReactiveKind`, `analyze_reactive_bindings` (Task 1). No changes to `semantic.rs` expected — this task proves an existing property of Task 1's implementation (alias resolution follows the *imported* name, not the *local* binding name).

- [ ] **Step 1: Create the aliased-import fixture**

`packages/compiler/tests/fixtures/aliased_signal.tsx`:
```tsx
import { signal as sig } from "@tez/signals";

export function AliasedCounter() {
  let count = sig(0);
  return <span>{count}</span>;
}
```

- [ ] **Step 2: Write the test**

Add to `semantic_tests` in `packages/compiler/src/lib.rs`:
```rust
    #[test]
    fn aliased_import_resolves_to_signal() {
        let source = include_str!("../tests/fixtures/aliased_signal.tsx");
        let bindings = analyze_reactive_bindings(source);
        assert_eq!(bindings.get("count"), Some(&ReactiveKind::Signal));
    }
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS immediately — no code changes needed in `semantic.rs`. This confirms (rather than requires new logic for) that `spec.imported.name()` correctly reads the *original* imported name (`signal`) even when the local binding is aliased (`sig`), since `check_declarator` matches on `imported_name`, not the local binding's own name. If this test fails, that indicates a real gap in Task 1's implementation — do not modify the test to make it pass; investigate and report BLOCKED with the failure output instead of guessing a fix.

- [ ] **Step 4: Commit**

```bash
git add packages/compiler
git commit -m "Add regression test for aliased signal import resolution"
```

---

## Task 3: Local-shadow false-positive rejection

**Files:**
- Create: `packages/compiler/tests/fixtures/shadowed_signal.tsx`
- Modify: `packages/compiler/src/lib.rs` (add one test)

**Interfaces:**
- Consumes: `find_reactive_bindings`, `ReactiveKind`, `analyze_reactive_bindings` (Task 1). No changes to `semantic.rs` expected — this task proves the import-source check correctly rejects a same-named local declaration that is NOT imported from `@tez/signals`.

- [ ] **Step 1: Create the local-shadow fixture**

`packages/compiler/tests/fixtures/shadowed_signal.tsx`:
```tsx
function signal(x: number): number {
  return x;
}

export function NotReallyReactive() {
  let count = signal(5);
  return <span>{count}</span>;
}
```

- [ ] **Step 2: Write the test**

Add to `semantic_tests` in `packages/compiler/src/lib.rs`:
```rust
    #[test]
    fn locally_declared_signal_function_is_not_detected_as_reactive() {
        let source = include_str!("../tests/fixtures/shadowed_signal.tsx");
        let bindings = analyze_reactive_bindings(source);
        assert_eq!(
            bindings.get("count"),
            None,
            "a call to a locally-declared function named 'signal' (not imported from @tez/signals) must not be detected as reactive"
        );
    }
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS immediately — no code changes needed. This confirms `check_declarator`'s `AstKind::ImportSpecifier(spec) = semantic.nodes().kind(decl_node_id) else { return };` line correctly returns early when the resolved declaration is a local `Function` (not an `ImportSpecifier`) — the exact property sub-cycle 1's syntactic heuristic could NOT provide (it would have counted this `signal(5)` call as a "signal call site" purely by spelling). If this test fails, report BLOCKED rather than modifying the test.

- [ ] **Step 4: Commit**

```bash
git add packages/compiler
git commit -m "Add regression test rejecting locally-shadowed signal function"
```

---

## Task 4: `computed()` detection

**Files:**
- Create: `packages/compiler/tests/fixtures/computed_binding.tsx`
- Modify: `packages/compiler/src/lib.rs` (add one test)

**Interfaces:**
- Consumes: `find_reactive_bindings`, `ReactiveKind`, `analyze_reactive_bindings` (Task 1). No changes to `semantic.rs` expected — this task proves both `ReactiveKind` variants resolve correctly in the same file.

- [ ] **Step 1: Create the computed-binding fixture**

`packages/compiler/tests/fixtures/computed_binding.tsx`:
```tsx
import { signal, computed } from "@tez/signals";

export function Doubler() {
  let count = signal(1);
  let double = computed(() => count * 2);
  return <span>{double}</span>;
}
```

- [ ] **Step 2: Write the test**

Add to `semantic_tests` in `packages/compiler/src/lib.rs`:
```rust
    #[test]
    fn both_signal_and_computed_bindings_are_detected() {
        let source = include_str!("../tests/fixtures/computed_binding.tsx");
        let bindings = analyze_reactive_bindings(source);
        assert_eq!(bindings.get("count"), Some(&ReactiveKind::Signal));
        assert_eq!(bindings.get("double"), Some(&ReactiveKind::Computed));
    }
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS — 10 tests total (6 pre-existing from sub-cycle 1 + 4 from this plan's Tasks 1–4). No code changes needed in `semantic.rs` — this confirms `check_declarator`'s `match imported_name.as_str() { "signal" => ..., "computed" => ..., _ => return }` branch already handles both variants from the same imported module in one pass.

- [ ] **Step 4: Commit**

```bash
git add packages/compiler
git commit -m "Add regression test for computed() binding detection"
```

---

## Piece Gate Checklist

- [ ] `oxc_semantic` added and pinned to `0.116.0`, matching the existing sub-cycle-1 pin comment (Task 1).
- [ ] `find_reactive_bindings()` correctly resolves `signal()`/`computed()` bindings via real import-source verification, not a spelling heuristic (Task 1).
- [ ] Aliased imports resolve by their *imported* name, not their local binding name (Task 2).
- [ ] A locally-declared function that happens to be spelled `signal` is correctly rejected as non-reactive (Task 3).
- [ ] Both `signal()` and `computed()` bindings resolve correctly in the same file (Task 4).
- [ ] Sub-cycle 1's `extract_structure()`/`StructuralSummary`/`signal_call_sites` remain untouched and all 6 of its original tests still pass alongside this plan's 4 new tests (10 total).

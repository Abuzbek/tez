# Compiler oxc Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up `packages/compiler` as a real Rust crate that parses `.tsx` source via oxc and extracts purely-syntactic structural facts (function names, JSX elements/attributes, expression containers, `signal(...)` call sites) via an AST visitor — no semantics, no codegen, no Node exposure.

**Architecture:** A single Rust crate, two public functions: `parse(source: &str) -> ParseResult` (oxc-based, arena-allocated) and `extract_structure(program: &Program) -> StructuralSummary` (an `oxc_ast_visit::Visit` implementation collecting structural facts). Four `.tsx` fixtures drive the test suite.

**Tech Stack:** Rust 2024 edition, `oxc_allocator`/`oxc_ast`/`oxc_ast_visit`/`oxc_parser`/`oxc_span`/`oxc_syntax` **all pinned to `0.116.0`** — this is a hard constraint of the currently-installed Rust toolchain (`rustc 1.91.1`), which cannot compile oxc versions newer than 0.116.x (they require `rustc 1.94.0`+). `cargo add` will otherwise silently try to resolve a newer, incompatible version and fail to build.

## Global Constraints

- `packages/compiler` has no dependency on `packages/runtime-dom` or `packages/signals` this cycle — it's a standalone Rust crate (repo layout rule: "compiler never imports runtime code").
- No `TEZ###` error codes this cycle — those begin at `TEZ101`, a semantic rule that doesn't exist until the next sub-cycle (reactivity analysis).
- No `oxc_semantic` dependency — scope/symbol resolution is out of scope; `extract_structure` only reports syntactic facts (e.g., "a call expression whose callee is spelled `signal`" is a heuristic, not confirmation that it's `@tez/signals`' export).
- All oxc crate versions pinned to `0.116.0` (see Tech Stack above) — do not let `cargo add` upgrade them.

---

## Task 1: Crate scaffold + `parse()`

**Files:**
- Create: `packages/compiler/Cargo.toml`
- Create: `packages/compiler/src/lib.rs`
- Create: `packages/compiler/tests/fixtures/malformed.tsx`
- Modify: `.gitignore` (add `target/`)

**Interfaces:**
- Produces: `pub struct ParseError { pub message: String }`, `pub fn parse(allocator: &Allocator, source: &str) -> Result<Program, Vec<ParseError>>`. Task 2's `extract_structure` consumes the `Program` this returns. Callers own the `Allocator` (oxc's AST is arena-allocated and borrows from it — see the `Allocator::default()` pattern in the tests below).

- [ ] **Step 1: Add `target/` to the root `.gitignore`**

`.gitignore`:
```
node_modules/
dist/
coverage/
.turbo/
*.log
.DS_Store
.worktrees/
target/
```

- [ ] **Step 2: Create the crate manifest**

`packages/compiler/Cargo.toml`:
```toml
[package]
name = "tez-compiler"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
oxc_allocator = "0.116.0"
oxc_ast = "0.116.0"
oxc_ast_visit = "0.116.0"
oxc_parser = "0.116.0"
oxc_span = "0.116.0"
oxc_syntax = "0.116.0"
```

- [ ] **Step 3: Write the failing test for `parse()`**

`packages/compiler/tests/fixtures/malformed.tsx`:
```tsx
export function Broken() {
  return <div>;
}
```

Append to `packages/compiler/src/lib.rs` (create the file first with just this test module — the implementation comes in Step 5):

```rust
#[cfg(test)]
mod parse_tests {
    use oxc_allocator::Allocator;

    #[test]
    fn malformed_source_returns_errors_not_panic() {
        let source = include_str!("../tests/fixtures/malformed.tsx");
        let allocator = Allocator::default();
        let result = crate::parse(&allocator, source);
        assert!(result.is_err(), "expected malformed source to produce parse errors");
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn valid_source_parses_successfully() {
        let source = "export function Empty() { return null; }";
        let allocator = Allocator::default();
        let result = crate::parse(&allocator, source);
        assert!(result.is_ok(), "expected valid source to parse without errors");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/compiler && cargo test`
Expected: FAIL — `cannot find function 'parse' in crate 'crate'` (or similar "unresolved" compile error), since `parse()` doesn't exist yet.

- [ ] **Step 4: Implement `parse()`**

`packages/compiler/src/lib.rs` (add above the `#[cfg(test)]` module):
```rust
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

pub fn parse<'a>(allocator: &'a Allocator, source: &'a str) -> Result<Program<'a>, Vec<ParseError>> {
    let source_type = SourceType::tsx();
    let ret = Parser::new(allocator, source, source_type).parse();

    if ret.panicked || !ret.errors.is_empty() {
        let errors = ret.errors.iter().map(|e| ParseError { message: e.to_string() }).collect();
        return Err(errors);
    }

    Ok(ret.program)
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS — 2 tests (`malformed_source_returns_errors_not_panic`, `valid_source_parses_successfully`).

- [ ] **Step 6: Commit**

```bash
git add .gitignore packages/compiler
git commit -m "Add compiler crate scaffold with oxc-based parse()"
```

---

## Task 2: `extract_structure()` — function declarations

**Files:**
- Modify: `packages/compiler/src/lib.rs`
- Create: `packages/compiler/tests/fixtures/static.tsx`

**Interfaces:**
- Consumes: `Program` from `parse()` (Task 1).
- Produces: `pub struct StructuralSummary { pub function_names: Vec<String>, pub jsx_elements: Vec<JsxElementInfo>, pub jsx_expression_containers: usize, pub signal_call_sites: usize }` (fields beyond `function_names` are added empty/zero in this task, populated in Tasks 3–4), `pub fn extract_structure(program: &Program) -> StructuralSummary`. Task 3 adds `JsxElementInfo` and populates `jsx_elements`/`jsx_expression_containers`; Task 4 populates `signal_call_sites`.

- [ ] **Step 1: Create the static fixture**

`packages/compiler/tests/fixtures/static.tsx`:
```tsx
export function Static() {
  return <div>Hello</div>;
}
```

- [ ] **Step 2: Write the failing test**

Add to `packages/compiler/src/lib.rs`, a new test module:
```rust
#[cfg(test)]
mod structure_tests {
    use oxc_allocator::Allocator;

    #[test]
    fn static_component_has_no_signals_or_expressions() {
        let source = include_str!("../tests/fixtures/static.tsx");
        let allocator = Allocator::default();
        let program = crate::parse(&allocator, source).unwrap();
        let summary = crate::extract_structure(&program);

        assert_eq!(summary.function_names, vec!["Static".to_string()]);
        assert_eq!(summary.jsx_expression_containers, 0);
        assert_eq!(summary.signal_call_sites, 0);
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd packages/compiler && cargo test`
Expected: FAIL — `cannot find function 'extract_structure'` / `cannot find type 'StructuralSummary'`.

- [ ] **Step 4: Implement `StructuralSummary` and `extract_structure()`**

Add to `packages/compiler/src/lib.rs` (above the test modules):
```rust
use oxc_ast_visit::Visit;
use oxc_syntax::scope::ScopeFlags;

#[derive(Debug, Default)]
pub struct JsxElementInfo {
    pub tag_name: String,
    pub is_native: bool,
    pub attribute_names: Vec<String>,
}

#[derive(Debug, Default)]
pub struct StructuralSummary {
    pub function_names: Vec<String>,
    pub jsx_elements: Vec<JsxElementInfo>,
    pub jsx_expression_containers: usize,
    pub signal_call_sites: usize,
}

struct StructureCollector {
    summary: StructuralSummary,
}

impl<'a> Visit<'a> for StructureCollector {
    fn visit_function(&mut self, it: &oxc_ast::ast::Function<'a>, flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.summary.function_names.push(id.name.as_str().to_string());
        }
        oxc_ast_visit::walk::walk_function(self, it, flags);
    }
}

pub fn extract_structure(program: &Program) -> StructuralSummary {
    let mut collector = StructureCollector { summary: StructuralSummary::default() };
    collector.visit_program(program);
    collector.summary
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS — 3 tests total (2 from Task 1, 1 new).

- [ ] **Step 6: Commit**

```bash
git add packages/compiler
git commit -m "Add extract_structure() with function declaration extraction"
```

---

## Task 3: JSX element + expression container extraction

**Files:**
- Modify: `packages/compiler/src/lib.rs`
- Create: `packages/compiler/tests/fixtures/mixed_tags.tsx`

**Interfaces:**
- Consumes: `JsxElementInfo`, `StructuralSummary`, `StructureCollector` (Task 2).
- Produces: `StructureCollector` now also implements `visit_jsx_element`, populating `jsx_elements` (tag name, `is_native`, `attribute_names`) and `jsx_expression_containers`. Task 4 builds on this same `impl Visit` block, adding `visit_call_expression`.

- [ ] **Step 1: Create the mixed-tags fixture**

`packages/compiler/tests/fixtures/mixed_tags.tsx`:
```tsx
export function App() {
  return (
    <div>
      <Profile />
    </div>
  );
}
```

- [ ] **Step 2: Write the failing tests**

Add to `structure_tests` in `packages/compiler/src/lib.rs`:
```rust
    #[test]
    fn static_component_jsx_element_is_native_div() {
        let source = include_str!("../tests/fixtures/static.tsx");
        let allocator = Allocator::default();
        let program = crate::parse(&allocator, source).unwrap();
        let summary = crate::extract_structure(&program);

        assert_eq!(summary.jsx_elements.len(), 1);
        assert_eq!(summary.jsx_elements[0].tag_name, "div");
        assert!(summary.jsx_elements[0].is_native);
        assert!(summary.jsx_elements[0].attribute_names.is_empty());
    }

    #[test]
    fn mixed_tags_distinguishes_native_from_component_references() {
        let source = include_str!("../tests/fixtures/mixed_tags.tsx");
        let allocator = Allocator::default();
        let program = crate::parse(&allocator, source).unwrap();
        let summary = crate::extract_structure(&program);

        assert_eq!(summary.jsx_elements.len(), 2);
        assert_eq!(summary.jsx_elements[0].tag_name, "div");
        assert!(summary.jsx_elements[0].is_native);
        assert_eq!(summary.jsx_elements[1].tag_name, "Profile");
        assert!(!summary.jsx_elements[1].is_native);
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd packages/compiler && cargo test`
Expected: FAIL — both new tests fail: `assertion failed: summary.jsx_elements.len() == 1` (actual: 0), since `jsx_elements` is never populated yet.

- [ ] **Step 4: Implement JSX element + expression container extraction**

In `packages/compiler/src/lib.rs`, add these imports:
```rust
use oxc_ast::ast::{JSXAttributeItem, JSXAttributeName, JSXChild, JSXElementName};
```

Add this method inside the existing `impl<'a> Visit<'a> for StructureCollector` block (alongside `visit_function`):
```rust
    fn visit_jsx_element(&mut self, it: &oxc_ast::ast::JSXElement<'a>) {
        let (tag_name, is_native) = match &it.opening_element.name {
            JSXElementName::Identifier(ident) => {
                let name = ident.name.as_str();
                let native = name.chars().next().map(|c| c.is_lowercase()).unwrap_or(false);
                (name.to_string(), native)
            }
            JSXElementName::IdentifierReference(ident) => {
                (ident.name.as_str().to_string(), false)
            }
            _ => ("<complex>".to_string(), false),
        };

        let attribute_names = it
            .opening_element
            .attributes
            .iter()
            .filter_map(|item| match item {
                JSXAttributeItem::Attribute(attr) => match &attr.name {
                    JSXAttributeName::Identifier(ident) => Some(ident.name.as_str().to_string()),
                    JSXAttributeName::NamespacedName(ns) => Some(format!(
                        "{}:{}",
                        ns.namespace.name.as_str(),
                        ns.name.name.as_str()
                    )),
                },
                JSXAttributeItem::SpreadAttribute(_) => None,
            })
            .collect();

        self.summary.jsx_elements.push(JsxElementInfo { tag_name, is_native, attribute_names });

        for child in &it.children {
            if matches!(child, JSXChild::ExpressionContainer(_)) {
                self.summary.jsx_expression_containers += 1;
            }
        }

        oxc_ast_visit::walk::walk_jsx_element(self, it);
    }
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS — 5 tests total (3 from Tasks 1–2, 2 new).

- [ ] **Step 6: Commit**

```bash
git add packages/compiler
git commit -m "Extract JSX element tag/attribute/expression-container structure"
```

---

## Task 4: `signal(...)` call-site detection + full `counter.tsx` coverage

**Files:**
- Modify: `packages/compiler/src/lib.rs`
- Create: `packages/compiler/tests/fixtures/counter.tsx`

**Interfaces:**
- Consumes: `StructureCollector`, `StructuralSummary` (Tasks 2–3).
- Produces: `StructureCollector` now also implements `visit_call_expression`, populating `signal_call_sites`. This completes `extract_structure()`'s full behavior for this sub-cycle — no further methods are added.

- [ ] **Step 1: Create the counter fixture (the mission's own example)**

`packages/compiler/tests/fixtures/counter.tsx`:
```tsx
export function Counter(props: { start: number }) {
  let count = signal(props.start);
  return (
    <button onClick={() => count++}>{count}</button>
  );
}
```

- [ ] **Step 2: Write the failing test**

Add to `structure_tests` in `packages/compiler/src/lib.rs`:
```rust
    #[test]
    fn counter_component_full_structure() {
        let source = include_str!("../tests/fixtures/counter.tsx");
        let allocator = Allocator::default();
        let program = crate::parse(&allocator, source).unwrap();
        let summary = crate::extract_structure(&program);

        assert_eq!(summary.function_names, vec!["Counter".to_string()]);
        assert_eq!(summary.jsx_elements.len(), 1);
        assert_eq!(summary.jsx_elements[0].tag_name, "button");
        assert!(summary.jsx_elements[0].is_native);
        assert_eq!(summary.jsx_elements[0].attribute_names, vec!["onClick".to_string()]);
        assert_eq!(summary.jsx_expression_containers, 1);
        assert_eq!(summary.signal_call_sites, 1);
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd packages/compiler && cargo test`
Expected: FAIL — `assertion failed: summary.signal_call_sites == 1` (actual: 0), since call expressions aren't visited yet.

- [ ] **Step 4: Implement `signal(...)` call-site detection**

Add this import to `packages/compiler/src/lib.rs`:
```rust
use oxc_ast::ast::Expression;
```

Add this method inside the existing `impl<'a> Visit<'a> for StructureCollector` block:
```rust
    fn visit_call_expression(&mut self, it: &oxc_ast::ast::CallExpression<'a>) {
        if let Expression::Identifier(ident) = &it.callee {
            if ident.name.as_str() == "signal" {
                self.summary.signal_call_sites += 1;
            }
        }
        oxc_ast_visit::walk::walk_call_expression(self, it);
    }
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd packages/compiler && cargo test`
Expected: PASS — 6 tests total (5 from Tasks 1–3, 1 new).

- [ ] **Step 6: Commit**

```bash
git add packages/compiler
git commit -m "Detect signal() call sites (syntactic heuristic, no import resolution)"
```

---

## Sub-cycle Gate Checklist

- [ ] `packages/compiler` is a real Rust crate (`Cargo.toml` + `src/lib.rs`), pinned to oxc `0.116.0` for all crates (Task 1).
- [ ] `parse()` returns `Result<Program, Vec<ParseError>>`; malformed source produces errors, not a panic (Task 1).
- [ ] `extract_structure()` reports function declarations, JSX element tag/native-vs-component/attributes, expression-container counts, and `signal(...)` call-site counts — all purely syntactic (Tasks 2–4).
- [ ] All 4 fixtures (`static.tsx`, `counter.tsx`, `mixed_tags.tsx`, `malformed.tsx`) exist and are exercised by tests (Tasks 1–4).
- [ ] No `oxc_semantic`, no `TEZ###` error codes, no codegen, no napi-rs — confirmed absent from `Cargo.toml` and `src/lib.rs`.

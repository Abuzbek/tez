# Compiler Codegen Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `compile_dom(source) -> Result<String, CompileError>` — compile static components to hoisted `template()` consts + clone calls, printed via `oxc_codegen`, with explicit `Unsupported` errors for everything deferred to later sub-cycles.

**Architecture:** Two new modules in the existing `tez-compiler` crate. `template_html.rs` serializes a static JSX element tree to an HTML string (escaping, void elements, boolean attributes, reserved `v-*`/`use:` namespaces). `codegen.rs` owns `CompileError` and the transform: a `VisitMut` pass replaces each JSX root inside a component with `_tN()`, then header statements (runtime import + template consts) are built by parsing a snippet into the same allocator, patching the placeholder string literals, and splicing before the first non-import statement; `Codegen::new().build(&program).code` prints the result. This exact technique (snippet parse + literal patch + splice) was prototyped and verified against oxc 0.116.0 before this plan was written.

**Tech Stack:** Rust (edition 2024, rustc 1.91.1), oxc 0.116.0 pinned family; adds `oxc_codegen = "0.116.0"` (sanctioned by design §2.4; verified to build on this toolchain).

**Design spec:** `docs/superpowers/specs/2026-07-07-compiler-dom-codegen-design.md` §2 (approved 2026-07-07).

## Global Constraints

- All work happens in the `phase1-compiler-codegen-skeleton` worktree at `.worktrees/phase1-compiler-codegen-skeleton/` (paths below relative to that root).
- oxc crates pinned at `0.116.0`; the ONLY permitted dependency addition is `oxc_codegen = "0.116.0"` (Task 2). Never `cargo update`.
- Tests live in `#[cfg(test)]` modules inside `packages/compiler/src/lib.rs` (crate convention). Fixtures in `packages/compiler/tests/fixtures/`, loaded with `include_str!`.
- Run tests with `cargo test` from `packages/compiler/`. 32 tests pre-exist and must keep passing.
- `oxc_codegen` output format facts (empirically verified): **tab** indentation, double-quoted strings with `\"` escapes inside, semicolons, trailing newline after the last `}`. Snapshot strings in this plan encode that exactly — `\t` in Rust string literals is the tab.
- `compile_dom` in this sub-cycle is parse → transform → print. It does NOT yet run `SemanticBuilder`/classification/TEZ101 — static-only input has nothing for them to analyze; they join in sub-cycle 2 when classification drives binding emission. (Deviation from the design's §2.1 pipeline wording, resolved this way deliberately; the design's own TEZ101 spec kept pipeline wiring out of scope.)
- TypeScript annotations in non-component code pass through to the output (printing them is `oxc_codegen`'s default); stripping TS is the Vite/napi layer's job in sub-cycle 4.
- Commit messages: imperative mood matching repo history. **Never add a `Co-Authored-By` trailer or any AI attribution.**

---

### Task 1: `CompileError` + static HTML serializer

**Files:**
- Create: `packages/compiler/src/codegen.rs` (just `CompileError` in this task)
- Create: `packages/compiler/src/template_html.rs`
- Modify: `packages/compiler/src/lib.rs` (module declarations + test module)

**Interfaces:**
- Consumes: `crate::ParseError` (existing, `pub struct ParseError { pub message: String }`), oxc AST JSX types.
- Produces: `pub enum CompileError { Parse(Vec<ParseError>), Unsupported { span: Span, what: String } }` in `crate::codegen`; `pub fn serialize_static(element: &JSXElement) -> Result<String, CompileError>` in `crate::template_html`. Task 2's transform calls `serialize_static` and extends `codegen.rs` around `CompileError`.

- [ ] **Step 1: Write the failing tests**

Add to the module declarations at the top of `packages/compiler/src/lib.rs` (alongside the existing `pub mod` lines):

```rust
pub mod codegen;
pub mod template_html;
```

Add at the bottom of `packages/compiler/src/lib.rs`:

```rust
#[cfg(test)]
mod template_html_tests {
    use oxc_allocator::Allocator;
    use oxc_ast_visit::Visit;

    use crate::codegen::CompileError;
    use crate::template_html::serialize_static;

    /// Parses `source` and serializes the first JSX element found.
    fn serialize_first(source: &str) -> Result<String, CompileError> {
        struct Grab {
            out: Option<Result<String, CompileError>>,
        }
        impl<'a> Visit<'a> for Grab {
            fn visit_jsx_element(&mut self, it: &oxc_ast::ast::JSXElement<'a>) {
                if self.out.is_none() {
                    self.out = Some(serialize_static(it));
                }
            }
        }

        let allocator = Allocator::default();
        let ret = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
        assert!(ret.errors.is_empty(), "unexpected parse errors");
        let mut grab = Grab { out: None };
        grab.visit_program(&ret.program);
        grab.out.expect("source contains a JSX element")
    }

    fn unsupported_what(result: Result<String, CompileError>) -> String {
        match result {
            Err(CompileError::Unsupported { what, .. }) => what,
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn text_ampersand_and_angles_are_escaped() {
        let html = serialize_first("let x = <div>fish & chips</div>;").unwrap();
        assert_eq!(html, "<div>fish &amp; chips</div>");
    }

    #[test]
    fn attribute_quotes_and_ampersands_are_escaped() {
        let html = serialize_first(r#"let x = <div title='say "hi" & wave'>ok</div>;"#).unwrap();
        assert_eq!(html, r#"<div title="say &quot;hi&quot; &amp; wave">ok</div>"#);
    }

    #[test]
    fn void_element_omits_closing_tag() {
        let html = serialize_first(r#"let x = <img src="x.png" />;"#).unwrap();
        assert_eq!(html, r#"<img src="x.png">"#);
    }

    #[test]
    fn boolean_attribute_emits_bare_name() {
        let html = serialize_first("let x = <input disabled />;").unwrap();
        assert_eq!(html, "<input disabled>");
    }

    #[test]
    fn nested_elements_serialize_in_order() {
        let html = serialize_first("let x = <section><h1>Title</h1><p>body</p></section>;").unwrap();
        assert_eq!(html, "<section><h1>Title</h1><p>body</p></section>");
    }

    #[test]
    fn expression_child_is_unsupported() {
        let what = unsupported_what(serialize_first("let x = <div>{name}</div>;"));
        assert!(what.contains("sub-cycle 2"), "should point at sub-cycle 2: {what}");
    }

    #[test]
    fn dynamic_attribute_value_is_unsupported() {
        let what = unsupported_what(serialize_first("let x = <div class={cls}>ok</div>;"));
        assert!(what.contains("sub-cycle 2"), "should point at sub-cycle 2: {what}");
    }

    #[test]
    fn spread_attribute_is_unsupported() {
        let what = unsupported_what(serialize_first("let x = <div {...props}>ok</div>;"));
        assert!(what.contains("TEZ102"), "should reference TEZ102/sub-cycle 3: {what}");
    }

    #[test]
    fn component_tag_is_unsupported() {
        let what = unsupported_what(serialize_first("let x = <Profile />;"));
        assert!(what.contains("Profile"), "should name the component: {what}");
    }

    #[test]
    fn v_directive_attribute_is_reserved() {
        let what = unsupported_what(serialize_first("let x = <input v-model={name} />;"));
        assert!(what.contains("reserved for the directives layer"), "reserved wording: {what}");
    }

    #[test]
    fn use_directive_attribute_is_reserved() {
        let what = unsupported_what(serialize_first("let x = <div use:clickOutside={close}>ok</div>;"));
        assert!(what.contains("reserved for the directives layer"), "reserved wording: {what}");
    }

    #[test]
    fn children_on_void_element_are_unsupported() {
        let what = unsupported_what(serialize_first("let x = <br>oops</br>;"));
        assert!(what.contains("void"), "should mention void element: {what}");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test template_html_tests` from `packages/compiler/`
Expected: COMPILE ERROR — `file not found for module codegen` (and `template_html`).

- [ ] **Step 3: Write the implementation**

Create `packages/compiler/src/codegen.rs`:

```rust
use oxc_span::Span;

use crate::ParseError;

/// Errors from `compile_dom`. `Unsupported` is temporary by design: each
/// later sub-cycle deletes the cases it implements (dynamic expressions in
/// sub-cycle 2; components, spreads, fragments in sub-cycle 3; directives
/// in cycle 4). Explicit errors, never silent wrong output.
#[derive(Debug)]
pub enum CompileError {
    /// Source failed to parse (wraps the existing ParseError list).
    Parse(Vec<ParseError>),
    /// A construct this sub-cycle cannot compile yet.
    Unsupported { span: Span, what: String },
}
```

Create `packages/compiler/src/template_html.rs`:

```rust
use oxc_ast::ast::{
    JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild, JSXElement, JSXElementName,
};
use oxc_span::Span;

use crate::codegen::CompileError;

/// HTML elements with no closing tag. Children on these are an error.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
    "track", "wbr",
];

/// Serializes a fully static JSX element tree to the HTML string embedded
/// in a `template()` call. Anything dynamic or deferred returns
/// `CompileError::Unsupported` with a span — this function is the single
/// authority on what "static" means for sub-cycle 1.
///
/// JSXText is emitted verbatim (whitespace included); JSX
/// whitespace-collapsing semantics arrive with dynamic text handling in
/// sub-cycle 2.
pub fn serialize_static(element: &JSXElement) -> Result<String, CompileError> {
    let mut out = String::new();
    write_element(element, &mut out)?;
    Ok(out)
}

fn unsupported(span: Span, what: impl Into<String>) -> CompileError {
    CompileError::Unsupported { span, what: what.into() }
}

fn write_element(el: &JSXElement, out: &mut String) -> Result<(), CompileError> {
    let name = match &el.opening_element.name {
        // Lowercase native tags parse as Identifier.
        JSXElementName::Identifier(ident) => ident.name.as_str(),
        // Capitalized component references parse as IdentifierReference.
        JSXElementName::IdentifierReference(ident) => {
            return Err(unsupported(
                el.span,
                format!("component tag <{}> (control-flow/component codegen arrives in sub-cycle 3)", ident.name),
            ));
        }
        _ => {
            return Err(unsupported(el.span, "complex JSX tag (member/namespaced/this expression)"));
        }
    };

    out.push('<');
    out.push_str(name);

    for item in &el.opening_element.attributes {
        match item {
            JSXAttributeItem::Attribute(attr) => {
                let attr_name = match &attr.name {
                    JSXAttributeName::Identifier(id) => id.name.as_str().to_string(),
                    JSXAttributeName::NamespacedName(ns) => {
                        format!("{}:{}", ns.namespace.name.as_str(), ns.name.name.as_str())
                    }
                };
                if attr_name.starts_with("v-") || attr_name.starts_with("use:") {
                    return Err(unsupported(
                        attr.span,
                        format!("`{attr_name}` is reserved for the directives layer (cycle 4), not yet supported"),
                    ));
                }
                match &attr.value {
                    None => {
                        out.push(' ');
                        out.push_str(&attr_name);
                    }
                    Some(JSXAttributeValue::StringLiteral(lit)) => {
                        out.push(' ');
                        out.push_str(&attr_name);
                        out.push_str("=\"");
                        out.push_str(&escape_attr(lit.value.as_str()));
                        out.push('"');
                    }
                    Some(JSXAttributeValue::ExpressionContainer(c)) => {
                        return Err(unsupported(c.span, "dynamic attribute value (sub-cycle 2)"));
                    }
                    Some(_) => {
                        return Err(unsupported(attr.span, "unsupported attribute value shape"));
                    }
                }
            }
            JSXAttributeItem::SpreadAttribute(sp) => {
                return Err(unsupported(
                    sp.span,
                    "spread attributes on native elements (TEZ102, sub-cycle 3)",
                ));
            }
        }
    }

    if VOID_ELEMENTS.contains(&name) {
        if !el.children.is_empty() {
            return Err(unsupported(el.span, format!("children on void element <{name}>")));
        }
        out.push('>');
        return Ok(());
    }

    out.push('>');
    for child in &el.children {
        match child {
            JSXChild::Text(t) => out.push_str(&escape_text(t.value.as_str())),
            JSXChild::Element(child_el) => write_element(child_el, out)?,
            JSXChild::ExpressionContainer(c) => {
                return Err(unsupported(c.span, "JSX expression in template (sub-cycle 2)"));
            }
            JSXChild::Fragment(f) => {
                return Err(unsupported(f.span, "fragment child (sub-cycle 3)"));
            }
            JSXChild::Spread(s) => {
                return Err(unsupported(s.span, "spread child"));
            }
        }
    }
    out.push_str("</");
    out.push_str(name);
    out.push('>');
    Ok(())
}

// `&` must be replaced first in both escapers.
fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test template_html_tests` from `packages/compiler/`
Expected: 12 passed.

Then the full suite: `cargo test`
Expected: 44 passed (32 pre-existing + 12 new), no warnings.

- [ ] **Step 5: Commit**

```bash
git add packages/compiler/src/codegen.rs packages/compiler/src/template_html.rs packages/compiler/src/lib.rs
git commit -m "Add static JSX-to-HTML serializer and CompileError type"
```

---

### Task 2: `compile_dom` transform

**Files:**
- Modify: `packages/compiler/Cargo.toml` (add `oxc_codegen = "0.116.0"` to `[dependencies]`)
- Modify: `packages/compiler/src/codegen.rs` (add the transform)
- Modify: `packages/compiler/src/tez101.rs` (make `ContainsJsx` reusable: `pub(crate)`)
- Create: `packages/compiler/tests/fixtures/codegen_two_components.tsx`
- Create: `packages/compiler/tests/fixtures/codegen_passthrough.tsx`
- Create: `packages/compiler/tests/fixtures/codegen_void_boolean.tsx`
- Modify: `packages/compiler/src/lib.rs` (add `codegen_tests` module)

**Interfaces:**
- Consumes: `serialize_static` and `CompileError` (Task 1); `crate::tez101::ContainsJsx` (existing probe — named-nested-function-skipping, fragment-aware; this task only widens its visibility).
- Produces: `pub fn compile_dom(source: &str) -> Result<String, CompileError>` in `crate::codegen` — the function sub-cycle 4's napi layer will wrap, and the entry point every later sub-cycle extends.

- [ ] **Step 1: Create the fixtures**

Create `packages/compiler/tests/fixtures/codegen_two_components.tsx`:

```tsx
export function A() {
  return <p>one</p>;
}

export function B() {
  return <p>two</p>;
}
```

Create `packages/compiler/tests/fixtures/codegen_passthrough.tsx`:

```tsx
import { helper } from "./helpers";

export const answer = 42;

export function Card() {
  return <section><h1>Title</h1><p>body</p></section>;
}

export function plain(x: number) {
  return x + 1;
}
```

Create `packages/compiler/tests/fixtures/codegen_void_boolean.tsx`:

```tsx
export function Fields() {
  return <p><img src="x.png" /><input disabled /><br /></p>;
}
```

- [ ] **Step 2: Write the failing tests**

Add at the bottom of `packages/compiler/src/lib.rs`:

```rust
#[cfg(test)]
mod codegen_tests {
    use crate::codegen::{compile_dom, CompileError};

    fn unsupported_what(result: Result<String, CompileError>) -> String {
        match result {
            Err(CompileError::Unsupported { what, .. }) => what,
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn static_component_compiles_to_template_clone() {
        let source = include_str!("../tests/fixtures/static.tsx");
        let out = compile_dom(source).unwrap();
        let expected = "import { template } from \"@tez/runtime-dom\";\nconst _t1 = template(\"<div>Hello</div>\");\nexport function Static() {\n\treturn _t1();\n}\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn two_components_get_sequential_templates() {
        let source = include_str!("../tests/fixtures/codegen_two_components.tsx");
        let out = compile_dom(source).unwrap();
        let expected = "import { template } from \"@tez/runtime-dom\";\nconst _t1 = template(\"<p>one</p>\");\nconst _t2 = template(\"<p>two</p>\");\nexport function A() {\n\treturn _t1();\n}\nexport function B() {\n\treturn _t2();\n}\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn non_component_code_passes_through() {
        let source = include_str!("../tests/fixtures/codegen_passthrough.tsx");
        let out = compile_dom(source).unwrap();
        let expected = "import { template } from \"@tez/runtime-dom\";\nimport { helper } from \"./helpers\";\nconst _t1 = template(\"<section><h1>Title</h1><p>body</p></section>\");\nexport const answer = 42;\nexport function Card() {\n\treturn _t1();\n}\nexport function plain(x: number) {\n\treturn x + 1;\n}\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn void_and_boolean_attributes_compile() {
        let source = include_str!("../tests/fixtures/codegen_void_boolean.tsx");
        let out = compile_dom(source).unwrap();
        assert!(
            out.contains(r#"template("<p><img src=\"x.png\"><input disabled><br></p>")"#),
            "template HTML wrong in: {out}"
        );
    }

    #[test]
    fn expression_container_is_unsupported() {
        let source = include_str!("../tests/fixtures/counter.tsx");
        let what = unsupported_what(compile_dom(source));
        assert!(what.contains("sub-cycle 2"), "should point at sub-cycle 2: {what}");
    }

    #[test]
    fn fragment_root_is_unsupported() {
        let what =
            unsupported_what(compile_dom("export function Pair() {\n  return <>hi</>;\n}\n"));
        assert!(what.contains("fragment root"), "fragment wording: {what}");
    }

    #[test]
    fn jsx_outside_component_is_unsupported() {
        let what = unsupported_what(compile_dom("const banner = <div>hi</div>;\n"));
        assert!(what.contains("outside a component"), "wording: {what}");
    }

    #[test]
    fn parse_errors_are_reported() {
        let source = include_str!("../tests/fixtures/malformed.tsx");
        match compile_dom(source) {
            Err(CompileError::Parse(errors)) => assert!(!errors.is_empty()),
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn module_without_components_gets_no_injected_import() {
        let out = compile_dom("export const n = 1;\n").unwrap();
        assert_eq!(out, "export const n = 1;\n");
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test codegen_tests` from `packages/compiler/`
Expected: COMPILE ERROR — `cannot find function compile_dom in module crate::codegen`.

- [ ] **Step 4: Write the implementation**

In `packages/compiler/Cargo.toml`, add to `[dependencies]` (keeping the pin comment intact):

```toml
oxc_codegen = "0.116.0"
```

In `packages/compiler/src/tez101.rs`, widen the probe (two edits):

```rust
// before
struct ContainsJsx {
    found: bool,
}
// after — codegen.rs reuses this probe for its component boundary,
// keeping "what counts as a component" defined in exactly one place.
pub(crate) struct ContainsJsx {
    pub(crate) found: bool,
}
```

Extend `packages/compiler/src/codegen.rs` (below `CompileError`) with:

```rust
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Function, Program, Statement};
use oxc_ast_visit::{Visit, VisitMut};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::{Atom, SourceType};
use oxc_syntax::scope::ScopeFlags;

use crate::template_html::serialize_static;
use crate::tez101::ContainsJsx;

/// Compiles a `.tsx` module's static components: each component's JSX root
/// becomes a hoisted `const _tN = template("…")` plus a `_tN()` clone call,
/// with one `import { template } from "@tez/runtime-dom"` injected. All
/// non-component code passes through. This sub-cycle is parse → transform →
/// print; semantic analyses join the pipeline in sub-cycle 2 when
/// classification starts driving binding emission.
pub fn compile_dom(source: &str) -> Result<String, CompileError> {
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
    if ret.panicked || !ret.errors.is_empty() {
        return Err(CompileError::Parse(
            ret.errors.iter().map(|e| crate::ParseError { message: e.to_string() }).collect(),
        ));
    }
    let mut program = ret.program;

    let mut transformer = ComponentTransformer {
        allocator: &allocator,
        component_depth: 0,
        templates: Vec::new(),
        error: None,
    };
    transformer.visit_program(&mut program);
    if let Some(error) = transformer.error {
        return Err(error);
    }

    if !transformer.templates.is_empty() {
        splice_header(&allocator, &mut program, &transformer.templates);
    }
    Ok(Codegen::new().build(&program).code)
}

/// Replaces each JSX root inside a component with `_tN()` and records its
/// serialized HTML. `component_depth` tracks whether the walk is inside a
/// component (same boundary as TEZ101: named function whose own body has
/// JSX); JSX encountered at depth 0 is an error rather than something to
/// guess about. Visitors cannot return `Result`, so the first error is
/// stashed and every visit method bails once it is set.
struct ComponentTransformer<'a> {
    allocator: &'a Allocator,
    component_depth: usize,
    templates: Vec<String>,
    error: Option<CompileError>,
}

impl<'a> VisitMut<'a> for ComponentTransformer<'a> {
    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        if self.error.is_some() {
            return;
        }
        let is_component = it.id.is_some() && {
            let mut probe = ContainsJsx { found: false };
            oxc_ast_visit::walk::walk_function(&mut probe, &*it, flags);
            probe.found
        };
        if is_component {
            self.component_depth += 1;
        }
        oxc_ast_visit::walk_mut::walk_function(self, it, flags);
        if is_component {
            self.component_depth -= 1;
        }
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.error.is_some() {
            return;
        }
        match it {
            Expression::JSXElement(el) => {
                if self.component_depth == 0 {
                    self.error = Some(CompileError::Unsupported {
                        span: el.span,
                        what: "JSX outside a component function".to_string(),
                    });
                    return;
                }
                match serialize_static(el) {
                    Ok(html) => {
                        self.templates.push(html);
                        let name = format!("_t{}", self.templates.len());
                        // The serializer already covered the whole element;
                        // no walk into the replaced subtree.
                        *it = parse_call_expression(self.allocator, &name);
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            Expression::JSXFragment(f) => {
                self.error = Some(CompileError::Unsupported {
                    span: f.span,
                    what: "fragment root (multi-node templates arrive in sub-cycle 3)".to_string(),
                });
            }
            _ => oxc_ast_visit::walk_mut::walk_expression(self, it),
        }
    }
}

/// Builds `_tN()` by parsing a tiny snippet in the same allocator — AST
/// nodes share the arena lifetime, so the expression can be moved into the
/// main program. (Snippet-parsing beats hand-assembling AstBuilder calls
/// for these fixed shapes; the dynamic part — the HTML — is patched into
/// the AST as a value, never string-concatenated into code.)
fn parse_call_expression<'a>(allocator: &'a Allocator, name: &str) -> Expression<'a> {
    let src = allocator.alloc_str(&format!("{name}()"));
    let ret = Parser::new(allocator, src, SourceType::mjs()).parse();
    let Some(Statement::ExpressionStatement(es)) = ret.program.body.into_iter().next() else {
        unreachable!("snippet is a single expression statement");
    };
    es.unbox().expression
}

/// Injects the runtime import as the first statement and the template
/// consts immediately before the first non-import statement.
fn splice_header<'a>(allocator: &'a Allocator, program: &mut Program<'a>, templates: &[String]) {
    let mut header_src = String::from("import { template } from \"@tez/runtime-dom\";\n");
    for i in 0..templates.len() {
        header_src.push_str(&format!("const _t{} = template(\"\");\n", i + 1));
    }
    let header_ret =
        Parser::new(allocator, allocator.alloc_str(&header_src), SourceType::mjs()).parse();
    debug_assert!(header_ret.errors.is_empty(), "header snippet must parse");

    let mut header_stmts = header_ret.program.body.into_iter();
    let import_stmt = header_stmts.next().expect("header has the import");
    let mut consts: Vec<Statement<'a>> = Vec::new();
    for (i, mut stmt) in header_stmts.enumerate() {
        patch_template_literal(allocator, &mut stmt, &templates[i]);
        consts.push(stmt);
    }

    let old_body = std::mem::replace(&mut program.body, oxc_allocator::Vec::new_in(allocator));
    let mut new_body = oxc_allocator::Vec::new_in(allocator);
    new_body.push(import_stmt);
    let mut pending: Option<Vec<Statement<'a>>> = Some(consts);
    for stmt in old_body {
        if pending.is_some() && !matches!(stmt, Statement::ImportDeclaration(_)) {
            for c in pending.take().unwrap() {
                new_body.push(c);
            }
        }
        new_body.push(stmt);
    }
    if let Some(rest) = pending.take() {
        for c in rest {
            new_body.push(c);
        }
    }
    program.body = new_body;
}

/// Replaces the placeholder `""` in `const _tN = template("");` with the
/// real HTML. Setting `raw = None` makes `oxc_codegen` re-quote and escape
/// from `value`, so backticks, quotes, or `${` in static content can never
/// corrupt the emitted JS.
fn patch_template_literal<'a>(allocator: &'a Allocator, stmt: &mut Statement<'a>, html: &str) {
    let Statement::VariableDeclaration(decl) = stmt else {
        unreachable!("header snippet statement is a const declaration")
    };
    let Some(Expression::CallExpression(call)) = &mut decl.declarations[0].init else {
        unreachable!("header const is initialized with a template() call")
    };
    let Some(arg) = call.arguments[0].as_expression_mut() else {
        unreachable!("template() argument is an expression")
    };
    let Expression::StringLiteral(lit) = arg else {
        unreachable!("template() argument is a string literal")
    };
    lit.value = Atom::from(allocator.alloc_str(html));
    lit.raw = None;
    lit.lone_surrogates = false;
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test codegen_tests` from `packages/compiler/`
Expected: 9 passed. If a snapshot mismatches only in whitespace, remember the verified format: tabs for indentation, trailing newline; fix the implementation or report the actual `oxc_codegen` output as a concern — do not weaken `assert_eq!` to `contains`.

Then the full suite: `cargo test`
Expected: 53 passed (44 from Task 1's state + 9 new), no warnings.

- [ ] **Step 6: Commit**

```bash
git add packages/compiler/Cargo.toml packages/compiler/Cargo.lock packages/compiler/src/codegen.rs packages/compiler/src/tez101.rs packages/compiler/src/lib.rs packages/compiler/tests/fixtures/codegen_two_components.tsx packages/compiler/tests/fixtures/codegen_passthrough.tsx packages/compiler/tests/fixtures/codegen_void_boolean.tsx
git commit -m "Add compile_dom: static components compile to hoisted template clones"
```

---

### Task 3: README documentation + final verification

**Files:**
- Modify: `packages/compiler/README.md`

**Interfaces:**
- Consumes: the Task 2 public surface (`compile_dom`, `CompileError`, `serialize_static`).
- Produces: documentation only.

- [ ] **Step 1: Update the README**

In `packages/compiler/README.md`, add these two bullets to the "Implemented so far" list (after the `check_body_signal_writes()` bullet):

```markdown
- `serialize_static()` — static JSX element tree → escaped HTML template
  string; owns void elements, boolean attributes, and the reserved `v-*` /
  `use:` directive namespaces (`src/template_html.rs`).
- `compile_dom()` — DOM codegen entry point (sub-cycle 1: static components
  only): hoists `const _tN = template("…")`, replaces JSX with `_tN()` clone
  calls, injects the `@tez/runtime-dom` import, prints via `oxc_codegen`.
  Everything dynamic is an explicit `Unsupported` error until its sub-cycle
  lands (`src/codegen.rs`).
```

And update the pin sentence at the bottom to mention the added crate:

```markdown
All oxc crates (including `oxc_codegen`) are pinned to 0.116.0 (rustc 1.91.1
compatibility) — see `Cargo.toml` before touching dependencies.
```

- [ ] **Step 2: Run the full suite one last time**

Run: `cargo test` from `packages/compiler/`
Expected: 53 passed, no warnings.

- [ ] **Step 3: Commit**

```bash
git add packages/compiler/README.md
git commit -m "Document compile_dom and the static template serializer"
```

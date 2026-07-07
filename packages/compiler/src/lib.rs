pub mod semantic;
pub mod reactivity;
pub mod diagnostics;
pub mod tez101;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast_visit::Visit;
use oxc_syntax::scope::ScopeFlags;
use oxc_ast::ast::{JSXAttributeItem, JSXAttributeName, JSXChild, JSXElementName};
use oxc_ast::ast::Expression;

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
            // Catches JSXElementName::MemberExpression (<Foo.Bar/>),
            // NamespacedName (<svg:rect/>), and ThisExpression (<this/>) --
            // none of the fixtures in this sub-cycle exercise these shapes.
            // A later sub-cycle handling SVG or namespaced tags will need to
            // give this arm real handling instead of this fallback.
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

    fn visit_call_expression(&mut self, it: &oxc_ast::ast::CallExpression<'a>) {
        // Heuristic only: matches a bare identifier callee spelled "signal".
        // No import resolution -- this does not confirm the call actually
        // resolves to @tez/signals' `signal` export. That requires semantic
        // analysis, out of scope for this sub-cycle.
        if let Expression::Identifier(ident) = &it.callee {
            if ident.name.as_str() == "signal" {
                self.summary.signal_call_sites += 1;
            }
        }
        oxc_ast_visit::walk::walk_call_expression(self, it);
    }
}

pub fn extract_structure(program: &Program) -> StructuralSummary {
    let mut collector = StructureCollector { summary: StructuralSummary::default() };
    collector.visit_program(program);
    collector.summary
}

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
}

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

    #[test]
    fn aliased_import_resolves_to_signal() {
        let source = include_str!("../tests/fixtures/aliased_signal.tsx");
        let bindings = analyze_reactive_bindings(source);
        assert_eq!(bindings.get("count"), Some(&ReactiveKind::Signal));
    }

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

    #[test]
    fn both_signal_and_computed_bindings_are_detected() {
        let source = include_str!("../tests/fixtures/computed_binding.tsx");
        let bindings = analyze_reactive_bindings(source);
        assert_eq!(bindings.get("count"), Some(&ReactiveKind::Signal));
        assert_eq!(bindings.get("double"), Some(&ReactiveKind::Computed));
    }
}

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

    #[test]
    fn static_component_has_no_expressions() {
        let source = include_str!("../tests/fixtures/static.tsx");
        let components = analyze(source);
        assert_eq!(components.len(), 1);
        let (name, expressions) = &components[0];
        assert_eq!(name, "Static");
        assert!(expressions.is_empty());
    }

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

    /// Regression test for the JSX-expression-collector walk not stopping at
    /// nested named function boundaries: `Outer`'s own `{count}` must not be
    /// merged with `Inner`'s `{count}`. `ComponentCollector` never discovers
    /// `Inner` as an independent top-level component (it stops recursing as
    /// soon as it registers `Outer`), so `Inner` gets no entry at all here --
    /// the fix's job is only to stop its JSX from leaking into `Outer`'s list.
    #[test]
    fn nested_named_function_does_not_leak_expressions_into_outer_component() {
        let source = include_str!("../tests/fixtures/nested_component.tsx");
        let components = analyze(source);
        assert_eq!(components.len(), 1, "Inner must not be registered as its own component");
        let (name, expressions) = &components[0];
        assert_eq!(name, "Outer");
        // Exactly one expression -- Outer's own `{count}` -- not two (which
        // would indicate Inner's `{count}` leaked in as well).
        assert_eq!(expressions.len(), 1);
        assert_eq!(expressions[0], (JsxExpressionKind::SignalDriven, 1));
    }

    /// `reactive_attribute_and_handler_classify_correctly` only checks
    /// `(kind, dependencies.len())` tuples, so it can't tell `count` and
    /// `isDisabled` apart -- a bug that recorded the wrong signal as a
    /// dependency would pass unnoticed. This test resolves the actual
    /// `SymbolId` for `disabled={isDisabled}`'s dependency back to its name
    /// and checks it is `"isDisabled"`, not `"count"`.
    #[test]
    fn reactive_attribute_dependency_resolves_to_correct_signal_name() {
        let source = include_str!("../tests/fixtures/reactive_attribute.tsx");

        let allocator = Allocator::default();
        let source_type = SourceType::tsx();
        let parser_ret = Parser::new(&allocator, source, source_type).parse();
        assert!(parser_ret.errors.is_empty(), "unexpected parse errors");

        let semantic_ret = SemanticBuilder::new().build(&parser_ret.program);
        assert!(semantic_ret.errors.is_empty(), "unexpected semantic errors");
        let semantic = semantic_ret.semantic;

        let reactive_bindings = crate::semantic::find_reactive_bindings(&parser_ret.program, &semantic);
        let components = classify_jsx_expressions(&parser_ret.program, &semantic, &reactive_bindings);

        assert_eq!(components.len(), 1);
        let component = &components[0];
        assert_eq!(component.component_name, "ToggleButton");
        assert_eq!(component.expressions.len(), 3);

        // expressions[0] is `disabled={isDisabled}`.
        let disabled_expr = &component.expressions[0];
        assert_eq!(disabled_expr.kind, JsxExpressionKind::SignalDriven);
        assert_eq!(disabled_expr.dependencies.len(), 1);
        let dep_name = semantic.scoping().symbol_name(disabled_expr.dependencies[0]);
        assert_eq!(dep_name, "isDisabled");
        assert_ne!(dep_name, "count");
    }
}

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

    #[test]
    fn factory_helper_returning_component_is_not_flagged() {
        let source = include_str!("../tests/fixtures/tez101_factory_helper.tsx");
        let diagnostics = analyze(source);
        assert!(
            diagnostics.is_empty(),
            "a factory helper's own write must not be attributed to a nested returned component: {diagnostics:?}"
        );
    }

    #[test]
    fn fragment_component_body_write_is_flagged() {
        let source = include_str!("../tests/fixtures/tez101_fragment_write.tsx");
        let diagnostics = analyze(source);
        assert_eq!(diagnostics.len(), 1, "a fragment counts as JSX for component detection");
        assert_eq!(span_text(source, &diagnostics[0]), "count.set(1)");
    }

    #[test]
    fn nested_component_write_is_attributed_to_inner_only() {
        let source = include_str!("../tests/fixtures/tez101_nested_component_write.tsx");
        let diagnostics = analyze(source);
        assert_eq!(diagnostics.len(), 1);
        let message = &diagnostics[0].message;
        assert!(message.contains("`inner`"), "message must name the signal: {message}");
        assert!(message.contains("`Inner`"), "message must attribute the write to Inner: {message}");
        assert!(!message.contains("`Outer`"), "message must not attribute the write to Outer: {message}");
    }

    #[test]
    fn write_inside_jsx_expression_is_flagged() {
        let source = include_str!("../tests/fixtures/tez101_jsx_expression_write.tsx");
        let diagnostics = analyze(source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(span_text(source, &diagnostics[0]), "count.set(1)");
    }

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
}

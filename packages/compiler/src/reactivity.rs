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

// "Component" here means a top-level *named function declaration* only.
// Arrow/const-assigned components (e.g. `export const App = () => <div/>`)
// are not detected at all and produce no entry in the result -- not even a
// `Static` one. This is a deliberate scope boundary matching the design
// doc's definition of "component," not a bug, but it's easy to rediscover
// as a surprise, so it's called out here.
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
            // Enter via `walk_function` rather than `jsx_collector.visit_function`:
            // the latter would immediately hit `JsxExpressionCollector`'s own
            // `visit_function` override (which skips *any* named function, to
            // keep nested named functions from leaking their JSX into this
            // component) and, since this function itself is named, return
            // without visiting anything at all.
            oxc_ast_visit::walk::walk_function(&mut jsx_collector, it, flags);
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
    // A nested *named* function declaration is a separate component in its
    // own right (`ComponentCollector` registers it independently wherever
    // it is discovered), so this sub-walk must not descend into it -- doing
    // so would fold that other component's JSX expressions into this one's
    // list. Anonymous functions (e.g. inline callbacks like
    // `array.map(function (item) { return <li>{item}</li>; })`) have no
    // independent component identity, so their JSX is still collected as
    // part of the enclosing component, matching the existing behavior for
    // arrow-function callbacks.
    fn visit_function(&mut self, it: &oxc_ast::ast::Function<'a>, flags: ScopeFlags) {
        if it.id.is_some() {
            return;
        }
        oxc_ast_visit::walk::walk_function(self, it, flags);
    }

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

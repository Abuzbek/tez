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
/// A component is a named function (the check keys on `it.id`, so named
/// function expressions qualify too, not just declarations) whose own body
/// -- excluding any nested named function, which is an independent
/// component in its own right -- contains at least one JSX element or
/// fragment (piece 2's boundary, amended during review to also count
/// fragments; named helpers without JSX/fragments are legal write sites and
/// are not checked). Within a component, every statement of the synchronous
/// body is checked -- if/loops/try included -- but nothing inside any
/// nested function: a nested function defers execution past render, which
/// is exactly what makes handler and effect() writes legal.
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

/// Sets `found` on the first JSX element or fragment in the walked subtree.
/// Not walking past a found element is fine -- one is enough.
// codegen.rs reuses this probe for its component boundary, keeping "what
// counts as a component" defined in exactly one place.
pub(crate) struct ContainsJsx {
    pub(crate) found: bool,
}

impl<'a> Visit<'a> for ContainsJsx {
    fn visit_jsx_element(&mut self, _it: &oxc_ast::ast::JSXElement<'a>) {
        self.found = true;
    }

    fn visit_jsx_fragment(&mut self, _it: &oxc_ast::ast::JSXFragment<'a>) {
        self.found = true;
    }

    // A nested *named* function is an independent component in its own
    // right (mirrors `JsxExpressionCollector::visit_function` in
    // reactivity.rs) -- its JSX must not leak into this probe and cause the
    // enclosing function to be misidentified as a component too. Anonymous
    // functions and arrows have no independent component identity, so this
    // probe still descends into them.
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.id.is_some() {
            return;
        }
        oxc_ast_visit::walk::walk_function(self, it, flags);
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

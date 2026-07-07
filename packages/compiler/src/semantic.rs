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

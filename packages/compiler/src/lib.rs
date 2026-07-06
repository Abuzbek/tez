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

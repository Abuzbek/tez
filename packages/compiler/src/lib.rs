use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast_visit::Visit;
use oxc_syntax::scope::ScopeFlags;

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
}

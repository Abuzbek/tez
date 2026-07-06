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

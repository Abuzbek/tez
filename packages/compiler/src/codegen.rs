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

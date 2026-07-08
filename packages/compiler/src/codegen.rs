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

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Function, Program, Statement};
use oxc_ast_visit::VisitMut;
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
                        what: "JSX outside a component function (note: arrow/const-assigned components are not yet supported — use a named function declaration)".to_string(),
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
                    what: "JSX fragment (multi-node templates arrive in sub-cycle 3)".to_string(),
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

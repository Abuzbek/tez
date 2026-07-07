use oxc_span::Span;

/// A compiler diagnostic satisfying the architecture spec's per-diagnostic
/// contract (spec §7): TEZ### code + doc URL + primary span + cause +
/// at least one concrete fix. `TEZ101` (tez101.rs) is the first producer;
/// TEZ102-104 will reuse this type. Deliberately NOT built yet: a code
/// registry, severity levels, multi-span labels, JSON output -- extend when
/// a second producer needs them.
#[derive(Debug)]
pub struct Diagnostic {
    pub code: &'static str,
    /// Primary span, byte offsets into the source text.
    pub span: Span,
    /// What happened, naming the specific bindings involved.
    pub message: String,
    /// Why it is an error.
    pub cause: String,
    /// At least one concrete fix.
    pub help: String,
    pub docs_url: String,
}

impl Diagnostic {
    /// Stable plain-text rendering -- the surface the error-message
    /// snapshot suite asserts against (spec §7's CI gate). Changing this
    /// format or any producer's wording requires updating snapshots.
    pub fn render(&self, source: &str) -> String {
        let (line, col) = line_col(source, self.span.start);
        format!(
            "error[{}]: {}\n  --> {}:{}\ncause: {}\nhelp: {}\ndocs: {}",
            self.code, self.message, line, col, self.cause, self.help, self.docs_url
        )
    }
}

/// 1-based (line, column) of a byte offset. Column counts chars, which
/// matches byte positions for the ASCII fixtures; full Unicode column
/// semantics are a rendering concern deferred until a real terminal
/// reporter exists.
fn line_col(source: &str, offset: u32) -> (usize, usize) {
    let prefix = &source[..offset as usize];
    let line = prefix.matches('\n').count() + 1;
    let col = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
    (line, col)
}

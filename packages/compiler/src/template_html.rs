use oxc_ast::ast::{
    JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild, JSXElement, JSXElementName,
};
use oxc_span::Span;

use crate::codegen::CompileError;

/// HTML elements with no closing tag. Children on these are an error.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
    "track", "wbr",
];

/// Elements whose content model is "raw text": browsers don't apply the
/// normal character-reference decoding to their children (script/style
/// bodies are CDATA-ish). Our serializer always entity-escapes, which would
/// silently corrupt raw-text content (e.g. `<script>a < b</script>` would
/// become `&lt;`, changing program behavior), so these tags are an explicit
/// error instead. `textarea`/`title` use the normal (escapable) text
/// content model and stay allowed.
const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

/// The curated set of named HTML character references this compiler
/// understands. Expand when a real need arises; anything outside this set
/// is an explicit `Unsupported` error rather than silently passing through
/// wrong (see `decode_character_references`).
fn lookup_named_reference(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{00A0}',
        "copy" => '\u{00A9}',
        "reg" => '\u{00AE}',
        "trade" => '\u{2122}',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        "hellip" => '\u{2026}',
        "laquo" => '\u{00AB}',
        "raquo" => '\u{00BB}',
        "times" => '\u{00D7}',
        "middot" => '\u{00B7}',
        _ => return None,
    })
}

/// Outcome of trying to parse an HTML character reference starting right
/// after an `&`.
enum ReferenceMatch {
    /// A well-formed, known reference: the decoded char and the number of
    /// bytes consumed after the `&` (including the trailing `;`).
    Decoded(char, usize),
    /// A well-formed named reference (`&name;`) not in our curated table.
    UnknownNamed(String),
    /// A well-formed numeric reference (`&#…;`) whose code point is
    /// invalid (surrogate, out of range, or zero). Carries the raw
    /// reference text (without the leading `&`) for the error message.
    InvalidNumeric(String),
}

/// Tries to match an HTML character reference in `after_amp` (the text
/// immediately following an `&`). Returns `None` when `after_amp` doesn't
/// form a well-terminated reference (no `;`, or an empty digit run) — the
/// `&` is then just a bare ampersand and is left alone for the escaper.
fn match_reference(after_amp: &str) -> Option<ReferenceMatch> {
    if let Some(rest) = after_amp.strip_prefix('#') {
        let (is_hex, digits_part) =
            if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
                (true, hex)
            } else {
                (false, rest)
            };
        let digit_count = digits_part
            .chars()
            .take_while(|c| if is_hex { c.is_ascii_hexdigit() } else { c.is_ascii_digit() })
            .count();
        if digit_count == 0 {
            return None;
        }
        let digits = &digits_part[..digit_count];
        let after_digits = &digits_part[digit_count..];
        if !after_digits.starts_with(';') {
            return None;
        }
        let prefix_len = 1 + usize::from(is_hex); // '#' plus optional 'x'
        let consumed = prefix_len + digit_count + 1; // + ';'
        let value = if is_hex {
            u32::from_str_radix(digits, 16).ok()
        } else {
            digits.parse::<u32>().ok()
        };
        return Some(match value.filter(|&v| v != 0).and_then(char::from_u32) {
            Some(c) => ReferenceMatch::Decoded(c, consumed),
            None => ReferenceMatch::InvalidNumeric(after_amp[..consumed].to_string()),
        });
    }

    let mut name_end = 0;
    for (i, c) in after_amp.char_indices() {
        let ok = if i == 0 { c.is_ascii_alphabetic() } else { c.is_ascii_alphanumeric() };
        if !ok {
            break;
        }
        name_end = i + c.len_utf8();
    }
    if name_end == 0 {
        return None;
    }
    let name = &after_amp[..name_end];
    if !after_amp[name_end..].starts_with(';') {
        return None;
    }
    let consumed = name_end + 1;
    Some(match lookup_named_reference(name) {
        Some(c) => ReferenceMatch::Decoded(c, consumed),
        None => ReferenceMatch::UnknownNamed(name.to_string()),
    })
}

/// Decodes HTML character references (`&amp;`, `&#65;`, `&#x41;`, …) in raw
/// JSX text/attribute values before escaping.
///
/// oxc does not decode these the way Babel does: `JSXText.value` and JSX
/// attribute `StringLiteral.value` are the raw source bytes, so `&amp;`
/// arrives as the literal three characters `&`, `a`, `m`, `p`, `;`. Running
/// that straight through `escape_text`/`escape_attr` would re-escape the
/// `&` and silently double-encode it (`&amp;` -> `&amp;amp;`, which renders
/// as the literal text "&amp;" instead of "&"). Decoding first makes the
/// round trip correct: `&amp;` decodes to `&`, which is then re-escaped to
/// `&amp;`.
///
/// A bare `&` that doesn't form a well-terminated reference (no `;`, e.g.
/// `fish & chips`, `a && b`, `&#;`) is left alone here and picked up by the
/// escaper as today.
fn decode_character_references(span: Span, input: &str) -> Result<String, CompileError> {
    if !input.contains('&') {
        return Ok(input.to_string());
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        let Some(amp_pos) = rest.find('&') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..amp_pos]);
        let after_amp = &rest[amp_pos + 1..];
        match match_reference(after_amp) {
            Some(ReferenceMatch::Decoded(c, consumed)) => {
                out.push(c);
                rest = &after_amp[consumed..];
            }
            Some(ReferenceMatch::UnknownNamed(name)) => {
                return Err(unsupported(
                    span,
                    format!(
                        "unknown HTML character reference `&{name};` (not in the supported named-entity set)"
                    ),
                ));
            }
            Some(ReferenceMatch::InvalidNumeric(raw)) => {
                return Err(unsupported(
                    span,
                    format!(
                        "invalid numeric character reference `&{raw}` (invalid or out-of-range code point)"
                    ),
                ));
            }
            None => {
                out.push('&');
                rest = after_amp;
            }
        }
    }
    Ok(out)
}

/// Serializes a fully static JSX element tree to the HTML string embedded
/// in a `template()` call. Anything dynamic or deferred returns
/// `CompileError::Unsupported` with a span — this function is the single
/// authority on what "static" means for sub-cycle 1.
///
/// JSXText is emitted verbatim (whitespace included); JSX
/// whitespace-collapsing semantics arrive with dynamic text handling in
/// sub-cycle 2.
pub fn serialize_static(element: &JSXElement) -> Result<String, CompileError> {
    let mut out = String::new();
    write_element(element, &mut out)?;
    Ok(out)
}

fn unsupported(span: Span, what: impl Into<String>) -> CompileError {
    CompileError::Unsupported { span, what: what.into() }
}

fn write_element(el: &JSXElement, out: &mut String) -> Result<(), CompileError> {
    let name = match &el.opening_element.name {
        // Lowercase native tags parse as Identifier.
        JSXElementName::Identifier(ident) => ident.name.as_str(),
        // Capitalized component references parse as IdentifierReference.
        JSXElementName::IdentifierReference(ident) => {
            return Err(unsupported(
                el.span,
                format!("component tag <{}> (control-flow/component codegen arrives in sub-cycle 3)", ident.name),
            ));
        }
        _ => {
            return Err(unsupported(el.span, "complex JSX tag (member/namespaced/this expression)"));
        }
    };

    if RAW_TEXT_ELEMENTS.contains(&name) {
        return Err(unsupported(
            el.span,
            format!(
                "<{name}> is a raw-text element; its content can't be entity-escaped like normal HTML (sub-cycle 2+)"
            ),
        ));
    }

    out.push('<');
    out.push_str(name);

    for item in &el.opening_element.attributes {
        match item {
            JSXAttributeItem::Attribute(attr) => {
                let attr_name = match &attr.name {
                    JSXAttributeName::Identifier(id) => id.name.as_str().to_string(),
                    JSXAttributeName::NamespacedName(ns) => {
                        format!("{}:{}", ns.namespace.name.as_str(), ns.name.name.as_str())
                    }
                };
                if attr_name.starts_with("v-") || attr_name.starts_with("use:") {
                    return Err(unsupported(
                        attr.span,
                        format!("`{attr_name}` is reserved for the directives layer (cycle 4), not yet supported"),
                    ));
                }
                match &attr.value {
                    None => {
                        out.push(' ');
                        out.push_str(&attr_name);
                    }
                    Some(JSXAttributeValue::StringLiteral(lit)) => {
                        let decoded = decode_character_references(lit.span, lit.value.as_str())?;
                        out.push(' ');
                        out.push_str(&attr_name);
                        out.push_str("=\"");
                        out.push_str(&escape_attr(&decoded));
                        out.push('"');
                    }
                    Some(JSXAttributeValue::ExpressionContainer(c)) => {
                        return Err(unsupported(c.span, "dynamic attribute value (sub-cycle 2)"));
                    }
                    Some(_) => {
                        return Err(unsupported(attr.span, "unsupported attribute value shape"));
                    }
                }
            }
            JSXAttributeItem::SpreadAttribute(sp) => {
                return Err(unsupported(
                    sp.span,
                    "spread attributes on native elements (TEZ102, sub-cycle 3)",
                ));
            }
        }
    }

    if VOID_ELEMENTS.contains(&name) {
        if !el.children.is_empty() {
            return Err(unsupported(el.span, format!("children on void element <{name}>")));
        }
        out.push('>');
        return Ok(());
    }

    out.push('>');
    for child in &el.children {
        match child {
            JSXChild::Text(t) => {
                let decoded = decode_character_references(t.span, t.value.as_str())?;
                out.push_str(&escape_text(&decoded));
            }
            JSXChild::Element(child_el) => write_element(child_el, out)?,
            JSXChild::ExpressionContainer(c) => {
                return Err(unsupported(c.span, "JSX expression in template (sub-cycle 2)"));
            }
            JSXChild::Fragment(f) => {
                return Err(unsupported(f.span, "fragment child (sub-cycle 3)"));
            }
            JSXChild::Spread(s) => {
                return Err(unsupported(s.span, "spread child"));
            }
        }
    }
    out.push_str("</");
    out.push_str(name);
    out.push('>');
    Ok(())
}

// `&` must be replaced first in both escapers.
fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

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
                        out.push(' ');
                        out.push_str(&attr_name);
                        out.push_str("=\"");
                        out.push_str(&escape_attr(lit.value.as_str()));
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
            JSXChild::Text(t) => out.push_str(&escape_text(t.value.as_str())),
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

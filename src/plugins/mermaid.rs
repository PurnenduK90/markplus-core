use super::{Plugin, escape_attr, escape_typst_string};
use serde_json::Value;

pub struct MermaidPlugin;

impl Plugin for MermaidPlugin {
    fn name(&self) -> &'static str {
        "mermaid"
    }

    fn render_html(&self, token: &Value) -> Option<String> {
        let raw = token.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        Some(format!("<div class=\"markplus-plugin\" data-plugin=\"mermaid\" data-raw=\"{}\"></div>", escape_attr(raw)))
    }

    fn render_typst(&self, token: &Value) -> Option<String> {
        let raw = token.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        Some(format!("#markplus-mermaid(\"{}\")", escape_typst_string(raw)))
    }
}

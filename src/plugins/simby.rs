use super::{Plugin, escape_attr, escape_typst_string};
use serde_json::Value;

pub struct SimbyPlugin;

impl Plugin for SimbyPlugin {
    fn name(&self) -> &'static str {
        "simby"
    }

    fn render_html(&self, token: &Value) -> Option<String> {
        let raw = token.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        Some(format!("<div class=\"markplus-plugin\" data-plugin=\"simby\" data-raw=\"{}\"></div>", escape_attr(raw)))
    }

    fn render_typst(&self, token: &Value) -> Option<String> {
        let raw = token.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        Some(format!("#markplus-simby(\"{}\")", escape_typst_string(raw)))
    }
}

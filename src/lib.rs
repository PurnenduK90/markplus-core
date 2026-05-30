pub mod config;
pub mod event_filter;
pub mod plugins;
pub mod targets;
use serde::{Deserialize, Serialize};

use crate::config::{CompilationMode, OutputTarget};
use crate::targets::json::SiteAsset;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    InvalidFrontmatter(String),
    UnsupportedMode(&'static str),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFrontmatter(msg) => write!(f, "invalid frontmatter: {msg}"),
            Self::UnsupportedMode(msg)    => f.write_str(msg),
        }
    }
}

impl std::error::Error for CompileError {}

// ---------------------------------------------------------------------------
// Result type returned to the native Tauri / deploy-pass caller
// ---------------------------------------------------------------------------

/// Everything produced from a single `.md` source file in one native pass.
///
/// The Tauri deploy pass calls `compile_document()` once per note and writes:
/// - `note_101.json` ← `site_asset.to_json()`     (meta + tokens)
///
/// For the live editor preview the Tauri frontend calls `render_html()` /
/// `render_typst()` directly, bypassing the deploy asset entirely.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompileResult {
    /// The two static files written to `dist/static_api/`.
    pub site_asset: SiteAsset,
    /// The plain Markdown body (no frontmatter) for AI consumption.
    pub stripped_md: String,
    /// Rendered output string for the requested target (HTML or Typst).
    pub rendered: String,
    /// Which target was rendered.
    pub target: OutputTarget,
}

// ---------------------------------------------------------------------------
// Native API (Tauri / deploy pass)
// ---------------------------------------------------------------------------

/// Full compile pass over a raw `.md` file that may contain frontmatter.
///
/// Returns a [`CompileResult`] whose `site_asset` field holds:
/// - `meta`  — parsed frontmatter as a JSON value tree (`None` if absent)
/// - `tokens` — the compiled AST of the body
///
/// The caller is responsible for writing the output file:
/// ```ignore
/// let cr = compile_document(raw_md, OutputTarget::Html)?;
/// std::fs::write("dist/static_api/note_101.json", &cr.site_asset.to_json().unwrap())?;
/// ```
pub fn compile_document(raw_md: &str, target: OutputTarget) -> Result<CompileResult, CompileError> {
    let registry = plugins::default_registry();
    let doc = event_filter::intercept_events(raw_md, CompilationMode::Native, &registry);
    let meta = targets::json::parse_frontmatter(doc.frontmatter.as_deref())?;

    let tokens = targets::ast::compile_to_ast(doc.events);
    let rendered = render(target, &tokens, &registry);

    Ok(CompileResult {
        site_asset: SiteAsset { meta, tokens },
        stripped_md: strip_frontmatter(raw_md).to_string(),
        rendered,
        target,
    })
}

pub fn strip_frontmatter(raw: &str) -> &str {
    if raw.starts_with("---\n") || raw.starts_with("---\r\n") {
        let suffix = if raw.starts_with("---\n") { &raw[4..] } else { &raw[5..] };
        if let Some(end) = suffix.find("\n---") {
            let after = &suffix[end + 4..];
            if after.starts_with('\n') {
                return &after[1..];
            } else if after.starts_with("\r\n") {
                return &after[2..];
            }
            return after;
        }
    }
    raw
}



// ---------------------------------------------------------------------------
// Native live-preview API  (Tauri editor mode — raw .md → rendered output)
//
// The editor holds the raw .md string in RAM.  Every keystroke calls one of
// these functions to refresh the split-pane preview.  Frontmatter is stripped
// by the parser but is NOT returned — the caller doesn't need it here.
//
// Usage in Tauri:
//   let html = preview_html(&editor_content);   // sub-millisecond
// ---------------------------------------------------------------------------

/// Compile a raw `.md` string (may include frontmatter) to HTML for the
/// live editor split-pane preview.  Frontmatter is stripped silently.
pub fn preview_html(raw_md: &str) -> String {
    let registry = plugins::default_registry();
    let doc = event_filter::intercept_events(raw_md, CompilationMode::Native, &registry);
    let tokens = targets::ast::compile_to_ast(doc.events);
    render(OutputTarget::Html, &tokens, &registry)
}

/// Compile a raw `.md` string (may include frontmatter) to Typst markup for
/// the live editor preview or on-demand PDF generation.
pub fn preview_typst(raw_md: &str) -> String {
    let registry = plugins::default_registry();
    let doc = event_filter::intercept_events(raw_md, CompilationMode::Native, &registry);
    let tokens = targets::ast::compile_to_ast(doc.events);
    render(OutputTarget::Typst, &tokens, &registry)
}

// ---------------------------------------------------------------------------
// Wasm API (web client — receives pre-stripped body from note_101.json)
//
// The JS caller fetches note_101.json once, reads json.body (plain markdown,
// no frontmatter), and passes that string here. There is no frontmatter to
// strip — the deploy pass already did that.
// ---------------------------------------------------------------------------

/// Compile a plain markdown string to HTML.
///
/// **Wasm callers:** pass `json.body` from the fetched `note_101.json` asset.
/// Do NOT pass a raw file that still contains a `---` frontmatter block;
/// the `---` lines will be rendered as thematic breaks, not silently discarded.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compile_to_html(tokens_json: String) -> String {
    let registry = plugins::default_registry();
    let tokens: Vec<serde_json::Value> = serde_json::from_str(&tokens_json).unwrap_or_default();
    render(OutputTarget::Html, &tokens, &registry)
}

/// Compile a plain markdown string to Typst markup.
///
/// **Wasm callers:** pass `json.body` from the fetched `note_101.json` asset.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compile_to_typst(tokens_json: String) -> String {
    let registry = plugins::default_registry();
    let tokens: Vec<serde_json::Value> = serde_json::from_str(&tokens_json).unwrap_or_default();
    render(OutputTarget::Typst, &tokens, &registry)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn render(target: OutputTarget, tokens: &[serde_json::Value], registry: &plugins::PluginRegistry) -> String {
    match target {
        OutputTarget::Html  => targets::html::compile_to_html(tokens, registry),
        OutputTarget::Typst => targets::typst::compile_to_typst(tokens, registry),
    }
}



// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const FULL_DOC: &str = r#"---
title: note_101
category: hardware
tags:
  - mixer
  - optics
---
# High Frequency Core

```simby
[RFSoC Mixer] -> [Filter Block] -> [Optical Modulator]
```"#;

    #[test]
    fn compile_document_splits_meta_and_body() {
        let cr = compile_document(FULL_DOC, OutputTarget::Html).unwrap();

        // meta is populated from frontmatter
        assert_eq!(
            cr.site_asset.meta,
            Some(json!({
                "title": "note_101",
                "category": "hardware",
                "tags": ["mixer", "optics"]
            }))
        );
        // tokens is populated
        assert!(!cr.site_asset.tokens.is_empty());
    }

    #[test]
    fn compile_document_html_renders_plugin_and_heading() {
        let cr = compile_document(FULL_DOC, OutputTarget::Html).unwrap();

        assert!(cr.rendered.contains("<h1>High Frequency Core</h1>"));
        assert!(cr.rendered.contains("data-plugin=\"simby\""));
        assert!(cr.rendered.contains("[RFSoC Mixer] -&gt; [Filter Block] -&gt; [Optical Modulator]"));
    }

    #[test]
    fn compile_document_typst_renders_plugin_call() {
        let cr = compile_document(FULL_DOC, OutputTarget::Typst).unwrap();

        assert!(cr.rendered.contains("= High Frequency Core"));
        assert!(cr.rendered.contains(
            "#markplus-simby(\"[RFSoC Mixer] -> [Filter Block] -> [Optical Modulator]\")"
        ));
    }

    #[test]
    fn site_asset_serializes_to_json() {
        let cr = compile_document(FULL_DOC, OutputTarget::Html).unwrap();
        let json_str = cr.site_asset.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["meta"]["title"], "note_101");
        assert!(parsed["tokens"].is_array());
    }

    #[test]
    fn no_frontmatter_document_produces_none_meta() {
        let md = "# Simple\n\nJust a note.";
        let cr = compile_document(md, OutputTarget::Html).unwrap();
        assert!(cr.site_asset.meta.is_none());
        assert!(!cr.site_asset.tokens.is_empty());
    }

    #[test]
    fn invalid_frontmatter_returns_error() {
        let md = "---\ntitle: [broken\n---\n# Oops";
        let err = compile_document(md, OutputTarget::Html).unwrap_err();
        assert!(matches!(err, CompileError::InvalidFrontmatter(_)));
    }

    #[test]
    fn wasm_plain_body_does_not_strip_leading_dashes() {
        // In wasm mode the caller passes the body already stripped from JSON.
        // If they mistakenly pass raw file content, --- renders as <hr>, not
        // silently disappears — this is intentional and documented.
        let body = "# Clean Note\n\nNo frontmatter.";
        let registry = plugins::default_registry();
        let doc = event_filter::intercept_events(body, CompilationMode::Native, &registry);
        let tokens = targets::ast::compile_to_ast(doc.events);
        let html = render(OutputTarget::Html, &tokens, &registry);
        assert!(html.contains("<h1>Clean Note</h1>"));
    }



    // ── Mode: native live preview (raw .md → HTML) ─────────────────────────

    #[test]
    fn preview_html_strips_frontmatter_and_renders_body() {
        let html = preview_html(FULL_DOC);

        // Body content rendered
        assert!(html.contains("<h1>High Frequency Core</h1>"));
        assert!(html.contains("data-plugin=\"simby\""));
        // Frontmatter must be silently stripped, not rendered
        assert!(!html.contains("title: note_101"));
        assert!(!html.contains("category:"));
    }

    #[test]
    fn preview_typst_strips_frontmatter_and_renders_body() {
        let typst = preview_typst(FULL_DOC);

        assert!(typst.contains("= High Frequency Core"));
        assert!(typst.contains("#markplus-simby("));
        assert!(!typst.contains("title: note_101"));
    }


}
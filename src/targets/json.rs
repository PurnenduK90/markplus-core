use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::CompileError;

// ---------------------------------------------------------------------------
// The static site asset emitted by the native deploy pass.
//
// note_101.json  →  { meta: {...frontmatter}, tokens: [...] }
//   Web client fetches this once, then pipes `tokens` through wasm
//   compile_to_html() or compile_to_typst() entirely on the client side.
//
// note_101.md    →  bare markdown body (no frontmatter)
//   Consumed only by AI / LLM tooling that reads plain text off disk.
// ---------------------------------------------------------------------------

/// The JSON asset written to `dist/static_api/note_101.json`.
///
/// The web client fetches this file once per page. The `meta` field is
/// displayed immediately (title, tags, date). The `tokens` field is piped
/// through the wasm `compile_to_html()` export to render the article, or
/// through `compile_to_typst()` when the user requests a PDF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteAsset {
    /// Frontmatter metadata deserialized from YAML into a JSON value tree.
    /// `null` when the document has no frontmatter block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,

    /// The parsed AST tokens. This is the array the
    /// wasm client receives and compiles on the fly.
    pub tokens: Vec<Value>,
}

impl SiteAsset {
    /// Serialize to the `note_101.json` wire format.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Pretty-print variant for human-readable files.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ---------------------------------------------------------------------------
// Frontmatter parsing (native only — serde_yml is not compiled into wasm)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_frontmatter(raw: Option<&str>) -> Result<Option<Value>, CompileError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(yaml) => serde_yml::from_str(yaml)
            .map(Some)
            .map_err(|e| CompileError::InvalidFrontmatter(e.to_string())),
        None => Ok(None),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn parse_frontmatter(raw: Option<&str>) -> Result<Option<Value>, CompileError> {
    // In wasm mode the caller already has the pre-built JSON and passes only
    // the stripped `tokens` array — frontmatter never reaches this path.
    if raw.is_some() {
        return Err(CompileError::UnsupportedMode(
            "frontmatter parsing is not available on wasm targets; \
             pass the tokens from the fetched JSON instead",
        ));
    }
    Ok(None)
}

//    Copyright [2026] [Purnendu Kumar]

//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at

//        http://www.apache.org/licenses/LICENSE-2.0

//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

//! Wire format for the compiled site asset (`note_XXX.json`).
//!
//! [`SiteAsset`] is the top-level output of [`crate::parse_document`].
//! It contains:
//! - `schema` — integer version so renderers can detect breaking changes.
//! - `meta` — parsed YAML frontmatter as a JSON value tree (native only).
//! - `ast` — array of MarkPlus block nodes (see [`crate::ast`]).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::CompileError;

// ---------------------------------------------------------------------------
// Wire format: note_XXXX.json
//
//  { "schema": 1, "meta": {...}, "ast": [...] }
//
//  Web client fetches this once per page.
//  - "meta" is displayed immediately (title, date, tags).
//  - "ast"  is passed to a renderer (wasm or external) to produce HTML/Typst.
// ---------------------------------------------------------------------------

/// The JSON asset written to `dist/static_api/note_XXX.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteAsset {
    /// Schema version — allows renderers to detect incompatible AST shapes.
    pub schema: u32,
    /// Frontmatter metadata deserialized from YAML into a JSON value tree.
    /// `null` when the document has no frontmatter block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    /// MarkPlus AST — array of block nodes.
    pub ast: Vec<Value>,
}

impl SiteAsset {
    /// Current wire-format schema version for serialized site assets.
    pub const SCHEMA_VERSION: u32 = 1;

    /// Build a site asset from optional frontmatter metadata and AST blocks.
    pub fn new(meta: Option<Value>, ast: Vec<Value>) -> Self {
        Self {
            schema: Self::SCHEMA_VERSION,
            meta,
            ast,
        }
    }

    /// Serialize to compact JSON (wire format).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty-printed JSON (debug / human-readable).
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ---------------------------------------------------------------------------
// Frontmatter parsing (native only — serde_yml not compiled into wasm)
// ---------------------------------------------------------------------------

/// Parse YAML frontmatter text into a JSON value tree (native targets only).
///
/// Returns `None` when `raw` is `None` or empty.
/// Returns `Err(CompileError::InvalidFrontmatter)` on malformed YAML.
#[cfg(not(target_arch = "wasm32"))]
pub fn parse_frontmatter(raw: Option<&str>) -> Result<Option<Value>, CompileError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(yaml) => serde_yml::from_str(yaml)
            .map(Some)
            .map_err(|e| CompileError::InvalidFrontmatter(e.to_string())),
        None => Ok(None),
    }
}

/// No-op on wasm targets — frontmatter is always `None`.
///
/// `serde_yml` is not compiled into the wasm binary. The native deploy pass
/// is responsible for parsing frontmatter before writing the `note.json` asset.
#[cfg(target_arch = "wasm32")]
pub fn parse_frontmatter(_raw: Option<&str>) -> Result<Option<Value>, CompileError> {
    Ok(None)
}

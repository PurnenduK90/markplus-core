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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
}

/// The JSON asset written to `dist/static_api/note_XXX.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteAsset {
    /// Schema version — allows renderers to detect incompatible AST shapes.
    pub schema: SchemaVersion,
    /// Frontmatter metadata deserialized from YAML into a JSON value tree.
    /// `null` when the document has no frontmatter block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    /// MarkPlus AST — array of block nodes.
    pub ast: Vec<Value>,
}

impl SiteAsset {
    /// Current wire-format schema version for serialized site assets.
    pub const SCHEMA_MAJOR: u32 = 1;
    pub const SCHEMA_MINOR: u32 = 1;

    /// Build a site asset from optional frontmatter metadata and AST blocks.
    pub fn new(meta: Option<Value>, ast: Vec<Value>) -> Self {
        Self {
            schema: SchemaVersion { major: Self::SCHEMA_MAJOR, minor: Self::SCHEMA_MINOR },
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

// ---------------------------------------------------------------------------
// JSON validation helpers (native only, lightweight)
// ---------------------------------------------------------------------------

/// Validate a parsed JSON `Value` representing a SiteAsset wire format.
///
/// Extracted validator that operates on an existing `Value` to avoid
/// duplicate parsing when callers already have the JSON tree.
#[cfg(not(target_arch = "wasm32"))]
pub fn validate_asset_json_value(v: &serde_json::Value) -> Result<(), Vec<String>> {
    let mut errs: Vec<String> = Vec::new();

    if !v.is_object() {
        return Err(vec!["top-level JSON is not an object".into()]);
    }
    let obj = v.as_object().unwrap();

    // schema
    match obj.get("schema") {
        Some(Value::Object(sv)) => {
            let major = sv.get("major").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let minor = sv.get("minor").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            if major != SiteAsset::SCHEMA_MAJOR {
                errs.push(format!(
                    "unexpected schema major version: {} (expected {})",
                    major,
                    SiteAsset::SCHEMA_MAJOR
                ));
            }
        }
        _ => errs.push("missing or invalid 'schema' field (must be an object with major/minor)".into()),
    }

    // meta
    if let Some(meta) = obj.get("meta")
        && !(meta.is_object() || meta.is_null())
    {
        errs.push("'meta' must be an object or null".into());
    }

    // ast
    match obj.get("ast") {
        Some(astv) if astv.is_array() => {}
        _ => errs.push("missing or invalid 'ast' field (array)".into()),
    }

    if errs.is_empty() { Ok(()) } else { Err(errs) }
}

/// Lightweight validation of the SiteAsset JSON string.
///
/// This validates the top-level wire format expected by markplus_core:
/// - top-level object
/// - integer `schema` field equal to SiteAsset::SCHEMA_VERSION
/// - optional `meta` (object or null)
/// - required `ast` array
///
/// Returns Ok(()) when the basic shape is correct, or Err(vec![...]) with
/// human-readable error messages when invalid.
#[cfg(not(target_arch = "wasm32"))]
pub fn validate_asset_json_str(s: &str) -> Result<(), Vec<String>> {
    let v: serde_json::Value = match serde_json::from_str(s) {
        Ok(v) => v,
        Err(e) => return Err(vec![format!("invalid JSON: {}", e)]),
    };
    validate_asset_json_value(&v)
}

/// Read a JSON file from `path`, validate it with the lightweight checker,
/// and deserialize into a [`SiteAsset`]. Returns Err(String) on any failure.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_and_validate_asset(path: &std::path::Path) -> Result<SiteAsset, String> {
    let v = read_asset_json(path)?;
    validate_asset_json_value(&v).map_err(|errs| errs.join("; "))?;
    serde_json::from_value(v).map_err(|e| e.to_string())
}

/// Read a JSON file and return the parsed JSON `Value` without validation.
/// Convenience wrapper for callers that only need the raw JSON tree.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_asset_json(path: &std::path::Path) -> Result<Value, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// JSON data → AST helpers (native only)
//
// Convert arbitrary JSON data files into displayable AST nodes.
// Distinct from the SiteAsset wire format above.
// ---------------------------------------------------------------------------

/// Parse a JSON data string and return AST block nodes.
///
/// Behavior:
/// - Array of objects → `table` node (object keys as headers)
/// - Single object    → `definition_list` node (key-value pairs)
/// - Anything else    → `fenced` node with `name="json"` (pretty-printed)
#[cfg(not(target_arch = "wasm32"))]
pub fn parse_json_data_to_ast(content: &str) -> Result<Vec<Value>, String> {
    use serde_json::json;

    let parsed: Value = serde_json::from_str(content).map_err(|e| e.to_string())?;

    match &parsed {
        Value::Array(arr) if !arr.is_empty() && arr[0].is_object() => {
            json_array_of_objects_to_table(arr)
        }
        Value::Object(obj) => json_object_to_definition_list(obj),
        _ => {
            let pretty = serde_json::to_string_pretty(&parsed).unwrap_or_default();
            Ok(vec![json!({
                "t": "fenced",
                "name": "json",
                "attrs": {},
                "raw": pretty
            })])
        }
    }
}

/// Read a JSON data file and return AST block nodes.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_json_data_as_ast(path: &std::path::Path) -> Result<Vec<Value>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_json_data_to_ast(&content)
}

#[cfg(not(target_arch = "wasm32"))]
fn json_array_of_objects_to_table(arr: &[Value]) -> Result<Vec<Value>, String> {
    use serde_json::json;

    let keys: Vec<String> = match &arr[0] {
        Value::Object(obj) => obj.keys().cloned().collect(),
        _ => return Ok(vec![]),
    };

    let header_cells: Vec<Value> = keys
        .iter()
        .map(|k| json!({ "t": "_cell", "children": [{ "t": "text", "text": k }] }))
        .collect();

    let align: Vec<Value> = keys.iter().map(|_| json!("none")).collect();

    let rows: Vec<Value> = arr
        .iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            let cells: Vec<Value> = keys
                .iter()
                .map(|k| {
                    let text = match obj.get(k) {
                        Some(Value::String(s)) => s.clone(),
                        Some(v) => v.to_string(),
                        None => String::new(),
                    };
                    json!({ "t": "_cell", "children": [{ "t": "text", "text": text }] })
                })
                .collect();
            Some(Value::Array(cells))
        })
        .collect();

    Ok(vec![json!({
        "t": "table",
        "align": align,
        "headers": header_cells,
        "rows": rows,
    })])
}

#[cfg(not(target_arch = "wasm32"))]
fn json_object_to_definition_list(
    obj: &serde_json::Map<String, Value>,
) -> Result<Vec<Value>, String> {
    use serde_json::json;

    let mut items: Vec<Value> = Vec::new();
    for (key, value) in obj {
        items.push(json!({ "t": "_def_title", "children": [{ "t": "text", "text": key }] }));
        let text = match value {
            Value::String(s) => s.clone(),
            v => v.to_string(),
        };
        items.push(json!({ "t": "_def_body", "children": [{ "t": "text", "text": text }] }));
    }

    Ok(vec![json!({ "t": "definition_list", "items": items })])
}

// ---------------------------------------------------------------------------
// Unit tests for json.rs — moved here from tests/ to improve per-file coverage
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn parse_frontmatter_some() {
        let raw = "title: hello\ndate: 2026-06-07\n";
        let meta = parse_frontmatter(Some(raw)).expect("parse failed");
        assert!(meta.is_some());
        let m = meta.unwrap();
        assert_eq!(m["title"], "hello");
    }

    #[test]
    fn parse_frontmatter_none() {
        let meta = parse_frontmatter(None).expect("parse failed");
        assert!(meta.is_none());
    }

    #[test]
    fn parse_frontmatter_invalid_yaml_errors() {
        let bad = "title: [unclosed\n";
        let err = parse_frontmatter(Some(bad));
        assert!(matches!(err, Err(CompileError::InvalidFrontmatter(_))));
    }

    #[test]
    fn validate_asset_json_str_valid_and_invalid() {
        let asset = SiteAsset::new(None, vec![]);
        let s = asset.to_json().unwrap();
        assert!(validate_asset_json_str(&s).is_ok());

        let bad = r#"{"schema":{"major":99,"minor":0},"ast":[]}"#;
        let err = validate_asset_json_str(bad).unwrap_err();
        assert!(err.iter().any(|e| e.contains("unexpected schema major version")));

        let syntactically_bad = "{ not json ";
        let err2 = validate_asset_json_str(syntactically_bad).unwrap_err();
        assert!(err2.iter().any(|e| e.contains("invalid JSON")));
    }

    #[test]
    fn read_and_validate_asset_file_roundtrip() {
        let asset = SiteAsset::new(Some(json!({"title":"x"})), vec![]);
        let s = asset.to_json_pretty().unwrap();

        let mut path = std::env::temp_dir();
        path.push("markplus_core_test_asset.json");
        fs::write(&path, &s).expect("write failed");

        let got = read_and_validate_asset(&path).expect("read/validate failed");
        assert_eq!(got.schema, asset.schema);
        assert_eq!(got.ast.len(), asset.ast.len());
        assert_eq!(got.meta.unwrap()["title"], "x");

        // Cleanup
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_and_validate_asset_invalid_json_file_returns_err() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_invalid_json.json");
        fs::write(&path, "{ not json }").expect("write failed");
        let res = read_and_validate_asset(&path);
        assert!(res.is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn site_asset_json_roundtrip_and_pretty() {
        let asset = SiteAsset::new(
            Some(json!({"k":"v"})),
            vec![json!({"t":"heading","level":1})],
        );
        let compact = asset.to_json().unwrap();
        let pretty = asset.to_json_pretty().unwrap();
        // round-trip
        let parsed: SiteAsset = serde_json::from_str(&compact).unwrap();
        assert_eq!(parsed, asset);
        // pretty should contain newlines
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn validate_top_level_not_object() {
        let err = validate_asset_json_str("[]").unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.contains("top-level JSON is not an object"))
        );
    }

    // -----------------------------
    // Tests for read_asset_json helper
    // -----------------------------

    #[test]
    fn read_asset_json_success_reads_tree() {
        let asset = SiteAsset::new(Some(json!({"title":"t"})), vec![]);
        let s = asset.to_json_pretty().unwrap();
        let mut path = std::env::temp_dir();
        path.push("markplus_core_test_read_tree.json");
        fs::write(&path, &s).expect("write failed");

        let v = read_asset_json(&path).expect("read json failed");
        assert_eq!(v["schema"]["major"], SiteAsset::SCHEMA_MAJOR);
        assert_eq!(v["schema"]["minor"], SiteAsset::SCHEMA_MINOR);
        assert_eq!(v["meta"]["title"], "t");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_asset_json_invalid_json_errs() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_test_read_tree_invalid.json");
        fs::write(&path, "{ not json }").expect("write failed");
        let res = read_asset_json(&path);
        assert!(res.is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_asset_json_missing_file_errs() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_nonexistent_read_tree_999.json");
        let res = read_asset_json(&path);
        assert!(res.is_err());
    }

    #[test]
    fn validate_missing_schema() {
        let bad = r#"{"ast":[]}"#;
        let err = validate_asset_json_str(bad).unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.contains("missing or invalid 'schema'"))
        );
    }

    #[test]
    fn validate_schema_wrong_type() {
        let bad = r#"{"schema":"one","ast":[]}"#;
        let err = validate_asset_json_str(bad).unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.contains("missing or invalid 'schema'"))
        );
    }

    #[test]
    fn validate_meta_wrong_type() {
        let bad = r#"{"schema":{"major":1,"minor":1},"meta":123,"ast":[]}"#;
        let err = validate_asset_json_str(bad).unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.contains("'meta' must be an object or null"))
        );
    }

    #[test]
    fn validate_ast_wrong_type_or_missing() {
        let bad1 = r#"{"schema":{"major":1,"minor":1}}"#;
        let err1 = validate_asset_json_str(bad1).unwrap_err();
        assert!(err1.iter().any(|e| e.contains("missing or invalid 'ast'")));

        let bad2 = r#"{"schema":{"major":1,"minor":1},"ast":{}}"#;
        let err2 = validate_asset_json_str(bad2).unwrap_err();
        assert!(err2.iter().any(|e| e.contains("missing or invalid 'ast'")));
    }

    #[test]
    fn read_and_validate_asset_missing_file_returns_err() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_nonexistent_12345.json");
        let res = read_and_validate_asset(&path);
        assert!(res.is_err());
    }
}

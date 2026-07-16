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

//! # markplus_core
//!
//! Universal Markdown → AST (JSON) compiler for the MarkPlus ecosystem.
//!
//! ## What this crate does
//!
//! Parses a Markdown document (with optional YAML frontmatter) into a
//! structured, versioned JSON AST.  It does **not** render HTML or Typst —
//! that is the responsibility of a downstream renderer that consumes the AST.
//!
//! ## Output shape
//!
//! ```text
//! {
//!   "schema": 1,
//!   "meta": { "title": "...", "tags": ["..."] },
//!   "ast": [
//!     { "t": "heading", "level": 1, "children": [{ "t": "text", "text": "Hi" }] },
//!     { "t": "fenced", "name": "mermaid", "attrs": {}, "raw": "graph TD\nA-->B" }
//!   ]
//! }
//! ```

pub mod ast;
#[cfg(not(target_arch = "wasm32"))]
/// Source code parsing logic converting code strings into fenced AST nodes.
pub mod code;
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
/// CSV parsing logic converting flat csv lines into table AST nodes.
pub mod csv;
pub mod event_filter;
pub mod json;
#[cfg(not(target_arch = "wasm32"))]
/// Mermaid logic converting .mmd definitions to SVG renderable fenced nodes.
pub mod mermaid;
pub mod yaml;

use json::SiteAsset;
use serde_json::Value;

#[cfg(not(target_arch = "wasm32"))]
pub use json::{read_and_validate_asset, read_asset_json, validate_asset_json_str};

#[cfg(not(target_arch = "wasm32"))]
pub use csv::{CsvReadOptions, read_csv_as_table_ast};

#[cfg(not(target_arch = "wasm32"))]
pub use code::{is_known_code_extension, parse_code_to_fenced_ast, read_code_as_fenced_ast};
#[cfg(not(target_arch = "wasm32"))]
pub use json::{parse_json_data_to_ast, read_json_data_as_ast};
#[cfg(not(target_arch = "wasm32"))]
pub use mermaid::{parse_mermaid_to_fenced_ast, read_mermaid_as_fenced_ast};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur while compiling Markdown into a [`SiteAsset`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The document frontmatter could not be parsed as YAML.
    InvalidFrontmatter(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFrontmatter(msg) => write!(f, "invalid frontmatter: {msg}"),
        }
    }
}

impl std::error::Error for CompileError {}

// ---------------------------------------------------------------------------
// Native API
// ---------------------------------------------------------------------------

/// Parse a raw `.md` source (may contain YAML frontmatter) into a
/// [`SiteAsset`] containing parsed metadata and the full MarkPlus AST.
///
/// Intended for:
/// - native deploy pass  → write `note.json`
/// - Tauri editor preview → render from AST
///
/// ```ignore
/// let asset = parse_document(raw_md)?;
/// std::fs::write("dist/note.json", asset.to_json().unwrap())?;
/// ```
pub fn parse_document(raw_md: &str) -> Result<SiteAsset, CompileError> {
    let (body, frontmatter_str) = event_filter::frontmatter_prepass(raw_md);
    let (masked, directives) = event_filter::directive_prepass(&body);
    let events = event_filter::parse(&masked);
    let rich = event_filter::transform_events(events);
    let ast = ast::build_ast(rich, directives);
    let meta = crate::yaml::parse_yaml_frontmatter(frontmatter_str.as_deref())?;
    Ok(SiteAsset::new(meta, ast))
}

/// Parse a pre-stripped Markdown body (no frontmatter) into an AST array.
///
/// Use this when the caller already has the body string (e.g. from
/// `SiteAsset.body` or any plain Markdown source without frontmatter).
pub fn parse_body(body: &str) -> Vec<Value> {
    let (masked, directives) = event_filter::directive_prepass(body);
    let events = event_filter::parse(&masked);
    let rich = event_filter::transform_events(events);
    ast::build_ast(rich, directives)
}

/// Return the Markdown body with a leading YAML frontmatter block removed.
pub fn strip_frontmatter(raw: &str) -> &str {
    let (_body, _) = event_filter::frontmatter_prepass(raw);
    // Since frontmatter_prepass returns an owned String if it strips it,
    // we have to adjust it. Wait, `frontmatter_prepass` returns `(String, Option<String>)`.
    // It is better to return the slice if possible.
    // We can just rely on the existing logic for `strip_frontmatter` if needed,
    // or just re-implement it as slice-based.
    // I will keep the original implementation here to avoid borrow checker issues with &str.
    let Some(mut offset) = raw
        .strip_prefix("---\n")
        .map(|suffix| raw.len() - suffix.len())
        .or_else(|| {
            raw.strip_prefix("---\r\n")
                .map(|suffix| raw.len() - suffix.len())
        })
    else {
        return raw;
    };

    while offset < raw.len() {
        let remaining = &raw[offset..];
        let line_len = remaining.find('\n').map_or(remaining.len(), |idx| idx + 1);
        let line = remaining[..line_len].trim_end_matches(['\r', '\n']);
        if line == "---" || line == "..." {
            return &raw[offset + line_len..];
        }
        offset += line_len;
    }

    raw
}

// ---------------------------------------------------------------------------
// Wasm API
// ---------------------------------------------------------------------------

/// Returns the AST as a JSON string (body only, no frontmatter).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_ast(body: String) -> String {
    parse_to_ast(body)
}

/// Returns the raw frontmatter YAML string (or "" if none).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_frontmatter(raw_md: String) -> String {
    let (_, raw_yaml) = event_filter::frontmatter_prepass(&raw_md);
    raw_yaml.unwrap_or_default()
}

/// Returns the full SiteAsset JSON: { schema, meta, ast }
/// meta is a parsed JSON object on both native and wasm.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_document_json(raw_md: String) -> String {
    let (body, frontmatter_str) = event_filter::frontmatter_prepass(&raw_md);
    let (masked, directives) = event_filter::directive_prepass(&body);
    let events = event_filter::parse(&masked);
    let rich = event_filter::transform_events(events);
    let ast = ast::build_ast(rich, directives);
    let meta = crate::yaml::parse_yaml_frontmatter(frontmatter_str.as_deref()).unwrap_or(None);
    let asset = SiteAsset::new(meta, ast);
    asset.to_json().unwrap_or_else(|_| "{}".into())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
```

Inline math: $E = mc^2$ and display math:

$$
\int_0^\infty e^{-x} dx = 1
$$

:[LO]{tooltip text="Local oscillator"}
"#;

    fn find_block<'a>(ast: &'a [Value], t: &str) -> &'a Value {
        ast.iter().find(|node| node["t"] == t).unwrap()
    }

    fn children(node: &Value) -> &[Value] {
        node.get("children")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn find_child<'a>(node: &'a Value, t: &str) -> &'a Value {
        children(node).iter().find(|child| child["t"] == t).unwrap()
    }

    #[test]
    fn parse_document_extracts_meta() {
        let asset = parse_document(FULL_DOC).unwrap();
        assert_eq!(asset.schema.major, SiteAsset::SCHEMA_MAJOR);
        assert_eq!(asset.schema.minor, SiteAsset::SCHEMA_MINOR);
        assert_eq!(
            asset.meta,
            Some(json!({
                "title": "note_101",
                "category": "hardware",
                "tags": ["mixer", "optics"]
            }))
        );
        assert!(!asset.ast.is_empty());
    }

    #[test]
    fn ast_contains_heading() {
        let asset = parse_document(FULL_DOC).unwrap();
        let heading = find_block(&asset.ast, "heading");
        assert_eq!(heading["level"], 1);
        assert_eq!(children(heading)[0]["text"], "High Frequency Core");
    }

    #[test]
    fn fenced_block_unified_shape() {
        let asset = parse_document(FULL_DOC).unwrap();
        let fenced = find_block(&asset.ast, "fenced");
        assert_eq!(fenced["name"], "simby");
        assert!(fenced["raw"].as_str().unwrap().contains("RFSoC Mixer"));
    }

    #[test]
    fn fenced_block_with_attrs() {
        let ast = parse_body("```python execute=true linenos\nprint('hi')\n```\n");
        let node = &ast[0];
        assert_eq!(node["t"], "fenced");
        assert_eq!(node["name"], "python");
        assert_eq!(node["attrs"]["execute"], "true");
        assert_eq!(node["attrs"]["linenos"], true);
        assert_eq!(node["raw"], "print('hi')");
    }

    #[test]
    fn inline_math_node() {
        let ast = parse_body("Inline: $E = mc^2$\n");
        let para = find_block(&ast, "paragraph");
        let math = find_child(para, "math_inline");
        assert!(math["src"].as_str().unwrap().contains("mc^2"));
    }

    #[test]
    fn display_math_node() {
        let ast = parse_body("$$\n\\pi\n$$\n");
        let node = find_block(&ast, "math_block");
        assert!(node["src"].as_str().unwrap().contains("pi"));
    }

    #[test]
    fn inline_widget_node() {
        let ast = parse_body("Some :[LO]{tooltip text=\"Local oscillator\"} text\n");
        let para = find_block(&ast, "paragraph");
        let widget = find_child(para, "widget");
        assert_eq!(widget["name"], "tooltip");
        assert_eq!(widget["text"], "LO");
        assert_eq!(widget["attrs"]["text"], "Local oscillator");
    }

    #[test]
    fn list_nested_structure() {
        let ast = parse_body("- alpha\n- beta\n- gamma\n");
        let list = find_block(&ast, "list");
        assert_eq!(list["ordered"], false);
        assert_eq!(list["items"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn ordered_list() {
        let ast = parse_body("1. first\n2. second\n");
        let list = find_block(&ast, "list");
        assert_eq!(list["ordered"], true);
        assert_eq!(list["start"], 1);
    }

    #[test]
    fn no_frontmatter_gives_none_meta() {
        let asset = parse_document("# Plain\n\nA note.\n").unwrap();
        assert!(asset.meta.is_none());
    }

    #[test]
    fn invalid_frontmatter_returns_error() {
        let md = "---\n- list item\n---\n# Body\n";
        assert!(matches!(
            parse_document(md),
            Err(CompileError::InvalidFrontmatter(_))
        ));
    }

    #[test]
    fn site_asset_serializes_with_schema_version() {
        let asset = parse_document(FULL_DOC).unwrap();
        let json: Value = serde_json::from_str(&asset.to_json().unwrap()).unwrap();
        assert_eq!(json["schema"]["major"], SiteAsset::SCHEMA_MAJOR);
        assert_eq!(json["schema"]["minor"], SiteAsset::SCHEMA_MINOR);
        assert!(json["ast"].is_array());
    }

    #[test]
    fn strip_frontmatter_removes_yaml_block() {
        let body = strip_frontmatter(FULL_DOC);
        assert!(!body.contains("title: note_101"));
        assert!(body.contains("# High Frequency Core"));
    }

    #[test]
    fn link_with_attrs() {
        let ast = parse_body("[datasheet](./rf.pdf){tooltip=docs download=true}\n");
        let para = find_block(&ast, "paragraph");
        let link = find_child(para, "link");
        assert_eq!(link["href"], "./rf.pdf");
        assert_eq!(link["attrs"]["tooltip"], "docs");
        assert_eq!(link["attrs"]["download"], "true");
    }

    #[test]
    fn link_without_attrs_still_works() {
        let ast = parse_body("[text](https://example.com)\n");
        let para = find_block(&ast, "paragraph");
        let link = find_child(para, "link");
        assert_eq!(link["href"], "https://example.com");
    }

    #[test]
    fn image_with_attrs() {
        let ast = parse_body("![Mixer chain](./mixer.png){width=480 class=diagram}\n");
        let para = find_block(&ast, "paragraph");
        let img = find_child(para, "image");
        assert_eq!(img["src"], "./mixer.png");
        assert_eq!(img["attrs"]["width"], "480");
        assert_eq!(img["attrs"]["class"], "diagram");
    }

    #[test]
    fn blockquote_with_inline_paragraph_children() {
        let ast = parse_body("> hello **world**\n");
        let blockquote = find_block(&ast, "blockquote");
        let para = find_child(blockquote, "paragraph");
        assert_eq!(children(para)[0]["text"], "hello ");
        assert_eq!(find_child(para, "strong")["t"], "strong");
    }

    #[test]
    fn list_item_with_nested_list() {
        let ast = parse_body("- item\n  - nested\n");
        let list = find_block(&ast, "list");
        let item = &list["items"].as_array().unwrap()[0];
        let nested = item["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|child| child["t"] == "list")
            .unwrap();
        assert_eq!(nested["ordered"], false);
        assert_eq!(nested["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn blockquote_with_fenced_block() {
        let ast = parse_body("> ```js\n> alert(1)\n> ```\n");
        let blockquote = find_block(&ast, "blockquote");
        let fenced = find_child(blockquote, "fenced");
        assert_eq!(fenced["name"], "js");
        assert_eq!(fenced["raw"], "alert(1)");
    }

    #[test]
    fn gfm_alert_note_blockquote_kind() {
        let ast = parse_body("> [!NOTE]\n> body\n");
        let blockquote = find_block(&ast, "blockquote");
        assert_eq!(blockquote["kind"], "note");
    }

    #[test]
    fn gfm_alert_warning_blockquote_kind() {
        let ast = parse_body("> [!WARNING]\n> body\n");
        let blockquote = find_block(&ast, "blockquote");
        assert_eq!(blockquote["kind"], "warning");
    }

    #[test]
    fn inline_math_in_heading_children() {
        let ast = parse_body("# Sum $x$\n");
        let heading = find_block(&ast, "heading");
        assert_eq!(find_child(heading, "math_inline")["src"], "x");
    }

    #[test]
    fn display_math_between_paragraphs() {
        let ast = parse_body("One\n\n$$\nA\n$$\n\nTwo\n");
        let first = ast
            .iter()
            .position(|node| {
                node["t"] == "paragraph"
                    && children(node).iter().any(|child| child["text"] == "One")
            })
            .unwrap();
        let math = ast
            .iter()
            .position(|node| node["t"] == "math_block")
            .unwrap();
        let second = ast
            .iter()
            .rposition(|node| {
                node["t"] == "paragraph"
                    && children(node).iter().any(|child| child["text"] == "Two")
            })
            .unwrap();
        assert!(first < math && math < second);
    }

    #[test]
    fn table_with_column_alignment() {
        let ast = parse_body("| L | C | R |\n| :-- | :-: | --: |\n| a | b | c |\n");
        let table = find_block(&ast, "table");
        assert_eq!(table["align"], json!(["left", "center", "right"]));
    }

    #[test]
    fn table_cell_with_inline_markup() {
        let ast = parse_body("| H |\n| - |\n| **x** |\n");
        let table = find_block(&ast, "table");
        let row = &table["rows"].as_array().unwrap()[0];
        let cell = &row.as_array().unwrap()[0];
        assert_eq!(find_child(cell, "strong")["t"], "strong");
    }

    #[test]
    fn strikethrough_node_in_paragraph() {
        let ast = parse_body("~~gone~~\n");
        let para = find_block(&ast, "paragraph");
        assert_eq!(find_child(para, "del")["t"], "del");
    }

    #[test]
    fn superscript_node_in_paragraph() {
        let ast = parse_body("^up^\n");
        let para = find_block(&ast, "paragraph");
        assert_eq!(find_child(para, "sup")["t"], "sup");
    }

    #[test]
    fn subscript_node_in_paragraph() {
        let ast = parse_body("~down~\n");
        let para = find_block(&ast, "paragraph");
        assert_eq!(find_child(para, "sub")["t"], "sub");
    }

    #[test]
    fn link_with_title() {
        let ast = parse_body("[text](url \"title\")\n");
        let para = find_block(&ast, "paragraph");
        let link = find_child(para, "link");
        assert_eq!(link["title"], "title");
    }

    #[test]
    fn image_without_attrs_omits_attrs_field() {
        let ast = parse_body("![alt](img.png)\n");
        let para = find_block(&ast, "paragraph");
        let image = find_child(para, "image");
        assert_eq!(image["src"], "img.png");
        assert!(image.get("attrs").is_none());
    }

    #[test]
    fn footnote_reference_node() {
        let ast = parse_body("ref[^1]\n\n[^1]: body\n");
        let para = find_block(&ast, "paragraph");
        assert_eq!(find_child(para, "footnote_ref")["label"], "1");
    }

    #[test]
    fn footnote_definition_block() {
        let ast = parse_body("ref[^1]\n\n[^1]: body\n");
        let def = find_block(&ast, "footnote_def");
        assert_eq!(def["label"], "1");
        assert_eq!(find_child(def, "paragraph")["t"], "paragraph");
    }

    #[test]
    fn widget_at_start_of_paragraph() {
        let ast = parse_body(":[LO]{tooltip text=hi} end\n");
        let para = find_block(&ast, "paragraph");
        assert_eq!(children(para)[0]["t"], "widget");
        assert_eq!(children(para)[1]["text"], " end");
    }

    #[test]
    fn widget_at_end_of_paragraph() {
        let ast = parse_body("start :[LO]{tooltip text=hi}\n");
        let para = find_block(&ast, "paragraph");
        let last = children(para).last().unwrap();
        assert_eq!(last["t"], "widget");
        assert_eq!(last["name"], "tooltip");
    }

    #[test]
    fn multiple_widgets_in_same_paragraph() {
        let ast = parse_body("A :[X]{w} B :[Y]{z}\n");
        let para = find_block(&ast, "paragraph");
        assert_eq!(
            children(para)
                .iter()
                .filter(|child| child["t"] == "widget")
                .count(),
            2
        );
    }

    #[test]
    fn indented_code_block_uses_empty_name() {
        let ast = parse_body("    code\n");
        assert_eq!(ast[0]["name"], "");
    }

    #[test]
    fn fenced_block_without_info_has_empty_name() {
        let ast = parse_body("```\nplain\n```\n");
        assert_eq!(ast[0]["name"], "");
    }

    #[test]
    fn fenced_block_flag_only_attr() {
        let ast = parse_body("```python flag\nprint(1)\n```\n");
        assert_eq!(ast[0]["name"], "python");
        assert_eq!(ast[0]["attrs"], json!({ "flag": true }));
    }

    #[test]
    fn strip_frontmatter_supports_dots_closing_delimiter() {
        let raw = "---\ntitle: dotted\n...\n# Body\n";
        assert_eq!(strip_frontmatter(raw), "# Body\n");
    }

    #[test]
    fn strip_frontmatter_without_frontmatter_returns_input() {
        let raw = "# Plain\n\nBody\n";
        assert_eq!(strip_frontmatter(raw), raw);
    }

    #[test]
    fn hard_break_node_in_paragraph() {
        let ast = parse_body("a  \nb\n");
        let para = find_block(&ast, "paragraph");
        assert_eq!(find_child(para, "hard_break")["t"], "hard_break");
    }

    #[test]
    fn site_asset_schema_version_is_1_2() {
        assert_eq!(SiteAsset::SCHEMA_MAJOR, 1);
        assert_eq!(SiteAsset::SCHEMA_MINOR, 2);
    }

    #[test]
    fn site_asset_json_round_trip() {
        let asset = parse_document(FULL_DOC).unwrap();
        let round_trip: SiteAsset = serde_json::from_str(&asset.to_json().unwrap()).unwrap();
        assert_eq!(round_trip, asset);
    }

    #[test]
    fn directive_basic() {
        let ast = parse_body(":::note\nhello\n:::\n");
        let dir = &ast[0];
        assert_eq!(dir["t"], "directive");
        assert_eq!(dir["name"], "note");
        assert_eq!(dir["children"][0]["t"], "paragraph");
        assert_eq!(dir["children"][0]["children"][0]["text"], "hello");
    }

    #[test]
    fn directive_with_attrs() {
        let ast = parse_body(":::callout type=warning\nbody\n:::\n");
        let dir = &ast[0];
        assert_eq!(dir["name"], "callout");
        assert_eq!(dir["attrs"]["type"], "warning");
    }

    #[test]
    fn directive_named_close() {
        let ast = parse_body(":::tabs\nA\n:::/tabs\n");
        let dir = &ast[0];
        assert_eq!(dir["name"], "tabs");
        assert_eq!(dir["children"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn directive_bare_close() {
        let ast = parse_body(":::tabs\nA\n:::\n");
        let dir = &ast[0];
        assert_eq!(dir["name"], "tabs");
        assert_eq!(dir["children"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn directive_nested() {
        let ast = parse_body(":::columns\n:::col\nA\n:::\n:::\n");
        let outer = &ast[0];
        assert_eq!(outer["name"], "columns");
        let inner = &outer["children"][0];
        assert_eq!(inner["name"], "col");
    }

    #[test]
    fn directive_fenced_guard() {
        let ast = parse_body("```\n:::fake\n```\n");
        let fenced = &ast[0];
        assert_eq!(fenced["t"], "fenced");
        assert!(fenced["raw"].as_str().unwrap().contains(":::fake"));
    }

    #[test]
    fn directive_ranges_correct() {
        let ast = parse_body("A\n\n:::note\nB\n:::\n\nC\n");
        assert_eq!(ast.len(), 3);
        assert_eq!(ast[0]["t"], "paragraph");
        assert_eq!(ast[1]["t"], "directive");
        assert_eq!(ast[2]["t"], "paragraph");
    }

    #[test]
    fn directive_mixed_content() {
        let ast = parse_body("# Head\n\n:::note\nA\n:::\n\nPara\n");
        assert_eq!(ast[0]["t"], "heading");
        assert_eq!(ast[1]["t"], "directive");
        assert_eq!(ast[2]["t"], "paragraph");
    }

    #[test]
    fn frontmatter_block_list() {
        let asset = parse_document("---\ntags:\n  - a\n  - b\n---\nhi").unwrap();
        assert_eq!(asset.meta.unwrap()["tags"], json!(["a", "b"]));
    }

    #[test]
    fn frontmatter_nested_object() {
        let asset = parse_document("---\nauthor:\n  name: x\n---\nhi").unwrap();
        assert_eq!(asset.meta.unwrap()["author"]["name"], "x");
    }

    #[test]
    fn frontmatter_flow_list() {
        let asset = parse_document("---\ntags: [a, b]\n---\nhi").unwrap();
        assert_eq!(asset.meta.unwrap()["tags"], json!(["a", "b"]));
    }

    #[test]
    fn frontmatter_bool() {
        let asset = parse_document("---\ndraft: true\n---\nhi").unwrap();
        assert_eq!(asset.meta.unwrap()["draft"], true);
    }

    #[test]
    fn frontmatter_int() {
        let asset = parse_document("---\nweight: 5\n---\nhi").unwrap();
        assert_eq!(asset.meta.unwrap()["weight"], 5);
    }

    #[test]
    fn frontmatter_quoted_string() {
        let asset = parse_document("---\ntitle: \"hello: world\"\n---\nhi").unwrap();
        assert_eq!(asset.meta.unwrap()["title"], "hello: world");
    }

    #[test]
    fn widget_across_softbreak() {
        let ast = parse_body(":[LO]{tooltip\ntext=hi}\n");
        let para = &ast[0];
        let widget = &para["children"][0];
        assert_eq!(widget["t"], "widget");
        assert_eq!(widget["name"], "tooltip");
    }

    #[test]
    fn link_attrs_remainder() {
        let ast = parse_body("[text](url){key=val} more\n");
        let para = &ast[0];
        let children = para["children"].as_array().unwrap();
        assert_eq!(children[0]["t"], "link");
        assert_eq!(children[0]["attrs"]["key"], "val");
        assert_eq!(children[1]["text"], " more");
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn get_ast_returns_json_array() {
        let json_str = crate::get_ast("hi".into());
        let val: Value = serde_json::from_str(&json_str).unwrap();
        assert!(val.is_array());
    }

    // -----------------------------------------------------------------------
    // Re-export tests for native validators at crate root
    // -----------------------------------------------------------------------

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn crate_root_and_module_validate_agree() {
        let asset = crate::json::SiteAsset::new(None, vec![]);
        let s = asset.to_json().unwrap();
        // crate root re-export
        assert!(crate::validate_asset_json_str(&s).is_ok());
        // module function
        assert!(crate::json::validate_asset_json_str(&s).is_ok());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn crate_root_read_and_validate_errs_on_bad_file() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_nonexistent_reexport_123.json");
        let res = crate::read_and_validate_asset(&path);
        assert!(res.is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn crate_root_validate_rejects_invalid_json() {
        let bad = "{ not json }";
        let res = crate::validate_asset_json_str(bad);
        assert!(res.is_err());
    }
}

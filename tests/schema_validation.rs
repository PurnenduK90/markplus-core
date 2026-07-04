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

//! Integration tests: parse every `.md` file in `tests/samples/` and validate
//! the resulting [`SiteAsset`] JSON against `schema/markplus-ast.v1.schema.json`.
//!
//! These tests act as a contract check — if any sample produces an AST node
//! that does not conform to the published schema, this test fails, ensuring
//! `markplus_core` and its schema stay in sync.

use std::fs;
use std::path::Path;

use markplus_core::parse_document;
use serde_json::Value;

/// Load and compile the v1 schema once for all tests in this file.
fn load_schema() -> Value {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schema")
        .join("markplus-ast.v1.1.schema.json");
    let raw = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("Cannot read schema at {:?}: {}", schema_path, e));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("Schema is not valid JSON: {}", e))
}

/// Validate a parsed `SiteAsset` JSON value against the compiled schema,
/// panicking with a descriptive message on failure.
fn assert_valid(schema: &Value, asset_json: &Value, label: &str) {
    let validator = jsonschema::validator_for(schema)
        .unwrap_or_else(|e| panic!("Schema failed to compile: {}", e));

    let mut errors = validator.iter_errors(asset_json).peekable();
    if errors.peek().is_some() {
        println!(
            "FAILING AST: {}",
            serde_json::to_string_pretty(asset_json).unwrap()
        );
        let messages: Vec<String> = errors
            .map(|e| format!("  • {} (at {})", e, e.instance_path()))
            .collect();
        panic!(
            "Schema validation failed for {}:\n{}",
            label,
            messages.join("\n")
        );
    }
}

/// Parse one Markdown file and return its `SiteAsset` as a `serde_json::Value`.
fn parse_to_value(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("Cannot read {:?}: {}", path, e));
    let asset =
        parse_document(&raw).unwrap_or_else(|e| panic!("Parse failed for {:?}: {}", path, e));
    let json_str = asset.to_json().expect("serialisation failed");
    serde_json::from_str(&json_str).expect("round-trip JSON parse failed")
}

// ---------------------------------------------------------------------------
// Per-sample tests
// ---------------------------------------------------------------------------

/// `md_api_reference_sample.md` — API-style markdown with headings, code
/// blocks, tables, and inline markup.
#[test]
fn sample_api_reference_validates() {
    let schema = load_schema();
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/samples/md_api_reference_sample.md");
    let value = parse_to_value(&path);
    assert_valid(&schema, &value, "md_api_reference_sample.md");
}

/// `md_release_notes_sample.md` — changelog-style markdown with lists,
/// headings, inline code, and links.
#[test]
fn sample_release_notes_validates() {
    let schema = load_schema();
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/samples/md_release_notes_sample.md");
    let value = parse_to_value(&path);
    assert_valid(&schema, &value, "md_release_notes_sample.md");
}

/// `md_edge_cases_sample.md` — tests horizontal rules, HTML blocks, task lists, and definition lists.
#[test]
fn sample_edge_cases_validates() {
    let schema = load_schema();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/samples/md_edge_cases_sample.md");
    let value = parse_to_value(&path);
    assert_valid(&schema, &value, "md_edge_cases_sample.md");
}

/// `md_sample_file_200KB.md` — large file stress test.
#[test]
fn sample_200kb_validates() {
    let schema = load_schema();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/samples/md_sample_file_200KB.md");
    let value = parse_to_value(&path);
    assert_valid(&schema, &value, "md_sample_file_200KB.md");
}

// ---------------------------------------------------------------------------
// Full-feature synthetic document
// ---------------------------------------------------------------------------

/// Validate a single document that exercises every supported node type,
/// ensuring complete coverage of the schema in one test.
#[test]
fn full_feature_document_validates() {
    let schema = load_schema();

    let md = r#"---
title: Schema Coverage
tags: [test]
---
# Heading 1 { #h1 .class }

## Heading 2

Plain paragraph with **strong**, *em*, ~~del~~, ^sup^, ~sub~, `code`, $math$.

> [!NOTE]
> GFM alert with a [link](url){class=doc}.

> Regular blockquote.

- Tight list item one
- Tight item two
  - Nested item

1. Ordered item one
2. Ordered item two

| Left | Center | Right |
| :--- | :----: | ----: |
| a    |   b    |     c |
| **x** | y | z |

```python execute=true
print("hello")
```

```mermaid theme=dark
graph TD; A --> B
```

![img](src.png){width=100}

Some :[widget]{tooltip text="hi"} inline.

[^1]: Footnote definition body.

Ref[^1].

- [ ] Task unchecked
- [x] Task checked

$$
\int_0^\infty e^{-x} dx
$$

<em>raw html</em>
"#;

    let asset = parse_document(md).expect("parse failed");
    let json_str = asset.to_json().expect("serialisation failed");
    let value: Value = serde_json::from_str(&json_str).expect("json parse failed");
    assert_valid(&schema, &value, "full_feature_document");
}

// ---------------------------------------------------------------------------
// Schema-level invariant tests
// ---------------------------------------------------------------------------

/// Schema version in `SiteAsset` must be { major: 1, minor: 1 }.
#[test]
fn site_asset_schema_field_is_1_1() {
    let asset = parse_document("# Hi").unwrap();
    let value: Value = serde_json::from_str(&asset.to_json().unwrap()).unwrap();
    assert_eq!(value["schema"]["major"], 1, "schema major field must equal 1");
    assert_eq!(value["schema"]["minor"], 1, "schema minor field must equal 1");
}

/// A document with unknown schema version must fail schema validation.
#[test]
fn wrong_schema_version_fails_validation() {
    let schema = load_schema();
    let invalid = serde_json::json!({ "schema": { "major": 99, "minor": 0 }, "ast": [] });
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(
        validator.validate(&invalid).is_err(),
        "schema major version 99 should fail validation"
    );
}

/// An empty document (no blocks) must produce a valid empty AST.
#[test]
fn empty_document_validates() {
    let schema = load_schema();
    let asset = parse_document("").unwrap();
    let value: Value = serde_json::from_str(&asset.to_json().unwrap()).unwrap();
    assert_valid(&schema, &value, "empty document");
    assert_eq!(value["ast"].as_array().unwrap().len(), 0);
}

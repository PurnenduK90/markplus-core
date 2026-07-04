// Native-only mermaid helpers to produce a fenced AST node.
// Not compiled into wasm builds.

use serde_json::{Map, Value, json};

/// Parse mermaid diagram content into a `fenced` AST node.
///
/// Extra `attrs` are passed through (e.g. `theme=dark`).
/// The `src` key, if present, is filtered out since it was the include path.
pub fn parse_mermaid_to_fenced_ast(content: &str, attrs: &Map<String, Value>) -> Value {
    let mut mermaid_attrs = Map::new();
    for (k, v) in attrs {
        if k != "src" {
            mermaid_attrs.insert(k.clone(), v.clone());
        }
    }
    json!({
        "t": "fenced",
        "name": "mermaid",
        "attrs": mermaid_attrs,
        "raw": content.trim_end()
    })
}

/// Read a `.mmd` file and produce a `fenced` mermaid AST node.
pub fn read_mermaid_as_fenced_ast(path: &std::path::Path) -> Result<Value, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let attrs = Map::new();
    Ok(parse_mermaid_to_fenced_ast(&content, &attrs))
}

/// Read a `.mmd` file with extra attributes and produce a `fenced` mermaid AST node.
pub fn read_mermaid_as_fenced_ast_with_attrs(
    path: &std::path::Path,
    attrs: &Map<String, Value>,
) -> Result<Value, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(parse_mermaid_to_fenced_ast(&content, attrs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_mermaid_basic() {
        let attrs = Map::new();
        let v = parse_mermaid_to_fenced_ast("graph TD\nA --> B\n", &attrs);
        assert_eq!(v["t"], "fenced");
        assert_eq!(v["name"], "mermaid");
        assert_eq!(v["raw"], "graph TD\nA --> B");
    }

    #[test]
    fn parse_mermaid_passes_attrs() {
        let mut attrs = Map::new();
        attrs.insert("theme".into(), json!("dark"));
        attrs.insert("src".into(), json!("./diagram.mmd"));
        let v = parse_mermaid_to_fenced_ast("graph LR\nX --> Y", &attrs);
        assert_eq!(v["attrs"]["theme"], "dark");
        assert!(v["attrs"].get("src").is_none());
    }

    #[test]
    fn read_mermaid_file_roundtrip() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_test_diagram.mmd");
        fs::write(&path, "sequenceDiagram\nA->>B: Hello\n").expect("write");
        let v = read_mermaid_as_fenced_ast(&path).expect("read");
        assert_eq!(v["name"], "mermaid");
        assert!(v["raw"].as_str().unwrap().contains("sequenceDiagram"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_mermaid_with_attrs() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_test_diagram_attrs.mmd");
        fs::write(&path, "graph TD\nA-->B").expect("write");
        let mut attrs = Map::new();
        attrs.insert("theme".into(), json!("forest"));
        let v = read_mermaid_as_fenced_ast_with_attrs(&path, &attrs).expect("read");
        assert_eq!(v["attrs"]["theme"], "forest");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_mermaid_missing_file_errs() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_nonexistent_mmd_999.mmd");
        assert!(read_mermaid_as_fenced_ast(&path).is_err());
    }
}

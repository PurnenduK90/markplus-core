// Native-only code-file helpers to produce a fenced AST node.
// Not compiled into wasm builds.

use serde_json::{json, Value};

/// Map a file extension to its canonical language name for syntax highlighting.
pub fn extension_to_language(ext: &str) -> &str {
    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "jsx" => "jsx",
        "tsx" => "tsx",
        "c" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "h" => "c",
        "cs" => "csharp",
        "java" => "java",
        "go" => "go",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "sh" | "bash" => "bash",
        "zsh" => "zsh",
        "sql" => "sql",
        "r" => "r",
        "lua" => "lua",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "zig" => "zig",
        "nim" => "nim",
        "dart" => "dart",
        "v" => "v",
        "tf" | "hcl" => "hcl",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        other => other,
    }
}

/// Return `true` when the extension is recognised as a source-code file.
pub fn is_known_code_extension(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx"
            | "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "cs"
            | "java" | "go" | "rb" | "php"
            | "swift" | "kt" | "kts" | "scala"
            | "sh" | "bash" | "zsh"
            | "sql" | "r" | "lua"
            | "yaml" | "yml" | "toml" | "xml"
            | "html" | "htm" | "css" | "scss" | "sass"
            | "dockerfile" | "makefile"
            | "zig" | "nim" | "dart" | "v"
            | "tf" | "hcl"
            | "proto" | "graphql" | "gql"
    )
}

/// Parse a code string into a `fenced` AST node.
///
/// `extension` is the bare file extension (e.g. `"py"`, `"rs"`).
pub fn parse_code_to_fenced_ast(content: &str, extension: &str) -> Value {
    let language = extension_to_language(extension);
    json!({
        "t": "fenced",
        "name": language,
        "attrs": {},
        "raw": content.trim_end()
    })
}

/// Read a source-code file from `path` and produce a `fenced` AST node.
pub fn read_code_as_fenced_ast(path: &std::path::Path) -> Result<Value, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    Ok(parse_code_to_fenced_ast(&content, ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn extension_mapping_rust() {
        assert_eq!(extension_to_language("rs"), "rust");
    }

    #[test]
    fn extension_mapping_python() {
        assert_eq!(extension_to_language("py"), "python");
    }

    #[test]
    fn extension_mapping_javascript() {
        assert_eq!(extension_to_language("js"), "javascript");
    }

    #[test]
    fn extension_mapping_passthrough() {
        assert_eq!(extension_to_language("custom"), "custom");
    }

    #[test]
    fn known_code_extension_true() {
        assert!(is_known_code_extension("rs"));
        assert!(is_known_code_extension("py"));
        assert!(is_known_code_extension("cpp"));
        assert!(is_known_code_extension("yaml"));
    }

    #[test]
    fn known_code_extension_false() {
        assert!(!is_known_code_extension("md"));
        assert!(!is_known_code_extension("csv"));
        assert!(!is_known_code_extension("json"));
        assert!(!is_known_code_extension("mmd"));
        assert!(!is_known_code_extension("png"));
    }

    #[test]
    fn parse_code_basic() {
        let v = parse_code_to_fenced_ast("print('hello')\n", "py");
        assert_eq!(v["t"], "fenced");
        assert_eq!(v["name"], "python");
        assert_eq!(v["raw"], "print('hello')");
    }

    #[test]
    fn parse_code_trims_trailing_whitespace() {
        let v = parse_code_to_fenced_ast("fn main() {}\n\n\n", "rs");
        assert_eq!(v["raw"], "fn main() {}");
    }

    #[test]
    fn read_code_file_roundtrip() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_test_code.py");
        fs::write(&path, "x = 42\n").expect("write");
        let v = read_code_as_fenced_ast(&path).expect("read");
        assert_eq!(v["name"], "python");
        assert_eq!(v["raw"], "x = 42");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_code_missing_file_errs() {
        let mut path = std::env::temp_dir();
        path.push("markplus_core_nonexistent_code_999.rs");
        assert!(read_code_as_fenced_ast(&path).is_err());
    }
}

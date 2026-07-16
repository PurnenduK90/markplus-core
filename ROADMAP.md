# markplus-core Roadmap

This document outlines the current state of `markplus-core`, identifies areas for improvement, and proposes a roadmap for future development.

## 1. Parser and AST Improvements

### Richer Source Maps & Diagnostics
- **Current State**: Nodes include byte `range` arrays, but error reporting (like `CompileError::InvalidFrontmatter`) only returns a simple string.
- **Goal**: Implement a robust diagnostic system (similar to `miette` or `codespan`) that provides precise line/column numbers and context snippets for parsing errors, especially for malformed YAML frontmatter or directive syntax.

### Custom YAML Parser Expansion (`yaml.rs`)
- **Current State**: `yaml.rs` is a minimal, pure-Rust implementation designed to avoid heavyweight dependencies (`serde_yml`) and keep Wasm sizes small. It handles basic block lists and maps.
- **Goal**: Expand support for YAML 1.2 features, particularly multi-line strings (`|` and `>`), anchors/aliases, and inline flow mappings. Alternatively, introduce a Cargo feature flag (`full-yaml`) to optionally drop in `serde_yml` for native use-cases where binary size is less critical.

### Extensibility & Plugin Architecture
- **Current State**: Extensions like Math (`$`), Directives (`:::`), and Widgets (`:[]`) are handled via hardcoded regex passes or prepasses (`event_filter::directive_prepass`).
- **Goal**: Formalize an extensible plugin API. Allow consumers to register custom block/inline matchers that integrate directly into the event stream, rather than relying entirely on string masking and prepasses.

## 2. WebAssembly (WASM) Optimizations

### Native JS Object Serialization
- **Current State**: The Wasm bindings (`get_document_json`, `get_ast`) serialize the AST to a JSON string in Rust, which the JavaScript client must then parse via `JSON.parse()`.
- **Goal**: Migrate to `serde-wasm-bindgen` to serialize Rust structs directly into JavaScript objects across the Wasm boundary. This eliminates the string-parsing overhead on the client side and improves performance for large documents.

### Bundle Size Reduction
- **Current State**: Standard compilation includes panic infrastructure and formatting strings.
- **Goal**: Provide a highly optimized Wasm build profile (e.g., `wee_alloc`, `panic="abort"`, LTO) to aggressively shrink the delivered `.wasm` file size for web clients.

## 3. Schema and Ecosystem

### AST Schema Migration Tools
- **Current State**: Schema is currently at `v1.2` (tracked in `SiteAsset`). Renderers are expected to strictly validate against this version.
- **Goal**: Provide a built-in AST migration tool or compatibility layer that can automatically down-convert `v1.2` ASTs into `v1.1` ASTs, preventing ecosystem fracture as the parser evolves.

### HTML and Content Processing
- **Current State**: `html_block` and `raw_html` nodes store raw string contents. 
- **Goal**: Optionally provide a lightweight HTML sanitizer or a DOM-tree parser for HTML blocks so downstream renderers can safely process or filter embedded HTML without bringing their own HTML parsers.

## Proposed Next Steps

1. **Short-term**: Finalize the Wasm bundle optimization and evaluate `serde-wasm-bindgen`.
2. **Medium-term**: Upgrade `yaml.rs` to support multi-line strings, which is a highly requested feature for frontmatter descriptions.
3. **Long-term**: Design the Extensible Plugin API to replace the current `event_filter` masking hacks.

---

> [!NOTE]
> **User Review Required**: Please review the priorities above. Should we focus first on the Wasm optimizations, the YAML parser, or the extensible plugin API? Let me know if there are any other features you'd like added to this roadmap!

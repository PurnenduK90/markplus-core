# MarkPlus Core

MarkPlus Core is a Markdown to HTML / Typst / JSON compiler. It acts as the underlying engine for parsing Markdown (with frontmatter), compiling it to a JSON intermediate representation, and rendering it to various targets like HTML and Typst.

## CLI Documentation

The `mpc` CLI provides commands to compile and preview Markdown files.

### Building

To build the standard `mpc` CLI without native PDF generation:
```bash
cargo build --release
```

To build a single `mpc` binary with native PDF compilation enabled (bundles the Typst compiler):
```bash
cargo build --release --features pdf
```

### `compile`
Full deploy-pass compile. It writes output files based on the requested target:
You can pass `--target site`, `--target html`, `--target typst`, or `--target pdf`.

```bash
mpc compile <FILE> [--target <html|typst|pdf|site>] [--out-dir <DIR>]
```
- `<FILE>`: Source markdown file (may contain frontmatter).
- `--target`: 
  - `site` (default): writes the compiled JSON asset (`<stem>.json`) and stripped markdown (`<stem>.md`).
  - `html`: writes the rendered HTML (`<stem>.html`).
  - `typst`: writes the rendered Typst markup (`<stem>.typ`).
  - `pdf`: writes the compiled PDF (`<stem>.pdf`) (Requires compiling the CLI with `--features pdf`).
- `--out-dir`: Output directory (defaults to the same directory as the input file).

### `preview`
Stream a raw `.md` file through the live-preview pipeline and print it to stdout. Frontmatter is silently stripped in this mode.

```bash
mpc preview <FILE> [--target <html|typst>]
```
- `<FILE>`: Source markdown file.
- `--target`: `html` (default) or `typst`.

---

## API Documentation

The core provides both Native APIs (for Tauri / deploy passes) and WebAssembly APIs (for web clients).

### Native API

Use the native APIs in your Rust application to parse and compile Markdown documents.

#### `compile_document`
Full compile pass over a raw `.md` file (which may contain frontmatter). Returns a `CompileResult` containing the separated frontmatter `meta`, the compiled AST `tokens`, and the `rendered` output.

```rust
pub fn compile_document(raw_md: &str, target: OutputTarget) -> Result<CompileResult, CompileError>
```

#### `preview_html` / `preview_typst`
Compiles a raw `.md` string to HTML or Typst markup for live editor previews. Frontmatter is stripped silently, making it very fast for continuous rendering.

```rust
pub fn preview_html(raw_md: &str) -> String
pub fn preview_typst(raw_md: &str) -> String
```

### Wasm API

WebAssembly callers interact with strings passed from the JavaScript environment. Since the deploy pass already strips frontmatter, Wasm endpoints expect plain Markdown bodies (`json.body`).

#### `compile_to_html` / `compile_to_typst`
Compile a plain Markdown string (or parsed tokens JSON) to HTML/Typst. 

```rust
#[wasm_bindgen]
pub fn compile_to_html(tokens_json: String) -> String

#[wasm_bindgen]
pub fn compile_to_typst(tokens_json: String) -> String
```

---

## How to Create a Plugin / Shortcode

Plugins in MarkPlus allow you to intercept specific code blocks or elements and render them customly for HTML and Typst output targets.

### The `Plugin` Trait

To create a plugin, you must implement the `Plugin` trait defined in `src/plugins/mod.rs`:

```rust
pub trait Plugin: Send + Sync {
    // Unique name of the plugin (e.g., used as the language in a code block)
    fn name(&self) -> &'static str;
    
    // Optional: Parse the raw input into a structured JSON value
    fn parse(&self, raw: &str) -> Option<serde_json::Value> { None }
    
    // Render the element to HTML
    fn render_html(&self, token: &serde_json::Value) -> Option<String> { None }
    
    // Render the element to Typst markup
    fn render_typst(&self, token: &serde_json::Value) -> Option<String> { None }
    
    // Optional: Specify HTML dependencies (scripts, wasm, styles) required by this plugin
    fn html_dependencies(&self) -> Option<HtmlDependencies> { None }
}
```

### Example: Creating a `simby` Plugin

Here is an example of a simple plugin that takes the contents of a ````simby` code block and renders it as a `div` in HTML and a custom function call in Typst.

1. **Implement the plugin logic:**

```rust
use markplus_core::plugins::{Plugin, escape_attr, escape_typst_string};
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
```

2. **Register the plugin:**
You must add your new plugin to the default registry inside `src/plugins/mod.rs`:

```rust
pub fn default_registry() -> PluginRegistry {
    let mut reg = PluginRegistry::new();
    reg.register(Box::new(mermaid::MermaidPlugin));
    reg.register(Box::new(SimbyPlugin)); // Add your new plugin here
    reg
}
```

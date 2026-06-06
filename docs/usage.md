# markplus_core — Usage Guide

This document covers how to use `markplus_core` in all three modes:
- **CLI** (`mpc`) — emit AST JSON from a Markdown file
- **Rust library** — native integration for Tauri / deploy pipelines
- **WebAssembly** — browser / PWA client-side parsing

---

## 1. CLI (`mpc`)

`mpc` is a minimal single-command tool that reads a Markdown file and writes
the MarkPlus AST JSON to stdout. Renderers and post-processors pipe from it.

### Build

```bash
cargo build --release
# Binary: target/release/mpc
```

### Usage

```
mpc [--pretty] <file.md>
mpc --help
```

| Flag | Description |
|---|---|
| `--pretty` / `-p` | Pretty-print JSON (default: compact) |
| `--help` / `-h` | Show help |

### Examples

```bash
# Emit compact JSON
mpc note.md > note.json

# Emit pretty JSON for inspection
mpc --pretty note.md | head -40

# Pipe into a renderer
mpc note.md | my-html-renderer > note.html

# Use in a deploy script
for f in docs/*.md; do
  stem="${f%.md}"
  mpc "$f" > "${stem}.json"
done
```

---

## 2. Rust library (native)

Add to `Cargo.toml`:

```toml
[dependencies]
markplus_core = { path = "../markplus-core" }
# or from registry once published:
# markplus_core = "0.2"
```

### `parse_document` — full document with frontmatter

Use this for the **deploy pass** — when reading raw `.md` files that may
contain a YAML frontmatter block.

```rust
use markplus_core::{parse_document, CompileError};
use std::fs;

fn deploy(path: &str) -> Result<(), CompileError> {
    let raw = fs::read_to_string(path).unwrap();
    let asset = parse_document(&raw)?;

    // asset.schema  — schema version (currently 1)
    // asset.meta    — Option<serde_json::Value> from YAML frontmatter
    // asset.ast     — Vec<serde_json::Value> block nodes

    let json = asset.to_json().unwrap();
    fs::write("dist/note.json", &json).unwrap();

    // Also write the bare body for AI/LLM consumption
    let body = markplus_core::strip_frontmatter(&raw);
    fs::write("dist/note.md", body).unwrap();

    Ok(())
}
```

### `parse_body` — pre-stripped body only

Use this for **live editor preview** when the caller already holds the raw
Markdown in memory (no frontmatter, or frontmatter already stripped).

```rust
use markplus_core::parse_body;

fn preview(body: &str) -> Vec<serde_json::Value> {
    parse_body(body)  // returns Vec<serde_json::Value>
}
```

### `strip_frontmatter` — extract body slice

Returns the body substring after the closing `---` delimiter, without
allocating a new String. Useful for writing the AI-facing `.md` file.

```rust
let body: &str = markplus_core::strip_frontmatter(&raw);
```

### Error handling

```rust
use markplus_core::CompileError;

match parse_document(&raw) {
    Ok(asset)                            => { /* use asset */ }
    Err(CompileError::InvalidFrontmatter(msg)) => eprintln!("bad YAML: {msg}"),
}
```

### Data pipeline — Tauri deploy pass

```
Raw .md file
    │
    ├─ parse_document()
    │       │
    │       ├── asset.meta     → note.json { "schema": 1, "meta": {...}, "ast": [...] }
    │       └── asset.ast      ┘
    │
    └─ strip_frontmatter()     → note.body.md  (bare markdown for AI tools)
```

---

## 3. WebAssembly API

Build the wasm package:

```bash
wasm-pack build --target web --release
# Output: pkg/
```

Import in JavaScript / TypeScript:

```js
import init, {
  parse_to_ast,
  parse_document_to_json
} from './pkg/markplus_core.js';

await init();
```

### `parse_to_ast(body: string) → string`

Parse a plain Markdown body (no frontmatter) and return the AST array as
compact JSON.

```js
// Web client data lifecycle:
// 1. Fetch the pre-built site asset once
const site = await fetch('/static_api/note.json').then(r => r.json());

// 2. Display meta immediately (title, tags, date)
document.title = site.meta.title;

// 3. Parse body → AST and render
const ast = JSON.parse(parse_to_ast(site.body));
renderHtml(ast);
```

> **Note:** `site.body` would be a `body` field you store alongside the AST,
> or you can re-parse from the `ast` directly in a renderer that accepts AST.

### `parse_document_to_json(rawMd: string) → string`

Parse a raw Markdown string (may include frontmatter) and return the full
`SiteAsset` JSON. In wasm, the `meta` field is always `null` because
`serde_yml` is not compiled into the wasm binary — use the native deploy pass
to produce the full `note.json` with parsed metadata.

```js
// Useful for in-browser live preview of raw markdown
const site = JSON.parse(parse_document_to_json(rawMarkdownText));
// site.ast is ready to pass to a renderer
```

---

## 4. Architecture overview

```
markplus_core  (this crate)
│
├── Parses Markdown → AST JSON
├── Extracts YAML frontmatter → meta JSON (native only)
└── Exports:
    ├── parse_document()          → SiteAsset { schema, meta, ast }
    ├── parse_body()              → Vec<Value>
    ├── strip_frontmatter()       → &str
    └── [wasm] parse_to_ast()
        [wasm] parse_document_to_json()

markplus_render_html  (separate crate / your code)
│
└── Consumes AST JSON → HTML string
    ├── Reads "t" field to dispatch node type
    ├── Handles "fenced" nodes by name (mermaid, simby, a2ui, ...)
    └── Falls back to <pre><code> for unknown names

markplus_render_typst  (separate crate / your code)
│
└── Consumes AST JSON → Typst markup string
```

The design rule: **`markplus_core` never knows about renderers or plugins**.
The renderer decides what `name: "mermaid"` means. Unknown extension nodes
degrade gracefully to plain code blocks in any well-written renderer.

---

## 5. Extension syntax reference

### Fenced block with attributes

````markdown
```python execute=true linenos timeout=30
print("hello")
```
````

→ AST: `{ "t": "fenced", "name": "python", "attrs": { "execute": "true", "linenos": true, "timeout": "30" }, "raw": "print(\"hello\")" }`

All fenced blocks — whether a programming language, a diagram plugin, or
anything else — produce the same `fenced` node shape. The renderer decides
what to do with it.

### Extended links

```markdown
[datasheet](./rf.pdf){download=true class=doc-link}
```

→ AST: `{ "t": "link", "href": "./rf.pdf", "title": "", "attrs": { "download": "true", "class": "doc-link" }, "children": [...] }`

Without `{...}`: works exactly as standard Markdown, no `attrs` field emitted.

### Extended images

```markdown
![Mixer chain](./mixer.png){width=480 class=diagram}
```

→ AST: `{ "t": "image", "src": "./mixer.png", "title": "", "attrs": { "width": "480", "class": "diagram" }, "children": [...] }`

### Inline widgets

```markdown
:[LO]{tooltip text="Local oscillator"}
:[beta]{badge tone=amber}
:[gain stage]{glossary id=gain-stage}
```

→ AST: `{ "t": "widget", "name": "tooltip", "text": "LO", "attrs": { "text": "Local oscillator" } }`

Widget syntax: `:[visible text]{name key=value key2="quoted value" flag}`

- First token in `{...}` = widget name
- Remaining `key=value` pairs = attrs
- Bare tokens = boolean flags (`{ "flag": true }`)
- Widget appears as an inline node inside `paragraph` / `heading` children

---

## 6. Schema versioning

Every `SiteAsset` carries a `schema` field:

```json
{ "schema": 1, "meta": { ... }, "ast": [ ... ] }
```

Renderers should reject or degrade gracefully when `schema` does not match
the version they were built against. Current version: **2**.

Breaking changes that bump the schema version:
- Renaming any node's `t` field value
- Removing a field from an existing node
- Changing the type of an existing field

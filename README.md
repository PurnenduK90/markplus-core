# markplus_core

[![Crates.io](https://img.shields.io/crates/v/markplus_core.svg)](https://crates.io/crates/markplus_core)
[![Docs.rs](https://docs.rs/markplus_core/badge.svg)](https://docs.rs/markplus_core)
[![License](https://img.shields.io/crates/l/markplus_core.svg)](https://crates.io/crates/markplus_core)
[![CI](https://github.com/PurnenduK90/markplus-core/actions/workflows/ci.yml/badge.svg)](https://github.com/PurnenduK90/markplus-core/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/PurnenduK90/markplus-core/graph/badge.svg)](https://codecov.io/gh/PurnenduK90/markplus-core)

A universal, high-performance Markdown → AST compiler written in Rust by **[Cadiora](https://cadiora.com)**.

`markplus_core` parses Markdown (with optional YAML frontmatter) into a
**structured, versioned JSON AST**. It does not render HTML or Typst —
rendering is the responsibility of a downstream crate or plugin that consumes
the AST. This keeps the parser lean and completely independent of any output
target.

---

## Key capabilities

| Feature | Detail |
|---|---|
| **Platform sovereign** | Compiles to native (Tauri, CLI) and WebAssembly (browser/PWA) from one codebase |
| **Versioned AST** | JSON output carries a `schema` version so renderers can detect incompatible shapes |
| **Frontmatter** | YAML frontmatter extracted and serialised to JSON; stripped body kept separately |
| **All pulldown-cmark extensions** | Tables, footnotes, strikethrough, task lists, math, GFM alerts, definition lists, superscript/subscript |
| **Data File Parsers** | Parse `.csv` and `.json` directly into Markdown Table/Definition List AST nodes |
| **Media File Parsers** | Parse Source Code and Mermaid (`.mmd`) files into Fenced Code Block AST nodes |
| **Fenced block attrs** | `` ```python execute=true linenos `` → `{ "name": "python", "attrs": {...} }` |
| **Extended links/images** | `[text](url){key=value}` and `![alt](src){width=480}` |
| **Inline widgets** | `:[text]{tooltip text="Local oscillator"}` for custom inline extensions |
| **Formal JSON Validation** | Built-in strict JSON validation for `SiteAsset` structure |

---

## Project structure

```text
markplus_core/
├── Cargo.toml
├── docs/
│   ├── usage.md          ← API / CLI usage guide
│   └── ast-reference.md  ← Markdown → AST node reference
└── src/
    ├── lib.rs            ← Public API (native + Wasm exports)
    ├── main.rs           ← mpc CLI (single-command AST emitter)
    ├── config.rs         ← Parser options / FrontmatterMode
    ├── event_filter.rs   ← Frontmatter stripper, passes events through
    ├── ast.rs            ← Stack-based AST builder
    ├── json.rs           ← SiteAsset wire format & validation
    ├── csv.rs            ← CSV file parser
    ├── json_data.rs      ← JSON data file parser
    ├── code.rs           ← Source code file parser
    └── mermaid.rs        ← Mermaid file parser
```

---

## Quick start

### Build

```bash
cargo build --release        # native lib + mpc CLI
wasm-pack build --target web # WebAssembly package → pkg/
```

### CLI

```bash
# Emit compact AST JSON
mpc note.md

# Emit pretty-printed AST JSON
mpc --pretty note.md
```

### Rust library

```rust
use markplus_core::{parse_document, parse_body};

// Full document with frontmatter
let asset = parse_document(raw_md)?;
let json  = asset.to_json()?;      // write to note.json

// Pre-stripped body only
let ast = parse_body(body_str);
```

### WebAssembly (JavaScript)

```js
import init, { parse_to_ast, parse_document_to_json } from './pkg/markplus_core.js';
await init();

// Plain markdown body → AST array JSON string
const ast = JSON.parse(parse_to_ast(markdownBody));

// Raw markdown (frontmatter ignored in wasm) → full SiteAsset JSON string
const site = JSON.parse(parse_document_to_json(rawMarkdown));
```

---

## Wire format — `note.json`

```json
{
  "schema": 1,
  "meta": {
    "title": "RFSoC Mixer Design Notes",
    "tags": ["rfsoc", "dsp"],
    "date": "2026-05-30"
  },
  "ast": [
    { "t": "heading", "level": 1, "children": [{ "t": "text", "text": "High Frequency Core" }] },
    { "t": "fenced",  "name": "simby", "attrs": {}, "raw": "[RFSoC] ──► [Filter]" }
  ]
}
```

See [`docs/ast-reference.md`](docs/ast-reference.md) for the full AST node schema.

---

## Schema

The file [`schema/markplus-ast.v1.2.schema.json`](schema/markplus-ast.v1.2.schema.json)
is a **JSON Schema 2020-12** document that formally defines every node type in the AST.

It is the **source of truth** for both `markplus_core` (producer) and all downstream
renderers and plugins (consumers). When a new node type is added, the schema is updated
first — then the core, then the renderers.

### Validate your AST

```bash
# Python (jsonschema)
pip install jsonschema
mpc note.md | python -c "
import sys, json
from jsonschema import validate
ast = json.loads(sys.stdin.read())
schema = json.load(open('schema/markplus-ast.v1.2.schema.json'))
validate(instance=ast, schema=schema)
print('valid')
"
```

See [`schema/CHANGELOG.md`](schema/CHANGELOG.md) for the schema versioning policy.

---

## Further reading

- [`docs/usage.md`](docs/usage.md) — detailed API and CLI usage
- [`docs/ast-reference.md`](docs/ast-reference.md) — every Markdown construct mapped to its AST node
- [`schema/markplus-ast.v1.2.schema.json`](schema/markplus-ast.v1.2.schema.json) — formal JSON Schema (machine-readable)
- [`schema/CHANGELOG.md`](schema/CHANGELOG.md) — schema versioning history and breaking-change policy

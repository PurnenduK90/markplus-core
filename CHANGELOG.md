# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-07-16

### Added
- **AST Schema v1.2 Support**: Full support for the new v1.2 AST JSON schema.
- **Pure-Rust YAML Parser (`yaml.rs`)**: Replaced `serde_yml` with a custom, high-performance, Wasm-compatible YAML parser.
- **Wasm Frontmatter Support**: Frontmatter is no longer ignored in WebAssembly! The new YAML parser allows full metadata extraction on Wasm targets.
- **Directives Syntax**: Added support for generic fenced directives (`:::name` and `:::/name`) that compile natively into AST components.
- **New Wasm API**: Introduced `get_ast`, `get_frontmatter`, and `get_document_json` for streamlined Wasm usage.

### Changed
- Generalized frontmatter parsing logic into a generic `.yaml` data file parser.

### Removed
- **Standalone `mpc` CLI**: The internal `mpc` binary was removed from `markplus-core`. The CLI is now officially provided and maintained by the `markplus` facade crate.
- Removed deprecated Wasm aliases `parse_to_ast` and `parse_document_to_json`.
- Removed `serde_yml` dependency.

## [1.1.0]

This major update transforms markplus_core from a strict Markdown compiler into a multi-format data ingestion engine. You can now seamlessly inject external data structures and media directly into your Markdown AST for downstream rendering!

### New Features & Parsers
- **Data File Parsers**: Added native support for parsing flat `.csv` files into fully structured Markdown Table nodes, and parsing `.json` data files into Table/Definition List nodes.
- **Media File Parsers**: Added native support for parsing `.mmd` (Mermaid) diagram definitions and Source Code (`.py`, `.rs`, `.js`, etc.) into cleanly formed Fenced Code Block AST nodes.
- **Format-Agnostic Processing**: The compiler now intelligently detects extensions and routes files to their respective custom parsing engines (`csv.rs`, `json_data.rs`, `code.rs`, `mermaid.rs`), completely bypassing the standard markdown tokenizer for non-markdown inputs.

### Validation & Stability
- **Strict JSON Validation**: The `SiteAsset` structure now undergoes rigorous, built-in structural validation via `validate_asset_json_value()` to guarantee that produced `.json` bundles strictly adhere to the `markplus-ast.schema.json` specification.
- **Schema Upgrade**: Lifted internal validation logic to target the `v1.1` JSON Schema syntax, splitting the monolithic schema ID into independent major and minor fields for better downstream API contract guarantees.

### Maintenance & DX
- Resolved deep structural nested-if complexities flagged by `cargo clippy`.
- Refactored unused variable assignments in the JSON parsing engines.
- Restructured GitHub Actions CI to properly route `v*-stable` tags to the secure OIDC crates.io publishing pipeline while simultaneously attaching compiled binaries as release drafts.

## [1.0.0]

**markplus-core v1.0.0 (Stable Release)**
We are incredibly proud to announce the 1.0.0 Stable Release of `markplus-core`!

This release marks the transition of the project into a production-ready, highly-tested, and fully cross-compiled Universal Markdown → AST compiler for the MarkPlus ecosystem.

### Key Capabilities
- **Universal Architecture**: Compiles effortlessly to Native binaries (CLI/TUI/Tauri) and WebAssembly (Browser/PWA) from a single Rust codebase.
- **Formal JSON Schema**: Emits strict, versioned JSON AST outputs. We have achieved ~90% test coverage validating complex edge cases (HTML Blocks, Task Lists, Definition Lists, and Horizontal Rules) directly against our formal `markplus-ast.v1.schema.json` contract.
- **Rich Markdown Support**: Full support for YAML frontmatter parsing, GFM extensions, display/inline math, deeply nested lists, fenced code blocks with custom attributes, and inline widgets.

### What's New in v1.0.0?
- **Full ARM64 Cross-Compilation**: `markplus-core` now officially provides highly-optimized, native binaries for Apple Silicon (M1/M2/M3), Windows on ARM (Snapdragon), and Linux ARM (AWS Graviton)!
- **Automated CI/CD**: Fully automated release pipeline via GitHub Actions. OIDC-secured automated publishing straight to Crates.io on tag pushes.
- **Codecov Integration**: Integrated Codecov coverage reporting into the CI pipeline to maintain rigorous validation standards moving forward.

### Downloads
Pre-compiled binaries are attached below for all major platforms and architectures:

To use the Native binaries, simply download the file matching your OS and architecture, rename it to `mpc`, ensure it is executable, and run `mpc --help`.

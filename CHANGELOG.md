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

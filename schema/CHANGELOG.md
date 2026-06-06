# MarkPlus AST Schema Changelog

This file tracks breaking and additive changes to the AST schema.

The `schema` field in every `SiteAsset` JSON output tells renderers and
validators which version of this schema to use.

---

## v1 — 2026-06-06

**Current version.** First published schema, released alongside `markplus_core 0.2`.

Includes:

- All standard CommonMark block nodes: `heading`, `paragraph`, `fenced`,
  `blockquote`, `list`, `list_item`, `table`, `definition_list`, `hr`,
  `html_block`, `footnote_def`
- All standard inline nodes: `text`, `em`, `strong`, `del`, `sup`, `sub`,
  `code_span`, `link`, `image`, `footnote_ref`, `task_marker`,
  `soft_break`, `hard_break`, `raw_html`
- Extension nodes: `math_block`, `math_inline` (pulldown-cmark ENABLE_MATH)
- MarkPlus-specific nodes: `widget` (`:[text]{name attrs}` syntax)
- Fenced block attr parsing: `name` + `attrs` object on all `fenced` nodes
- Extended link/image attrs: optional `attrs` on `link` and `image` nodes
- GFM alert blockquotes: optional `kind` on `blockquote` nodes
- Tight list support: `list_item.children` allows both block and inline nodes

---

## What counts as a breaking change (bumps `schema` version)

- Renaming a `t` field value
- Removing a required field from an existing node type
- Changing the type of an existing field
- Removing a node type entirely

## What does NOT bump the version

- Adding a new optional field to an existing node type
- Adding a new node type to `BlockNode` or `InlineNode`
- Adding new enum values to `blockquote.kind`
- Changes to `meta` (frontmatter is unstructured by design)
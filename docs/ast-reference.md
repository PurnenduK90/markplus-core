# MarkPlus AST Reference

Every Markdown construct maps to a JSON node with a `"t"` (type) field.
Block nodes appear at the top level of the `ast` array. Inline nodes appear
inside a parent node's `"children"` array.

All nodes are produced by `markplus_core::parse_document()` or
`markplus_core::parse_body()`.

---

## Quick index

| Node `t` | Kind | Source syntax |
|---|---|---|
| `heading` | block | `# … ######` |
| `paragraph` | block | plain text block |
| `fenced` | block | `` ``` … ``` `` |
| `blockquote` | block | `> …` |
| `list` | block | `- …` / `1. …` |
| `list_item` | block (child of list) | each item |
| `table` | block | GFM table |
| `definition_list` | block | GFM definition list |
| `hr` | block | `---` / `***` |
| `html_block` | block | raw HTML block |
| `footnote_def` | block | `[^label]: …` |
| `math_block` | block | `$$ … $$` |
| `raw_html` | inline | inline HTML tag |
| `text` | inline | plain text |
| `em` | inline | `*…*` / `_…_` |
| `strong` | inline | `**…**` / `__…__` |
| `del` | inline | `~~…~~` |
| `sup` | inline | `^…^` |
| `sub` | inline | `~…~` |
| `code_span` | inline | `` `…` `` |
| `math_inline` | inline | `$…$` |
| `link` | inline | `[text](url)` |
| `image` | inline | `![alt](src)` |
| `widget` | inline | `:[text]{name …}` |
| `footnote_ref` | inline | `[^label]` |
| `task_marker` | inline | `[ ]` / `[x]` |
| `soft_break` | inline | line break (no `\`) |
| `hard_break` | inline | `  \n` or `\` + newline |

---

## Block nodes

### `heading`

```markdown
# Title
## Section { #my-id .highlight }
```

```json
{
  "t": "heading",
  "level": 1,
  "children": [{ "t": "text", "text": "Title" }]
}
```

With heading attributes (`# Title { #id .class key=val }`):

```json
{
  "t": "heading",
  "level": 2,
  "id": "my-id",
  "classes": ["highlight"],
  "attrs": { "key": "val" },
  "children": [{ "t": "text", "text": "Section" }]
}
```

Fields: `level` (1–6), optional `id`, optional `classes` (array), optional `attrs` (object), `children` (inline array).

---

### `paragraph`

```markdown
This is a paragraph with **bold** and a [link](url).
```

```json
{
  "t": "paragraph",
  "children": [
    { "t": "text",   "text": "This is a paragraph with " },
    { "t": "strong", "children": [{ "t": "text", "text": "bold" }] },
    { "t": "text",   "text": " and a " },
    { "t": "link",   "href": "url", "title": "", "children": [{ "t": "text", "text": "link" }] }
  ]
}
```

Fields: `children` (inline array).

---

### `fenced`

All fenced code blocks — languages, plugins, diagrams — produce the **same**
node shape. The renderer decides what `name` means.

````markdown
```python execute=true linenos
print("hi")
```
````

```json
{
  "t": "fenced",
  "name": "python",
  "attrs": { "execute": "true", "linenos": true },
  "raw": "print(\"hi\")"
}
```

````markdown
```mermaid theme=dark
graph TD
A --> B
```
````

```json
{
  "t": "fenced",
  "name": "mermaid",
  "attrs": { "theme": "dark" },
  "raw": "graph TD\nA --> B"
}
```

Indented code blocks:

```json
{ "t": "fenced", "name": "", "attrs": {}, "raw": "indented content" }
```

Fields: `name` (string, empty for indented), `attrs` (object, may be empty), `raw` (string, trailing whitespace trimmed).

**Attr syntax:** first whitespace-separated token after the language name is the key=value list.
- `key=value` → `"key": "value"`
- bare `flag` → `"flag": true`
- `key="quoted value"` → `"key": "quoted value"`

---

### `blockquote`

```markdown
> Regular quote

> [!NOTE]
> GFM alert note
```

```json
{ "t": "blockquote", "children": [...] }
```

With GFM alert kind:

```json
{ "t": "blockquote", "kind": "note", "children": [...] }
```

`kind` values: `"note"`, `"tip"`, `"important"`, `"warning"`, `"caution"`.

Fields: optional `kind`, `children` (block array from the quote body).

---

### `list`

```markdown
- alpha
- beta

1. first
2. second
```

```json
{
  "t": "list",
  "ordered": false,
  "items": [
    { "t": "list_item", "children": [{ "t": "paragraph", "children": [...] }] },
    { "t": "list_item", "children": [...] }
  ]
}
```

Ordered list:

```json
{ "t": "list", "ordered": true, "start": 1, "items": [...] }
```

Fields: `ordered` (bool), optional `start` (number, only when ordered), `items` (array of `list_item`).

---

### `list_item`

Child of `list`. Contains block-level children (paragraphs, nested lists, etc.).

```json
{ "t": "list_item", "children": [ ... ] }
```

Task list items include a `task_marker` as the first inline child of their paragraph:

```markdown
- [ ] unchecked
- [x] checked
```

```json
{
  "t": "list_item",
  "children": [{
    "t": "paragraph",
    "children": [
      { "t": "task_marker", "checked": false },
      { "t": "text", "text": " unchecked" }
    ]
  }]
}
```

---

### `table`

```markdown
| Name   | Value | Unit |
|--------|-------|------|
| Gain   | 12    | dB   |
```

```json
{
  "t": "table",
  "align": ["none", "none", "none"],
  "headers": [
    { "t": "_cell", "children": [{ "t": "text", "text": "Name" }] },
    { "t": "_cell", "children": [{ "t": "text", "text": "Value" }] },
    { "t": "_cell", "children": [{ "t": "text", "text": "Unit" }] }
  ],
  "rows": [
    [
      { "t": "_cell", "children": [{ "t": "text", "text": "Gain" }] },
      { "t": "_cell", "children": [{ "t": "text", "text": "12" }] },
      { "t": "_cell", "children": [{ "t": "text", "text": "dB" }] }
    ]
  ]
}
```

`align` values per column: `"left"`, `"right"`, `"center"`, `"none"`.

Fields: `align` (array), `headers` (array of `_cell`), `rows` (array of arrays of `_cell`).

---

### `definition_list`

```markdown
Term
: Definition one
: Definition two
```

```json
{
  "t": "definition_list",
  "items": [
    { "t": "_def_title", "children": [{ "t": "text", "text": "Term" }] },
    { "t": "_def_body",  "children": [{ "t": "text", "text": "Definition one" }] },
    { "t": "_def_body",  "children": [{ "t": "text", "text": "Definition two" }] }
  ]
}
```

---

### `hr`

```markdown
---
```

```json
{ "t": "hr" }
```

---

### `math_block`

```markdown
$$
\int_0^\infty e^{-x} dx = 1
$$
```

```json
{ "t": "math_block", "src": "\\int_0^\\infty e^{-x} dx = 1\n" }
```

Fields: `src` (raw LaTeX string, including whitespace as emitted by the parser).

---

### `html_block`

```markdown
<div class="callout">
Custom HTML block.
</div>
```

```json
{ "t": "html_block", "html": "<div class=\"callout\">\nCustom HTML block.\n</div>\n" }
```

Fields: `html` (raw HTML string).

---

### `footnote_def`

```markdown
[^1]: Footnote body text here.
```

```json
{
  "t": "footnote_def",
  "label": "1",
  "children": [{ "t": "paragraph", "children": [...] }]
}
```

---

## Inline nodes

Inline nodes appear inside a parent's `children` array.

---

### `text`

Plain text content.

```json
{ "t": "text", "text": "Hello, world!" }
```

---

### `em`

```markdown
*italic* or _italic_
```

```json
{ "t": "em", "children": [{ "t": "text", "text": "italic" }] }
```

---

### `strong`

```markdown
**bold** or __bold__
```

```json
{ "t": "strong", "children": [{ "t": "text", "text": "bold" }] }
```

---

### `del`

```markdown
~~strikethrough~~
```

```json
{ "t": "del", "children": [{ "t": "text", "text": "strikethrough" }] }
```

---

### `sup`

```markdown
^superscript^
```

```json
{ "t": "sup", "children": [{ "t": "text", "text": "superscript" }] }
```

---

### `sub`

```markdown
~subscript~
```

> Note: `~` is subscript when `ENABLE_SUBSCRIPT` is on (enabled by default).
> Use `~~double~~ ` for strikethrough instead.

```json
{ "t": "sub", "children": [{ "t": "text", "text": "subscript" }] }
```

---

### `code_span`

```markdown
`inline code`
```

```json
{ "t": "code_span", "text": "inline code" }
```

---

### `math_inline`

```markdown
$E = mc^2$
```

```json
{ "t": "math_inline", "src": "E = mc^2" }
```

---

### `link`

```markdown
[text](https://example.com "title")
```

```json
{
  "t": "link",
  "href": "https://example.com",
  "title": "title",
  "children": [{ "t": "text", "text": "text" }]
}
```

With extended attributes:

```markdown
[datasheet](./rf.pdf){download=true class=doc-link}
```

```json
{
  "t": "link",
  "href": "./rf.pdf",
  "title": "",
  "attrs": { "download": "true", "class": "doc-link" },
  "children": [{ "t": "text", "text": "datasheet" }]
}
```

Fields: `href`, `title`, optional `attrs`, `children` (inline, the link label text).

---

### `image`

```markdown
![alt text](./image.png "caption")
```

```json
{
  "t": "image",
  "src": "./image.png",
  "title": "caption",
  "children": [{ "t": "text", "text": "alt text" }]
}
```

With extended attributes:

```markdown
![Mixer chain](./mixer.png){width=480 class=diagram}
```

```json
{
  "t": "image",
  "src": "./mixer.png",
  "title": "",
  "attrs": { "width": "480", "class": "diagram" },
  "children": [{ "t": "text", "text": "Mixer chain" }]
}
```

Fields: `src`, `title`, optional `attrs`, `children` (inline, the alt text nodes).

---

### `widget`

MarkPlus inline extension for custom components.

```markdown
:[LO]{tooltip text="Local oscillator"}
:[beta]{badge tone=amber}
:[gain stage]{glossary id=gain-stage}
```

```json
{ "t": "widget", "name": "tooltip", "text": "LO", "attrs": { "text": "Local oscillator" } }
{ "t": "widget", "name": "badge",   "text": "beta", "attrs": { "tone": "amber" } }
{ "t": "widget", "name": "glossary","text": "gain stage", "attrs": { "id": "gain-stage" } }
```

Syntax: `:[visible text]{name key=value key2="quoted value" flag}`

- First token in `{...}` = `name`
- `key=value` → string attr
- `key="quoted value"` → string attr (quotes stripped)
- bare `flag` → `true` bool attr

Fields: `name`, `text`, `attrs`.

---

### `footnote_ref`

```markdown
See the appendix[^1].
```

```json
{ "t": "footnote_ref", "label": "1" }
```

---

### `task_marker`

Emitted as the first inline child of a task-list item paragraph.

```markdown
- [x] done
- [ ] todo
```

```json
{ "t": "task_marker", "checked": true }
{ "t": "task_marker", "checked": false }
```

---

### `soft_break`

A line break that is not a hard break (no trailing spaces or `\`).

```json
{ "t": "soft_break" }
```

---

### `hard_break`

Two trailing spaces or a backslash before a newline.

```markdown
line one\
line two
```

```json
{ "t": "hard_break" }
```

---

### `raw_html`

An inline HTML tag not interpretable as a block.

```markdown
Text with <em>inline HTML</em> tag.
```

```json
{ "t": "raw_html", "html": "<em>" }
```

---

## SiteAsset wire format

The `SiteAsset` produced by `parse_document()` and serialised to `note.json`:

```json
{
  "schema": 1,
  "meta": {
    "title": "RFSoC Mixer Design Notes",
    "category": "hardware",
    "tags": ["rfsoc", "dsp"],
    "date": "2026-05-30",
    "draft": false
  },
  "ast": [ ... block nodes ... ]
}
```

- `schema` — integer, bumped on breaking AST shape changes. Current: **1**.
- `meta` — `null` if no frontmatter; otherwise the YAML document deserialised into a JSON value tree.
- `ast` — array of block nodes as described in this document.

Renderers should guard on `schema`:

```js
if (site.schema !== 1) throw new Error(`Unsupported AST schema ${site.schema}`);
```

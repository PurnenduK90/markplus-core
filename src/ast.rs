//    Copyright [2026] [Purnendu Kumar]

//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at

//        http://www.apache.org/licenses/LICENSE-2.0

//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

//! Stack-based Markdown AST builder.
//!
//! [`build_ast`] converts a flat pulldown-cmark event stream into a nested
//! JSON block tree. Every node carries a `"t"` field (type discriminant).
//! Block nodes appear at the top level; inline nodes nest inside `"children"`
//! arrays of their parent block.
//!
//! # Node shapes
//!
//! See [`docs/ast-reference.md`](../docs/ast-reference.md) for the full
//! mapping from Markdown syntax to AST node shapes.

use pulldown_cmark::{Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

use crate::event_filter::{DirectiveTable, RichEvent};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Convert a pulldown-cmark event stream into the MarkPlus AST (JSON array).
///
/// The AST is a flat-top `Vec<Value>` where each element is a block node.
/// Block nodes may contain `children` arrays for inline content.
/// Fenced code blocks (any info string) become `"fenced"` nodes — the
/// renderer is responsible for deciding whether `name` means syntax-
/// highlight, diagram, or plugin.
pub fn build_ast(rich_events: Vec<RichEvent<'_>>, directives: DirectiveTable) -> Vec<Value> {
    AstBuilder::new(rich_events, directives).build()
}

// ---------------------------------------------------------------------------
// Builder internals
// ---------------------------------------------------------------------------

/// One frame on the builder stack — each open block tag gets a frame.
struct Frame {
    /// The AST node being assembled.
    node: Map<String, Value>,
    /// Inline children being accumulated inside this frame.
    inline: Vec<Value>,
    /// Raw text buffer used by code blocks / fenced blocks.
    text: String,
}

impl Frame {
    /// Create a new frame for a node with the given type tag.
    fn new(t: &str, range: std::ops::Range<usize>) -> Self {
        let mut node = Map::new();
        node.insert("t".into(), Value::String(t.to_owned()));
        node.insert("range".into(), json!([range.start, range.end]));
        Self {
            node,
            inline: Vec::new(),
            text: String::new(),
        }
    }

    /// Insert an additional field into the node being assembled.
    fn with(mut self, key: &str, val: Value) -> Self {
        self.node.insert(key.into(), val);
        self
    }
}

/// Stack-based builder that converts a pulldown-cmark event sequence into
/// the MarkPlus nested JSON AST.
///
/// Each open tag pushes a [`Frame`] onto the stack; each close tag pops the
/// frame, finalises the node, and either pushes it onto the parent frame's
/// inline list or directly onto the top-level block list.
struct AstBuilder<'a> {
    events: std::vec::IntoIter<RichEvent<'a>>,
    directives: DirectiveTable,
    /// Block-level output collected so far.
    blocks: Vec<Value>,
    /// Stack of open block frames. The last entry is the innermost open tag.
    stack: Vec<Frame>,
    /// State flag: inside a table head (for header vs body cell distinction).
    in_table_head: bool,
    /// Accumulated table headers (filled during TableHead span).
    table_headers: Vec<Value>,
    /// Accumulated rows being built.
    table_rows: Vec<Value>,
    /// Current row cells being built.
    current_row: Vec<Value>,
}

impl<'a> AstBuilder<'a> {
    fn new(events: Vec<RichEvent<'a>>, directives: DirectiveTable) -> Self {
        Self {
            events: events.into_iter(),
            directives,
            blocks: Vec::new(),
            stack: Vec::new(),
            in_table_head: false,
            table_headers: Vec::new(),
            table_rows: Vec::new(),
            current_row: Vec::new(),
        }
    }

    fn build(mut self) -> Vec<Value> {
        while let Some(event) = self.events.next() {
            self.handle(event);
        }
        merge_directives(self.blocks, self.directives)
    }

    fn handle(&mut self, rich_event: RichEvent<'_>) {
        match rich_event {
            RichEvent::Widget {
                name,
                text,
                attrs,
                range,
            } => {
                let mut node_attrs = Map::new();
                for (k, v) in attrs {
                    node_attrs.insert(k, v);
                }
                self.push_inline(json!({
                    "t": "widget",
                    "name": name,
                    "text": text,
                    "attrs": node_attrs,
                    "range": [range.start, range.end]
                }));
            }
            RichEvent::LinkImageAttrs(attrs) => {
                if let Some(f) = self.stack.last_mut() {
                    let mut node_attrs = match f.node.get("attrs") {
                        Some(Value::Object(existing)) => existing.clone(),
                        _ => Map::new(),
                    };
                    for (k, v) in attrs {
                        node_attrs.insert(k, v);
                    }
                    f.node.insert("attrs".into(), Value::Object(node_attrs));
                }
            }
            RichEvent::Cmark(event, range) => match event {
                // ── Block open tags ───────────────────────────────────────────
                Event::Start(tag) => self.open(tag, range),

                // ── Block close tags ──────────────────────────────────────────
                Event::End(end) => self.close(end),

                // ── Leaf events (no paired End) ───────────────────────────────
                Event::Rule => {
                    self.push_block(json!({"t": "hr", "range": [range.start, range.end]}))
                }

                Event::TaskListMarker(checked) => {
                    self.push_inline(json!({"t": "task_marker", "checked": checked, "range": [range.start, range.end]}));
                }

                Event::Text(t) => {
                    self.push_inline(
                        json!({"t": "text", "text": t.as_ref(), "range": [range.start, range.end]}),
                    );
                }

                Event::Code(t) => {
                    self.push_inline(json!({"t": "code_span", "text": t.as_ref(), "range": [range.start, range.end]}));
                }

                Event::InlineMath(t) => {
                    self.push_inline(json!({"t": "math_inline", "src": t.as_ref(), "range": [range.start, range.end]}));
                }

                Event::DisplayMath(t) => {
                    // Display math appears at block level (between paragraphs)
                    self.push_block(json!({"t": "math_block", "src": t.as_ref(), "range": [range.start, range.end]}));
                }

                Event::Html(t) | Event::InlineHtml(t) => {
                    let s = t.as_ref();
                    if self.stack.is_empty() {
                        self.push_block(
                            json!({"t": "raw_html", "html": s, "range": [range.start, range.end]}),
                        );
                    } else {
                        self.push_inline(
                            json!({"t": "raw_html", "html": s, "range": [range.start, range.end]}),
                        );
                    }
                }

                Event::FootnoteReference(label) => {
                    self.push_inline(json!({"t": "footnote_ref", "label": label.as_ref(), "range": [range.start, range.end]}));
                }

                Event::SoftBreak => {
                    self.push_inline(json!({"t": "soft_break", "range": [range.start, range.end]}));
                }

                Event::HardBreak => {
                    self.push_inline(json!({"t": "hard_break", "range": [range.start, range.end]}));
                }
            },
        }
    }

    // ── Stack operations ──────────────────────────────────────────────────

    fn open(&mut self, tag: Tag<'_>, range: std::ops::Range<usize>) {
        match tag {
            Tag::Paragraph => self.stack.push(Frame::new("paragraph", range.clone())),

            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => {
                let mut f = Frame::new("heading", range.clone());
                f.node.insert("level".into(), json!(heading_level(level)));
                if let Some(id) = id {
                    f.node.insert("id".into(), json!(id.as_ref()));
                }
                if !classes.is_empty() {
                    let cls: Vec<Value> = classes.iter().map(|c| json!(c.as_ref())).collect();
                    f.node.insert("classes".into(), Value::Array(cls));
                }
                if !attrs.is_empty() {
                    let mut m = Map::new();
                    for (k, v) in &attrs {
                        m.insert(k.as_ref().to_owned(), json!(v.as_deref()));
                    }
                    f.node.insert("attrs".into(), Value::Object(m));
                }
                self.stack.push(f);
            }

            Tag::BlockQuote(kind) => {
                let mut f = Frame::new("blockquote", range.clone());
                if let Some(k) = kind {
                    f.node.insert("kind".into(), json!(blockquote_kind(k)));
                }
                self.stack.push(f);
            }

            Tag::CodeBlock(CodeBlockKind::Fenced(info)) => {
                let (name, attrs) = parse_fence_info(info.as_ref());
                let mut f = Frame::new("fenced", range.clone());
                f.node.insert("name".into(), json!(name));
                if !attrs.is_empty() {
                    f.node.insert(
                        "attrs".into(),
                        Value::Object(attrs.into_iter().map(|(k, v)| (k, json!(v))).collect()),
                    );
                }
                self.stack.push(f);
            }

            Tag::CodeBlock(CodeBlockKind::Indented) => {
                let mut f = Frame::new("fenced", range.clone());
                f.node.insert("name".into(), json!(""));
                self.stack.push(f);
            }

            Tag::HtmlBlock => self.stack.push(Frame::new("html_block", range.clone())),

            Tag::List(start) => {
                let mut f = Frame::new("list", range.clone());
                if let Some(n) = start {
                    f.node.insert("ordered".into(), json!(true));
                    f.node.insert("start".into(), json!(n));
                } else {
                    f.node.insert("ordered".into(), json!(false));
                }
                self.stack.push(f);
            }

            Tag::Item => self.stack.push(Frame::new("list_item", range.clone())),

            Tag::FootnoteDefinition(label) => {
                let f =
                    Frame::new("footnote_def", range.clone()).with("label", json!(label.as_ref()));
                self.stack.push(f);
            }

            Tag::Table(alignments) => {
                let aligns: Vec<Value> = alignments.iter().map(|a| json!(col_align(*a))).collect();
                let mut f = Frame::new("table", range.clone());
                f.node.insert("align".into(), Value::Array(aligns));
                self.table_headers.clear();
                self.table_rows.clear();
                self.stack.push(f);
            }
            Tag::TableHead => {
                self.in_table_head = true;
            }
            Tag::TableRow => {
                self.current_row.clear();
            }
            Tag::TableCell => self.stack.push(Frame::new("_cell", range.clone())),

            Tag::DefinitionList => self
                .stack
                .push(Frame::new("definition_list", range.clone())),
            Tag::DefinitionListTitle => self.stack.push(Frame::new("_def_title", range.clone())),
            Tag::DefinitionListDefinition => {
                self.stack.push(Frame::new("_def_body", range.clone()))
            }

            Tag::Emphasis => self.stack.push(Frame::new("em", range.clone())),
            Tag::Strong => self.stack.push(Frame::new("strong", range.clone())),
            Tag::Strikethrough => self.stack.push(Frame::new("del", range.clone())),
            Tag::Superscript => self.stack.push(Frame::new("sup", range.clone())),
            Tag::Subscript => self.stack.push(Frame::new("sub", range.clone())),

            Tag::Link {
                dest_url, title, ..
            } => {
                let f = Frame::new("link", range.clone())
                    .with("href", json!(dest_url.as_ref()))
                    .with("title", json!(title.as_ref()));
                self.stack.push(f);
            }

            Tag::Image {
                dest_url, title, ..
            } => {
                let f = Frame::new("image", range.clone())
                    .with("src", json!(dest_url.as_ref()))
                    .with("title", json!(title.as_ref()));
                self.stack.push(f);
            }

            Tag::MetadataBlock(_) => {} // handled in event_filter before reaching here
        }
    }

    fn close(&mut self, end: TagEnd) {
        match end {
            // ── Inline span closes: pop frame, attach as inline child of parent ──
            TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript => {
                if let Some(mut f) = self.stack.pop() {
                    f.node.insert(
                        "children".into(),
                        Value::Array(f.inline.drain(..).collect()),
                    );
                    let node = Value::Object(f.node);
                    self.push_inline(node);
                }
            }

            TagEnd::Link | TagEnd::Image => {
                if let Some(mut f) = self.stack.pop() {
                    // For image, inline children are the alt text
                    f.node.insert(
                        "children".into(),
                        Value::Array(f.inline.drain(..).collect()),
                    );
                    let node = Value::Object(f.node);
                    self.push_inline(node);
                }
            }

            // ── Block span closes: pop frame, flush to parent or top ──────
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::Item
            | TagEnd::FootnoteDefinition => {
                if let Some(mut f) = self.stack.pop() {
                    let children: Vec<Value> = f.inline.drain(..).collect();
                    f.node.insert("children".into(), Value::Array(children));
                    let node = Value::Object(f.node);
                    self.flush_block(node);
                }
            }

            TagEnd::CodeBlock => {
                if let Some(mut f) = self.stack.pop() {
                    f.node.insert("raw".into(), json!(f.text.trim_end()));
                    let node = Value::Object(f.node);
                    self.flush_block(node);
                }
            }

            TagEnd::HtmlBlock => {
                if let Some(mut f) = self.stack.pop() {
                    f.node.insert("html".into(), json!(f.text));
                    let node = Value::Object(f.node);
                    self.flush_block(node);
                }
            }

            TagEnd::List(_) => {
                if let Some(mut f) = self.stack.pop() {
                    // children were accumulated as block children
                    f.node
                        .insert("items".into(), Value::Array(f.inline.drain(..).collect()));
                    let node = Value::Object(f.node);
                    self.flush_block(node);
                }
            }

            // ── Table handling ────────────────────────────────────────────
            TagEnd::TableCell => {
                if let Some(mut f) = self.stack.pop() {
                    f.node.insert(
                        "children".into(),
                        Value::Array(f.inline.drain(..).collect()),
                    );
                    let cell = Value::Object(f.node);
                    if self.in_table_head {
                        self.table_headers.push(cell);
                    } else {
                        self.current_row.push(cell);
                    }
                }
            }
            TagEnd::TableHead => {
                self.in_table_head = false;
            }
            TagEnd::TableRow => {
                if !self.current_row.is_empty() {
                    let row = Value::Array(self.current_row.drain(..).collect());
                    self.table_rows.push(row);
                }
            }
            TagEnd::Table => {
                if let Some(mut f) = self.stack.pop() {
                    f.node.insert(
                        "headers".into(),
                        Value::Array(self.table_headers.drain(..).collect()),
                    );
                    f.node.insert(
                        "rows".into(),
                        Value::Array(self.table_rows.drain(..).collect()),
                    );
                    let node = Value::Object(f.node);
                    self.flush_block(node);
                }
            }

            // ── Definition list ───────────────────────────────────────────
            TagEnd::DefinitionListTitle | TagEnd::DefinitionListDefinition => {
                if let Some(mut f) = self.stack.pop() {
                    f.node.insert(
                        "children".into(),
                        Value::Array(f.inline.drain(..).collect()),
                    );
                    let node = Value::Object(f.node);
                    // Append as child of enclosing definition_list
                    self.push_inline(node);
                }
            }
            TagEnd::DefinitionList => {
                if let Some(mut f) = self.stack.pop() {
                    f.node
                        .insert("items".into(), Value::Array(f.inline.drain(..).collect()));
                    let node = Value::Object(f.node);
                    self.flush_block(node);
                }
            }

            TagEnd::MetadataBlock(_) => {} // already stripped in event_filter
        }
    }

    // ── Routing helpers ───────────────────────────────────────────────────

    /// Push an inline node into the innermost open frame, or to blocks if no frame.
    fn push_inline(&mut self, node: Value) {
        // Fenced/html_block frames accumulate raw text, not inline children.
        if let Some(f) = self.stack.last_mut() {
            match f.node.get("t").and_then(|v| v.as_str()) {
                Some("fenced") | Some("html_block") => {
                    // raw text accumulation
                    if let Value::Object(ref obj) = node
                        && let Some(Value::String(s)) = obj.get("text")
                    {
                        f.text.push_str(s);
                        return;
                    }
                    // SoftBreak/HardBreak inside fenced → newline in raw
                    if let Value::Object(ref obj) = node
                        && let Some(Value::String(t)) = obj.get("t")
                        && (t == "soft_break" || t == "hard_break")
                    {
                        f.text.push('\n');
                        return;
                    }
                }
                _ => {}
            }
            f.inline.push(node);
        } else {
            self.blocks.push(node);
        }
    }

    /// Flush a completed block node into the parent frame's inline list
    /// (for nested blocks like list items) or into the top-level block list.
    fn flush_block(&mut self, node: Value) {
        if let Some(f) = self.stack.last_mut() {
            f.inline.push(node);
        } else {
            self.blocks.push(node);
        }
    }

    /// Push a leaf block node directly onto the top-level block list,
    /// bypassing the stack. Used for `hr`, `math_block`, and raw HTML blocks
    /// that appear at document root.
    fn push_block(&mut self, node: Value) {
        self.blocks.push(node);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a pulldown-cmark [`HeadingLevel`] to a plain integer (1–6).
fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Map a GFM [`BlockQuoteKind`] alert type to its string tag.
fn blockquote_kind(k: BlockQuoteKind) -> &'static str {
    match k {
        BlockQuoteKind::Note => "note",
        BlockQuoteKind::Tip => "tip",
        BlockQuoteKind::Important => "important",
        BlockQuoteKind::Warning => "warning",
        BlockQuoteKind::Caution => "caution",
    }
}

/// Map a pulldown-cmark [`Alignment`] to its CSS-style string name.
fn col_align(a: Alignment) -> &'static str {
    match a {
        Alignment::Left => "left",
        Alignment::Right => "right",
        Alignment::Center => "center",
        Alignment::None => "none",
    }
}

/// Parse a fenced info string into `(name, attrs)`.
///
/// Format: `<name> [key=value|key="quoted value"|flag]*`
///
/// The **first** whitespace-separated token is the name (e.g. the language
/// or plugin identifier). The remaining tokens are attributes.
pub fn parse_fence_info(info: &str) -> (String, HashMap<String, Value>) {
    let tokens = crate::event_filter::tokenize_attrs(info);
    let mut iter = tokens.into_iter();
    let name = iter.next().unwrap_or_default();
    let mut attrs = HashMap::new();
    for token in iter {
        if let Some((k, v)) = token.split_once('=') {
            attrs.insert(k.to_owned(), json!(v));
        } else {
            attrs.insert(token, json!(true));
        }
    }
    (name, attrs)
}

fn merge_directives(mut blocks: Vec<Value>, directives: DirectiveTable) -> Vec<Value> {
    for (start_byte, node) in directives {
        let idx = blocks
            .iter()
            .position(|b| {
                if let Some(arr) = b.get("range").and_then(|v| v.as_array())
                    && let Some(start) = arr.first().and_then(|v| v.as_u64())
                {
                    return start as usize >= start_byte;
                }
                false
            })
            .unwrap_or(blocks.len());
        blocks.insert(idx, node);
    }
    blocks
}

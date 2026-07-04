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
pub fn build_ast(events: Vec<(Event<'_>, std::ops::Range<usize>)>) -> Vec<Value> {
    AstBuilder::new(events).build()
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
    events: std::vec::IntoIter<(Event<'a>, std::ops::Range<usize>)>,
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
    fn new(events: Vec<(Event<'a>, std::ops::Range<usize>)>) -> Self {
        Self {
            events: events.into_iter(),
            blocks: Vec::new(),
            stack: Vec::new(),
            in_table_head: false,
            table_headers: Vec::new(),
            table_rows: Vec::new(),
            current_row: Vec::new(),
        }
    }

    fn build(mut self) -> Vec<Value> {
        while let Some((event, range)) = self.events.next() {
            self.handle(event, range);
        }
        self.blocks
    }

    fn handle(&mut self, event: Event<'_>, range: std::ops::Range<usize>) {
        match event {
            // ── Block open tags ───────────────────────────────────────────
            Event::Start(tag) => self.open(tag, range),

            // ── Block close tags ──────────────────────────────────────────
            Event::End(end) => self.close(end),

            // ── Leaf events (no paired End) ───────────────────────────────
            Event::Rule => self.push_block(json!({"t": "hr", "range": [range.start, range.end]})),

            Event::TaskListMarker(checked) => {
                self.push_inline(json!({"t": "task_marker", "checked": checked, "range": [range.start, range.end]}));
            }

            Event::Text(t) => {
                let s = t.as_ref();
                // Scan the text for inline widgets :[text]{name k=v ...}
                // and emit multiple inline nodes if needed.
                for node in scan_inline_widgets(s, range.start) {
                    self.push_inline(node);
                }
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
                    self.push_block(json!({"t": "raw_html", "html": s, "range": [range.start, range.end]}));
                } else {
                    self.push_inline(json!({"t": "raw_html", "html": s, "range": [range.start, range.end]}));
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
                let f = Frame::new("footnote_def", range.clone()).with("label", json!(label.as_ref()));
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

            Tag::DefinitionList => self.stack.push(Frame::new("definition_list", range.clone())),
            Tag::DefinitionListTitle => self.stack.push(Frame::new("_def_title", range.clone())),
            Tag::DefinitionListDefinition => self.stack.push(Frame::new("_def_body", range.clone())),

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
                    let children = coalesce_and_scan_widgets(f.inline.drain(..).collect());
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
    let tokens = tokenize_attrs(info);
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

/// Parse an attribute block string where **all** tokens are `key=value` or
/// bare flags — there is no leading name token.
///
/// Used for `[link](url){key=value ...}` and `![img](src){key=value ...}`.
fn parse_attr_block(s: &str) -> HashMap<String, Value> {
    let mut attrs = HashMap::new();
    for token in tokenize_attrs(s) {
        if let Some((k, v)) = token.split_once('=') {
            attrs.insert(k.to_owned(), json!(v));
        } else {
            attrs.insert(token, json!(true));
        }
    }
    attrs
}

/// Tokenize a space-separated attribute string, respecting `"quoted values"`.
fn tokenize_attrs(s: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in s.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
            }
            ' ' | '\t' if !in_quote => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Try to parse an inline widget shortcode: `:[text]{name key=value ...}`
///
/// Returns `None` if the string does not match the pattern at position 0.
fn parse_inline_widget(s: &str) -> Option<(Value, usize)> {
    let rest = s.strip_prefix(":[")?;
    let bracket_end = rest.find("]{")?;
    let text = &rest[..bracket_end];
    let after = &rest[bracket_end + 2..];
    let brace_end = after.find('}')?;
    let attrs_str = &after[..brace_end];
    let (name, attrs) = parse_fence_info(attrs_str);
    if name.is_empty() {
        return None;
    }
    let consumed = 2 + bracket_end + 2 + brace_end + 1; // :[  ]{ attrs }
    Some((
        json!({
            "t": "widget",
            "name": name,
            "text": text,
            "attrs": attrs
        }),
        consumed,
    ))
}

/// Scan a text string for zero or more inline widgets, emitting plain text
/// nodes for surrounding content and widget nodes for each match.
fn scan_inline_widgets(s: &str, mut start_offset: usize) -> Vec<Value> {
    let mut result = Vec::new();
    let mut remaining = s;

    while !remaining.is_empty() {
        if let Some(pos) = remaining.find(":[") {
            // Emit text before the widget
            if pos > 0 {
                let chunk_len = remaining[..pos].len();
                result.push(json!({"t": "text", "text": &remaining[..pos], "range": [start_offset, start_offset + chunk_len]}));
                start_offset += chunk_len;
            }
            let candidate = &remaining[pos..];
            if let Some((mut widget, consumed)) = parse_inline_widget(candidate) {
                if let Value::Object(ref mut map) = widget {
                    map.insert("range".into(), json!([start_offset, start_offset + consumed]));
                }
                result.push(widget);
                remaining = &remaining[pos + consumed..];
                start_offset += consumed;
            } else {
                // Not a valid widget — emit the `:[` literally and advance past it
                let chunk_len = remaining[..pos + 2].len();
                result.push(json!({"t": "text", "text": &remaining[..pos + 2], "range": [start_offset, start_offset + chunk_len]}));
                remaining = &remaining[pos + 2..];
                start_offset += chunk_len;
            }
        } else {
            let chunk_len = remaining.len();
            result.push(json!({"t": "text", "text": remaining, "range": [start_offset, start_offset + chunk_len]}));
            break;
        }
    }

    result
}

/// Merge consecutive plain-text children in an inline list, scan the merged
/// buffer for widgets, absorb trailing `{attrs}` into preceding link/image
/// nodes, and return the expanded result.
///
/// Non-text nodes (em, strong, links, etc.) are kept in place as boundaries
/// unless immediately followed by an attr block.
fn coalesce_and_scan_widgets(children: Vec<Value>) -> Vec<Value> {
    // Pass 1: coalesce consecutive text nodes and scan for widgets.
    let mut pass1: Vec<Value> = Vec::new();
    let mut text_buf = String::new();
    let mut current_offset: Option<usize> = None;

    let flush_text = |buf: &mut String, offset: Option<usize>, out: &mut Vec<Value>| {
        if !buf.is_empty() {
            for node in scan_inline_widgets(buf, offset.unwrap_or(0)) {
                out.push(node);
            }
            buf.clear();
        }
    };

    for child in children {
        match child.get("t").and_then(|v| v.as_str()) {
            Some("text") => {
                if let Some(s) = child.get("text").and_then(|v| v.as_str()) {
                    if current_offset.is_none() {
                        if let Some(arr) = child.get("range").and_then(|v| v.as_array()) {
                            if let Some(n) = arr.get(0).and_then(|v| v.as_u64()) {
                                current_offset = Some(n as usize);
                            }
                        }
                    }
                    text_buf.push_str(s);
                }
            }
            _ => {
                flush_text(&mut text_buf, current_offset, &mut pass1);
                current_offset = None;
                pass1.push(child);
            }
        }
    }
    flush_text(&mut text_buf, current_offset, &mut pass1);

    // Pass 2: absorb trailing `{...}` text nodes into preceding link/image.
    let mut result: Vec<Value> = Vec::new();
    let mut iter = pass1.into_iter().peekable();

    while let Some(mut node) = iter.next() {
        let t = node
            .get("t")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        if matches!(t.as_str(), "link" | "image") {
            // Clone the peeked value to release the borrow before calling next().
            let maybe_attrs: Option<(String, String)> = iter.peek().and_then(|next| {
                let text = next.get("text")?.as_str()?;
                let attrs_block = text.strip_prefix('{')?;
                let end = attrs_block.find('}')?;
                Some((
                    attrs_block[..end].to_owned(),
                    attrs_block[end + 1..].to_owned(),
                ))
            });

            if let Some((attrs_str, remainder)) = maybe_attrs {
                let attrs = parse_attr_block(&attrs_str);
                if !attrs.is_empty() {
                    // Absorb attrs into the link/image node.
                    if let Value::Object(ref mut m) = node {
                        let existing = m.get("attrs").cloned();
                        let mut merged = match existing {
                            Some(Value::Object(e)) => e,
                            _ => serde_json::Map::new(),
                        };
                        merged.extend(attrs);
                        m.insert("attrs".into(), Value::Object(merged));
                    }
                    iter.next(); // consume the text node — borrow is now dropped
                    if !remainder.trim().is_empty() {
                        result.push(node);
                        result.push(json!({"t": "text", "text": remainder})); // Note: range is lost for this synthetic text fragment, acceptable for inline attr trailing
                        continue;
                    }
                }
            }
        }
        result.push(node);
    }

    result
}

//    Copyright [2026] [Purnendu Kumar]
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

//! Pre-passes and event stream transformations.
//!
//! 1. `frontmatter_prepass`: extracts YAML from `---` blocks.
//! 2. `directive_prepass`: extracts `:::name` blocks into a side-table and masks them.
//! 3. `parse`: standard pulldown-cmark run on the masked source.
//! 4. `transform_events`: rich inline transformations (widgets, link attrs).

use pulldown_cmark::{Event, Parser, TagEnd};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::ops::Range;

use crate::ast::parse_fence_info;
use crate::config::parser_options; // Re-use the one in ast.rs for now

pub type DirectiveTable = Vec<(usize, Value)>;

pub enum RichEvent<'a> {
    Cmark(Event<'a>, Range<usize>),
    Widget {
        name: String,
        text: String,
        attrs: HashMap<String, Value>,
        range: Range<usize>,
    },
    LinkImageAttrs(HashMap<String, Value>),
}

// ---------------------------------------------------------------------------
// Pre-passes
// ---------------------------------------------------------------------------

/// Strips the YAML frontmatter block (if present).
/// Returns `(body_string, Some(raw_yaml_string))` or `(original_string, None)`.
pub fn frontmatter_prepass(raw: &str) -> (String, Option<String>) {
    let mut offset = if let Some(suffix) = raw.strip_prefix("---\n") {
        raw.len() - suffix.len()
    } else if let Some(suffix) = raw.strip_prefix("---\r\n") {
        raw.len() - suffix.len()
    } else {
        return (raw.to_string(), None);
    };

    let mut frontmatter = String::new();

    while offset < raw.len() {
        let remaining = &raw[offset..];
        let line_len = remaining.find('\n').map_or(remaining.len(), |idx| idx + 1);
        let line = &remaining[..line_len];
        let trimmed = line.trim_end_matches(['\r', '\n']);

        if trimmed == "---" || trimmed == "..." {
            return (raw[offset + line_len..].to_string(), Some(frontmatter));
        }

        frontmatter.push_str(line);
        offset += line_len;
    }

    (raw.to_string(), None)
}

struct DirectiveFrame {
    start_byte: usize,
    name: String,
    attrs: HashMap<String, Value>,
    body_start: usize,
}

pub fn directive_prepass(src: &str) -> (String, DirectiveTable) {
    let mut masked = String::with_capacity(src.len());
    let mut directives = Vec::new();
    let mut stack: Vec<DirectiveFrame> = Vec::new();
    let mut in_fenced = false;
    let mut fence_char = '`';
    let mut fence_len = 0;

    let mut offset = 0;
    for line in src.split_inclusive('\n') {
        let line_len = line.len();
        let trimmed = line.trim_start_matches(' ');
        let indent = line_len - trimmed.len();

        // Handle fenced blocks guarding
        if indent < 4 {
            if !in_fenced {
                if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                    in_fenced = true;
                    fence_char = trimmed.chars().next().unwrap();
                    fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
                }
            } else if trimmed.starts_with(fence_char) {
                let end_len = trimmed.chars().take_while(|&c| c == fence_char).count();
                if end_len >= fence_len {
                    in_fenced = false;
                }
            }
        }

        let mut handled = false;

        if !in_fenced && indent < 4 && trimmed.starts_with(":::") {
            let rest = &trimmed[3..];
            let is_close = rest.trim_end().is_empty() || rest.starts_with('/');

            if is_close && !stack.is_empty() {
                // Close directive
                let frame = stack.pop().unwrap();
                let _body_len = offset - frame.body_start;
                let body = &src[frame.body_start..offset];

                // Recursively process body
                let (inner_masked, inner_directives) = directive_prepass(body);
                let inner_events = parse(&inner_masked);
                let inner_rich = transform_events(inner_events);
                let inner_ast = crate::ast::build_ast(inner_rich, inner_directives);

                let node = json!({
                    "t": "directive",
                    "name": frame.name,
                    "attrs": frame.attrs,
                    "children": inner_ast,
                    "range": [frame.start_byte, offset + line_len]
                });
                if stack.is_empty() {
                    directives.push((frame.start_byte, node));
                }

                // Mask the line
                masked.push_str(&mask_line(line));
                handled = true;
            } else if !is_close {
                // Open directive
                let info = rest.trim_end();
                if !info.is_empty() {
                    let (name, attrs) = parse_fence_info(info);
                    if !name.is_empty() {
                        stack.push(DirectiveFrame {
                            start_byte: offset,
                            name,
                            attrs,
                            body_start: offset + line_len,
                        });
                    }
                }

                // Mask the line if we are inside or opening a directive
                if !stack.is_empty() {
                    masked.push_str(&mask_line(line));
                    handled = true;
                }
            }
        }

        if !handled {
            if !stack.is_empty() {
                masked.push_str(&mask_line(line));
            } else {
                masked.push_str(line);
            }
        }

        offset += line_len;
    }

    if !stack.is_empty() {
        let first_open = stack[0].start_byte;
        masked.truncate(first_open);
        masked.push_str(&src[first_open..]);
    }

    (masked, directives)
}

fn mask_line(line: &str) -> String {
    let len = line.len();
    if len == 0 {
        return String::new();
    }
    let mut s = String::with_capacity(len);
    if line.ends_with("\r\n") {
        s.push_str(&" ".repeat(len - 2));
        s.push_str("\r\n");
    } else if line.ends_with('\n') {
        s.push_str(&" ".repeat(len - 1));
        s.push('\n');
    } else {
        s.push_str(&" ".repeat(len));
    }
    s
}

pub fn parse<'a>(raw: &'a str) -> Vec<(Event<'a>, Range<usize>)> {
    let parser = Parser::new_ext(raw, parser_options());
    parser.into_offset_iter().collect()
}

// ---------------------------------------------------------------------------
// Event Transforms
// ---------------------------------------------------------------------------

pub fn transform_events<'a>(events: Vec<(Event<'a>, Range<usize>)>) -> Vec<RichEvent<'a>> {
    // Pass 1: coalesce adjacent Text and SoftBreak
    let mut pass1 = Vec::new();
    let mut text_buf = String::new();
    let mut current_offset = None;

    let flush =
        |buf: &mut String, off: &mut Option<usize>, out: &mut Vec<(Event<'a>, Range<usize>)>| {
            if !buf.is_empty() {
                let start = off.unwrap();
                let end = start + buf.len(); // approx, we don't care exactly if it's just for scanning
                // For scanning, we only need a continuous string and start offset.
                let e = Event::Text(buf.clone().into());
                out.push((e, start..end));
                buf.clear();
                *off = None;
            }
        };

    for (event, range) in events {
        match event {
            Event::Text(ref t) => {
                if current_offset.is_none() {
                    current_offset = Some(range.start);
                }
                text_buf.push_str(t.as_ref());
            }
            Event::SoftBreak => {
                if current_offset.is_none() {
                    current_offset = Some(range.start);
                }
                text_buf.push('\n');
            }
            other => {
                flush(&mut text_buf, &mut current_offset, &mut pass1);
                pass1.push((other, range));
            }
        }
    }
    flush(&mut text_buf, &mut current_offset, &mut pass1);

    // Pass 2: scan widgets in text
    let mut pass2 = Vec::new();
    for (event, range) in pass1 {
        if let Event::Text(t) = event {
            let s = t.as_ref();
            for rich in scan_inline_widgets(s, range.start) {
                pass2.push(rich);
            }
        } else {
            pass2.push(RichEvent::Cmark(event, range));
        }
    }

    // Pass 3: Absorb optional {key=val} suffix into preceding Link/Image
    let mut result = Vec::with_capacity(pass2.len());
    let mut link_image_starts = Vec::new();

    let mut iter = pass2.into_iter().peekable();

    while let Some(rich) = iter.next() {
        if let RichEvent::Cmark(Event::Start(pulldown_cmark::Tag::Link { .. }), _)
        | RichEvent::Cmark(Event::Start(pulldown_cmark::Tag::Image { .. }), _) = &rich
        {
            // Track the index where this start event goes
            link_image_starts.push(result.len());
            result.push(rich);
            continue;
        }

        if let RichEvent::Cmark(Event::End(TagEnd::Link), _)
        | RichEvent::Cmark(Event::End(TagEnd::Image), _) = &rich
        {
            let start_idx = link_image_starts.pop();

            let mut extracted = None;
            if let Some(RichEvent::Cmark(Event::Text(t), text_range)) = iter.peek() {
                let s = t.as_ref();
                if s.starts_with('{')
                    && let Some(end) = s.find('}')
                {
                    extracted = Some((
                        s[1..end].to_string(),
                        s[end + 1..].to_string(),
                        text_range.start + end + 1,
                        text_range.end,
                    ));
                }
            }

            if let Some((attrs_str, remainder, remainder_start, remainder_end)) = extracted {
                let attrs = parse_attr_block(&attrs_str);

                if let Some(idx) = start_idx
                    && !attrs.is_empty()
                {
                    result.insert(idx + 1, RichEvent::LinkImageAttrs(attrs));
                }

                iter.next(); // Consume the text event

                if !remainder.trim_start().is_empty() {
                    result.push(rich);
                    result.push(RichEvent::Cmark(
                        Event::Text(remainder.to_string().into()),
                        remainder_start..remainder_end,
                    ));
                    continue;
                }
            }
        }

        result.push(rich);
    }

    result
}

pub fn tokenize_attrs(s: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in s.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
            }
            ' ' | '\t' | '\n' | '\r' | ',' if !in_quote => {
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

pub fn parse_attr_block(s: &str) -> HashMap<String, Value> {
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

fn parse_inline_widget(s: &str) -> Option<(String, String, HashMap<String, Value>, usize)> {
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
    let consumed = 2 + bracket_end + 2 + brace_end + 1;
    Some((name, text.to_string(), attrs, consumed))
}

fn scan_inline_widgets<'a>(s: &str, mut start_offset: usize) -> Vec<RichEvent<'a>> {
    let mut result = Vec::new();
    let mut remaining = s;

    while !remaining.is_empty() {
        if let Some(pos) = remaining.find(":[") {
            if pos > 0 {
                let chunk_len = remaining[..pos].len();
                let t = &remaining[..pos];
                // For RichEvent we need an owned String or cow.
                // Let's use pulldown_cmark::CowStr.
                result.push(RichEvent::Cmark(
                    Event::Text(t.to_string().into()),
                    start_offset..start_offset + chunk_len,
                ));
                start_offset += chunk_len;
            }
            let candidate = &remaining[pos..];
            if let Some((name, text, attrs, consumed)) = parse_inline_widget(candidate) {
                result.push(RichEvent::Widget {
                    name,
                    text,
                    attrs,
                    range: start_offset..start_offset + consumed,
                });
                remaining = &remaining[pos + consumed..];
                start_offset += consumed;
            } else {
                let chunk_len = remaining[..pos + 2].len();
                let t = &remaining[..pos + 2];
                result.push(RichEvent::Cmark(
                    Event::Text(t.to_string().into()),
                    start_offset..start_offset + chunk_len,
                ));
                remaining = &remaining[pos + 2..];
                start_offset += chunk_len;
            }
        } else {
            let chunk_len = remaining.len();
            result.push(RichEvent::Cmark(
                Event::Text(remaining.to_string().into()),
                start_offset..start_offset + chunk_len,
            ));
            break;
        }
    }

    result
}

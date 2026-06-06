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

//! Frontmatter extraction layer.
//!
//! Wraps pulldown-cmark's parser and strips the YAML metadata block (if
//! present) from the event stream before the AST builder sees it.
//! The raw YAML text is returned separately in [`ParsedDocument::frontmatter`]
//! so the native deploy pass can parse it with `serde_yml`.

use pulldown_cmark::{Event, MetadataBlockKind, Parser, Tag, TagEnd};

use crate::config::{FrontmatterMode, parser_options};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Raw output of one parse pass over a source document.
#[derive(Debug)]
pub struct ParsedDocument<'a> {
    /// Raw YAML text between `---` delimiters, if present and mode is Enabled.
    pub frontmatter: Option<String>,
    /// All markdown events with frontmatter stripped out.
    pub events: Vec<Event<'a>>,
}

// ---------------------------------------------------------------------------
// Core parse pass
// ---------------------------------------------------------------------------

/// Parse `raw` into a [`ParsedDocument`].
///
/// Frontmatter is collected and removed from the event stream when
/// `mode == FrontmatterMode::Enabled`. All other events pass through
/// unchanged — the AST builder in `src/ast.rs` interprets them.
pub fn parse<'a>(raw: &'a str, mode: FrontmatterMode) -> ParsedDocument<'a> {
    let parser = Parser::new_ext(raw, parser_options(mode));

    let mut frontmatter_buf = String::new();
    let mut in_frontmatter = false;
    let mut events: Vec<Event<'a>> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                in_frontmatter = true;
                frontmatter_buf.clear();
            }
            Event::End(TagEnd::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                in_frontmatter = false;
            }
            Event::Text(ref t) if in_frontmatter => {
                frontmatter_buf.push_str(t.as_ref());
            }
            other if in_frontmatter => {
                // Swallow other events inside frontmatter block.
                let _ = other;
            }
            other => {
                events.push(other);
            }
        }
    }

    ParsedDocument {
        frontmatter: (!frontmatter_buf.trim().is_empty()).then_some(frontmatter_buf),
        events,
    }
}

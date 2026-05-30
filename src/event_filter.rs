use pulldown_cmark::{CodeBlockKind, Event, MetadataBlockKind, Parser, Tag, TagEnd};

use crate::config::{parser_options, CompilationMode};
use crate::plugins::PluginRegistry;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginBlock {
    pub name: String,
    pub body: String,
}

/// A single token in the processed event stream.  Plugin fences are lifted
/// out of the raw pulldown-cmark stream into a typed variant so that each
/// rendering target can handle them without string-parsing sentinels.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkplusEvent<'a> {
    Markdown(Event<'a>),
    Plugin(PluginBlock),
}

/// The result of a full parse pass over one document.
#[derive(Debug)]
pub struct FilteredDocument<'a> {
    /// Raw YAML frontmatter text, present only when the document opened with
    /// a `---` block and the parser was in `Native` mode.
    pub frontmatter: Option<String>,
    /// Markdown body stripped of frontmatter + plugin blocks replaced with
    /// typed `Plugin` variants.
    pub events: Vec<MarkplusEvent<'a>>,
}

// ---------------------------------------------------------------------------
// Core filter pass
// ---------------------------------------------------------------------------

pub fn intercept_events<'a>(raw: &'a str, mode: CompilationMode, registry: &PluginRegistry) -> FilteredDocument<'a> {
    let parser = Parser::new_ext(raw, parser_options(mode));

    let mut frontmatter_buf = String::new();
    let mut in_frontmatter = false;

    let mut plugin_name: Option<String> = None;
    let mut plugin_body = String::new();

    let mut events: Vec<MarkplusEvent<'a>> = Vec::new();

    for event in parser {
        match event {
            // ── Frontmatter open / close ─────────────────────────────────
            Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                in_frontmatter = true;
                frontmatter_buf.clear();
            }
            Event::End(TagEnd::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                in_frontmatter = false;
            }

            // ── Plugin fence open ────────────────────────────────────────
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info)))
                if registry.get(info.as_ref().split_whitespace().next().unwrap_or("")).is_some() =>
            {
                plugin_name = Some(
                    info.as_ref()
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_owned(),
                );
                plugin_body.clear();
            }

            // ── Code block close ─────────────────────────────────────────
            Event::End(TagEnd::CodeBlock) => {
                if let Some(name) = plugin_name.take() {
                    events.push(MarkplusEvent::Plugin(PluginBlock {
                        name,
                        body: plugin_body.trim().to_owned(),
                    }));
                    plugin_body.clear();
                } else if !in_frontmatter {
                    // Normal (non-plugin) code block end — pass through.
                    events.push(MarkplusEvent::Markdown(Event::End(TagEnd::CodeBlock)));
                }
            }

            // ── Text while inside a special region ───────────────────────
            other if in_frontmatter => collect_text(&mut frontmatter_buf, &other),
            other if plugin_name.is_some() => collect_text(&mut plugin_body, &other),

            // ── Everything else passes through ───────────────────────────
            other => events.push(MarkplusEvent::Markdown(other)),
        }
    }

    FilteredDocument {
        frontmatter: (!frontmatter_buf.trim().is_empty()).then_some(frontmatter_buf),
        events,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_text(buf: &mut String, event: &Event<'_>) {
    match event {
        Event::Text(t)
        | Event::Code(t)
        | Event::Html(t)
        | Event::InlineHtml(t) => buf.push_str(t.as_ref()),
        Event::SoftBreak | Event::HardBreak => buf.push('\n'),
        _ => {}
    }
}

use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use serde_json::{json, Value};

use crate::event_filter::MarkplusEvent;

pub fn compile_to_ast<'a>(stream: impl IntoIterator<Item = MarkplusEvent<'a>>) -> Vec<Value> {
    let mut tokens = Vec::new();
    let mut text_buf = String::new();
    let mut current_tag: Option<Tag<'_>> = None;

    // For table state
    let mut in_table_head = false;
    let mut table_headers = Vec::new();
    let mut table_rows = Vec::new();
    let mut current_row = Vec::new();

    let mut stream = stream.into_iter();

    while let Some(ev) = stream.next() {
        match ev {
            MarkplusEvent::Plugin(plugin) => {
                tokens.push(json!({
                    "type": "plugin",
                    "engine": plugin.name,
                    "raw": plugin.body
                }));
            }
            MarkplusEvent::Markdown(event) => match event {
                Event::Start(tag) => {
                    current_tag = Some(tag.clone());
                    text_buf.clear();

                    match tag {
                        Tag::TableHead => in_table_head = true,
                        Tag::TableRow => current_row.clear(),
                        _ => {}
                    }
                }
                Event::End(end_tag) => {
                    match end_tag {
                        TagEnd::Heading(level) => {
                            let level_num = level as u32;
                            tokens.push(json!({
                                "type": "heading",
                                "level": level_num,
                                "text": text_buf.trim()
                            }));
                        }
                        TagEnd::Paragraph => {
                            tokens.push(json!({
                                "type": "paragraph",
                                "text": text_buf.trim()
                            }));
                        }
                        TagEnd::CodeBlock => {
                            if let Some(Tag::CodeBlock(kind)) = &current_tag {
                                let lang = match kind {
                                    CodeBlockKind::Fenced(l) => l.as_ref(),
                                    CodeBlockKind::Indented => "",
                                };
                                tokens.push(json!({
                                    "type": "code_block",
                                    "language": lang,
                                    "code": text_buf.trim_end()
                                }));
                            }
                        }
                        TagEnd::TableCell => {
                            if in_table_head {
                                table_headers.push(text_buf.trim().to_string());
                            } else {
                                current_row.push(text_buf.trim().to_string());
                            }
                            text_buf.clear();
                        }
                        TagEnd::TableRow => {
                            if !in_table_head {
                                table_rows.push(current_row.clone());
                            }
                        }
                        TagEnd::TableHead => {
                            in_table_head = false;
                        }
                        TagEnd::Table => {
                            tokens.push(json!({
                                "type": "table",
                                "headers": table_headers,
                                "rows": table_rows
                            }));
                            table_headers.clear();
                            table_rows.clear();
                        }
                        TagEnd::List(_start) => {
                            // Simplified list handling for now
                        }
                        TagEnd::Item => {
                            tokens.push(json!({
                                "type": "list_item",
                                "text": text_buf.trim()
                            }));
                        }
                        _ => {}
                    }
                    current_tag = None;
                    text_buf.clear();
                }
                Event::Text(t) | Event::Code(t) | Event::Html(t) | Event::InlineHtml(t) => {
                    text_buf.push_str(t.as_ref());
                }
                Event::SoftBreak | Event::HardBreak => {
                    text_buf.push('\n');
                }
                Event::Rule => {
                    tokens.push(json!({ "type": "hr" }));
                }
                _ => {}
            },
        }
    }

    tokens
}

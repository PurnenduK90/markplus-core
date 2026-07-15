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

//! Minimal pure-Rust YAML subset parser for frontmatter.
//!
//! Replaces `serde_yml` to eliminate dependencies and unify native/wasm behaviour.
//! Supports basic scalars (strings, quoted strings, ints, floats, bools, dates),
//! block lists, flow lists, and nested objects.

use crate::CompileError;
use serde_json::{Map, Value};

/// Parse raw YAML frontmatter text into a JSON Value.
/// Returns Ok(None) for empty/missing input.
pub(crate) fn parse_yaml_frontmatter(raw: Option<&str>) -> Result<Option<Value>, CompileError> {
    let raw = match raw {
        Some(s) => s.trim(),
        None => return Ok(None),
    };
    if raw.is_empty() {
        return Ok(None);
    }

    let mut lines = raw.lines().peekable();
    if let Some(Value::Object(map)) = parse_block(&mut lines, 0) {
        if map.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Value::Object(map)))
        }
    } else {
        Err(CompileError::InvalidFrontmatter("invalid block".into()))
    }
}

fn parse_block(
    lines: &mut std::iter::Peekable<std::str::Lines>,
    current_indent: usize,
) -> Option<Value> {
    let mut map = Map::new();
    let mut list = Vec::new();
    let mut is_list = false;

    while let Some(&line) = lines.peek() {
        if line.trim().is_empty() {
            lines.next();
            continue;
        }

        let indent = get_indent(line);
        if indent < current_indent {
            break; // End of this block
        }
        if indent > current_indent {
            // Should not happen for valid well-formed block start, but skip or err?
            // Usually we consume strictly. Let's just break if it's over-indented unexpectedly.
            break;
        }

        let line = lines.next().unwrap();
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("- ") {
            is_list = true;
            let val_str = rest.trim();
            if val_str.is_empty() {
                // Next line might be nested block
                if let Some(&next) = lines.peek() {
                    let next_ind = get_indent(next);
                    if next_ind > current_indent {
                        if let Some(v) = parse_block(lines, next_ind) {
                            list.push(v);
                        }
                    } else {
                        list.push(Value::Null);
                    }
                }
            } else {
                list.push(parse_scalar(val_str));
            }
        } else if let Some((key, val)) = split_key_val(trimmed) {
            let key = key.to_string();
            if val.is_empty() {
                // Next line might be nested block or list
                if let Some(&next) = lines.peek() {
                    let next_ind = get_indent(next);
                    if next_ind > current_indent {
                        if let Some(v) = parse_block(lines, next_ind) {
                            map.insert(key, v);
                        }
                    } else {
                        map.insert(key, Value::Null);
                    }
                } else {
                    map.insert(key, Value::Null);
                }
            } else {
                map.insert(key, parse_scalar(val));
            }
        } else {
            // Unrecognized line format
            return None;
        }
    }

    if is_list {
        Some(Value::Array(list))
    } else {
        Some(Value::Object(map))
    }
}

fn get_indent(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ').count()
}

fn split_key_val(s: &str) -> Option<(&str, &str)> {
    let colon = s.find(':')?;
    // Colon must be followed by space or end of line in YAML, unless in quotes (but keys aren't usually quoted here)
    if colon == s.len() - 1 || s[colon + 1..].starts_with(' ') {
        Some((s[..colon].trim(), s[colon + 1..].trim()))
    } else {
        None
    }
}

fn parse_scalar(s: &str) -> Value {
    if s == "true" {
        Value::Bool(true)
    } else if s == "false" {
        Value::Bool(false)
    } else if s.starts_with('[') && s.ends_with(']') {
        // Flow list
        let inner = &s[1..s.len() - 1];
        let items: Vec<Value> = inner.split(',').map(|x| parse_scalar(x.trim())).collect();
        Value::Array(items)
    } else if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\''))
    {
        Value::String(s[1..s.len() - 1].to_string())
    } else if let Ok(i) = s.parse::<i64>() {
        Value::Number(i.into())
    } else if let Ok(f) = s.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            Value::Number(n)
        } else {
            Value::String(s.to_string())
        }
    } else {
        // Treat as string (includes dates)
        Value::String(s.to_string())
    }
}

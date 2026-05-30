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

use serde_json::Value;
use crate::plugins::PluginRegistry;

pub fn compile_to_typst(tokens: &[Value], registry: &PluginRegistry) -> String {
    let mut typst = String::new();

    for token in tokens {
        match token["type"].as_str().unwrap_or("") {
            "heading" => {
                let level = token["level"].as_u64().unwrap_or(1) as usize;
                let text = token["text"].as_str().unwrap_or("");
                let eq = "=".repeat(level);
                typst.push_str(&format!("{} {}\n\n", eq, text));
            }
            "paragraph" => {
                let text = token["text"].as_str().unwrap_or("");
                typst.push_str(&format!("{}\n\n", text));
            }
            "plugin" => {
                let engine = token["engine"].as_str().unwrap_or("");
                if let Some(p) = registry.get(engine) {
                    if let Some(rendered) = p.render_typst(token) {
                        typst.push_str(&rendered);
                        typst.push_str("\n\n");
                    }
                }
            }
            "code_block" => {
                let lang = token["language"].as_str().unwrap_or("");
                let code = token["code"].as_str().unwrap_or("");
                typst.push_str(&format!("```{}\n{}\n```\n\n", lang, code));
            }
            "hr" => {
                typst.push_str("---\n\n");
            }
            "list_item" => {
                let text = token["text"].as_str().unwrap_or("");
                typst.push_str(&format!("- {}\n", text));
            }
            "table" => {
                typst.push_str("#table(\n");
                
                let headers = token["headers"].as_array().unwrap();
                let columns = headers.len();
                typst.push_str(&format!("  columns: {},\n", columns));
                
                for h in headers {
                    typst.push_str(&format!("  [{}],\n", h.as_str().unwrap_or("")));
                }
                
                if let Some(rows) = token["rows"].as_array() {
                    for row in rows {
                        if let Some(cells) = row.as_array() {
                            for cell in cells {
                                typst.push_str(&format!("  [{}],\n", cell.as_str().unwrap_or("")));
                            }
                        }
                    }
                }
                typst.push_str(")\n\n");
            }
            _ => {}
        }
    }

    typst.trim().to_owned()
}
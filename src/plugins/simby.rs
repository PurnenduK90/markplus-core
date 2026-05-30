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

use super::{Plugin, escape_attr, escape_typst_string};
use serde_json::Value;

pub struct SimbyPlugin;

impl Plugin for SimbyPlugin {
    fn name(&self) -> &'static str {
        "simby"
    }

    fn render_html(&self, token: &Value) -> Option<String> {
        let raw = token.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        Some(format!("<div class=\"markplus-plugin\" data-plugin=\"simby\" data-raw=\"{}\"></div>", escape_attr(raw)))
    }

    fn render_typst(&self, token: &Value) -> Option<String> {
        let raw = token.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        Some(format!("#markplus-simby(\"{}\")", escape_typst_string(raw)))
    }
}

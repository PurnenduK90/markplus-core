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

pub mod mermaid;
pub mod simby;

pub struct HtmlDependencies {
    pub scripts: Vec<String>,
    pub wasm_modules: Vec<String>,
    pub styles: Vec<String>,
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn parse(&self, _raw: &str) -> Option<Value> { None }
    fn render_html(&self, _token: &Value) -> Option<String> { None }
    fn html_dependencies(&self) -> Option<HtmlDependencies> { None }
    fn render_typst(&self, _token: &Value) -> Option<String> { None }
}

pub struct PluginRegistry {
    plugins: std::collections::HashMap<&'static str, Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.insert(plugin.name(), plugin);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(|b| b.as_ref())
    }
}

// ---------------------------------------------------------------------------
// Shared Utility Functions for Plugins
// ---------------------------------------------------------------------------

pub fn default_registry() -> PluginRegistry {
    let mut reg = PluginRegistry::new();
    reg.register(Box::new(mermaid::MermaidPlugin));
    reg.register(Box::new(simby::SimbyPlugin));
    reg
}

pub fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&'  => out.push_str("&amp;"),
            '<'  => out.push_str("&lt;"),
            '>'  => out.push_str("&gt;"),
            '"'  => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _    => out.push(ch),
        }
    }
    out
}

pub fn escape_typst_string(s: &str) -> String {
    s.replace("\"", "\\\"")
}

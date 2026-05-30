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

use pulldown_cmark::Options;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationMode {
    Native,
    Wasm,
}

/// Output rendering target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputTarget {
    Html,
    Typst,
}

pub fn parser_options(mode: CompilationMode) -> Options {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    // YAML frontmatter is only parsed on native; in wasm the caller already
    // stripped it (they fetched the pre-built JSON which contains the body).
    if matches!(mode, CompilationMode::Native) {
        opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    }
    opts
}


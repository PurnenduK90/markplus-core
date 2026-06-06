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

//! Parser configuration and pulldown-cmark option flags.
//!
//! [`FrontmatterMode`] controls whether the parser intercepts YAML `---` blocks.
//! [`parser_options`] assembles the full [`pulldown_cmark::Options`] set used by
//! every parse pass in this crate.

use pulldown_cmark::Options;

/// Whether the source document may contain YAML frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontmatterMode {
    /// Parse and extract YAML frontmatter (native/deploy pass).
    Enabled,
    /// Skip frontmatter parsing — caller provides pre-stripped body.
    Disabled,
}

/// Build the pulldown-cmark option set.
/// All stable extensions enabled; YAML frontmatter is gated on `mode`.
pub fn parser_options(mode: FrontmatterMode) -> Options {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    opts.insert(Options::ENABLE_MATH);
    opts.insert(Options::ENABLE_GFM);
    opts.insert(Options::ENABLE_DEFINITION_LIST);
    opts.insert(Options::ENABLE_SUPERSCRIPT);
    opts.insert(Options::ENABLE_SUBSCRIPT);
    if matches!(mode, FrontmatterMode::Enabled) {
        opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    }
    opts
}

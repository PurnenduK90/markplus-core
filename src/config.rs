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

/// Build the pulldown-cmark option set.
/// All stable extensions enabled; YAML frontmatter is handled by a pre-pass.
pub fn parser_options() -> Options {
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

    opts
}

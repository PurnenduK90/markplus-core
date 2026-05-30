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


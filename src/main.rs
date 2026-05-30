use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};

use markplus_core::{
    compile_document, preview_html, preview_typst,
    config::OutputTarget,
};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "mpc",
    about = "MarkPlus Core — Markdown → HTML / Typst / JSON compiler",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Full deploy-pass compile: always writes <stem>.json.
    /// Pass --target html|typst to also emit a rendered <stem>.html or <stem>.typ.
    Compile {
        /// Source markdown file (may contain frontmatter)
        file: PathBuf,

        /// `site` (default) writes only the JSON asset + bare body.
        /// `html` or `typst` additionally writes the rendered output file.
        #[arg(short, long, value_enum, default_value = "site")]
        target: CompileTarget,

        /// Output directory (default: same directory as the input file)
        #[arg(short, long)]
        out_dir: Option<PathBuf>,
    },

    /// Stream a raw .md file through the live-preview pipeline and print to
    /// stdout.  Frontmatter is silently stripped.
    Preview {
        /// Source markdown file
        file: PathBuf,

        /// Render target
        #[arg(short, long, value_enum, default_value = "html")]
        target: RenderTarget,
    },

}

/// Target for the `compile` subcommand — includes the `site`-only option.
#[derive(Clone, ValueEnum)]
enum CompileTarget {
    /// Write only <stem>.json (no rendered file)
    Site,
    /// Write JSON + rendered <stem>.html
    Html,
    /// Write JSON + rendered <stem>.typ
    Typst,
    /// Write JSON + rendered <stem>.pdf
    #[cfg(feature = "pdf")]
    Pdf,
}

/// Target for `preview` and `render` subcommands — always produces output.
#[derive(Clone, ValueEnum)]
enum RenderTarget {
    Html,
    Typst,
}

impl From<RenderTarget> for OutputTarget {
    fn from(t: RenderTarget) -> Self {
        match t {
            RenderTarget::Html  => OutputTarget::Html,
            RenderTarget::Typst => OutputTarget::Typst,
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Compile { file, target, out_dir } => cmd_compile(&file, target, out_dir),
        Command::Preview { file, target }          => cmd_preview(&file, target),
    }
}

// ---------------------------------------------------------------------------
// Subcommand implementations
// ---------------------------------------------------------------------------

fn cmd_compile(file: &Path, target: CompileTarget, out_dir: Option<PathBuf>) -> Result<()> {
    let raw_md = fs::read_to_string(file)
        .with_context(|| format!("Cannot read {}", file.display()))?;

    // For site-only mode we still need to call compile_document; use Html as
    // the internal target (cheapest) but discard the rendered field.
    let output_target = match &target {
        CompileTarget::Html  => OutputTarget::Html,
        CompileTarget::Typst => OutputTarget::Typst,
        CompileTarget::Site  => OutputTarget::Html,
        #[cfg(feature = "pdf")]
        CompileTarget::Pdf   => OutputTarget::Typst,
    };

    // Compile everything before touching the filesystem.
    let result = compile_document(&raw_md, output_target)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let stem = file
        .file_stem()
        .context("Input file has no stem")?
        .to_string_lossy();

    let dir = out_dir
        .as_deref()
        .or_else(|| file.parent())
        .unwrap_or_else(|| Path::new("."));

    fs::create_dir_all(dir)
        .with_context(|| format!("Cannot create output dir {}", dir.display()))?;

    // Write output files based on the requested target.
    match target {
        CompileTarget::Site => {
            let json_str = result
                .site_asset
                .to_json()
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let json_path = dir.join(format!("{stem}.json"));
            fs::write(&json_path, &json_str)
                .with_context(|| format!("Cannot write {}", json_path.display()))?;
            println!("✓ {}", json_path.display());

            let md_path = dir.join(format!("{stem}.md"));
            fs::write(&md_path, result.stripped_md.as_bytes())
                .with_context(|| format!("Cannot write {}", md_path.display()))?;
            println!("✓ {}", md_path.display());
        }
        CompileTarget::Html => {
            let path = dir.join(format!("{stem}.html"));
            fs::write(&path, result.rendered.as_bytes())
                .with_context(|| format!("Cannot write {}", path.display()))?;
            println!("✓ {}", path.display());
        }
        CompileTarget::Typst => {
            let path = dir.join(format!("{stem}.typ"));
            fs::write(&path, result.rendered.as_bytes())
                .with_context(|| format!("Cannot write {}", path.display()))?;
            println!("✓ {}", path.display());
        }
        #[cfg(feature = "pdf")]
        CompileTarget::Pdf => {
            println!("Compiling PDF... (this may take a moment)");
            
            // Prepend a preamble with dummy plugin functions so Typst doesn't error
            let preamble = "#let markplus-mermaid(x) = rect(fill: luma(240), inset: 10pt)[*Mermaid:* #x]\n#let markplus-simby(x) = rect(fill: luma(240), inset: 10pt)[*Simby:* #x]\n\n";
            let full_typst = format!("{}{}", preamble, result.rendered);

            // typwriter requires an input file on disk, so we write the .typ file first.
            let typ_path = dir.join(format!("{stem}.typ"));
            fs::write(&typ_path, full_typst.as_bytes())
                .with_context(|| format!("Cannot write {}", typ_path.display()))?;
            
            let pdf_path = dir.join(format!("{stem}.pdf"));
            
            let params = typwriter::CompileParams {
                input: typ_path,
                output: pdf_path.clone(),
                ..Default::default()
            };
            
            typwriter::compile(&params)
                .map_err(|e| anyhow::anyhow!("Typst compilation failed: {:?}", e))?;
                
            println!("✓ {}", pdf_path.display());
        }
    }

    Ok(())
}

fn cmd_preview(file: &Path, target: RenderTarget) -> Result<()> {
    let raw_md = fs::read_to_string(file)
        .with_context(|| format!("Cannot read {}", file.display()))?;

    let output = match target {
        RenderTarget::Html  => preview_html(&raw_md),
        RenderTarget::Typst => preview_typst(&raw_md),
    };

    print!("{output}");
    Ok(())
}



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

//! mpc — MarkPlus Core CLI
//!
//! Parses a Markdown file and emits the MarkPlus AST as JSON to stdout.
//!
//! Usage:
//!   mpc <file.md>           → compact JSON to stdout
//!   mpc --pretty <file.md>  → pretty-printed JSON to stdout
//!   mpc --help              → print this help

use markplus_core::parse_document;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut pretty = false;
    let mut file: Option<&str> = None;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--help" | "-h" => {
                eprintln!("mpc — MarkPlus Core CLI");
                eprintln!();
                eprintln!("Usage:");
                eprintln!("  mpc <file.md>           emit AST JSON (compact)");
                eprintln!("  mpc --pretty <file.md>  emit AST JSON (pretty)");
                eprintln!("  mpc --help              show this message");
                process::exit(0);
            }
            "--pretty" | "-p" => pretty = true,
            other if !other.starts_with('-') => file = Some(other),
            other => {
                eprintln!("Unknown option: {other}");
                process::exit(1);
            }
        }
    }

    let path = match file {
        Some(p) => p,
        None => {
            eprintln!("mpc: no input file. Run `mpc --help` for usage.");
            process::exit(1);
        }
    };

    let raw = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("mpc: cannot read {path}: {e}");
        process::exit(1);
    });

    let asset = parse_document(&raw).unwrap_or_else(|e| {
        eprintln!("mpc: parse error: {e}");
        process::exit(1);
    });

    let json = if pretty {
        asset.to_json_pretty()
    } else {
        asset.to_json()
    }
    .unwrap_or_else(|e| {
        eprintln!("mpc: serialisation error: {e}");
        process::exit(1);
    });

    println!("{json}");
}

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{error, info};
use std::fs;
use std::path::PathBuf;
use std::process;

mod bytecode;
mod compiler;
mod parser;

use bytecode::BytecodeEmitter;
use compiler::Compiler;
use parser::{HackerScriptParser, Rule};

#[derive(Parser)]
#[command(name = "hs1", about = "HackerScript Compiler", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile .hcs file to .bc bytecode
    Compile {
        /// Input source file
        #[arg(short, long)]
        input: PathBuf,

        /// Output bytecode file (default: same name with .bc)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Emit human-readable bytecode dump
        #[arg(long)]
        dump: bool,

        /// Use Cranelift to generate native object file instead of bytecode
        #[arg(long)]
        native: bool,
    },

    /// Check syntax only (parse without codegen)
    Check {
        /// Input source file
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile {
            input,
            output,
            dump,
            native,
        } => {
            if !input.exists() {
                anyhow::bail!("Input file does not exist: {}", input.display());
            }

            let source = fs::read_to_string(input).context("Failed to read source file")?;

            let pairs = HackerScriptParser::parse(Rule::program, &source)
            .map_err(|e| anyhow::anyhow!("Parse error:\n{}", e))?;

            let mut compiler = Compiler::new();

            for pair in pairs {
                compiler.compile_pair(pair)?;
            }

            let bytecode = compiler.finish();

            let out_path = output
            .clone()
            .unwrap_or_else(|| input.with_extension("bc"));

            if *native {
                // TODO: Cranelift native codegen (placeholder)
                info!("Native codegen requested, but not yet implemented. Falling back to bytecode.");
            }

            bytecode::write_to_file(&bytecode, &out_path)?;

            info!("Compiled {} → {}", input.display(), out_path.display());

            if *dump {
                println!("\nBytecode dump:");
                bytecode::pretty_print(&bytecode);
            }
        }

        Commands::Check { input } => {
            let source = fs::read_to_string(input)?;
            let _ = HackerScriptParser::parse(Rule::program, &source)?;
            println!("Syntax OK: {}", input.display());
        }
    }

    Ok(())
}

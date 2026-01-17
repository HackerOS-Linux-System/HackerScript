use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::info;
use std::fs;
use std::path::PathBuf;

mod bytecode;
mod compiler;
mod parser;

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
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        dump: bool,
        #[arg(long)]
        native: bool,
    },
    /// Check syntax only
    Check {
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile { input, output, dump, native } => {
            if !input.exists() {
                anyhow::bail!("Input file does not exist: {}", input.display());
            }

            let source = fs::read_to_string(input).context("Failed to read source file")?;

            let pairs = <HackerScriptParser as pest::Parser<Rule>>::parse(Rule::program, &source)
                .map_err(|e| anyhow::anyhow!("Parse error:\n{}", e))?;

            let mut compiler = Compiler::new();
            for pair in pairs {
                compiler.compile_pair(pair)?;
            }

            let bytecode = compiler.finish();

            let out_path = output.clone().unwrap_or_else(|| input.with_extension("bc"));

            if *native {
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
            let _ = <HackerScriptParser as pest::Parser<Rule>>::parse(Rule::program, &source)?;
            println!("Syntax OK: {}", input.display());
        }
    }

    Ok(())
}

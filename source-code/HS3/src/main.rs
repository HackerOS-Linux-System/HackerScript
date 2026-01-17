use std::env;
use std::fs;
use std::process;
use pest::Parser;
use pest::error::Error;
#[derive(pest_derive::Parser)]
#[grammar = "hackerscript.pest"]
pub struct HackerScriptParser;
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: hs3 <file.hcs>");
        process::exit(1);
    }
    let file_path = &args[1];
    let code = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{}': {}", file_path, err);
            process::exit(1);
        }
    };
    match HackerScriptParser::parse(Rule::program, &code) {
        Ok(pairs) => {
            // Since no AST is wanted, just print the parse pairs for debugging/inspection
            println!("Parse successful. Pairs:");
            for pair in pairs {
                println!("{:?}", pair);
            }
        }
        Err(err) => {
            // Raw error output; diagnostics handled by HSDF separately
            eprintln!("Parse error:\n{}", format_error(err, &code));
            process::exit(1);
        }
    }
}
// Helper to format error without miette (since that's for HSDF)
fn format_error(err: Error<Rule>, code: &str) -> String {
    let line_col = match err.location {
        pest::error::InputLocation::Pos(pos) => (pos, pos),
        pest::error::InputLocation::Span((start, end)) => (start, end),
    };
    let line_num = code[..line_col.0].matches('\n').count() + 1;
    format!("Error at line {}: {}", line_num, err.variant)
}

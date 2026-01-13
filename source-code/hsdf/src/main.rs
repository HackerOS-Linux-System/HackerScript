use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use clap::Parser;
use miette::{Diagnostic, GraphicalReportHandler, IntoDiagnostic, Report, Result, SourceSpan};
use regex::Regex;
use thiserror::Error;

#[derive(Parser, Debug)]
#[command(name = "hsdf", about = "HackerScript Diagnostic Files - Diagnoses .hcs files with pretty errors")]
struct Args {
    /// Path to the .hcs file to diagnose
    #[arg(required = true)]
    file: PathBuf,
}

#[derive(Debug, Error, Diagnostic)]
enum HcsError {
    #[error("File not found or unable to read: {0}")]
    IoError(#[from] io::Error),
    #[error("Unclosed block comment")]
    #[diagnostic(code(hcs::unclosed_block_comment))]
    UnclosedBlockComment {
        #[source_code]
        src: String,
        #[label("Block comment started here but never closed")]
        span: SourceSpan,
    },
    #[error("Unmatched closing bracket ']' without opening '['")]
    #[diagnostic(code(hcs::unmatched_closing_bracket))]
    UnmatchedClosingBracket {
        #[source_code]
        src: String,
        #[label("This ']' has no matching '['")]
        span: SourceSpan,
    },
    #[error("Unclosed sh block")]
    #[diagnostic(code(hcs::unclosed_sh_block))]
    UnclosedShBlock {
        #[source_code]
        src: String,
        #[label("sh block started here but never closed")]
        span: SourceSpan,
    },
    #[error("Unclosed block (indent level > 0 at EOF)")]
    #[diagnostic(code(hcs::unclosed_block))]
    UnclosedBlock {
        #[source_code]
        src: String,
        #[label("Block opened here but not closed")]
        span: SourceSpan,
    },
    #[error("Invalid syntax: {message}")]
    #[diagnostic(code(hcs::invalid_syntax))]
    InvalidSyntax {
        message: String,
        #[source_code]
        src: String,
        #[label("Invalid syntax here")]
        span: SourceSpan,
    },
    #[error("Multiple errors found")]
    MultipleErrors(Vec<Report>),
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut file = File::open(&args.file).into_diagnostic()?;
    let mut code = String::new();
    file.read_to_string(&mut code).into_diagnostic()?;
    match diagnose_hcs(&code) {
        Ok(_) => {
            println!("No errors found in {}", args.file.display());
            Ok(())
        }
        Err(HcsError::MultipleErrors(errors)) => {
            let handler = GraphicalReportHandler::new();
            for err in errors {
                let mut out = String::new();
                handler.render_report(&mut out, err.as_ref()).into_diagnostic()?;
                print!("{}", out);
            }
            std::process::exit(1);
        }
        Err(err) => {
            let report = Report::new(err);
            let handler = GraphicalReportHandler::new();
            let mut out = String::new();
            handler.render_report(&mut out, report.as_ref()).into_diagnostic()?;
            print!("{}", out);
            std::process::exit(1);
        }
    }
}

fn diagnose_hcs(code: &str) -> std::result::Result<(), HcsError> {
    let lines: Vec<&str> = code.lines().collect();
    let mut errors: Vec<Report> = Vec::new();
    let mut indent_level = 0;
    let mut in_sh_block = false;
    let mut in_block_comment = false;
    let mut block_comment_start: Option<usize> = None;
    let mut sh_block_start: Option<usize> = None;
    let mut last_open_block_pos: Option<usize> = None;

    let rust_re = Regex::new(r"<rust:([\w\-]+)(?:=([\d\.]+))?>").unwrap();
    let c_re = Regex::new(r"<c:(.*)>").unwrap();
    let virus_vira_re = Regex::new(r"import\s+<(virus|vira):([\w\-]+)>").unwrap();
    let core_import_re = Regex::new(r"import\s+<core:([\w\.]+)>").unwrap();
    let require_re = Regex::new(r"require\s+<([\w\./]+)>").unwrap();
    let comment_re = Regex::new(r"@.*").unwrap();
    let block_comment_start_re = Regex::new(r"-/").unwrap();
    let block_comment_end_re = Regex::new(r"-\\").unwrap();

    let mut pos = 0;
    for (_line_num, line) in lines.iter().enumerate() {
        let line_start = pos;
        let mut raw_line = line.trim().to_string();
        // Advance pos
        pos += line.len() + 1; // +1 for newline

        // Handle block comments
        if block_comment_start_re.is_match(&raw_line) {
            if in_block_comment {
                errors.push(Report::new(HcsError::InvalidSyntax {
                    message: "Nested block comment start".to_string(),
                                        src: code.to_string(),
                                        span: (line_start, raw_line.len()).into(),
                }));
            }
            in_block_comment = true;
            block_comment_start = Some(line_start);
            continue;
        }
        if block_comment_end_re.is_match(&raw_line) {
            if !in_block_comment {
                errors.push(Report::new(HcsError::InvalidSyntax {
                    message: "Unmatched block comment end".to_string(),
                                        src: code.to_string(),
                                        span: (line_start, raw_line.len()).into(),
                }));
            }
            in_block_comment = false;
            block_comment_start = None;
            continue;
        }
        if in_block_comment {
            continue;
        }

        // Remove line comments
        raw_line = comment_re.replace(&raw_line, "").trim().to_string();
        if raw_line.is_empty() && !in_sh_block {
            continue;
        }

        // Require
        if require_re.is_match(&raw_line) {
            // Valid
            continue;
        }

        // Special imports: rust, c, virus/vira, core
        if rust_re.is_match(&raw_line) {
            // Valid
            continue;
        }
        if c_re.is_match(&raw_line) {
            // Valid
            continue;
        }
        if virus_vira_re.is_match(&raw_line) {
            // Valid
            continue;
        }
        if core_import_re.is_match(&raw_line) {
            // Valid
            continue;
        }

        // Manual mode
        if raw_line.contains("--- manual ---") {
            // Valid
            continue;
        }

        // Numpy/Tensor syntax
        if raw_line.starts_with("tensor ") || raw_line.starts_with("matrix ") || raw_line.starts_with("vector ") {
            // Check if it looks like assignment or declaration
            if raw_line.contains("=") || raw_line.contains("zeros(") || raw_line.contains("ones(") {
                // Assume valid
                continue;
            } else {
                errors.push(Report::new(HcsError::InvalidSyntax {
                    message: "Invalid tensor/matrix/vector declaration".to_string(),
                                        src: code.to_string(),
                                        span: (line_start, raw_line.len()).into(),
                }));
                continue;
            }
        }

        // SH commands
        if raw_line == "sh [" {
            if in_sh_block {
                errors.push(Report::new(HcsError::InvalidSyntax {
                    message: "Nested sh block".to_string(),
                                        src: code.to_string(),
                                        span: (line_start, raw_line.len()).into(),
                }));
            }
            in_sh_block = true;
            sh_block_start = Some(line_start);
            continue;
        }
        if in_sh_block {
            if raw_line == "]" {
                in_sh_block = false;
                sh_block_start = None;
                continue;
            }
            // Otherwise, sh content, assume valid
            continue;
        }
        if raw_line.starts_with("sh [") && raw_line.ends_with("]") {
            // Single line sh, valid
            continue;
        }

        // Object (class)
        if raw_line.starts_with("object ") {
            // Valid, similar to func
            if raw_line.ends_with("[") {
                indent_level += 1;
                last_open_block_pos = Some(line_start);
            }
            continue;
        }

        // Keywords: func, fast func, log
        if raw_line.starts_with("func ") || raw_line.starts_with("fast func ") || raw_line.starts_with("log ") {
            // Valid
            if raw_line.ends_with("[") {
                indent_level += 1;
                last_open_block_pos = Some(line_start);
            }
            continue;
        }

        // Block handling
        if raw_line.starts_with("] except") || raw_line.starts_with("] else") {
            if indent_level == 0 {
                errors.push(Report::new(HcsError::UnmatchedClosingBracket {
                    src: code.to_string(),
                                        span: (line_start, raw_line.len()).into(),
                }));
            } else {
                indent_level -= 1;
            }
            // For except/else, might open new block if followed by [
            if raw_line.ends_with("[") {
                indent_level += 1;
                last_open_block_pos = Some(line_start);
            }
            continue;
        }
        if raw_line == "]" {
            if indent_level == 0 {
                errors.push(Report::new(HcsError::UnmatchedClosingBracket {
                    src: code.to_string(),
                                        span: (line_start, raw_line.len()).into(),
                }));
            } else {
                indent_level -= 1;
            }
            continue;
        }

        // Opening blocks
        if raw_line.ends_with("[") {
            indent_level += 1;
            last_open_block_pos = Some(line_start);
            continue;
        }

        // Operations like dot
        if raw_line.contains(" dot ") {
            // Assume valid in expressions
            continue;
        }

        // If we reach here and it's not recognized, flag as invalid
        if !raw_line.is_empty() {
            errors.push(Report::new(HcsError::InvalidSyntax {
                message: "Unrecognized syntax".to_string(),
                                    src: code.to_string(),
                                    span: (line_start, raw_line.len()).into(),
            }));
        }
    }

    // Check for unclosed states
    if in_block_comment {
        if let Some(start) = block_comment_start {
            errors.push(Report::new(HcsError::UnclosedBlockComment {
                src: code.to_string(),
                                    span: (start, 2).into(), // Approximate span for "-/"
            }));
        }
    }
    if in_sh_block {
        if let Some(start) = sh_block_start {
            errors.push(Report::new(HcsError::UnclosedShBlock {
                src: code.to_string(),
                                    span: (start, 4).into(), // "sh ["
            }));
        }
    }
    if indent_level > 0 {
        if let Some(start) = last_open_block_pos {
            errors.push(Report::new(HcsError::UnclosedBlock {
                src: code.to_string(),
                                    span: (start, 1).into(), // "["
            }));
        }
    }

    if !errors.is_empty() {
        Err(HcsError::MultipleErrors(errors))
    } else {
        Ok(())
    }
}

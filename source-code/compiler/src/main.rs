use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use miette::{self, Diagnostic, Report, SourceSpan};
use thiserror::Error;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{alpha1, alphanumeric1, char, multispace0, one_of},
    combinator::{map, opt, recognize},
    multi::{many0, separated_list0},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};

use bincode::{serialize, Error as BincodeError};
use byteorder::{BigEndian, WriteBytesExt};

// Define errors with miette support
#[derive(Error, Diagnostic, Debug)]
enum CompilerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Parse error at {span:?}: {message}")]
    #[diagnostic(code(parse_error))]
    Parse {
        message: String,
        #[source_code]
        src: String,
        #[label("Here")]
        span: SourceSpan,
    },

    #[error("Serialization error: {0}")]
    Serialize(#[from] BincodeError),

    #[error("Byteorder error: {0}")]
    Byteorder(#[from] byteorder::Error),
}

// AST nodes
#[derive(Debug, Clone, serde::Serialize)]
enum AstNode {
    Import(String), // Simplified: <source<lib:details>>
    Log(String),    // log"message"
    Func(String, Vec<AstNode>), // func name [ body ]
    Class(String, Vec<AstNode>), // class name [ body ]
    // Add more as needed...
}

// Parser functions using nom

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)
}

fn string_literal(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), take_while(|c| c != '"'), char('"'))(input)
}

fn import_stmt(input: &str) -> IResult<&str, AstNode> {
    map(
        preceded(
            tuple((multispace0, tag("import"), multispace0)),
            delimited(char('<'), take_while(|c| c != '>'), char('>')),
        ),
        |s: &str| AstNode::Import(s.to_string()),
    )(input)
}

fn log_stmt(input: &str) -> IResult<&str, AstNode> {
    map(
        preceded(
            tuple((multispace0, tag("log"))),
            string_literal,
        ),
        |s: &str| AstNode::Log(s.to_string()),
    )(input)
}

fn block(input: &str) -> IResult<&str, Vec<AstNode>> {
    delimited(
        tuple((multispace0, char('['), multispace0)),
        many0(statement),
        tuple((multispace0, char(']'), multispace0)),
    )(input)
}

fn func_def(input: &str) -> IResult<&str, AstNode> {
    map(
        tuple((
            multispace0,
            tag("func"),
            multispace0,
            identifier,
            multispace0,
            block,
        )),
        |(_, _, _, name, _, body)| AstNode::Func(name.to_string(), body),
    )(input)
}

fn class_def(input: &str) -> IResult<&str, AstNode> {
    map(
        tuple((
            multispace0,
            tag("class"),
            multispace0,
            identifier,
            multispace0,
            block,
        )),
        |(_, _, _, name, _, body)| AstNode::Class(name.to_string(), body),
    )(input)
}

fn statement(input: &str) -> IResult<&str, AstNode> {
    alt((import_stmt, log_stmt, func_def, class_def))(input)
}

fn program(input: &str) -> IResult<&str, Vec<AstNode>> {
    many0(statement)(input)
}

// Comment skipping - comments start with @
fn skip_comments(input: &str) -> IResult<&str, ()> {
    map(
        opt(preceded(
            multispace0,
            preceded(tag("@"), take_while(|c| c != '\n')),
        )),
        |_| (),
    )(input)
}

// Main parse function with comment skipping
fn parse_program(source: &str) -> Result<Vec<AstNode>, CompilerError> {
    let mut input = source;
    let mut ast = Vec::new();

    while !input.is_empty() {
        let (rest, _) = skip_comments(input)?;
        input = rest;

        match statement(input) {
            Ok((rest, node)) => {
                ast.push(node);
                input = rest;
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let offset = source.len() - input.len();
                return Err(CompilerError::Parse {
                    message: format!("Parse error: {:?}", e.code),
                    src: source.to_string(),
                    span: (offset, input.len()).into(),
                });
            }
            Err(_) => unreachable!(),
        }
    }

    Ok(ast)
}

// Compile AST to .object (simple bincode serialization for now)
fn compile_to_object(ast: Vec<AstNode>, output_path: &Path) -> Result<(), CompilerError> {
    let serialized = serialize(&ast)?;
    let mut file = File::create(output_path)?;
    // Add a simple header: magic number + version
    file.write_u32::<BigEndian>(0x48434B52)?; // 'HCKR' in ASCII
    file.write_u16::<BigEndian>(1)?; // version 1
    file.write_all(&serialized)?;
    Ok(())
}

fn main() -> miette::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: hs1 <input.hcs> <output.object>");
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    let mut source = String::new();
    File::open(input_path)?.read_to_string(&mut source)?;

    let ast = parse_program(&source).map_err(Report::from)?;
    compile_to_object(ast, output_path).map_err(Report::from)?;

    Ok(())
}

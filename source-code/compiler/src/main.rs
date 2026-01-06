use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use bincode::{serialize, Error as BincodeError};
use byteorder::{BigEndian, WriteBytesExt};
use miette::{self, Diagnostic, NamedSource, SourceSpan};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric1, multispace0},
    combinator::{map, opt, recognize, all_consuming},
    multi::{many0, separated_list0},
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};
use nom::error::Error as NomError;
use thiserror::Error;

// Typy
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum Type {
    String,
    Number,
    Bool,
    Null,
    Array(Box<Type>),
    Object, // Dla class/self
    Any, // Fallback
}

// AST: Expr
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum Expr {
    Literal(Lit),
    Ident(String),
    SelfRef,
    Dot(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Array(Vec<Expr>),
    Interp(Vec<InterpPart>), // Dla interpolated strings
    Index(Box<Expr>, Box<Expr>),
    New(String, Vec<Expr>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum InterpPart {
    Text(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum Lit {
    String(String),
    Number(f64),
    Null,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum BinOp {
    Eq, Ne, Gt, Lt, Ge, Le, Add, Sub, Mul, Div, And, Or,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum UnaryOp {
    Not, Neg,
}

// AST: LValue dla assignments
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum LValue {
    Ident(String),
    Dot(Box<Expr>, String),
    // Index jeśli potrzeba w przyszłości
}

// AST: Stmt
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum Stmt {
    Log(Expr),
    Func(String, Vec<String>, Vec<Stmt>),
    Class(String, Vec<Stmt>),
    Import(String),
    Comment(String),
    Assign(LValue, Expr),
    If(Expr, Vec<Stmt>, Vec<(Expr, Vec<Stmt>)>, Option<Vec<Stmt>>),
    For(String, Expr, Vec<Stmt>),
    Return(Option<Expr>),
    ExprStmt(Expr),
    MemoryMode(MemoryMode),
}

// Program
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Program {
    stmts: Vec<Stmt>,
    memory_mode: MemoryMode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
enum MemoryMode {
    Manual,
    Auto,
}

// Błędy
#[derive(Error, Debug, Diagnostic)]
enum CompilerError {
    #[error("Błąd parsowania: {0}")]
    #[diagnostic(code(hs1::parse_error))]
    ParseError(String, #[source_code] NamedSource<String>, #[label("tutaj")] SourceSpan),
    #[error("Błąd typów: {0}")]
    #[diagnostic(code(hs1::type_error))]
    TypeError(String),
    #[error("Błąd IO: {0}")]
    Io(#[from] io::Error),
    #[error("Błąd serializacji: {0}")]
    Bincode(#[from] BincodeError),
    #[error("Nieznany tryb pamięci: {0}")]
    UnknownMemoryMode(String),
}

// Parsery
fn parse_comment(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    map(preceded(tag("@"), take_while(|c| c != '\n')), |s: &str| Stmt::Comment(s.trim().to_string()))(input)
}

fn parse_identifier(input: &str) -> IResult<&str, String, NomError<&str>> {
    map(recognize(pair(
        alt((alphanumeric1, tag("_"))),
        many0(alt((alphanumeric1, tag("_"), tag("-")))),
    )), |s: &str| s.to_string())(input)
}

fn parse_literal(input: &str) -> IResult<&str, Lit, NomError<&str>> {
    alt((
        map(delimited(tag("\""), take_while(|c| c != '"'), tag("\"")), |s: &str| Lit::String(s.to_string())),
        map(take_while(|c: char| c.is_digit(10) || c == '.'), |s: &str| Lit::Number(s.parse().unwrap_or(0.0))),
        map(tag("null"), |_| Lit::Null),
    ))(input)
}

fn parse_interp_string(input: &str) -> IResult<&str, Expr, NomError<&str>> {
    let mut parts = Vec::new();
    let (mut i, _) = tag("\"")(input)?;
    loop {
        if i.starts_with("\"") {
            i = &i[1..];
            break;
        } else if i.starts_with("{") {
            let (rem, expr) = delimited(tag("{"), parse_expr, tag("}"))(i)?;
            parts.push(InterpPart::Expr(Box::new(expr)));
            i = rem;
        } else {
            let (rem, text) = take_while(|c| c != '"' && c != '{'})(i)?;
            if !text.is_empty() {
                parts.push(InterpPart::Text(text.to_string()));
            }
            i = rem;
        }
    }
    Ok((i, Expr::Interp(parts)))
}

fn parse_log(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    map(preceded(tag("log"), parse_interp_string), Stmt::Log)(input)
}

fn parse_import(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    map(delimited(tag("import <"), take_while(|c| c != '>'), tag(">")), |src: &str| Stmt::Import(src.to_string()))(input)
}

fn parse_block(input: &str) -> IResult<&str, Vec<Stmt>, NomError<&str>> {
    delimited(
        preceded(multispace0, tag("[")),
        many0(parse_stmt),
        preceded(multispace0, tag("]")),
    )(input)
}

fn parse_params(input: &str) -> IResult<&str, Vec<String>, NomError<&str>> {
    delimited(
        preceded(multispace0, tag("(")),
        separated_list0(preceded(multispace0, tag(",")), preceded(multispace0, parse_identifier)),
        preceded(multispace0, tag(")")),
    )(input)
}

fn parse_func(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    let (input, name) = preceded(tuple((tag("func"), multispace0)), parse_identifier)(input)?;
    let (input, params) = opt(preceded(multispace0, parse_params))(input)?;
    let (input, body) = preceded(multispace0, parse_block)(input)?;
    Ok((input, Stmt::Func(name, params.unwrap_or_default(), body)))
}

fn parse_class(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    let (input, name) = preceded(tuple((tag("class"), multispace0)), parse_identifier)(input)?;
    let (input, body) = preceded(multispace0, parse_block)(input)?;
    Ok((input, Stmt::Class(name, body)))
}

fn parse_memory_mode(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    alt((
        map(tag("--- manual ---"), |_| Stmt::MemoryMode(MemoryMode::Manual)),
        map(tag("--- auto ---"), |_| Stmt::MemoryMode(MemoryMode::Auto)),
        map(tag("--- automatic ---"), |_| Stmt::MemoryMode(MemoryMode::Auto)),
    ))(input)
}

fn parse_bin_op(input: &str) -> IResult<&str, BinOp, NomError<&str>> {
    preceded(multispace0, alt((
        map(tag("=="), |_| BinOp::Eq),
        map(tag("!="), |_| BinOp::Ne),
        map(tag(">"), |_| BinOp::Gt),
        map(tag("<"), |_| BinOp::Lt),
        map(tag(">="), |_| BinOp::Ge),
        map(tag("<="), |_| BinOp::Le),
        map(tag("+"), |_| BinOp::Add),
        map(tag("-"), |_| BinOp::Sub),
        map(tag("*"), |_| BinOp::Mul),
        map(tag("/"), |_| BinOp::Div),
        map(tag("&&"), |_| BinOp::And),
        map(tag("||"), |_| BinOp::Or),
    )))(input)
}

fn parse_expr(input: &str) -> IResult<&str, Expr, NomError<&str>> {
    let (mut input, mut left) = parse_term(input)?;
    while let Ok((rem, op)) = parse_bin_op(input) {
        let (rem2, right) = parse_term(rem)?;
        left = Expr::Binary(Box::new(left), op, Box::new(right));
        input = rem2;
    }
    Ok((input, left))
}

fn parse_unary(input: &str) -> IResult<&str, Expr, NomError<&str>> {
    let (input, op) = opt(preceded(multispace0, alt((
        map(tag("!"), |_| UnaryOp::Not),
        map(tag("-"), |_| UnaryOp::Neg),
    ))))(input)?;
    let (input, primary) = parse_primary(input)?;
    if let Some(op) = op {
        Ok((input, Expr::Unary(op, Box::new(primary))))
    } else {
        Ok((input, primary))
    }
}

fn parse_term(input: &str) -> IResult<&str, Expr, NomError<&str>> {
    let (mut input, mut expr) = parse_unary(input)?;
    loop {
        if let Ok((rem, field)) = preceded(preceded(multispace0, tag(".")), parse_identifier)(input) {
            expr = Expr::Dot(Box::new(expr), field);
            input = rem;
            continue;
        }
        if let Ok((rem, args)) = delimited(preceded(multispace0, tag("(")), separated_list0(preceded(multispace0, tag(",")), preceded(multispace0, parse_expr)), preceded(multispace0, tag(")")))(input) {
            expr = Expr::Call(Box::new(expr), args);
            input = rem;
            continue;
        }
        if let Ok((rem, idx)) = delimited(preceded(multispace0, tag("[")), preceded(multispace0, parse_expr), preceded(multispace0, tag("]")))(input) {
            expr = Expr::Index(Box::new(expr), Box::new(idx));
            input = rem;
            continue;
        }
        break;
    }
    Ok((input, expr))
}

fn parse_primary(input: &str) -> IResult<&str, Expr, NomError<&str>> {
    preceded(multispace0, alt((
        map(parse_literal, Expr::Literal),
        map(tag("self"), |_| Expr::SelfRef),
        map(parse_identifier, Expr::Ident),
        delimited(tag("("), parse_expr, preceded(multispace0, tag(")"))),
        map(delimited(tag("["), separated_list0(preceded(multispace0, tag(",")), parse_expr), tag("]")), Expr::Array),
        map(tuple((tag("new"), multispace0, parse_identifier, opt(delimited(preceded(multispace0, tag("(")), separated_list0(preceded(multispace0, tag(",")), preceded(multispace0, parse_expr)), preceded(multispace0, tag(")")))))), |(_, _, name, opt_args)| Expr::New(name, opt_args.unwrap_or(vec![]))),
    )))(input)
}

fn parse_lvalue(input: &str) -> IResult<&str, LValue, NomError<&str>> {
    let (input, base) = parse_term(input)?;
    match base {
        Expr::Ident(id) => Ok((input, LValue::Ident(id))),
        Expr::Dot(base, field) => Ok((input, LValue::Dot(base, field))),
        _ => Err(nom::Err::Error(NomError::new(input, nom::error::ErrorKind::Many0))),
    }
}

fn parse_assign(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    let (input, lval) = parse_lvalue(input)?;
    let (input, _) = preceded(multispace0, tag("="))(input)?;
    let (input, rval) = preceded(multispace0, parse_expr)(input)?;
    Ok((input, Stmt::Assign(lval, rval)))
}

fn parse_if(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    let (input, _) = preceded(multispace0, tag("if"))(input)?;
    let (input, cond) = preceded(multispace0, parse_expr)(input)?;
    let (input, body) = preceded(multispace0, parse_block)(input)?;
    let mut elifs = Vec::new();
    let mut input = input;
    loop {
        match preceded(multispace0, tag("elif"))(input) {
            Ok((rem, _)) => {
                let (rem2, cond_el) = preceded(multispace0, parse_expr)(rem)?;
                let (rem3, body_el) = preceded(multispace0, parse_block)(rem2)?;
                elifs.push((cond_el, body_el));
                input = rem3;
            }
            Err(_) => break,
        }
    }
    let (input, else_body) = opt(preceded(multispace0, preceded(tag("else"), preceded(multispace0, parse_block))))(input)?;
    Ok((input, Stmt::If(cond, body, elifs, else_body)))
}

fn parse_for(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    let (input, _) = preceded(multispace0, tag("for"))(input)?;
    let (input, var) = preceded(multispace0, parse_identifier)(input)?;
    let (input, _) = preceded(multispace0, tag("in"))(input)?;
    let (input, iter) = preceded(multispace0, parse_expr)(input)?;
    let (input, body) = preceded(multispace0, parse_block)(input)?;
    Ok((input, Stmt::For(var, iter, body)))
}

fn parse_return(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    let (input, _) = preceded(multispace0, tag("return"))(input)?;
    let (input, expr) = opt(preceded(multispace0, parse_expr))(input)?;
    Ok((input, Stmt::Return(expr)))
}

fn parse_stmt(input: &str) -> IResult<&str, Stmt, NomError<&str>> {
    preceded(
        multispace0,
        alt((
            parse_comment,
            parse_log,
            parse_import,
            parse_func,
            parse_class,
            parse_memory_mode,
            parse_assign,
            parse_if,
            parse_for,
            parse_return,
            map(parse_expr, Stmt::ExprStmt),
        )),
    )(input)
}

fn parse_program(input: &str) -> IResult<&str, Program, NomError<&str>> {
    let (input, mem_mode_stmt) = opt(preceded(multispace0, parse_memory_mode))(input)?;
    let (input, stmts) = many0(parse_stmt)(input)?;
    let memory_mode = if let Some(Stmt::MemoryMode(mode)) = mem_mode_stmt {
        mode
    } else {
        MemoryMode::Manual
    };
    Ok((input, Program { stmts, memory_mode }))
}

// Type inference/checker
fn infer_type(expr: &Expr) -> Type {
    match expr {
        Expr::Literal(lit) => match lit {
            Lit::String(_) => Type::String,
            Lit::Number(_) => Type::Number,
            Lit::Null => Type::Null,
        },
        Expr::Ident(_) => Type::Any,
        Expr::SelfRef => Type::Object,
        Expr::Dot(_, _) => Type::Any,
        Expr::Call(_, _) => Type::Any,
        Expr::Binary(_, op, _) => match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => Type::Number,
            BinOp::Eq | BinOp::Ne | BinOp::Gt | BinOp::Lt | BinOp::Ge | BinOp::Le => Type::Bool,
            _ => Type::Any,
        },
        Expr::Unary(op, _) => match op {
            UnaryOp::Not => Type::Bool,
            UnaryOp::Neg => Type::Number,
        },
        Expr::Array(elems) => Type::Array(Box::new(if elems.is_empty() { Type::Any } else { infer_type(&elems[0]) })),
        Expr::Interp(_) => Type::String,
        Expr::Index(base, _) => if let Type::Array(t) = infer_type(base) { *t } else { Type::Any },
        Expr::New(_, _) => Type::Object,
    }
}

fn check_types(program: &Program) -> Result<(), CompilerError> {
    if program.memory_mode == MemoryMode::Auto {
        return Err(CompilerError::TypeError("Auto memory management not implemented".to_string()));
    }
    // Dodatkowe sprawdzanie: dla if cond Bool, dla index base Array itp.
    // Dla demo: OK
    Ok(())
}

// Kompilacja do bytecode
fn compile_to_bytecode(program: &Program) -> Result<Vec<u8>, CompilerError> {
    let mut bytecode = Vec::new();
    let serialized_ast = serialize(program)?;
    bytecode.write_u32::<BigEndian>(serialized_ast.len() as u32)?;
    bytecode.extend(serialized_ast);
    // Przykładowe opcodes
    for stmt in &program.stmts {
        match stmt {
            Stmt::Log(_) => bytecode.push(0x01),
            Stmt::Assign(_, _) => bytecode.push(0x02),
            Stmt::If(_, _, _, _) => bytecode.push(0x04),
            Stmt::For(_, _, _) => bytecode.push(0x05),
            Stmt::Return(_) => bytecode.push(0x06),
            Stmt::ExprStmt(expr) => match expr {
                Expr::New(_, _) => bytecode.push(0x07),
                Expr::Index(_, _) => bytecode.push(0x08),
                _ => {},
            },
            _ => {},
        }
    }
    Ok(bytecode)
}

fn main() -> miette::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Użycie: hs1 <input.hcs> <output.object>");
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];
    let mut file = File::open(input_path).map_err(|e| CompilerError::Io(e))?;
    let mut source = String::new();
    file.read_to_string(&mut source).map_err(|e| CompilerError::Io(e))?;
    let parse_result = all_consuming(parse_program)(&source);
    let program = match parse_result {
        Ok((_, program)) => program,
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            let offset = source.len() - e.input.len();
            let span = SourceSpan::new(offset.into(), e.input.len());
            return Err(CompilerError::ParseError(
                format!("Błąd parsowania: {:?}", e.code),
                NamedSource::new(input_path, source),
                span,
            ).into());
        }
        Err(_) => unreachable!(),
    };
    check_types(&program)?;
    let bytecode = compile_to_bytecode(&program)?;
    let mut output_file = File::create(output_path).map_err(|e| CompilerError::Io(e))?;
    output_file.write_all(&bytecode).map_err(|e| CompilerError::Io(e))?;
    println!("Skompilowano {} do {}", input_path, output_path);
    Ok(())
}

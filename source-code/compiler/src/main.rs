use anyhow::{bail, anyhow, Result};
use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context as LlvmContext;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use inkwell::OptimizationLevel;
use std::collections::HashMap;
use std::env;
use std::path::Path;
// Token kinds
#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Invalid,
    Eof,
    Identifier(String),
    Number(String),
    String(String),
    Log,
    Import,
    Class,
    Func,
    OpenBracket, // [
    CloseBracket, // ]
    Manual, // --- manual ---
    Auto, // --- auto ---
    Colon,
    Semicolon,
    Assign,
    IntType,
    StringType,
    // Add more
}
// Token
#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}
// AST Node kinds
#[derive(Debug, Clone)]
enum AstNodeKind {
    Program,
    Import,
    ClassDef,
    FuncDef,
    LogStmt,
    Block,
    VarDecl,
    AssignStmt,
    Expr,
    // Add more
}
// AST Node
#[derive(Debug, Clone)]
struct AstNode {
    kind: AstNodeKind,
    children: Vec<AstNode>,
    token: Token,
    typ: Option<String>, // For semantic: "i32", "string", etc.
}
// Memory Mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum MemoryMode {
    Arc,
    Manual,
}
// Parser context
struct Parser<'a> {
    source: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    token_idx: usize,
    ast: Option<AstNode>,
    memory_mode: MemoryMode,
    symbols: HashMap<String, String>, // name -> type
}
impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
            token_idx: 0,
            ast: None,
            memory_mode: MemoryMode::Arc,
            symbols: HashMap::new(),
        }
    }
    fn next_char(&mut self) -> Option<char> {
        if self.pos >= self.chars.len() {
            return None;
        }
        let c = self.chars[self.pos];
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }
    fn peek_char(&self) -> Option<char> {
        if self.pos >= self.chars.len() {
            return None;
        }
        Some(self.chars[self.pos])
    }
    fn lex(&mut self) -> Result<()> {
        while let Some(c) = self.next_char() {
            match c {
                ' ' | '\t' | '\r' => continue,
                '\n' => continue,
                '@' => { // Comment
                    while let Some(nc) = self.peek_char() {
                        if nc == '\n' { break; }
                        self.next_char();
                    }
                    // Skip comment
                }
                '[' => self.tokens.push(Token { kind: TokenKind::OpenBracket, line: self.line, column: self.column }),
                ']' => self.tokens.push(Token { kind: TokenKind::CloseBracket, line: self.line, column: self.column }),
                '"' => { // String
                    let mut text = String::new();
                    while let Some(nc) = self.next_char() {
                        if nc == '"' { break; }
                        text.push(nc);
                    }
                    self.tokens.push(Token { kind: TokenKind::String(text), line: self.line, column: self.column });
                }
                ':' => self.tokens.push(Token { kind: TokenKind::Colon, line: self.line, column: self.column }),
                ';' => self.tokens.push(Token { kind: TokenKind::Semicolon, line: self.line, column: self.column }),
                '=' => self.tokens.push(Token { kind: TokenKind::Assign, line: self.line, column: self.column }),
                _ if c.is_alphabetic() || c == '_' => {
                    let mut text = String::new();
                    text.push(c);
                    while let Some(nc) = self.peek_char() {
                        if !nc.is_alphanumeric() && nc != '_' { break; }
                        self.next_char();
                        text.push(nc);
                    }
                    let kind = match text.as_str() {
                        "import" => TokenKind::Import,
                        "class" => TokenKind::Class,
                        "func" => TokenKind::Func,
                        "log" => TokenKind::Log,
                        "--- manual ---" => TokenKind::Manual,
                        "--- auto ---" | "--- automatic ---" => TokenKind::Auto,
                        "int" => TokenKind::IntType,
                        "string" => TokenKind::StringType,
                        _ => TokenKind::Identifier(text),
                    };
                    self.tokens.push(Token { kind, line: self.line, column: self.column });
                }
                _ if c.is_digit(10) => {
                    let mut text = String::new();
                    text.push(c);
                    while let Some(nc) = self.peek_char() {
                        if !nc.is_digit(10) && nc != '.' { break; }
                        self.next_char();
                        text.push(nc);
                    }
                    self.tokens.push(Token { kind: TokenKind::Number(text), line: self.line, column: self.column });
                }
                _ => bail!("Unexpected char '{}'", c),
            }
        }
        self.tokens.push(Token { kind: TokenKind::Eof, line: self.line, column: self.column });
        Ok(())
    }
    fn next_token(&mut self) -> &Token {
        let tok = &self.tokens[self.token_idx];
        self.token_idx += 1;
        tok
    }
    fn peek_token(&self) -> &Token {
        &self.tokens[self.token_idx]
    }
    fn parse_program(&mut self) -> Result<()> {
        let mut program = AstNode {
            kind: AstNodeKind::Program,
            children: Vec::new(),
            token: Token { kind: TokenKind::Invalid, line: 0, column: 0 },
            typ: None,
        };
        while self.peek_token().kind != TokenKind::Eof {
            match self.peek_token().kind {
                TokenKind::Manual => {
                    self.memory_mode = MemoryMode::Manual;
                    self.next_token();
                }
                TokenKind::Auto => {
                    self.memory_mode = MemoryMode::Arc;
                    self.next_token();
                }
                _ => {
                    let stmt = self.parse_statement()?;
                    program.children.push(stmt);
                }
            }
        }
        self.ast = Some(program);
        Ok(())
    }
    fn parse_statement(&mut self) -> Result<AstNode> {
        let tok = self.next_token().clone();
        match tok.kind {
            TokenKind::Import => Ok(AstNode {
                kind: AstNodeKind::Import,
                children: Vec::new(),
                token: tok,
                typ: None,
            }),
            TokenKind::Class => {
                let name_tok = self.next_token().clone();
                if !matches!(name_tok.kind, TokenKind::Identifier(_)) {
                    bail!("Expected class name");
                }
                if self.next_token().kind != TokenKind::OpenBracket {
                    bail!("Expected [");
                }
                let body = self.parse_block()?;
                Ok(AstNode {
                    kind: AstNodeKind::ClassDef,
                    children: vec![body],
                    token: name_tok,
                    typ: None,
                })
            }
            TokenKind::Func => {
                let name_tok = self.next_token().clone();
                if !matches!(name_tok.kind, TokenKind::Identifier(_)) {
                    bail!("Expected func name");
                }
                if self.next_token().kind != TokenKind::OpenBracket {
                    bail!("Expected [");
                }
                let body = self.parse_block()?;
                Ok(AstNode {
                    kind: AstNodeKind::FuncDef,
                    children: vec![body],
                    token: name_tok,
                    typ: None,
                })
            }
            TokenKind::Log => {
                let str_tok = self.next_token().clone();
                if !matches!(str_tok.kind, TokenKind::String(_)) {
                    bail!("Expected string after log");
                }
                Ok(AstNode {
                    kind: AstNodeKind::LogStmt,
                    children: Vec::new(),
                    token: str_tok,
                    typ: None,
                })
            }
            TokenKind::IntType | TokenKind::StringType => {
                let typ = if matches!(tok.kind, TokenKind::IntType) { "i32".to_string() } else { "string".to_string() };
                let name_tok = self.next_token().clone();
                if !matches!(name_tok.kind, TokenKind::Identifier(_)) {
                    bail!("Expected identifier");
                }
                if self.next_token().kind != TokenKind::Assign {
                    bail!("Expected =");
                }
                let expr = self.parse_expression()?;
                if self.next_token().kind != TokenKind::Semicolon {
                    bail!("Expected ;");
                }
                Ok(AstNode {
                    kind: AstNodeKind::VarDecl,
                    children: vec![expr],
                    token: name_tok,
                    typ: Some(typ),
                })
            }
            TokenKind::Identifier(_) => {
                if self.next_token().kind != TokenKind::Assign {
                    bail!("Expected =");
                }
                let expr = self.parse_expression()?;
                if self.next_token().kind != TokenKind::Semicolon {
                    bail!("Expected ;");
                }
                Ok(AstNode {
                    kind: AstNodeKind::AssignStmt,
                    children: vec![expr],
                    token: tok,
                    typ: None,
                })
            }
            _ => bail!("Unexpected token {:?}", tok.kind),
        }
    }
    fn parse_block(&mut self) -> Result<AstNode> {
        let mut block = AstNode {
            kind: AstNodeKind::Block,
            children: Vec::new(),
            token: Token { kind: TokenKind::Invalid, line: 0, column: 0 },
            typ: None,
        };
        while !matches!(self.peek_token().kind, TokenKind::CloseBracket | TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            block.children.push(stmt);
        }
        if self.peek_token().kind == TokenKind::CloseBracket {
            self.next_token();
        } else {
            bail!("Unclosed block");
        }
        Ok(block)
    }
    fn parse_expression(&mut self) -> Result<AstNode> {
        let tok = self.next_token().clone();
        let typ = match &tok.kind {
            TokenKind::Number(_) => Some("i32".to_string()),
            TokenKind::String(_) => Some("string".to_string()),
            TokenKind::Identifier(_) => None, // Resolve later
            _ => bail!("Unexpected in expr"),
        };
        Ok(AstNode {
            kind: AstNodeKind::Expr,
            children: Vec::new(),
            token: tok,
            typ,
        })
    }
    // Semantic analysis
    fn semantic_check(&mut self, node: &mut AstNode) -> Result<()> {
        match node.kind {
            AstNodeKind::Program | AstNodeKind::Block => {
                for child in &mut node.children {
                    self.semantic_check(child)?;
                }
            }
            AstNodeKind::VarDecl => {
                let name = if let TokenKind::Identifier(n) = &node.token.kind { n.clone() } else { unreachable!() };
                if self.symbols.contains_key(&name) {
                    bail!("Redefinition of {}", name);
                }
                let decl_type = node.typ.clone().unwrap();
                self.symbols.insert(name, decl_type.clone());
                if !node.children.is_empty() {
                    self.semantic_check(&mut node.children[0])?;
                    let expr_type = node.children[0].typ.clone().unwrap();
                    if expr_type != decl_type {
                        bail!("Type mismatch in decl");
                    }
                }
            }
            AstNodeKind::AssignStmt => {
                let name = if let TokenKind::Identifier(n) = &node.token.kind { n.clone() } else { unreachable!() };
                let var_type = self.symbols.get(&name).cloned().ok_or_else(|| anyhow!("Undefined var"))?;
                if !node.children.is_empty() {
                    self.semantic_check(&mut node.children[0])?;
                    let expr_type = node.children[0].typ.clone().unwrap();
                    if expr_type != var_type {
                        bail!("Type mismatch in assign");
                    }
                }
            }
            AstNodeKind::Expr => {
                if let TokenKind::Identifier(n) = &node.token.kind {
                    let var_type = self.symbols.get(n).cloned().ok_or_else(|| anyhow!("Undefined ident"))?;
                    node.typ = Some(var_type);
                }
            }
            AstNodeKind::LogStmt => {
                if !matches!(node.token.kind, TokenKind::String(_)) {
                    bail!("Log expects string");
                }
            }
            // Add for func, class
            _ => {}
        }
        Ok(())
    }
}
// Codegen context
struct CodeGen<'ctx> {
    context: &'ctx LlvmContext,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
    printf: FunctionValue<'ctx>,
}
impl<'ctx> CodeGen<'ctx> {
    fn new(context: &'ctx LlvmContext) -> Self {
        let module = context.create_module("hackerscript");
        let builder = context.create_builder();
        // Declare printf
        let i32_type = context.i32_type();
        let ptr_type = context.ptr_type(AddressSpace::default());
        let printf_type = i32_type.fn_type(&[ptr_type.into()], true);
        let printf = module.add_function("printf", printf_type, None);
        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            printf,
        }
    }
    fn codegen(&mut self, ast: &AstNode, _memory_mode: MemoryMode) -> Result<()> {
        // Create main function
        let i32_type = self.context.i32_type();
        let main_type = i32_type.fn_type(&[], false);
        let main_func = self.module.add_function("main", main_type, None);
        let entry_bb = self.context.append_basic_block(main_func, "entry");
        self.builder.position_at_end(entry_bb);
        self.codegen_node(ast)?;
        let zero = i32_type.const_int(0, false);
        self.builder.build_return(Some(&zero))?;
        Ok(())
    }
    fn codegen_node(&mut self, node: &AstNode) -> Result<()> {
        match node.kind {
            AstNodeKind::Program | AstNodeKind::Block => {
                for child in &node.children {
                    self.codegen_node(child)?;
                }
            }
            AstNodeKind::VarDecl => {
                let name = if let TokenKind::Identifier(n) = &node.token.kind { n } else { unreachable!() };
                let typ = node.typ.as_ref().unwrap();
                let ty: BasicTypeEnum = if typ == "i32" {
                    self.context.i32_type().into()
                } else { // string as ptr
                    self.context.ptr_type(AddressSpace::default()).into()
                };
                let alloca = self.builder.build_alloca(ty, name)?;
                self.variables.insert(name.clone(), (alloca, ty));
                if !node.children.is_empty() {
                    let value = self.codegen_expr(&node.children[0])?;
                    self.builder.build_store(alloca, value)?;
                }
            }
            AstNodeKind::AssignStmt => {
                let name = if let TokenKind::Identifier(n) = &node.token.kind { n } else { unreachable!() };
                let (alloca, _ty) = *self.variables.get(name).unwrap();
                let value = self.codegen_expr(&node.children[0])?;
                self.builder.build_store(alloca, value)?;
            }
            AstNodeKind::LogStmt => {
                let msg = if let TokenKind::String(s) = &node.token.kind { format!("{}\n\0", s) } else { unreachable!() };
                let i8_type = self.context.i8_type();
                let array_type = i8_type.array_type(msg.len() as u32);
                let global = self.module.add_global(array_type, None, "str");
                global.set_initializer(&i8_type.const_array(&msg.bytes().map(|b| i8_type.const_int(b as u64, false)).collect::<Vec<_>>()));
                let zero = self.context.i32_type().const_int(0, false);
                let gep = unsafe { self.builder.build_gep(array_type, global.as_pointer_value(), &[zero, zero], "gep")? };
                self.builder.build_call(self.printf, &[gep.into()], "call")?;
            }
            AstNodeKind::Expr => {
                // Handled in codegen_expr
            }
            _ => {} // Skip import, class, func for simplicity
        }
        Ok(())
    }
    fn codegen_expr(&mut self, node: &AstNode) -> Result<BasicValueEnum<'ctx>> {
        match &node.token.kind {
            TokenKind::Number(n) => {
                let i32_type = self.context.i32_type();
                Ok(i32_type.const_int(n.parse::<u64>().unwrap(), false).into())
            }
            TokenKind::String(s) => {
                let msg = format!("{}\0", s);
                let i8_type = self.context.i8_type();
                let array_type = i8_type.array_type(msg.len() as u32);
                let global = self.module.add_global(array_type, None, "str");
                global.set_initializer(&i8_type.const_array(&msg.bytes().map(|b| i8_type.const_int(b as u64, false)).collect::<Vec<_>>()));
                let zero = self.context.i32_type().const_int(0, false);
                let gep = unsafe { self.builder.build_gep(array_type, global.as_pointer_value(), &[zero, zero], "gep")? };
                Ok(gep.into())
            }
            TokenKind::Identifier(n) => {
                let (alloca, ty) = *self.variables.get(n).unwrap();
                Ok(self.builder.build_load(ty, alloca, "load")?)
            }
            _ => bail!("Invalid expr"),
        }
    }
    fn compile_to_object(&self, path: &Path) -> Result<()> {
        let target_triple = TargetTriple::create("x86_64-unknown-linux-gnu");
        Target::initialize_all(&InitializationConfig::default());
        let target = Target::from_triple(&target_triple).map_err(|e| anyhow!("Failed to create target: {}", e))?;
        let target_machine = target
        .create_target_machine(
            &target_triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::PIC,
            CodeModel::Default,
        ).ok_or_else(|| anyhow!("Failed to create target machine"))?;
        target_machine
        .write_to_file(&self.module, FileType::Object, path)
        .map_err(|e| anyhow!("Failed to write object file: {}", e))
    }
    // For ELF, would need to link, but for simplicity, assume object is fine, or use linker externally.
}
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        bail!("Usage: HackerScript-Compiler <input.hcs> -o <output>");
    }
    let input_path = &args[1];
    let source = std::fs::read_to_string(input_path)?;
    let mut parser = Parser::new(&source);
    parser.lex()?;
    parser.parse_program()?;
    let mut ast = parser.ast.take().unwrap();
    parser.semantic_check(&mut ast)?;
    let llvm_context = LlvmContext::create();
    let mut codegen = CodeGen::new(&llvm_context);
    codegen.codegen(&ast, parser.memory_mode)?;
    // For simplicity, output to object file
    let output_path = Path::new("output.o");
    codegen.compile_to_object(output_path)?;
    println!("Compiled to {}", output_path.display());
    Ok(())
}

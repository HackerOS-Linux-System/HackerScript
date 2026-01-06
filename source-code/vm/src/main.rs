use std::env;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

use bincode::{deserialize, Error as BincodeError};
use byteorder::{BigEndian, ReadBytesExt};
use miette::{self, Diagnostic};
use thiserror::Error;

// Ponowne użycie struktur z HS1 (AST)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum AstNode {
    Log(String),
    Func(String, Vec<AstNode>),
    Class(String, Vec<AstNode>),
    Import(String),
    Block(Vec<AstNode>),
    Comment(String),
    // Dodaj więcej
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Program {
    nodes: Vec<AstNode>,
    memory_mode: MemoryMode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
enum MemoryMode {
    Manual,
    Auto,
}

// Błędy
#[derive(Error, Debug, Diagnostic)]
enum VmError {
    #[error("Błąd IO: {0}")]
    Io(#[from] io::Error),

    #[error("Błąd deserializacji: {0}")]
    Bincode(#[from] BincodeError),

    #[error("Błąd odczytu bytecode: {0}")]
    Byteorder(String),

    #[error("Nieznany opcode: {0}")]
    UnknownOpcode(u8),

    #[error("Błąd wykonania: {0}")]
    ExecutionError(String),

    #[error("Nieobsługiwany tryb pamięci: Auto nieimplementowane")]
    UnsupportedMemoryMode,
}

// Prosta VM
struct Vm {
    program: Program,
    bytecode: Vec<u8>,
    pc: usize,  // Program counter
    memory: Vec<u8>,  // Symulowana pamięć (manual management)
}

impl Vm {
    fn new(program: Program, bytecode: Vec<u8>) -> Self {
        Vm {
            program,
            bytecode,
            pc: 0,
            memory: vec![0; 1024 * 1024],  // 1MB pamięci
        }
    }

    fn run(&mut self) -> Result<(), VmError> {
        if self.program.memory_mode == MemoryMode::Auto {
            return Err(VmError::UnsupportedMemoryMode);
        }

        while self.pc < self.bytecode.len() {
            let opcode = self.bytecode[self.pc];
            self.pc += 1;

            match opcode {
                0x01 => {  // Log
                    let len = self.read_u32()?;
                    let msg_bytes = &self.bytecode[self.pc..self.pc + len as usize];
                    let msg = String::from_utf8_lossy(msg_bytes).to_string();
                    println!("{}", msg);  // Wykonaj log
                    self.pc += len as usize;
                }
                _ => return Err(VmError::UnknownOpcode(opcode)),
            }
        }
        Ok(())
    }

    fn read_u32(&mut self) -> Result<u32, VmError> {
        if self.pc + 4 > self.bytecode.len() {
            return Err(VmError::Byteorder("Niewystarczająco bajtów na u32".to_string()));
        }
        let mut slice = [0u8; 4];
        slice.copy_from_slice(&self.bytecode[self.pc..self.pc + 4]);
        self.pc += 4;
        Ok(u32::from_be_bytes(slice))  // BigEndian
    }

    // Symulacja manual memory management (jak w Odin)
    fn allocate(&mut self, size: usize) -> usize {
        // Prosta alokacja: znajdź wolne miejsce (uproszczone, bez realnego zarządzania)
        let addr = 0;  // Zawsze od 0 dla demo
        // W realu: znajdź wolny blok
        addr
    }

    fn deallocate(&mut self, _addr: usize) {
        // Uproszczone: nic nie rób
    }
}

fn main() -> miette::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Użycie: hs3 <input.object>");
        std::process::exit(1);
    }

    let input_path = &args[1];

    // Ładuj .object
    let mut file = File::open(input_path)?;
    let mut reader = BufReader::new(&mut file);

    // Odczytaj header (długość serialized AST)
    let ast_len = reader.read_u32::<BigEndian>().map_err(|e| VmError::Io(e))?;

    // Odczytaj serialized AST
    let mut ast_bytes = vec![0u8; ast_len as usize];
    reader.read_exact(&mut ast_bytes)?;
    let program: Program = deserialize(&ast_bytes)?;

    // Odczytaj resztę jako bytecode
    let mut bytecode = Vec::new();
    reader.read_to_end(&mut bytecode)?;

    // Uruchom VM
    let mut vm = Vm::new(program, bytecode);
    vm.run()?;

    println!("Wykonanie VM zakończone.");

    Ok(())
}

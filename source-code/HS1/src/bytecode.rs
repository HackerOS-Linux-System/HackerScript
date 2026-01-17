use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    Nop = 0,
    PushConst = 1,     // u32 index
    LogString = 3,
    BeginFunc = 10,
    EndFunc = 11,
    Halt = 255,
}

#[derive(Debug)]
pub struct Bytecode {
    pub code: Vec<u8>,
    pub constants: Vec<String>, // na razie tylko stringi
}

pub struct BytecodeEmitter {
    code: Vec<u8>,
    constants: Vec<String>,
}

impl BytecodeEmitter {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn emit(&mut self, op: Opcode) {
        self.code.push(op as u8);
    }

    pub fn emit_u32(&mut self, value: u32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    pub fn add_constant(&mut self, s: String) -> usize {
        let idx = self.constants.len();
        self.constants.push(s);
        idx
    }

    pub fn finish(self) -> Bytecode {
        Bytecode {
            code: self.code,
            constants: self.constants,
        }
    }
}

pub fn write_to_file(bytecode: &Bytecode, path: &Path) -> Result<()> {
    let mut file = File::create(path).context("Cannot create output file")?;

    // Prosty format:
    // u32 code_len
    // [code]
    // u32 const_count
    // [u32 len][utf8 bytes] × const_count

    let code_len = bytecode.code.len() as u32;
    file.write_all(&code_len.to_le_bytes())?;

    file.write_all(&bytecode.code)?;

    let const_count = bytecode.constants.len() as u32;
    file.write_all(&const_count.to_le_bytes())?;

    for s in &bytecode.constants {
        let bytes = s.as_bytes();
        let len = bytes.len() as u32;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(bytes)?;
    }

    Ok(())
}

pub fn pretty_print(bytecode: &Bytecode) {
    println!("Constants ({}):", bytecode.constants.len());
    for (i, s) in bytecode.constants.iter().enumerate() {
        println!("  {:3}: {:?}", i, s);
    }

    println!("\nCode:");
    let mut i = 0;
    while i < bytecode.code.len() {
        let op = bytecode.code[i];
        print!("{:04x}: ", i);
        match op {
            0 => println!("nop"),
            1 => {
                if i + 4 < bytecode.code.len() {
                    let idx = u32::from_le_bytes([
                        bytecode.code[i + 1],
                        bytecode.code[i + 2],
                        bytecode.code[i + 3],
                        bytecode.code[i + 4],
                    ]);
                    println!("push_const {}", idx);
                    i += 4;
                } else {
                    println!("push_const <incomplete>");
                }
            }
            3 => println!("log_string"),
            10 => println!("begin_func"),
            11 => println!("end_func"),
            255 => println!("halt"),
            _ => println!("??? (0x{:02x})", op),
        }
        i += 1;
    }
}

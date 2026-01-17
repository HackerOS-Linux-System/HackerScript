use anyhow::Result;
use pest::iterators::Pair;
use crate::parser::Rule;
use crate::bytecode::{BytecodeEmitter, Opcode};

pub struct Compiler {
    emitter: BytecodeEmitter,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            emitter: BytecodeEmitter::new(),
        }
    }

    pub fn compile_pair(&mut self, pair: Pair<Rule>) -> Result<()> {
        match pair.as_rule() {
            Rule::program => {
                for inner in pair.into_inner() {
                    self.compile_pair(inner)?;
                }
            }
            Rule::log_stmt => {
                let mut inner = pair.into_inner();
                let string_pair = inner.next().unwrap();
                let s = string_pair.as_str().trim_matches('"');
                let idx = self.emitter.add_constant(s.to_string());

                self.emitter.emit(Opcode::PushConst);
                self.emitter.emit_u32(idx as u32);
                self.emitter.emit(Opcode::LogString);
            }
            Rule::func_def => {
                self.emitter.emit(Opcode::BeginFunc);
                // skip "func" + identifier + "(" + params? + ")"
                for stmt in pair.into_inner().skip(2) {
                    self.compile_pair(stmt)?;
                }
                self.emitter.emit(Opcode::EndFunc);
            }
            Rule::EOI | Rule::comment | Rule::ws | Rule::newline => {}
            other => {
                log::warn!("Unhandled rule: {:?}", other);
            }
        }
        Ok(())
    }

    pub fn finish(self) -> crate::bytecode::Bytecode {
        self.emitter.finish()
    }
}

use anyhow::Result;
use pest::iterators::{Pair, Pairs};
use crate::parser::{Rule, HackerScriptParser};
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
                self.emitter.emit(Opcode::PushConst(idx as u32));
                self.emitter.emit(Opcode::LogString);
            }

            Rule::func_def => {
                // bardzo uproszczone – w realnym kompilatorze trzeba obsługiwać scope, params, itd.
                self.emitter.emit(Opcode::BeginFunc);
                for stmt in pair.into_inner().skip(2) { // skip "func" + name
                    self.compile_pair(stmt)?;
                }
                self.emitter.emit(Opcode::EndFunc);
            }

            // ... inne reguły: object, import, require, expressions, etc.

            Rule::EOI | Rule::comment | Rule::ws | Rule::newline => {
                // ignorujemy
            }

            other => {
                log::warn!("Unhandled rule in compiler: {:?}", other);
            }
        }
        Ok(())
    }

    pub fn finish(self) -> crate::bytecode::Bytecode {
        self.emitter.finish()
    }
}

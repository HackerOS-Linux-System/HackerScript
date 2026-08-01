from __future__ import annotations

from dataclasses import dataclass, field

from . import ast_nodes as A

RUST_TYPE_MAP = {
    "Int": "i64",
    "Float": "f64",
    "Str": "String",
    "Bool": "bool",
    "Void": "()",
}


class NativeCodegenError(Exception):
    def __init__(self, message: str, line: int, code: str = "E0010"):
        super().__init__(f"[hackerc] {code} (linia {line}): {message}")
        self.line = line
        self.code = code


def rust_type(t: A.TypeRef | None, line: int) -> str:
    if t is None:
        raise NativeCodegenError(
            "'native fun' wymaga jawnego typu (brak adnotacji typu)", line
        )
    if t.name == "List":
        if t.generic is None:
            raise NativeCodegenError(
                "'List' w 'native fun' wymaga typu elementu, np. List<Int>", line
            )
        return f"Vec<{rust_type(t.generic, line)}>"
    if t.name in RUST_TYPE_MAP:
        return RUST_TYPE_MAP[t.name]
    raise NativeCodegenError(
        f"typ {t.name!r} nie jest jeszcze wspierany w 'native fun' "
        f"(dozwolone: Int, Float, Str, Bool, Void, List<T>) - "
        f"patrz docs/ROADMAP.md",
        line,
    )


def _rust_string_literal(s: str) -> str:
    out = ['"']
    for ch in s:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


@dataclass
class NativeFunGen:
    """Generuje kod Rust dla pojedynczej `native fun`."""

    direct_blocks: dict
    lines: list = field(default_factory=list)
    indent: int = 0

    def emit(self, text: str = ""):
        self.lines.append(("    " * self.indent + text) if text else "")

    def gen_fun(self, node: A.FunDecl) -> str:
        params = []
        for p in node.params:
            params.append(f"{p.name}: {rust_type(p.type_, node.line)}")
        ret = rust_type(node.ret_type, node.line) if node.ret_type else "()"
        self.emit("#[pyfunction]")
        self.emit(f"pub fn {node.name}({', '.join(params)}) -> {ret} {{")
        self.indent += 1
        for stmt in node.body:
            self.gen_stmt(stmt)
        self.indent -= 1
        self.emit("}")
        self.emit("")
        return "\n".join(self.lines)

    # -- statements ----------------------------------------------------

    def gen_block(self, stmts: list):
        self.emit("{")
        self.indent += 1
        for s in stmts:
            self.gen_stmt(s)
        self.indent -= 1
        self.emit("}")

    def gen_stmt(self, node):
        if isinstance(node, A.LetStmt):
            hint = f": {rust_type(node.type_, node.line)}" if node.type_ else ""
            value = self.gen_expr(node.value) if node.value is not None else "Default::default()"
            kw = "let" if node.is_const else "let mut"
            self.emit(f"{kw} {node.name}{hint} = {value};")
            return
        if isinstance(node, A.AssignStmt):
            self.emit(f"{self.gen_expr(node.target)} {node.op} {self.gen_expr(node.value)};")
            return
        if isinstance(node, A.IfStmt):
            self.emit(f"if {self.gen_expr(node.cond)} {{")
            self.indent += 1
            for s in node.body:
                self.gen_stmt(s)
            self.indent -= 1
            for econd, ebody in node.elifs:
                self.emit(f"}} else if {self.gen_expr(econd)} {{")
                self.indent += 1
                for s in ebody:
                    self.gen_stmt(s)
                self.indent -= 1
            if node.else_body is not None:
                self.emit("} else {")
                self.indent += 1
                for s in node.else_body:
                    self.gen_stmt(s)
                self.indent -= 1
            self.emit("}")
            return
        if isinstance(node, A.WhileStmt):
            self.emit(f"while {self.gen_expr(node.cond)} {{")
            self.indent += 1
            for s in node.body:
                self.gen_stmt(s)
            self.indent -= 1
            self.emit("}")
            return
        if isinstance(node, A.ForStmt):
            self.emit(f"for {node.var} in {self.gen_expr(node.iterable)} {{")
            self.indent += 1
            for s in node.body:
                self.gen_stmt(s)
            self.indent -= 1
            self.emit("}")
            return
        if isinstance(node, A.ReturnStmt):
            if node.value is not None:
                self.emit(f"return {self.gen_expr(node.value)};")
            else:
                self.emit("return;")
            return
        if isinstance(node, A.BreakStmt):
            self.emit("break;")
            return
        if isinstance(node, A.ContinueStmt):
            self.emit("continue;")
            return
        if isinstance(node, A.ManualBlock):
            # W native fun 'manual' to PRAWDZIWY blok unsafe Rusta - nie
            # tylko komentarz (w przeciwienstwie do backendu Python).
            self.emit("unsafe {")
            self.indent += 1
            for s in node.body:
                self.gen_stmt(s)
            self.indent -= 1
            self.emit("}")
            return
        if isinstance(node, A.GcPragma):
            self.emit(f"// gc:use::{node.mode} - bez znaczenia w native fun (Rust: brak GC, wlasnosc/pozyczanie)")
            return
        if isinstance(node, A.StructDecl):
            raise NativeCodegenError(
                "'struct' wewnatrz 'native fun' nie jest jeszcze wspierane - "
                "patrz docs/ROADMAP.md",
                node.line,
            )
        if isinstance(node, A.ExprStmt):
            e = node.expr
            if (
                isinstance(e, A.Call)
                and isinstance(e.callee, A.Ident)
                and e.callee.name == "__direct__"
            ):
                idx = int(e.args[0].value)
                raw = self.direct_blocks.get(idx, "")
                for line in (raw.splitlines() or [""]):
                    self.emit(line)
                return
            if isinstance(e, A.Call) and isinstance(e.callee, A.Ident) and e.callee.name == "log":
                self.emit(self._gen_log(e.args) + ";")
                return
            self.emit(self.gen_expr(e) + ";")
            return
        raise NativeCodegenError(f"nieobslugiwana instrukcja w native fun: {node!r}", getattr(node, "line", 0))

    def _gen_log(self, args: list) -> str:
        fmt = " ".join(["{}"] * len(args))
        rendered = ", ".join(self.gen_expr(a) for a in args)
        sep = ", " if args else ""
        return f'println!("{fmt}"{sep}{rendered})'

    # -- expressions ------------------------------------------------------

    def gen_expr(self, node) -> str:
        if isinstance(node, A.NumberLit):
            return node.value
        if isinstance(node, A.StringLit):
            return _rust_string_literal(node.value) + ".to_string()"
        if isinstance(node, A.BoolLit):
            return "true" if node.value else "false"
        if isinstance(node, A.NullLit):
            return "None"
        if isinstance(node, A.Ident):
            return node.name
        if isinstance(node, A.ListLit):
            return "vec![" + ", ".join(self.gen_expr(i) for i in node.items) + "]"
        if isinstance(node, A.UnaryOp):
            if node.op == "not":
                return f"!({self.gen_expr(node.operand)})"
            return f"{node.op}({self.gen_expr(node.operand)})"
        if isinstance(node, A.BinOp):
            op = {"and": "&&", "or": "||"}.get(node.op, node.op)
            return f"({self.gen_expr(node.left)} {op} {self.gen_expr(node.right)})"
        if isinstance(node, A.Attr):
            return f"{self.gen_expr(node.target)}.{node.name}"
        if isinstance(node, A.Index):
            return f"{self.gen_expr(node.target)}[{self.gen_expr(node.index)} as usize]"
        if isinstance(node, A.Call):
            if isinstance(node.callee, A.Ident) and node.callee.name == "log":
                return self._gen_log(node.args)
            args = ", ".join(self.gen_expr(a) for a in node.args)
            return f"{self.gen_expr(node.callee)}({args})"
        raise NativeCodegenError(f"nieobslugiwane wyrazenie w native fun: {node!r}", getattr(node, "line", 0))


def generate_native_module(fun_decls: list, package_name: str, direct_blocks: dict) -> str:
    """Generuje kompletny plik `lib.rs` (crate cdylib + bindingi PyO3) dla
    wszystkich `native fun` znalezionych w jednym pliku zrodlowym .hcs."""
    header = [
        "// Plik wygenerowany automatycznie przez hackerc (backend native/Rust).",
        "// NIE EDYTUJ RECZNIE.",
        "#![allow(unused_mut)]",
        "",
        "use pyo3::prelude::*;",
        "",
    ]
    body = []
    names = []
    for fn in fun_decls:
        gen = NativeFunGen(direct_blocks=direct_blocks)
        body.append(gen.gen_fun(fn))
        names.append(fn.name)

    module_fn = [
        "#[pymodule]",
        f"fn {package_name}_native(_py: Python, m: &PyModule) -> PyResult<()> {{",
    ]
    for name in names:
        module_fn.append(f"    m.add_function(wrap_pyfunction!({name}, m)?)?;")
    module_fn.append("    Ok(())")
    module_fn.append("}")

    return "\n".join(header) + "\n" + "\n".join(body) + "\n" + "\n".join(module_fn) + "\n"


def generate_native_cargo_toml(package_name: str) -> str:
    return f"""[package]
name = "{package_name}"
version = "0.0.1"
edition = "2021"

[lib]
name = "{package_name}"
crate-type = ["cdylib"]

[dependencies]
pyo3 = {{ version = "0.22", features = ["extension-module"] }}
"""

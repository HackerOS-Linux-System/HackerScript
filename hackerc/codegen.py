from __future__ import annotations

from . import ast_nodes as A
from .typeinfer import Signatures, TypeEnv, infer_expr_type

TYPE_MAP = {
    "Int": "int",
    "Float": "float",
    "Str": "str",
    "Bool": "bool",
    "Any": "object",
    "Void": "None",
    "List": "list",
    "Map": "dict",
}


def py_type(t: A.TypeRef | None) -> str:
    if t is None:
        return ""
    base = TYPE_MAP.get(t.name, t.name)
    if t.generic is not None:
        return f"{base}[{py_type(t.generic)}]"
    return base


class CodeGen:
    def __init__(self, direct_blocks: dict[int, str] | None = None, native_package: str = "hackerscript"):
        self.lines: list[str] = []
        self.indent = 0
        self.direct_blocks = direct_blocks or {}
        self.needs_dataclass = False
        self.native_package = native_package
        self.natives: list[A.FunDecl] = []
        self.sigs: Signatures | None = None
        self.env: TypeEnv | None = None

    def emit(self, text: str = ""):
        if text == "":
            self.lines.append("")
        else:
            self.lines.append("    " * self.indent + text)

    def gen_program(self, prog: A.Program) -> str:
        header = [
            "#!/usr/bin/env python3",
            "# -*- coding: utf-8 -*-",
            "#",
            "# Plik wygenerowany automatycznie przez hackerc (transpilator HackerScript).",
            "# NIE EDYTUJ RECZNIE - edytuj zrodlo .hcs w cmd/ i uruchom `virus build` ponownie.",
            "#",
            "from __future__ import annotations",
            "",
        ]
        body_lines: list[str] = []
        has_main = any(isinstance(s, A.FunDecl) and s.name == "main" for s in prog.body)

        self.sigs = Signatures(prog)
        self.env = TypeEnv(self.sigs)  # srodowisko globalne (module-level let)

        for stmt in prog.body:
            self.gen_stmt(stmt)

        if self.needs_dataclass:
            header.insert(len(header) - 1, "from dataclasses import dataclass, field")
        if self.natives:
            native_names = ", ".join(sorted(fn.name for fn in self.natives))
            header.insert(
                len(header) - 1,
                f"from {self.native_package}_native import {native_names}  "
                f"# native fun -> Rust/PyO3, patrz cache/native/",
            )

        out = header + self.lines
        if has_main:
            out += ["", "", 'if __name__ == "__main__":', "    main()"]
        return "\n".join(out) + "\n"

    # -- statements ---------------------------------------------------

    def gen_block(self, stmts: list):
        self.indent += 1
        if not stmts:
            self.emit("pass")
        for s in stmts:
            self.gen_stmt(s)
        self.indent -= 1

    def gen_stmt(self, node):
        if isinstance(node, A.UsingStmt):
            self.emit(f"# using <{node.version}>  (wymagana wersja hackerc)")
            return
        if isinstance(node, A.GetImportStmt):
            self.gen_get_import(node)
            return
        if isinstance(node, A.GcPragma):
            self.emit(f"# gc:use::{node.mode}  (pragma zarzadzania pamiecia - patrz libs/core)")
            return
        if isinstance(node, A.LetStmt):
            type_ = node.type_
            if type_ is None and node.value is not None:
                # REALNA inferencja (hackerc.typeinfer), nie poleganie na
                # dynamicznym typowaniu Pythona - wynik trafia jako jawna
                # adnotacja do wygenerowanego kodu.
                type_ = infer_expr_type(node.value, self.env)
            self.env.declare(node.name, type_)
            hint = f": {py_type(type_)}" if type_ else ""
            value = self.gen_expr(node.value) if node.value is not None else "None"
            tag = "  # const" if node.is_const else ""
            self.emit(f"{node.name}{hint} = {value}{tag}")
            return
        if isinstance(node, A.AssignStmt):
            self.emit(f"{self.gen_expr(node.target)} {node.op} {self.gen_expr(node.value)}")
            return
        if isinstance(node, A.FunDecl):
            self.gen_fun(node)
            return
        if isinstance(node, A.IfStmt):
            self.emit(f"if {self.gen_expr(node.cond)}:")
            self.gen_block(node.body)
            for econd, ebody in node.elifs:
                self.emit(f"elif {self.gen_expr(econd)}:")
                self.gen_block(ebody)
            if node.else_body is not None:
                self.emit("else:")
                self.gen_block(node.else_body)
            return
        if isinstance(node, A.WhileStmt):
            self.emit(f"while {self.gen_expr(node.cond)}:")
            self.gen_block(node.body)
            return
        if isinstance(node, A.ForStmt):
            self.emit(f"for {node.var} in {self.gen_expr(node.iterable)}:")
            self.gen_block(node.body)
            return
        if isinstance(node, A.ReturnStmt):
            if node.value is not None:
                self.emit(f"return {self.gen_expr(node.value)}")
            else:
                self.emit("return")
            return
        if isinstance(node, A.BreakStmt):
            self.emit("break")
            return
        if isinstance(node, A.ContinueStmt):
            self.emit("continue")
            return
        if isinstance(node, A.ManualBlock):
            self.emit("# --- manual [unsafe] block: start (reczne zarzadzanie pamiecia) ---")
            for s in node.body:
                self.gen_stmt(s)
            self.emit("# --- manual [unsafe] block: end ---")
            return
        if isinstance(node, A.StructDecl):
            self.gen_struct(node)
            return
        if isinstance(node, A.ExprStmt):
            e = node.expr
            if isinstance(e, A.StringLit) and getattr(e, "_is_doc", False):
                self.emit(f'"""{e.value}"""')
                return
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
            self.emit(self.gen_expr(e))
            return
        raise NotImplementedError(f"nieobslugiwany wezel: {node!r}")

    def gen_get_import(self, node: A.GetImportStmt):
        src = node.source
        name = node.name
        ver_comment = f"  # wersja: {node.version}" if node.version and src in ("pypi", "crates") else ""

        if src in ("std", "core"):
            # Nazwa musi byc zgodna 1:1 z hackerc.project.flat_module_name -
            # to jest cala "umowa" systemu modulow miedzy plikami .hcs:
            # `get <core:memory::arena>` zawsze odpowiada plikowi
            # libs/core/lib/memory/arena.hcs, transpilowanemu przez
            # hackerc.project do pliku o DOKLADNIE tej samej splaszczonej
            # nazwie w tym samym katalogu wyjsciowym. Patrz docs/SYNTAX.md.
            from .project import flat_module_name

            module = flat_module_name(src, name, node.version)
            if node.details:
                self.emit(f"from {module} import {', '.join(node.details)}")
            else:
                self.emit(f"import {module}")
            return

        if src == "crates":
            self.emit(
                f"# get <crates:{name}{'::' + node.version if node.version else ''}> "
                f"-> natywny modul Rust (linkowany statycznie przez virus)"
            )
            if node.details:
                self.emit(f"from {name}_native import {', '.join(node.details)}{ver_comment}")
            else:
                self.emit(f"import {name}_native as {name}{ver_comment}")
            return

        if src == "pypi":
            if node.details:
                self.emit(f"from {name} import {', '.join(node.details)}{ver_comment}")
            else:
                self.emit(f"import {name}{ver_comment}")
            return

        self.emit(f"# get <{src}:{name}> - nieznane zrodlo, pomijam import{ver_comment}")

    def gen_fun(self, node: A.FunDecl):
        if node.is_native:
            self.natives.append(node)
            self.emit(f"# fun {node.name}(...) -> native (Rust/PyO3) - patrz cache/native/src/lib.rs")
            return
        params = []
        fn_env = TypeEnv(self.sigs)
        for p in node.params:
            hint = f": {py_type(p.type_)}" if p.type_ else ""
            default = f" = {self.gen_expr(p.default)}" if p.default is not None else ""
            params.append(f"{p.name}{hint}{default}")
            fn_env.declare(p.name, p.type_)
        ret = f" -> {py_type(node.ret_type)}" if node.ret_type else ""
        vis = "" if node.is_pub or True else ""  # brak realnej roznicy w Pythonie na tym etapie
        self.emit(f"def {node.name}({', '.join(params)}){ret}:")
        prev_env, self.env = self.env, fn_env
        self.gen_block(node.body)
        self.env = prev_env
        self.emit("")

    def gen_struct(self, node: A.StructDecl):
        self.needs_dataclass = True
        self.emit("@dataclass")
        self.emit(f"class {node.name}:")
        self.indent += 1
        if not node.fields:
            self.emit("pass")
        for f in node.fields:
            hint = py_type(f.type_) if f.type_ else "object"
            self.emit(f"{f.name}: {hint}")
        self.indent -= 1
        self.emit("")

    # -- expressions ----------------------------------------------------

    def gen_expr(self, node) -> str:
        if isinstance(node, A.NumberLit):
            return node.value
        if isinstance(node, A.StringLit):
            return repr(node.value)
        if isinstance(node, A.BoolLit):
            return "True" if node.value else "False"
        if isinstance(node, A.NullLit):
            return "None"
        if isinstance(node, A.Ident):
            return node.name
        if isinstance(node, A.ListLit):
            return "[" + ", ".join(self.gen_expr(i) for i in node.items) + "]"
        if isinstance(node, A.UnaryOp):
            if node.op == "not":
                return f"not ({self.gen_expr(node.operand)})"
            return f"{node.op}({self.gen_expr(node.operand)})"
        if isinstance(node, A.BinOp):
            op = node.op
            if op == "or":
                op = "or"
            elif op == "and":
                op = "and"
            return f"({self.gen_expr(node.left)} {op} {self.gen_expr(node.right)})"
        if isinstance(node, A.Attr):
            return f"{self.gen_expr(node.target)}.{node.name}"
        if isinstance(node, A.Index):
            return f"{self.gen_expr(node.target)}[{self.gen_expr(node.index)}]"
        if isinstance(node, A.Call):
            callee = node.callee
            if isinstance(callee, A.Ident) and callee.name == "log":
                args = ", ".join(self.gen_expr(a) for a in node.args)
                return f"print({args})"
            args = ", ".join(self.gen_expr(a) for a in node.args)
            return f"{self.gen_expr(callee)}({args})"
        raise NotImplementedError(f"nieobslugiwany wezel wyrazenia: {node!r}")


def generate(prog: A.Program, direct_blocks: dict[int, str] | None = None, native_package: str = "hackerscript"):
    gen = CodeGen(direct_blocks=direct_blocks, native_package=native_package)
    python_code = gen.gen_program(prog)
    return python_code, gen.natives

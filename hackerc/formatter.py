from __future__ import annotations

from . import ast_nodes as A
from .parser import parse
from .transpiler import _extract_direct_blocks


class Formatter:
    def __init__(self, direct_blocks: dict | None = None):
        self.lines: list[str] = []
        self.indent = 0
        self.direct_blocks = direct_blocks or {}

    def emit(self, text: str = ""):
        self.lines.append(("    " * self.indent + text) if text else "")

    def fmt_type(self, type_: A.TypeRef) -> str:
        """Renderuje TypeRef WLACZAJAC argument generyczny - `type_.name`
        samo w sobie gubi `<Int>` w `List<Int>` (bug: formater uzywal
        golego `.name` w kilku miejscach, ignorujac `.generic`)."""
        if type_.generic is not None and type_.generic2 is not None:
            return f"{type_.name}<{self.fmt_type(type_.generic)}, {self.fmt_type(type_.generic2)}>"
        if type_.generic is not None:
            return f"{type_.name}<{self.fmt_type(type_.generic)}>"
        return type_.name

    def format_program(self, prog: A.Program) -> str:
        for i, stmt in enumerate(prog.body):
            if i > 0 and isinstance(stmt, A.FunDecl):
                self.emit()
            self.fmt_stmt(stmt)
        text = "\n".join(self.lines)
        return text.rstrip("\n") + "\n"

    # -- statements ---------------------------------------------------

    def fmt_block(self, stmts: list):
        self.indent += 1
        if not stmts:
            pass
        for s in stmts:
            self.fmt_stmt(s)
        self.indent -= 1

    def fmt_stmt(self, node):
        for comment in getattr(node, "_leading_comments", None) or []:
            self.emit(f"! {comment}" if comment else "!")
        if isinstance(node, A.UsingStmt):
            self.emit(f"using <{node.version}>")
            return
        if isinstance(node, A.GetImportStmt):
            head = f"get <{node.source}:{node.name}"
            if node.version:
                head += f"::{node.version}"
            head += ">"
            if node.details:
                head += " import <" + "::".join(node.details) + ">"
            self.emit(head)
            return
        if isinstance(node, A.IncludeStmt):
            self.emit(f"include <{node.path}>")
            return
        if isinstance(node, A.GcPragma):
            self.emit(f"gc:use::{node.mode}")
            return
        if isinstance(node, A.LetStmt):
            kw = "const" if node.is_const else "let"
            hint = f": {self.fmt_type(node.type_)}" if node.type_ else ""
            value = f" = {self.fmt_expr(node.value)}" if node.value is not None else ""
            self.emit(f"{kw} {node.name}{hint}{value}")
            return
        if isinstance(node, A.AssignStmt):
            self.emit(f"{self.fmt_expr(node.target)} {node.op} {self.fmt_expr(node.value)}")
            return
        if isinstance(node, A.FunDecl):
            self.fmt_fun(node)
            return
        if isinstance(node, A.ExternFunDecl):
            params = []
            for p in node.params:
                hint = f": {self.fmt_type(p.type_)}" if p.type_ else ""
                params.append(f"{p.name}{hint}")
            ret = f" -> {self.fmt_type(node.ret_type)}" if node.ret_type else ""
            self.emit(f'extern "{node.lib}" fun {node.name}({", ".join(params)}){ret}')
            return
        if isinstance(node, A.IfStmt):
            self.emit(f"if {self.fmt_expr(node.cond)} [")
            self.fmt_block(node.body)
            for econd, ebody in node.elifs:
                self.emit(f"] elif {self.fmt_expr(econd)} [")
                self.fmt_block(ebody)
            if node.else_body is not None:
                self.emit("] else [")
                self.fmt_block(node.else_body)
            self.emit("]")
            return
        if isinstance(node, A.WhileStmt):
            self.emit(f"while {self.fmt_expr(node.cond)} [")
            self.fmt_block(node.body)
            self.emit("]")
            return
        if isinstance(node, A.ForStmt):
            self.emit(f"for {node.var} in {self.fmt_expr(node.iterable)} [")
            self.fmt_block(node.body)
            self.emit("]")
            return
        if isinstance(node, A.ReturnStmt):
            self.emit(f"end {self.fmt_expr(node.value)}" if node.value is not None else "end")
            return
        if isinstance(node, A.BreakStmt):
            self.emit("break")
            return
        if isinstance(node, A.ContinueStmt):
            self.emit("continue")
            return
        if isinstance(node, A.ManualBlock):
            self.emit("manual [")
            self.fmt_block(node.body)
            self.emit("]")
            return
        if isinstance(node, A.StructDecl):
            gen_head = f"<{', '.join(node.type_params)}>" if node.type_params else ""
            self.emit(f"struct {node.name}{gen_head} [")
            self.indent += 1
            for i, f in enumerate(node.fields):
                comma = "," if i < len(node.fields) - 1 else ""
                type_name = self.fmt_type(f.type_) if f.type_ else "Any"
                self.emit(f"{f.name}: {type_name}{comma}")
            self.indent -= 1
            self.emit("]")
            return
        if isinstance(node, A.EnumDecl):
            gen_head = f"<{', '.join(node.type_params)}>" if node.type_params else ""
            self.emit(f"enum {node.name}{gen_head} [")
            self.indent += 1
            for i, v in enumerate(node.variants):
                comma = "," if i < len(node.variants) - 1 else ""
                if v.fields:
                    args = ", ".join(self.fmt_type(t) for t in v.fields)
                    self.emit(f"{v.name}({args}){comma}")
                else:
                    self.emit(f"{v.name}{comma}")
            self.indent -= 1
            self.emit("]")
            return
        if isinstance(node, A.ImplDecl):
            gen_head = f"<{', '.join(node.type_params)}>" if node.type_params else ""
            self.emit(f"impl {node.struct_name}{gen_head} [")
            self.indent += 1
            for i, m in enumerate(node.methods):
                if i > 0:
                    self.emit()
                self.fmt_fun(m)
            self.indent -= 1
            self.emit("]")
            return
        if isinstance(node, A.MatchStmt):
            self.emit(f"match {self.fmt_expr(node.subject)} [")
            self.indent += 1
            for arm in node.arms:
                head = arm.variant
                if arm.binds:
                    head += "(" + ", ".join(arm.binds) + ")"
                self.emit(f"{head} -> [")
                self.fmt_block(arm.body)
                self.emit("]")
            self.indent -= 1
            self.emit("]")
            return
        if isinstance(node, A.ExprStmt):
            e = node.expr
            if isinstance(e, A.StringLit) and getattr(e, "_is_doc", False):
                self.emit(f"!! {e.value}")
                return
            if (
                isinstance(e, A.Call)
                and isinstance(e.callee, A.Ident)
                and e.callee.name == "__direct__"
            ):
                idx = int(e.args[0].value)
                raw = self.direct_blocks.get(idx, "")
                self.emit("direct [")
                self.indent += 1
                for line in (raw.splitlines() or [""]):
                    self.emit(line)
                self.indent -= 1
                self.emit("]")
                return
            self.emit(self.fmt_expr(e))
            return
        raise NotImplementedError(f"formatter: nieobslugiwany wezel {node!r}")

    def fmt_fun(self, node: A.FunDecl):
        for doc in getattr(node, "_leading_doc_comments", None) or []:
            self.emit(f"!! {doc}")
        params = []
        for p in node.params:
            hint = f": {self.fmt_type(p.type_)}" if p.type_ else ""
            default = f" = {self.fmt_expr(p.default)}" if p.default is not None else ""
            params.append(f"{p.name}{hint}{default}")
        ret = f" -> {self.fmt_type(node.ret_type)}" if node.ret_type else ""
        prefix = "pub " if node.is_pub else ""
        gen_head = f"<{', '.join(node.type_params)}>" if node.type_params else ""
        self.emit(f"{prefix}fun {node.name}{gen_head}({', '.join(params)}){ret} [")
        self.fmt_block(node.body)
        self.emit("]")

    # -- expressions ------------------------------------------------------

    def fmt_expr(self, node) -> str:
        if node is None:
            return ""
        if isinstance(node, A.NumberLit):
            return node.value
        if isinstance(node, A.StringLit):
            escaped = node.value.replace("\\", "\\\\").replace('"', '\\"')
            return f'"{escaped}"'
        if isinstance(node, A.BoolLit):
            return "true" if node.value else "false"
        if isinstance(node, A.NullLit):
            return "null"
        if isinstance(node, A.Ident):
            return node.name
        if isinstance(node, A.ListLit):
            return "[" + ", ".join(self.fmt_expr(i) for i in node.items) + "]"
        if isinstance(node, A.UnaryOp):
            if node.op == "not":
                return f"not {self.fmt_expr(node.operand)}"
            return f"{node.op}{self.fmt_expr(node.operand)}"
        if isinstance(node, A.BinOp):
            return f"{self.fmt_expr(node.left)} {node.op} {self.fmt_expr(node.right)}"
        if isinstance(node, A.Attr):
            return f"{self.fmt_expr(node.target)}.{node.name}"
        if isinstance(node, A.Index):
            return f"{self.fmt_expr(node.target)}[{self.fmt_expr(node.index)}]"
        if isinstance(node, A.Cast):
            return f"{self.fmt_expr(node.target)} as {self.fmt_type(node.type_)}"
        if isinstance(node, A.TryOp):
            return f"{self.fmt_expr(node.target)}?"
        if isinstance(node, A.Call):
            args = ", ".join(self.fmt_expr(a) for a in node.args)
            return f"{self.fmt_expr(node.callee)}({args})"
        raise NotImplementedError(f"formatter: nieobslugiwane wyrazenie {node!r}")


def format_source(source: str, filename: str = "<hcs>") -> str:
    stripped, direct_blocks = _extract_direct_blocks(source)
    program = parse(stripped)
    return Formatter(direct_blocks=direct_blocks).format_program(program)

from __future__ import annotations

from . import ast_nodes as A

_NUMERIC = {"Int", "Float"}


def _t(name: str, generic: A.TypeRef | None = None) -> A.TypeRef:
    return A.TypeRef(name=name, generic=generic)


class Signatures:
    """Zebrane z gory sygnatury funkcji i pol structow calego programu -
    potrzebne, bo wywolania/uzycia moga poprzedzac deklaracje w pliku."""

    def __init__(self, program: A.Program):
        self.functions: dict[str, A.FunDecl] = {}
        self.structs: dict[str, A.StructDecl] = {}
        for stmt in program.body:
            if isinstance(stmt, A.FunDecl):
                self.functions[stmt.name] = stmt
            elif isinstance(stmt, A.StructDecl):
                self.structs[stmt.name] = stmt


class TypeEnv:
    """Srodowisko typow zmiennych lokalnych - jedno na funkcje (bootstrap:
    bez blokowego scoping, tak jak faktyczna semantyka Pythona pod spodem)."""

    def __init__(self, sigs: Signatures):
        self.sigs = sigs
        self.vars: dict[str, A.TypeRef | None] = {}

    def declare(self, name: str, type_: A.TypeRef | None):
        self.vars[name] = type_

    def lookup(self, name: str) -> A.TypeRef | None:
        return self.vars.get(name)


def _types_equal(a: A.TypeRef | None, b: A.TypeRef | None) -> bool:
    if a is None or b is None:
        return False
    if a.name != b.name:
        return False
    if (a.generic is None) != (b.generic is None):
        return False
    if a.generic is not None:
        return _types_equal(a.generic, b.generic)
    return True


def infer_expr_type(expr, env: TypeEnv) -> A.TypeRef | None:
    if isinstance(expr, A.NumberLit):
        return _t("Float") if "." in expr.value else _t("Int")
    if isinstance(expr, A.StringLit):
        return _t("Str")
    if isinstance(expr, A.BoolLit):
        return _t("Bool")
    if isinstance(expr, A.NullLit):
        return None
    if isinstance(expr, A.Ident):
        return env.lookup(expr.name)
    if isinstance(expr, A.UnaryOp):
        if expr.op == "not":
            return _t("Bool")
        return infer_expr_type(expr.operand, env)
    if isinstance(expr, A.BinOp):
        if expr.op in ("and", "or", "==", "!=", "<", ">", "<=", ">="):
            return _t("Bool")
        lt = infer_expr_type(expr.left, env)
        rt = infer_expr_type(expr.right, env)
        if expr.op == "+" and lt is not None and rt is not None and lt.name == "Str" and rt.name == "Str":
            return _t("Str")
        if lt is not None and rt is not None and lt.name in _NUMERIC and rt.name in _NUMERIC:
            return _t("Float") if "Float" in (lt.name, rt.name) else _t("Int")
        return None
    if isinstance(expr, A.ListLit):
        if not expr.items:
            return _t("List", _t("Any"))
        elem_types = [infer_expr_type(i, env) for i in expr.items]
        first = elem_types[0]
        if first is not None and all(_types_equal(first, t) for t in elem_types):
            return _t("List", first)
        return _t("List", _t("Any"))
    if isinstance(expr, A.Attr):
        target_t = infer_expr_type(expr.target, env)
        if target_t is not None and target_t.name in env.sigs.structs:
            struct = env.sigs.structs[target_t.name]
            for f in struct.fields:
                if f.name == expr.name:
                    return f.type_
        return None
    if isinstance(expr, A.Index):
        target_t = infer_expr_type(expr.target, env)
        if target_t is not None and target_t.name == "List":
            return target_t.generic
        return None
    if isinstance(expr, A.Call):
        callee = expr.callee
        if isinstance(callee, A.Ident):
            if callee.name == "log":
                return None  # Void
            if callee.name in env.sigs.functions:
                return env.sigs.functions[callee.name].ret_type
            if callee.name in env.sigs.structs:
                return _t(callee.name)  # wywolanie struct(...) jako konstruktor
        return None
    return None

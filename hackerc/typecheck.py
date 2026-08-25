from __future__ import annotations

from . import ast_nodes as A
from .diagnostics import Diagnostic
from .typeinfer import Signatures, TypeEnv, infer_expr_type, _types_equal

KNOWN_GET_SOURCES = {"pypi", "crates", "std", "core", "selfhost", "virus"}
_BUILTIN_FUNCS = {"log", "elog", "__direct__", "some", "none", "ok", "err", "read_file", "write_file", "dict", "env_var", "run_command", "run_command_combined", "http_get", "current_dir"}


class Checker:
    def __init__(self, program: A.Program, extra_variant_names: set | None = None):
        self.program = program
        self.sigs = Signatures(program)
        self.diags: list[Diagnostic] = []
        self.imported_names: set[str] = set()
        self.variant_names: set[str] = {
            v.name for e in self.sigs.enums.values() for v in e.variants
        }
        if extra_variant_names:
            # Warianty enum ZAIMPORTOWANYCH z innych plikow (`get
            # <selfhost:ast_nodes> import <Expr>`) - bez tego
            # konstruktor wariantu (`Var(...)`) z importowanego enuma
            # dawal spurious W0002 ("nieznana funkcja"), bo Checker
            # dziala na JEDNYM pliku i nie widzi wariantow zadeklarowanych
            # w pliku, z ktorego ten enum pochodzi. Wypelniane przez
            # `cmd_build` (cli.py) uzywajac
            # `project.collect_project_signatures()` PRZED wywolaniem
            # `check_program()` - patrz docs/ROADMAP.md.
            self.variant_names |= extra_variant_names
        for stmt in program.body:
            if isinstance(stmt, A.GetImportStmt):
                self.imported_names.update(stmt.details)

    def check(self) -> list[Diagnostic]:
        for stmt in self.program.body:
            if isinstance(stmt, A.GetImportStmt):
                self._check_get(stmt)
            elif isinstance(stmt, A.FunDecl):
                self._check_fun(stmt)
            elif isinstance(stmt, A.ImplDecl):
                for m in stmt.methods:
                    self._check_fun(m, self_type=A.TypeRef(name=stmt.struct_name))
        return self.diags

    def _err(self, code: str, message: str, node):
        self.diags.append(Diagnostic("error", code, message, line=getattr(node, "line", 0)))

    def _warn(self, code: str, message: str, node):
        self.diags.append(Diagnostic("warning", code, message, line=getattr(node, "line", 0)))

    def _check_get(self, node: A.GetImportStmt):
        if node.source not in KNOWN_GET_SOURCES:
            self._err(
                "E0003",
                f"nieznane zrodlo {node.source!r} w 'get <{node.source}:{node.name}>' "
                f"(dozwolone: {', '.join(sorted(KNOWN_GET_SOURCES))})",
                node,
            )

    def _check_fun(self, fn: A.FunDecl, self_type: A.TypeRef | None = None):
        env = TypeEnv(self.sigs)
        for p in fn.params:
            if p.name == "self" and self_type is not None:
                env.declare("self", self_type)
            else:
                env.declare(p.name, p.type_)

        used: set[str] = set()
        declared: dict[str, A.LetStmt] = {}

        def visit_expr(e):
            if isinstance(e, A.Ident):
                used.add(e.name)
            elif isinstance(e, A.BinOp):
                visit_expr(e.left)
                visit_expr(e.right)
            elif isinstance(e, A.UnaryOp):
                visit_expr(e.operand)
            elif isinstance(e, A.Attr):
                visit_expr(e.target)
            elif isinstance(e, A.Index):
                visit_expr(e.target)
                visit_expr(e.index)
            elif isinstance(e, A.ListLit):
                for i in e.items:
                    visit_expr(i)
            elif isinstance(e, A.Call):
                if isinstance(e.callee, A.Ident):
                    used.add(e.callee.name)
                    self._check_call(e)
                else:
                    visit_expr(e.callee)
                for a in e.args:
                    visit_expr(a)
            elif isinstance(e, A.Cast):
                visit_expr(e.target)
            elif isinstance(e, A.TryOp):
                visit_expr(e.target)
                if fn.ret_type is None or fn.ret_type.name not in ("Result", "Option"):
                    self.diags.append(Diagnostic(
                        "error", "E0011",
                        f"'?' uzyte w '{fn.name}', ktora nie zwraca Result<T,E> ani "
                        f"Option<T> - '?' propaguje Err/None do OTACZAJACEJ funkcji, "
                        f"wiec jej typ zwracany musi na to pozwalac",
                        getattr(e, "line", fn.line),
                    ))

        def visit_stmts(stmts: list):
            for s in stmts:
                visit_stmt(s)

        def visit_stmt(s):
            if isinstance(s, A.LetStmt):
                if s.value is not None:
                    visit_expr(s.value)
                inferred = infer_expr_type(s.value, env) if s.value is not None else None
                if s.type_ is not None and inferred is not None and not _types_equal(s.type_, inferred):
                    self._err(
                        "E0005",
                        f"zmienna '{s.name}' zadeklarowana jako {s.type_.name}, "
                        f"ale przypisana wartosc ma wywnioskowany typ {inferred.name}",
                        s,
                    )
                env.declare(s.name, s.type_ or inferred)
                declared[s.name] = s
            elif isinstance(s, A.AssignStmt):
                visit_expr(s.value)
                if not isinstance(s.target, A.Ident):
                    visit_expr(s.target)
            elif isinstance(s, A.IfStmt):
                visit_expr(s.cond)
                visit_stmts(s.body)
                for econd, ebody in s.elifs:
                    visit_expr(econd)
                    visit_stmts(ebody)
                if s.else_body is not None:
                    visit_stmts(s.else_body)
            elif isinstance(s, A.WhileStmt):
                visit_expr(s.cond)
                visit_stmts(s.body)
            elif isinstance(s, A.ForStmt):
                visit_expr(s.iterable)
                env.declare(s.var, None)
                visit_stmts(s.body)
            elif isinstance(s, A.ReturnStmt):
                if s.value is not None:
                    visit_expr(s.value)
            elif isinstance(s, A.ManualBlock):
                visit_stmts(s.body)
            elif isinstance(s, A.ExprStmt):
                visit_expr(s.expr)
            elif isinstance(s, A.MatchStmt):
                visit_expr(s.subject)
                for arm in s.arms:
                    for b in arm.binds:
                        env.declare(b, None)
                    visit_stmts(arm.body)

        visit_stmts(fn.body)

        # E0002: funkcja z typem zwracanym musi miec przynajmniej jedno
        # `end <wartosc>` gdzies w ciele (analiza uproszczona - nie sledzi
        # wszystkich sciezek wykonania, tylko obecnosc takiej instrukcji).
        if fn.ret_type is not None and not _has_value_return(fn.body):
            self._err(
                "E0002",
                f"funkcja '{fn.name}' deklaruje typ zwracany {fn.ret_type.name}, "
                f"ale nigdzie nie ma 'end <wartosc>'",
                fn,
            )

        # W0001: nieuzywane zmienne (proste, best-effort - bez analizy
        # przeplywu sterowania/martwego kodu).
        for name, let_node in declared.items():
            if name not in used and not name.startswith("_"):
                self._warn("W0001", f"zmienna '{name}' jest zadeklarowana, ale nigdy nie uzyta", let_node)

    def _check_call(self, call: A.Call):
        name = call.callee.name
        if name in _BUILTIN_FUNCS or name in self.sigs.structs or name in self.variant_names or name in self.imported_names:
            return
        fn = self.sigs.functions.get(name)
        if fn is None:
            # Bootstrap: nie widzimy funkcji z innych plikow/bibliotek jeszcze
            # (patrz docs/ROADMAP.md - system modulow), wiec to tylko warning.
            self._warn(
                "W0002",
                f"wywolanie nieznanej funkcji '{name}' (jesli pochodzi z 'get <...>', "
                f"to ograniczenie bootstrapu - patrz docs/ROADMAP.md)",
                call,
            )
            return
        if len(call.args) != len(fn.params):
            self._err(
                "E0001",
                f"'{name}' oczekuje {len(fn.params)} argument(ow), otrzymano {len(call.args)}",
                call,
            )


def _has_value_return(stmts: list) -> bool:
    for s in stmts:
        if isinstance(s, A.ReturnStmt) and s.value is not None:
            return True
        if isinstance(s, A.IfStmt):
            if _has_value_return(s.body):
                return True
            for _, ebody in s.elifs:
                if _has_value_return(ebody):
                    return True
            if s.else_body is not None and _has_value_return(s.else_body):
                return True
        if isinstance(s, A.WhileStmt) and _has_value_return(s.body):
            return True
        if isinstance(s, A.ForStmt) and _has_value_return(s.body):
            return True
        if isinstance(s, A.ManualBlock) and _has_value_return(s.body):
            return True
        if isinstance(s, A.MatchStmt):
            for arm in s.arms:
                if _has_value_return(arm.body):
                    return True
    return False


def check_program(program: A.Program, extra_variant_names: set | None = None) -> list[Diagnostic]:
    return Checker(program, extra_variant_names=extra_variant_names).check()

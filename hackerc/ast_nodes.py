from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class Node:
    line: int = field(default=0, kw_only=True)


# ---- wyrazenia (expressions) -------------------------------------------------

@dataclass
class NumberLit(Node):
    value: str = ""


@dataclass
class StringLit(Node):
    value: str = ""


@dataclass
class BoolLit(Node):
    value: bool = False


@dataclass
class NullLit(Node):
    pass


@dataclass
class Ident(Node):
    name: str = ""


@dataclass
class BinOp(Node):
    op: str = ""
    left: Any = None
    right: Any = None


@dataclass
class UnaryOp(Node):
    op: str = ""
    operand: Any = None


@dataclass
class Call(Node):
    callee: Any = None
    args: list = field(default_factory=list)


@dataclass
class Index(Node):
    target: Any = None
    index: Any = None


@dataclass
class Cast(Node):
    """`wyrazenie as Typ` - rzutowanie typu, 1:1 na Rust `as`."""
    target: Any = None
    type_: Optional["TypeRef"] = None


@dataclass
class TryOp(Node):
    """`wyrazenie?` - propagacja bledu/braku wartosci, 1:1 na Rust `?`.
    Dziala na Result<T,E> (zwraca Err(e) z otaczajacej fun/metody gdy
    wartosc to Err) i Option<T> (analogicznie dla None) - patrz
    docs/SYNTAX.md."""
    target: Any = None


@dataclass
class Attr(Node):
    target: Any = None
    name: str = ""


@dataclass
class ListLit(Node):
    items: list = field(default_factory=list)


# ---- typy ---------------------------------------------------------------

@dataclass
class TypeRef(Node):
    name: str = ""
    generic: Optional["TypeRef"] = None
    # Drugi argument generyczny - potrzebny WYLACZNIE dla wbudowanego
    # `Result<T, E>` (Option<T> ma tylko jeden, wiec starcza `generic`).
    # Nie jest to ogolny system generykow wieloargumentowych (np.
    # user-defined Dict<K, V>) - patrz docs/ROADMAP.md.
    generic2: Optional["TypeRef"] = None


# ---- instrukcje (statements) ----------------------------------------------

@dataclass
class Param(Node):
    name: str = ""
    type_: Optional[TypeRef] = None
    default: Any = None


@dataclass
class LetStmt(Node):
    name: str = ""
    type_: Optional[TypeRef] = None
    value: Any = None
    is_const: bool = False


@dataclass
class AssignStmt(Node):
    target: Any = None
    op: str = "="
    value: Any = None


@dataclass
class FunDecl(Node):
    name: str = ""
    params: list = field(default_factory=list)
    ret_type: Optional[TypeRef] = None
    body: list = field(default_factory=list)
    is_pub: bool = False
    # Parametry generyczne `<T, U>` - PRAWDZIWE generyki Rusta (Rust je
    # sam monomorfizuje, hackerc nie robi wlasnej monomorfizacji).
    # Patrz docs/ROADMAP.md.
    type_params: list = field(default_factory=list)


@dataclass
class ExternFunDecl(Node):
    """`extern "libname" fun name(params) -> Typ` - deklaracja FFI bez ciala,
    tlumaczona do bloku `extern "C" { fn ... }` w Ruscie (patrz SYNTAX.md)."""
    lib: str = ""
    name: str = ""
    params: list = field(default_factory=list)
    ret_type: Optional[TypeRef] = None


@dataclass
class IfStmt(Node):
    cond: Any = None
    body: list = field(default_factory=list)
    elifs: list = field(default_factory=list)  # list[(cond, body)]
    else_body: Optional[list] = None


@dataclass
class WhileStmt(Node):
    cond: Any = None
    body: list = field(default_factory=list)


@dataclass
class ForStmt(Node):
    var: str = ""
    iterable: Any = None
    body: list = field(default_factory=list)


@dataclass
class ReturnStmt(Node):
    value: Any = None


@dataclass
class BreakStmt(Node):
    pass


@dataclass
class ContinueStmt(Node):
    pass


@dataclass
class ExprStmt(Node):
    expr: Any = None


@dataclass
class GetImportStmt(Node):
    source: str = ""       # np. "pypi", "crates", "std", "core"
    name: str = ""
    version: Optional[str] = None
    details: list = field(default_factory=list)  # z <a:b::c>


@dataclass
class IncludeStmt(Node):
    """`include <sciezka>` - odpowiednik Rustowego `mod` (patrz duza
    uwaga w project.py przy `resolve_include_path`): sciezka WZGLEDEM
    katalogu BIEZACEGO pliku, bez oznaczenia zrodla (w odroznieniu od
    `get <source:name>`, ktory jest osobny i pozostaje nietkniety).
    `path` to surowy tekst jak napisany (`memory/arena`, `io.hcs`,
    `helpers`) - rozwiazywanie na prawdziwa sciezke pliku dzieje sie w
    project.py, nie tutaj."""
    path: str = ""


@dataclass
class UsingStmt(Node):
    version: str = ""


@dataclass
class DirectBlock(Node):
    """Surowy blok Pythona przechodzacy 1:1 do wygenerowanego kodu."""
    raw_lines: list = field(default_factory=list)


@dataclass
class ManualBlock(Node):
    """Odpowiednik `unsafe` - blok recznego zarzadzania pamiecia."""
    body: list = field(default_factory=list)


@dataclass
class GcPragma(Node):
    mode: str = "always"


@dataclass
class StructDecl(Node):
    name: str = ""
    fields: list = field(default_factory=list)  # list[Param]
    type_params: list = field(default_factory=list)  # ["T", "U"] dla struct Nazwa<T, U>


@dataclass
class EnumVariant(Node):
    """Jeden wariant `enum` - `Nazwa` (jednostkowy) albo `Nazwa(Typ, ...)`
    (krotkowy, jak `Some(T)` w Rust) - odpowiada Rust `enum X { A, B(T) }`."""
    name: str = ""
    fields: list = field(default_factory=list)  # list[TypeRef], puste = jednostkowy


@dataclass
class EnumDecl(Node):
    name: str = ""
    variants: list = field(default_factory=list)  # list[EnumVariant]
    type_params: list = field(default_factory=list)


@dataclass
class MatchArm(Node):
    """Jedna galaz `match`: `Wariant(bind1, bind2) -> [ ... ]` albo
    `_ -> [ ... ]` (wildcard/domyslna galaz, wariant name == "_")."""
    variant: str = ""
    binds: list = field(default_factory=list)  # list[str] - nazwy bindowanych zmiennych
    body: list = field(default_factory=list)


@dataclass
class MatchStmt(Node):
    subject: Any = None
    arms: list = field(default_factory=list)  # list[MatchArm]


@dataclass
class ImplDecl(Node):
    """`impl Nazwa [ fun metoda(self, ...) -> Typ [ ... ] ... ]` - metody
    dla struct. Generuje OSOBNY blok Rust `impl Nazwa { ... }` obok tego,
    ktory `gen_struct` juz emituje automatycznie dla `new()` - Rust
    pozwala na wiele blokow `impl` dla tego samego typu w jednym pliku."""
    struct_name: str = ""
    methods: list = field(default_factory=list)  # list[FunDecl]
    type_params: list = field(default_factory=list)


@dataclass
class Program(Node):
    body: list = field(default_factory=list)

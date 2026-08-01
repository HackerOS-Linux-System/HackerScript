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
    is_native: bool = False


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


@dataclass
class Program(Node):
    body: list = field(default_factory=list)

"""
hackerc.parser
==============
Recursive-descent parser: tokeny -> AST (ast_nodes.Program).

Gramatyka jest celowo prosta (bootstrap 0.0.1) - patrz docs/SYNTAX.md
w repo HackerScript po pelny opis skladni docelowej.
"""

from __future__ import annotations

from .lexer import Token, TokKind, tokenize
from . import ast_nodes as A


class ParseError(Exception):
    def __init__(self, message: str, line: int):
        super().__init__(f"[hackerc] blad skladni (linia {line}): {message}")
        self.line = line


_ASSIGN_OPS = {"=", "+=", "-=", "*=", "/="}


class Parser:
    def __init__(self, tokens: list[Token]):
        self.toks = tokens
        self.pos = 0

    # -- helpers ------------------------------------------------------

    def cur(self) -> Token:
        return self.toks[self.pos]

    def at_end(self) -> bool:
        return self.cur().kind == TokKind.EOF

    def advance(self) -> Token:
        t = self.toks[self.pos]
        if self.pos < len(self.toks) - 1:
            self.pos += 1
        return t

    def check(self, kind: TokKind, value: str | None = None) -> bool:
        t = self.cur()
        if t.kind != kind:
            return False
        if value is not None and t.value != value:
            return False
        return True

    def match(self, kind: TokKind, value: str | None = None) -> Token | None:
        if self.check(kind, value):
            return self.advance()
        return None

    def expect(self, kind: TokKind, value: str | None = None) -> Token:
        if not self.check(kind, value):
            t = self.cur()
            expected = value if value else kind.name
            raise ParseError(f"oczekiwano {expected!r}, otrzymano {t.value!r}", t.line)
        return self.advance()

    def skip_newlines(self):
        while self.match(TokKind.NEWLINE):
            pass

    # -- program --------------------------------------------------------

    def parse_program(self) -> A.Program:
        body = []
        self.skip_newlines()
        while not self.at_end():
            stmt = self.parse_statement()
            if stmt is not None:
                body.append(stmt)
            self.skip_newlines()
        return A.Program(body=body, line=0)

    # -- statements -------------------------------------------------------

    def parse_block(self) -> list:
        """Parsuje blok otwierany '[' i zamykany ']'."""
        self.expect(TokKind.OPEN)
        self.skip_newlines()
        stmts = []
        while not self.check(TokKind.CLOSE):
            if self.at_end():
                raise ParseError("nieoczekiwany koniec pliku wewnatrz bloku '[' ... ']'", self.cur().line)
            stmts.append(self.parse_statement())
            self.skip_newlines()
        self.expect(TokKind.CLOSE)
        return stmts

    def parse_statement(self):
        t = self.cur()

        if t.kind == TokKind.DOC_COMMENT:
            self.advance()
            return A.ExprStmt(expr=A.StringLit(value=t.value, line=t.line), line=t.line)

        if t.kind == TokKind.KEYWORD:
            kw = t.value
            if kw == "using":
                return self.parse_using()
            if kw == "get":
                return self.parse_get_import()
            if kw in ("let", "const"):
                return self.parse_let(is_const=(kw == "const"))
            if kw == "pub":
                self.advance()
                inner = self.parse_statement()
                if isinstance(inner, A.FunDecl):
                    inner.is_pub = True
                return inner
            if kw == "fun":
                return self.parse_fun()
            if kw == "if":
                return self.parse_if()
            if kw == "while":
                return self.parse_while()
            if kw == "for":
                return self.parse_for()
            if kw == "end":
                line = self.advance().line
                value = None
                if not self.check(TokKind.NEWLINE) and not self.check(TokKind.CLOSE):
                    value = self.parse_expr()
                return A.ReturnStmt(value=value, line=line)
            if kw == "break":
                line = self.advance().line
                return A.BreakStmt(line=line)
            if kw == "continue":
                line = self.advance().line
                return A.ContinueStmt(line=line)
            if kw == "direct":
                return self.parse_direct()
            if kw == "manual":
                return self.parse_manual()
            if kw == "gc":
                return self.parse_gc_pragma()
            if kw == "struct":
                return self.parse_struct()

        # wyrazenie / przypisanie
        expr = self.parse_expr()
        if self.cur().kind == TokKind.OP and self.cur().value in _ASSIGN_OPS:
            op = self.advance().value
            value = self.parse_expr()
            return A.AssignStmt(target=expr, op=op, value=value, line=t.line)
        return A.ExprStmt(expr=expr, line=t.line)

    def parse_using(self) -> A.UsingStmt:
        line = self.expect(TokKind.KEYWORD, "using").line
        self.expect(TokKind.LANGLE)
        version_parts = []
        while not self.check(TokKind.RANGLE):
            version_parts.append(self.advance().value)
        self.expect(TokKind.RANGLE)
        return A.UsingStmt(version="".join(version_parts), line=line)

    def _read_angle_segment(self) -> str:
        """Czyta tekst az do napotkania '::', '>' lub ':' (na poziomie top)."""
        parts = []
        while not (self.check(TokKind.DCOLON) or self.check(TokKind.RANGLE) or self.check(TokKind.COLON)):
            parts.append(str(self.advance().value))
        return "".join(parts)

    def parse_get_import(self) -> A.GetImportStmt:
        line = self.expect(TokKind.KEYWORD, "get").line
        self.expect(TokKind.LANGLE)
        source = self._read_angle_segment()
        self.expect(TokKind.COLON)
        name = self._read_angle_segment()
        version = None
        if self.match(TokKind.DCOLON):
            version = self._read_angle_segment()
        self.expect(TokKind.RANGLE)

        details: list[str] = []
        if self.match(TokKind.KEYWORD, "import"):
            self.expect(TokKind.LANGLE)
            details.append(self._read_angle_segment())
            while self.match(TokKind.DCOLON):
                details.append(self._read_angle_segment())
            self.expect(TokKind.RANGLE)

        return A.GetImportStmt(source=source, name=name, version=version, details=details, line=line)

    def parse_type(self) -> A.TypeRef:
        t = self.expect(TokKind.IDENT if self.check(TokKind.IDENT) else TokKind.KEYWORD)
        generic = None
        if self.match(TokKind.LANGLE):
            generic = self.parse_type()
            self.expect(TokKind.RANGLE)
        return A.TypeRef(name=t.value, generic=generic, line=t.line)

    def parse_let(self, is_const: bool) -> A.LetStmt:
        line = self.advance().line  # 'let' / 'const'
        name = self.expect(TokKind.IDENT).value
        type_ = None
        if self.match(TokKind.COLON):
            type_ = self.parse_type()
        value = None
        if self.match(TokKind.OP, "="):
            value = self.parse_expr()
        return A.LetStmt(name=name, type_=type_, value=value, is_const=is_const, line=line)

    def parse_params(self) -> list:
        params = []
        self.expect(TokKind.LPAREN)
        while not self.check(TokKind.RPAREN):
            pname = self.expect(TokKind.IDENT).value
            ptype = None
            if self.match(TokKind.COLON):
                ptype = self.parse_type()
            default = None
            if self.match(TokKind.OP, "="):
                default = self.parse_expr()
            params.append(A.Param(name=pname, type_=ptype, default=default))
            if not self.match(TokKind.COMMA):
                break
        self.expect(TokKind.RPAREN)
        return params

    def parse_fun(self) -> A.FunDecl:
        line = self.expect(TokKind.KEYWORD, "fun").line
        name = self.expect(TokKind.IDENT).value
        params = self.parse_params()
        ret_type = None
        if self.match(TokKind.OP, "->"):
            ret_type = self.parse_type()
        body = self.parse_block()
        return A.FunDecl(name=name, params=params, ret_type=ret_type, body=body, line=line)

    def parse_if(self) -> A.IfStmt:
        line = self.expect(TokKind.KEYWORD, "if").line
        cond = self.parse_expr()
        body = self.parse_block()
        elifs = []
        else_body = None
        self.skip_newlines_soft()
        while self.check(TokKind.KEYWORD, "elif"):
            self.advance()
            econd = self.parse_expr()
            ebody = self.parse_block()
            elifs.append((econd, ebody))
            self.skip_newlines_soft()
        if self.check(TokKind.KEYWORD, "else"):
            self.advance()
            else_body = self.parse_block()
        return A.IfStmt(cond=cond, body=body, elifs=elifs, else_body=else_body, line=line)

    def skip_newlines_soft(self):
        """Podglada czy po newline'ach jest elif/else, zeby obslugujic
        'else' na nowej linii po ']'. Cofa sie jesli nie."""
        save = self.pos
        while self.match(TokKind.NEWLINE):
            pass
        if not (self.check(TokKind.KEYWORD, "elif") or self.check(TokKind.KEYWORD, "else")):
            self.pos = save

    def parse_while(self) -> A.WhileStmt:
        line = self.expect(TokKind.KEYWORD, "while").line
        cond = self.parse_expr()
        body = self.parse_block()
        return A.WhileStmt(cond=cond, body=body, line=line)

    def parse_for(self) -> A.ForStmt:
        line = self.expect(TokKind.KEYWORD, "for").line
        var = self.expect(TokKind.IDENT).value
        self.expect(TokKind.KEYWORD, "in")
        iterable = self.parse_expr()
        body = self.parse_block()
        return A.ForStmt(var=var, iterable=iterable, body=body, line=line)

    def parse_direct(self) -> A.DirectBlock:
        line = self.expect(TokKind.KEYWORD, "direct").line
        # direct[] jest wyjatkowe: bierzemy surowy tekst zrodlowy pomiedzy [ ]
        # licząc nawiasy, zamiast tokenizowac jako HackerScript.
        raise ParseError("parse_direct powinien byc obslugiwany na etapie preprocessing", line)

    def parse_manual(self) -> A.ManualBlock:
        line = self.expect(TokKind.KEYWORD, "manual").line
        body = self.parse_block()
        return A.ManualBlock(body=body, line=line)

    def parse_gc_pragma(self) -> A.GcPragma:
        line = self.expect(TokKind.KEYWORD, "gc").line
        self.expect(TokKind.COLON)
        self.expect(TokKind.KEYWORD, "use") if self.check(TokKind.KEYWORD, "use") else self.expect(TokKind.IDENT)
        self.expect(TokKind.DCOLON)
        mode = self.advance().value
        return A.GcPragma(mode=mode, line=line)

    def parse_struct(self) -> A.StructDecl:
        line = self.expect(TokKind.KEYWORD, "struct").line
        name = self.expect(TokKind.IDENT).value
        self.expect(TokKind.OPEN)
        self.skip_newlines()
        fields = []
        while not self.check(TokKind.CLOSE):
            fname = self.expect(TokKind.IDENT).value
            self.expect(TokKind.COLON)
            ftype = self.parse_type()
            fields.append(A.Param(name=fname, type_=ftype))
            self.match(TokKind.COMMA)
            self.skip_newlines()
        self.expect(TokKind.CLOSE)
        return A.StructDecl(name=name, fields=fields, line=line)

    # -- expressions (precedence climbing) --------------------------------

    def parse_expr(self):
        return self.parse_or()

    def parse_or(self):
        left = self.parse_and()
        while self.check(TokKind.KEYWORD, "or") or self.check(TokKind.OP, "||"):
            op = self.advance().value
            right = self.parse_and()
            left = A.BinOp(op="or", left=left, right=right)
        return left

    def parse_and(self):
        left = self.parse_not()
        while self.check(TokKind.KEYWORD, "and") or self.check(TokKind.OP, "&&"):
            op = self.advance().value
            right = self.parse_not()
            left = A.BinOp(op="and", left=left, right=right)
        return left

    def parse_not(self):
        if self.check(TokKind.KEYWORD, "not") or self.check(TokKind.OP, "!"):
            self.advance()
            operand = self.parse_not()
            return A.UnaryOp(op="not", operand=operand)
        return self.parse_comparison()

    def parse_comparison(self):
        left = self.parse_additive()
        while self.check(TokKind.OP) and self.cur().value in ("==", "!=", "<=", ">=") \
                or self.check(TokKind.LANGLE) or self.check(TokKind.RANGLE):
            op_tok = self.advance()
            op = op_tok.value
            right = self.parse_additive()
            left = A.BinOp(op=op, left=left, right=right)
        return left

    def parse_additive(self):
        left = self.parse_mult()
        while self.check(TokKind.OP) and self.cur().value in ("+", "-"):
            op = self.advance().value
            right = self.parse_mult()
            left = A.BinOp(op=op, left=left, right=right)
        return left

    def parse_mult(self):
        left = self.parse_unary()
        while self.check(TokKind.OP) and self.cur().value in ("*", "/", "%"):
            op = self.advance().value
            right = self.parse_unary()
            left = A.BinOp(op=op, left=left, right=right)
        return left

    def parse_unary(self):
        if self.check(TokKind.OP) and self.cur().value in ("-", "+"):
            op = self.advance().value
            operand = self.parse_unary()
            return A.UnaryOp(op=op, operand=operand)
        return self.parse_postfix()

    def parse_postfix(self):
        expr = self.parse_primary()
        while True:
            if self.match(TokKind.LPAREN):
                args = []
                while not self.check(TokKind.RPAREN):
                    args.append(self.parse_expr())
                    if not self.match(TokKind.COMMA):
                        break
                self.expect(TokKind.RPAREN)
                expr = A.Call(callee=expr, args=args)
            elif self.check(TokKind.OP) and self.cur().value == ".":
                self.advance()
                name = self.expect(TokKind.IDENT).value
                expr = A.Attr(target=expr, name=name)
            else:
                break
        return expr

    def parse_primary(self):
        t = self.cur()
        if t.kind == TokKind.NUMBER:
            self.advance()
            return A.NumberLit(value=t.value, line=t.line)
        if t.kind == TokKind.STRING:
            self.advance()
            return A.StringLit(value=t.value, line=t.line)
        if t.kind == TokKind.KEYWORD and t.value in ("true", "false"):
            self.advance()
            return A.BoolLit(value=(t.value == "true"), line=t.line)
        if t.kind == TokKind.KEYWORD and t.value == "null":
            self.advance()
            return A.NullLit(line=t.line)
        if t.kind == TokKind.KEYWORD and t.value == "self":
            self.advance()
            return A.Ident(name="self", line=t.line)
        if t.kind == TokKind.IDENT:
            self.advance()
            return A.Ident(name=t.value, line=t.line)
        if t.kind == TokKind.LPAREN:
            self.advance()
            e = self.parse_expr()
            self.expect(TokKind.RPAREN)
            return e
        if t.kind == TokKind.OPEN:
            self.advance()
            items = []
            self.skip_newlines()
            while not self.check(TokKind.CLOSE):
                items.append(self.parse_expr())
                self.skip_newlines()
                if not self.match(TokKind.COMMA):
                    self.skip_newlines()
                    break
                self.skip_newlines()
            self.expect(TokKind.CLOSE)
            return A.ListLit(items=items, line=t.line)
        raise ParseError(f"nieoczekiwany token {t.value!r}", t.line)


def parse(source: str) -> A.Program:
    tokens = tokenize(source)
    return Parser(tokens).parse_program()

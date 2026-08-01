from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto


class TokKind(Enum):
    NEWLINE = auto()
    OPEN = auto()     # [
    CLOSE = auto()    # ]
    LPAREN = auto()
    RPAREN = auto()
    LANGLE = auto()   # <
    RANGLE = auto()   # >
    COLON = auto()
    DCOLON = auto()   # ::
    COMMA = auto()
    OP = auto()
    NUMBER = auto()
    STRING = auto()
    IDENT = auto()
    KEYWORD = auto()
    DOC_COMMENT = auto()
    EOF = auto()


KEYWORDS = {
    "fun", "let", "const", "if", "else", "elif", "while", "for", "in",
    "return", "end", "get", "import", "using", "direct", "manual",
    "true", "false", "null", "struct", "enum", "match", "break",
    "continue", "gc", "pub", "self", "and", "or", "not", "native",
}

_MULTI_OPS = [
    "==", "!=", "<=", ">=", "->", "::", "&&", "||", "+=", "-=", "*=", "/=",
]
_SINGLE_OPS = set("+-*/%=<>!.,&|^~")


@dataclass
class Token:
    kind: TokKind
    value: str
    line: int
    col: int
    tight: bool = False  # True = brak bialych znakow przed tym tokenem (uzywane dla '[' -> indeksowanie vs blok)

    def __repr__(self) -> str:  # pragma: no cover - debug helper
        return f"Token({self.kind.name}, {self.value!r}, L{self.line})"


class LexError(Exception):
    def __init__(self, message: str, line: int, col: int = 1):
        super().__init__(f"[hackerc] blad leksykalny (linia {line}): {message}")
        self.line = line
        self.col = col
        self.message = message


def strip_comments(source: str) -> str:
    """Usuwa komentarze != ... =! (wieloliniowe) zamieniajac je na
    puste linie (zeby numery linii sie zgadzaly)."""
    out = []
    i = 0
    n = len(source)
    line = 1
    while i < n:
        # wieloliniowy komentarz: != ... =!
        if source[i : i + 2] == "!=":
            start_line = line
            j = source.find("=!", i + 2)
            if j == -1:
                raise LexError("niezamkniety komentarz wieloliniowy != ... =!", start_line)
            block = source[i:j]
            line += block.count("\n")
            out.append("\n" * block.count("\n"))
            i = j + 2
            continue
        out.append(source[i])
        if source[i] == "\n":
            line += 1
        i += 1
    return "".join(out)


def tokenize(source: str) -> list[Token]:
    source = strip_comments(source)
    tokens: list[Token] = []
    line = 1
    line_start = 0  # indeks w `source` gdzie zaczyna sie biezaca linia
    i = 0
    n = len(source)

    def peek(off: int = 0) -> str:
        j = i + off
        return source[j] if j < n else ""

    while i < n:
        col = i - line_start + 1
        c = source[i]

        if c == "\n":
            tokens.append(Token(TokKind.NEWLINE, "\n", line, col))
            line += 1
            line_start = i + 1
            i += 1
            continue

        if c in " \t\r":
            i += 1
            continue

        # !! doc comment (do konca linii)
        if c == "!" and peek(1) == "!":
            j = i + 2
            while j < n and source[j] != "\n":
                j += 1
            tokens.append(Token(TokKind.DOC_COMMENT, source[i + 2 : j].strip(), line, col))
            i = j
            continue

        # ! komentarz jednoliniowy (do konca linii) - ale nie mylic z != ktore
        # jest juz usuwane w strip_comments, oraz nie mylic z operatorem != (nierownosc)
        if c == "!" and not (peek(1).isalnum() and False):
            # sprawdz czy to naprawde komentarz, a nie operator '!=' uzyty jako porownanie.
            # Zasada: '!' rozpoczyna komentarz jednoliniowy chyba ze nastepny znak to '='
            # (to jest operator != obslugiwany jako MULTI_OP powyzej i juz wyciety jako
            # blok komentarza wieloliniowego przez strip_comments -- wiec tu zawsze to
            # jest czysty komentarz jednoliniowy).
            j = i + 1
            while j < n and source[j] != "\n":
                j += 1
            i = j
            continue

        if c == '"' or c == "'":
            quote = c
            j = i + 1
            buf = []
            while j < n and source[j] != quote:
                if source[j] == "\\" and j + 1 < n:
                    buf.append(source[j : j + 2])
                    j += 2
                    continue
                if source[j] == "\n":
                    raise LexError("niezamkniety string", line, col)
                buf.append(source[j])
                j += 1
            if j >= n:
                raise LexError("niezamkniety string", line, col)
            tokens.append(Token(TokKind.STRING, "".join(buf), line, col))
            i = j + 1
            continue

        if c.isdigit():
            j = i
            while j < n and (source[j].isdigit() or source[j] == "."):
                j += 1
            tokens.append(Token(TokKind.NUMBER, source[i:j], line, col))
            i = j
            continue

        if c.isalpha() or c == "_":
            j = i
            while j < n and (source[j].isalnum() or source[j] == "_"):
                j += 1
            word = source[i:j]
            kind = TokKind.KEYWORD if word in KEYWORDS else TokKind.IDENT
            tokens.append(Token(kind, word, line, col))
            i = j
            continue

        if c == "[":
            tight = i > 0 and source[i - 1] not in " \t\r\n"
            tokens.append(Token(TokKind.OPEN, "[", line, col, tight=tight)); i += 1; continue
        if c == "]":
            tokens.append(Token(TokKind.CLOSE, "]", line, col)); i += 1; continue
        if c == "(":
            tokens.append(Token(TokKind.LPAREN, "(", line, col)); i += 1; continue
        if c == ")":
            tokens.append(Token(TokKind.RPAREN, ")", line, col)); i += 1; continue
        if c == ",":
            tokens.append(Token(TokKind.COMMA, ",", line, col)); i += 1; continue

        if source[i : i + 2] == "::":
            tokens.append(Token(TokKind.DCOLON, "::", line, col)); i += 2; continue
        if c == ":":
            tokens.append(Token(TokKind.COLON, ":", line, col)); i += 1; continue

        matched = False
        for op in _MULTI_OPS:
            if source[i : i + len(op)] == op:
                tokens.append(Token(TokKind.OP, op, line, col))
                i += len(op)
                matched = True
                break
        if matched:
            continue

        # '<' i '>' pojedyncze - jako LANGLE/RANGLE (uzywane zarowno jako
        # nawiasy generic/get<...> jak i operatory porownania w parserze).
        # Warianty dwuznakowe (<=, >=, ->) sa juz obsluzone powyzej przez _MULTI_OPS.
        if c == "<":
            tokens.append(Token(TokKind.LANGLE, "<", line, col)); i += 1; continue
        if c == ">":
            tokens.append(Token(TokKind.RANGLE, ">", line, col)); i += 1; continue

        if c in _SINGLE_OPS:
            tokens.append(Token(TokKind.OP, c, line, col)); i += 1; continue

        raise LexError(f"nieoczekiwany znak {c!r}", line, col)

    tokens.append(Token(TokKind.EOF, "", line, i - line_start + 1))
    return tokens

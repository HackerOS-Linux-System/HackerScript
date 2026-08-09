from __future__ import annotations

from . import ast_nodes as A
from .typeinfer import Signatures, TypeEnv, infer_expr_type, _types_equal

RUST_TYPE_MAP = {
    "Int": "i64",
    "Float": "f64",
    "Str": "String",
    "Bool": "bool",
    "Void": "()",
}


def _contains_any(t: A.TypeRef | None) -> bool:
    if t is None:
        return False
    if t.name == "Any":
        return True
    return _contains_any(t.generic)


class CodegenError(Exception):
    def __init__(self, message: str, line: int, code: str = "E0010"):
        super().__init__(f"[hackerc] {code} (linia {line}): {message}")
        self.line = line
        self.code = code


def rust_type(t: A.TypeRef | None, line: int, structs: dict | None = None, enums: dict | None = None, type_params: set | None = None) -> str:
    if t is None:
        raise CodegenError("brakuje jawnego typu (Any/brak adnotacji nie jest wspierane w kompilacji do Rusta)", line)
    if type_params and t.name in type_params:
        # Parametr generyczny (np. 'T' w 'struct Box<T>'/'fun f<T>(...)') -
        # Rust sam go monomorfizuje, hackerc nie sprawdza tu nic wiecej
        # (brak np. ograniczen typu 'T: Clone') - patrz docs/ROADMAP.md.
        return t.name
    if t.name == "List":
        if t.generic is None:
            raise CodegenError("'List' wymaga typu elementu, np. List<Int>", line)
        return f"Vec<{rust_type(t.generic, line, structs, enums, type_params)}>"
    if t.name == "Dict":
        if t.generic is None or t.generic2 is None:
            raise CodegenError("'Dict' wymaga dwoch typow, np. Dict<Str, Int> (klucz, wartosc)", line)
        return f"std::collections::HashMap<{rust_type(t.generic, line, structs, enums, type_params)}, {rust_type(t.generic2, line, structs, enums, type_params)}>"
    if t.name == "Option":
        if t.generic is None:
            raise CodegenError("'Option' wymaga typu wartosci, np. Option<Int>", line)
        return f"Option<{rust_type(t.generic, line, structs, enums, type_params)}>"
    if t.name == "Result":
        if t.generic is None or t.generic2 is None:
            raise CodegenError("'Result' wymaga dwoch typow, np. Result<Str, Str> (wartosc, blad)", line)
        return f"Result<{rust_type(t.generic, line, structs, enums, type_params)}, {rust_type(t.generic2, line, structs, enums, type_params)}>"
    if t.name in RUST_TYPE_MAP:
        return RUST_TYPE_MAP[t.name]
    if structs and t.name in structs:
        return t.name if t.generic is None else f"{t.name}<{rust_type(t.generic, line, structs, enums, type_params)}>"
    if enums and t.name in enums:
        return t.name if t.generic is None else f"{t.name}<{rust_type(t.generic, line, structs, enums, type_params)}>"
    raise CodegenError(
        f"typ {t.name!r} nie jest wspierany (dozwolone: Int, Float, Str, Bool, "
        f"Void, List<T>, Dict<K,V>, Option<T>, Result<T,E>, lub nazwa znanego "
        f"struct/enum/parametru generycznego) - patrz docs/ROADMAP.md",
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


def _python_raw_string(s: str) -> str:
    """Rust raw string literal r#"..."# ktory bezpiecznie pomiesci dowolny
    kod Pythona (w tym cudzyslowy) - uzywane przez direct[]."""
    hashes = "#"
    while f'"{hashes}' in s:
        hashes += "#"
    return f'r{hashes}"{s}"{hashes}'


_MUTATING_METHODS = {"push", "pop", "remove", "insert", "clear", "sort", "extend", "truncate"}


def _mutated_names_in_body(body: list) -> set[str]:
    """Rdzen analizy z `_compute_mut_params`/`_compute_method_mut_params`
    wydzielony do wspolnego uzytku: zwraca nazwy zmiennych/parametrow,
    ktorych POLA sa gdzies w podanym ciele przypisywane LUB mutowane
    metoda typu .push/.pop."""
    mutated: set[str] = set()

    def mark_base(base):
        while isinstance(base, A.Index):
            base = base.target
        if isinstance(base, A.Attr) and isinstance(base.target, A.Ident):
            mutated.add(base.target.name)
        elif isinstance(base, A.Ident):
            mutated.add(base.name)

    def walk_expr(expr):
        if expr is None:
            return
        if isinstance(expr, A.Call):
            if isinstance(expr.callee, A.Attr) and expr.callee.name in _MUTATING_METHODS:
                mark_base(expr.callee.target)
            walk_expr(expr.callee)
            for a in expr.args:
                walk_expr(a)
        elif isinstance(expr, A.BinOp):
            walk_expr(expr.left)
            walk_expr(expr.right)
        elif isinstance(expr, A.UnaryOp):
            walk_expr(expr.operand)
        elif isinstance(expr, A.Attr):
            walk_expr(expr.target)
        elif isinstance(expr, A.Index):
            walk_expr(expr.target)
            walk_expr(expr.index)
        elif isinstance(expr, A.ListLit):
            for it in expr.items:
                walk_expr(it)

    def walk(node):
        if isinstance(node, A.AssignStmt):
            target = node.target
            while isinstance(target, A.Index):
                target = target.target
            if isinstance(target, A.Attr) and isinstance(target.target, A.Ident):
                mutated.add(target.target.name)
            elif isinstance(target, A.Ident):
                mutated.add(target.name)
            walk_expr(node.value)
        elif isinstance(node, A.ExprStmt):
            walk_expr(node.expr)
        elif isinstance(node, A.LetStmt):
            walk_expr(node.value)
        elif isinstance(node, A.ReturnStmt):
            walk_expr(node.value)
        elif isinstance(node, A.IfStmt):
            walk_expr(node.cond)
            for s in node.body:
                walk(s)
            for econd, ebody in node.elifs:
                walk_expr(econd)
                for s in ebody:
                    walk(s)
            if node.else_body:
                for s in node.else_body:
                    walk(s)
        elif isinstance(node, A.WhileStmt):
            walk_expr(node.cond)
            for s in node.body:
                walk(s)
        elif isinstance(node, (A.ForStmt, A.ManualBlock)):
            for s in node.body:
                walk(s)
        elif isinstance(node, A.MatchStmt):
            walk_expr(node.subject)
            for arm in node.arms:
                for s in arm.body:
                    walk(s)

    for s in body:
        walk(s)
    return mutated


def _compute_mut_params(prog: A.Program) -> dict[str, set[str]]:
    """Zwraca {nazwa_funkcji: {nazwy parametrow ktorych POLA sa gdzies w
    ciele przypisywane LUB mutowane metoda typu .push/.pop}} - potrzebne,
    zeby zdecydowac czy parametr typu struct powinien byc w Rust `&mut T`
    (mutowany), `&T` (tylko odczyt), czy zwyklym typem."""
    result: dict[str, set[str]] = {}
    for stmt in prog.body:
        if not isinstance(stmt, A.FunDecl):
            continue
        result[stmt.name] = _mutated_names_in_body(stmt.body)
    return result


def _find_self_method_calls(body: list) -> set:
    """Zwraca nazwy metod wywolanych jako `self.metoda(...)` gdziekolwiek
    w ciele (w tym zagniezdzone w if/while/match) - uzywane przez
    `_compute_method_mut_params` do wykrycia POSREDNIEJ mutacji `self`
    (metoda A woa self.B(), a B mutuje - A rowniez potrzebuje &mut self,
    nie tylko metody z BEZPOSREDNIM przypisaniem self.pole = ...)."""
    calls: set = set()

    def walk_expr(e):
        if e is None:
            return
        if isinstance(e, A.Call):
            if (
                isinstance(e.callee, A.Attr)
                and isinstance(e.callee.target, A.Ident)
                and e.callee.target.name == "self"
            ):
                calls.add(e.callee.name)
            walk_expr(e.callee)
            for a in e.args:
                walk_expr(a)
        elif isinstance(e, A.BinOp):
            walk_expr(e.left)
            walk_expr(e.right)
        elif isinstance(e, A.UnaryOp):
            walk_expr(e.operand)
        elif isinstance(e, A.Attr):
            walk_expr(e.target)
        elif isinstance(e, A.Index):
            walk_expr(e.target)
            walk_expr(e.index)
        elif isinstance(e, A.ListLit):
            for it in e.items:
                walk_expr(it)
        elif isinstance(e, A.Cast):
            walk_expr(e.target)
        elif isinstance(e, A.TryOp):
            walk_expr(e.target)

    def walk(node):
        if isinstance(node, A.AssignStmt):
            walk_expr(node.target)
            walk_expr(node.value)
        elif isinstance(node, A.ExprStmt):
            walk_expr(node.expr)
        elif isinstance(node, A.LetStmt):
            walk_expr(node.value)
        elif isinstance(node, A.ReturnStmt):
            walk_expr(node.value)
        elif isinstance(node, A.IfStmt):
            walk_expr(node.cond)
            for s in node.body:
                walk(s)
            for econd, ebody in node.elifs:
                walk_expr(econd)
                for s in ebody:
                    walk(s)
            if node.else_body:
                for s in node.else_body:
                    walk(s)
        elif isinstance(node, A.WhileStmt):
            walk_expr(node.cond)
            for s in node.body:
                walk(s)
        elif isinstance(node, (A.ForStmt, A.ManualBlock)):
            for s in node.body:
                walk(s)
        elif isinstance(node, A.MatchStmt):
            walk_expr(node.subject)
            for arm in node.arms:
                for s in arm.body:
                    walk(s)

    for s in body:
        walk(s)
    return calls


def _compute_method_mut_params(prog: A.Program, extra_method_mut_params: dict | None = None) -> dict:
    """Jak `_compute_mut_params`, ale dla metod w blokach `impl` - klucz
    to "StructName::method" (nazwy metod moga sie powtarzac miedzy
    strukturami, w przeciwienstwie do wolnych funkcji).

    Uwzglednia MUTACJE POSREDNIE: jesli metoda A wywoluje `self.B()`, a
    B potrzebuje `&mut self` (bezposrednio albo TEZ posrednio), to A
    rowniez potrzebuje `&mut self` - inaczej Rust odrzuci wywolanie
    metody `&mut self` przez `&self` ("cannot borrow as mutable").
    Liczone jako punkt staly, bo wywolania moga byc zagniezdzone na
    dowolna glebokosc (A->B->C->...) albo cykliczne (rekurencja).

    `extra_method_mut_params` to znane Z GORY mut-params metod z INNYCH
    plikow (np. `impl Parser [ ... ]` rozbity na dwa pliki .hcs
    polaczone przez `get <selfhost:...>`) - bez tego metoda w TYM pliku
    wywolujaca `self.metoda_z_innego_pliku()` nie wiedzialaby, czy ta
    metoda mutuje, i zostalaby blednie wygenerowana z `&self`."""
    result = dict(extra_method_mut_params) if extra_method_mut_params else {}
    method_info = {}
    for stmt in prog.body:
        if not isinstance(stmt, A.ImplDecl):
            continue
        for m in stmt.methods:
            key = f"{stmt.struct_name}::{m.name}"
            result[key] = _mutated_names_in_body(m.body)
            method_info[key] = (stmt.struct_name, m.name, m.body)

    changed = True
    while changed:
        changed = False
        for key, (struct_name, _method_name, body) in method_info.items():
            if "self" in result[key]:
                continue
            for called in _find_self_method_calls(body):
                other_key = f"{struct_name}::{called}"
                if other_key in result and "self" in result[other_key]:
                    result[key].add("self")
                    changed = True
                    break
    return result


class CodeGen:
    def __init__(self, direct_blocks: dict[int, str] | None = None, module_name: str = "module"):
        self.lines: list[str] = []
        self.indent = 0
        self.direct_blocks = direct_blocks or {}
        self.module_name = module_name
        self.sigs: Signatures | None = None
        self.needs_pyo3 = False
        self.extern_libs: set[str] = set()
        self.mut_params: dict[str, set[str]] = {}
        self.method_mut_params: dict[str, set[str]] = {}
        self.env: TypeEnv | None = None
        self.local_structs: set[str] = set()
        # {nazwa_wariantu: nazwa_enuma} zebrane ze WSZYSTKICH `enum` w
        # programie - zakladamy unikalnosc nazw wariantow w calym pliku
        # (uproszczenie bootstrapu: pozwala kwalifikowac `Circle(5.0)`
        # jako `Shape::Circle(5.0)` bez znajomosci statycznego typu
        # wyrazenia dopasowywanego w `match`). Patrz docs/ROADMAP.md.
        self.variant_to_enum: dict[str, str] = {}
        self.variant_arity: dict[str, int] = {}
        self._BUILTIN_VARIANTS = {"Some": 1, "None": 0, "Ok": 1, "Err": 1}
        # {(nazwa_struct, nazwa_metody): FunDecl} - potrzebne w gen_expr,
        # zeby wywolania metod (obj.method(args)) mogly dostac te sama
        # auto-referencje `&`/`&mut` na argumentach co wolne funkcje.
        self.methods_registry: dict[tuple[str, str], A.FunDecl] = {}
        # Pola struct/warianty enum ktore sa (bezposrednio LUB posrednio -
        # patrz _build_recursion_info) samo-referencyjne i dostaja
        # automatyczne `Box<...>` w Ruscie. Wartosc to 'direct' albo
        # 'option' (rozstrzyga miedzy `Box<X>` i `Option<Box<X>>`).
        self.boxed_struct_fields: dict[str, dict] = {}
        self.boxed_variant_fields: dict[tuple, list] = {}
        self._no_default_structs: set = set()
        # Metody/mut-params z INNYCH plikow .hcs (gdy `impl TenStruct`
        # jest rozbite na wiele plikow polaczonych przez `get
        # <selfhost/std/core:...>`) - ustawiane z zewnatrz przez
        # `generate()` przed wywolaniem `gen_program()`.
        self._extra_methods: dict | None = None
        self._extra_method_mut_params: dict | None = None
        # Zwracany typ aktualnie generowanej fun/metody - uzywane, zeby
        # `end self.pole`/`return self.pole` (zwrocenie POLA obiektu
        # przekazanego przez referencje - self/param dostaja zawsze
        # `&`/`&mut`) dostalo `.clone()`; bez tego Rust odrzuca proba
        # przeniesienia wlasnosci pola spod referencji (E0507).
        self.current_ret_type: A.TypeRef | None = None
        # Parametry generyczne aktualnie w zasiegu (np. {"T"} wewnatrz
        # 'struct Box<T>'/'fun f<T>(...)'/'impl<T> Box<T>') - ustawiane
        # na wejsciu do gen_struct/gen_enum/gen_fun/gen_impl/gen_method i
        # przekazywane do rust_type(), zeby rozpoznac 'T' jako poprawny typ.
        self.current_type_params: set = set()

    def emit(self, text: str = ""):
        self.lines.append(("    " * self.indent + text) if text else "")

    def _build_variant_registry(self):
        self.variant_to_enum = {}
        self.variant_arity = {}
        for enum_name, enum_decl in self.sigs.enums.items():
            for v in enum_decl.variants:
                if v.name in self._BUILTIN_VARIANTS:
                    raise CodegenError(
                        f"wariant '{v.name}' w 'enum {enum_name}' koliduje z wbudowanym "
                        f"wariantem Option/Result o tej samej nazwie (Some/None/Ok/Err sa "
                        f"zarezerwowane) - wybierz inna nazwe",
                        enum_decl.line,
                    )
                existing = self.variant_to_enum.get(v.name)
                if existing is not None and existing != enum_name:
                    # Nazwy wariantow musza byc unikalne w CALYM programie, nie
                    # tylko w obrebie jednego enuma - kwalifikacja `Circle(...)`
                    # -> `Shape::Circle(...)` w gen_expr/gen_match dziala przez
                    # globalny rejestr wariant->enum, bez znajomosci statycznego
                    # typu dopasowywanego wyrazenia. Cicha kolizja bylaby gorsza
                    # niz jawny blad tutaj - patrz docs/ROADMAP.md.
                    raise CodegenError(
                        f"wariant '{v.name}' jest zdefiniowany w wiecej niz jednym enum "
                        f"('{existing}' i '{enum_name}') - nazwy wariantow musza byc "
                        f"unikalne w calym programie (ograniczenie bootstrapu, patrz "
                        f"docs/ROADMAP.md)",
                        enum_decl.line,
                    )
                self.variant_to_enum[v.name] = enum_name
                self.variant_arity[v.name] = len(v.fields)

    def _build_methods_registry(self, prog: A.Program, extra_methods: dict | None = None):
        self.methods_registry = dict(extra_methods) if extra_methods else {}
        for stmt in prog.body:
            if isinstance(stmt, A.ImplDecl):
                for m in stmt.methods:
                    self.methods_registry[(stmt.struct_name, m.name)] = m

    @staticmethod
    def _sizing_edge(type_: "A.TypeRef"):
        """Czy `type_` przyczynia sie do rozmiaru struct/enum "przez
        wartosc" (a nie przez wskaznik/indeksowanie) - tylko takie typy
        moga stworzyc cykl o nieskonczonym rozmiarze, ktory Rust
        odrzuci. Zwraca (nazwa_celu, 'direct'|'option') albo (None, None).

        `List<X>`/`Dict<K,V>` NIE licza sie - `Vec`/`HashMap` sa juz
        posrednie (alokowane na kopcu), wiec `struct S [ xs: List<S> ]`
        kompiluje sie bez Box. Tylko `X` (bezposrednio) albo `Option<X>`
        licza sie."""
        if type_.generic is None and type_.generic2 is None:
            return type_.name, "direct"
        if type_.name == "Option" and type_.generic is not None:
            return type_.generic.name, "option"
        return None, None

    def _build_recursion_info(self, prog: A.Program):
        """Wykrywa BEZPOSREDNIA *i POSREDNIA* rekurencje w polach
        struct/enum (`next: Node` w `struct Node`, ale rowniez `struct A
        [ b: B ]` + `struct B [ a: A ]`) i automatycznie owija JEDNO
        pole na kazdym cyklu w `Box<...>` w wygenerowanym Ruscie - bez
        tego Rust odrzuci typ o nieskonczonym rozmiarze.

        Algorytm: budujemy graf skierowany struct/enum -> struct/enum po
        polach "liczacych sie do rozmiaru" (patrz `_sizing_edge`), potem
        standardowe DFS z kolorowaniem (biale/szare/czarne): kazda
        krawedz prowadzaca do wezla AKTUALNIE SZAREGO (czyli lezacego na
        biezacej sciezce DFS) zamyka cykl - taka krawedz (czyli
        KONKRETNE pole) oznaczamy do zboxowania. To standardowa technika
        (feedback arc set przez DFS) - gwarantuje, ze po zboxowaniu
        oznaczonych pol graf przyczynowy dla rozmiaru jest bez cykli,
        niezaleznie ile cykli (w tym zachodzacych na siebie) bylo w
        oryginalnym grafie."""
        known = set(self.sigs.structs) | set(self.sigs.enums)

        # edges: (owner, field_key, target, option_kind) - field_key to
        # nazwa pola (struct) albo (nazwa_wariantu, indeks_pola) (enum).
        edges = []
        adjacency = {}
        for name, decl in self.sigs.structs.items():
            for f in decl.fields:
                if f.type_ is None:
                    continue
                target, kind = self._sizing_edge(f.type_)
                if target in known:
                    e = (name, f.name, target, kind)
                    edges.append(e)
                    adjacency.setdefault(name, []).append(e)
        for name, decl in self.sigs.enums.items():
            for v in decl.variants:
                for idx, t in enumerate(v.fields):
                    target, kind = self._sizing_edge(t)
                    if target in known:
                        e = (name, (v.name, idx), target, kind)
                        edges.append(e)
                        adjacency.setdefault(name, []).append(e)

        WHITE, GRAY, BLACK = 0, 1, 2
        color = {n: WHITE for n in known}
        boxed_edges = set()

        def dfs(node):
            color[node] = GRAY
            for (owner, field_key, target, _kind) in adjacency.get(node, []):
                if color.get(target) == GRAY:
                    boxed_edges.add((owner, field_key))
                elif color.get(target, WHITE) == WHITE:
                    dfs(target)
            color[node] = BLACK

        for n in known:
            if color[n] == WHITE:
                dfs(n)

        self.boxed_struct_fields = {}
        self.boxed_variant_fields = {}
        for (owner, field_key, target, kind) in edges:
            if (owner, field_key) not in boxed_edges:
                continue
            if owner in self.sigs.structs:
                self.boxed_struct_fields.setdefault(owner, {})[field_key] = kind
            else:
                variant_name, idx = field_key
                vkey = (owner, variant_name)
                if vkey not in self.boxed_variant_fields:
                    variant_decl = next(vv for vv in self.sigs.enums[owner].variants if vv.name == variant_name)
                    self.boxed_variant_fields[vkey] = [None] * len(variant_decl.fields)
                self.boxed_variant_fields[vkey][idx] = kind

        # `#[derive(Default)]` na struct wymaga, zeby KAZDE pole samo
        # implementowalo Default. To NIE JEST prawda dla:
        #   - pol typu enum (enumy NIGDY nie derive'uja Default w tym
        #     codegen - Rust nie ma dla nich "oczywistego" wariantu
        #     domyslnego bez jawnej #[default]);
        #   - pol typu `Result<T, E>` (Rust std NIE implementuje
        #     Default dla Result w ogole);
        #   - pol typu struct, ktory SAM nie moze derive'owac Default
        #     (rekurencyjnie - w tym struct w cyklu, patrz Box wyzej);
        #   - pol typu `Box<X>` bez `Option` (bezposrednio boxowane -
        #     `Box<X>: Default` wymaga `X: Default`).
        # `Option<X>`/`List<X>`/`Dict<K,V>`/`Option<Box<X>>` sa ZAWSZE
        # Default niezaleznie od X (Option/Vec/HashMap default do
        # None/puste, bez wymagan na X) - stad NIE blokuja Default.
        # Liczymy punkt staly: zaczynamy od struct z bezposrednio
        # blokujacym polem, potem dodajemy KAZDY struct, ktory (przez
        # wartosc) zawiera juz zablokowany struct - i tak, az nic sie
        # nie zmienia.
        def _field_blocks_default(t: A.TypeRef) -> bool:
            if t.name == "Option" or t.name == "List" or t.name == "Dict":
                return False  # zawsze Default niezaleznie od argumentu
            if t.name in self.sigs.enums:
                return True  # enumy nigdy nie derive'uja Default
            if t.name == "Result":
                return True  # Rust nie implementuje Default dla Result
            if t.name in self.sigs.structs:
                return t.name in self._no_default_structs
            return False  # Int/Float/Bool/Str/parametr generyczny - Default

        self._no_default_structs = set()
        changed = True
        while changed:
            changed = False
            for name, decl in self.sigs.structs.items():
                if name in self._no_default_structs:
                    continue
                for f in decl.fields:
                    if f.type_ is None:
                        continue
                    kind = self.boxed_struct_fields.get(name, {}).get(f.name)
                    if kind == "option":
                        continue  # Option<Box<X>> - zawsze Default
                    if kind == "direct" or _field_blocks_default(f.type_):
                        self._no_default_structs.add(name)
                        changed = True
                        break

    # -- program --------------------------------------------------------

    def gen_program(self, prog: A.Program, _skip_sig_setup: bool = False) -> str:
        if not _skip_sig_setup:
            self.sigs = Signatures(prog)
            self.mut_params = _compute_mut_params(prog)
            self.local_structs = set(self.sigs.structs.keys())
        self.method_mut_params = _compute_method_mut_params(prog, self._extra_method_mut_params)
        self._build_variant_registry()
        self._build_methods_registry(prog, self._extra_methods)
        self._build_recursion_info(prog)
        header = [
            "// Plik wygenerowany automatycznie przez hackerc (HackerScript -> Rust).",
            "// NIE EDYTUJ RECZNIE - edytuj zrodlo .hcs i uruchom `virus build` ponownie.",
            "#![allow(non_snake_case, unused_mut, dead_code)]",
            "",
        ]
        for stmt in prog.body:
            self.gen_toplevel(stmt)

        if self.needs_pyo3:
            header.insert(len(header) - 1, "use pyo3::prelude::*;")
        for lib in sorted(self.extern_libs):
            pass  # linki #[link(name=...)] sa emitowane bezposrednio przy ExternFunDecl

        return "\n".join(header + self.lines) + "\n"

    def gen_toplevel(self, node):
        if isinstance(node, A.UsingStmt):
            self.emit(f"// using <{node.version}> (wymagana wersja hackerc)")
            return
        if isinstance(node, A.GetImportStmt):
            self.gen_get_import(node)
            return
        if isinstance(node, A.GcPragma):
            self.emit(f"// gc:use::{node.mode} - Rust: brak GC, wlasnosc/pozyczanie zamiast tego")
            return
        if isinstance(node, A.StructDecl):
            self.gen_struct(node)
            return
        if isinstance(node, A.EnumDecl):
            self.gen_enum(node)
            return
        if isinstance(node, A.ImplDecl):
            self.gen_impl(node)
            return
        if isinstance(node, A.ExternFunDecl):
            self.gen_extern(node)
            return
        if isinstance(node, A.FunDecl):
            self.gen_fun(node)
            return
        if isinstance(node, A.LetStmt) and node.is_const:
            # stala globalna. Str MUSI byc &str (nie String) - .to_string()
            # nie jest funkcja const, wiec `const X: String = "y".to_string()`
            # nie skompilowalby sie.
            if node.type_ is not None and node.type_.name == "Str":
                if not isinstance(node.value, A.StringLit):
                    raise CodegenError("stala globalna typu Str musi byc literalem string", node.line)
                self.emit(f'pub const {node.name.upper()}: &str = {_rust_string_literal(node.value.value)};')
                return
            hint = rust_type(node.type_, node.line, self.sigs.structs, self.sigs.enums, self.current_type_params) if node.type_ else "i64"
            self.emit(f"pub const {node.name.upper()}: {hint} = {self.gen_expr(node.value)};")
            return
        if isinstance(node, A.ExprStmt) and isinstance(node.expr, A.StringLit) and getattr(node.expr, "_is_doc", False):
            self.emit(f"//! {node.expr.value}")
            return
        raise CodegenError(f"nieobslugiwana instrukcja na najwyzszym poziomie: {node!r}", getattr(node, "line", 0))

    def gen_get_import(self, node: A.GetImportStmt):
        src = node.source
        name = node.name
        if src in ("std", "core", "selfhost"):
            from .project import flat_module_name

            module = flat_module_name(src, name, node.version)
            if node.details:
                self.emit(f"use crate::{module}::{{{', '.join(node.details)}}};")
            else:
                self.emit(f"use crate::{module}::*;")
            return
        if src == "crates":
            # prawdziwa zaleznosc Cargo (dopisywana do Cargo.toml przez
            # hackerc.project) - tu tylko `use`.
            if node.details:
                self.emit(f"use {name}::{{{', '.join(node.details)}}};")
            else:
                self.emit(f"use {name}::*;")
            return
        if src == "pypi":
            self.emit(
                f"// get <pypi:{name}> - dostepne wewnatrz bloku direct[ ... ] "
                f"(interpreter Pythona), nie bezposrednio w kodzie Rust"
            )
            return
        if src in ("npm", "jsr"):
            self.emit(
                f"// get <{src}:{name}> - pobierane przez `virus install` do cache/libs/{src}/{name}/; "
                f"integracja z uruchomieniem JS jest jeszcze w budowie, patrz docs/ROADMAP.md"
            )
            return
        self.emit(f"// get <{src}:{name}> - nieznane zrodlo, pomijam import")

    def gen_extern(self, node: A.ExternFunDecl):
        params = ", ".join(
            f"{p.name}: {rust_type(p.type_, node.line, self.sigs.structs, self.sigs.enums, self.current_type_params)}" for p in node.params
        )
        ret = f" -> {rust_type(node.ret_type, node.line, self.sigs.structs, self.sigs.enums, self.current_type_params)}" if node.ret_type else ""
        self.emit(f'#[link(name = "{node.lib}")]')
        self.emit("extern \"C\" {")
        self.indent += 1
        self.emit(f"pub fn {node.name}({params}){ret};")
        self.indent -= 1
        self.emit("}")
        self.emit("")

    def _field_rust_type(self, owner_name: str, field_name: str, f_type: A.TypeRef | None, line: int) -> str:
        """Jak `rust_type()`, ale automatycznie owija pole oznaczone w
        `boxed_struct_fields` (przez `_build_recursion_info` - obejmuje
        BEZPOSREDNIA i POSREDNIA rekurencje) w `Box<...>` wokol
        FAKTYCZNEGO typu pola (`next: Node` -> `Box<Node>`, `next:
        Option<Node>` -> `Option<Box<Node>>`, ale rowniez np. `b: B` w
        `struct A` gdy `A`/`B` sie cyklicznie odwolują) - bez tego Rust
        odrzuci typ o nieskonczonym rozmiarze."""
        if f_type is None:
            return "i64"
        kind = self.boxed_struct_fields.get(owner_name, {}).get(field_name)
        if kind == "option" and f_type.generic is not None:
            inner = rust_type(f_type.generic, line, self.sigs.structs, self.sigs.enums, self.current_type_params)
            return f"Option<Box<{inner}>>"
        if kind == "direct":
            inner = rust_type(f_type, line, self.sigs.structs, self.sigs.enums, self.current_type_params)
            return f"Box<{inner}>"
        return rust_type(f_type, line, self.sigs.structs, self.sigs.enums, self.current_type_params)

    def gen_struct(self, node: A.StructDecl):
        self.current_type_params = set(node.type_params)
        gen_head = f"<{', '.join(node.type_params)}>" if node.type_params else ""
        is_recursive = node.name in self._no_default_structs
        derive = "#[derive(Debug, Clone, PartialEq)]" if (node.type_params or is_recursive) else "#[derive(Debug, Clone, PartialEq, Default)]"
        # 'Default' pomijane dla generycznych struct (wymagaloby T: Default)
        # i dla (bezposrednio lub posrednio) samo-referencyjnych struct
        # (Box<Self>: Default istnieje, ale wywolanie na runtime wpadloby
        # w nieskonczona rekursje - patrz docs/ROADMAP.md).
        self.emit(derive)
        self.emit(f"pub struct {node.name}{gen_head} {{")
        self.indent += 1
        for f in node.fields:
            hint = self._field_rust_type(node.name, f.name, f.type_, node.line)
            self.emit(f"pub {f.name}: {hint},")
        self.indent -= 1
        self.emit("}")
        self.emit("")
        # Konstruktor pozycyjny `Nazwa(a, b)` w HackerScript -> `Nazwa::new(a, b)`.
        # Parametry konstruktora NIE sa opakowane w Box - uzytkownik podaje
        # zwykla (nie-boxowana) wartosc, boxowanie dzieje sie wewnatrz.
        params = ", ".join(
            f"{f.name}: {rust_type(f.type_, node.line, self.sigs.structs, self.sigs.enums, self.current_type_params) if f.type_ else 'i64'}"
            for f in node.fields
        )
        boxed = self.boxed_struct_fields.get(node.name, {})
        init_parts = []
        for f in node.fields:
            kind = boxed.get(f.name)
            if kind is None:
                init_parts.append(f.name)
            elif kind == "option":
                init_parts.append(f"{f.name}: {f.name}.map(Box::new)")
            else:
                init_parts.append(f"{f.name}: Box::new({f.name})")
        fields_init = ", ".join(init_parts)
        self.emit(f"impl{gen_head} {node.name}{gen_head} {{")
        self.indent += 1
        self.emit(f"pub fn new({params}) -> Self {{")
        self.indent += 1
        self.emit(f"{node.name} {{ {fields_init} }}")
        self.indent -= 1
        self.emit("}")
        self.indent -= 1
        self.emit("}")
        self.emit("")
        self.current_type_params = set()

    def _variant_field_rust_type(self, owner_name: str, variant_name: str, idx: int, t: A.TypeRef, line: int) -> str:
        flags = self.boxed_variant_fields.get((owner_name, variant_name))
        kind = flags[idx] if flags and idx < len(flags) else None
        if kind == "option" and t.generic is not None:
            inner = rust_type(t.generic, line, self.sigs.structs, self.sigs.enums, self.current_type_params)
            return f"Option<Box<{inner}>>"
        if kind == "direct":
            inner = rust_type(t, line, self.sigs.structs, self.sigs.enums, self.current_type_params)
            return f"Box<{inner}>"
        return rust_type(t, line, self.sigs.structs, self.sigs.enums, self.current_type_params)

    def gen_enum(self, node: A.EnumDecl):
        self.current_type_params = set(node.type_params)
        gen_head = f"<{', '.join(node.type_params)}>" if node.type_params else ""
        self.emit("#[derive(Debug, Clone, PartialEq)]")
        self.emit(f"pub enum {node.name}{gen_head} {{")
        self.indent += 1
        for v in node.variants:
            if v.fields:
                args = ", ".join(
                    self._variant_field_rust_type(node.name, v.name, i, t, node.line)
                    for i, t in enumerate(v.fields)
                )
                self.emit(f"{v.name}({args}),")
            else:
                self.emit(f"{v.name},")
        self.indent -= 1
        self.emit("}")
        self.emit("")
        self.current_type_params = set()

    def gen_impl(self, node: A.ImplDecl):
        self.current_type_params = set(node.type_params)
        gen_head = f"<{', '.join(node.type_params)}>" if node.type_params else ""
        self.emit(f"impl{gen_head} {node.struct_name}{gen_head} {{")
        self.indent += 1
        for m in node.methods:
            self.gen_method(node.struct_name, m)
        self.indent -= 1
        self.emit("}")
        self.emit("")
        self.current_type_params = set()

    def gen_method(self, struct_name: str, node: A.FunDecl):
        """Metoda z bloku `impl` - jak `gen_fun`, ale pierwszy parametr
        `self` staje sie odbiornikiem `&self`/`&mut self` (mutowalnosc
        wnioskowana z ciala metody, patrz `_compute_method_mut_params`),
        a pozostale parametry uzywaja tej samej logiki auto-ref co wolne
        funkcje (`_is_refable`). `self.current_type_params` jest juz
        ustawiony przez `gen_impl` (parametry generyczne struct sa w
        zasiegu we wszystkich jej metodach)."""
        if not node.params or node.params[0].name != "self":
            raise CodegenError(
                f"metoda '{node.name}' w 'impl {struct_name}' musi miec 'self' jako pierwszy parametr",
                node.line,
            )
        self.env = TypeEnv(self.sigs)
        self.env.declare("self", A.TypeRef(name=struct_name, line=node.line))
        key = f"{struct_name}::{node.name}"
        mutated = self.method_mut_params.get(key, set())

        params_parts = ["&mut self" if "self" in mutated else "&self"]
        for p in node.params[1:]:
            if p.name == "self":
                raise CodegenError("'self' moze byc tylko pierwszym parametrem metody", node.line)
            self.env.declare(p.name, p.type_)
            base = rust_type(p.type_, node.line, self.sigs.structs, self.sigs.enums, self.current_type_params)
            if self._is_refable(p.type_):
                prefix = "&mut " if p.name in mutated else "&"
                params_parts.append(f"{p.name}: {prefix}{base}")
            else:
                params_parts.append(f"{p.name}: {base}")
        params = ", ".join(params_parts)
        ret = f" -> {rust_type(node.ret_type, node.line, self.sigs.structs, self.sigs.enums, self.current_type_params)}" if node.ret_type else ""
        for doc in getattr(node, "_leading_doc_comments", None) or []:
            self.emit(f"/// {doc}")
        self.current_ret_type = node.ret_type
        self.emit(f"pub fn {node.name}({params}){ret} {{")
        self.indent += 1
        for s in node.body:
            self.gen_stmt(s)
        self.indent -= 1
        self.emit("}")
        self.emit("")
        self.current_ret_type = None


    def _is_refable(self, type_: A.TypeRef | None) -> bool:
        if type_ is None:
            return False
        return type_.name in self.sigs.structs or type_.name in self.sigs.enums or type_.name in ("Str", "List")

    def _param_type_str(self, fun_name: str, param: A.Param, line: int) -> str:
        """Typ parametru w sygnaturze Rusta. Dla parametrow typu struct/
        List/Str (wszystkie non-Copy w Ruscie) automatycznie dobiera
        `&mut T` (jesli funkcja mutuje) albo `&T` (tylko odczyt) -
        zamiast przenoszenia wlasnosci, ktore uniemozliwiloby ponowne
        uzycie zmiennej po jednym wywolaniu (np. `f(xs); g(xs);`)."""
        base = rust_type(param.type_, line, self.sigs.structs, self.sigs.enums, self.current_type_params)
        if self._is_refable(param.type_):
            mutated = self.mut_params.get(fun_name, set())
            return f"&mut {base}" if param.name in mutated else f"&{base}"
        return base

    def _call_arg_str(self, fun_name: str, index: int, arg_node) -> str:
        """Generuje wyrazenie argumentu wywolania, dodajac `&`/`&mut`
        jesli odpowiadajacy parametr znanej funkcji jest referencja
        (patrz _param_type_str)."""
        fn = self.sigs.functions.get(fun_name)
        rendered = self.gen_expr(arg_node)
        if fn is None or index >= len(fn.params):
            return rendered
        p = fn.params[index]
        if self._is_refable(p.type_):
            mutated = self.mut_params.get(fun_name, set())
            prefix = "&mut " if p.name in mutated else "&"
            return f"{prefix}{rendered}"
        return rendered

    def gen_fun(self, node: A.FunDecl):
        self.current_type_params = set(node.type_params)
        gen_head = f"<{', '.join(node.type_params)}>" if node.type_params else ""
        self.env = TypeEnv(self.sigs)
        for p in node.params:
            self.env.declare(p.name, p.type_)

        params = ", ".join(
            f"{p.name}: {self._param_type_str(node.name, p, node.line)}" for p in node.params
        )
        ret = f" -> {rust_type(node.ret_type, node.line, self.sigs.structs, self.sigs.enums, self.current_type_params)}" if node.ret_type else ""
        self.current_ret_type = node.ret_type
        self.emit(f"pub fn {node.name}{gen_head}({params}){ret} {{")
        self.indent += 1
        for s in node.body:
            self.gen_stmt(s)
        self.indent -= 1
        self.emit("}")
        self.emit("")
        self.current_type_params = set()
        self.current_ret_type = None

    # -- statements ---------------------------------------------------

    def _gen_return_expr(self, value) -> str:
        """Jak `gen_expr`, ale dla wartosci `end`/`return`: gdy zwracamy
        DOSTEP DO POLA (`self.pole`, `param.pole.pod_pole`, ...) i typ
        zwracany funkcji/metody jest non-Copy (struct/enum/List/Str),
        doklejamy `.clone()`. `self`/refable parametry w naszym codegen
        ZAWSZE sa `&`/`&mut` (patrz `_is_refable`), wiec `return
        self.pole;` probowaloby przeniesc wlasnosc pola spod referencji -
        Rust to odrzuca (E0507: cannot move out of ... behind a shared
        reference). Heurystyka jest bezpieczna w obie strony: doklejenie
        `.clone()` nigdy nie jest bledem, najwyzej niepotrzebnym
        kosztem gdy baza wyrazenia byla juz wlasnoscia lokalna."""
        rendered = self.gen_expr(value)
        if isinstance(value, A.Attr) and self._is_refable(self.current_ret_type):
            return f"{rendered}.clone()"
        return rendered

    def gen_stmt(self, node):
        if isinstance(node, A.LetStmt):
            type_ = node.type_
            if type_ is None and node.value is not None:
                type_ = infer_expr_type(node.value, self.env)
            if self.env is not None:
                self.env.declare(node.name, type_)
            skip_hint = type_ is not None and (
                (type_.name in self.sigs.structs and type_.name not in self.local_structs)
                or _contains_any(type_)
                or (type_.generic is None and (
                    getattr(self.sigs.structs.get(type_.name), "type_params", None)
                    or getattr(self.sigs.enums.get(type_.name), "type_params", None)
                ))
            )
            hint = f": {rust_type(type_, node.line, self.sigs.structs, self.sigs.enums, self.current_type_params)}" if (type_ and not skip_hint) else ""
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
            self.emit(f"return {self._gen_return_expr(node.value)};" if node.value is not None else "return;")
            return
        if isinstance(node, A.BreakStmt):
            self.emit("break;")
            return
        if isinstance(node, A.ContinueStmt):
            self.emit("continue;")
            return
        if isinstance(node, A.ManualBlock):
            self.emit("unsafe {")
            self.indent += 1
            for s in node.body:
                self.gen_stmt(s)
            self.indent -= 1
            self.emit("}")
            return
        if isinstance(node, A.GcPragma):
            self.emit(f"// gc:use::{node.mode} - bez znaczenia w Rust (wlasnosc/pozyczanie)")
            return
        if isinstance(node, A.StructDecl):
            raise CodegenError("'struct' musi byc zadeklarowany na najwyzszym poziomie pliku, nie wewnatrz funkcji", node.line)
        if isinstance(node, A.MatchStmt):
            self.gen_match(node)
            return
        if isinstance(node, A.ExprStmt):
            self.gen_expr_stmt(node.expr)
            return
        raise CodegenError(f"nieobslugiwana instrukcja: {node!r}", getattr(node, "line", 0))

    def _pattern_str(self, variant: str, binds: list[str], line: int) -> str:
        """Renderuje wzorzec jednej galezi `match` w Ruscie. `_` to
        wildcard. `Some/None/Ok/Err` sa natywnymi wariantami Rusta (bez
        kwalifikacji), user-zdefiniowane warianty enum sa kwalifikowane
        `NazwaEnuma::Wariant` (patrz `self.variant_to_enum`, budowane w
        `_build_variant_registry` - zaklada unikalnosc nazw wariantow w
        calym programie, patrz docs/ROADMAP.md)."""
        if variant == "_":
            if binds:
                raise CodegenError("wildcard '_' w 'match' nie moze miec bindowanych zmiennych", line)
            return "_"
        args = f"({', '.join(binds)})" if binds else ""
        if variant in self._BUILTIN_VARIANTS:
            return f"{variant}{args}"
        if variant in self.variant_to_enum:
            return f"{self.variant_to_enum[variant]}::{variant}{args}"
        raise CodegenError(
            f"nieznany wariant '{variant}' w 'match' (nie jest to Some/None/Ok/Err ani "
            f"wariant zadnego zadeklarowanego 'enum')",
            line,
        )

    def gen_match(self, node: A.MatchStmt):
        self.emit(f"match {self.gen_expr(node.subject)} {{")
        self.indent += 1
        for arm in node.arms:
            pattern = self._pattern_str(arm.variant, arm.binds, node.line)
            self.emit(f"{pattern} => {{")
            self.indent += 1
            prev_types = {}
            for b in arm.binds:
                prev_types[b] = (self.env.is_declared(b), self.env.lookup(b)) if self.env else (False, None)
                if self.env:
                    self.env.declare(b, None)
            for s in arm.body:
                self.gen_stmt(s)
            if self.env:
                for b, (was_declared, prev_t) in prev_types.items():
                    if was_declared:
                        self.env.declare(b, prev_t)
                    else:
                        self.env.vars.pop(b, None)
            self.indent -= 1
            self.emit("}")
        self.indent -= 1
        self.emit("}")

    def gen_expr_stmt(self, e):
        if isinstance(e, A.StringLit) and getattr(e, "_is_doc", False):
            self.emit(f"/// {e.value}")
            return
        if isinstance(e, A.Call) and isinstance(e.callee, A.Ident) and e.callee.name == "__direct__":
            self.gen_direct(int(e.args[0].value))
            return
        if isinstance(e, A.Call) and isinstance(e.callee, A.Ident) and e.callee.name == "log":
            self.emit(self._gen_log(e.args) + ";")
            return
        self.emit(self.gen_expr(e) + ";")

    def gen_direct(self, idx: int):
        """direct[ ... ] = surowy kod PYTHONA, wykonywany przez wbudowany
        interpreter (PyO3 `Python::with_gil`) - Rust jest hostem."""
        self.needs_pyo3 = True
        raw = self.direct_blocks.get(idx, "")
        self.emit("{")
        self.indent += 1
        self.emit("Python::with_gil(|py| -> PyResult<()> {")
        self.indent += 1
        self.emit(f"py.run({_python_raw_string(raw)}, None, None)?;")
        self.emit("Ok(())")
        self.indent -= 1
        self.emit('}).expect("direct[ ... ] (Python) block failed");')
        self.indent -= 1
        self.emit("}")

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
            if self.env is not None and not self.env.is_declared(node.name):
                if node.name == "None":
                    return "None"
                if node.name in self.variant_arity and self.variant_arity[node.name] == 0:
                    return f"{self.variant_to_enum[node.name]}::{node.name}"
            return node.name
        if isinstance(node, A.ListLit):
            return "vec![" + ", ".join(self.gen_expr(i) for i in node.items) + "]"
        if isinstance(node, A.UnaryOp):
            if node.op == "not":
                return f"!({self.gen_expr(node.operand)})"
            return f"{node.op}({self.gen_expr(node.operand)})"
        if isinstance(node, A.BinOp):
            op = {"and": "&&", "or": "||"}.get(node.op, node.op)
            if node.op == "+":
                lt = infer_expr_type(node.left, self.env) if self.env else None
                rt = infer_expr_type(node.right, self.env) if self.env else None
                is_str = (lt is not None and lt.name == "Str") or (rt is not None and rt.name == "Str") \
                    or isinstance(node.left, A.StringLit) or isinstance(node.right, A.StringLit)
                is_list = (lt is not None and lt.name == "List") or (rt is not None and rt.name == "List") \
                    or isinstance(node.left, A.ListLit) or isinstance(node.right, A.ListLit)
                if is_list:
                    # Bug znaleziony i naprawiony: `[a, b].concat()` wymaga
                    # DOKLADNIE tego samego typu obu elementow tablicy - psulo
                    # sie gdy jeden operand byl referencja (&Vec<T>, np.
                    # parametr metody/funkcji auto-ref'owany) a drugi
                    # wlasnoscia (Vec<T>, np. self.pole). `.iter().cloned()`
                    # dziala jednakowo na Vec<T> i &Vec<T> (auto-deref),
                    # wiec nie zalezy juz od tego ktory operand jest
                    # referencja. Test: test_list_concat_handles_mixed_ref_and_owned_operands.
                    left = self.gen_expr(node.left)
                    right = self.gen_expr(node.right)
                    return (
                        f"{left}.iter().cloned().chain({right}.iter().cloned())"
                        ".collect::<Vec<_>>()"
                    )
                if is_str:
                    return f"format!(\"{{}}{{}}\", {self.gen_expr(node.left)}, {self.gen_expr(node.right)})"
            return f"({self.gen_expr(node.left)} {op} {self.gen_expr(node.right)})"
        if isinstance(node, A.Attr):
            return f"{self.gen_expr(node.target)}.{node.name}"
        if isinstance(node, A.Index):
            base = f"{self.gen_expr(node.target)}[{self.gen_expr(node.index)} as usize]"
            # `xs[i]` dla non-Copy elementu (struct/enum/List/Str/Dict) nie
            # da sie po prostu "wziac" jako wartosc - `Index` w Ruscie
            # zwraca miejsce (`&T` pod maska), a `let x = xs[i];` dla
            # takiego T probowaloby przeniesc wlasnosc z indeksowania,
            # co Rust odrzuca (ta sama klasa bledu co E0507 dla
            # `return self.pole` - patrz `_gen_return_expr`). Bezpieczna
            # heurystyka: klonuj zawsze dla non-Copy elementow (nigdy nie
            # psuje poprawnosci, czasem to niepotrzebny klon gdy wynik i
            # tak jest tylko podstawa do `.pole`/`.metoda()` dalej).
            elem_t = None
            if self.env is not None:
                target_t = infer_expr_type(node.target, self.env)
                if target_t is not None and target_t.name == "List":
                    elem_t = target_t.generic
            if elem_t is not None and self._is_refable(elem_t):
                return f"{base}.clone()"
            return base
        if isinstance(node, A.Cast):
            if node.type_.name == "Str":
                # Rust `as` NIE wspiera konwersji numeryczny/Bool -> String
                # (dziala tylko miedzy typami numerycznymi/wskaznikami) -
                # `x as Str` musi wygenerowac `.to_string()`, nie
                # niepoprawne `x as String`. Bug znaleziony przy pisaniu
                # bootstrap/hackerc-self/ast_nodes.hcs.
                return f"({self.gen_expr(node.target)}).to_string()"
            return f"({self.gen_expr(node.target)} as {rust_type(node.type_, getattr(node, 'line', 0), self.sigs.structs, self.sigs.enums, self.current_type_params)})"
        if isinstance(node, A.TryOp):
            return f"({self.gen_expr(node.target)})?"
        if isinstance(node, A.Call):
            if isinstance(node.callee, A.Ident) and node.callee.name == "log":
                return self._gen_log(node.args)
            if isinstance(node.callee, A.Ident) and node.callee.name == "read_file" and len(node.args) == 1:
                # get <std:io> - read_file(sciezka) -> Result<Str, Str>.
                # std::fs::read_to_string zwraca io::Result<String>=
                # Result<String, std::io::Error> - .map_err() dostosowuje
                # typ bledu do Str, zeby pasowal do deklarowanego Result<T,Str>.
                path_expr = self.gen_expr(node.args[0])
                return f"std::fs::read_to_string(&{path_expr}).map_err(|e| e.to_string())"
            if isinstance(node.callee, A.Ident) and node.callee.name == "write_file" and len(node.args) == 2:
                # get <std:io> - write_file(sciezka, tresc) -> Result<Void, Str>.
                path_expr = self.gen_expr(node.args[0])
                content_expr = self.gen_expr(node.args[1])
                return f"std::fs::write(&{path_expr}, {content_expr}).map_err(|e| e.to_string())"
            if isinstance(node.callee, A.Ident) and node.callee.name in ("some", "ok", "err"):
                rust_name = {"some": "Some", "ok": "Ok", "err": "Err"}[node.callee.name]
                args = ", ".join(self.gen_expr(a) for a in node.args)
                return f"{rust_name}({args})"
            if isinstance(node.callee, A.Ident) and node.callee.name == "none" and not node.args:
                return "None"
            if isinstance(node.callee, A.Ident) and node.callee.name in self.variant_to_enum:
                enum_name = self.variant_to_enum[node.callee.name]
                box_flags = self.boxed_variant_fields.get((enum_name, node.callee.name))
                rendered = []
                for i, a in enumerate(node.args):
                    expr = self.gen_expr(a)
                    kind = box_flags[i] if box_flags and i < len(box_flags) else None
                    if kind == "option":
                        expr = f"{expr}.map(Box::new)"
                    elif kind == "direct":
                        expr = f"Box::new({expr})"
                    rendered.append(expr)
                args = ", ".join(rendered)
                return f"{enum_name}::{node.callee.name}({args})"
            if (
                isinstance(node.callee, A.Attr)
                and node.callee.name == "len"
                and not node.args
            ):
                # Vec::len() zwraca usize, ale caly system typow
                # HackerScript zaklada Int==i64 - bez rzutowania
                # jakakolwiek arytmetyka/porownanie z wynikiem .len()
                # nie skompilowalaby sie (usize vs i64).
                return f"({self.gen_expr(node.callee.target)}.len() as i64)"
            if isinstance(node.callee, A.Ident) and node.callee.name == "dict" and not node.args:
                # get <std:...> Dict<K,V> - konstruktor. Nazwa metody
                # odczytu to '.fetch()' (nie '.get()' jak w Rust) bo
                # 'get' jest zarezerwowanym slowem kluczowym (get <...>)
                # i nie da sie go uzyc jako nazwy metody - patrz parser.py.
                return "std::collections::HashMap::new()"
            if isinstance(node.callee, A.Attr) and node.callee.name == "char_at" and len(node.args) == 1 and self.env is not None:
                target_t = infer_expr_type(node.callee.target, self.env)
                if target_t is not None and target_t.name == "Str":
                    idx_expr = self.gen_expr(node.args[0])
                    target_expr = self.gen_expr(node.callee.target)
                    # Str to UTF-8 - indeksowanie bajtowe (jak w Ruscie
                    # 's[i]') mogloby przeciac znak wielobajtowy w polowie,
                    # wiec indeksujemy po ZNAKACH (.chars()), nie bajtach.
                    return f"({target_expr}.chars().nth({idx_expr} as usize).map(|c| c.to_string()).unwrap_or_default())"
            if isinstance(node.callee, A.Attr) and node.callee.name == "slice" and len(node.args) == 2 and self.env is not None:
                target_t = infer_expr_type(node.callee.target, self.env)
                if target_t is not None and target_t.name == "Str":
                    start_expr = self.gen_expr(node.args[0])
                    end_expr = self.gen_expr(node.args[1])
                    target_expr = self.gen_expr(node.callee.target)
                    return (
                        f"({target_expr}.chars().skip({start_expr} as usize)"
                        f".take((({end_expr}) - ({start_expr})) as usize).collect::<String>())"
                    )
            if isinstance(node.callee, A.Attr) and node.callee.name in ("fetch", "contains", "remove") and len(node.args) == 1 and self.env is not None:
                target_t = infer_expr_type(node.callee.target, self.env)
                if target_t is not None and target_t.name == "Dict":
                    key_expr = self.gen_expr(node.args[0])
                    target_expr = self.gen_expr(node.callee.target)
                    if node.callee.name == "fetch":
                        return f"{target_expr}.get(&{key_expr}).cloned()"
                    if node.callee.name == "contains":
                        return f"{target_expr}.contains_key(&{key_expr})"
                    return f"{target_expr}.remove(&{key_expr})"
            if isinstance(node.callee, A.Attr) and self.env is not None:
                # Wywolanie metody (obj.method(args)) zadeklarowanej w
                # 'impl' - target_t pozwala znalezc sygnature metody w
                # methods_registry, zeby argumenty inne niz 'self'
                # dostaly ta sama auto-referencje `&`/`&mut` co w wolnych
                # funkcjach (patrz _call_arg_str). Bez znanego statycznego
                # typu targetu (np. wynik innego wywolania funkcji) ta
                # optymalizacja jest pomijana - argumenty ida bez zmian
                # (ograniczenie bootstrapu, patrz docs/ROADMAP.md).
                target_t = infer_expr_type(node.callee.target, self.env)
                if target_t is not None:
                    m = self.methods_registry.get((target_t.name, node.callee.name))
                    if m is not None:
                        method_params = [p for p in m.params if p.name != "self"]
                        mutated = self.method_mut_params.get(f"{target_t.name}::{m.name}", set())
                        rendered_args = []
                        for i, a in enumerate(node.args):
                            rendered = self.gen_expr(a)
                            if i < len(method_params) and self._is_refable(method_params[i].type_):
                                prefix = "&mut " if method_params[i].name in mutated else "&"
                                rendered = f"{prefix}{rendered}"
                            rendered_args.append(rendered)
                        args_str = ", ".join(rendered_args)
                        return f"{self.gen_expr(node.callee.target)}.{node.callee.name}({args_str})"
            if isinstance(node.callee, A.Ident) and self.sigs and node.callee.name in self.sigs.structs:
                args = ", ".join(self.gen_expr(a) for a in node.args)
                return f"{node.callee.name}::new({args})"
            if isinstance(node.callee, A.Ident) and self.sigs and node.callee.name in self.sigs.functions:
                args = ", ".join(self._call_arg_str(node.callee.name, i, a) for i, a in enumerate(node.args))
                return f"{node.callee.name}({args})"
            args = ", ".join(self.gen_expr(a) for a in node.args)
            return f"{self.gen_expr(node.callee)}({args})"
        raise CodegenError(f"nieobslugiwane wyrazenie: {node!r}", getattr(node, "line", 0))


def generate(
    prog: A.Program,
    direct_blocks: dict[int, str] | None = None,
    module_name: str = "module",
    extra_functions: dict | None = None,
    extra_structs: dict | None = None,
    extra_enums: dict | None = None,
    extra_mut_params: dict | None = None,
    extra_methods: dict | None = None,
    extra_method_mut_params: dict | None = None,
):
    gen = CodeGen(direct_blocks=direct_blocks, module_name=module_name)
    gen.sigs = Signatures(prog)
    gen.mut_params = _compute_mut_params(prog)
    gen.local_structs = set(gen.sigs.structs.keys())
    if extra_functions:
        gen.sigs.functions = {**extra_functions, **gen.sigs.functions}
    if extra_structs:
        gen.sigs.structs = {**extra_structs, **gen.sigs.structs}
    if extra_enums:
        gen.sigs.enums = {**extra_enums, **gen.sigs.enums}
    if extra_mut_params:
        gen.mut_params = {**extra_mut_params, **gen.mut_params}
    gen._extra_methods = extra_methods
    gen._extra_method_mut_params = extra_method_mut_params
    rust_code = gen.gen_program(prog, _skip_sig_setup=True)
    return rust_code, gen.needs_pyo3

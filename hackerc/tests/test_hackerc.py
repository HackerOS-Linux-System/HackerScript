import contextlib
import sys
import tempfile
import textwrap
from pathlib import Path

try:
    import pytest  # type: ignore

    HAVE_PYTEST = True
except ImportError:
    HAVE_PYTEST = False

    class _ExcInfo:
        def __init__(self):
            self.value = None

    class _FakePytest:
        @staticmethod
        @contextlib.contextmanager
        def raises(exc_type):
            info = _ExcInfo()
            try:
                yield info
            except exc_type as exc:
                info.value = exc
                return
            else:
                raise AssertionError(f"oczekiwano wyjatku {exc_type}, nic nie rzucono")

        @staticmethod
        def skip(reason):
            raise _Skip(reason)

    class _Skip(Exception):
        pass

    pytest = _FakePytest()  # type: ignore

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from hackerc.transpiler import transpile_source, transpile_source_full, TranspileError
from hackerc.codegen import CodegenError
from hackerc.parser import parse, ParseError
from hackerc.lexer import LexError, tokenize
from hackerc.typecheck import check_program
from hackerc.formatter import format_source
from hackerc.project import build_project, collect_project_signatures, flat_module_name, ProjectError


# ---- lexer / indeksowanie vs blok ------------------------------------------

def test_tight_bracket_distinguishes_index_from_block():
    toks = tokenize("if x [\n]")
    open_tok = [t for t in toks if t.value == "["][0]
    assert open_tok.tight is False

    toks = tokenize("x[0]")
    open_tok = [t for t in toks if t.value == "["][0]
    assert open_tok.tight is True


def test_indexing_generates_rust_index_expr():
    src = textwrap.dedent("""
        fun main() [
            let xs = [1, 2, 3]
            if xs[0] < xs[1] [
                log("ok")
            ]
            end
        ]
    """)
    code = transpile_source(src)
    assert "xs[0" in code and "as usize]" in code


# ---- podstawowa transpilacja do Rusta -----------------------------------

def test_hello_world_rust_structure():
    src = 'fun main() [\n    log("hi")\n    end\n]\n'
    code = transpile_source(src)
    assert "pub fn main()" in code
    assert 'println!("{}", "hi".to_string());' in code


def test_recursion_and_types():
    src = textwrap.dedent("""
        fun fib(n: Int) -> Int [
            if n <= 1 [
                end n
            ]
            end fib(n - 1) + fib(n - 2)
        ]
    """)
    code = transpile_source(src)
    assert "pub fn fib(n: i64) -> i64 {" in code
    assert "fib((n - 1))" in code


def test_struct_generates_rust_struct_and_constructor():
    src = textwrap.dedent("""
        struct Point [
            x: Int,
            y: Int
        ]
        fun main() [
            let p = Point(1, 2)
            log(p.x, p.y)
            end
        ]
    """)
    code = transpile_source(src)
    assert "pub struct Point {" in code
    assert "pub x: i64," in code
    assert "impl Point {" in code
    assert "pub fn new(x: i64, y: i64) -> Self {" in code
    assert "Point::new(1, 2)" in code


def test_direct_block_embeds_python_via_pyo3():
    src = textwrap.dedent("""
        fun main() [
            direct [
                print("raw python")
            ]
            end
        ]
    """)
    result = transpile_source_full(src)
    assert result.needs_pyo3 is True
    assert "use pyo3::prelude::*;" in result.rust_code
    assert "Python::with_gil" in result.rust_code
    assert 'print("raw python")' in result.rust_code


def test_manual_block_is_real_unsafe():
    src = textwrap.dedent("""
        fun main() [
            manual [
                let x = 1
            ]
            end
        ]
    """)
    code = transpile_source(src)
    assert "unsafe {" in code


def test_extern_fun_generates_ffi_block():
    src = 'extern "m" fun sqrt_native(x: Float) -> Float\n'
    code = transpile_source(src)
    assert '#[link(name = "m")]' in code
    assert 'extern "C" {' in code
    assert "pub fn sqrt_native(x: f64) -> f64;" in code


def test_any_type_is_rejected():
    src = textwrap.dedent("""
        fun broken(a: Any) -> Int [
            end 0
        ]
    """)
    with pytest.raises(TranspileError):
        transpile_source(src)


# ---- struct-by-reference (bezpieczenstwo pamieci: brak przenoszenia -------
# ---- wlasnosci przy wielokrotnym uzyciu tej samej zmiennej struct) --------

def test_struct_param_passed_by_mut_reference_when_mutated():
    src = textwrap.dedent("""
        struct Counter [
            value: Int
        ]
        fun bump(c: Counter) [
            c.value = c.value + 1
            end
        ]
        fun main() [
            let c = Counter(0)
            bump(c)
            bump(c)
            end
        ]
    """)
    code = transpile_source(src)
    assert "pub fn bump(c: &mut Counter)" in code
    assert "bump(&mut c);" in code
    assert code.count("bump(&mut c);") == 2  # oba wywolania, bez przenoszenia wlasnosci


def test_struct_param_passed_by_shared_reference_when_read_only():
    src = textwrap.dedent("""
        struct Point [
            x: Int
        ]
        fun read_x(p: Point) -> Int [
            end p.x
        ]
    """)
    code = transpile_source(src)
    assert "pub fn read_x(p: &Point) -> i64" in code


def test_list_concatenation_uses_concat_not_plus_operator():
    src = textwrap.dedent("""
        fun main() [
            let xs = [1, 2]
            let ys = xs + [3]
            log(ys)
            end
        ]
    """)
    code = transpile_source(src)
    assert ".iter().cloned().chain(" in code
    assert ".collect::<Vec<_>>()" in code
    assert "xs + " not in code  # nie generujemy Vec + Vec (nie istnieje w Rust)


def test_list_concat_handles_mixed_ref_and_owned_operands():
    """Bug znaleziony podczas testow impl/self: `[a, b].concat()` wymagalo
    IDENTYCZNEGO typu obu elementow - psulo sie gdy jeden operand byl
    referencja (np. parametr metody `xs: &Vec<i64>`) a drugi wlasnoscia
    (`self.items: Vec<i64>`). `.iter().cloned().chain()` dziala jednakowo
    na Vec<T> i &Vec<T>, wiec mieszanka referencja/wartosc juz nie psuje
    kompilacji."""
    src = textwrap.dedent(
        """
        struct Bag [
            items: List<Int>
        ]

        impl Bag [
            fun add_all(self, xs: List<Int>) [
                self.items = self.items + xs
            ]
        ]
        """
    )
    code = transpile_source(src)
    assert "fn add_all(&mut self, xs: &Vec<i64>)" in code
    assert "self.items.iter().cloned().chain(xs.iter().cloned()).collect::<Vec<_>>()" in code


def test_not_equal_operator_not_confused_with_comment():
    # Realny bug znaleziony podczas pisania libs/std/cybersecurity/:
    # '!=' jako operator nierownosci bylo mylone z '!=...=!' (komentarz
    # wieloliniowy) i z '!' (komentarz jednoliniowy - warunek byl zawsze
    # prawdziwy przez blad 'x and False').
    src = textwrap.dedent("""
        fun neq(a: Int, b: Int) -> Bool [
            end a != b
        ]
    """)
    code = transpile_source(src)
    assert "(a != b)" in code


# ---- typecheck (bez zmian koncepcyjnych - dziala na poziomie AST) --------

def test_typecheck_catches_wrong_arg_count():
    src = textwrap.dedent("""
        fun add(a: Int, b: Int) -> Int [
            end a + b
        ]
        fun main() [
            let r = add(1, 2, 3)
            end
        ]
    """)
    diags = check_program(parse(src))
    assert "E0001" in {d.code for d in diags}


def test_typecheck_catches_missing_return_value():
    src = textwrap.dedent("""
        fun broken() -> Int [
            log("no end here")
        ]
    """)
    diags = check_program(parse(src))
    assert "E0002" in {d.code for d in diags}


def test_typecheck_catches_type_mismatch():
    src = textwrap.dedent("""
        fun main() [
            let bad: Str = 5
            end
        ]
    """)
    diags = check_program(parse(src))
    assert "E0005" in {d.code for d in diags}


def test_typecheck_no_false_positive_on_self_reference_loop():
    src = textwrap.dedent("""
        fun main() [
            let i = 0
            while i < 3 [
                i = i + 1
            ]
            end
        ]
    """)
    diags = check_program(parse(src))
    assert not any(d.code == "W0001" and "i" in d.message for d in diags)


# ---- system modulow (get<std/core> -> realny crate Rust wielo-plikowy) ---

def test_module_system_generates_valid_cross_file_references():
    repo_root = Path(__file__).resolve().parents[2]
    entry = repo_root / "examples" / "module-demo" / "cmd" / "main.hcs"
    if not entry.exists():
        pytest.skip("examples/module-demo nie istnieje w tym checkout")

    out_dir = Path("/tmp/hackerc_test_moddemo_rs")
    if out_dir.exists():
        import shutil

        shutil.rmtree(out_dir)
    result = build_project(entry, out_dir)

    assert not result.warnings, result.warnings
    assert result.cargo_toml.exists()
    assert any(flat_module_name("core", "memory", "arena") == k for k in result.module_files)

    main_rs = result.main_rs.read_text()
    assert "mod _hks_core_memory_arena;" in main_rs
    assert "use crate::_hks_core_memory_arena::" in main_rs
    # &mut/& musza byc poprawnie dodane NAWET dla funkcji zdefiniowanej w
    # INNYM pliku - to jest sedno fazy 1 (discovery) w project.py.
    assert "&mut a" in main_rs
    assert "&a" in main_rs


def test_flat_module_name_matches_codegen_convention():
    name = flat_module_name("core", "memory", "arena")
    assert name == "_hks_core_memory_arena"


# ---- diagnostyka (lekser/parser) -----------------------------------------

def test_lex_error_reports_line_and_col():
    with pytest.raises(LexError) as exc_info:
        tokenize('let s = "niezamkniety')
    assert exc_info.value.line == 1
    assert exc_info.value.col > 0


def test_parse_error_reports_line():
    with pytest.raises(ParseError):
        parse("fun main( [\n]\n")


# ---- formatter ------------------------------------------------------------

def test_formatter_is_idempotent():
    src = textwrap.dedent("""
        fun add(a:Int,b:Int)->Int[
        end a+b
        ]
    """)
    once = format_source(src)
    twice = format_source(once)
    assert once == twice
    assert "fun add(a: Int, b: Int) -> Int [" in once


def test_formatter_handles_extern():
    src = 'extern "m" fun sqrt_native(x: Float) -> Float\n'
    formatted = format_source(src)
    assert 'extern "m" fun sqrt_native(x: Float) -> Float' in formatted


def test_formatter_preserves_line_comments():
    """Regresja dla docs/ROADMAP.md #8: komentarze jednoliniowe `!`
    bezposrednio przed instrukcja w programie/bloku nie moga byc gubione
    przez `hackerc fmt` (wczesniej lekser wycinal je BEZPOWROTNIE przed
    parsowaniem)."""
    src = textwrap.dedent(
        """
        ! top level comment
        fun main() [
            ! inner comment
            let x = 5
            log(x)
        ]
        """
    )
    formatted = format_source(src)
    assert "! top level comment" in formatted
    assert "! inner comment" in formatted
    # kolejnosc: komentarz tuz przed 'fun', wciety komentarz tuz przed 'let'
    assert formatted.index("! top level comment") < formatted.index("fun main")
    assert formatted.index("! inner comment") < formatted.index("let x")
    # idempotencja: sformatowanie wyniku ponownie nic nie zmienia
    assert format_source(formatted) == formatted


def test_formatter_preserves_generic_type_args():
    """Bug znaleziony podczas testow: formatter uzywal golego
    `type_.name`, ignorujac `TypeRef.generic`, wiec `List<Int>` wychodzilo
    z `hackerc fmt` jako `List` - cichy loss danych w typie."""
    src = "fun sum_list(xs: List<Int>) -> Int [\n    end 0\n]\n"
    formatted = format_source(src)
    assert "List<Int>" in formatted
    assert "xs: List [" not in formatted  # regresja: nie ucina '<Int>'


def test_not_equal_still_works_after_comment_tokenization():
    """`!=` (operator) nie moze zostac omylkowo potraktowany jako `!`
    (komentarz) + `=` teraz, gdy komentarze jednoliniowe sa realnymi
    tokenami zamiast bycia cicho usuwanymi przez lekser."""
    src = textwrap.dedent(
        """
        fun main() [
            let a = 1
            let b = 2
            if a != b [
                log("different")
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert "a != b" in rust


def test_enum_generates_rust_enum_with_tuple_variants():
    src = textwrap.dedent(
        """
        enum Shape [
            Circle(Float),
            Square(Float),
            Empty
        ]
        """
    )
    rust = transpile_source(src)
    assert "pub enum Shape {" in rust
    assert "Circle(f64)," in rust
    assert "Square(f64)," in rust
    assert "Empty," in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_match_generates_qualified_rust_match_arms():
    src = textwrap.dedent(
        """
        enum Shape [
            Circle(Float),
            Empty
        ]

        fun area(s: Shape) -> Float [
            match s [
                Circle(r) -> [
                    end r * r
                ]
                Empty -> [
                    end 0.0
                ]
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert "match s {" in rust
    assert "Shape::Circle(r) =>" in rust
    assert "Shape::Empty =>" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_match_wildcard_arm():
    src = textwrap.dedent(
        """
        enum Shape [
            Circle(Float),
            Square(Float),
            Empty
        ]

        fun describe(s: Shape) -> Str [
            match s [
                Circle(r) -> [
                    end "circle"
                ]
                _ -> [
                    end "other"
                ]
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert "_ => {" in rust


def test_option_and_result_map_to_native_rust_types():
    src = textwrap.dedent(
        """
        fun safe_div(a: Int, b: Int) -> Result<Int, Str> [
            if b == 0 [
                end err("dzielenie przez zero")
            ]
            end ok(a / b)
        ]

        fun find_first(xs: List<Int>) -> Option<Int> [
            if xs.len() == 0 [
                end none()
            ]
            end some(xs[0])
        ]
        """
    )
    rust = transpile_source(src)
    assert "-> Result<i64, String>" in rust
    assert "-> Option<i64>" in rust
    assert "Err(" in rust and ".to_string()" in rust
    assert "Ok((a / b))" in rust
    assert "None" in rust
    assert "Some(xs[0 as usize])" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_impl_methods_infer_self_mutability():
    """Metoda ktora mutuje pole (self.value = ...) musi dostac &mut self,
    metoda tylko-do-odczytu musi dostac &self - ta sama logika co przy
    parametrach struct w wolnych funkcjach (patrz _compute_method_mut_params)."""
    src = textwrap.dedent(
        """
        struct Counter [
            value: Int
        ]

        impl Counter [
            fun increment(self) [
                self.value = self.value + 1
            ]

            fun read(self) -> Int [
                end self.value
            ]
        ]

        fun main() [
            let c = Counter(0)
            c.increment()
            log(c.read())
        ]
        """
    )
    rust = transpile_source(src)
    assert "pub fn increment(&mut self)" in rust
    assert "pub fn read(&self) -> i64" in rust
    assert "impl Counter {" in rust  # blok z new() ORAZ osobny blok z metodami
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_impl_method_call_args_get_auto_ref():
    """Argumenty metody (poza self) typu struct/List/Str dostaja te sama
    auto-referencje &/&mut co parametry wolnych funkcji - zarowno w
    SYGNATURZE metody jak i w MIEJSCU WYWOLANIA (obj.method(arg))."""
    src = textwrap.dedent(
        """
        struct Bag [
            items: List<Int>
        ]

        impl Bag [
            fun add_all(self, xs: List<Int>) [
                self.items = self.items + xs
            ]
        ]

        fun main() [
            let b = Bag([])
            let extra = [1, 2, 3]
            b.add_all(extra)
        ]
        """
    )
    rust = transpile_source(src)
    assert "fn add_all(&mut self, xs: &Vec<i64>)" in rust
    assert "b.add_all(&extra);" in rust


def test_impl_requires_self_as_first_param():
    src = textwrap.dedent(
        """
        struct Counter [
            value: Int
        ]

        impl Counter [
            fun broken(x: Int) [
                log(x)
            ]
        ]
        """
    )
    with pytest.raises(TranspileError):
        transpile_source(src)


def test_io_stdlib_module_typechecks_and_uses_result():
    """libs/std/lib/io.hcs (read_file_or/file_readable/write_file_ok,
    zbudowane na wbudowanych read_file/write_file zwracajacych
    Result<Str,Str>/Result<Void,Str>) musi przechodzic typecheck bez
    bledow i faktycznie generowac wywolania std::fs w Ruscie."""
    io_path = Path(__file__).resolve().parent.parent.parent / "libs" / "std" / "lib" / "io.hcs"
    src = io_path.read_text(encoding="utf-8")
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]
    rust = transpile_source(src)
    assert "std::fs::read_to_string" in rust
    assert "std::fs::write" in rust
    # Str to typ non-Copy (jak struct/List), wiec parametry `path`/`tresc`
    # dostaja te sama auto-referencje co gdziekolwiek indziej w jezyku -
    # patrz test_struct_param_passed_by_shared_reference_when_read_only.
    assert "fn read_file_or(path: &String, domyslna: &String) -> String" in rust
    assert "fn file_readable(path: &String) -> bool" in rust
    assert "fn write_file_ok(path: &String, tresc: &String) -> bool" in rust


def test_generic_struct_and_fun_compile_to_rust_generics():
    src = textwrap.dedent(
        """
        struct Box<T> [
            value: T
        ]

        impl Box<T> [
            fun read(self) -> T [
                end self.value
            ]
        ]

        fun identity<T>(x: T) -> T [
            end x
        ]

        fun main() [
            let b = Box(5)
            log(b.read())
            log(identity(10))
        ]
        """
    )
    rust = transpile_source(src)
    assert "pub struct Box<T> {" in rust
    assert "impl<T> Box<T> {" in rust
    assert "pub fn read(&self) -> T {" in rust
    assert "pub fn identity<T>(x: T) -> T {" in rust
    # let bez adnotacji generycznej - Box (nie Box<i64>) jest niepoprawnym
    # Rustem, wiec typ musi byc calkowicie wywnioskowany z inicjalizatora
    assert "let mut b = Box::new(5);" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_generic_type_params_survive_formatter_roundtrip():
    src = textwrap.dedent(
        """
        struct Box<T> [
            value: T
        ]

        fun identity<T>(x: T) -> T [
            end x
        ]
        """
    )
    out1 = format_source(src)
    out2 = format_source(out1)
    assert "struct Box<T> [" in out1
    assert "fun identity<T>(x: T) -> T [" in out1
    assert out1 == out2


def test_dict_maps_to_rust_hashmap():
    """`get`/`Get` sa slowami kluczowymi (`get <...>`), wiec metoda odczytu
    Dict nazywa sie `.fetch()` a nie `.get()` - patrz docs/SYNTAX.md."""
    src = textwrap.dedent(
        """
        fun main() [
            let scores: Dict<Str, Int> = dict()
            scores.insert("alice", 10)
            let found = scores.fetch("alice")
            match found [
                Some(v) -> [
                    log("found:", v)
                ]
                None -> [
                    log("not found")
                ]
            ]
            if scores.contains("alice") [
                scores.remove("alice")
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert "std::collections::HashMap<String, i64>" in rust
    assert "std::collections::HashMap::new()" in rust
    assert 'scores.insert("alice".to_string(), 10);' in rust
    assert '.get("alice".to_string().as_str()).cloned()' in rust
    assert ".contains_key(" in rust
    assert ".remove(" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_recursive_enum_gets_automatic_box():
    """Bez auto-Box Rust odrzuci `Add(Expr, Expr)` jako typ o nieskonczonym
    rozmiarze - patrz docs/ROADMAP.md ('Box/typy rekurencyjne')."""
    src = textwrap.dedent(
        """
        enum Expr [
            Num(Int),
            Add(Expr, Expr)
        ]

        fun eval(e: Expr) -> Int [
            match e [
                Num(n) -> [
                    end n
                ]
                Add(l, r) -> [
                    end eval(l) + eval(r)
                ]
            ]
        ]

        fun main() [
            let e = Add(Num(1), Add(Num(2), Num(3)))
            log(eval(e))
        ]
        """
    )
    rust = transpile_source(src)
    assert "Add(Box<Expr>, Box<Expr>)," in rust
    assert "Expr::Add(Box::new(Expr::Num(1))," in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_recursive_struct_via_option_gets_automatic_box():
    """`next: Option<Node>` w `struct Node` (klasyczna lista wiazana) -
    typ pola staje sie `Option<Box<Node>>`, konstruktor przyjmuje zwykle
    `Option<Node>` i mapuje przez `.map(Box::new)` wewnatrz."""
    src = textwrap.dedent(
        """
        struct Node [
            value: Int,
            next: Option<Node>
        ]

        fun main() [
            let tail = Node(2, none())
            let head = Node(1, some(tail))
            log(head.value)
        ]
        """
    )
    rust = transpile_source(src)
    assert "pub next: Option<Box<Node>>," in rust
    assert "pub fn new(value: i64, next: Option<Node>) -> Self {" in rust
    assert "next: next.map(Box::new)" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_str_char_at_and_slice():
    src = textwrap.dedent(
        """
        fun main() [
            let s = "hello world"
            log(s.char_at(0))
            log(s.slice(0, 5))
        ]
        """
    )
    rust = transpile_source(src)
    assert ".chars().nth(0 as usize)" in rust
    assert ".chars().skip(0 as usize).take(((5) - (0)) as usize).collect::<String>()" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_result_stdlib_module_uses_real_generics():
    """libs/std/lib/result.hcs (unwrap_or/unwrap_or_default/is_ok/is_err/
    is_some) - dopiero generyki uzytkownika (fun f<T,E>(...)) umozliwily
    napisanie tych funkcji jako prawdziwego, ogolnego kodu HackerScript
    zamiast recznej monomorfizacji dla kazdego typu."""
    result_path = Path(__file__).resolve().parent.parent.parent / "libs" / "std" / "lib" / "result.hcs"
    src = result_path.read_text(encoding="utf-8")
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]
    rust = transpile_source(src)
    assert "pub fn unwrap_or<T, E>(r: Result<T, E>, domyslna: T) -> T {" in rust
    assert "pub fn unwrap_or_default<T>(o: Option<T>, domyslna: T) -> T {" in rust
    assert "pub fn is_ok<T, E>(r: Result<T, E>) -> bool {" in rust
    assert "pub fn is_err<T, E>(r: Result<T, E>) -> bool {" in rust
    assert "pub fn is_some<T>(o: Option<T>) -> bool {" in rust


def test_string_escapes_are_not_double_escaped():
    """Bug znaleziony przy pisaniu bootstrap/hackerc-self/lexer.hcs:
    zrodlowe `"\\n"` (backslash+n w .hcs) musi wygenerowac Rust `"\n"`
    (POJEDYNCZY backslash - prawdziwy escape nowej linii), nie `"\\n"`
    (podwojnie zescape'owany - dosl. backslash+n jako tekst). Lekser
    zachowywal RAW '\\'+litera w buforze tokena, a codegen re-escape'owal
    kazdy backslash jeszcze raz -> podwojne escape'owanie dla `\n`, `\t`,
    `\"`, `\\` w KAZDYM stringu w jezyku."""
    src = textwrap.dedent(
        r"""
        fun main() [
            let s = "line1\nline2\ttabbed"
            log(s)
        ]
        """
    )
    rust = transpile_source(src)
    assert '"line1\\nline2\\ttabbed"' in rust  # Rust: pojedynczy backslash-escape
    assert '\\\\n' not in rust  # NIE podwojny backslash przed 'n'
    assert '\\\\t' not in rust


def test_empty_list_literal_matches_any_declared_generic_type():
    """Bug znaleziony przy pisaniu bootstrap/hackerc-self/lexer.hcs:
    `let tokens: List<Token> = []` falszywie wywalalo E0005, bo pusta
    lista wnioskuje sie jako `List<Any>`, a `Any` (placeholder) byl
    porownywany jak prawdziwy typ zamiast dopasowywac sie do wszystkiego."""
    src = textwrap.dedent(
        """
        struct Token [
            value: Int
        ]

        fun main() [
            let tokens: List<Token> = []
            log(tokens.len())
        ]
        """
    )
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_bootstrap_lexer_checks_and_transpiles():
    """bootstrap/hackerc-self/lexer.hcs - pierwszy prawdziwy kawalek
    samo-hostowanego hackerc (patrz docs/ROADMAP.md, 'Bootstrap -
    stage0'). Musi przechodzic typecheck i transpilowac sie do
    strukturalnie sensownego Rusta (uzywajac enum/match/struct/Str
    metod/generykow dodanych w tej i poprzednich sesjach)."""
    lexer_path = Path(__file__).resolve().parent.parent.parent / "bootstrap" / "hackerc-self" / "lexer.hcs"
    src = lexer_path.read_text(encoding="utf-8")
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]
    rust = transpile_source(src)
    assert "pub enum TokKind {" in rust
    assert "pub struct Token {" in rust
    assert "pub fn tokenize(source: &String) -> Vec<Token> {" in rust
    assert "pub fn main() {" in rust
    # Dopisane w sesji "pelny lexer.hcs": DocComment/LineComment/Question
    # jako WLASNE warianty TokKind (parytet z TokKind.DOC_COMMENT/
    # LINE_COMMENT/QUESTION w lexer.py), pole `tight: bool` na Token.
    assert "DocComment," in rust
    assert "LineComment," in rust
    assert "Question," in rust
    assert "pub tight: bool," in rust
    assert "pub fn trim_str(s: &String) -> String {" in rust
    assert "pub fn strip_multiline_comments(source: &String) -> String {" in rust
    assert "pub fn resolve_escape(c: &String) -> String {" in rust


def test_bootstrap_lexer_doc_comment_gets_own_token_kind_not_op():
    """`?` w lexer.hcs jest WLASNYM `TokKind::Question`, NIE `Op` -
    `parser.hcs::parse_postfix` musi sprawdzac `check(Question, ...)`,
    nie `check(Op, some("?"))` (ta druga wersja bylaby martwym kodem -
    nigdy nie dopasowalaby sie do prawdziwego tokenu `?`). (Test
    zaktualizowany w kroku 3/N - `expr_parser.hcs`/`self.peek()`
    zastapione przez `parser.hcs`/`self.check()` po konsolidacji.)"""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "parser.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    assert "self.check(&TokKind::Question, None)" in main_rs
    # Stara wersja (poprzednia sesja) sprawdzala '?' jako `Op` o
    # wartosci "?" - to musialo zniknac z lancucha postfiksowego (Op
    # nadal wystepuje gdzie indziej, np. dla '-', wiec nie sprawdzamy
    # jego calkowitej nieobecnosci w pliku).
    assert 'check(&TokKind::Op, Some("?".to_string()))' not in main_rs


def test_returning_self_field_gets_cloned():
    """Bug: `end self.pole`/`return self.pole` w metodzie z `&self`
    probowaloby przeniesc wlasnosc pola spod referencji (Rust E0507:
    cannot move out of ... behind a shared reference). Naprawione
    doklejeniem `.clone()` gdy zwracana wartosc to dostep do pola, a
    typ zwracany jest non-Copy (struct/enum/List/Str)."""
    src = textwrap.dedent(
        """
        struct Bag [
            items: List<Int>
        ]

        struct Holder [
            bag: Bag
        ]

        impl Holder [
            fun get_bag(self) -> Bag [
                end self.bag
            ]
        ]

        fun main() [
            let h = Holder(Bag([]))
            log(h.get_bag())
        ]
        """
    )
    rust = transpile_source(src)
    assert "return self.bag.clone();" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_chained_method_call_receiver_gets_auto_ref():
    """Bug z docs/ROADMAP.md: auto-ref argumentow metody dzialal tylko
    dla prostych odbiornikow (zmienna). `h.get_bag().add_all(extra)` -
    gdzie odbiornik `add_all` to WYNIK innej metody (`get_bag()`) - musi
    tez dostac `&extra`, wymaga znajomosci zadeklarowanego typu zwracanego
    `get_bag` (Signatures.methods w typeinfer.py)."""
    src = textwrap.dedent(
        """
        struct Bag [
            items: List<Int>
        ]

        impl Bag [
            fun add_all(self, xs: List<Int>) [
                self.items = self.items + xs
            ]
        ]

        struct Holder [
            bag: Bag
        ]

        impl Holder [
            fun get_bag(self) -> Bag [
                end self.bag
            ]
        ]

        fun main() [
            let h = Holder(Bag([]))
            let extra = [1, 2, 3]
            h.get_bag().add_all(extra)
        ]
        """
    )
    rust = transpile_source(src)
    assert "h.get_bag().add_all(&extra);" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_duplicate_variant_name_across_enums_raises_clear_error():
    """Wczesniej: ciche kolizje (ostatni zarejestrowany enum wygrywal).
    Teraz: jawny blad zamiast po cichu zlego kodu."""
    src = textwrap.dedent(
        """
        enum Shape [
            Circle(Float),
            Empty
        ]

        enum Status [
            Empty,
            Active
        ]
        """
    )
    with pytest.raises(TranspileError):
        transpile_source(src)


def test_variant_colliding_with_builtin_option_result_raises_clear_error():
    src = textwrap.dedent(
        """
        enum MyEnum [
            Some(Int),
            None
        ]
        """
    )
    with pytest.raises(TranspileError):
        transpile_source(src)


def test_question_operator_propagates_result_error():
    src = textwrap.dedent(
        """
        fun parse_num(s: Str) -> Result<Int, Str> [
            if s == "" [
                end err("puste")
            ]
            end ok(42)
        ]

        fun double_num(s: Str) -> Result<Int, Str> [
            let n = parse_num(s)?
            end ok(n * 2)
        ]
        """
    )
    rust = transpile_source(src)
    assert "(parse_num(&s))?;" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_question_operator_outside_result_function_is_an_error():
    src = textwrap.dedent(
        """
        fun parse_num(s: Str) -> Result<Int, Str> [
            end ok(42)
        ]

        fun main() [
            let n = parse_num("5")?
            log(n)
        ]
        """
    )
    diags = check_program(parse(src))
    errors = [d for d in diags if d.severity == "error"]
    assert any(d.code == "E0011" for d in errors)


def test_indirect_mutual_recursion_gets_boxed():
    """Bug z docs/ROADMAP.md: 'Box wykrywa tylko bezposrednia
    samo-referencje'. `struct A [ b: B ]` + `struct B [ a: A ]` (A i B
    sie NAWZAJEM zawieraja, zaden wprost siebie) tworzy typ o
    nieskonczonym rozmiarze dokladnie tak samo jak `next: Node` w
    samym `Node` - wykrywane przez ogolny algorytm cykli w grafie
    zaleznosci (patrz `_build_recursion_info` w codegen.py)."""
    src = textwrap.dedent(
        """
        struct A [
            b: B,
            value: Int
        ]

        struct B [
            a: A,
            value: Int
        ]

        fun main() [
            log(1)
        ]
        """
    )
    rust = transpile_source(src)
    # dokladnie JEDNA strona cyklu dostaje Box (ktora - to szczegol
    # implementacji DFS, nie kontrakt) - test sprawdza ze cykl jest
    # PRZERWANY (`Box<A>` XOR `Box<B>`), a #[derive(Default)] jest
    # pominiete po OBU stronach (przenoszenie sie przez wartosc -
    # Box<T>: Default nadal wymaga T: Default).
    assert ("pub a: Box<A>," in rust) != ("pub b: Box<B>," in rust)
    assert "#[derive(Debug, Clone, PartialEq, Default)]" not in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_three_way_indirect_recursion_gets_boxed():
    """Cykl przez TRZY structy (A -> B -> C -> A) - nie tylko para."""
    src = textwrap.dedent(
        """
        struct A [
            b: B
        ]

        struct B [
            c: C
        ]

        struct C [
            a: A
        ]

        fun main() [
            log(1)
        ]
        """
    )
    rust = transpile_source(src)
    assert "Box<A>" in rust or "Box<B>" in rust or "Box<C>" in rust
    assert "#[derive(Debug, Clone, PartialEq, Default)]" not in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_list_of_self_does_not_need_box():
    """`List<T>`/`Dict<K,V>` sa juz posrednie (Vec/HashMap alokowane na
    kopcu) - `struct Tree [ children: List<Tree> ]` NIE potrzebuje Box
    (w przeciwienstwie do `next: Tree` bezposrednio)."""
    src = textwrap.dedent(
        """
        struct Tree [
            children: List<Tree>
        ]

        fun main() [
            log(1)
        ]
        """
    )
    rust = transpile_source(src)
    assert "pub children: Vec<Tree>," in rust
    assert "Box<" not in rust
    assert "#[derive(Debug, Clone, PartialEq, Default)]" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_indexing_non_copy_list_element_gets_cloned():
    """Bug znaleziony przy projektowaniu bootstrap/hackerc-self/parser.hcs:
    `let t = tokens[0]` gdzie `tokens: List<Token>` (Token to struct,
    non-Copy) probowaloby przeniesc wlasnosc z indeksowania - Rust to
    odrzuca (ta sama klasa bledu co `return self.pole`). Naprawione
    doklejeniem `.clone()`. Elementy Copy (Int/Float/Bool) NIE dostaja
    klonowania (niepotrzebne)."""
    src = textwrap.dedent(
        """
        struct Token [
            value: Str
        ]

        fun first_value(tokens: List<Token>) -> Str [
            let t = tokens[0]
            end t.value
        ]

        fun main() [
            let xs = [1, 2, 3]
            log(xs[0])
        ]
        """
    )
    rust = transpile_source(src)
    assert "tokens[0 as usize].clone();" in rust
    assert "xs[0 as usize])" in rust  # Copy (Int) - bez .clone()
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_cast_to_str_uses_to_string_not_invalid_as():
    """Bug znaleziony przy pisaniu bootstrap/hackerc-self/ast_nodes.hcs:
    `n as Str` (n: Float) generowalo `n as String`, co NIE JEST
    poprawnym Rustem - `as` dziala tylko miedzy typami numerycznymi/
    wskaznikami, nie konwertuje liczby na String. Naprawione: `x as
    Str` zawsze generuje `.to_string()`."""
    src = textwrap.dedent(
        """
        fun num_to_str(n: Float) -> Str [
            end n as Str
        ]
        """
    )
    rust = transpile_source(src)
    assert "(n).to_string()" in rust
    assert " as String" not in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_struct_with_enum_field_skips_default_derive():
    """Bug znaleziony przy pisaniu bootstrap/hackerc-self/expr_parser.hcs:
    `struct Token [ kind: TokKind ]` derywowal `#[derive(Default)]`, co
    sie NIE SKOMPILUJE - enumy nigdy nie implementuja `Default` w tym
    codegen. Naprawione ogolnym sprawdzeniem typu KAZDEGO pola (nie
    tylko cykli struct<->struct jak wczesniej)."""
    src = textwrap.dedent(
        """
        enum TokKind [
            Number,
            Ident
        ]

        struct Token [
            kind: TokKind,
            text: Str
        ]

        fun main() [
            log(1)
        ]
        """
    )
    rust = transpile_source(src)
    assert "#[derive(Debug, Clone, PartialEq)]\npub struct Token {" in rust
    assert "pub struct Token" in rust and "Default" not in rust.split("pub struct Token")[0].split("\n")[-2]
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_enum_values_support_equality_comparison():
    """Bug znaleziony przy pisaniu bootstrap/hackerc-self/expr_parser.hcs:
    `self.peek() == TokKind::Plus` sie nie skompiluje bez `PartialEq` na
    enumie (Rust nie derywuje go domyslnie). Naprawione dodaniem
    `PartialEq` do derive() kazdego enum (i struct, dla spojnosci)."""
    src = textwrap.dedent(
        """
        enum TokKind [
            Plus,
            Minus
        ]

        fun is_plus(k: TokKind) -> Bool [
            end k == Plus
        ]
        """
    )
    rust = transpile_source(src)
    assert "#[derive(Debug, Clone, PartialEq)]" in rust
    assert "(k == TokKind::Plus)" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_bootstrap_ast_nodes_checks_and_transpiles():
    """bootstrap/hackerc-self/ast_nodes.hcs - krok 3/N przepisania
    calego hackerc (patrz docs/ROADMAP.md, 'W TOKU'). PELNY parytet z
    ast_nodes.py: WSZYSTKIE wezly (Expr+Stmt+TypeRef+Program) w JEDNYM
    pliku - zastepuje dawne ast_nodes.hcs (tylko Expr) + stmt_nodes.hcs
    + decl_nodes.hcs (usuniete w tej sesji). `Expr` jest samo-referencyjny
    w wielu wariantach (BinOp/UnaryOp/Call/Index/Cast/TryOp/Attr) ->
    KAZDY dostaje Box; `Stmt` NIE jest (tylko `List<Stmt>`, posrednie)
    -> ZADEN wariant nie dostaje Box; `TypeRef` jest samo-referencyjny
    przez `Option<TypeRef>` -> `Option<Box<TypeRef>>`."""
    path = Path(__file__).resolve().parent.parent.parent / "bootstrap" / "hackerc-self" / "ast_nodes.hcs"
    src = path.read_text(encoding="utf-8")
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]
    rust = transpile_source(src)
    assert "pub struct TypeRef {" in rust
    assert "pub generic: Option<Box<TypeRef>>," in rust
    assert "pub enum Expr {" in rust
    assert "BinOp(String, Box<Expr>, Box<Expr>)," in rust
    assert "Call(Box<Expr>, Vec<Expr>)," in rust
    assert "pub enum Stmt {" in rust
    assert "IfStmt(Expr, Vec<Stmt>, Vec<ElifArm>, Option<Vec<Stmt>>)," in rust
    # Stmt nie ma ZADNEGO bezposredniego samo-odwolania (tylko przez
    # List/Option<List>) - zaden wariant Stmt nie powinien dostac Box.
    stmt_enum_block = rust.split("pub enum Stmt {")[1].split("}")[0]
    assert "Box<" not in stmt_enum_block
    assert "pub struct Program {" in rust
    assert "pub fn main() {" in rust


def test_bootstrap_parser_full_grammar_links_and_builds():
    """bootstrap/hackerc-self/parser.hcs - krok 3/N, PELNY parytet z
    parser.py (618 linii) w JEDNYM pliku (zastepuje dawne
    expr_parser.hcs/stmt_parser.hcs/decl_parser.hcs, usuniete w tej
    sesji - ich trojpodzial byl rozsadny przyrostowo, ale rozjezdzal
    sie z prawdziwym parser.py, gdzie WSZYSTKO jest jedna klasa
    Parser). Weryfikuje cala gramatyke: instrukcje najwyzszego poziomu
    (struct/enum/fun/impl/match/get-import/using/direct/manual/gc),
    blokowe (if/elif/else/while/for), i precedence climbing dla
    wyrazen (or/and/not/porownania/addytywne/multiplikatywne/unarne/
    postfiks z `.`/wywolaniem/`as`/`?`/`[indeks]`)."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "parser.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    lexer_rs = (out_dir / "src" / "_hks_selfhost_lexer.rs").read_text(encoding="utf-8")
    ast_rs = (out_dir / "src" / "_hks_selfhost_ast_nodes.rs").read_text(encoding="utf-8")

    assert "use crate::_hks_selfhost_lexer::{TokKind, Token, tokenize};" in main_rs
    assert "pub struct Parser {" in main_rs
    # Bug znaleziony i naprawiony w TEJ sesji: `kind: TokKind` jest
    # ZAWSZE przekazywany jako `&TokKind` (enum = "refable" w
    # _is_refable), wiec `t.kind != kind` (String vs &String -owy
    # odpowiednik dla enumow) bylby niezgodnoscia typow w Ruscie -
    # `.clone()` na referencji auto-derefuje sie (metody, w
    # przeciwienstwie do operatorow ==/!=, auto-derefuja odbiornik).
    assert "pub fn check(&self, kind: &TokKind, value: Option<String>) -> bool {" in main_rs
    assert "if (t.kind != kind.clone())" in main_rs
    # Cala gramatyka - reprezentatywna probka funkcji ze wszystkich
    # warstw (top-level/blokowe/wyrazenia), wszystkie &mut self
    # (posrednio mutuja przez self.advance()).
    for fn_sig in (
        "pub fn parse_program(&mut self) -> Program {",
        "pub fn parse_block(&mut self) -> Vec<Stmt> {",
        "pub fn parse_statement(&mut self) -> Stmt {",
        "pub fn parse_struct(&mut self) -> Stmt {",
        "pub fn parse_enum(&mut self) -> Stmt {",
        "pub fn parse_impl(&mut self) -> Stmt {",
        "pub fn parse_match(&mut self) -> Stmt {",
        "pub fn parse_if(&mut self) -> Stmt {",
        "pub fn parse_for(&mut self) -> Stmt {",
        "pub fn parse_get_import(&mut self) -> Stmt {",
        "pub fn parse_or(&mut self) -> Expr {",
        "pub fn parse_postfix(&mut self) -> Expr {",
        "pub fn parse_primary(&mut self) -> Expr {",
    ):
        assert fn_sig in main_rs, fn_sig
    # `pub fun` rekonstruuje FunDecl przez match/destrukturyzacje
    # (enumy sa niemutowalne - nie mozna przypisac do pola wariantu w
    # miejscu), zamiast mutacji `inner.is_pub = True` jak w Pythonie.
    assert "Stmt::FunDecl(name, params, ret_type, body, is_pub, type_params) => {" in main_rs
    assert "return Stmt::FunDecl(name, params, ret_type, body, true, type_params);" in main_rs
    # `x[i]` (postfiks indeksowania) dziala TYLKO gdy `[` jest "tight"
    # (Token.tight z lexer.hcs) - odrozniajac je od nowego bloku.
    assert "self.check(&TokKind::Open, None) && self.cur().tight" in main_rs
    assert "TryOp(Box<Expr>)," in ast_rs
    assert "pub tight: bool," in lexer_rs
    assert "pub fn main() {" in main_rs


def test_selfhost_module_system_links_bootstrap_files():
    """`get <selfhost:...>` laczy pliki w `bootstrap/hackerc-self/`
    przez ten sam mechanizm co `get <core:...>`/`get <std:...>` dla
    `libs/` - `find_bootstrap_root` auto-wykrywa katalog, cross-file
    sygnatury (w tym enumy) pozwalaja skonstruowac warianty z pliku,
    ktory tylko IMPORTUJE enum, nie deklaruje go u siebie. (Test
    zaktualizowany w kroku 3/N - `expr_parser.hcs` zastapione przez
    `parser.hcs` po konsolidacji AST+parsera w dwa pliki.)"""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "parser.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings
    assert (out_dir / "src" / "_hks_selfhost_lexer.rs").exists()
    assert (out_dir / "src" / "_hks_selfhost_ast_nodes.rs").exists()
    assert (out_dir / "Cargo.toml").exists()


def test_collect_project_signatures_finds_selfhost_enums():
    """Zaktualizowany w kroku 3/N: `expr_parser.hcs` -> `parser.hcs`,
    nazwy wariantow Expr zaktualizowane do nowego, pelnego AST
    (`NumberLit`/`IdentExpr` zamiast dawnych skroconych
    `NumLit`/`Var` - patrz ast_nodes.hcs 'Ograniczenia' co do
    `IdentExpr` zamiast `Ident`, kolizja nazw z `TokKind::Ident`)."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "parser.hcs"
    functions, structs, enums, mut_params, methods, method_mut_params, warnings = collect_project_signatures(entry)
    assert not warnings
    assert "Expr" in enums
    assert {v.name for v in enums["Expr"].variants} >= {"NumberLit", "StringLit", "IdentExpr", "BinOp", "UnaryOp", "Call"}
    assert "Stmt" in enums
    assert "TokKind" in enums
    assert "Token" in structs
    assert ("Parser", "advance") in methods
    assert "self" in method_mut_params.get("Parser::advance", set())


def test_method_calling_mutating_method_gets_mut_self_transitively():
    """Bug znaleziony przy laczeniu bootstrap/hackerc-self/parser.hcs
    z lexer.hcs przez system modulow: metoda ktora sama nie przypisuje
    `self.pole = ...` ale WYWOLUJE inna metode ktora to robi (`outer`
    wywoluje `self.inner()`, a `inner` mutuje `self.value`) musi TEZ
    dostac `&mut self` - inaczej Rust odrzuca wywolanie metody `&mut
    self` przez `&self` ('cannot borrow as mutable')."""
    src = textwrap.dedent(
        """
        struct Counter [
            value: Int
        ]

        impl Counter [
            fun inner(self) [
                self.value = self.value + 1
            ]

            fun outer(self) [
                self.inner()
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert "pub fn inner(&mut self)" in rust
    assert "pub fn outer(&mut self)" in rust
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


def test_check_program_accepts_cross_file_variant_names():
    """Bug znaleziony i naprawiony (`hackerc build` dawal spurious W0002
    dla konstruktorow wariantow z ZAIMPORTOWANEGO enuma, bo
    check_program() domyslnie widzi tylko jeden plik). Naprawione:
    check_program(program, extra_variant_names=...) + cmd_build w
    cli.py woa project.collect_project_signatures() PRZED
    check_program(), zeby znac warianty z innych plikow."""
    src_import_only = textwrap.dedent(
        """
        get <selfhost:ast_nodes> import <Expr>

        fun main() [
            let e = Var("x")
            log(e)
        ]
        """
    )
    # bez extra_variant_names: W0002 (Var nieznana - plik nie deklaruje Expr)
    diags_without = check_program(parse(src_import_only))
    assert any(d.code == "W0002" for d in diags_without)

    # z extra_variant_names (jak dostarcza cmd_build): brak W0002
    diags_with = check_program(parse(src_import_only), extra_variant_names={"Var", "NumLit", "BinOp"})
    assert not any(d.code == "W0002" for d in diags_with)


def test_bootstrap_ast_nodes_and_parser_are_the_current_canonical_files():
    """Regresja strukturalna po konsolidacji w kroku 3/N: dawne
    stmt_nodes.hcs/decl_nodes.hcs/expr_parser.hcs/stmt_parser.hcs/
    decl_parser.hcs (z poprzednich sesji) NIE MOGA wrocic obok nowych
    skonsolidowanych plikow - ich stare enumy `Decl`/osobny `Stmt`
    kolidowalyby nazwami wariantow z nowym ast_nodes.hcs (E0010,
    dokladnie ten blad napotkany przy pisaniu tego kroku)."""
    bootstrap_dir = Path(__file__).resolve().parent.parent.parent / "bootstrap" / "hackerc-self"
    existing = {p.name for p in bootstrap_dir.glob("*.hcs")}
    assert existing == {"lexer.hcs", "ast_nodes.hcs", "parser.hcs", "diagnostics.hcs", "typeinfer.hcs", "typecheck.hcs", "codegen.hcs", "transpiler.hcs", "formatter.hcs", "project.hcs", "cli.hcs"}

def test_bootstrap_typeinfer_links_and_builds_without_box_ref_mismatches():
    """typeinfer.hcs - krok 4/N (patrz docs/ROADMAP.md, 'W TOKU').
    Parytet z typeinfer.py: `Signatures`/`TypeEnv`/`types_equal`/
    `infer_expr_type`. Ten plik mial NAJWIECEJ pulapek Box/referencja
    ze wszystkich krokow dotychczas: `Expr`/`TypeRef` sa
    samo-referencyjne (Box na poziomie Rusta), a dopasowanie wzorca
    enuma na wartosci `Box<T>` wymagaloby jawnej dereferencji, ktorej
    HackerScript nie ma - stad `expr_as_ident_name`/`expr_call_shape`/
    zagniezdzone `match` na WYRAZENIACH pol (`ta.generic`) zamiast
    rekurencji przez wywolania funkcji na wyciagnietych polach."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "typeinfer.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    assert "pub struct Signatures {" in main_rs
    assert "pub struct TypeEnv {" in main_rs
    assert "pub fn infer_expr_type(expr: &Expr, env: &TypeEnv) -> Option<TypeRef> {" in main_rs
    assert "pub fn types_equal(a: Option<TypeRef>, b: Option<TypeRef>) -> bool {" in main_rs
    # Param.type_ref NIE jest Boxowane (Param nie jest czescia cyklu
    # TypeRef -> TypeRef, w przeciwienstwie do TypeRef.generic, ktore
    # JEST) - zweryfikowane, ze te dwa rozne przypadki maja rozny ksztalt.
    ast_rs = (out_dir / "src" / "_hks_selfhost_ast_nodes.rs").read_text(encoding="utf-8")
    assert "pub type_ref: Option<TypeRef>," in ast_rs
    assert "pub generic: Option<Box<TypeRef>>," in ast_rs
    # AttrCallShape unika przechowywania surowego `Expr` (bezpieczne -
    # patrz uwaga projektowa w typeinfer.hcs), tylko juz-wyliczony typ.
    assert "pub target_type: Option<TypeRef>," in main_rs
    assert "pub is_attr_call: bool," in main_rs
    # Bug znaleziony i naprawiony w TEJ sesji: log() zawsze uzywa `{}`
    # (Display), struct/enum maja tylko `Debug` - log(Option<TypeRef>)
    # bezposrednio by sie nie skompilowalo. `type_opt_to_str` konwertuje
    # najpierw na Str.
    assert "pub fn type_opt_to_str(t: Option<TypeRef>) -> String {" in main_rs
    assert 'type_opt_to_str(infer_expr_type(&e1, &env))' in main_rs
    assert "pub fn main() {" in main_rs


def test_bootstrap_typeinfer_collect_signatures_and_dict_helpers():
    """`collect_signatures` klonuje `stmt`/`m` PRZED dopasowaniem
    (`stmt.clone()`) - inaczej wstawienie CALEGO `stmt` do Dict
    WEWNATRZ galezi jego wlasnego `match` byloby uzyciem czesciowo
    przeniesionej wartosci (Rust by to odrzucil). `variant_owner` to
    mapa odwrotna (wariant->enum) zbudowana Z GORY, bo ten bootstrap
    nie ma iteracji po Dict (`.keys()`/`.items()`)."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "typeinfer.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    assert "let mut stmt_copy = stmt.clone();" in main_rs
    assert "variant_owner.insert(v.name, name.clone());" in main_rs
    assert "pub variant_owner: std::collections::HashMap<String, String>," in main_rs
    # DOPISANE w kolejnej sesji (codegen.hcs, krok 6/N): listy nazw dla
    # iteracji, bo ten bootstrap nie ma .keys()/.values()/.items() na Dict.
    assert "pub struct_names: Vec<String>," in main_rs
    assert "pub enum_names: Vec<String>," in main_rs
    assert "pub function_names: Vec<String>," in main_rs


def test_bootstrap_typecheck_links_and_builds_without_ref_owned_mismatches():
    """typecheck.hcs - krok 5/N (patrz docs/ROADMAP.md, 'W TOKU').
    Parytet z typecheck.py: `FnChecker`/`Checker`/`check_program`
    (E0001/E0002/E0003/E0005/E0011/W0001/W0002). Weryfikuje NOWA
    pulapke odkryta w tej sesji: `self.pole` (przez referencje `self`)
    NIE MOZE byc dopasowywane bez `.clone()` (w przeciwienstwie do pola
    OWNED lokalnej zmiennej), oraz ze `TypeEnv` NIE jest polem
    `FnChecker` (bo user-owe metody na polu `self` nie wymuszaja
    `&mut self` w tym codegen)."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "typecheck.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    assert "pub struct FnChecker {" in main_rs
    assert "pub struct Checker {" in main_rs
    # FnChecker NIE ma pola `env: TypeEnv` - patrz uwaga projektowa w
    # typecheck.hcs.
    fnchecker_block = main_rs.split("pub struct FnChecker {")[1].split("}")[0]
    assert "TypeEnv" not in fnchecker_block
    assert "pub vars: std::collections::HashMap<String, Option<TypeRef>>," in fnchecker_block
    assert "pub fn check_program(prog: &Program, extra_variant_names: &Vec<String>) -> Vec<Diagnostic> {" in main_rs
    for fn_sig in (
        "pub fn check_call(&mut self, name: &String, arg_count: i64) {",
        "pub fn visit_expr(&mut self, e: &Expr) {",
        "pub fn visit_stmt(&mut self, s: &Stmt) {",
        "pub fn check_fun(&mut self, fn_stmt: &Stmt, self_type: Option<TypeRef>) {",
        "pub fn check_get(&mut self, source: &String, name: &String) {",
    ):
        assert fn_sig in main_rs, fn_sig
    # Bug znaleziony i naprawiony w TEJ sesji: `self.ret_type` musi
    # byc sklonowane PRZED dopasowaniem (referencja `self`) - "cannot
    # move out of `self.ret_type` which is behind a reference" w
    # prawdziwym Rust.
    assert "let mut rt = self.ret_type.clone();" in main_rs
    assert "match rt {" in main_rs
    # Drugi bug: `extra_variant_names` (parametr, wiec `&Vec<String>`)
    # przekazywany BEZPOSREDNIO do konstruktora `Checker` (ktory
    # oczekuje OWNED `Vec<String>`) bylby niezgodnoscia typow.
    assert "Checker::new((sigs).clone(), vec![], (imported_names).clone(), extra_variant_names.clone());" in main_rs
    assert "pub fn main() {" in main_rs


def test_bootstrap_codegen_type_rendering_and_recursion_detection():
    """codegen.hcs - krok 6/N, CZESCIOWY (patrz docs/ROADMAP.md, 'W
    TOKU') - warstwa ANALIZY (rust_type_name + wykrywanie Box/rekurencji
    przez RecursionAnalyzer), NIE warstwa emisji (gen_expr/gen_stmt
    jeszcze nie istnieja). Weryfikuje, ze `struct Node [ next: Node ]`
    (bezposrednia samo-rekurencja) i posrednia rekurencja przez dwa
    structy (`A -> B -> Option<A>`) dostaja poprawnie oznaczone
    krawedzie do zboxowania - DOKLADNIE ten algorytm, ktory kazdy
    poprzedni krok wykonywal RECZNIE, teraz jako kod."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "codegen.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    assert "pub fn rust_type_name(t: &TypeRef, sigs: &Signatures, type_params: &Vec<String>) -> String {" in main_rs
    assert "pub struct Edge {" in main_rs
    assert "pub struct RecursionAnalyzer {" in main_rs
    assert "pub fn dfs(&mut self, node: &String) {" in main_rs
    assert "pub fn build_recursion_info(sigs: &Signatures) -> RecursionInfo {" in main_rs
    assert "pub fn main() {" in main_rs
    # Regresja: KAZDE indeksowanie kolekcji nietrywialnego typu w tym
    # pliku musi miec `.clone()` - znaleziony bug: zmienne zadeklarowane
    # przez `let x = wyrazenie.clone()` TRACA sledzenie typu w
    # Pythonowym typeinfer.py (`.clone()` nie jest na liscie
    # rozpoznawanych wywolan metod), co kaskadowo wylacza
    # auto-`.clone()` przy PUZNIEJSZYM indeksowaniu takiej zmiennej -
    # "cannot move out of index of Vec" w prawdziwym Rust.
    assert "let mut e = edges[i as usize].clone();" in main_rs
    assert "let mut name = known_structs[i as usize].clone();" in main_rs
    assert "all_nodes.push(known_structs[kk as usize].clone());" in main_rs
    # Drugi bug: `Dict<K,V>` NIE jest "refable" w tym codegen (tylko
    # Str/List/struct/enum sa) - `rust_type_name` uzywalo `structs`/
    # `enums` (Dict) WIELOKROTNIE w jednym wywolaniu (np. dla
    # Dict<K,V> trzeba rekurencyjnie wyrenderowac K i V), co przy
    # przekazaniu Dict PRZEZ WARTOSC bylo by "value used after move".
    # Naprawione: `rust_type_name` bierze `sigs: Signatures` (STRUCT,
    # WIEC refable - `&Signatures`, mozna pozyczac dowolnie wiele
    # razy) zamiast osobnych `structs: Dict<...>`/`enums: Dict<...>`.
    assert "let mut sname = known_structs[m as usize].clone();" in main_rs


def test_bootstrap_codegen_mut_params_analysis_matches_transitive_self_calls():
    """codegen.hcs, kontynuacja kroku 6/N: `MutTracker`/`SelfCallTracker`/
    `compute_mut_params`/`compute_method_mut_params` - parytet z
    `_mutated_names_in_body`/`_find_self_method_calls`/
    `_compute_mut_params`/`_compute_method_mut_params`. To DOKLADNIE
    ten algorytm (z punktem stalym dla posredniej mutacji
    self.metoda()), ktory pozwolil KAZDEMU poprzedniemu krokowi tej
    sesji pisac metody bez recznego `&mut`."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "codegen.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    assert "pub struct MutTracker {" in main_rs
    assert "pub struct SelfCallTracker {" in main_rs
    assert "pub fn compute_mut_params(prog: &Program) -> std::collections::HashMap<String, bool> {" in main_rs
    assert "pub fn compute_method_mut_params(prog: &Program, extra_method_mut_params: &std::collections::HashMap<String, bool>) -> std::collections::HashMap<String, bool> {" in main_rs
    # Punkt staly (fixed-point) dla posredniej mutacji.
    assert "let mut changed: bool = true;" in main_rs
    assert "while changed {" in main_rs
    # Wszystkie metody MutTracker/SelfCallTracker poprawnie dostaly
    # &mut self (bezposrednio LUB tranzytywnie przez self.metoda()).
    for fn_sig in (
        "pub fn mark_mutated(&mut self, name: &String) {",
        "pub fn mark_base(&mut self, base: &Expr) {",
        "pub fn handle_mutating_call(&mut self, callee: &Expr) {",
        "pub fn mark_call(&mut self, name: &String) {",
        "pub fn handle_call(&mut self, callee: &Expr) {",
    ):
        assert fn_sig in main_rs, fn_sig
    # Regresja: `elifs[i]`/`arms[j]`/`methods[j]` (structy non-Copy)
    # musialy dostac `.clone()` recznie - ten sam rodzaj bledu co w
    # poprzednim tescie, znaleziony w NOWYM miejscu.
    assert "let mut arm = elifs[i as usize].clone();" in main_rs
    assert "let mut arm2 = arms[j as usize].clone();" in main_rs
    assert "let mut m = methods[j as usize].clone();" in main_rs
    # `&collection[i]` (bez `let`, przekazane BEZPOSREDNIO jako
    # argument referencyjny) NIE potrzebuje `.clone()` - to POZYCZENIE,
    # nie proba przeniesienia - inaczej niz `let x = collection[i]`.
    assert "self.walk_expr(&args[i as usize]);" in main_rs


def test_bootstrap_codegen_gen_expr_emits_correct_special_cases():
    """codegen.hcs, kontynuacja kroku 6/N: `gen_expr`/`gen_call`/
    `gen_binop`/`gen_ident`/`gen_index`/`gen_cast` - parytet z
    `CodeGen.gen_expr()` (najwieksza, najbardziej rozgalęziona czesc
    codegen.py). Pinuje DWA bledy znalezione w TEJ sesji: (1)
    `gen_ident` zwracalo goly parametr `Str` (zawsze `&String` w tym
    codegen) tam gdzie funkcja deklaruje zwracanie `Str` (owned) -
    naprawione `name + ""`; (2) `type_ref_generic`/`type_ref_generic2`
    probowaly ZWROCIC `Box<TypeRef>` jako `TypeRef` (Rust nie ma
    automatycznego odpakowania Box przy zwracaniu, w przeciwienstwie
    do koercji `&Box<T> -> &T` przy przekazywaniu argumentu) - funkcje
    CALKOWICIE USUNIETE, zastapione bezpiecznymi
    `rust_type_name_of_generic`/`type_ref_generic_name`, ktore NIGDY
    nie probuja 'zwrocic' samego TypeRef wyciagnietego z Box."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "codegen.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    assert "pub struct CodeGen {" in main_rs
    assert "pub fn gen_expr(&self, node: &Expr) -> String {" in main_rs
    assert "pub fn gen_call(&self, callee: &Expr, args: &Vec<Expr>) -> String {" in main_rs
    assert "pub fn gen_binop(&self, op: &String, left: &Expr, right: &Expr) -> String {" in main_rs
    # Bug 1: gen_ident zwracalo goly `name` (parametr, zawsze &String w
    # tym codegen) - naprawione wymuszona konkatenacja.
    assert 'return format!("{}{}", name, "".to_string()).to_string();' in main_rs
    assert "pub fn gen_ident(&self, name: &String) -> String {" in main_rs
    # Bug 2: type_ref_generic/type_ref_generic2 CALKOWICIE usuniete -
    # zastapione bezpiecznymi wariantami.
    assert "pub fn type_ref_generic(" not in main_rs
    assert "pub fn type_ref_generic2(" not in main_rs
    assert "pub fn type_ref_generic_name(&self" not in main_rs  # to WOLNA funkcja, nie metoda
    assert "pub fn type_ref_generic_name(t: &TypeRef) -> String {" in main_rs
    assert "pub fn rust_type_name_of_generic(t: &TypeRef, sigs: &Signatures, type_params: &Vec<String>) -> String {" in main_rs
    assert "return rust_type_name(&inner, &sigs, &type_params).to_string();" in main_rs
    # `+` na Str -> format!, `+` na List -> chain().collect() (nigdy
    # surowy Rustowy `+` na String/Vec, ktory konsumowalby LHS).
    assert '.iter().cloned().chain(' in main_rs
    assert 'format!("{}{}", ' in main_rs
    # Dict.fetch/contains/remove specjalne wywolania.
    assert '.get(' in main_rs
    assert '.contains_key(' in main_rs
    assert "pub fn main() {" in main_rs


def test_bootstrap_codegen_gen_stmt_emits_correct_special_cases():
    """codegen.hcs, kontynuacja kroku 6/N: `gen_stmt`/`gen_let_stmt`/
    `gen_if_stmt`/`gen_match`/`gen_return_expr`/`gen_expr_stmt` -
    parytet z `CodeGen.gen_stmt()` i pomocnikow (wszystkie 20 wariantow
    `Stmt`). Kluczowa zmiana strukturalna: `CodeGen.env` przestal byc
    `Option<TypeEnv>` (co uniemozliwialoby bezpieczna mutacje, jak
    `FnChecker` w typecheck.hcs juz odkryl) - teraz `env_vars:
    Dict<Str, Option<TypeRef>>` BEZPOSREDNIO jako pole, mutowane przez
    `self.env_vars.insert(...)` (wykrywalne przez auto-&mut self)."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "codegen.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    # CodeGen nie ma juz pola `env: Option<TypeEnv>` - splaszczone do
    # `env_vars` bezposrednio.
    codegen_struct_block = main_rs.split("pub struct CodeGen {")[1].split("}")[0]
    assert "TypeEnv" not in codegen_struct_block
    assert "pub env_vars: std::collections::HashMap<String, Option<TypeRef>>," in codegen_struct_block
    for fn_sig in (
        "pub fn emit(&mut self, text: &String) {",
        "pub fn declare_env(&mut self, name: &String, t: Option<TypeRef>) {",
        "pub fn gen_let_stmt(&mut self, name: &String, type_ref: Option<TypeRef>, value: Option<Expr>, is_const: bool) {",
        "pub fn gen_if_stmt(&mut self, cond: &Expr, body: &Vec<Stmt>, elifs: &Vec<ElifArm>, else_body: Option<Vec<Stmt>>) {",
        "pub fn gen_match(&mut self, subject: &Expr, arms: &Vec<MatchArm>) {",
        "pub fn gen_return_expr(&self, value: &Expr) -> String {",
        "pub fn gen_expr_stmt(&mut self, e: &Expr) {",
        "pub fn gen_stmt(&mut self, node: &Stmt) {",
        "pub fn gen_stmts(&mut self, stmts: &Vec<Stmt>) {",
    ):
        assert fn_sig in main_rs, fn_sig
    # `self.pole` zwracane z funkcji o "refable" typie zwracanym
    # dostaje `.clone()` (return self.pole by przenosilo wlasnosc
    # spod referencji - Rust by to odrzucil).
    assert '.clone()".to_string()' in main_rs
    # gen_match zapisuje/przywraca stan `env_vars` wokol kazdej galezi.
    assert "prev_present.push(was_present);" in main_rs
    assert "self.env_vars.remove(b2.as_str());" in main_rs
    # Demonstracyjne uzycie w main() rzeczywiscie konstruuje CodeGen i
    # woa gen_stmts - regresja na blad znaleziony PODCZAS pisania tego
    # demo: powtorne uzycie tego samego (nie-refable, wiec przenoszonego
    # przez wartosc) Dict jako DWOCH roznych argumentow konstruktora
    # bylby podwojnym przeniesieniem - naprawione osobnymi Dict-ami.
    assert "CodeGen::new((empty_sigs).clone(), (empty_vars).clone(), (empty_dict1).clone(), (empty_dict2).clone(), (empty_dict3).clone(), (empty_dict4).clone(), vec![]" in main_rs
    assert "gen.gen_stmts(&body);" in main_rs


def test_bootstrap_codegen_gen_struct_gen_fun_gen_impl_produce_correct_rust():
    """codegen.hcs, zamkniecie kroku 6/N: `gen_struct`/`gen_enum`/
    `gen_fun`/`gen_impl`/`gen_program` - generatory DEKLARACJI
    najwyzszego poziomu, spinajace WSZYSTKO napisane w tym kroku
    (rust_type_name/Box/auto-mut/gen_expr/gen_stmt) w jeden dzialajacy
    generator calego pliku Rust. Krzyzowo zweryfikowane wobec
    PRAWDZIWEGO Pythonowego hackerc na rownowaznym programie
    (`struct Point [ x: Int, y: Int ]` + `fun add_one_two() -> Int
    [ end 1 + 2 ]`) - identyczny ksztalt wyjscia.

    Pinuje TEZ bug znaleziony w tej czesci: `new_codegen`'s `sigs`
    parametr (struct, wiec zawsze `&Signatures`) przekazywany
    BEZPOSREDNIO do `CodeGen::new` (ktore oczekuje OWNED `Signatures`)
    bylby niezgodnoscia typow - naprawione `sigs.clone()`."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "codegen.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    for fn_sig in (
        "pub fn gen_struct(&mut self, name: &String, fields: &Vec<Param>, type_params: &Vec<String>) {",
        "pub fn gen_enum(&mut self, name: &String, variants: &Vec<EnumVariant>, type_params: &Vec<String>) {",
        "pub fn gen_fun(&mut self, name: &String, params: &Vec<Param>, ret_type: Option<TypeRef>, body: &Vec<Stmt>, type_params: &Vec<String>) {",
        "pub fn gen_method(&mut self, struct_name: &String, m: &Stmt) {",
        "pub fn gen_impl(&mut self, struct_name: &String, methods: &Vec<Stmt>, type_params: &Vec<String>) {",
        "pub fn gen_program(prog: &Program) -> Vec<String> {",
    ):
        assert fn_sig in main_rs, fn_sig
    # Bug: `sigs.clone()` konieczne w new_codegen (patrz docstring).
    assert "return CodeGen::new(sigs.clone(), (env_vars).clone(), (variant_arity).clone(), (boxed_fields).clone(), (method_mut_params).clone(), (mut_params).clone(), vec![], None, vec![], 0, (no_default_structs).clone(), false, (direct_blocks).clone(), false);" in main_rs
    # Struct bez generykow/rekurencji dostaje #[derive(..., Default)];
    # konstruktor pozycyjny `Nazwa::new(...)`.
    assert '#[derive(Debug, Clone, PartialEq, Default)]' in main_rs
    assert 'pub fn new(' in main_rs
    # Krzyzowa weryfikacja: PRAWDZIWY Pythonowy hackerc na rownowaznym
    # programie produkuje DOKLADNIE ta strukture (sprawdzone osobno w
    # tej sesji, patrz docs/ROADMAP.md) - tu tylko potwierdzamy, ze
    # demo `main()` w tym pliku faktycznie WYWOLUJE `gen_program`.
    assert "let mut generated: Vec<String> = gen_program(&demo_prog);" in main_rs
    assert "pub fn main() {" in main_rs


def test_bootstrap_codegen_gen_direct_pyo3_and_gen_toplevel():
    """codegen.hcs - dopelnienie kroku 6/N na wyrazna prosbe: `gen_direct`
    (`direct[ ... ]` -> PyO3 `Python::with_gil`) + `gen_toplevel`
    (dysponent WSZYSTKICH form najwyzszego poziomu, nie tylko
    struct/enum/fun/impl - dodaje `using`/`get <...>`/`gc:use::`/
    `extern`/`const`) + naglowek pliku (`#![allow(...)]` +
    `use pyo3::prelude::*;` TYLKO gdy `needs_pyo3`).

    `direct_blocks: Dict<Str,Str>` (surowy tekst Pythona per-indeks)
    jest DANE WEJSCIOWE tej funkcji - populowane przez ekstrakcje ZE
    ZRODLA .hcs PRZED tokenizacja (`transpiler.py`/`_extract_direct_blocks`,
    NIE przepisane w tej sesji, patrz parser.hcs::parse_direct) -
    `gen_direct` samo w sobie jest KOMPLETNE, gotowe na dane wejsciowe
    kiedy `transpiler.hcs` (przyszly krok) zacznie je dostarczac."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "codegen.hcs"
    out_dir = Path(tempfile.mkdtemp())
    result = build_project(entry, out_dir)
    assert not result.warnings, result.warnings
    main_rs = result.main_rs.read_text(encoding="utf-8")
    assert "pub fn gen_direct(&mut self, idx_text: &String) {" in main_rs
    assert "self.needs_pyo3 = true;" in main_rs
    assert 'self.emit(&"Python::with_gil(|py| -> PyResult<()> {".to_string());' in main_rs
    assert 'py.run(' in main_rs
    assert 'python_raw_string(&raw)' in main_rs
    assert 'block failed' in main_rs
    assert "pub fn python_raw_string(s: &String) -> String {" in main_rs
    assert "pub fn gen_toplevel(&mut self, node: &Stmt) {" in main_rs
    assert "pub fn gen_get_import(&mut self, source: &String, name: &String, version: Option<String>, details: &Vec<String>) {" in main_rs
    assert "pub fn gen_extern(&mut self, lib: &String, name: &String, params: &Vec<Param>, ret_type: Option<TypeRef>) {" in main_rs
    assert "pub fn gen_const(&mut self, name: &String, type_ref: Option<TypeRef>, value: Option<Expr>) {" in main_rs
    assert "pub fn str_to_upper(s: &String) -> String {" in main_rs
    assert "pub fn flat_module_name(source: &String, name: &String, version: Option<String>) -> String {" in main_rs
    # Naglowek pliku dodaje `use pyo3::prelude::*;` WARUNKOWO.
    assert 'if gen.needs_pyo3 {' in main_rs
    assert 'header.push("use pyo3::prelude::*;".to_string());' in main_rs
    # CodeGen ma teraz needs_pyo3/direct_blocks jako pola.
    codegen_struct_block = main_rs.split("pub struct CodeGen {")[1].split("}")[0]
    assert "pub needs_pyo3: bool," in codegen_struct_block
    assert "pub direct_blocks: std::collections::HashMap<String, String>," in codegen_struct_block


def test_bootstrap_diagnostics_checks_and_transpiles_without_ref_owned_mismatch():
    """diagnostics.hcs - krok 2/N przepisania calego hackerc (patrz
    docs/ROADMAP.md, 'W TOKU'). Musi przechodzic typecheck i
    transpilowac sie do strukturalnie sensownego Rusta uzywajacego
    struct/impl/Str-metod/Int->Str castow. Pinuje TEZ dwa konkretne
    bledy znalezione i naprawione w tej sesji: `let tag = severity`
    oraz `d.filename = filename` generowaly `String = &String`
    (E0308 w prawdziwym rustc - niewykrywalne przez Pythonowy
    typecheck.py, ktory nie sledzi tego rodzaju niezgodnosci
    referencja/wlasnosc) - naprawione przez wymuszenie konkatenacji
    (`x + ""`), ktora ZAWSZE produkuje wlasciwy `String` (`format!`)."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "diagnostics.hcs"
    src = entry.read_text(encoding="utf-8")
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]
    rust = transpile_source(src)
    assert "pub struct Diagnostic {" in rust
    assert "pub fn render(source: &String" in rust
    assert "pub fn render_many(source: &String, filename: &String, diagnostics: &Vec<Diagnostic>) -> String {" in rust
    assert "pub fn main() {" in rust
    # Regresja: te dwie linie NIGDY nie moga wrocic w tej formie -
    # `String` po lewej, `&String`-owy parametr bez konkatenacji po
    # prawej, to niezgodnosc typow w prawdziwym Rust.
    assert "let mut tag: String = severity;" not in rust
    assert "d.filename = filename;" not in rust
    assert 'let mut tag: String = format!("{}{}", severity, "".to_string());' in rust
    assert 'd.filename = format!("{}{}", filename, "".to_string());' in rust


def test_bootstrap_diagnostics_split_lines_matches_python_splitlines_edge_cases():
    """split_lines w diagnostics.hcs musi zachowywac sie jak Pythonowe
    `source.splitlines() or [\"\"]` w trzech granicznych przypadkach:
    zrodlo puste -> [\"\"], zrodlo z koncowym '\\n' -> BEZ fantomowej
    dodatkowej puste linii, zrodlo bez koncowego '\\n' -> ostatnia
    czesciowa linia ZACHOWANA. Weryfikacja strukturalna (ksztalt
    wygenerowanego Rusta), nie wykonanie (brak rustc w tym
    srodowisku - patrz docs/ROADMAP.md)."""
    root = Path(__file__).resolve().parent.parent.parent
    entry = root / "bootstrap" / "hackerc-self" / "diagnostics.hcs"
    src = entry.read_text(encoding="utf-8")
    rust = transpile_source(src)
    assert 'if ((cur.to_string() != "".to_string().to_string()) || ((lines.len() as i64) == 0)) {' in rust
    assert "lines.push((cur).to_string());" in rust


def test_not_equal_inside_doc_comment_does_not_break_parsing():
    """Bug znaleziony przy pisaniu bootstrap/hackerc-self/expr_parser.hcs:
    `!=` WEWNATRZ TEKSTU komentarza (`!`/`!!`) bylo blednie rozpoznawane
    przez strip_comments() jako otwarcie komentarza wieloliniowego
    `!= ... =!`, psujac caly dalszy plik. Naprawione: strip_comments()
    teraz pomija stringi i komentarze jednoliniowe/dokumentacyjne W
    CALOSCI (bez interpretacji ich tresci) zanim w ogole sprawdzi, czy
    dany `!=` to operator czy otwarcie komentarza."""
    src = textwrap.dedent(
        """
        !! Ten komentarz opisuje operatory: `== != < > <= >=` dla liczb.
        fun main() [
            let a = 1
            let b = 2
            if a != b [
                log("different")
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert "if (a != b) {" in rust
    assert '"different".to_string()' in rust


def test_not_equal_inside_string_literal_does_not_break_parsing():
    src = textwrap.dedent(
        """
        fun main() [
            let s = "result != expected"
            ! komentarz z != wewnatrz
            let a = 1
            let b = 2
            if a != b [
                log(s)
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert '"result != expected".to_string()' in rust
    assert "if (a != b) {" in rust


def test_multiline_comment_with_not_equal_inside_still_stripped():
    src = textwrap.dedent(
        """
        fun main() [
            let a = 1
            != to jest
            komentarz wieloliniowy
            z operatorem != w srodku =!
            let b = 2
            if a != b [
                log("ok")
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert "if (a != b) {" in rust
    assert "to jest" not in rust


def test_doc_comment_before_method_in_impl_is_preserved():
    """Bug znaleziony przy pisaniu bootstrap plikow: `!!` przed metoda w
    `impl` juz nie crashowalo parsera (naprawione wczesniej), ale tresc
    byla cicho odrzucana. Naprawione calkowicie: doczepiana do metody
    jako `_leading_doc_comments`, `gen_method` odtwarza jako Rust `///`
    (doc comment), formatter odtwarza jako `!!` (idempotentnie)."""
    src = textwrap.dedent(
        """
        struct Counter [
            value: Int
        ]

        impl Counter [
            !! Zwieksza licznik o 1.
            fun increment(self) [
                self.value = self.value + 1
            ]
        ]
        """
    )
    rust = transpile_source(src)
    assert "/// Zwieksza licznik o 1." in rust
    assert "pub fn increment(&mut self)" in rust

    out1 = format_source(src)
    out2 = format_source(out1)
    assert "!! Zwieksza licznik o 1." in out1
    assert out1 == out2
    diags = check_program(parse(src))
    assert not [d for d in diags if d.severity == "error"]


if __name__ == "__main__":
    if HAVE_PYTEST:
        raise SystemExit(pytest.main([__file__, "-v"]))

    tests = [(name, fn) for name, fn in list(globals().items()) if name.startswith("test_") and callable(fn)]
    passed, failed, skipped = 0, [], []
    for name, fn in tests:
        try:
            fn()
            print(f"PASS  {name}")
            passed += 1
        except _Skip as exc:
            print(f"SKIP  {name}: {exc}")
            skipped.append(name)
        except Exception as exc:  # noqa: BLE001
            print(f"FAIL  {name}: {exc}")
            failed.append(name)
    print(f"\n{passed}/{len(tests)} testow przeszlo ({len(skipped)} pominietych)")
    if failed:
        print("Nieudane:", ", ".join(failed))
    raise SystemExit(1 if failed else 0)

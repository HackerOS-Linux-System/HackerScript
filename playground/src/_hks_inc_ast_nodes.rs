#![allow(non_snake_case, unused_mut, dead_code)]

//! bootstrap/hackerc-self/ast_nodes.hcs
//! 
//! Krok 3/N przepisania calego hackerc na HackerScript (patrz
//! docs/ROADMAP.md, "W TOKU"). PELNY parytet z
//! hackerc/hackerc/ast_nodes.py (280 linii) - WSZYSTKIE wezly AST
//! (wyrazenia I instrukcje I deklaracje najwyzszego poziomu) w JEDNYM
//! pliku, tak jak w oryginale (Python trzyma je razem, bez rozdzialu
//! na "Expr"/"Stmt"/"Decl" jako osobne hierarchie).
//! 
//! **TEN PLIK ZASTĘPUJE** poprzednie `stmt_nodes.hcs`/`decl_nodes.hcs`
//! (usuniete w tej sesji, patrz docs/ROADMAP.md "Zrobione w tej
//! sesji") - ich zawartosc (Stmt/MatchArm oraz Decl/Param/
//! StructField/EnumVariant) jest tu POLACZONA i ROZSZERZONA do
//! pelnego zakresu Pythonowego ast_nodes.py.
//! 
//! Kompilowany dzis przez STAGE0 (Pythonowy hackerc) - patrz
//! bootstrap/README.md.
//! 
//! Wszystkie wezly Pythona dziedzicza `line: int` po klasie bazowej
//! `Node` - TA WERSJA GO NIE PRZECHOWUJE (celowo, patrz "Ograniczenia":
//! parser.hcs rowniez go dzis nie sledzi - to jest ten sam,
//! udokumentowany od pierwszej sesji brak numerow linii w
//! diagnostykach samo-hostowanego kompilatora).
//! Typ (`Nazwa` albo `Nazwa<generic>` albo `Nazwa<generic, generic2>` -
//! DRUGI argument istnieje WYLACZNIE dla wbudowanego `Result<T, E>`,
//! `Option<T>` uzywa tylko `generic`, parytet z TypeRef w ast_nodes.py).
//! Samo-referencyjny przez `Option<TypeRef>` (kind "option" w
//! box-detection codegen.py - dostaje automatyczny Box, zweryfikowane
//! nizej w wygenerowanym Rust).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeRef {
    pub name: String,
    pub generic: Option<Box<TypeRef>>,
    pub generic2: Option<Box<TypeRef>>,
}

impl TypeRef {
    pub fn new(name: String, generic: Option<TypeRef>, generic2: Option<TypeRef>) -> Self {
        TypeRef { name, generic: generic.map(Box::new), generic2: generic2.map(Box::new) }
    }
}

// Wyrazenia. Warianty odwolujace sie do `Expr` BEZPOSREDNIO (nie przez
// `List<Expr>`) tworza cykl rozmiaru i dostaja automatyczny `Box` -
// (`BinOp`/`UnaryOp`/`Call.callee`/`Index`/`Cast.target`/`TryOp`/
// `Attr.target`) - zweryfikowane w wygenerowanym Rust.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    NumberLit(String),
    StringLit(String, bool),
    BoolLit(bool),
    NullLit,
    IdentExpr(String),
    BinOp(String, Box<Expr>, Box<Expr>),
    UnaryOp(String, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Cast(Box<Expr>, TypeRef),
    TryOp(Box<Expr>),
    Attr(Box<Expr>, String),
    ListLit(Vec<Expr>),
}

// Jeden parametr `fun`/pole `struct` - `type_ref`/`default_expr` sa
// `Option`, bo NIE KAZDY parametr ma jawna adnotacje typu (`self`) czy
// wartosc domyslna (parytet z `type_: Optional[TypeRef]`/`default: Any
// = None` w Param z ast_nodes.py - nazwy pol zmienione na
// `type_ref`/`default_expr`, bo `type`/`default` sa juz uzywane
// gdzie indziej jako nazwy - patrz "Ograniczenia").
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Param {
    pub name: String,
    pub type_ref: Option<TypeRef>,
    pub default_expr: Option<Expr>,
}

impl Param {
    pub fn new(name: String, type_ref: Option<TypeRef>, default_expr: Option<Expr>) -> Self {
        Param { name, type_ref, default_expr }
    }
}

// Jedna galaz `elif cond [ body ]` - w Pythonie to `tuple[Expr,
// list]` w `IfStmt.elifs`, tu WLASNY struct (HackerScript nie ma
// generycznych tupli).
#[derive(Debug, Clone, PartialEq)]
pub struct ElifArm {
    pub cond: Expr,
    pub body: Vec<Stmt>,
}

impl ElifArm {
    pub fn new(cond: Expr, body: Vec<Stmt>) -> Self {
        ElifArm { cond, body }
    }
}

// Jeden wariant `enum` - `Nazwa` (jednostkowy, `fields` puste) albo
// `Nazwa(Typ, Typ2, ...)` (krotkowy).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<TypeRef>,
}

impl EnumVariant {
    pub fn new(name: String, fields: Vec<TypeRef>) -> Self {
        EnumVariant { name, fields }
    }
}

// Jedna galaz `match`: `Wariant(bind1, bind2) -> [ ... ]` albo
// `_ -> [ ... ]` (wildcard, `variant == "_"`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MatchArm {
    pub variant: String,
    pub binds: Vec<String>,
    pub body: Vec<Stmt>,
}

impl MatchArm {
    pub fn new(variant: String, binds: Vec<String>, body: Vec<Stmt>) -> Self {
        MatchArm { variant, binds, body }
    }
}

// Instrukcje/deklaracje - WSZYSTKO, co moze zwrocic `parse_statement`
// w parser.py, w JEDNYM enum (tak jak w Pythonie WSZYSTKIE te klasy
// dziedzicza po `Node` i po prostu ladują w tej samej liscie
// `Program.body`). Zaden wariant nie odwoluje sie do `Stmt`
// BEZPOSREDNIO (tylko przez `List<Stmt>`/`List<ElifArm>`/
// `List<MatchArm>` - wszystkie posrednie) - `Stmt` NIE POTRZEBUJE
// `Box` w zadnym wariancie, zweryfikowane w wygenerowanym Rust.
// `IncludeStmt(Str)` - `include <sciezka>`, odpowiednik Rustowego
// `mod`, WZGLEDEM katalogu biezacego pliku (patrz
// project.hcs::resolve_include_path). Osobny od `GetImportStmt`/
// `get <source:name>`, ktory pozostaje nietkniety - `include` nie ma
// zrodla/wersji/`import <details>`, tylko jeden segment sciezki.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    LetStmt(String, Option<TypeRef>, Option<Expr>, bool),
    AssignStmt(Expr, String, Expr),
    FunDecl(String, Vec<Param>, Option<TypeRef>, Vec<Stmt>, bool, Vec<String>),
    ExternFunDecl(String, String, Vec<Param>, Option<TypeRef>),
    IfStmt(Expr, Vec<Stmt>, Vec<ElifArm>, Option<Vec<Stmt>>),
    WhileStmt(Expr, Vec<Stmt>),
    ForStmt(String, Expr, Vec<Stmt>),
    ReturnStmt(Option<Expr>),
    BreakStmt,
    ContinueStmt,
    ExprStmt(Expr),
    GetImportStmt(String, String, Option<String>, Vec<String>),
    IncludeStmt(String),
    UsingStmt(String),
    DirectBlock(Vec<String>),
    ManualBlock(Vec<Stmt>),
    GcPragma(String),
    StructDecl(String, Vec<Param>, Vec<String>),
    EnumDecl(String, Vec<EnumVariant>, Vec<String>),
    MatchStmt(Expr, Vec<MatchArm>),
    ImplDecl(String, Vec<Stmt>, Vec<String>),
}

// Caly program - lista instrukcji/deklaracji najwyzszego poziomu.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub body: Vec<Stmt>,
}

impl Program {
    pub fn new(body: Vec<Stmt>) -> Self {
        Program { body }
    }
}

// Demonstracyjne uzycie - buduje reczne AST dla `fun add(a: Int, b:
// Int) -> Int [ end a + b ]` (bez przechodzenia przez parser.hcs -
// ten plik jest samodzielny, testowany osobno) i wypisuje liczbe
// instrukcji.
pub fn main() {
    let mut int_type: TypeRef = TypeRef::new("Int".to_string(), None, None);
    let mut a_param: Param = Param::new("a".to_string(), Some(int_type.clone()), None);
    let mut b_param: Param = Param::new("b".to_string(), Some(int_type.clone()), None);
    let mut body: Vec<Stmt> = vec![Stmt::ReturnStmt(Some(Expr::BinOp("+".to_string(), Box::new(Expr::IdentExpr("a".to_string())), Box::new(Expr::IdentExpr("b".to_string())))))];
    let mut f: Stmt = Stmt::FunDecl("add".to_string(), vec![a_param, b_param], Some((int_type).clone()), (body).clone(), false, vec![]);
    let mut prog: Program = Program::new(vec![f]);
    println!("{} {}", "instrukcje w programie:".to_string(), (prog.body.len() as i64));
}

// ## Ograniczenia tej wersji (patrz docs/ROADMAP.md)
// 
// - `Expr::IdentExpr` jest nazwane INACZEJ niz Pythonowe `class Ident`
// - `TokKind::Ident` (lexer.hcs) i `Expr::Ident` kolidowalyby jako
// dwa warianty o tej samej nazwie w dwoch roznych enumach, co ten
// bootstrap ZABRANIA globalnie (patrz E0010 - nazwy wariantow musza
// byc unikalne w CALYM programie, nie tylko w obrebie jednego enuma,
// dokladnie ten sam rodzaj kolizji co juz udokumentowany gdzie
// indziej w docs/ROADMAP.md). `TokKind::Ident` zostalo, bo ma wiecej
// miejsc uzycia (lexer.hcs + caly parser.hcs); zmienic nazwe
// musialo `Expr::IdentExpr`.
// - Brak pola `line: Int` na kazdym wezle (Pythonowe `Node.line`) -
// `parser.hcs` (kolejny plik) rowniez go dzis nie ustawia - komunikaty
// bledow z `typecheck.hcs` (przyszly krok) beda musialy sobie radzic
// bez precyzyjnych numerow linii, albo ten brak trzeba bedzie
// nadrobic pozniej (osobny krok, NIE ten).
// - `StringLit` ma DODATKOWE pole `is_doc: Bool`, ktorego NIE MA w
// Pythonowej dataclass `StringLit` (tam `_is_doc = True` jest
// dynamicznym atrybutem dolepianym po konstrukcji WYLACZNIE dla
// komentarzy dokumentacyjnych `!!` na poziomie instrukcji - patrz
// `parse_statement` w parser.py). HackerScript nie ma dynamicznych
// atrybutow struct, wiec to pole musialo stac sie CZESCIA
// definicji, nie dolepiane po fakcie.
// - `Param`/`LetStmt` uzywaja nazw `type_ref`/`default_expr` zamiast
// Pythonowych `type_`/`default` - `type`/`default` kolidowalyby z
// innymi uzyciami w tym samo-hostowanym kompilatorze (np. `type_ref`
// jako parametr funkcji gdzie indziej).
// - `ImplDecl.methods`/`impl`-blok w `DirectBlock.raw_lines` uzywaja
// `List<Stmt>`/`List<Str>` bez STATYCZNEGO wymuszenia "tylko
// FunDecl"/"tylko linie tekstu" - to (jak w Pythonie, gdzie `list`
// rowniez nie wymusza typu elementow) jest kontraktem parsera
// (`parser.hcs`), nie systemu typow.
// - NIEPRZETESTOWANE na prawdziwym wejsciu w tym srodowisku (brak
// rustc) - zweryfikowane strukturalnie przez `hackerc check`/
// `build` i inspekcje wygenerowanego Rusta, patrz
// tests/test_hackerc.py.

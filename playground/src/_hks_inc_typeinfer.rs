#![allow(non_snake_case, unused_mut, dead_code)]

//! bootstrap/hackerc-self/typeinfer.hcs
//! 
//! Krok 4/N przepisania calego hackerc na HackerScript (patrz
//! docs/ROADMAP.md, "W TOKU"). Parytet z hackerc/hackerc/typeinfer.py
//! (193 linie) - inferencja typu `let x = wyrazenie` BEZ jawnej
//! adnotacji: literaly, arytmetyka, porownania, wywolania znanych
//! funkcji/konstruktorow, listy, `.pole`, `x[i]`, `as`, `?`.
//! 
//! Kompilowany dzis przez STAGE0 (Pythonowy hackerc) - patrz
//! bootstrap/README.md.
//! 
//! ## WAZNA UWAGA PROJEKTOWA - dlaczego kilka funkcji tutaj wyglada
//! niestandardowo w stosunku do Pythona
//! 
//! `Expr`/`Stmt` sa TAGOWANYMI UNIAMI (Rust `enum`) bez nazwanych pol -
//! w przeciwienstwie do Pythonowych dataclass, NIE MA `expr.callee`/
//! `fdecl.ret_type` skladni - jedyny sposob wyciagniecia danych z
//! wariantu to `match`. To samo w sobie jest OK, ale RODZI PROBLEM
//! gdy pole wariantu jest SAMO-REFERENCYJNE (np. `Call(Expr, ...)`,
//! ktore w Ruscie jest `Call(Box<Expr>, ...)` - PATRZ ast_nodes.hcs):
//! dopasowywanie WZORCA enuma na wartosci typu `Box<Expr>` (a nie
//! `Expr`) wymagaloby jawnego `*` (dereferencji), ktorego HackerScript
//! NIE MA (brak takiego operatora w gramatyce - patrz parser.py). Za
//! to WYWOLANIA FUNKCJI, gdzie parametr jest typu `Expr` (co ten
//! codegen ZAWSZE przekazuje jako `&Expr`), DZIALAJA poprawnie z
//! wartoscia `Box<Expr>` dzieki standardowej koercji Rusta
//! `&Box<T> -> &T` (przy PRZEKAZYWANIU ARGUMENTU, nie dopasowywaniu
//! wzorca). Dlatego: gdziekolwiek trzeba "zajrzec" w KSZTALT
//! zagniezdzonego pod-wyrazenia (np. `Call.callee` jest `IdentExpr`
//! czy `Attr`?), robie to przez ODDZIELNE WYWOLANIE FUNKCJI (nie
//! zagniezdzony `match` na juz-dopasowanej zmiennej) - patrz
//! `expr_as_ident_name`/`expr_call_shape` nizej.
//! 
//! Ten sam problem (i to samo rozwiazanie) dotyczy `TypeRef.generic`/
//! `.generic2` (rowniez samo-referencyjne, `Option<Box<TypeRef>>` w
//! Ruscie) - `types_equal` PONIZEJ jest CELOWO w calosci jedna funkcja
//! z zagniezdzonymi `match`-ami NA WYRAZENIACH DOSTEPU DO POLA
//! (`ta.generic`), NIE przekazuje `.generic` do INNEJ funkcji (co
//! wymagaloby koercji `Option<Box<TypeRef>> -> Option<TypeRef>`,
//! ktorej Rust NIE ROBI automatycznie dla `Option` - w przeciwienstwie
//! do referencji, `Option`-owy parametr generyczny nie jest
//! kowariantny/nie ma automatycznego odpakowania Box). Ograniczenie:
//! porownuje generyki tylko do 1 poziomu zagniezdzenia (patrz
//! "Ograniczenia" - w praktyce w tym repo generyki nigdy nie sa
//! zagniezdzone glebiej).
use crate::_hks_inc_ast_nodes::*;
pub fn is_numeric_type_name(n: &String) -> bool {
    return ((n.to_string() == "Int".to_string().to_string()) || (n.to_string() == "Float".to_string().to_string()));
}

pub fn make_type0(name: &String) -> TypeRef {
    return TypeRef::new((name).to_string(), None, None);
}

pub fn make_type1(name: &String, generic: &TypeRef) -> TypeRef {
    return TypeRef::new((name).to_string(), Some((generic).clone()), None);
}

pub fn make_type2(name: &String, generic: &TypeRef, generic2: &TypeRef) -> TypeRef {
    return TypeRef::new((name).to_string(), Some((generic).clone()), Some((generic2).clone()));
}

// `"." in value` - reczna wersja (bez operatora `in` na Str w tej
// wersji bootstrapu).
pub fn str_has_dot(s: &String) -> bool {
    let mut i: i64 = 0;
    let mut n = (s.len() as i64);
    while (i < n) {
        if ((s.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == ".".to_string().to_string()) {
            return true;
        }
        i = (i + 1);
    }
    return false;
}

// -- Signatures -------------------------------------------------------
// Sygnatury calego programu, zebrane Z GORY (wywolania moga
// poprzedzac deklaracje w pliku) - parytet z klasa `Signatures` w
// typeinfer.py. Przechowuje CALE `Stmt` (np. `FunDecl`), nie same
// typy zwracane - odczyt konkretnego pola wymaga funkcji
// pomocniczej (patrz `fun_decl_ret_type` itd. nizej), bo `Stmt` to
// tagowana unia bez nazwanych pol.
// 
// `methods` uzywa klucza `"NazwaStruct::nazwa_metody"` (Str) zamiast
// Pythonowej krotki `(str, str)` - HackerScript nie ma typu krotki.
// `variant_owner` (klucz: nazwa wariantu enuma, wartosc: nazwa
// enuma) NIE MA odpowiednika w Pythonie - tam `infer_expr_type`
// po prostu ITERUJE `env.sigs.enums.items()` szukajac wariantu; ten
// bootstrap NIE MA iteracji po Dict (brak `.keys()`/`.values()`/
// `.items()` - patrz "Ograniczenia"), wiec ta odwrotna mapa jest
// budowana Z GORY podczas `collect_signatures`, zeby pozniejsze
// wyszukiwanie bylo pojedynczym `.fetch()`.
// DOPISANE w kolejnej sesji (`codegen.hcs`, krok 6/N): `struct_names`/
// `enum_names`/`function_names` (List<Str>, w kolejnosci deklaracji)
// - `codegen.hcs` musi ITEROWAC WSZYSTKIE structy/enumy (np. zeby
// zbudowac graf rekurencji Box), ale ten bootstrap NIE MA iteracji po
// Dict (`.keys()`/`.values()`/`.items()`) - te listy sa jedynym
// sposobem, zeby pozniej "przejrzec" zawartosc `structs`/`enums`/
// `functions` (Dict daje tylko `.fetch(known_key)`, nie enumeracje).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Signatures {
    pub functions: std::collections::HashMap<String, Stmt>,
    pub structs: std::collections::HashMap<String, Stmt>,
    pub enums: std::collections::HashMap<String, Stmt>,
    pub methods: std::collections::HashMap<String, Stmt>,
    pub variant_owner: std::collections::HashMap<String, String>,
    pub struct_names: Vec<String>,
    pub enum_names: Vec<String>,
    pub function_names: Vec<String>,
}

impl Signatures {
    pub fn new(functions: std::collections::HashMap<String, Stmt>, structs: std::collections::HashMap<String, Stmt>, enums: std::collections::HashMap<String, Stmt>, methods: std::collections::HashMap<String, Stmt>, variant_owner: std::collections::HashMap<String, String>, struct_names: Vec<String>, enum_names: Vec<String>, function_names: Vec<String>) -> Self {
        Signatures { functions, structs, enums, methods, variant_owner, struct_names, enum_names, function_names }
    }
}

pub fn fun_decl_ret_type(f: &Stmt) -> Option<TypeRef> {
    match f {
        Stmt::FunDecl(name, params, ret_type, body, is_pub, type_params) => {
            return ret_type.clone();
        }
        _ => {
            return None;
        }
    }
}

pub fn struct_decl_fields(s: &Stmt) -> Vec<Param> {
    match s {
        Stmt::StructDecl(name, fields, type_params) => {
            return fields.clone();
        }
        _ => {
            return vec![];
        }
    }
}

// Buduje `Signatures` z calego programu. Idzie po `prog.body` PLUS,
// dla kazdego `ImplDecl`, po jego `methods` (zawsze `FunDecl` z
// konwencji parsera - patrz "Ograniczenia" w ast_nodes.hcs).
// 
// Klonuje `stmt`/`m` PRZED dopasowaniem (`stmt.clone()`) - inaczej
// wstawienie CALEGO `stmt` do Dict WEWNATRZ galezi `match stmt [...]`
// byloby "uzyciem czesciowo przeniesionej wartosci" (dopasowanie
// wyciaga pola jak `name` przez przeniesienie, wiec `stmt` sam w
// sobie staje sie niedostepny w tej samej galezi) - Rust by to
// odrzucil.
pub fn collect_signatures(prog: &Program) -> Signatures {
    let mut functions: std::collections::HashMap<String, Stmt> = std::collections::HashMap::new();
    let mut structs: std::collections::HashMap<String, Stmt> = std::collections::HashMap::new();
    let mut enums: std::collections::HashMap<String, Stmt> = std::collections::HashMap::new();
    let mut methods: std::collections::HashMap<String, Stmt> = std::collections::HashMap::new();
    let mut variant_owner: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut struct_names: Vec<String> = vec![];
    let mut enum_names: Vec<String> = vec![];
    let mut function_names: Vec<String> = vec![];
    let mut i: i64 = 0;
    let mut n = (prog.body.len() as i64);
    while (i < n) {
        let mut stmt: Stmt = prog.body[i as usize].clone();
        let mut stmt_copy = stmt.clone();
        match stmt {
            Stmt::FunDecl(name, params, ret_type, body, is_pub, type_params) => {
                functions.insert(name.clone(), stmt_copy);
                function_names.push(name);
            }
            Stmt::StructDecl(name, fields, type_params) => {
                structs.insert(name.clone(), stmt_copy);
                struct_names.push(name);
            }
            Stmt::EnumDecl(name, variants, type_params) => {
                enums.insert(name.clone(), stmt_copy);
                enum_names.push(name.clone());
                let mut k: i64 = 0;
                let mut vn = (variants.len() as i64);
                while (k < vn) {
                    let mut v = variants[k as usize].clone();
                    variant_owner.insert(v.name, name.clone());
                    k = (k + 1);
                }
            }
            Stmt::ImplDecl(struct_name, impl_methods, type_params) => {
                let mut j: i64 = 0;
                let mut mn = (impl_methods.len() as i64);
                while (j < mn) {
                    let mut m = impl_methods[j as usize].clone();
                    let mut m_copy = m.clone();
                    match m {
                        Stmt::FunDecl(mname, mparams, mret, mbody, mis_pub, mtype_params) => {
                            methods.insert(format!("{}{}", format!("{}{}", struct_name.clone(), "::".to_string()), mname), m_copy);
                        }
                        _ => {
                        }
                    }
                    j = (j + 1);
                }
            }
            _ => {
            }
        }
        i = (i + 1);
    }
    return Signatures::new((functions).clone(), (structs).clone(), (enums).clone(), (methods).clone(), (variant_owner).clone(), (struct_names).clone(), (enum_names).clone(), (function_names).clone());
}

// -- TypeEnv -----------------------------------------------------------
// Srodowisko typow zmiennych lokalnych - jedno na funkcje (bootstrap:
// bez blokowego scoping, tak jak parytet Pythona). `vars` przechowuje
// `Option<TypeRef>` jako WARTOSC (nie tylko `TypeRef`) - odpowiada
// Pythonowemu `dict[str, TypeRef | None]` (zmienna MOZE byc
// zadeklarowana bez znanego typu, np. `let x = f()` gdzie `f` zwraca
// Void/nieznany typ).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeEnv {
    pub sigs: Signatures,
    pub vars: std::collections::HashMap<String, Option<TypeRef>>,
}

impl TypeEnv {
    pub fn new(sigs: Signatures, vars: std::collections::HashMap<String, Option<TypeRef>>) -> Self {
        TypeEnv { sigs, vars }
    }
}

impl TypeEnv {
    pub fn declare(&mut self, name: &String, type_: Option<TypeRef>) {
        self.vars.insert((name).to_string(), type_);
    }

    /// `self.vars.fetch(name)` zwraca `Option<Option<TypeRef>>` (Dict
    /// samo w sobie opakowuje w Option dla "klucz istnieje?", a
    /// WARTOSC juz jest `Option<TypeRef>`) - splaszcza obie warstwy do
    /// JEDNEGO `Option<TypeRef>`, parytet z Pythonowym `dict.get(name)`
    /// (ktore zwraca `None` zarowno gdy klucza nie ma, jak i gdy
    /// wartosc jest jawnie `None`).
    pub fn lookup(&self, name: &String) -> Option<TypeRef> {
        match self.vars.get(name.as_str()).cloned() {
            Some(inner) => {
                return inner;
            }
            None => {
                return None;
            }
        }
    }

    pub fn is_declared(&self, name: &String) -> bool {
        return self.vars.contains_key(name.as_str());
    }

}

// Rownosc typow uwzgledniajaca `Any` jako "nieznany, do
// wywnioskowania z kontekstu" (np. element pustej listy `[]`) -
// dopasowanie ZAWSZE prawdziwe wobec `Any`, zeby np. `let xs:
// List<Token> = []` nie falszywie nie zgadzalo sie typow (`List<Any>`
// != `List<Token>`) - dokladnie ten sam bug/fix co w typeinfer.py
// (patrz komentarz przy `_types_equal` tam), teraz odtworzony tu.
// 
// Patrz uwaga projektowa na gorze pliku co do tego, DLACZEGO ta cala
// funkcja jest jedna funkcja z zagniezdzonymi `match`-ami zamiast
// wywolan rekurencyjnych na `.generic`/`.generic2`.
pub fn types_equal(a: Option<TypeRef>, b: Option<TypeRef>) -> bool {
    match a {
        Some(ta) => {
            match b {
                Some(tb) => {
                    if ((ta.name.to_string() == "Any".to_string().to_string()) || (tb.name.to_string() == "Any".to_string().to_string())) {
                        return true;
                    }
                    if (ta.name != tb.name) {
                        return false;
                    }
                    /// REKURENCYJNE wywolanie `types_equal` (nie plytkie
                    /// `ga.name != gb.name`) - inaczej `List<Any>` (typ
                    /// wywnioskowany pustej listy `[]`) vs `List<Diagnostic>`
                    /// (typ zadeklarowany) falszywie wypada jako NIErowne,
                    /// bo "Any" != "Diagnostic" na tym poziomie stringow,
                    /// mimo ze `types_equal` na SAMYM `Any` (gdyby
                    /// porownywane bezposrednio) zwrocilby `true`. Bug
                    /// znaleziony przy uzyciu skompilowanego stage1
                    /// (samo-hostowanego hackerc) do zbudowania cli.hcs -
                    /// Pythonowy `typecheck.py`/`typeinfer.py` NIE MIAL tego
                    /// bledu (tam `_types_equal` jest juz rekurencyjne,
                    /// patrz `typeinfer.py::_types_equal`) - `typeinfer.hcs`
                    /// mial PLYTSZA, niepelna kopie tej samej funkcji.
                    if !(types_equal((ta.generic).map(|b| *b), (tb.generic).map(|b| *b))) {
                        return false;
                    }
                    if !(types_equal((ta.generic2).map(|b| *b), (tb.generic2).map(|b| *b))) {
                        return false;
                    }
                    return true;
                }
                None => {
                    return false;
                }
            }
        }
        None => {
            /// `None` (brak generyku po OBU stronach - np. `generic2`
            /// kazdego prostego `List<X>`) MUSI byc traktowane jako
            /// ROWNE, nie "automatycznie false" - inaczej KAZDE
            /// porownanie `List<X>` (jeden poziom generyku, `generic2`
            /// zawsze `None`) wywoluje `types_equal(None, None)` dla
            /// `generic2` i falszywie wypada jako NIErowne. Druga
            /// polowa TEGO SAMEGO bledu co wyzej - `_types_equal` w
            /// Pythonie NIGDY nie rekuruje na `None` wprost (sprawdza
            /// `(a.generic is None) != (b.generic is None)` i rekuruje
            /// TYLKO gdy oba Some) - ta rekurencyjna wersja HCS musi
            /// wiec sama obsluzyc przypadek `None, None` -> `true`.
            match b {
                Some(_tb) => {
                    return false;
                }
                None => {
                    return true;
                }
            }
        }
    }
}

// -- Wywolania (Call) --------------------------------------------------
// Zwraca nazwe identyfikatora, jesli `e` to `IdentExpr`, inaczej
// pusty Str. Patrz uwaga projektowa na gorze pliku - to jest
// WYWOLANIE FUNKCJI (nie zagniezdzony match) specjalnie po to, zeby
// bezpiecznie obsluzyc `Box<Expr>` przychodzacy z zewnetrznego
// `match Call(callee, args)`.
pub fn expr_as_ident_name(e: &Expr) -> String {
    let mut e2 = e.clone();
    match e2 {
        Expr::IdentExpr(name) => {
            return name.to_string();
        }
        _ => {
            return "".to_string();
        }
    }
}

// Ksztalt `wyrazenie.metoda` uzywany TYLKO do inferencji typu
// wywolania - CELOWO nie przechowuje surowego `target: Expr` (co
// zderzyloby sie z tym samym problemem co on sam mial rozwiazac -
// `Box<Expr>` wstawiany w pole zadeklarowane jako zwykle `Expr`),
// tylko JUZ WYLICZONY typ celu (`target_type: Option<TypeRef>`,
// bezpieczny, bo nie jest czescia grafu cykli `Expr`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AttrCallShape {
    pub is_attr_call: bool,
    pub method_name: String,
    pub target_type: Option<TypeRef>,
}

impl AttrCallShape {
    pub fn new(is_attr_call: bool, method_name: String, target_type: Option<TypeRef>) -> Self {
        AttrCallShape { is_attr_call, method_name, target_type }
    }
}

pub fn expr_call_shape(e: &Expr, env: &TypeEnv) -> AttrCallShape {
    let mut e2 = e.clone();
    match e2 {
        Expr::Attr(target, name) => {
            let mut tt: Option<TypeRef> = infer_expr_type(&target, &env);
            return AttrCallShape::new(true, name, tt);
        }
        _ => {
            return AttrCallShape::new(false, "".to_string(), None);
        }
    }
}

// Wywolanie przez IDENTYFIKATOR: `f(...)`, `Struct(...)`,
// `WariantEnuma(...)`, wbudowane (`log`/`read_file`/`write_file`/
// `some`/`none`/`ok`/`err`/`dict`).
pub fn infer_ident_call_type(name: &String, env: &TypeEnv) -> Option<TypeRef> {
    if (name.to_string() == "log".to_string().to_string()) {
        return None;
    }
    if (name.to_string() == "read_file".to_string().to_string()) {
        return Some(make_type2(&"Result".to_string(), &make_type0(&"Str".to_string()), &make_type0(&"Str".to_string())));
    }
    if (name.to_string() == "write_file".to_string().to_string()) {
        return Some(make_type2(&"Result".to_string(), &make_type0(&"Void".to_string()), &make_type0(&"Str".to_string())));
    }
    if (name.to_string() == "env_var".to_string().to_string()) {
        return Some(make_type1(&"Option".to_string(), &make_type0(&"Str".to_string())));
    }
    if (name.to_string() == "run_command".to_string().to_string()) {
        return Some(make_type2(&"Result".to_string(), &make_type0(&"Str".to_string()), &make_type0(&"Str".to_string())));
    }
    if (name.to_string() == "http_get".to_string().to_string()) {
        return Some(make_type2(&"Result".to_string(), &make_type0(&"Str".to_string()), &make_type0(&"Str".to_string())));
    }
    if (((((name.to_string() == "some".to_string().to_string()) || (name.to_string() == "none".to_string().to_string())) || (name.to_string() == "ok".to_string().to_string())) || (name.to_string() == "err".to_string().to_string())) || (name.to_string() == "dict".to_string().to_string())) {
        return None;
    }
    match env.sigs.functions.get(name.as_str()).cloned() {
        Some(f) => {
            return fun_decl_ret_type(&f);
        }
        None => {
        }
    }
    if env.sigs.structs.contains_key(name.as_str()) {
        return Some(make_type0(&name));
    }
    match env.sigs.variant_owner.get(name.as_str()).cloned() {
        Some(enum_name) => {
            return Some(make_type0(&enum_name));
        }
        None => {
            return None;
        }
    }
}

// Wywolanie metody: `.fetch`/`.remove` na `Dict<K,V>` -> `Option<V>`,
// `.char_at`/`.slice` na `Str` -> `Str`, w przeciwnym razie
// zadeklarowany typ zwracany metody uzytkownika z `impl` (jesli
// `target_type` jest znanym structem majacym taka metode) - parytet z
// `elif isinstance(callee, A.Attr): ... env.sigs.methods.get((target_t.name,
// callee.name))` w typeinfer.py. BRAK tego fallbacku byl bugiem
// znalezionym przy uzyciu skompilowanego stage1 (samo-hostowanego
// hackerc) do zbudowania cli.hcs w tej sesji - `formatter.hcs::fmt_expr`
// wola `self.fmt_expr(...)` (metode uzytkownika), a bez tego fallbacku
// `infer_expr_type` zwracal `None` dla takiego wywolania, co psulo
// wykrywanie konkatenacji Str (`expr_is_strish`) w `op + self.fmt_expr(
// operand)` i generowalo niekompilujacy sie Rust (`&String + String`).
pub fn infer_attr_call_type(method_name: &String, target_type: Option<TypeRef>, sigs: &Signatures) -> Option<TypeRef> {
    match target_type {
        Some(tt) => {
            if (((method_name.to_string() == "fetch".to_string().to_string()) || (method_name.to_string() == "remove".to_string().to_string())) && (tt.name.to_string() == "Dict".to_string().to_string())) {
                match tt.generic2 {
                    Some(g2) => {
                        return Some(make_type1(&"Option".to_string(), &g2));
                    }
                    None => {
                    }
                }
            }
            if (((method_name.to_string() == "char_at".to_string().to_string()) || (method_name.to_string() == "slice".to_string().to_string())) && (tt.name.to_string() == "Str".to_string().to_string())) {
                return Some(make_type0(&"Str".to_string()));
            }
            match sigs.methods.get(format!("{}{}", format!("{}{}", tt.name.clone(), "::".to_string()), method_name).as_str()).cloned() {
                Some(fn_stmt) => {
                    match fn_stmt {
                        Stmt::FunDecl(fname, fparams, fret, fbody, fis_pub, ftype_params) => {
                            return fret;
                        }
                        _ => {
                        }
                    }
                }
                None => {
                }
            }
            return None;
        }
        None => {
            return None;
        }
    }
}

// -- Glowna funkcja: infer_expr_type -----------------------------------
// Parytet z `infer_expr_type(expr, env)` w typeinfer.py. Klonuje `expr`
// do wlasnej, WLASNEJ (owned) lokalnej zmiennej NA STARCIE (patrz
// uwaga projektowa na gorze pliku) - dzieki temu KAZDE dopasowanie w
// tej funkcji jest na wartosci OWNED, wiec zwykle pola (nie-Boxowane,
// np. `NumberLit`'s `Str`) sa bezposrednio uzywalne, a pola Boxowane
// (`Expr` w `BinOp`/`Call`/`Attr`/itd.) sa przekazywane DALEJ tylko
// jako ARGUMENTY WYWOLAN (bezpieczne dzieki koercji Rusta), nigdy
// nie dopasowywane bezposrednio.
pub fn infer_expr_type(expr: &Expr, env: &TypeEnv) -> Option<TypeRef> {
    let mut e = expr.clone();
    match e {
        Expr::NumberLit(value) => {
            if str_has_dot(&value) {
                return Some(make_type0(&"Float".to_string()));
            }
            return Some(make_type0(&"Int".to_string()));
        }
        Expr::StringLit(value, is_doc) => {
            return Some(make_type0(&"Str".to_string()));
        }
        Expr::BoolLit(value) => {
            return Some(make_type0(&"Bool".to_string()));
        }
        Expr::NullLit => {
            return None;
        }
        Expr::IdentExpr(name) => {
            return env.lookup(&name);
        }
        Expr::UnaryOp(op, operand) => {
            if (op.to_string() == "not".to_string().to_string()) {
                return Some(make_type0(&"Bool".to_string()));
            }
            return infer_expr_type(&operand, &env);
        }
        Expr::BinOp(op, left, right) => {
            if ((((((((op.to_string() == "and".to_string().to_string()) || (op.to_string() == "or".to_string().to_string())) || (op.to_string() == "==".to_string().to_string())) || (op.to_string() == "!=".to_string().to_string())) || (op.to_string() == "<".to_string().to_string())) || (op.to_string() == ">".to_string().to_string())) || (op.to_string() == "<=".to_string().to_string())) || (op.to_string() == ">=".to_string().to_string())) {
                return Some(make_type0(&"Bool".to_string()));
            }
            let mut lt: Option<TypeRef> = infer_expr_type(&left, &env);
            let mut rt: Option<TypeRef> = infer_expr_type(&right, &env);
            match lt {
                Some(ltv) => {
                    match rt {
                        Some(rtv) => {
                            if (((op.to_string() == "+".to_string().to_string()) && (ltv.name.to_string() == "Str".to_string().to_string())) && (rtv.name.to_string() == "Str".to_string().to_string())) {
                                return Some(make_type0(&"Str".to_string()));
                            }
                            if (is_numeric_type_name(&ltv.name) && is_numeric_type_name(&rtv.name)) {
                                if ((ltv.name.to_string() == "Float".to_string().to_string()) || (rtv.name.to_string() == "Float".to_string().to_string())) {
                                    return Some(make_type0(&"Float".to_string()));
                                }
                                return Some(make_type0(&"Int".to_string()));
                            }
                            return None;
                        }
                        None => {
                            return None;
                        }
                    }
                }
                None => {
                    return None;
                }
            }
        }
        Expr::Call(callee, args) => {
            let mut ident_name: String = expr_as_ident_name(&callee);
            if (ident_name.to_string() != "".to_string().to_string()) {
                return infer_ident_call_type(&ident_name, &env);
            }
            let mut shape: AttrCallShape = expr_call_shape(&callee, &env);
            if shape.is_attr_call {
                return infer_attr_call_type(&shape.method_name, shape.target_type, &env.sigs);
            }
            return None;
        }
        Expr::ListLit(items) => {
            if ((items.len() as i64) == 0) {
                return Some(make_type1(&"List".to_string(), &make_type0(&"Any".to_string())));
            }
            let mut first: Option<TypeRef> = infer_expr_type(&items[0 as usize], &env);
            match first {
                Some(ft) => {
                    let mut all_same: bool = true;
                    let mut i: i64 = 1;
                    let mut n = (items.len() as i64);
                    while (i < n) {
                        let mut it: Option<TypeRef> = infer_expr_type(&items[i as usize], &env);
                        if !(types_equal(Some(ft.clone()), it)) {
                            all_same = false;
                        }
                        i = (i + 1);
                    }
                    if all_same {
                        return Some(make_type1(&"List".to_string(), &ft));
                    }
                    return Some(make_type1(&"List".to_string(), &make_type0(&"Any".to_string())));
                }
                None => {
                    return Some(make_type1(&"List".to_string(), &make_type0(&"Any".to_string())));
                }
            }
        }
        Expr::Attr(target, name) => {
            let mut target_t: Option<TypeRef> = infer_expr_type(&target, &env);
            match target_t {
                Some(tt) => {
                    match env.sigs.structs.get(tt.name.as_str()).cloned() {
                        Some(s) => {
                            let mut fields: Vec<Param> = struct_decl_fields(&s);
                            let mut i: i64 = 0;
                            let mut n = (fields.len() as i64);
                            while (i < n) {
                                let mut f = fields[i as usize].clone();
                                if (f.name.to_string() == name.to_string()) {
                                    return f.type_ref;
                                }
                                i = (i + 1);
                            }
                            return None;
                        }
                        None => {
                            return None;
                        }
                    }
                }
                None => {
                    return None;
                }
            }
        }
        Expr::Index(target, index) => {
            let mut target_t: Option<TypeRef> = infer_expr_type(&target, &env);
            match target_t {
                Some(tt) => {
                    if (tt.name.to_string() == "List".to_string().to_string()) {
                        return tt.generic.map(|b| *b);
                    }
                    return None;
                }
                None => {
                    return None;
                }
            }
        }
        Expr::Cast(target, type_ref) => {
            return Some(type_ref);
        }
        Expr::TryOp(target) => {
            let mut target_t: Option<TypeRef> = infer_expr_type(&target, &env);
            match target_t {
                Some(tt) => {
                    if ((tt.name.to_string() == "Result".to_string().to_string()) || (tt.name.to_string() == "Option".to_string().to_string())) {
                        return tt.generic.map(|b| *b);
                    }
                    return None;
                }
                None => {
                    return None;
                }
            }
        }
    }
}

// `log()` w tym codegen ZAWSZE uzywa `{}` (Display), NIGDY `{:?}`
// (Debug) - `_gen_log` w codegen.py nie rozroznia typu argumentu.
// Structy/enumy w tym bootstrapie dostaja `#[derive(..., Debug,
// ...)]`, ale NIGDY `Display` - wiec `log(jakis_struct_lub_enum)`
// (w tym `Option<TypeRef>`) NIE SKOMPILUJE SIE w prawdziwym Rust
// (E0277: `TypeRef`/`Option<TypeRef>` nie implementuje `Display`).
// Znalezione przy pisaniu tego pliku - `log()` jest wiec BEZPIECZNY
// tylko dla Int/Float/Str/Bool, NIGDY dla struct/enum/Option<...>
// bezposrednio. Ten maly helper konwertuje `Option<TypeRef>` na Str
// (tylko nazwa najwyzszego poziomu, bez generykow - wystarczy do
// demonstracji) PRZED przekazaniem do `log`.
pub fn type_opt_to_str(t: Option<TypeRef>) -> String {
    match t {
        Some(tv) => {
            return tv.name.clone();
        }
        None => {
            return "?".to_string();
        }
    }
}

// Demonstracyjne uzycie - buduje reczne `Signatures`/`TypeEnv` dla
// `struct Point [ x: Int, y: Int ]` + `fun make() -> Point [...]` i
// sprawdza inferencje dla `1 + 2` (Int), `1.5 + 1` (Float),
// `"a" + "b"` (Str), `make().x` (Int, przez pole struct).
pub fn main() {
    let mut point_fields: Vec<Param> = vec![Param::new("x".to_string(), Some(make_type0(&"Int".to_string())), None), Param::new("y".to_string(), Some(make_type0(&"Int".to_string())), None)];
    let mut point_decl: Stmt = Stmt::StructDecl("Point".to_string(), (point_fields).clone(), vec![]);
    let mut make_decl: Stmt = Stmt::FunDecl("make".to_string(), vec![], Some(make_type0(&"Point".to_string())), vec![], false, vec![]);
    let mut prog = Program::new(vec![point_decl, make_decl]);
    let mut sigs: Signatures = collect_signatures(&prog);
    let mut vars: std::collections::HashMap<String, Option<TypeRef>> = std::collections::HashMap::new();
    let mut env: TypeEnv = TypeEnv::new((sigs).clone(), (vars).clone());
    let mut e1: Expr = Expr::BinOp("+".to_string(), Box::new(Expr::NumberLit("1".to_string())), Box::new(Expr::NumberLit("2".to_string())));
    let mut e2: Expr = Expr::BinOp("+".to_string(), Box::new(Expr::NumberLit("1.5".to_string())), Box::new(Expr::NumberLit("1".to_string())));
    let mut e3: Expr = Expr::BinOp("+".to_string(), Box::new(Expr::StringLit("a".to_string(), false)), Box::new(Expr::StringLit("b".to_string(), false)));
    let mut e4: Expr = Expr::Attr(Box::new(Expr::Call(Box::new(Expr::IdentExpr("make".to_string())), vec![])), "x".to_string());
    println!("{} {}", "1+2 ->".to_string(), type_opt_to_str(infer_expr_type(&e1, &env)));
    println!("{} {}", "1.5+1 ->".to_string(), type_opt_to_str(infer_expr_type(&e2, &env)));
    println!("{} {}", "a+b ->".to_string(), type_opt_to_str(infer_expr_type(&e3, &env)));
    println!("{} {}", "make().x ->".to_string(), type_opt_to_str(infer_expr_type(&e4, &env)));
}

// ## Ograniczenia tej wersji (patrz docs/ROADMAP.md)
// 
// - `types_equal` porownuje generyki TYLKO do 1 poziomu zagniezdzenia
// (`ta.generic.name == tb.generic.name`, bez dalszej rekursji w
// `generic.generic`) - patrz uwaga projektowa na gorze pliku
// (dlaczego rekurencja przez wywolanie funkcji nie dziala tutaj).
// W praktyce w calym tym repo generyki nigdy nie sa zagniezdzone
// glebiej niz `Zewnetrzny<Wewnetrzny>` (np. `List<Int>`,
// `Dict<Str,Stmt>`, `Option<TypeRef>`), wiec to ograniczenie nie ma
// dzis praktycznego znaczenia.
// - Brak iteracji po `Dict` (`.keys()`/`.values()`/`.items()` nie
// istnieja w tym bootstrapie) - `Signatures.variant_owner` to
// obejscie zbudowane Z GORY podczas `collect_signatures` zamiast
// iterowac `enums` pozniej (jak w Pythonie).
// - `is_declared` uzywa `Dict.contains` (parytet z `name in
// self.vars` w Pythonie).
// - **Nowe, potwierdzone dzis ograniczenie codegen.py**: `log(...)`
// ZAWSZE uzywa formatu `{}` (Display), nigdy `{:?}` (Debug) -
// struct/enum w tym bootstrapie dostaja `Debug`, NIGDY `Display`,
// wiec `log(jakis_struct_lub_enum)` (w tym `Option<T>` gdzie T to
// struct/enum) NIE SKOMPILUJE SIE. `log()` jest bezpieczny TYLKO
// dla Int/Float/Str/Bool. Patrz `type_opt_to_str` powyzej.
// - NIEPODPIETE jeszcze pod `parser.hcs` (parser sam nie wywoluje
// `infer_expr_type` - to zrobi dopiero `typecheck.hcs`, kolejny
// krok, ktory rzeczywiscie UZYWA wynikow inferencji do
// wykrywania niezgodnosci typow).
// - NIEPRZETESTOWANE na prawdziwym wejsciu w tym srodowisku (brak
// rustc) - zweryfikowane strukturalnie przez `hackerc check`/
// `build` i BARDZO dokladna inspekcje wygenerowanego Rusta (patrz
// docs/ROADMAP.md - ten plik mial NAJWIECEJ potencjalnych pulapek
// typu Box/referencja ze wszystkich dotychczasowych krokow).

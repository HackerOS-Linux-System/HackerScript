#![allow(non_snake_case, unused_mut, dead_code)]

//! bootstrap/hackerc-self/codegen.hcs
//! 
//! Krok 6/N przepisania calego hackerc na HackerScript (patrz
//! docs/ROADMAP.md, "W TOKU"). Parytet z hackerc/hackerc/codegen.py
//! (1297 linii - NAJWIEKSZY i najbardziej zlozony modul). **TEN PLIK
//! JEST NIEKOMPLETNY** (patrz "Stan tej sesji" na koncu) - port
//! codegen.py to wielosesyjne przedsiewziecie samo w sobie. Ta sesja
//! dostarcza WARSTWE ANALIZY (renderowanie typow + wykrywanie
//! rekurencji/Box) - WARSTWA EMISJI (gen_expr/gen_stmt/gen_program,
//! wieksza polowa oryginalu) NIE ISTNIEJE JESZCZE.
//! 
//! Kompilowany dzis przez STAGE0 (Pythonowy hackerc) - patrz
//! bootstrap/README.md.
//! 
//! ## Dlaczego TEN plik jest inny niz poprzednie - implementuje
//! WLASNIE TE analize, ktora ta cala sesja robila RECZNIE
//! 
//! Kazdy poprzedni krok (typeinfer.hcs, typecheck.hcs, nawet ten plik
//! sam w sobie) musial OMIJAC pulapki Box/referencja/mut RECZNIE,
//! jedna po drugiej, w KAZDYM miejscu gdzie sie pojawily. `codegen.hcs`
//! to miejsce, gdzie te REGULY same staja sie kodem - `rust_type_name`
//! i `RecursionAnalyzer` PONIZEJ to dokladnie te algorytmy, ktore w
//! Pythonowym `codegen.py` DECYDUJA, KIEDY wstawic `Box<...>` - a
//! zeby je NAPISAC, trzeba bylo zastosowac WSZYSTKIE dotychczas
//! odkryte zasady (patrz `rust_type_name` nizej - rekurencja przez
//! GRANICE WYWOLANIA FUNKCJI, nigdy przez bezposrednie dopasowanie
//! Boxowanego pola, dokladnie jak w typeinfer.hcs).
use crate::_hks_inc_ast_nodes::*;
use crate::_hks_inc_typeinfer::*;
use crate::_hks_inc_typecheck::*;
pub fn list_contains_str(xs: &Vec<String>, s: &String) -> bool {
    let mut i: i64 = 0;
    let mut n = (xs.len() as i64);
    while (i < n) {
        if (xs[i as usize].clone().to_string() == s.to_string()) {
            return true;
        }
        i = (i + 1);
    }
    return false;
}

// Reczne wyszukiwanie podciagu (bez wbudowanej metody "contains" na
// Str w tym bootstrapie) - potrzebne przez `python_raw_string`.
pub fn str_contains_substring(haystack: &String, needle: &String) -> bool {
    let mut hn = (haystack.len() as i64);
    let mut nn = (needle.len() as i64);
    if (nn == 0) {
        return true;
    }
    if (nn > hn) {
        return false;
    }
    let mut i: i64 = 0;
    while (i <= (hn - nn)) {
        let mut j: i64 = 0;
        let mut matched: bool = true;
        while ((j < nn) && matched) {
            if ((haystack.chars().nth((i + j) as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() != (needle.chars().nth(j as usize).map(|c| c.to_string()).unwrap_or_default()).to_string()) {
                matched = false;
            }
            j = (j + 1);
        }
        if matched {
            return true;
        }
        i = (i + 1);
    }
    return false;
}

// Rust raw string literal `r#"..."#`, ktory bezpiecznie pomiesci
// DOWOLNY kod Pythona (w tym cudzyslowy) - parytet z
// `_python_raw_string()`. Dolicza tyle `#` ile potrzeba, zeby `"#..`
// nie wystapilo WEWNATRZ tresci (co przedwczesnie zamknelo by
// literal).
pub fn python_raw_string(s: &String) -> String {
    let mut hashes: String = "#".to_string();
    while str_contains_substring(&s, &format!("{}{}", "\"".to_string(), hashes)) {
        hashes = format!("{}{}", hashes, "#".to_string());
    }
    return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "r".to_string(), hashes), "\"".to_string()), s), "\"".to_string()), hashes).to_string();
}

pub fn char_to_upper(c: &String) -> String {
    let mut lower: String = "abcdefghijklmnopqrstuvwxyz".to_string();
    let mut upper: String = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string();
    let mut i: i64 = 0;
    let mut n = (lower.len() as i64);
    while (i < n) {
        if ((lower.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == c.to_string()) {
            return (upper.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default()).to_string();
        }
        i = (i + 1);
    }
    return c.to_string();
}

// Parytet z Pythonowym `.upper()` (uzywane przez nazwy stalych
// globalnych `const X = ...` -> `pub const X: ...`) - reczna wersja
// bez wbudowanej metody Str, ASCII-only (parytet ze skladnia
// identyfikatorow HackerScript, ktora i tak jest ASCII-only).
pub fn str_to_upper(s: &String) -> String {
    let mut out: String = "".to_string();
    let mut i: i64 = 0;
    let mut n = (s.len() as i64);
    while (i < n) {
        out = format!("{}{}", out, char_to_upper(&(s.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default())));
        i = (i + 1);
    }
    return out.to_string();
}

// Parytet z `flat_module_name()` w project.py - splaszczona,
// deterministyczna nazwa modulu Rust dla `get <std:...>`/
// `get <core:...>`/`get <selfhost:...>`. Male, samodzielne
// wyprzedzenie project.hcs (jeszcze nieistniejacego) - potrzebne TU,
// bo `gen_get_import` musi wyemitowac `use crate::<modul>::...;` w
// DOKLADNIE ten sam sposob, w jaki project.hcs kiedys nazwie
// faktyczne moduly (patrz "Ograniczenia").
pub fn flat_module_name(source: &String, name: &String, version: Option<String>) -> String {
    let mut out: String = format!("{}{}", format!("{}{}", format!("{}{}", "_hks_".to_string(), str_replace_dash(&source)), "_".to_string()), str_replace_dash(&name));
    match version {
        Some(v) => {
            out = format!("{}{}", format!("{}{}", out, "_".to_string()), str_replace_dash(&v));
        }
        None => {
        }
    }
    return out.to_string();
}

// Parytet z `flat_include_module_name` (project.py) - nazwa modulu
// dla `include <sciezka>` (bez zrodla/wersji, w odroznieniu od
// `flat_module_name` wyzej dla `get <...>`). Prefiks `_hks_inc_`
// (zamiast `_hks_`) zeby nigdy nie kolidowac z nazwa z `get <...>`.
pub fn flat_include_module_name(path: &String) -> String {
    let mut p: String = (path).to_string();
    let __hks_chars_p: Vec<char> = p.chars().collect();
    if (((p.len() as i64) >= 4) && (({ let __v = &__hks_chars_p; let __s = ((((p.len() as i64) - 4)) as usize).min(__v.len()); let __e = (((p.len() as i64)) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() }).to_string() == ".hcs".to_string().to_string())) {
        p = ({ let __v = &__hks_chars_p; let __s = ((0) as usize).min(__v.len()); let __e = ((((p.len() as i64) - 4)) as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() });
    }
    let mut out: String = "".to_string();
    let mut i: i64 = 0;
    let mut n = (p.len() as i64);
    while (i < n) {
        let mut c: String = (__hks_chars_p.get(i as usize).map(|c| c.to_string()).unwrap_or_default());
        if ((c.to_string() == "-".to_string().to_string()) || (c.to_string() == "/".to_string().to_string())) {
            out = format!("{}{}", out, "_".to_string());
        } else {
            out = format!("{}{}", out, c);
        }
        i = (i + 1);
    }
    return format!("{}{}", "_hks_inc_".to_string(), out).to_string();
}

pub fn str_replace_dash(s: &String) -> String {
    let mut out: String = "".to_string();
    let mut i: i64 = 0;
    let mut n = (s.len() as i64);
    while (i < n) {
        let mut c: String = (s.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default());
        if (c.to_string() == "-".to_string().to_string()) {
            out = format!("{}{}", out, "_".to_string());
        } else {
            out = format!("{}{}", out, c);
        }
        i = (i + 1);
    }
    return out.to_string();
}

// -- Renderowanie typow (parytet z `rust_type()`) ----------------------
pub fn is_scalar_type_name(n: &String) -> bool {
    return (((((n.to_string() == "Int".to_string().to_string()) || (n.to_string() == "Float".to_string().to_string())) || (n.to_string() == "Str".to_string().to_string())) || (n.to_string() == "Bool".to_string().to_string())) || (n.to_string() == "Void".to_string().to_string()));
}

// Parytet z `RUST_TYPE_MAP`.
pub fn rust_scalar_name(n: &String) -> String {
    if (n.to_string() == "Int".to_string().to_string()) {
        return "i64".to_string();
    }
    if (n.to_string() == "Float".to_string().to_string()) {
        return "f64".to_string();
    }
    if (n.to_string() == "Str".to_string().to_string()) {
        return "String".to_string();
    }
    if (n.to_string() == "Bool".to_string().to_string()) {
        return "bool".to_string();
    }
    return "()".to_string();
}

// Czy `t` ma ustawione `.generic` (odpowiednik `t.generic is not
// None` w Pythonie) - patrz uwaga projektowa na gorze pliku: `t` tu
// MOZE byc referencja (parametr funkcji), stad `.clone()` PRZED
// dopasowaniem.
pub fn type_ref_has_generic(t: &TypeRef) -> bool {
    match t.generic.clone() {
        Some(inner) => {
            return true;
        }
        None => {
            return false;
        }
    }
}

// **BUG ZNALEZIONY I NAPRAWIONY W TEJ SESJI** (patrz docs/ROADMAP.md):
// pierwotna wersja tej funkcji probowala ZWROCIC `inner` (typu
// `Box<TypeRef>`, bo `.generic` to WLASNE, samo-referencyjne pole
// TypeRef - Boxowane) jako `TypeRef` (odpakowane) - Rust NIE MA
// zadnego sposobu automatycznego "odpakowania" `Box<T>` -> `T` przy
// ZWRACANIU wartosci (w przeciwienstwie do `&Box<T> -> &T` przy
// PRZEKAZANIU ARGUMENTU) - i HackerScript nie ma operatora `*`, wiec
// nie da sie tego zrobic jawnie. **Nie istnieje bezpieczny sposob
// napisania funkcji "zwroc mi TypeRef z Option<TypeRef>-owego pola
// innego TypeRef" w tym bootstrapie** - jedyne bezpieczne operacje na
// `inner: Box<TypeRef>` to (a) dostep do POLA (`inner.name`, auto-
// derefuje) i (b) przekazanie go DALEJ jako ARGUMENT innej funkcji
// (koercja `Box<T> -> &T`). Dlatego ta funkcja zostala CALKOWICIE
// USUNIETA - w jej miejsce sa DWIE bezpieczne, wezsze funkcje nizej:
// `type_ref_generic_name` (zwraca tylko `.name`, Str - bezpieczne,
// bo Str NIE jest czescia grafu cykli TypeRef) i
// `rust_type_name_of_generic`/`_of_generic2` (laczy odpakowanie Z
// rekurencyjnym wywolaniem `rust_type_name` W JEDNYM kroku, bez
// nigdy "zwracania" samego TypeRef).
// Zwraca TYLKO `.generic.name` (Str) - bezpieczne, bo `Str` nie jest
// czescia grafu samo-referencji `TypeRef` (patrz uwaga wyzej).
pub fn type_ref_generic_name(t: &TypeRef) -> String {
    match t.generic.clone() {
        Some(inner) => {
            return format!("{}{}", inner.name, "".to_string()).to_string();
        }
        None => {
            return "".to_string();
        }
    }
}

pub fn type_ref_has_generic2(t: &TypeRef) -> bool {
    match t.generic2.clone() {
        Some(inner) => {
            return true;
        }
        None => {
            return false;
        }
    }
}

// Renderuje `TypeRef` na tekst typu Rust - parytet z `rust_type()`.
// `t` jest PARAMETREM (nie `self.pole`), wiec bezposrednie porownania
// `t.name == "..."` sa bezpieczne (borrow, nie ruch) - tylko
// DOPASOWANIE (`match`) `.generic`/`.generic2` wymaga obejscia przez
// `rust_type_name_of_generic`/`_of_generic2` (definicje PONIZEJ tej
// funkcji, ale HackerScript nie wymaga kolejnosci - patrz wczesniejsze
// przyklady wzajemnej rekurencji miedzy plikami tej sesji).
// 
// W przeciwienstwie do `rust_type()` w Pythonie, ktora RZUCA
// `CodegenError` na nieprawidlowy typ, ta wersja (bootstrap bez
// wyjatkow, patrz "Ograniczenia") po prostu `log`-uje ostrzezenie i
// zwraca najlepsze przyblizenie - PRAWDZIWE zglaszanie bledow czeka
// na podpiecie `diagnostics.hcs`.
// 
// **Bierze `sigs: Signatures` (NIE osobne `structs: Dict<...>`/
// `enums: Dict<...>`)** - `Dict<K,V>` NIE jest "refable" w tym
// codegen (`_is_refable` uznaje za referencyjne tylko `Str`/`List`/
// znane struct/enum - `Dict` pominiety, prawdopodobnie przeoczenie w
// oryginale), wiec parametr typu `Dict<...>` byl by przekazywany
// PRZEZ WARTOSC (przenoszony/moved) - a ta funkcja uzywa
// `structs`/`enums` WIELOKROTNIE w JEDNYM wywolaniu (np. renderujac
// `Dict<K,V>` trzeba dwa razy rekurencyjnie wywolac `rust_type_name`
// z TYMI SAMYMI mapami) - drugie uzycie po pierwszym `move`
// bylo by bledem kompilacji Rust ("value used after move"). `sigs:
// Signatures` OMIJA to, bo `Signatures` to STRUCT (WIEC "refable" -
// przekazywany jako `&Signatures`), a `sigs.structs.contains(...)`
// to tylko POZYCZENIE (`&sigs.structs`), nie przeniesienie - mozna
// go robic dowolnie wiele razy. **Bug znaleziony i naprawiony w TEJ
// sesji** - patrz docs/ROADMAP.md.
pub fn rust_type_name(t: &TypeRef, sigs: &Signatures, type_params: &Vec<String>) -> String {
    if list_contains_str(&type_params, &t.name) {
        return t.name.clone();
    }
    if (t.name.to_string() == "List".to_string().to_string()) {
        if !(type_ref_has_generic(&t)) {
            println!("{}", "[hackerc-self] codegen: 'List' wymaga typu elementu, np. List<Int>".to_string());
            return "Vec<()>".to_string();
        }
        return format!("{}{}", format!("{}{}", "Vec<".to_string(), rust_type_name_of_generic(&t, &sigs, &type_params)), ">".to_string()).to_string();
    }
    if (t.name.to_string() == "Dict".to_string().to_string()) {
        if (!(type_ref_has_generic(&t)) || !(type_ref_has_generic2(&t))) {
            println!("{}", "[hackerc-self] codegen: 'Dict' wymaga dwoch typow, np. Dict<Str, Int>".to_string());
            return "std::collections::HashMap<(), ()>".to_string();
        }
        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "std::collections::HashMap<".to_string(), rust_type_name_of_generic(&t, &sigs, &type_params)), ", ".to_string()), rust_type_name_of_generic2(&t, &sigs, &type_params)), ">".to_string()).to_string();
    }
    if (t.name.to_string() == "Option".to_string().to_string()) {
        if !(type_ref_has_generic(&t)) {
            println!("{}", "[hackerc-self] codegen: 'Option' wymaga typu wartosci, np. Option<Int>".to_string());
            return "Option<()>".to_string();
        }
        return format!("{}{}", format!("{}{}", "Option<".to_string(), rust_type_name_of_generic(&t, &sigs, &type_params)), ">".to_string()).to_string();
    }
    if (t.name.to_string() == "Result".to_string().to_string()) {
        if (!(type_ref_has_generic(&t)) || !(type_ref_has_generic2(&t))) {
            println!("{}", "[hackerc-self] codegen: 'Result' wymaga dwoch typow, np. Result<Str, Str>".to_string());
            return "Result<(), ()>".to_string();
        }
        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "Result<".to_string(), rust_type_name_of_generic(&t, &sigs, &type_params)), ", ".to_string()), rust_type_name_of_generic2(&t, &sigs, &type_params)), ">".to_string()).to_string();
    }
    if is_scalar_type_name(&t.name) {
        return rust_scalar_name(&t.name).to_string();
    }
    if sigs.structs.contains_key(t.name.as_str()) {
        if type_ref_has_generic(&t) {
            return format!("{}{}", format!("{}{}", format!("{}{}", t.name, "<".to_string()), rust_type_name_of_generic(&t, &sigs, &type_params)), ">".to_string()).to_string();
        }
        return t.name.clone();
    }
    if sigs.enums.contains_key(t.name.as_str()) {
        if type_ref_has_generic(&t) {
            return format!("{}{}", format!("{}{}", format!("{}{}", t.name, "<".to_string()), rust_type_name_of_generic(&t, &sigs, &type_params)), ">".to_string()).to_string();
        }
        return t.name.clone();
    }
    println!("{}", format!("{}{}", format!("{}{}", "[hackerc-self] codegen: nieznany typ '".to_string(), t.name), "' (dozwolone: Int, Float, Str, Bool, Void, List<T>, Dict<K,V>, Option<T>, Result<T,E>, lub nazwa znanego struct/enum/parametru generycznego)".to_string()));
    return t.name.clone();
}

// Odpakowuje `.generic`/`.generic2` I OD RAZU rekurencyjnie woa
// `rust_type_name` NA WYNIKU, wszystko w JEDNYM kroku - `inner: Box<
// TypeRef>` nigdy nie "ucieka" z tej funkcji jako zwracana wartosc
// (co bylo pierwotnym bledem - patrz uwaga przy usunietym
// `type_ref_generic` wyzej), tylko od razu jest PRZEKAZANE JAKO
// ARGUMENT do `rust_type_name` (bezpieczna koercja `Box<T> -> &T`).
pub fn rust_type_name_of_generic(t: &TypeRef, sigs: &Signatures, type_params: &Vec<String>) -> String {
    match t.generic.clone() {
        Some(inner) => {
            return rust_type_name(&inner, &sigs, &type_params).to_string();
        }
        None => {
            return "()".to_string();
        }
    }
}

pub fn rust_type_name_of_generic2(t: &TypeRef, sigs: &Signatures, type_params: &Vec<String>) -> String {
    match t.generic2.clone() {
        Some(inner) => {
            return rust_type_name(&inner, &sigs, &type_params).to_string();
        }
        None => {
            return "()".to_string();
        }
    }
}

// -- Wykrywanie rekurencji/Box (parytet z `_sizing_edge`/`_build_recursion_info`) --
// Jedna krawedz grafu rozmiaru: `owner`'s pole `field_key` wskazuje
// (przez wartosc) na `target`, `kind` to "direct" albo "option".
// `field_key` to nazwa pola dla structow, a `"WariantEnuma#idx"` dla
// pol enumow (Str zamiast Pythonowej krotki `(nazwa, idx)` - ten
// bootstrap nie ma typu krotki).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Edge {
    pub owner: String,
    pub field_key: String,
    pub target: String,
    pub kind: String,
}

impl Edge {
    pub fn new(owner: String, field_key: String, target: String, kind: String) -> Self {
        Edge { owner, field_key, target, kind }
    }
}

// Parytet z `_sizing_edge()` - `t` jest PARAMETREM (bezpieczne
// porownania `.name`, jak wyzej).
pub fn sizing_edge_target(t: &TypeRef) -> String {
    if (!(type_ref_has_generic(&t)) && !(type_ref_has_generic2(&t))) {
        return t.name.clone();
    }
    if ((t.name.to_string() == "Option".to_string().to_string()) && type_ref_has_generic(&t)) {
        return type_ref_generic_name(&t).to_string();
    }
    return "".to_string();
}

pub fn sizing_edge_kind(t: &TypeRef) -> String {
    if (!(type_ref_has_generic(&t)) && !(type_ref_has_generic2(&t))) {
        return "direct".to_string();
    }
    if ((t.name.to_string() == "Option".to_string().to_string()) && type_ref_has_generic(&t)) {
        return "option".to_string();
    }
    return "".to_string();
}

// Analizator grafu rekurencji - odpowiednik lokalnych zmiennych
// `color`/`boxed_edges`/`adjacency` w `_build_recursion_info()`,
// podniesiony do struct+`impl`, bo HackerScript nie ma domkniec
// (closures) - `dfs` musi byc METODA (rekurencyjna przez `self`,
// mutujaca `self.color`/`self.boxed_edges` - oba wzorce juz
// sprawdzone w poprzednich krokach).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecursionAnalyzer {
    pub adjacency: std::collections::HashMap<String, Vec<Edge>>,
    pub color: std::collections::HashMap<String, String>,
    pub boxed_edges: std::collections::HashMap<String, bool>,
}

impl RecursionAnalyzer {
    pub fn new(adjacency: std::collections::HashMap<String, Vec<Edge>>, color: std::collections::HashMap<String, String>, boxed_edges: std::collections::HashMap<String, bool>) -> Self {
        RecursionAnalyzer { adjacency, color, boxed_edges }
    }
}

impl RecursionAnalyzer {
    pub fn color_of(&self, node: &String) -> String {
        match self.color.get(node.as_str()).cloned() {
            Some(c) => {
                return c.to_string();
            }
            None => {
                return "white".to_string();
            }
        }
    }

    /// DFS z kolorowaniem (biale/szare/czarne) - parytet z zagniezdzonym
    /// `dfs()` w `_build_recursion_info()`. Krawedz do wezla SZAREGO
    /// (na biezacej sciezce DFS) zamyka cykl -> oznaczamy `(owner,
    /// field_key)` do zboxowania (klucz w `boxed_edges`:
    /// `owner + "::" + field_key`).
    pub fn dfs(&mut self, node: &String) {
        self.color.insert(node.clone(), "gray".to_string());
        match self.adjacency.get(node.clone().as_str()).cloned() {
            Some(edges) => {
                let mut i: i64 = 0;
                let mut n = (edges.len() as i64);
                while (i < n) {
                    let mut e = edges[i as usize].clone();
                    let mut tc: String = self.color_of(&e.target.clone());
                    if (tc.to_string() == "gray".to_string().to_string()) {
                        self.boxed_edges.insert(format!("{}{}", format!("{}{}", e.owner.clone(), "::".to_string()), e.field_key.clone()), true);
                    } else if (tc.to_string() == "white".to_string().to_string()) {
                        self.dfs(&e.target.clone());
                    }
                    i = (i + 1);
                }
            }
            None => {
            }
        }
        self.color.insert((node).to_string(), "black".to_string());
    }

}

// Wynik analizy rekurencji - parytet z `self.boxed_struct_fields`/
// `self.boxed_variant_fields` (splaszczone do JEDNEGO Dict, patrz
// "Ograniczenia" - Python uzywa Dict[str, Dict[str, str]], ten
// bootstrap nie ma latwych zagniezdzonych Dict, wiec klucz laczy oba
// poziomy: `"StructName::pole"` albo `"EnumName::Wariant#idx"`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecursionInfo {
    pub boxed_fields: std::collections::HashMap<String, String>,
}

impl RecursionInfo {
    pub fn new(boxed_fields: std::collections::HashMap<String, String>) -> Self {
        RecursionInfo { boxed_fields }
    }
}

// Buduje pola struct pol JEDNEGO struct do listy krawedzi (parytet z
// petla po `decl.fields` w `_build_recursion_info`).
pub fn struct_field_edges(owner: &String, fields: &Vec<Param>, known_structs: &Vec<String>, known_enums: &Vec<String>) -> Vec<Edge> {
    let mut out: Vec<Edge> = vec![];
    let mut i: i64 = 0;
    let mut n = (fields.len() as i64);
    while (i < n) {
        let mut f = fields[i as usize].clone();
        match f.type_ref {
            Some(ft) => {
                let mut target: String = sizing_edge_target(&ft.clone());
                if (list_contains_str(&known_structs, &target.clone()) || list_contains_str(&known_enums, &target.clone())) {
                    out.push(Edge::new(owner.clone(), (f.name).to_string(), (target).to_string(), sizing_edge_kind(&ft)));
                }
            }
            None => {
            }
        }
        i = (i + 1);
    }
    return out;
}

// Jak `struct_field_edges`, ale dla pol WARIANTOW jednego enuma
// (kazde pole kazdego wariantu, `field_key` = `"Wariant#idx"`).
pub fn enum_field_edges(owner: &String, variants: &Vec<EnumVariant>, known_structs: &Vec<String>, known_enums: &Vec<String>) -> Vec<Edge> {
    let mut out: Vec<Edge> = vec![];
    let mut i: i64 = 0;
    let mut n = (variants.len() as i64);
    while (i < n) {
        let mut v = variants[i as usize].clone();
        let mut j: i64 = 0;
        let mut fn_ = (v.fields.len() as i64);
        while (j < fn_) {
            let mut ft = v.fields[j as usize].clone();
            let mut target: String = sizing_edge_target(&ft.clone());
            if (list_contains_str(&known_structs, &target.clone()) || list_contains_str(&known_enums, &target.clone())) {
                out.push(Edge::new(owner.clone(), format!("{}{}", format!("{}{}", v.name.clone(), "#".to_string()), (j).to_string()), (target).to_string(), sizing_edge_kind(&ft)));
            }
            j = (j + 1);
        }
        i = (i + 1);
    }
    return out;
}

// Parytet z `struct_decl_fields`/`enum_decl_variants` gdyby istnialy
// w typeinfer.hcs pod tymi nazwami dla EnumDecl - potrzebne tutaj, bo
// typeinfer.hcs eksportuje tylko `struct_decl_fields` (uzywane tam
// do wnioskowania `.pole`), nie akcesor dla wariantow enuma.
pub fn enum_decl_variants(e: &Stmt) -> Vec<EnumVariant> {
    match e {
        Stmt::EnumDecl(name, variants, type_params) => {
            return variants.clone();
        }
        _ => {
            return vec![];
        }
    }
}

pub fn struct_decl_fields_local(s: &Stmt) -> Vec<Param> {
    match s {
        Stmt::StructDecl(name, fields, type_params) => {
            return fields.clone();
        }
        _ => {
            return vec![];
        }
    }
}

// Zwraca nazwy parametrow typu Str, na ktorych `.char_at`/`.slice`
// jest wywolywane WIELOKROTNIE (>=2 razy) gdziekolwiek w ciele -
// uzywane przez `gen_fun` do zdecydowania, ktore parametry oplaca sie
// zmaterializowac raz jako `Vec<char>` na poczatku funkcji, zamiast
// re-skanowac string od poczatku przy KAZDYM wywolaniu `.char_at(i)`
// (`.chars().nth(i)` w Rust jest O(i) - w petli to O(n^2) razem).
// `str_param_names` jako `List<Str>` (nie `Dict`) - ten bootstrap nie
// ma iteracji po Dict (patrz "Ograniczenia" w typeinfer.hcs), a
// potrzebujemy przejsc PO nazwach, nie tylko sprawdzac przynaleznosc.
// Parytet z `_char_indexed_str_params` (codegen.py). Bug wydajnosciowy
// znaleziony przy uzyciu skompilowanego stage1 (samo-hostowanego
// hackerc) do zbudowania duzych plikow w tej sesji.
// 
// Wewnetrzne `walk_*_for_char_indexing` ZWRACAJA listy trafien
// (`List<Str>`, jedna pozycja na wystapienie, duplikaty dozwolone)
// zamiast MUTOWAC wspolny Dict przekazywany przez wiele poziomow
// wywolan - auto-`&mut` w tym bootstrapie (patrz `compute_mut_params`)
// wykrywa TYLKO BEZPOSREDNIE `.insert()`/`.push()` na parametrze W
// TEJ SAMEJ funkcji, nie propaguje tranzytywnie przez wywolania innych
// funkcji - Dict mutowany przez funkcje WYWOLYWANA nie kompilowalby
// sie (E0596 "cannot borrow as mutable"). Zwracanie List<Str> i
// zliczanie na samej gorze (w tej funkcji) unika tego calkowicie -
// `counts` jest wtedy mutowany TYLKO lokalnie, w JEDNEJ funkcji.
// Zwraca zadeklarowany typ `let X = f(...)` gdy `X` NIE MA jawnej
// adnotacji typu, ale `f` jest ZNANA funkcja (w `sigs.functions`) -
// wtedy typem `let`-a jest typ zwracany `f`. Uzywane WYLACZNIE przez
// `char_indexed_str_params` do wykrycia lokalnych zmiennych Str typu
// `let src = strip_multiline_comments(source)` (BEZ adnotacji) -
// prostsze niz pelna inferencja typow (`infer_expr_type` + `TypeEnv`),
// ale wystarczajace dla TEGO konkretnego, najczesciej wystepujacego
// wzorca w calym bootstrapie. Bug znaleziony przy uzyciu
// skompilowanego stage1 na duzych plikach - `lexer.hcs::tokenize`
// (najgorszy przypadek) robi DOKLADNIE to na samym poczatku ciala.
pub fn char_str_type_of_let_call(s: &Stmt, sigs: &Signatures) -> Option<TypeRef> {
    let mut sc = s.clone();
    match sc {
        Stmt::LetStmt(lname, ltype, lvalue, lconst) => {
            match ltype {
                Some(t) => {
                    return Some(t);
                }
                None => {
                    match lvalue {
                        Some(v) => {
                            match v {
                                Expr::Call(callee, args) => {
                                    let mut fname: String = expr_as_ident_name(&callee);
                                    if (fname.to_string() != "".to_string().to_string()) {
                                        match sigs.functions.get(fname.as_str()).cloned() {
                                            Some(fn_stmt) => {
                                                match fn_stmt {
                                                    Stmt::FunDecl(n2, p2, ret2, b2, pub2, tp2) => {
                                                        return ret2;
                                                    }
                                                    _ => {
                                                    }
                                                }
                                            }
                                            None => {
                                            }
                                        }
                                    }
                                }
                                _ => {
                                }
                            }
                        }
                        None => {
                        }
                    }
                }
            }
        }
        _ => {
        }
    }
    return None;
}

pub fn char_indexed_str_params(body: &Vec<Stmt>, str_param_names: &Vec<String>, sigs: &Signatures) -> std::collections::HashMap<String, bool> {
    let mut str_names = str_param_names.clone();
    let mut li: i64 = 0;
    let mut ln = (body.len() as i64);
    while (li < ln) {
        match body[li as usize].clone().clone() {
            Stmt::LetStmt(lname2, ltype2, lvalue2, lconst2) => {
                match char_str_type_of_let_call(&body[li as usize].clone().clone(), &sigs) {
                    Some(t2) => {
                        if (t2.name.to_string() == "Str".to_string().to_string()) {
                            str_names.push(lname2);
                        }
                    }
                    None => {
                    }
                }
            }
            _ => {
            }
        }
        li = (li + 1);
    }
    let mut hits: Vec<String> = walk_stmts_for_char_indexing(&body);
    let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut hi: i64 = 0;
    let mut hn = (hits.len() as i64);
    while (hi < hn) {
        let mut name = hits[hi as usize].clone().clone();
        match counts.get(name.clone().as_str()).cloned() {
            Some(c) => {
                counts.insert(name, (c + 1));
            }
            None => {
                counts.insert(name, 1);
            }
        }
        hi = (hi + 1);
    }
    let mut out: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut j: i64 = 0;
    let mut n = (str_names.len() as i64);
    while (j < n) {
        let mut pname = str_names[j as usize].clone();
        match counts.get(pname.clone().as_str()).cloned() {
            Some(c2) => {
                if (c2 >= 2) {
                    out.insert(pname, true);
                }
            }
            None => {
            }
        }
        j = (j + 1);
    }
    return out;
}

// Zwraca nazwe identyfikatora, na ktorym wywolano `.char_at`/`.slice`
// (np. `"source"` dla `source.char_at(i)`), albo `""` jesli `e` nie
// jest takim wywolaniem - pomocnicze dla `walk_expr_for_char_indexing`.
// Osobna funkcja (przyjmujaca zwykly, auto-zreferencjonowany `Expr`)
// zamiast dopasowania WPROST na `callee`/`target` w miejscu uzycia -
// `callee`/`target` sa `Box<Expr>` w wygenerowanym Ruscie (Expr jest
// posrednio samo-rekurencyjny), a dopasowanie wzorca WPROST na
// `Box<T>` nie kompiluje sie bez `.as_ref()`/funkcji posredniej - ten
// sam problem rozwiazany identycznie w transpiler.hcs (`direct_call_index`).
pub fn expr_as_char_slice_target_name(e: &Expr) -> String {
    let mut c = e.clone();
    match c {
        Expr::Attr(target, name) => {
            if ((name.to_string() == "char_at".to_string().to_string()) || (name.to_string() == "slice".to_string().to_string())) {
                return expr_as_ident_name(&target).to_string();
            }
        }
        _ => {
        }
    }
    return "".to_string();
}

pub fn walk_expr_for_char_indexing(e: &Expr) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut c = e.clone();
    match c {
        Expr::Call(callee, args) => {
            let mut slice_target: String = expr_as_char_slice_target_name(&callee);
            if (slice_target.to_string() != "".to_string().to_string()) {
                out.push((slice_target).to_string());
            }
            out.extend(walk_expr_for_char_indexing(&callee));
            let mut i: i64 = 0;
            let mut n = (args.len() as i64);
            while (i < n) {
                out.extend(walk_expr_for_char_indexing(&args[i as usize].clone()));
                i = (i + 1);
            }
        }
        Expr::BinOp(op, left, right) => {
            out.extend(walk_expr_for_char_indexing(&left));
            out.extend(walk_expr_for_char_indexing(&right));
        }
        Expr::UnaryOp(op, operand) => {
            out.extend(walk_expr_for_char_indexing(&operand));
        }
        Expr::Attr(target, name) => {
            out.extend(walk_expr_for_char_indexing(&target));
        }
        Expr::Index(target, index) => {
            out.extend(walk_expr_for_char_indexing(&target));
            out.extend(walk_expr_for_char_indexing(&index));
        }
        Expr::ListLit(items) => {
            let mut i: i64 = 0;
            let mut n = (items.len() as i64);
            while (i < n) {
                out.extend(walk_expr_for_char_indexing(&items[i as usize].clone()));
                i = (i + 1);
            }
        }
        Expr::Cast(target, type_ref) => {
            out.extend(walk_expr_for_char_indexing(&target));
        }
        Expr::TryOp(target) => {
            out.extend(walk_expr_for_char_indexing(&target));
        }
        _ => {
        }
    }
    return out;
}

pub fn walk_stmts_for_char_indexing(body: &Vec<Stmt>) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut i: i64 = 0;
    let mut n = (body.len() as i64);
    while (i < n) {
        out.extend(walk_stmt_for_char_indexing(&body[i as usize].clone().clone()));
        i = (i + 1);
    }
    return out;
}

pub fn walk_stmt_for_char_indexing(node: &Stmt) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut s = node.clone();
    match s {
        Stmt::AssignStmt(target, op, value) => {
            out.extend(walk_expr_for_char_indexing(&target));
            out.extend(walk_expr_for_char_indexing(&value));
        }
        Stmt::ExprStmt(e) => {
            out.extend(walk_expr_for_char_indexing(&e));
        }
        Stmt::LetStmt(name, type_ref, value, is_const) => {
            match value {
                Some(v) => {
                    out.extend(walk_expr_for_char_indexing(&v));
                }
                None => {
                }
            }
        }
        Stmt::ReturnStmt(value) => {
            match value {
                Some(v) => {
                    out.extend(walk_expr_for_char_indexing(&v));
                }
                None => {
                }
            }
        }
        Stmt::IfStmt(cond, body, elifs, else_body) => {
            out.extend(walk_expr_for_char_indexing(&cond));
            out.extend(walk_stmts_for_char_indexing(&body));
            let mut i: i64 = 0;
            let mut n = (elifs.len() as i64);
            while (i < n) {
                let mut arm = elifs[i as usize].clone();
                out.extend(walk_expr_for_char_indexing(&arm.cond));
                out.extend(walk_stmts_for_char_indexing(&arm.body));
                i = (i + 1);
            }
            match else_body {
                Some(eb) => {
                    out.extend(walk_stmts_for_char_indexing(&eb));
                }
                None => {
                }
            }
        }
        Stmt::WhileStmt(cond, body) => {
            out.extend(walk_expr_for_char_indexing(&cond));
            out.extend(walk_stmts_for_char_indexing(&body));
        }
        Stmt::ForStmt(var, iterable, body) => {
            out.extend(walk_stmts_for_char_indexing(&body));
        }
        Stmt::ManualBlock(body) => {
            out.extend(walk_stmts_for_char_indexing(&body));
        }
        Stmt::MatchStmt(subject, arms) => {
            out.extend(walk_expr_for_char_indexing(&subject));
            let mut i: i64 = 0;
            let mut n = (arms.len() as i64);
            while (i < n) {
                let mut arm = arms[i as usize].clone();
                out.extend(walk_stmts_for_char_indexing(&arm.body));
                i = (i + 1);
            }
        }
        _ => {
        }
    }
    return out;
}

// Buduje CALY graf (WSZYSTKIE structy + WSZYSTKIE enumy) i odpala DFS
// z kazdego jeszcze-nieodwiedzonego wezla - parytet z glowna petla
// `_build_recursion_info()`.
pub fn build_recursion_info(sigs: &Signatures) -> RecursionInfo {
    let mut known_structs = sigs.struct_names.clone();
    let mut known_enums = sigs.enum_names.clone();
    let mut adjacency: std::collections::HashMap<String, Vec<Edge>> = std::collections::HashMap::new();
    let mut i: i64 = 0;
    let mut sn = (known_structs.len() as i64);
    while (i < sn) {
        let mut name = known_structs[i as usize].clone();
        match sigs.structs.get(name.clone().as_str()).cloned() {
            Some(decl) => {
                let mut fields: Vec<Param> = struct_decl_fields_local(&decl);
                let mut edges: Vec<Edge> = struct_field_edges(&name.clone(), &fields, &known_structs.clone(), &known_enums.clone());
                if ((edges.len() as i64) > 0) {
                    adjacency.insert(name, (edges).clone());
                }
            }
            None => {
            }
        }
        i = (i + 1);
    }
    let mut j: i64 = 0;
    let mut en = (known_enums.len() as i64);
    while (j < en) {
        let mut ename = known_enums[j as usize].clone();
        match sigs.enums.get(ename.clone().as_str()).cloned() {
            Some(decl) => {
                let mut variants: Vec<EnumVariant> = enum_decl_variants(&decl);
                let mut edges: Vec<Edge> = enum_field_edges(&ename.clone(), &variants, &known_structs.clone(), &known_enums.clone());
                if ((edges.len() as i64) > 0) {
                    adjacency.insert(ename, (edges).clone());
                }
            }
            None => {
            }
        }
        j = (j + 1);
    }
    let mut analyzer: RecursionAnalyzer = RecursionAnalyzer::new((adjacency).clone(), std::collections::HashMap::new(), std::collections::HashMap::new());
    let mut k: i64 = 0;
    let mut all_nodes: Vec<String> = vec![];
    let mut kk: i64 = 0;
    while (kk < (known_structs.len() as i64)) {
        all_nodes.push(known_structs[kk as usize].clone());
        kk = (kk + 1);
    }
    let mut ll: i64 = 0;
    while (ll < (known_enums.len() as i64)) {
        all_nodes.push(known_enums[ll as usize].clone());
        ll = (ll + 1);
    }
    while (k < (all_nodes.len() as i64)) {
        let mut node = all_nodes[k as usize].clone().clone();
        if (analyzer.color_of(&node.clone()).to_string() == "white".to_string().to_string()) {
            analyzer.dfs(&node);
        }
        k = (k + 1);
    }
    let mut boxed_fields: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut m: i64 = 0;
    let mut sn2 = (known_structs.len() as i64);
    while (m < sn2) {
        let mut sname = known_structs[m as usize].clone();
        match sigs.structs.get(sname.clone().as_str()).cloned() {
            Some(decl) => {
                let mut fields2: Vec<Param> = struct_decl_fields_local(&decl);
                let mut p: i64 = 0;
                let mut fn2 = (fields2.len() as i64);
                while (p < fn2) {
                    let mut f2 = fields2[p as usize].clone();
                    match f2.type_ref {
                        Some(ft2) => {
                            let mut edge_key = format!("{}{}", format!("{}{}", sname.clone(), "::".to_string()), f2.name.clone());
                            if analyzer.boxed_edges.contains_key(edge_key.clone().as_str()) {
                                boxed_fields.insert(edge_key, sizing_edge_kind(&ft2));
                            }
                        }
                        None => {
                        }
                    }
                    p = (p + 1);
                }
            }
            None => {
            }
        }
        m = (m + 1);
    }
    let mut q: i64 = 0;
    let mut en2 = (known_enums.len() as i64);
    while (q < en2) {
        let mut ename2 = known_enums[q as usize].clone();
        match sigs.enums.get(ename2.clone().as_str()).cloned() {
            Some(decl2) => {
                let mut variants2: Vec<EnumVariant> = enum_decl_variants(&decl2);
                let mut r: i64 = 0;
                let mut vn2 = (variants2.len() as i64);
                while (r < vn2) {
                    let mut v2 = variants2[r as usize].clone();
                    let mut s2: i64 = 0;
                    let mut vfn2 = (v2.fields.len() as i64);
                    while (s2 < vfn2) {
                        let mut ft3 = v2.fields[s2 as usize].clone();
                        let mut field_key2 = format!("{}{}", format!("{}{}", v2.name.clone(), "#".to_string()), (s2).to_string());
                        let mut edge_key2 = format!("{}{}", format!("{}{}", ename2.clone(), "::".to_string()), field_key2);
                        if analyzer.boxed_edges.contains_key(edge_key2.clone().as_str()) {
                            boxed_fields.insert(edge_key2, sizing_edge_kind(&ft3));
                        }
                        s2 = (s2 + 1);
                    }
                    r = (r + 1);
                }
            }
            None => {
            }
        }
        q = (q + 1);
    }
    return RecursionInfo::new((boxed_fields).clone());
}

// -- Auto-`&mut self`/auto-`&mut param` (parytet z `_mutated_names_in_body`/
// `_compute_mut_params`/`_find_self_method_calls`/
// `_compute_method_mut_params`) -------------------------------------
// Parytet z `_MUTATING_METHODS`.
pub fn is_mutating_method(name: &String) -> bool {
    return ((((((((name.to_string() == "push".to_string().to_string()) || (name.to_string() == "pop".to_string().to_string())) || (name.to_string() == "remove".to_string().to_string())) || (name.to_string() == "insert".to_string().to_string())) || (name.to_string() == "clear".to_string().to_string())) || (name.to_string() == "sort".to_string().to_string())) || (name.to_string() == "extend".to_string().to_string())) || (name.to_string() == "truncate".to_string().to_string()));
}

// Zbiera nazwy zmiennych/parametrow, ktorych POLA sa gdzies w ciele
// przypisywane LUB mutowane metoda typu `.push`/`.pop` - parytet z
// lokalnym `mutated: set[str]` w `_mutated_names_in_body()`. `order`
// (List<Str>, bez duplikatow) istnieje TYLKO dlatego, ze ten bootstrap
// nie ma iteracji po Dict - wywolujacy (`compute_mut_params`) musi
// jakos "przejrzec" znalezione nazwy.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MutTracker {
    pub mutated: std::collections::HashMap<String, bool>,
    pub order: Vec<String>,
}

impl MutTracker {
    pub fn new(mutated: std::collections::HashMap<String, bool>, order: Vec<String>) -> Self {
        MutTracker { mutated, order }
    }
}

impl MutTracker {
    pub fn mark_mutated(&mut self, name: &String) {
        if !(self.mutated.contains_key(name.clone().as_str())) {
            self.order.push(name.clone());
        }
        self.mutated.insert((name).to_string(), true);
    }

    /// Parytet z `mark_base()` - obiera warstwy `Index` (`xs[i].pole =
    /// ...` mutuje `xs`, nie samo `i`), potem oznacza `Attr(Ident)`/
    /// `Ident` jako zmutowane. Rekurencja zamiast Pythonowej petli
    /// `while isinstance(base, A.Index): base = base.target` - ten sam
    /// efekt (dowolna glebokosc `[i][j][k]...`), ale HackerScript nie
    /// ma przypisywalnej zmiennej-akumulatora dla WZORCA (tylko dla
    /// wartosci).
    pub fn mark_base(&mut self, base: &Expr) {
        let mut b = base.clone();
        match b {
            Expr::Index(target, index) => {
                self.mark_base(&target);
            }
            Expr::Attr(target, name) => {
                let mut target_name: String = expr_as_ident_name(&target);
                if (target_name.to_string() != "".to_string().to_string()) {
                    self.mark_mutated(&target_name);
                }
            }
            Expr::IdentExpr(name) => {
                self.mark_mutated(&name);
            }
            _ => {
            }
        }
    }

    /// Sprawdza, czy `callee` (cel wywolania) to `Attr` o mutujacej
    /// nazwie metody (`.push`/itd.) - jesli tak, oznacza JEJ CEL jako
    /// zmutowany. Osobna metoda (nie zagniezdzony `match` w
    /// `walk_expr`) z tego samego powodu co `expr_as_ident_name` w
    /// typeinfer.hcs - `callee` jest polem Boxowanym (`Call(Expr,
    /// ...)`), wiec bezposrednie dopasowanie wymagaloby dereferencji,
    /// ktorej HackerScript nie ma - trzeba przejsc przez GRANICE
    /// WYWOLANIA (bezpieczna koercja `Box<Expr> -> &Expr`).
    pub fn handle_mutating_call(&mut self, callee: &Expr) {
        let mut c = callee.clone();
        match c {
            Expr::Attr(target, name) => {
                if is_mutating_method(&name) {
                    self.mark_base(&target);
                }
            }
            _ => {
            }
        }
    }

    /// Parytet z zagniezdzonym `walk_expr()` w `_mutated_names_in_body`
    /// - BEZ `Cast`/`TryOp` (w przeciwienstwie do `SelfCallTracker`
    /// nizej - tak samo jak w oryginale, gdzie te dwie funkcje maja
    /// LEKKO rozne pokrycie drzewa, nie jest to bledem/niedopatrzeniem
    /// z mojej strony, tylko wiernym odtworzeniem).
    pub fn walk_expr(&mut self, e: &Expr) {
        let mut e2 = e.clone();
        match e2 {
            Expr::Call(callee, args) => {
                self.handle_mutating_call(&callee);
                self.walk_expr(&callee);
                let mut i: i64 = 0;
                let mut n = (args.len() as i64);
                while (i < n) {
                    self.walk_expr(&args[i as usize]);
                    i = (i + 1);
                }
            }
            Expr::BinOp(op, left, right) => {
                self.walk_expr(&left);
                self.walk_expr(&right);
            }
            Expr::UnaryOp(op, operand) => {
                self.walk_expr(&operand);
            }
            Expr::Attr(target, name) => {
                self.walk_expr(&target);
            }
            Expr::Index(target, index) => {
                self.walk_expr(&target);
                self.walk_expr(&index);
            }
            Expr::ListLit(items) => {
                let mut j: i64 = 0;
                let mut m = (items.len() as i64);
                while (j < m) {
                    self.walk_expr(&items[j as usize]);
                    j = (j + 1);
                }
            }
            _ => {
            }
        }
    }

    pub fn walk_stmts(&mut self, stmts: &Vec<Stmt>) {
        let mut i: i64 = 0;
        let mut n = (stmts.len() as i64);
        while (i < n) {
            self.walk_stmt(&stmts[i as usize].clone());
            i = (i + 1);
        }
    }

    /// Parytet z zagniezdzonym `walk(node)` w `_mutated_names_in_body`.
    pub fn walk_stmt(&mut self, s: &Stmt) {
        let mut s2 = s.clone();
        match s2 {
            Stmt::AssignStmt(target, op, value) => {
                self.mark_base(&target);
                self.walk_expr(&value);
            }
            Stmt::ExprStmt(expr) => {
                self.walk_expr(&expr);
            }
            Stmt::LetStmt(name, type_ref, value, is_const) => {
                match value {
                    Some(v) => {
                        self.walk_expr(&v);
                    }
                    None => {
                    }
                }
            }
            Stmt::ReturnStmt(value) => {
                match value {
                    Some(v) => {
                        self.walk_expr(&v);
                    }
                    None => {
                    }
                }
            }
            Stmt::IfStmt(cond, body, elifs, else_body) => {
                self.walk_expr(&cond);
                self.walk_stmts(&body);
                let mut i: i64 = 0;
                let mut n = (elifs.len() as i64);
                while (i < n) {
                    let mut arm = elifs[i as usize].clone();
                    self.walk_expr(&arm.cond);
                    self.walk_stmts(&arm.body);
                    i = (i + 1);
                }
                match else_body {
                    Some(eb) => {
                        self.walk_stmts(&eb);
                    }
                    None => {
                    }
                }
            }
            Stmt::WhileStmt(cond, body) => {
                self.walk_expr(&cond);
                self.walk_stmts(&body);
            }
            Stmt::ForStmt(var, iterable, body) => {
                self.walk_stmts(&body);
            }
            Stmt::ManualBlock(body) => {
                self.walk_stmts(&body);
            }
            Stmt::MatchStmt(subject, arms) => {
                self.walk_expr(&subject);
                let mut j: i64 = 0;
                let mut m = (arms.len() as i64);
                while (j < m) {
                    let mut arm2 = arms[j as usize].clone();
                    self.walk_stmts(&arm2.body);
                    j = (j + 1);
                }
            }
            _ => {
            }
        }
    }

}

// Parytet z wolnej funkcji `_mutated_names_in_body()`.
pub fn mutated_names_in_body(body: &Vec<Stmt>) -> MutTracker {
    let mut tracker: MutTracker = MutTracker::new(std::collections::HashMap::new(), vec![]);
    tracker.walk_stmts(&body);
    return tracker;
}

// Jak `MutTracker`, ale zbiera nazwy metod wywolanych jako
// `self.metoda(...)` (parytet z `_find_self_method_calls()`) - UZYWA
// `Cast`/`TryOp` w `walk_expr` (w przeciwienstwie do `MutTracker` -
// patrz komentarz tam), bo Pythonowy oryginal tak ma.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelfCallTracker {
    pub calls: std::collections::HashMap<String, bool>,
    pub order: Vec<String>,
}

impl SelfCallTracker {
    pub fn new(calls: std::collections::HashMap<String, bool>, order: Vec<String>) -> Self {
        SelfCallTracker { calls, order }
    }
}

impl SelfCallTracker {
    pub fn mark_call(&mut self, name: &String) {
        if !(self.calls.contains_key(name.clone().as_str())) {
            self.order.push(name.clone());
        }
        self.calls.insert((name).to_string(), true);
    }

    /// Jak `MutTracker::handle_mutating_call`, ale sprawdza czy CEL
    /// wywolania to `self` (nie czy NAZWA metody jest mutujaca).
    pub fn handle_call(&mut self, callee: &Expr) {
        let mut c = callee.clone();
        match c {
            Expr::Attr(target, name) => {
                let mut target_name: String = expr_as_ident_name(&target);
                if (target_name.to_string() == "self".to_string().to_string()) {
                    self.mark_call(&name);
                }
            }
            _ => {
            }
        }
    }

    pub fn walk_expr(&mut self, e: &Expr) {
        let mut e2 = e.clone();
        match e2 {
            Expr::Call(callee, args) => {
                self.handle_call(&callee);
                self.walk_expr(&callee);
                let mut i: i64 = 0;
                let mut n = (args.len() as i64);
                while (i < n) {
                    self.walk_expr(&args[i as usize]);
                    i = (i + 1);
                }
            }
            Expr::BinOp(op, left, right) => {
                self.walk_expr(&left);
                self.walk_expr(&right);
            }
            Expr::UnaryOp(op, operand) => {
                self.walk_expr(&operand);
            }
            Expr::Attr(target, name) => {
                self.walk_expr(&target);
            }
            Expr::Index(target, index) => {
                self.walk_expr(&target);
                self.walk_expr(&index);
            }
            Expr::ListLit(items) => {
                let mut j: i64 = 0;
                let mut m = (items.len() as i64);
                while (j < m) {
                    self.walk_expr(&items[j as usize]);
                    j = (j + 1);
                }
            }
            Expr::Cast(target, type_ref) => {
                self.walk_expr(&target);
            }
            Expr::TryOp(target) => {
                self.walk_expr(&target);
            }
            _ => {
            }
        }
    }

    pub fn walk_stmts(&mut self, stmts: &Vec<Stmt>) {
        let mut i: i64 = 0;
        let mut n = (stmts.len() as i64);
        while (i < n) {
            self.walk_stmt(&stmts[i as usize].clone());
            i = (i + 1);
        }
    }

    /// Rozni sie od `MutTracker::walk_stmt` w JEDNYM miejscu:
    /// `AssignStmt` woa `walk_expr(target)` (zwykly przeglad), NIE
    /// `mark_base(target)` - tu nie interesuje nas KTORA zmienna jest
    /// mutowana, tylko KTORE `self.metoda()` sa wywolane, wiec cel
    /// przypisania jest po prostu kolejnym wyrazeniem do przejrzenia.
    pub fn walk_stmt(&mut self, s: &Stmt) {
        let mut s2 = s.clone();
        match s2 {
            Stmt::AssignStmt(target, op, value) => {
                self.walk_expr(&target);
                self.walk_expr(&value);
            }
            Stmt::ExprStmt(expr) => {
                self.walk_expr(&expr);
            }
            Stmt::LetStmt(name, type_ref, value, is_const) => {
                match value {
                    Some(v) => {
                        self.walk_expr(&v);
                    }
                    None => {
                    }
                }
            }
            Stmt::ReturnStmt(value) => {
                match value {
                    Some(v) => {
                        self.walk_expr(&v);
                    }
                    None => {
                    }
                }
            }
            Stmt::IfStmt(cond, body, elifs, else_body) => {
                self.walk_expr(&cond);
                self.walk_stmts(&body);
                let mut i: i64 = 0;
                let mut n = (elifs.len() as i64);
                while (i < n) {
                    let mut arm = elifs[i as usize].clone();
                    self.walk_expr(&arm.cond);
                    self.walk_stmts(&arm.body);
                    i = (i + 1);
                }
                match else_body {
                    Some(eb) => {
                        self.walk_stmts(&eb);
                    }
                    None => {
                    }
                }
            }
            Stmt::WhileStmt(cond, body) => {
                self.walk_expr(&cond);
                self.walk_stmts(&body);
            }
            Stmt::ForStmt(var, iterable, body) => {
                self.walk_stmts(&body);
            }
            Stmt::ManualBlock(body) => {
                self.walk_stmts(&body);
            }
            Stmt::MatchStmt(subject, arms) => {
                self.walk_expr(&subject);
                let mut j: i64 = 0;
                let mut m = (arms.len() as i64);
                while (j < m) {
                    let mut arm2 = arms[j as usize].clone();
                    self.walk_stmts(&arm2.body);
                    j = (j + 1);
                }
            }
            _ => {
            }
        }
    }

}

pub fn find_self_method_calls(body: &Vec<Stmt>) -> SelfCallTracker {
    let mut tracker: SelfCallTracker = SelfCallTracker::new(std::collections::HashMap::new(), vec![]);
    tracker.walk_stmts(&body);
    return tracker;
}

// Parytet z `_compute_mut_params()` - dla KAZDEJ wolnej funkcji w
// programie, ktore z jej parametrow sa mutowane. Splaszczone do
// `Dict<Str, Bool>` z kluczem `"NazwaFunkcji::NazwaParametru"`
// (Pythonowe `dict[str, set[str]]` - dwupoziomowa struktura - ten
// bootstrap nie ma wygodnych zagniezdzonych Dict, patrz podobne
// decyzje w `RecursionInfo`/`Signatures.methods` wczesniej).
pub fn compute_mut_params(prog: &Program) -> std::collections::HashMap<String, bool> {
    let mut result: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut i: i64 = 0;
    let mut n = (prog.body.len() as i64);
    while (i < n) {
        let mut stmt: Stmt = prog.body[i as usize].clone();
        match stmt {
            Stmt::FunDecl(name, params, ret_type, body, is_pub, type_params) => {
                let mut tracker: MutTracker = mutated_names_in_body(&body);
                let mut j: i64 = 0;
                let mut on = (tracker.order.len() as i64);
                while (j < on) {
                    let mut vname: String = tracker.order[j as usize].clone();
                    result.insert(format!("{}{}", format!("{}{}", name.clone(), "::".to_string()), vname), true);
                    j = (j + 1);
                }
            }
            _ => {
            }
        }
        i = (i + 1);
    }
    return result;
}

// Parytet z `_compute_method_mut_params()` - jak wyzej, ale dla metod
// w `impl` (klucz `"Struct::metoda::zmienna"`) - Z PUNKTEM STALYM dla
// posredniej mutacji (`self.metoda()` gdzie `metoda` sama potrzebuje
// `&mut self` -> wywolujacy TEZ potrzebuje). `extra_method_mut_params`
// to znane Z GORY wyniki z INNYCH plikow (cross-file `impl`, patrz
// "Ograniczenia") - scalane na starcie, tak jak w Pythonie.
pub fn compute_method_mut_params(prog: &Program, extra_method_mut_params: &std::collections::HashMap<String, bool>) -> std::collections::HashMap<String, bool> {
    let mut result: std::collections::HashMap<String, bool> = extra_method_mut_params.clone();
    let mut method_bodies: std::collections::HashMap<String, Vec<Stmt>> = std::collections::HashMap::new();
    let mut method_struct_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut method_keys: Vec<String> = vec![];
    let mut i: i64 = 0;
    let mut n = (prog.body.len() as i64);
    while (i < n) {
        let mut stmt: Stmt = prog.body[i as usize].clone();
        match stmt {
            Stmt::ImplDecl(struct_name, methods, type_params) => {
                let mut j: i64 = 0;
                let mut mn = (methods.len() as i64);
                while (j < mn) {
                    let mut m = methods[j as usize].clone();
                    match m {
                        Stmt::FunDecl(mname, mparams, mret, mbody, mis_pub, mtype_params) => {
                            let mut key = format!("{}{}", format!("{}{}", struct_name.clone(), "::".to_string()), mname.clone());
                            let mut tracker: MutTracker = mutated_names_in_body(&mbody.clone());
                            let mut k: i64 = 0;
                            let mut on = (tracker.order.len() as i64);
                            while (k < on) {
                                let mut vname: String = tracker.order[k as usize].clone();
                                result.insert(format!("{}{}", format!("{}{}", key.clone(), "::".to_string()), vname), true);
                                k = (k + 1);
                            }
                            method_bodies.insert(key.clone(), mbody);
                            method_struct_names.insert(key.clone(), struct_name.clone());
                            method_keys.push(key);
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
    let mut changed: bool = true;
    while changed {
        changed = false;
        let mut p: i64 = 0;
        let mut kn = (method_keys.len() as i64);
        while (p < kn) {
            let mut key2: String = method_keys[p as usize].clone();
            if !(result.contains_key(format!("{}{}", key2.clone(), "::self".to_string()).as_str())) {
                match method_struct_names.get(key2.clone().as_str()).cloned() {
                    Some(struct_name2) => {
                        match method_bodies.get(key2.clone().as_str()).cloned() {
                            Some(body2) => {
                                let mut self_calls: SelfCallTracker = find_self_method_calls(&body2);
                                let mut c: i64 = 0;
                                let mut cn = (self_calls.order.len() as i64);
                                let mut found: bool = false;
                                while ((c < cn) && !(found)) {
                                    let mut called_name: String = self_calls.order[c as usize].clone();
                                    let mut other_key = format!("{}{}", format!("{}{}", struct_name2.clone(), "::".to_string()), called_name);
                                    if result.contains_key(format!("{}{}", other_key, "::self".to_string()).as_str()) {
                                        result.insert(format!("{}{}", key2.clone(), "::self".to_string()), true);
                                        changed = true;
                                        found = true;
                                    }
                                    c = (c + 1);
                                }
                            }
                            None => {
                            }
                        }
                    }
                    None => {
                    }
                }
            }
            p = (p + 1);
        }
    }
    return result;
}

// -- Emisja wyrazen: `gen_expr` (parytet z `CodeGen.gen_expr()`) ------
// Ucieka string na literal Rust (`"..."`) - parytet z
// `_rust_string_literal()`.
pub fn rust_string_literal(s: &String) -> String {
    let mut out: String = "\"".to_string();
    let mut i: i64 = 0;
    let mut n = (s.len() as i64);
    while (i < n) {
        let mut c: String = (s.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default());
        if (c.to_string() == "\\".to_string().to_string()) {
            out = format!("{}{}", out, "\\\\".to_string());
        } else if (c.to_string() == "\"".to_string().to_string()) {
            out = format!("{}{}", out, "\\\"".to_string());
        } else if (c.to_string() == "\n".to_string().to_string()) {
            out = format!("{}{}", out, "\\n".to_string());
        } else if (c.to_string() == "\t".to_string().to_string()) {
            out = format!("{}{}", out, "\\t".to_string());
        } else if (c.to_string() == "\r".to_string().to_string()) {
            out = format!("{}{}", out, "\\r".to_string());
        } else {
            out = format!("{}{}", out, c);
        }
        i = (i + 1);
    }
    out = format!("{}{}", out, "\"".to_string());
    return out.to_string();
}

// Zlicza pola KAZDEGO wariantu w CALYM programie (klucz: NAZWA
// WARIANTU, nie "Enum::Wariant" - parytet z `self.variant_arity` w
// Pythonie, ktore TEZ jest plaskie po samej nazwie wariantu, bo
// nazwy wariantow sa unikalne w calym programie w tym bootstrapie -
// patrz E0010 w decl_parser.hcs/parser.hcs z wczesniejszych sesji).
pub fn build_variant_arity(sigs: &Signatures) -> std::collections::HashMap<String, i64> {
    let mut out: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut i: i64 = 0;
    let mut n = (sigs.enum_names.len() as i64);
    while (i < n) {
        let mut ename = sigs.enum_names[i as usize].clone().clone();
        match sigs.enums.get(ename.as_str()).cloned() {
            Some(decl) => {
                let mut variants: Vec<EnumVariant> = enum_decl_variants(&decl);
                let mut j: i64 = 0;
                let mut vn = (variants.len() as i64);
                while (j < vn) {
                    let mut v = variants[j as usize].clone();
                    out.insert((v.name).to_string(), (v.fields.len() as i64));
                    j = (j + 1);
                }
            }
            None => {
            }
        }
        i = (i + 1);
    }
    return out;
}

// Ksztalt `wyrazenie.metoda(...)` - jak `AttrCallShape` w
// typeinfer.hcs, ale dla CODEGEN: dodatkowo trzyma JUZ
// WYRENDEROWANY tekst celu (`target_rendered`), zeby nie renderowac
// go dwa razy. Tak jak tam - CELOWO bez surowego pola `Expr` (patrz
// uwaga projektowa w typeinfer.hcs, ten sam powod).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MethodCallShape {
    pub is_method_call: bool,
    pub method_name: String,
    pub target_rendered: String,
    pub target_type: Option<TypeRef>,
}

impl MethodCallShape {
    pub fn new(is_method_call: bool, method_name: String, target_rendered: String, target_type: Option<TypeRef>) -> Self {
        MethodCallShape { is_method_call, method_name, target_rendered, target_type }
    }
}

// `seen_real_toplevel_item`: czy juz wyemitowano jakikolwiek prawdziwy
// element najwyzszego poziomu (struct/enum/impl/extern fun/fun/const/
// use) - potrzebne, zeby wiedziec kiedy kolejny `!!` moze byc jeszcze
// `//!` (inner doc, dozwolony TYLKO przed wszystkimi innymi elementami
// pliku), a kiedy musi spasc do zwyklego `//` (E0753 w Ruscie w
// przeciwnym razie) - patrz `emit_module_doc_comment`. Bug znaleziony
// przy pierwszej realnej kompilacji `cargo build` wygenerowanego kodu
// w tej sesji (poprzednio niemozliwe w tym srodowisku bez rustc).
// `char_cache_params`: nazwy parametrow Str biezacej funkcji
// zmaterializowane jako `Vec<char>` w prologu (patrz `gen_fun`/
// `char_indexed_str_params`) - Dict jako surogat Set (`.contains`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CodeGen {
    pub sigs: Signatures,
    pub env_vars: std::collections::HashMap<String, Option<TypeRef>>,
    pub variant_arity: std::collections::HashMap<String, i64>,
    pub boxed_fields: std::collections::HashMap<String, String>,
    pub method_mut_params: std::collections::HashMap<String, bool>,
    pub mut_params: std::collections::HashMap<String, bool>,
    pub current_type_params: Vec<String>,
    pub current_ret_type: Option<TypeRef>,
    pub output: Vec<String>,
    pub indent: i64,
    pub no_default_structs: std::collections::HashMap<String, bool>,
    pub needs_pyo3: bool,
    pub direct_blocks: std::collections::HashMap<String, String>,
    pub seen_real_toplevel_item: bool,
    pub char_cache_params: std::collections::HashMap<String, bool>,
}

impl CodeGen {
    pub fn new(sigs: Signatures, env_vars: std::collections::HashMap<String, Option<TypeRef>>, variant_arity: std::collections::HashMap<String, i64>, boxed_fields: std::collections::HashMap<String, String>, method_mut_params: std::collections::HashMap<String, bool>, mut_params: std::collections::HashMap<String, bool>, current_type_params: Vec<String>, current_ret_type: Option<TypeRef>, output: Vec<String>, indent: i64, no_default_structs: std::collections::HashMap<String, bool>, needs_pyo3: bool, direct_blocks: std::collections::HashMap<String, String>, seen_real_toplevel_item: bool, char_cache_params: std::collections::HashMap<String, bool>) -> Self {
        CodeGen { sigs, env_vars, variant_arity, boxed_fields, method_mut_params, mut_params, current_type_params, current_ret_type, output, indent, no_default_structs, needs_pyo3, direct_blocks, seen_real_toplevel_item, char_cache_params }
    }
}

impl CodeGen {
    /// Parytet z `self.env: TypeEnv | None` w Pythonie - **UPROSZCZONE**:
    /// zamiast trzymac `Option<TypeEnv>` jako pole (co, jak `FnChecker`
    /// w typecheck.hcs juz odkryl, uniemozliwialoby BEZPIECZNA mutacje -
    /// `self.env.declare(...)` NIE jest wykrywane przez mechanizm
    /// auto-`&mut self`, patrz uwaga projektowa w typecheck.hcs), `env`
    /// trzyma TYLKO `env_vars: Dict<Str, Option<TypeRef>>` BEZPOSREDNIO
    /// jako pole `CodeGen` - `self.env_vars.insert(...)` (w
    /// `declare_env` nizej) JEST wykrywalne (pojedynczy poziom
    /// `self.pole.WBUDOWANA_METODA()`). "Brak aktywnego srodowiska"
    /// (Pythonowe `None`) jest reprezentowane jako PUSTY Dict zamiast
    /// `Option` - w PRAKTYCE `gen_stmt`/`gen_expr` sa wywolywane TYLKO
    /// wewnatrz cial funkcji (gdzie `env` w oryginale ZAWSZE jest
    /// ustawiony przez `gen_fun`/`gen_impl` przed wejsciem), wiec ta
    /// roznica nie ma znaczenia w praktyce - udokumentowane uproszczenie.
    pub fn infer_type(&self, e: &Expr) -> Option<TypeRef> {
        let mut env = TypeEnv::new(self.sigs.clone(), self.env_vars.clone());
        return infer_expr_type(&e, &env);
    }

    /// Deklaruje zmienna w biezacym srodowisku typow - parytet z
    /// `self.env.declare(...)` wywolanym Z ZEWNATRZ `TypeEnv` (nie
    /// Z jego wlasnej metody) - stad bezposrednio na `self.env_vars`.
    pub fn declare_env(&mut self, name: &String, t: Option<TypeRef>) {
        self.env_vars.insert((name).to_string(), t);
    }

    /// Parytet z `_is_refable()` - patrz uwaga w rust_type_name/
    /// docs/ROADMAP.md co do tego, ze `Dict` NIE jest tu wliczony
    /// (prawdopodobne przeoczenie w oryginale, odtworzone WIERNIE -
    /// nie "naprawiane" tutaj, bo cala reszta codegen.py polega na
    /// DOKLADNIE tym zachowaniu).
    pub fn is_refable_type(&self, t: &TypeRef) -> bool {
        if (((t.name.to_string() == "Str".to_string().to_string()) || (t.name.to_string() == "List".to_string().to_string())) || (t.name.to_string() == "Dict".to_string().to_string())) {
            return true;
        }
        if self.sigs.structs.contains_key(t.name.clone().as_str()) {
            return true;
        }
        if self.sigs.enums.contains_key(t.name.clone().as_str()) {
            return true;
        }
        return false;
    }

    pub fn expr_is_string_lit(&self, e: &Expr) -> bool {
        let mut c = e.clone();
        match c {
            Expr::StringLit(value, is_doc) => {
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    pub fn expr_is_list_lit(&self, e: &Expr) -> bool {
        let mut c = e.clone();
        match c {
            Expr::ListLit(items) => {
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    /// `+` na Str MUSI stac sie `format!(...)` (nie Rustowe `+` na
    /// `String`, ktore konsumuje LHS) - decyzja "czy to Str" bierze
    /// pod uwage ZARWNO wywnioskowany typ JAK I to, czy operand jest
    /// DOSLOWNIE literalem stringowym (dla przypadkow gdy inferencja
    /// nie wie jeszcze nic, np. `"a" + zmienna_bez_adnotacji`) -
    /// parytet z odpowiednia czescia `gen_expr()` dla `BinOp("+", ...)`.
    /// Rekurencyjnie po lancuchach `a + b + c` (lewostronnie laczne w
    /// gramatyce) - `infer_type` na WEWNETRZNYM `BinOp("+")` czesto nie
    /// zwraca Str nawet gdy oba operandy sa Str, co psulo zewnetrzny
    /// `+` w lancuchu (np. `a + "::" + b` -> `format!(...) + b`, gdzie
    /// `String + &String` sie nie kompiluje - brak impl Add). Parytet z
    /// `_expr_is_strish()` w codegen.py (bug znaleziony przy pierwszej
    /// realnej kompilacji `cargo build` w tej sesji).
    pub fn expr_is_strish(&self, e: &Expr) -> bool {
        if self.expr_is_string_lit(&e.clone()) {
            return true;
        }
        let mut c = e.clone();
        match c {
            Expr::BinOp(op, left, right) => {
                if (op.to_string() == "+".to_string().to_string()) {
                    if (self.expr_is_strish(&left) || self.expr_is_strish(&right)) {
                        return true;
                    }
                }
            }
            _ => {
            }
        }
        let mut t: Option<TypeRef> = self.infer_type(&e);
        match t {
            Some(tv) => {
                if (tv.name.to_string() == "Str".to_string().to_string()) {
                    return true;
                }
            }
            None => {
            }
        }
        return false;
    }

    pub fn type_or_lit_is_str(&self, t: Option<TypeRef>, e: &Expr) -> bool {
        match t {
            Some(tv) => {
                if (tv.name.to_string() == "Str".to_string().to_string()) {
                    return true;
                }
            }
            None => {
            }
        }
        return self.expr_is_string_lit(&e);
    }

    pub fn type_or_lit_is_list(&self, t: Option<TypeRef>, e: &Expr) -> bool {
        match t {
            Some(tv2) => {
                if (tv2.name.to_string() == "List".to_string().to_string()) {
                    return true;
                }
            }
            None => {
            }
        }
        return self.expr_is_list_lit(&e);
    }

    /// Parytet z `_gen_log()`.
    pub fn gen_log(&self, args: &Vec<Expr>) -> String {
        let mut fmt: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (args.len() as i64);
        while (i < n) {
            if (i > 0) {
                fmt = format!("{}{}", fmt, " ".to_string());
            }
            fmt = format!("{}{}", fmt, "{}".to_string());
            i = (i + 1);
        }
        let mut rendered: String = self.gen_args_list(&args);
        let mut sep: String = "".to_string();
        if ((args.len() as i64) > 0) {
            sep = ", ".to_string();
        }
        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "println!(\"".to_string(), fmt), "\"".to_string()), sep), rendered), ")".to_string()).to_string();
    }

    /// Argument, ktory MUSI byc WLASNOSCIA (nie referencja) - np. pole
    /// konstruktora struct/`Some(...)`/wariant enuma. Identyfikator
    /// odnoszacy sie do auto-zreferencjonowanego parametru (`&String`/
    /// `&SomeStruct`) przekazany wprost w takie miejsce nie kompiluje sie
    /// (E0308). `.to_string()`/`.clone()` dziala jednakowo na referencje
    /// i wlasnosc - bezpieczna heurystyka "zawsze konwertuj", parytet z
    /// `_gen_owned_arg()` w codegen.py (bug znaleziony przy pierwszej
    /// realnej kompilacji `cargo build` w tej sesji).
    /// OGRANICZONE do `IdentExpr`/`Attr` - jedyne ksztalty, ktore MOGA
    /// byc referencja (auto-`&` parametru/pola). `Call`/lista/literaly
    /// ZAWSZE produkuja swieza wlasnosc - klonowanie ich jest
    /// bezuzyteczne (i psulo dokladne dopasowania w testach). `Index`
    /// celowo pominiety - `gen_expr` dla `Index` juz sam dokleja
    /// `.clone()` gdy trzeba, wiec ponowne klonowanie tu byloby
    /// podwojne. Parytet z ograniczeniem w `_gen_owned_arg` (codegen.py).
    pub fn expr_is_ident_or_attr(&self, e: &Expr) -> bool {
        let mut c = e.clone();
        match c {
            Expr::IdentExpr(name) => {
                return true;
            }
            Expr::Attr(target, name) => {
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    /// Czy `e` to `cos.generic` albo `cos.generic2` - pola ZAWSZE
    /// `Option<Box<TypeRef>>` w wygenerowanym Ruscie (TypeRef jest
    /// bezposrednio rekurencyjny) - patrz `gen_owned_arg` ponizej.
    pub fn expr_is_generic_field_attr(&self, e: &Expr) -> bool {
        let mut c = e.clone();
        match c {
            Expr::Attr(target, name) => {
                return ((name.to_string() == "generic".to_string().to_string()) || (name.to_string() == "generic2".to_string().to_string()));
            }
            _ => {
                return false;
            }
        }
    }

    pub fn gen_owned_arg(&self, a: &Expr) -> String {
        let mut rendered: String = self.gen_expr(&a.clone());
        if !(self.expr_is_ident_or_attr(&a.clone())) {
            return rendered.to_string();
        }
        if self.expr_is_generic_field_attr(&a.clone()) {
            /// `.map(|b| *b)` odpakowuje `Box` przy zachowaniu `Option` -
            /// parytet z identyczna poprawka w `_gen_owned_arg` (codegen.py),
            /// ktora sama jest uogolnieniem `_gen_return_expr`'owej
            /// poprawki (dla `return`) na KAZDA pozycje wymagajaca
            /// wlasnosci (argument/`let`). Bug znaleziony przy uzyciu
            /// skompilowanego stage1 do zbudowania cli.hcs w tej sesji.
            return format!("{}{}", format!("{}{}", "(".to_string(), rendered), ").map(|b| *b)".to_string()).to_string();
        }
        let mut t: Option<TypeRef> = self.infer_type(&a);
        match t {
            Some(tt) => {
                if (tt.name.to_string() == "Str".to_string().to_string()) {
                    return format!("{}{}", format!("{}{}", "(".to_string(), rendered), ").to_string()".to_string()).to_string();
                }
                if ((((tt.name.to_string() == "Dict".to_string().to_string()) || (tt.name.to_string() == "List".to_string().to_string())) || self.sigs.structs.contains_key(tt.name.clone().as_str())) || self.sigs.enums.contains_key(tt.name.clone().as_str())) {
                    return format!("{}{}", format!("{}{}", "(".to_string(), rendered), ").clone()".to_string()).to_string();
                }
            }
            None => {
            }
        }
        return rendered.to_string();
    }

    pub fn gen_args_list(&self, args: &Vec<Expr>) -> String {
        let mut out: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (args.len() as i64);
        while (i < n) {
            if (i > 0) {
                out = format!("{}{}", out, ", ".to_string());
            }
            out = format!("{}{}", out, self.gen_owned_arg(&args[i as usize].clone()));
            i = (i + 1);
        }
        return out.to_string();
    }

    /// Konstruktor wariantu enuma: `Wariant(a, b)` -> `Enum::Wariant(a,
    /// b)`, doklejajac `Box::new(...)`/`.map(Box::new)` na kazdym
    /// argumencie, ktory `boxed_fields` (z `build_recursion_info`)
    /// oznaczyl jako "direct"/"option" - parytet z odpowiednim
    /// fragmentem `gen_expr()` dla `A.Call` na konstruktorze wariantu.
    pub fn gen_variant_call(&self, enum_name: &String, variant_name: &String, args: &Vec<Expr>) -> String {
        let mut out: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (args.len() as i64);
        while (i < n) {
            if (i > 0) {
                out = format!("{}{}", out, ", ".to_string());
            }
            let mut rendered: String = self.gen_owned_arg(&args[i as usize].clone());
            let mut key = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", enum_name.clone(), "::".to_string()), variant_name.clone()), "#".to_string()), (i).to_string());
            match self.boxed_fields.get(key.as_str()).cloned() {
                Some(kind) => {
                    if (kind.to_string() == "option".to_string().to_string()) {
                        rendered = format!("{}{}", rendered, ".map(Box::new)".to_string());
                    } else if (kind.to_string() == "direct".to_string().to_string()) {
                        rendered = format!("{}{}", format!("{}{}", "Box::new(".to_string(), rendered), ")".to_string());
                    }
                }
                None => {
                }
            }
            out = format!("{}{}", out, rendered);
            i = (i + 1);
        }
        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", enum_name, "::".to_string()), variant_name), "(".to_string()), out), ")".to_string()).to_string();
    }

    /// `idx`-ty parametr POMIJAJAC `self` (jesli jest pierwszy) -
    /// potrzebne, bo `args` (argumenty WYWOLANIA) nie zawieraja
    /// odpowiednika `self`, ale `params` (parametry DEKLARACJI) go
    /// MOGA zawierac.
    pub fn nth_non_self_param(&self, params: &Vec<Param>, idx: i64) -> Option<Param> {
        let mut filtered: Vec<Param> = vec![];
        let mut i: i64 = 0;
        let mut n = (params.len() as i64);
        while (i < n) {
            let mut p = params[i as usize].clone();
            if (p.name.to_string() != "self".to_string().to_string()) {
                filtered.push((p).clone());
            }
            i = (i + 1);
        }
        if (idx < (filtered.len() as i64)) {
            return Some(filtered[idx as usize].clone());
        }
        return None;
    }

    /// Wywolanie metody uzytkownika `cel.metoda(args)` - auto-`&`/
    /// `&mut` na KAZDYM argumencie wedlug `is_refable_type` +
    /// `method_mut_params` (parytet z `_call_arg_str()` zastosowanym
    /// do wywolan metod).
    pub fn gen_user_method_call(&self, shape: &MethodCallShape, target_type: &TypeRef, m: &Stmt, args: &Vec<Expr>) -> String {
        let mut params: Vec<Param> = fun_decl_params(&m.clone());
        let mut method_name_local: String = fun_decl_name(&m);
        let mut out: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (args.len() as i64);
        while (i < n) {
            if (i > 0) {
                out = format!("{}{}", out, ", ".to_string());
            }
            let mut rendered: String = self.gen_expr(&args[i as usize].clone().clone());
            match self.nth_non_self_param(&params.clone(), i) {
                Some(p) => {
                    match p.type_ref.clone() {
                        Some(pt) => {
                            if self.is_refable_type(&pt) {
                                let mut mut_key = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", target_type.name.clone(), "::".to_string()), method_name_local.clone()), "::".to_string()), p.name.clone());
                                if self.method_mut_params.contains_key(mut_key.as_str()) {
                                    rendered = format!("{}{}", "&mut ".to_string(), rendered);
                                } else {
                                    rendered = format!("{}{}", "&".to_string(), rendered);
                                }
                            } else {
                                rendered = self.gen_owned_arg(&args[i as usize].clone().clone());
                            }
                        }
                        None => {
                        }
                    }
                }
                None => {
                }
            }
            out = format!("{}{}", out, rendered);
            i = (i + 1);
        }
        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", shape.target_rendered, ".".to_string()), shape.method_name), "(".to_string()), out), ")".to_string()).to_string();
    }

    /// Wywolanie wolnej funkcji `f(args)` - jak wyzej, ale klucz mutacji
    /// to `"NazwaFunkcji::param"` (`self.mut_params`, nie
    /// `method_mut_params`).
    pub fn gen_call_args_for_fun(&self, fn_name: &String, f: &Stmt, args: &Vec<Expr>) -> String {
        let mut params: Vec<Param> = fun_decl_params(&f);
        let mut out: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (args.len() as i64);
        while (i < n) {
            if (i > 0) {
                out = format!("{}{}", out, ", ".to_string());
            }
            let mut rendered: String = self.gen_expr(&args[i as usize].clone().clone());
            if (i < (params.len() as i64)) {
                let mut p = params[i as usize].clone();
                match p.type_ref.clone() {
                    Some(pt) => {
                        if self.is_refable_type(&pt) {
                            let mut mut_key = format!("{}{}", format!("{}{}", fn_name.clone(), "::".to_string()), p.name.clone());
                            if self.mut_params.contains_key(mut_key.as_str()) {
                                rendered = format!("{}{}", "&mut ".to_string(), rendered);
                            } else {
                                rendered = format!("{}{}", "&".to_string(), rendered);
                            }
                        } else {
                            rendered = self.gen_owned_arg(&args[i as usize].clone().clone());
                        }
                    }
                    None => {
                    }
                }
            }
            out = format!("{}{}", out, rendered);
            i = (i + 1);
        }
        return out.to_string();
    }

    /// Rozpoznaje `cel.metoda` - patrz uwaga projektowa na gorze pliku
    /// (typeinfer.hcs) co do `Box<Expr>`/granicy wywolania - `target`
    /// (Boxowane pole `Attr`) jest tu uzyte DWA RAZY jako ARGUMENT
    /// (`self.gen_expr(target)`/`self.infer_type(target)`) - to
    /// POZYCZENIE (typ `Expr` jest "refable"), nie przeniesienie, wiec
    /// wielokrotne uzycie jest bezpieczne (patrz codegen.hcs, sekcja
    /// auto-`&mut self` - to ta sama zasada co pozwolila `edges[i]`
    /// itd. dzialac PO DODANIU `.clone()`, tu dziala BEZ potrzeby
    /// `.clone()`, bo `target` NIE jest indeksowaniem kolekcji, tylko
    /// zwyklym bindingiem z dopasowania).
    pub fn gen_method_call_shape(&self, e: &Expr) -> MethodCallShape {
        let mut c = e.clone();
        match c {
            Expr::Attr(target, name) => {
                let mut rendered: String = self.gen_expr(&target);
                let mut tt: Option<TypeRef> = self.infer_type(&target);
                return MethodCallShape::new(true, name, (rendered).to_string(), tt);
            }
            _ => {
                return MethodCallShape::new(false, "".to_string(), "".to_string(), None);
            }
        }
    }

    /// Identyfikator - jesli NIE jest zadeklarowana lokalna zmienna o
    /// tej nazwie, a nazwa odpowiada WARIANTOWI ENUMA bez pol
    /// (`arity == 0`), renderuje `Enum::Wariant` zamiast golej nazwy -
    /// parytet z odpowiednim fragmentem `gen_expr()` dla `A.Ident`.
    pub fn gen_ident(&self, name: &String) -> String {
        if !(self.env_vars.contains_key(name.clone().as_str())) {
            match self.variant_arity.get(name.clone().as_str()).cloned() {
                Some(arity) => {
                    if (arity == 0) {
                        match self.sigs.variant_owner.get(name.clone().as_str()).cloned() {
                            Some(enum_name) => {
                                return format!("{}{}", format!("{}{}", enum_name, "::".to_string()), name).to_string();
                            }
                            None => {
                            }
                        }
                    }
                }
                None => {
                }
            }
        }
        return format!("{}{}", name, "".to_string()).to_string();
    }

    pub fn gen_binop(&self, op: &String, left: &Expr, right: &Expr) -> String {
        let mut rust_op: String = (op).to_string();
        if (op.to_string() == "and".to_string().to_string()) {
            rust_op = "&&".to_string();
        }
        if (op.to_string() == "or".to_string().to_string()) {
            rust_op = "||".to_string();
        }
        if (op.to_string() == "+".to_string().to_string()) {
            let mut lt: Option<TypeRef> = self.infer_type(&left);
            let mut rt: Option<TypeRef> = self.infer_type(&right);
            if (self.type_or_lit_is_list(lt.clone(), &left) || self.type_or_lit_is_list(rt.clone(), &right)) {
                let mut l: String = self.gen_expr(&left);
                let mut r: String = self.gen_expr(&right);
                return format!("{}{}", format!("{}{}", format!("{}{}", l, ".iter().cloned().chain(".to_string()), r), ".iter().cloned()).collect::<Vec<_>>()".to_string()).to_string();
            }
            if (((self.type_or_lit_is_str(lt, &left) || self.type_or_lit_is_str(rt, &right)) || self.expr_is_strish(&left)) || self.expr_is_strish(&right)) {
                return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "format!(\"{}{}\", ".to_string(), self.gen_expr(&left)), ", ".to_string()), self.gen_expr(&right)), ")".to_string()).to_string();
            }
        }
        if ((((((op.to_string() == "==".to_string().to_string()) || (op.to_string() == "!=".to_string().to_string())) || (op.to_string() == "<".to_string().to_string())) || (op.to_string() == ">".to_string().to_string())) || (op.to_string() == "<=".to_string().to_string())) || (op.to_string() == ">=".to_string().to_string())) {
            let mut lt2: Option<TypeRef> = self.infer_type(&left.clone());
            let mut rt2: Option<TypeRef> = self.infer_type(&right.clone());
            if (self.type_or_lit_is_str(lt2, &left.clone()) || self.type_or_lit_is_str(rt2, &right.clone())) {
                return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "(".to_string(), self.gen_expr(&left)), ".to_string() ".to_string()), rust_op), " ".to_string()), self.gen_expr(&right)), ".to_string())".to_string()).to_string();
            }
        }
        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "(".to_string(), self.gen_expr(&left)), " ".to_string()), rust_op), " ".to_string()), self.gen_expr(&right)), ")".to_string()).to_string();
    }

    /// `xs[i]` - `.clone()` doklejane TYLKO gdy element jest "refable"
    /// (struct/enum/Str/List) - parytet z heurystyka "klonuj zawsze dla
    /// non-Copy elementow" opisana w komentarzu przy `A.Index` w
    /// codegen.py (i juz uzyta RECZNIE wielokrotnie w tym bootstrapie,
    /// patrz sekcje wyzej o `.clone()`-utracie-typu).
    pub fn gen_index(&self, target: &Expr, index: &Expr) -> String {
        let mut base: String = format!("{}{}", format!("{}{}", format!("{}{}", self.gen_expr(&target.clone()), "[".to_string()), self.gen_expr(&index)), " as usize]".to_string());
        let mut tt: Option<TypeRef> = self.infer_type(&target);
        match tt {
            Some(t) => {
                if (t.name.to_string() == "List".to_string().to_string()) {
                    match t.generic.clone() {
                        Some(elem) => {
                            if self.is_refable_type(&elem) {
                                return format!("{}{}", base, ".clone()".to_string()).to_string();
                            }
                        }
                        None => {
                        }
                    }
                }
            }
            None => {
            }
        }
        return base.to_string();
    }

    /// `wyrazenie as Typ` - `as Str` jest specjalne (`.to_string()`,
    /// dziala dla KAZDEGO typu z `Display`/numerycznego), inne rzutuja
    /// przez Rustowe `as`.
    pub fn gen_cast(&self, target: &Expr, type_ref: &TypeRef) -> String {
        if (type_ref.name.to_string() == "Str".to_string().to_string()) {
            return format!("{}{}", format!("{}{}", "(".to_string(), self.gen_expr(&target)), ").to_string()".to_string()).to_string();
        }
        let mut rendered_type: String = rust_type_name(&type_ref, &self.sigs, &self.current_type_params);
        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "(".to_string(), self.gen_expr(&target)), " as ".to_string()), rendered_type), ")".to_string()).to_string();
    }

    /// Dysponent wywolan - parytet z DLUGA seria `if`/`elif` w
    /// `gen_expr()` dla `A.Call` (wbudowane `log`/`read_file`/
    /// `write_file`/`some`/`ok`/`err`/`none`/`dict`, konstruktor
    /// wariantu enuma, `.len()`/`.char_at`/`.slice` (Str), `.fetch`/
    /// `.contains`/`.remove` (Dict), metoda uzytkownika, konstruktor
    /// struct, wolna funkcja, i na koncu przypadek ogolny).
    pub fn gen_call(&self, callee: &Expr, args: &Vec<Expr>) -> String {
        let mut ident_name: String = expr_as_ident_name(&callee.clone());
        if (ident_name.to_string() == "log".to_string().to_string()) {
            return self.gen_log(&args).to_string();
        }
        if ((ident_name.to_string() == "read_file".to_string().to_string()) && ((args.len() as i64) == 1)) {
            return format!("{}{}", format!("{}{}", "std::fs::read_to_string(&".to_string(), self.gen_expr(&args[0 as usize].clone())), ").map_err(|e| e.to_string())".to_string()).to_string();
        }
        if ((ident_name.to_string() == "args".to_string().to_string()) && ((args.len() as i64) == 0)) {
            return "std::env::args().skip(1).collect::<Vec<String>>()".to_string();
        }
        if ((ident_name.to_string() == "exit".to_string().to_string()) && ((args.len() as i64) == 1)) {
            return format!("{}{}", format!("{}{}", "std::process::exit((".to_string(), self.gen_expr(&args[0 as usize].clone())), ") as i32)".to_string()).to_string();
        }
        if ((ident_name.to_string() == "write_file".to_string().to_string()) && ((args.len() as i64) == 2)) {
            return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "std::fs::write(&".to_string(), self.gen_expr(&args[0 as usize].clone())), ", ".to_string()), self.gen_expr(&args[1 as usize].clone())), ").map_err(|e| e.to_string())".to_string()).to_string();
        }
        if ((ident_name.to_string() == "dir_exists".to_string().to_string()) && ((args.len() as i64) == 1)) {
            return format!("{}{}", format!("{}{}", "std::path::Path::new(&".to_string(), self.gen_expr(&args[0 as usize].clone())), ").is_dir()".to_string()).to_string();
        }
        if ((ident_name.to_string() == "create_dir".to_string().to_string()) && ((args.len() as i64) == 1)) {
            return format!("{}{}", format!("{}{}", "std::fs::create_dir_all(&".to_string(), self.gen_expr(&args[0 as usize].clone())), ").map_err(|e| e.to_string())".to_string()).to_string();
        }
        if ((ident_name.to_string() == "remove_file".to_string().to_string()) && ((args.len() as i64) == 1)) {
            return format!("{}{}", format!("{}{}", "std::fs::remove_file(&".to_string(), self.gen_expr(&args[0 as usize].clone())), ").map_err(|e| e.to_string())".to_string()).to_string();
        }
        if ((ident_name.to_string() == "list_dir".to_string().to_string()) && ((args.len() as i64) == 1)) {
            return format!("{}{}", format!("{}{}", "(|| -> Result<Vec<String>, String> { let mut out = Vec::new(); for entry in std::fs::read_dir(&".to_string(), self.gen_expr(&args[0 as usize].clone())), ").map_err(|e| e.to_string())? { let entry = entry.map_err(|e| e.to_string())?; out.push(entry.file_name().to_string_lossy().to_string()); } Ok(out) })()".to_string()).to_string();
        }
        if ((ident_name.to_string() == "env_var".to_string().to_string()) && ((args.len() as i64) == 1)) {
            /// env_var(nazwa) -> Option<Str> - get <std:env>. `.ok()`
            /// zamienia Result<String, VarError> na Option<String> -
            /// parytet z hackerc/hackerc/codegen.py (Python).
            return format!("{}{}", format!("{}{}", "std::env::var(&".to_string(), self.gen_expr(&args[0 as usize].clone())), ").ok()".to_string()).to_string();
        }
        if ((ident_name.to_string() == "run_command".to_string().to_string()) && ((args.len() as i64) == 2)) {
            /// run_command(program, argumenty) -> Result<Str, Str> -
            /// get <std:process>. Uruchamia proces potomny BEZ powloki
            /// (fork/exec bezposrednio), czeka na zakonczenie, zwraca
            /// caly stdout (kod 0) albo caly stderr (kod != 0 / nie
            /// da sie uruchomic) - parytet z hackerc/hackerc/codegen.py.
            let mut program_expr: String = self.gen_expr(&args[0 as usize].clone());
            let mut args_expr: String = self.gen_expr(&args[1 as usize].clone());
            return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "(|| -> Result<String, String> {\n        let __hks_cmd_out = std::process::Command::new(&".to_string(), program_expr), ")\n            .args(".to_string()), args_expr), ")\n            .output()\n            .map_err(|e| e.to_string())?;\n        if __hks_cmd_out.status.success() {\n            Ok(String::from_utf8_lossy(&__hks_cmd_out.stdout).to_string())\n        } else {\n            Err(String::from_utf8_lossy(&__hks_cmd_out.stderr).to_string())\n        }\n    })()".to_string()).to_string();
        }
        if ((ident_name.to_string() == "http_get".to_string().to_string()) && ((args.len() as i64) == 1)) {
            /// http_get(url) -> Result<Str, Str> - get <std:http>.
            /// WYMAGA `get <crates:ureq::2>` w pliku uzywajacym (patrz
            /// libs/std/lib/http.hcs) - parytet z codegen.py (Python).
            let mut url_expr: String = self.gen_expr(&args[0 as usize].clone());
            return format!("{}{}", format!("{}{}", "(|| -> Result<String, String> {\n        let __hks_http_resp = ureq::get(&".to_string(), url_expr), ")\n            .call()\n            .map_err(|e| e.to_string())?;\n        __hks_http_resp.into_string().map_err(|e| e.to_string())\n    })()".to_string()).to_string();
        }
        if (ident_name.to_string() == "some".to_string().to_string()) {
            return format!("{}{}", format!("{}{}", "Some(".to_string(), self.gen_args_list(&args)), ")".to_string()).to_string();
        }
        if (ident_name.to_string() == "ok".to_string().to_string()) {
            return format!("{}{}", format!("{}{}", "Ok(".to_string(), self.gen_args_list(&args)), ")".to_string()).to_string();
        }
        if (ident_name.to_string() == "err".to_string().to_string()) {
            return format!("{}{}", format!("{}{}", "Err(".to_string(), self.gen_args_list(&args)), ")".to_string()).to_string();
        }
        if ((ident_name.to_string() == "none".to_string().to_string()) && ((args.len() as i64) == 0)) {
            return "None".to_string();
        }
        if ((ident_name.to_string() == "dict".to_string().to_string()) && ((args.len() as i64) == 0)) {
            return "std::collections::HashMap::new()".to_string();
        }
        if (ident_name.to_string() != "".to_string().to_string()) {
            match self.sigs.variant_owner.get(ident_name.clone().as_str()).cloned() {
                Some(enum_name) => {
                    return self.gen_variant_call(&enum_name, &ident_name, &args).to_string();
                }
                None => {
                }
            }
        }
        let mut shape: MethodCallShape = self.gen_method_call_shape(&callee.clone());
        if ((shape.is_method_call && (shape.method_name.to_string() == "len".to_string().to_string())) && ((args.len() as i64) == 0)) {
            return format!("{}{}", format!("{}{}", "(".to_string(), shape.target_rendered), ".len() as i64)".to_string()).to_string();
        }
        if (shape.is_method_call && ((shape.method_name.to_string() == "char_at".to_string().to_string()) || (shape.method_name.to_string() == "slice".to_string().to_string()))) {
            match shape.target_type.clone() {
                Some(tt) => {
                    /// Cel jest zmaterializowany jako `Vec<char>` w prologu
                    /// biezacej funkcji (patrz `gen_fun`/
                    /// `char_indexed_str_params`) -> O(1) indeksowanie
                    /// zamiast O(i) `.chars().nth(i)`. `target_rendered`
                    /// to juz gotowy tekst Rusta - dla golego identyfikatora
                    /// (typowy przypadek parametru w petli) jest to po prostu
                    /// jego nazwa, ktora mozna sprawdzic w `char_cache_params`.
                    let mut is_cached = self.char_cache_params.contains_key(shape.target_rendered.clone().as_str());
                    if (((tt.name.to_string() == "Str".to_string().to_string()) && (shape.method_name.to_string() == "char_at".to_string().to_string())) && ((args.len() as i64) == 1)) {
                        if is_cached {
                            return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "(__hks_chars_".to_string(), shape.target_rendered.clone()), ".get(".to_string()), self.gen_expr(&args[0 as usize].clone())), " as usize).map(|c| c.to_string()).unwrap_or_default())".to_string()).to_string();
                        }
                        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "(".to_string(), shape.target_rendered), ".chars().nth(".to_string()), self.gen_expr(&args[0 as usize].clone())), " as usize).map(|c| c.to_string()).unwrap_or_default())".to_string()).to_string();
                    }
                    if (((tt.name.to_string() == "Str".to_string().to_string()) && (shape.method_name.to_string() == "slice".to_string().to_string())) && ((args.len() as i64) == 2)) {
                        let mut start_e: String = self.gen_expr(&args[0 as usize].clone());
                        let mut end_e: String = self.gen_expr(&args[1 as usize].clone());
                        if is_cached {
                            let mut cache_var2 = format!("{}{}", "__hks_chars_".to_string(), shape.target_rendered.clone());
                            return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "({ let __v = &".to_string(), cache_var2), "; let __s = ((".to_string()), start_e), ") as usize).min(__v.len()); let __e = ((".to_string()), end_e), ") as usize).min(__v.len()).max(__s); __v[__s..__e].iter().collect::<String>() })".to_string()).to_string();
                        }
                        return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "(".to_string(), shape.target_rendered), ".chars().skip(".to_string()), start_e), " as usize).take(((".to_string()), end_e), ") - (".to_string()), start_e), ")) as usize).collect::<String>())".to_string()).to_string();
                    }
                }
                None => {
                }
            }
        }
        if ((shape.is_method_call && (((shape.method_name.to_string() == "fetch".to_string().to_string()) || (shape.method_name.to_string() == "contains".to_string().to_string())) || (shape.method_name.to_string() == "remove".to_string().to_string()))) && ((args.len() as i64) == 1)) {
            match shape.target_type.clone() {
                Some(tt2) => {
                    if (tt2.name.to_string() == "Dict".to_string().to_string()) {
                        let mut key_e: String = self.gen_expr(&args[0 as usize].clone());
                        if (shape.method_name.to_string() == "fetch".to_string().to_string()) {
                            return format!("{}{}", format!("{}{}", format!("{}{}", shape.target_rendered, ".get(".to_string()), key_e), ".as_str()).cloned()".to_string()).to_string();
                        }
                        if (shape.method_name.to_string() == "contains".to_string().to_string()) {
                            return format!("{}{}", format!("{}{}", format!("{}{}", shape.target_rendered, ".contains_key(".to_string()), key_e), ".as_str())".to_string()).to_string();
                        }
                        return format!("{}{}", format!("{}{}", format!("{}{}", shape.target_rendered, ".remove(".to_string()), key_e), ".as_str())".to_string()).to_string();
                    }
                }
                None => {
                }
            }
        }
        if shape.is_method_call {
            match shape.target_type.clone() {
                Some(tt3) => {
                    match self.sigs.methods.get(format!("{}{}", format!("{}{}", tt3.name.clone(), "::".to_string()), shape.method_name.clone()).as_str()).cloned() {
                        Some(m) => {
                            return self.gen_user_method_call(&shape, &tt3, &m, &args).to_string();
                        }
                        None => {
                        }
                    }
                }
                None => {
                }
            }
        }
        if ((ident_name.to_string() != "".to_string().to_string()) && self.sigs.structs.contains_key(ident_name.clone().as_str())) {
            return format!("{}{}", format!("{}{}", format!("{}{}", ident_name, "::new(".to_string()), self.gen_args_list(&args)), ")".to_string()).to_string();
        }
        if (ident_name.to_string() != "".to_string().to_string()) {
            match self.sigs.functions.get(ident_name.clone().as_str()).cloned() {
                Some(f) => {
                    return format!("{}{}", format!("{}{}", format!("{}{}", ident_name, "(".to_string()), self.gen_call_args_for_fun(&ident_name, &f, &args)), ")".to_string()).to_string();
                }
                None => {
                }
            }
        }
        if shape.is_method_call {
            return format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", shape.target_rendered, ".".to_string()), shape.method_name), "(".to_string()), self.gen_args_list(&args)), ")".to_string()).to_string();
        }
        return format!("{}{}", format!("{}{}", format!("{}{}", self.gen_expr(&callee), "(".to_string()), self.gen_args_list(&args)), ")".to_string()).to_string();
    }

    /// Glowny dysponent - parytet z `gen_expr()`. Klonuje `node` na
    /// WLASNA, owned lokalna zmienna NA STARCIE (ta sama zasada co w
    /// `infer_expr_type`, `typeinfer.hcs` - patrz uwaga projektowa
    /// tam) - dzieki temu KAZDE Boxowane pole wyciagniete tu przez
    /// `match` jest bezpiecznie przekazywane DALEJ tylko jako ARGUMENT
    /// (nigdy dopasowywane bezposrednio poza ta funkcja).
    pub fn gen_expr(&self, node: &Expr) -> String {
        let mut e = node.clone();
        match e {
            Expr::NumberLit(value) => {
                return value.to_string();
            }
            Expr::StringLit(value, is_doc) => {
                return format!("{}{}", rust_string_literal(&value), ".to_string()".to_string()).to_string();
            }
            Expr::BoolLit(value) => {
                if value {
                    return "true".to_string();
                }
                return "false".to_string();
            }
            Expr::NullLit => {
                return "None".to_string();
            }
            Expr::IdentExpr(name) => {
                return self.gen_ident(&name).to_string();
            }
            Expr::ListLit(items) => {
                return format!("{}{}", format!("{}{}", "vec![".to_string(), self.gen_args_list(&items)), "]".to_string()).to_string();
            }
            Expr::UnaryOp(op, operand) => {
                if (op.to_string() == "not".to_string().to_string()) {
                    return format!("{}{}", format!("{}{}", "!(".to_string(), self.gen_expr(&operand)), ")".to_string()).to_string();
                }
                return format!("{}{}", format!("{}{}", format!("{}{}", op, "(".to_string()), self.gen_expr(&operand)), ")".to_string()).to_string();
            }
            Expr::BinOp(op, left, right) => {
                return self.gen_binop(&op, &left, &right).to_string();
            }
            Expr::Attr(target, name) => {
                return format!("{}{}", format!("{}{}", self.gen_expr(&target), ".".to_string()), name).to_string();
            }
            Expr::Index(target, index) => {
                return self.gen_index(&target, &index).to_string();
            }
            Expr::Cast(target, type_ref) => {
                return self.gen_cast(&target, &type_ref).to_string();
            }
            Expr::TryOp(target) => {
                return format!("{}{}", format!("{}{}", "(".to_string(), self.gen_expr(&target)), ")?".to_string()).to_string();
            }
            Expr::Call(callee, args) => {
                return self.gen_call(&callee, &args).to_string();
            }
        }
    }

}

// -- Emisja instrukcji: `gen_stmt` (parytet z `CodeGen.gen_stmt()`) ---
// Parytet z `_contains_any()` - czy `t` LUB ktorykolwiek z jego
// `.generic`-ow (dowolnej glebokosci) to `Any` (typ elementu pustej
// listy `[]`, patrz `infer_expr_type`'s `ListLit` w typeinfer.hcs) -
// taki typ NIE moze byc uzyty jako jawna adnotacja Rusta.
pub fn type_contains_any(t: &TypeRef) -> bool {
    if (t.name.to_string() == "Any".to_string().to_string()) {
        return true;
    }
    match t.generic.clone() {
        Some(inner) => {
            return type_contains_any(&inner);
        }
        None => {
            return false;
        }
    }
}

pub fn is_builtin_variant(name: &String) -> bool {
    return ((((name.to_string() == "Some".to_string().to_string()) || (name.to_string() == "None".to_string().to_string())) || (name.to_string() == "Ok".to_string().to_string())) || (name.to_string() == "Err".to_string().to_string()));
}

// Ksztalt `wyrazenie(...)` uzywany TYLKO do rozpoznania specjalnych
// wywolan na poziomie INSTRUKCJI (`log(...)`, `__direct__(...)`) -
// przechowuje `args: List<Expr>` BEZPOSREDNIO (bezpieczne - `List<Expr>`
// jest polem WPROST na `Call`, NIE Boxowanym, w przeciwienstwie do
// `callee: Expr`, patrz ast_nodes.hcs "Wyrazenia" - `Call(Expr,
// List<Expr>)` box'uje TYLKO `callee`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CallShape {
    pub is_call: bool,
    pub ident_name: String,
    pub args: Vec<Expr>,
}

impl CallShape {
    pub fn new(is_call: bool, ident_name: String, args: Vec<Expr>) -> Self {
        CallShape { is_call, ident_name, args }
    }
}

impl CodeGen {
    pub fn emit(&mut self, text: &String) {
        if (text.to_string() == "".to_string().to_string()) {
            self.output.push("".to_string());
        } else {
            self.output.push(format!("{}{}", self.indent_str(), text));
        }
    }

    pub fn indent_str(&self) -> String {
        let mut out: String = "".to_string();
        let mut i: i64 = 0;
        while (i < self.indent) {
            out = format!("{}{}", out, "    ".to_string());
            i = (i + 1);
        }
        return out.to_string();
    }

    pub fn call_shape(&self, e: &Expr) -> CallShape {
        let mut c = e.clone();
        match c {
            Expr::Call(callee, args) => {
                return CallShape::new(true, expr_as_ident_name(&callee), args);
            }
            _ => {
                return CallShape::new(false, "".to_string(), vec![]);
            }
        }
    }

    pub fn expr_is_doc_string(&self, e: &Expr) -> bool {
        let mut c = e.clone();
        match c {
            Expr::StringLit(value, is_doc) => {
                return is_doc;
            }
            _ => {
                return false;
            }
        }
    }

    pub fn emit_doc_comment(&mut self, e: &Expr) {
        let mut c = e.clone();
        match c {
            Expr::StringLit(value, is_doc) => {
                self.emit(&format!("{}{}", "/// ".to_string(), value));
            }
            _ => {
            }
        }
    }

    pub fn expr_is_attr(&self, e: &Expr) -> bool {
        let mut c = e.clone();
        match c {
            Expr::Attr(target, name) => {
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    /// Parytet z `_gen_return_expr()` - `self.pole` zwracane z funkcji o
    /// "refable" typie zwracanym dostaje `.clone()` (bo `self`/parametry
    /// refable sa ZAWSZE referencjami w tym codegen - `return self.pole;`
    /// probowaloby przeniesc wlasnosc spod referencji, Rust to odrzuca -
    /// ten SAM mechanizm co juz zastosowany RECZNIE wielokrotnie w tej
    /// sesji, np. w typecheck.hcs).
    pub fn gen_return_expr(&self, value: &Expr) -> String {
        let mut rendered: String = self.gen_expr(&value.clone());
        if self.expr_is_attr(&value.clone()) {
            match self.current_ret_type.clone() {
                Some(rt) => {
                    if self.is_refable_type(&rt) {
                        return format!("{}{}", rendered, ".clone()".to_string()).to_string();
                    }
                }
                None => {
                }
            }
        }
        /// Zwracanie GOLEGO identyfikatora typu Str z funkcji `-> Str` -
        /// moze byc `&String` (auto-referencja parametru) - patrz
        /// identyczny komentarz w codegen.py.
        if (!(self.expr_is_attr(&value.clone())) && !(self.expr_is_string_lit(&value.clone()))) {
            match self.current_ret_type.clone() {
                Some(rtstr) => {
                    if (rtstr.name.to_string() == "Str".to_string().to_string()) {
                        return format!("{}{}", rendered, ".to_string()".to_string()).to_string();
                    }
                }
                None => {
                }
            }
        }
        /// `TypeRef.generic`/`.generic2` sa ZAWSZE `Option<Box<TypeRef>>` w
        /// wygenerowanym Ruscie (TypeRef jest bezposrednio rekurencyjny -
        /// patrz `TypeRef::new` w ast_nodes.hcs) - zwrocenie ich wprost z
        /// funkcji zadeklarowanej jako `-> Option<TypeRef>` nie kompiluje
        /// sie (E0308). `.map(|b| *b)` odpakowuje `Box`. Bug znaleziony
        /// przy pierwszej realnej kompilacji `cargo build` w tej sesji.
        let mut vc = value.clone();
        match vc {
            Expr::Attr(target, name) => {
                if ((name.to_string() == "generic".to_string().to_string()) || (name.to_string() == "generic2".to_string().to_string())) {
                    match self.current_ret_type.clone() {
                        Some(rt2) => {
                            if (rt2.name.to_string() == "Option".to_string().to_string()) {
                                return format!("{}{}", rendered, ".map(|b| *b)".to_string()).to_string();
                            }
                        }
                        None => {
                        }
                    }
                }
            }
            _ => {
            }
        }
        return rendered.to_string();
    }

    pub fn gen_return_stmt(&mut self, value: Option<Expr>) {
        match value {
            Some(v) => {
                self.emit(&format!("{}{}", format!("{}{}", "return ".to_string(), self.gen_return_expr(&v)), ";".to_string()));
            }
            None => {
                self.emit(&"return;".to_string());
            }
        }
    }

    /// Parytet z `LetStmt` w `gen_stmt()`. Deklaruje w `env_vars` PRZED
    /// wyliczeniem podpowiedzi typu (`hint`), tak jak Python deklaruje
    /// PRZED renderowaniem - kolejnosc ma znaczenie dla poprawnosci
    /// `self.gen_expr(node.value)`, jesli WARTOSC odwoluje sie do
    /// WLASNEJ nazwy (rekurencja `let x = ... x ...` - rzadkie, ale
    /// Python tez tak robi, wiernie odtworzone).
    /// 
    /// UPROSZCZENIE: `skip_hint` w oryginale ma TRZY warunki (struct
    /// spoza `local_structs`, `_contains_any`, generyczny struct/enum
    /// bez podanego argumentu) - ta wersja implementuje TYLKO drugi
    /// (`type_contains_any`) - pozostale dwa wymagalyby sledzenia
    /// `local_structs`/pelnych `type_params` struktur (infrastruktura z
    /// `project.hcs`, ktory jeszcze nie istnieje) - patrz "Ograniczenia".
    pub fn gen_let_stmt(&mut self, name: &String, type_ref: Option<TypeRef>, value: Option<Expr>, is_const: bool) {
        let mut inferred_type = type_ref.clone();
        match type_ref.clone() {
            Some(_) => {
            }
            None => {
                match value.clone() {
                    Some(v) => {
                        inferred_type = self.infer_type(&v);
                    }
                    None => {
                    }
                }
            }
        }
        self.declare_env(&name.clone(), inferred_type.clone());
        let mut hint: String = "".to_string();
        match inferred_type.clone() {
            Some(t) => {
                if !(type_contains_any(&t.clone())) {
                    hint = format!("{}{}", ": ".to_string(), rust_type_name(&t, &self.sigs, &self.current_type_params));
                }
            }
            None => {
            }
        }
        let mut value_rendered: String = "Default::default()".to_string();
        match value {
            Some(v2) => {
                value_rendered = self.gen_owned_arg(&v2);
            }
            None => {
            }
        }
        let mut kw: String = "let mut".to_string();
        if is_const {
            kw = "let".to_string();
        }
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", kw, " ".to_string()), name), hint), " = ".to_string()), value_rendered), ";".to_string()));
        match inferred_type.clone() {
            Some(itype) => {
                if ((itype.name.to_string() == "Str".to_string().to_string()) && self.char_cache_params.contains_key(name.clone().as_str())) {
                    self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "let __hks_chars_".to_string(), name.clone()), ": Vec<char> = ".to_string()), name.clone()), ".chars().collect();".to_string()));
                }
            }
            None => {
            }
        }
    }

    pub fn gen_if_stmt(&mut self, cond: &Expr, body: &Vec<Stmt>, elifs: &Vec<ElifArm>, else_body: Option<Vec<Stmt>>) {
        self.emit(&format!("{}{}", format!("{}{}", "if ".to_string(), self.gen_expr(&cond)), " {".to_string()));
        self.indent = (self.indent + 1);
        self.gen_stmts(&body);
        self.indent = (self.indent - 1);
        let mut i: i64 = 0;
        let mut n = (elifs.len() as i64);
        while (i < n) {
            let mut arm = elifs[i as usize].clone().clone();
            self.emit(&format!("{}{}", format!("{}{}", "} else if ".to_string(), self.gen_expr(&arm.cond)), " {".to_string()));
            self.indent = (self.indent + 1);
            self.gen_stmts(&arm.body);
            self.indent = (self.indent - 1);
            i = (i + 1);
        }
        match else_body {
            Some(eb) => {
                self.emit(&"} else {".to_string());
                self.indent = (self.indent + 1);
                self.gen_stmts(&eb);
                self.indent = (self.indent - 1);
            }
            None => {
            }
        }
        self.emit(&"}".to_string());
    }

    pub fn join_binds(&self, binds: &Vec<String>) -> String {
        let mut out: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (binds.len() as i64);
        while (i < n) {
            if (i > 0) {
                out = format!("{}{}", out, ", ".to_string());
            }
            out = format!("{}{}", out, binds[i as usize].clone().clone());
            i = (i + 1);
        }
        return out.to_string();
    }

    /// Parytet z `_pattern_str()`. `_` (wildcard) renderuje sie wprost;
    /// `Some`/`None`/`Ok`/`Err` sa natywne (bez kwalifikacji); INNE
    /// warianty sa kwalifikowane `Enum::Wariant` przez `sigs.variant_owner`.
    pub fn pattern_str(&self, variant: &String, binds: &Vec<String>) -> String {
        if (variant.to_string() == "_".to_string().to_string()) {
            return "_".to_string();
        }
        let mut args: String = "".to_string();
        if ((binds.len() as i64) > 0) {
            args = format!("{}{}", format!("{}{}", "(".to_string(), self.join_binds(&binds)), ")".to_string());
        }
        if is_builtin_variant(&variant.clone()) {
            return format!("{}{}", variant, args).to_string();
        }
        match self.sigs.variant_owner.get(variant.clone().as_str()).cloned() {
            Some(enum_name) => {
                return format!("{}{}", format!("{}{}", format!("{}{}", enum_name, "::".to_string()), variant), args).to_string();
            }
            None => {
                println!("{}", format!("{}{}", format!("{}{}", "[hackerc-self] codegen: nieznany wariant '".to_string(), variant), "' w 'match'".to_string()));
                return format!("{}{}", variant, args).to_string();
            }
        }
    }

    /// Parytet z `gen_match()` - ZAPISUJE/PRZYWRACA stan `env_vars` dla
    /// kazdej nazwy bindowanej w galezi (zeby bind nie "wyciekl" poza
    /// `match` i zeby TYMCZASOWO nie zaslanial ISTNIEJACEJ zmiennej o
    /// tej samej nazwie dla PODA nastepnych galezi/kodu po `match`).
    pub fn gen_match(&mut self, subject: &Expr, arms: &Vec<MatchArm>) {
        self.emit(&format!("{}{}", format!("{}{}", "match ".to_string(), self.gen_expr(&subject)), " {".to_string()));
        self.indent = (self.indent + 1);
        let mut i: i64 = 0;
        let mut n = (arms.len() as i64);
        while (i < n) {
            let mut arm = arms[i as usize].clone().clone();
            let mut pattern: String = self.pattern_str(&arm.variant, &arm.binds.clone());
            self.emit(&format!("{}{}", pattern, " => {".to_string()));
            self.indent = (self.indent + 1);
            let mut prev_present: Vec<bool> = vec![];
            let mut prev_values: Vec<Option<TypeRef>> = vec![];
            let mut j: i64 = 0;
            let mut bn = (arm.binds.len() as i64);
            while (j < bn) {
                let mut b = arm.binds[j as usize].clone();
                let mut was_present = self.env_vars.contains_key(b.clone().as_str());
                prev_present.push(was_present);
                match self.env_vars.get(b.clone().as_str()).cloned() {
                    Some(pv) => {
                        prev_values.push(pv);
                    }
                    None => {
                        prev_values.push(None);
                    }
                }
                self.declare_env(&b, None);
                j = (j + 1);
            }
            self.gen_stmts(&arm.body);
            let mut k: i64 = 0;
            while (k < bn) {
                let mut b2 = arm.binds[k as usize].clone();
                if prev_present[k as usize] {
                    self.declare_env(&b2, prev_values[k as usize].clone());
                } else {
                    self.env_vars.remove(b2.as_str());
                }
                k = (k + 1);
            }
            self.indent = (self.indent - 1);
            self.emit(&"}".to_string());
            i = (i + 1);
        }
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
    }

    /// Parytet z `gen_expr_stmt()` - trzy specjalne przypadki na
    /// poziomie INSTRUKCJI (komentarz dokumentacyjny `!!`, `__direct__`,
    /// `log`), reszta to zwykle wyrazenie + `;`.
    /// Wyciaga wartosc `NumberLit` (Str) z wyrazenia - potrzebne, zeby
    /// wyciagnac indeks `N` z `__direct__(N)` (parytet z
    /// `int(e.args[0].value)` w gen_expr_stmt).
    pub fn expr_number_lit_value(&self, e: &Expr) -> String {
        let mut c = e.clone();
        match c {
            Expr::NumberLit(value) => {
                return format!("{}{}", value, "".to_string()).to_string();
            }
            _ => {
                return "".to_string();
            }
        }
    }

    /// Parytet z `gen_direct()` - `__direct__(0)` = surowy kod PYTHONA,
    /// wykonywany przez wbudowany interpreter (PyO3 `Python::with_gil`)
    /// - Rust jest hostem. `direct_blocks: Dict<Str,Str>` (klucz = indeks
    /// jako Str) jest DANE WEJSCIOWE - populowane PRZEZ WYCIAGANIE
    /// surowego tekstu Pythona ZE ZRODLA `.hcs` PRZED tokenizacja
    /// (parytet z `_extract_direct_blocks` w transpiler.py) - TA
    /// EKSTRAKCJA nie jest przepisana w tej sesji (patrz "Ograniczenia"
    /// - `parser.hcs::parse_direct` juz to dokumentuje jako "siec
    /// bezpieczenstwa"), wiec `direct_blocks` bedzie PUSTY dopoki
    /// `transpiler.hcs` (przyszly krok) nie zacznie go wypelniac -
    /// `gen_direct` samo w sobie jest jednak KOMPLETNE i gotowe na TEN
    /// moment.
    pub fn gen_direct(&mut self, idx_text: &String) {
        self.needs_pyo3 = true;
        let mut raw: String = "".to_string();
        match self.direct_blocks.get(idx_text.as_str()).cloned() {
            Some(r) => {
                raw = r;
            }
            None => {
            }
        }
        self.emit(&"{".to_string());
        self.indent = (self.indent + 1);
        self.emit(&"Python::with_gil(|py| -> PyResult<()> {".to_string());
        self.indent = (self.indent + 1);
        self.emit(&format!("{}{}", format!("{}{}", "py.run(".to_string(), python_raw_string(&raw)), ", None, None)?;".to_string()));
        self.emit(&"Ok(())".to_string());
        self.indent = (self.indent - 1);
        self.emit(&"}).expect(\"__direct__(1) (Python) block failed\");".to_string());
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
    }

    pub fn gen_expr_stmt(&mut self, e: &Expr) {
        if self.expr_is_doc_string(&e.clone()) {
            self.emit_doc_comment(&e);
            return;
        }
        let mut shape: CallShape = self.call_shape(&e.clone());
        if ((shape.is_call && (shape.ident_name.to_string() == "__direct__".to_string().to_string())) && ((shape.args.len() as i64) == 1)) {
            let mut idx_text: String = self.expr_number_lit_value(&shape.args[0 as usize].clone().clone());
            self.gen_direct(&idx_text);
            return;
        }
        if (shape.is_call && (shape.ident_name.to_string() == "log".to_string().to_string())) {
            self.emit(&format!("{}{}", self.gen_log(&shape.args), ";".to_string()));
            return;
        }
        self.emit(&format!("{}{}", self.gen_expr(&e), ";".to_string()));
    }

    /// Glowny dysponent instrukcji - parytet z `gen_stmt()`. `struct`
    /// zagniezdzony w ciele funkcji NIE JEST obslugiwany (parytet z
    /// `CodegenError` w oryginale - "'struct' musi byc zadeklarowany na
    /// najwyzszym poziomie") - tu po prostu `log`-uje i pomija, patrz
    /// filozofia bledow w reszcie tego pliku/parser.hcs.
    pub fn gen_stmt(&mut self, node: &Stmt) {
        let mut s = node.clone();
        match s {
            Stmt::LetStmt(name, type_ref, value, is_const) => {
                self.gen_let_stmt(&name, type_ref, value, is_const);
            }
            Stmt::AssignStmt(target, op, value) => {
                if (op.to_string() == "=".to_string().to_string()) {
                    self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", self.gen_expr(&target), " ".to_string()), op), " ".to_string()), self.gen_owned_arg(&value)), ";".to_string()));
                } else {
                    self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", self.gen_expr(&target), " ".to_string()), op), " ".to_string()), self.gen_expr(&value)), ";".to_string()));
                }
            }
            Stmt::IfStmt(cond, body, elifs, else_body) => {
                self.gen_if_stmt(&cond, &body, &elifs, else_body);
            }
            Stmt::WhileStmt(cond, body) => {
                self.emit(&format!("{}{}", format!("{}{}", "while ".to_string(), self.gen_expr(&cond)), " {".to_string()));
                self.indent = (self.indent + 1);
                self.gen_stmts(&body);
                self.indent = (self.indent - 1);
                self.emit(&"}".to_string());
            }
            Stmt::ForStmt(var, iterable, body) => {
                self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "for ".to_string(), var), " in ".to_string()), self.gen_expr(&iterable)), " {".to_string()));
                self.indent = (self.indent + 1);
                self.gen_stmts(&body);
                self.indent = (self.indent - 1);
                self.emit(&"}".to_string());
            }
            Stmt::ReturnStmt(value) => {
                self.gen_return_stmt(value);
            }
            Stmt::BreakStmt => {
                self.emit(&"break;".to_string());
            }
            Stmt::ContinueStmt => {
                self.emit(&"continue;".to_string());
            }
            Stmt::ManualBlock(body) => {
                self.emit(&"unsafe {".to_string());
                self.indent = (self.indent + 1);
                self.gen_stmts(&body);
                self.indent = (self.indent - 1);
                self.emit(&"}".to_string());
            }
            Stmt::GcPragma(mode) => {
                self.emit(&format!("{}{}", format!("{}{}", "// gc:use::".to_string(), mode), " - bez znaczenia w Rust (wlasnosc/pozyczanie)".to_string()));
            }
            Stmt::MatchStmt(subject, arms) => {
                self.gen_match(&subject, &arms);
            }
            Stmt::ExprStmt(expr) => {
                self.gen_expr_stmt(&expr);
            }
            _ => {
                println!("{}", "[hackerc-self] codegen: nieobslugiwana instrukcja w gen_stmt (np. 'struct'/'enum'/'fun'/'impl' zagniezdzone w ciele - dozwolone TYLKO na najwyzszym poziomie pliku)".to_string());
            }
        }
    }

    pub fn gen_stmts(&mut self, stmts: &Vec<Stmt>) {
        let mut i: i64 = 0;
        let mut n = (stmts.len() as i64);
        while (i < n) {
            self.gen_stmt(&stmts[i as usize].clone());
            i = (i + 1);
        }
    }

}

// -- Emisja deklaracji: `gen_struct`/`gen_enum`/`gen_fun`/`gen_impl` --
// -- (parytet z tymi samymi metodami `CodeGen`) --------------------
// Parytet z `_field_blocks_default()` - czy pole typu `t` BLOKUJE
// `#[derive(Default)]` na wlasnym strukcie (enumy/`Result` NIGDY nie
// maja Default, `Option`/`List`/`Dict` ZAWSZE maja Default
// niezaleznie od argumentu, struct zalezy REKURENCYJNIE od
// `no_default_structs`, ktory buduje `compute_no_default_structs`
// ponizej punktem stalym).
pub fn field_type_blocks_default(t: &TypeRef, sigs: &Signatures, no_default_structs: &std::collections::HashMap<String, bool>) -> bool {
    if (((t.name.to_string() == "Option".to_string().to_string()) || (t.name.to_string() == "List".to_string().to_string())) || (t.name.to_string() == "Dict".to_string().to_string())) {
        return false;
    }
    if sigs.enums.contains_key(t.name.clone().as_str()) {
        return true;
    }
    if (t.name.to_string() == "Result".to_string().to_string()) {
        return true;
    }
    if sigs.structs.contains_key(t.name.clone().as_str()) {
        return no_default_structs.contains_key(t.name.as_str());
    }
    return false;
}

// Parytet z petla punktu-stalego przy `self._no_default_structs = ...`
// w `__init__`/`gen_program`. Pole Boxowane "direct" ZAWSZE blokuje
// (`Box<X>: Default` wymaga `X: Default`, ktorego nie sprawdzamy tu
// rekurencyjnie - parytet z oryginalem, ktory rowniez po prostu
// blokuje na "direct" bez dalszej analizy); "option" NIGDY nie
// blokuje (`Option<Box<X>>` jest Default niezaleznie od X).
pub fn compute_no_default_structs(sigs: &Signatures, boxed_fields: &std::collections::HashMap<String, String>) -> std::collections::HashMap<String, bool> {
    let mut result: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut changed: bool = true;
    while changed {
        changed = false;
        let mut i: i64 = 0;
        let mut n = (sigs.struct_names.len() as i64);
        while (i < n) {
            let mut name = sigs.struct_names[i as usize].clone().clone();
            if !(result.contains_key(name.clone().as_str())) {
                match sigs.structs.get(name.clone().as_str()).cloned() {
                    Some(decl) => {
                        let mut fields: Vec<Param> = struct_decl_fields_local(&decl);
                        let mut j: i64 = 0;
                        let mut fn_ = (fields.len() as i64);
                        let mut blocks: bool = false;
                        while ((j < fn_) && !(blocks)) {
                            let mut f = fields[j as usize].clone().clone();
                            match f.type_ref {
                                Some(ft) => {
                                    let mut key = format!("{}{}", format!("{}{}", name.clone(), "::".to_string()), f.name.clone());
                                    let mut kind: String = "".to_string();
                                    match boxed_fields.get(key.as_str()).cloned() {
                                        Some(k) => {
                                            kind = k;
                                        }
                                        None => {
                                        }
                                    }
                                    if (kind.to_string() == "direct".to_string().to_string()) {
                                        blocks = true;
                                    } else if ((kind.to_string() != "option".to_string().to_string()) && field_type_blocks_default(&ft, &sigs, &result)) {
                                        blocks = true;
                                    }
                                }
                                None => {
                                }
                            }
                            j = (j + 1);
                        }
                        if blocks {
                            result.insert(name, true);
                            changed = true;
                        }
                    }
                    None => {
                    }
                }
            }
            i = (i + 1);
        }
    }
    return result;
}

impl CodeGen {
    pub fn generic_head(&self, type_params: &Vec<String>) -> String {
        if ((type_params.len() as i64) == 0) {
            return "".to_string();
        }
        let mut out: String = "<".to_string();
        let mut i: i64 = 0;
        let mut n = (type_params.len() as i64);
        while (i < n) {
            if (i > 0) {
                out = format!("{}{}", out, ", ".to_string());
            }
            out = format!("{}{}", out, type_params[i as usize].clone().clone());
            i = (i + 1);
        }
        out = format!("{}{}", out, ">".to_string());
        return out.to_string();
    }

    /// Parytet z `_field_rust_type()` - jak `rust_type_name`, ale
    /// doklejajac `Box<...>`/`Option<Box<...>>` wokol pola oznaczonego
    /// w `boxed_fields`.
    pub fn field_rust_type(&self, owner_name: &String, field_name: &String, f_type: Option<TypeRef>) -> String {
        match f_type {
            Some(ft) => {
                let mut key: String = format!("{}{}", format!("{}{}", owner_name, "::".to_string()), field_name);
                match self.boxed_fields.get(key.as_str()).cloned() {
                    Some(kind) => {
                        if (kind.to_string() == "option".to_string().to_string()) {
                            match ft.generic.clone() {
                                Some(inner) => {
                                    return format!("{}{}", format!("{}{}", "Option<Box<".to_string(), rust_type_name(&inner, &self.sigs, &self.current_type_params)), ">>".to_string()).to_string();
                                }
                                None => {
                                    return rust_type_name(&ft, &self.sigs, &self.current_type_params).to_string();
                                }
                            }
                        }
                        if (kind.to_string() == "direct".to_string().to_string()) {
                            return format!("{}{}", format!("{}{}", "Box<".to_string(), rust_type_name(&ft, &self.sigs, &self.current_type_params)), ">".to_string()).to_string();
                        }
                        return rust_type_name(&ft, &self.sigs, &self.current_type_params).to_string();
                    }
                    None => {
                        return rust_type_name(&ft, &self.sigs, &self.current_type_params).to_string();
                    }
                }
            }
            None => {
                return "i64".to_string();
            }
        }
    }

    pub fn struct_ctor_params(&self, fields: &Vec<Param>) -> String {
        let mut out: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (fields.len() as i64);
        while (i < n) {
            if (i > 0) {
                out = format!("{}{}", out, ", ".to_string());
            }
            let mut f = fields[i as usize].clone().clone();
            let mut t: String = "i64".to_string();
            match f.type_ref {
                Some(ft) => {
                    t = rust_type_name(&ft, &self.sigs, &self.current_type_params);
                }
                None => {
                }
            }
            out = format!("{}{}", format!("{}{}", format!("{}{}", out, f.name), ": ".to_string()), t);
            i = (i + 1);
        }
        return out.to_string();
    }

    /// Parytet z budowaniem `fields_init` w `gen_struct()` - kazde pole
    /// oznaczone w `boxed_fields` dostaje `Box::new(...)`/`.map(Box::new)`
    /// przy KONSTRUKCJI (uzytkownik `Nazwa(a, b)` podaje ZWYKLE,
    /// nie-boxowane wartosci - boxowanie dzieje sie WEWNATRZ `::new`).
    pub fn struct_ctor_init(&self, name: &String, fields: &Vec<Param>) -> String {
        let mut out: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (fields.len() as i64);
        while (i < n) {
            if (i > 0) {
                out = format!("{}{}", out, ", ".to_string());
            }
            let mut f = fields[i as usize].clone().clone();
            let mut key = format!("{}{}", format!("{}{}", name.clone(), "::".to_string()), f.name.clone());
            match self.boxed_fields.get(key.as_str()).cloned() {
                Some(kind) => {
                    if (kind.to_string() == "option".to_string().to_string()) {
                        out = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", out, f.name.clone()), ": ".to_string()), f.name), ".map(Box::new)".to_string());
                    } else if (kind.to_string() == "direct".to_string().to_string()) {
                        out = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", out, f.name.clone()), ": Box::new(".to_string()), f.name), ")".to_string());
                    } else {
                        out = format!("{}{}", out, f.name);
                    }
                }
                None => {
                    out = format!("{}{}", out, f.name);
                }
            }
            i = (i + 1);
        }
        return out.to_string();
    }

    /// Parytet z `gen_struct()`: naglowek `#[derive(...)]` (bez
    /// `Default` dla generycznych/samo-referencyjnych - patrz
    /// `compute_no_default_structs`), pola (z auto-Box), i konstruktor
    /// pozycyjny `Nazwa(a, b)` -> `impl Nazwa { pub fn new(a, b) ->
    /// Self }`.
    pub fn gen_struct(&mut self, name: &String, fields: &Vec<Param>, type_params: &Vec<String>) {
        self.current_type_params = type_params.clone();
        let mut gen_head: String = self.generic_head(&type_params.clone());
        let mut is_recursive = self.no_default_structs.contains_key(name.clone().as_str());
        let mut derive: String = "#[derive(Debug, Clone, PartialEq)]".to_string();
        if (((type_params.len() as i64) == 0) && !(is_recursive)) {
            derive = "#[derive(Debug, Clone, PartialEq, Default)]".to_string();
        }
        self.emit(&derive);
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", "pub struct ".to_string(), name.clone()), gen_head.clone()), " {".to_string()));
        self.indent = (self.indent + 1);
        let mut i: i64 = 0;
        let mut n = (fields.len() as i64);
        while (i < n) {
            let mut f = fields[i as usize].clone().clone();
            let mut hint: String = self.field_rust_type(&name.clone(), &f.name.clone(), f.type_ref);
            self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "pub ".to_string(), f.name), ": ".to_string()), hint), ",".to_string()));
            i = (i + 1);
        }
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
        self.emit(&"".to_string());
        let mut params: String = self.struct_ctor_params(&fields.clone());
        let mut fields_init: String = self.struct_ctor_init(&name.clone(), &fields);
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "impl".to_string(), gen_head.clone()), " ".to_string()), name.clone()), gen_head), " {".to_string()));
        self.indent = (self.indent + 1);
        self.emit(&format!("{}{}", format!("{}{}", "pub fn new(".to_string(), params), ") -> Self {".to_string()));
        self.indent = (self.indent + 1);
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", name, " { ".to_string()), fields_init), " }".to_string()));
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
        self.emit(&"".to_string());
        self.current_type_params = vec![];
    }

    pub fn variant_field_rust_type(&self, owner_name: &String, variant_name: &String, idx: i64, t: &TypeRef) -> String {
        let mut key: String = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", owner_name, "::".to_string()), variant_name), "#".to_string()), (idx).to_string());
        match self.boxed_fields.get(key.as_str()).cloned() {
            Some(kind) => {
                if (kind.to_string() == "option".to_string().to_string()) {
                    match t.generic.clone() {
                        Some(inner) => {
                            return format!("{}{}", format!("{}{}", "Option<Box<".to_string(), rust_type_name(&inner, &self.sigs, &self.current_type_params)), ">>".to_string()).to_string();
                        }
                        None => {
                            return rust_type_name(&t, &self.sigs, &self.current_type_params).to_string();
                        }
                    }
                }
                if (kind.to_string() == "direct".to_string().to_string()) {
                    return format!("{}{}", format!("{}{}", "Box<".to_string(), rust_type_name(&t, &self.sigs, &self.current_type_params)), ">".to_string()).to_string();
                }
                return rust_type_name(&t, &self.sigs, &self.current_type_params).to_string();
            }
            None => {
                return rust_type_name(&t, &self.sigs, &self.current_type_params).to_string();
            }
        }
    }

    pub fn gen_enum(&mut self, name: &String, variants: &Vec<EnumVariant>, type_params: &Vec<String>) {
        self.current_type_params = type_params.clone();
        let mut gen_head: String = self.generic_head(&type_params);
        self.emit(&"#[derive(Debug, Clone, PartialEq)]".to_string());
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", "pub enum ".to_string(), name.clone()), gen_head), " {".to_string()));
        self.indent = (self.indent + 1);
        let mut i: i64 = 0;
        let mut n = (variants.len() as i64);
        while (i < n) {
            let mut v = variants[i as usize].clone().clone();
            if ((v.fields.len() as i64) > 0) {
                let mut args: String = "".to_string();
                let mut j: i64 = 0;
                let mut fn_ = (v.fields.len() as i64);
                while (j < fn_) {
                    if (j > 0) {
                        args = format!("{}{}", args, ", ".to_string());
                    }
                    args = format!("{}{}", args, self.variant_field_rust_type(&name.clone(), &v.name.clone(), j, &v.fields[j as usize].clone()));
                    j = (j + 1);
                }
                self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", v.name, "(".to_string()), args), "),".to_string()));
            } else {
                self.emit(&format!("{}{}", v.name, ",".to_string()));
            }
            i = (i + 1);
        }
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
        self.emit(&"".to_string());
        self.current_type_params = vec![];
    }

    pub fn param_type_str(&self, fun_name: &String, p: &Param) -> String {
        match p.type_ref.clone() {
            Some(pt) => {
                let mut base: String = rust_type_name(&pt.clone(), &self.sigs, &self.current_type_params);
                if self.is_refable_type(&pt) {
                    let mut mkey = format!("{}{}", format!("{}{}", fun_name, "::".to_string()), p.name.clone());
                    if self.mut_params.contains_key(mkey.as_str()) {
                        return format!("{}{}", "&mut ".to_string(), base).to_string();
                    }
                    return format!("{}{}", "&".to_string(), base).to_string();
                }
                return base.to_string();
            }
            None => {
                return "i64".to_string();
            }
        }
    }

    /// Parytet z `gen_fun()` - CZYSCI/USTAWIA `env_vars` na NOWO (nowa
    /// funkcja = nowy zasieg zmiennych, patrz uwaga o `env_vars` przy
    /// `CodeGen` wyzej), deklaruje parametry, i emituje sygnature z
    /// auto-`&`/`&mut` (`param_type_str`) + cialo (`gen_stmts`).
    pub fn gen_fun(&mut self, name: &String, params: &Vec<Param>, ret_type: Option<TypeRef>, body: &Vec<Stmt>, type_params: &Vec<String>) {
        self.current_type_params = type_params.clone();
        let mut gen_head: String = self.generic_head(&type_params);
        self.env_vars = std::collections::HashMap::new();
        let mut i: i64 = 0;
        let mut n = (params.len() as i64);
        while (i < n) {
            let mut p = params[i as usize].clone().clone();
            self.declare_env(&p.name, p.type_ref);
            i = (i + 1);
        }
        let mut params_str: String = "".to_string();
        let mut j: i64 = 0;
        while (j < n) {
            let mut p2 = params[j as usize].clone().clone();
            if (j > 0) {
                params_str = format!("{}{}", params_str, ", ".to_string());
            }
            params_str = format!("{}{}", format!("{}{}", format!("{}{}", params_str, p2.name.clone()), ": ".to_string()), self.param_type_str(&name.clone(), &p2));
            j = (j + 1);
        }
        let mut ret: String = "".to_string();
        match ret_type.clone() {
            Some(rt) => {
                ret = format!("{}{}", " -> ".to_string(), rust_type_name(&rt, &self.sigs, &self.current_type_params));
            }
            None => {
            }
        }
        self.current_ret_type = ret_type;
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "pub fn ".to_string(), name), gen_head), "(".to_string()), params_str), ")".to_string()), ret), " {".to_string()));
        self.indent = (self.indent + 1);
        let mut str_param_names: Vec<String> = vec![];
        let mut k: i64 = 0;
        while (k < n) {
            let mut pk = params[k as usize].clone().clone();
            match pk.type_ref.clone() {
                Some(pt) => {
                    if (pt.name.to_string() == "Str".to_string().to_string()) {
                        str_param_names.push(pk.name);
                    }
                }
                None => {
                }
            }
            k = (k + 1);
        }
        self.char_cache_params = char_indexed_str_params(&body, &str_param_names, &self.sigs);
        let mut ci: i64 = 0;
        let mut cn = (str_param_names.len() as i64);
        while (ci < cn) {
            let mut pname = str_param_names[ci as usize].clone().clone();
            if self.char_cache_params.contains_key(pname.clone().as_str()) {
                self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "let __hks_chars_".to_string(), pname.clone()), ": Vec<char> = ".to_string()), pname.clone()), ".chars().collect();".to_string()));
            }
            ci = (ci + 1);
        }
        self.gen_stmts(&body);
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
        self.emit(&"".to_string());
        self.current_type_params = vec![];
        self.current_ret_type = None;
        self.char_cache_params = std::collections::HashMap::new();
    }

    /// Parytet z `gen_method()` - jak `gen_fun`, ale pierwszy parametr
    /// `self` staje sie `&self`/`&mut self` (mutowalnosc z
    /// `method_mut_params`, klucz `"Struct::metoda::self"`), reszta
    /// parametrow uzywa tej samej logiki co wolne funkcje, tylko z
    /// kluczem `"Struct::metoda::param"` zamiast `"funkcja::param"`.
    pub fn gen_method(&mut self, struct_name: &String, m: &Stmt) {
        let mut name: String = fun_decl_name(&m.clone());
        let mut params: Vec<Param> = fun_decl_params(&m.clone());
        let mut ret_type: Option<TypeRef> = fun_decl_ret_type(&m.clone());
        let mut body: Vec<Stmt> = fun_decl_body(&m);
        self.env_vars = std::collections::HashMap::new();
        self.declare_env(&"self".to_string(), Some(TypeRef::new(struct_name.clone(), None, None)));
        let mut key_prefix = format!("{}{}", format!("{}{}", format!("{}{}", struct_name.clone(), "::".to_string()), name.clone()), "::".to_string());
        let mut params_str: String = "&self".to_string();
        if self.method_mut_params.contains_key(format!("{}{}", key_prefix.clone(), "self".to_string()).as_str()) {
            params_str = "&mut self".to_string();
        }
        let mut i: i64 = 0;
        let mut n = (params.len() as i64);
        while (i < n) {
            let mut p = params[i as usize].clone().clone();
            if (p.name.to_string() != "self".to_string().to_string()) {
                self.declare_env(&p.name.clone(), p.type_ref.clone());
                match p.type_ref.clone() {
                    Some(pt) => {
                        let mut base: String = rust_type_name(&pt.clone(), &self.sigs, &self.current_type_params);
                        if self.is_refable_type(&pt) {
                            let mut kp: String = key_prefix.clone();
                            let mut pn: String = p.name.clone();
                            let mut mkey: String = format!("{}{}", kp, pn);
                            let mut prefix: String = "&".to_string();
                            if self.method_mut_params.contains_key(mkey.as_str()) {
                                prefix = "&mut ".to_string();
                            }
                            params_str = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", params_str, ", ".to_string()), p.name), ": ".to_string()), prefix), base);
                        } else {
                            params_str = format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", params_str, ", ".to_string()), p.name), ": ".to_string()), base);
                        }
                    }
                    None => {
                        params_str = format!("{}{}", format!("{}{}", format!("{}{}", params_str, ", ".to_string()), p.name), ": i64".to_string());
                    }
                }
            }
            i = (i + 1);
        }
        let mut ret: String = "".to_string();
        match ret_type.clone() {
            Some(rt) => {
                ret = format!("{}{}", " -> ".to_string(), rust_type_name(&rt, &self.sigs, &self.current_type_params));
            }
            None => {
            }
        }
        self.current_ret_type = ret_type;
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "pub fn ".to_string(), name), "(".to_string()), params_str), ")".to_string()), ret), " {".to_string()));
        self.indent = (self.indent + 1);
        self.gen_stmts(&body);
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
        self.emit(&"".to_string());
        self.current_ret_type = None;
    }

    pub fn gen_impl(&mut self, struct_name: &String, methods: &Vec<Stmt>, type_params: &Vec<String>) {
        self.current_type_params = type_params.clone();
        let mut gen_head: String = self.generic_head(&type_params);
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "impl".to_string(), gen_head.clone()), " ".to_string()), struct_name.clone()), gen_head), " {".to_string()));
        self.indent = (self.indent + 1);
        let mut i: i64 = 0;
        let mut n = (methods.len() as i64);
        while (i < n) {
            self.gen_method(&struct_name.clone(), &methods[i as usize].clone().clone());
            i = (i + 1);
        }
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
        self.emit(&"".to_string());
        self.current_type_params = vec![];
    }

    /// Parytet z `gen_get_import()` - `use crate::<modul>::...;` dla
    /// `std`/`core`/`selfhost` i `crates` (prawdziwa zaleznosc Cargo,
    /// sama zaleznosc dopisywana do Cargo.toml przez `project.hcs`,
    /// jeszcze nieistniejacy - tu TYLKO `use`), komentarz-wyjasnienie
    /// dla `pypi` (dostepne tylko w `__direct__(2)`) i `npm`/`jsr` (jeszcze
    /// w budowie).
    /// Parytet z `gen_include()` (codegen.py) - `include <sciezka>`
    /// zawsze glob-importuje (`use crate::module::*;`), bez `import
    /// <details>` (patrz `IncludeStmt` w ast_nodes.hcs).
    pub fn gen_include(&mut self, path: &String) {
        let mut module: String = flat_include_module_name(&path);
        self.emit(&format!("{}{}", format!("{}{}", "use crate::".to_string(), module), "::*;".to_string()));
    }

    pub fn gen_get_import(&mut self, source: &String, name: &String, version: Option<String>, details: &Vec<String>) {
        if (((source.to_string() == "std".to_string().to_string()) || (source.to_string() == "core".to_string().to_string())) || (source.to_string() == "selfhost".to_string().to_string())) {
            let mut module: String = flat_module_name(&source, &name.clone(), version);
            if ((details.len() as i64) > 0) {
                self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "use crate::".to_string(), module), "::{".to_string()), self.join_binds(&details)), "};".to_string()));
            } else {
                self.emit(&format!("{}{}", format!("{}{}", "use crate::".to_string(), module), "::*;".to_string()));
            }
            return;
        }
        if (source.to_string() == "crates".to_string().to_string()) {
            if ((details.len() as i64) > 0) {
                self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "use ".to_string(), name.clone()), "::{".to_string()), self.join_binds(&details)), "};".to_string()));
            } else {
                self.emit(&format!("{}{}", format!("{}{}", "use ".to_string(), name), "::*;".to_string()));
            }
            return;
        }
        if (source.to_string() == "pypi".to_string().to_string()) {
            self.emit(&format!("{}{}", format!("{}{}", "// get <pypi:".to_string(), name), "> - dostepne wewnatrz bloku __direct__(3) (interpreter Pythona), nie bezposrednio w kodzie Rust".to_string()));
            return;
        }
        if ((source.to_string() == "npm".to_string().to_string()) || (source.to_string() == "jsr".to_string().to_string())) {
            self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "// get <".to_string(), source), ":".to_string()), name), "> - pobierane przez 'virus install' do cache/libs/".to_string()), source), "/".to_string()), name), "/; integracja z uruchomieniem JS jest jeszcze w budowie, patrz docs/ROADMAP.md".to_string()));
            return;
        }
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "// get <".to_string(), source), ":".to_string()), name), "> - nieznane zrodlo, pomijam import".to_string()));
    }

    /// Parytet z `gen_extern()` - deklaracja FFI: `#[link(name =
    /// "lib")] extern "C" { pub fn nazwa(...) -> Typ; }`.
    pub fn gen_extern(&mut self, lib: &String, name: &String, params: &Vec<Param>, ret_type: Option<TypeRef>) {
        let mut params_str: String = "".to_string();
        let mut i: i64 = 0;
        let mut n = (params.len() as i64);
        while (i < n) {
            if (i > 0) {
                params_str = format!("{}{}", params_str, ", ".to_string());
            }
            let mut p = params[i as usize].clone().clone();
            let mut t: String = "i64".to_string();
            match p.type_ref {
                Some(pt) => {
                    t = rust_type_name(&pt, &self.sigs, &self.current_type_params);
                }
                None => {
                }
            }
            params_str = format!("{}{}", format!("{}{}", format!("{}{}", params_str, p.name), ": ".to_string()), t);
            i = (i + 1);
        }
        let mut ret: String = "".to_string();
        match ret_type {
            Some(rt) => {
                ret = format!("{}{}", " -> ".to_string(), rust_type_name(&rt, &self.sigs, &self.current_type_params));
            }
            None => {
            }
        }
        self.emit(&format!("{}{}", format!("{}{}", "#[link(name = \"".to_string(), lib), "\")]".to_string()));
        self.emit(&"extern \"C\" {".to_string());
        self.indent = (self.indent + 1);
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "pub fn ".to_string(), name), "(".to_string()), params_str), ")".to_string()), ret), ";".to_string()));
        self.indent = (self.indent - 1);
        self.emit(&"}".to_string());
        self.emit(&"".to_string());
    }

    pub fn string_lit_value(&self, e: &Expr) -> String {
        let mut c = e.clone();
        match c {
            Expr::StringLit(value, is_doc) => {
                return format!("{}{}", value, "".to_string()).to_string();
            }
            _ => {
                return "".to_string();
            }
        }
    }

    /// Parytet z obsluga `LetStmt` (`is_const`) w `gen_toplevel()` -
    /// stala globalna. `Str` MUSI byc `&str` (nie `String`), bo `.
    /// to_string()` NIE jest funkcja `const` w Rust - stad wymog, ze
    /// wartosc MUSI byc doslownym literalem string (nie dowolnym
    /// wyrazeniem).
    pub fn gen_const(&mut self, name: &String, type_ref: Option<TypeRef>, value: Option<Expr>) {
        let mut upper_name: String = str_to_upper(&name);
        let mut is_str_type: bool = false;
        match type_ref.clone() {
            Some(t) => {
                if (t.name.to_string() == "Str".to_string().to_string()) {
                    is_str_type = true;
                }
            }
            None => {
            }
        }
        if is_str_type {
            match value.clone() {
                Some(v) => {
                    if self.expr_is_string_lit(&v.clone()) {
                        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "pub const ".to_string(), upper_name), ": &str = ".to_string()), rust_string_literal(&self.string_lit_value(&v))), ";".to_string()));
                        return;
                    }
                    println!("{}", "[hackerc-self] codegen: stala globalna typu Str musi byc literalem string".to_string());
                    return;
                }
                None => {
                }
            }
            return;
        }
        let mut hint: String = "i64".to_string();
        match type_ref {
            Some(t2) => {
                hint = rust_type_name(&t2, &self.sigs, &self.current_type_params);
            }
            None => {
            }
        }
        let mut value_rendered: String = "Default::default()".to_string();
        match value {
            Some(v2) => {
                value_rendered = self.gen_expr(&v2);
            }
            None => {
            }
        }
        self.emit(&format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "pub const ".to_string(), upper_name), ": ".to_string()), hint), " = ".to_string()), value_rendered), ";".to_string()));
    }

    /// Dysponent instrukcji NAJWYZSZEGO POZIOMU - parytet z
    /// `gen_toplevel()`. Obejmuje WSZYSTKO, co moze wystapic w
    /// `Program.body` (nie tylko `struct`/`enum`/`fun`/`impl` jak
    /// wczesniejsza, prostsza wersja `gen_program`'s petli).
    pub fn gen_toplevel(&mut self, node: &Stmt) {
        let mut s = node.clone();
        match s {
            Stmt::UsingStmt(version) => {
                self.emit(&format!("{}{}", format!("{}{}", "// using <".to_string(), version), "> (wymagana wersja hackerc)".to_string()));
            }
            Stmt::GetImportStmt(source, name, version, details) => {
                self.seen_real_toplevel_item = true;
                self.gen_get_import(&source, &name, version, &details);
            }
            Stmt::IncludeStmt(path) => {
                self.seen_real_toplevel_item = true;
                self.gen_include(&path);
            }
            Stmt::GcPragma(mode) => {
                self.emit(&format!("{}{}", format!("{}{}", "// gc:use::".to_string(), mode), " - Rust: brak GC, wlasnosc/pozyczanie zamiast tego".to_string()));
            }
            Stmt::StructDecl(name, fields, type_params) => {
                self.seen_real_toplevel_item = true;
                self.gen_struct(&name, &fields, &type_params);
            }
            Stmt::EnumDecl(name, variants, type_params) => {
                self.seen_real_toplevel_item = true;
                self.gen_enum(&name, &variants, &type_params);
            }
            Stmt::ImplDecl(struct_name, methods, itype_params) => {
                self.seen_real_toplevel_item = true;
                self.gen_impl(&struct_name, &methods, &itype_params);
            }
            Stmt::ExternFunDecl(lib, name, params, ret_type) => {
                self.seen_real_toplevel_item = true;
                self.gen_extern(&lib, &name, &params, ret_type);
            }
            Stmt::FunDecl(fname, fparams, fret, fbody, fis_pub, ftype_params) => {
                self.seen_real_toplevel_item = true;
                self.gen_fun(&fname, &fparams, fret, &fbody, &ftype_params);
            }
            Stmt::LetStmt(cname, ctype, cvalue, cis_const) => {
                if cis_const {
                    self.seen_real_toplevel_item = true;
                    self.gen_const(&cname, ctype, cvalue);
                } else {
                    println!("{}", "[hackerc-self] codegen: 'let' (nie-const) niedozwolone na najwyzszym poziomie - tylko 'const'".to_string());
                }
            }
            Stmt::ExprStmt(expr) => {
                if self.expr_is_doc_string(&expr.clone()) {
                    self.emit_module_doc_comment(&expr);
                } else {
                    println!("{}", "[hackerc-self] codegen: nieobslugiwane wyrazenie na najwyzszym poziomie".to_string());
                }
            }
            _ => {
                println!("{}", "[hackerc-self] codegen: nieobslugiwana instrukcja na najwyzszym poziomie (np. 'match'/'if'/'while' - dozwolone TYLKO wewnatrz cial funkcji)".to_string());
            }
        }
    }

    /// Komentarz dokumentacyjny modulu (`//!`) - TYLKO gdy jeszcze nie
    /// wyemitowano zadnego prawdziwego elementu (struct/enum/impl/fun/const/
    /// use) - w przeciwnym razie `//!` jest niepoprawnym Rustem (E0753,
    /// patrz komentarz przy polu `seen_real_toplevel_item`), wiec spada do
    /// zwyklego `//`. Parytet z `ExprStmt(StringLit)` na najwyzszym poziomie
    /// w `gen_toplevel()`.
    pub fn emit_module_doc_comment(&mut self, e: &Expr) {
        let mut c = e.clone();
        match c {
            Expr::StringLit(value, is_doc) => {
                if self.seen_real_toplevel_item {
                    self.emit(&format!("{}{}", "// ".to_string(), value));
                } else {
                    self.emit(&format!("{}{}", "//! ".to_string(), value));
                }
            }
            _ => {
            }
        }
    }

}

// Konstruktor `CodeGen` z pustymi mapami "roboczymi" (`env_vars`/
// `output`) i JUZ POLICZONYMI mapami "globalnymi" (sygnatury/Box/mut) -
// parytet z `CodeGen.__init__`.
// Konstruktor `CodeGen` z pustymi mapami "roboczymi" (`env_vars`/
// `output`) i JUZ POLICZONYMI mapami "globalnymi" (sygnatury/Box/mut) -
// parytet z `CodeGen.__init__`. `sigs.clone()` KONIECZNE (nie
// kosmetyczne): `sigs: Signatures` jest PARAMETREM (struct, wiec
// "refable" - zawsze `&Signatures` w Rust), a pole `CodeGen.sigs` jest
// zwyklym, OWNED `Signatures` - podanie GOLEGO `sigs` (referencji) do
// konstruktora bylo by niezgodnoscia typow (`&Signatures` vs
// `Signatures`) - **bug znaleziony i naprawiony w TEJ sesji**, ten sam
// rodzaj co `extra_variant_names.clone()` w typecheck.hcs (krok 5/N).
pub fn new_codegen(sigs: &Signatures, boxed_fields: &std::collections::HashMap<String, String>, mut_params: &std::collections::HashMap<String, bool>, method_mut_params: &std::collections::HashMap<String, bool>, variant_arity: &std::collections::HashMap<String, i64>, no_default_structs: &std::collections::HashMap<String, bool>, direct_blocks: &std::collections::HashMap<String, String>) -> CodeGen {
    let mut env_vars: std::collections::HashMap<String, Option<TypeRef>> = std::collections::HashMap::new();
    return CodeGen::new(sigs.clone(), (env_vars).clone(), (variant_arity).clone(), (boxed_fields).clone(), (method_mut_params).clone(), (mut_params).clone(), vec![], None, vec![], 0, (no_default_structs).clone(), false, (direct_blocks).clone(), false, std::collections::HashMap::new());
}

// Punkt wejscia - parytet z `CodeGen.gen_program()`. Liczy WSZYSTKIE
// analizy (sygnatury, Box, auto-`&mut`, `no_default_structs`) RAZ, na
// poczatku, potem przechodzi `prog.body` emitujac struct/enum/fun/impl
// po kolei. Zwraca `List<Str>` (linie wygenerowanego Rusta) - CZYSTA
// WARTOSC, nie napisany plik - laczenie WIELU plikow (naglowek
// `use`/moduly) to zadanie `project.hcs` (jeszcze nieistniejacego).
// Punkt wejscia - parytet z `CodeGen.gen_program()`. Liczy WSZYSTKIE
// analizy (sygnatury, Box, auto-`&mut`, `no_default_structs`) RAZ, na
// poczatku, potem przechodzi `prog.body` przez `gen_toplevel`
// (obsluguje WSZYSTKIE formy najwyzszego poziomu, nie tylko
// struct/enum/fun/impl). Doklada NAGLOWEK (komentarz +
// `#![allow(...)]` + `use pyo3::prelude::*;` TYLKO jesli
// `gen.needs_pyo3` - ustawiane przez `gen_direct` - patrz uwaga
// projektowa przy `gen_direct` co do `direct_blocks` jako danych
// WEJSCIOWYCH). Zwraca `List<Str>` (linie wygenerowanego Rusta) -
// CZYSTA WARTOSC, nie napisany plik - laczenie WIELU plikow (moduly)
// to zadanie `project.hcs` (jeszcze nieistniejacego).
pub fn gen_program(prog: &Program) -> Vec<String> {
    let mut sigs = collect_signatures(&prog.clone());
    let mut info: RecursionInfo = build_recursion_info(&sigs.clone());
    let mut mparams: std::collections::HashMap<String, bool> = compute_mut_params(&prog.clone());
    let mut empty_extra: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut mmparams: std::collections::HashMap<String, bool> = compute_method_mut_params(&prog.clone(), &empty_extra);
    let mut varity: std::collections::HashMap<String, i64> = build_variant_arity(&sigs.clone());
    let mut ndstructs: std::collections::HashMap<String, bool> = compute_no_default_structs(&sigs.clone(), &info.boxed_fields.clone());
    let mut empty_direct_blocks: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut gen: CodeGen = new_codegen(&sigs, &info.boxed_fields, &mparams, &mmparams, &varity, &ndstructs, &empty_direct_blocks);
    let mut i: i64 = 0;
    let mut n = (prog.body.len() as i64);
    while (i < n) {
        gen.gen_toplevel(&prog.body[i as usize].clone().clone());
        i = (i + 1);
    }
    let mut header: Vec<String> = vec!["// Plik wygenerowany automatycznie przez hackerc (HackerScript -> Rust).".to_string(), "// NIE EDYTUJ RECZNIE - edytuj zrodlo .hcs i uruchom 'virus build' ponownie.".to_string(), "#![allow(non_snake_case, unused_mut, dead_code)]".to_string()];
    if gen.needs_pyo3 {
        header.push("use pyo3::prelude::*;".to_string());
    }
    header.push("".to_string());
    let mut out: Vec<String> = vec![];
    let mut hi: i64 = 0;
    let mut hn = (header.len() as i64);
    while (hi < hn) {
        out.push(header[hi as usize].clone().clone());
        hi = (hi + 1);
    }
    let mut oi: i64 = 0;
    let mut on = (gen.output.len() as i64);
    while (oi < on) {
        out.push(gen.output[oi as usize].clone().clone());
        oi = (oi + 1);
    }
    return out;
}

// Demonstracyjne uzycie - `struct Node [ next: Node ]` (bezposrednia
// samo-rekurencja, oczekuje "direct") i `struct A [ b: B ], struct B [
// a: Option<A> ]` (POSREDNIA rekurencja przez dwa structy, jedna
// krawedz powinna dostac "option").
pub fn main() {
    let mut node_fields: Vec<Param> = vec![Param::new("next".to_string(), Some(TypeRef::new("Node".to_string(), None, None)), None)];
    let mut node_decl: Stmt = Stmt::StructDecl("Node".to_string(), (node_fields).clone(), vec![]);
    let mut a_fields: Vec<Param> = vec![Param::new("b".to_string(), Some(TypeRef::new("B".to_string(), None, None)), None)];
    let mut a_decl: Stmt = Stmt::StructDecl("A".to_string(), (a_fields).clone(), vec![]);
    let mut b_fields: Vec<Param> = vec![Param::new("a".to_string(), Some(TypeRef::new("Option".to_string(), Some(TypeRef::new("A".to_string(), None, None)), None)), None)];
    let mut b_decl: Stmt = Stmt::StructDecl("B".to_string(), (b_fields).clone(), vec![]);
    let mut prog = Program::new(vec![node_decl, a_decl, b_decl]);
    let mut sigs = collect_signatures(&prog);
    let mut info: RecursionInfo = build_recursion_info(&sigs);
    println!("{} {}", "Node::next boxed?".to_string(), info.boxed_fields.contains_key("Node::next".to_string().as_str()));
    println!("{} {}", "A::b boxed?".to_string(), info.boxed_fields.contains_key("A::b".to_string().as_str()));
    println!("{} {}", "B::a boxed?".to_string(), info.boxed_fields.contains_key("B::a".to_string().as_str()));
    let mut body: Vec<Stmt> = vec![Stmt::LetStmt("msg".to_string(), None, Some(Expr::StringLit("".to_string(), false)), false), Stmt::IfStmt(Expr::BinOp(">".to_string(), Box::new(Expr::IdentExpr("n".to_string())), Box::new(Expr::NumberLit("0".to_string()))), vec![Stmt::AssignStmt(Expr::IdentExpr("msg".to_string()), "=".to_string(), Expr::StringLit("dodatnie".to_string(), false))], vec![], Some(vec![Stmt::AssignStmt(Expr::IdentExpr("msg".to_string()), "=".to_string(), Expr::StringLit("niedodatnie".to_string(), false))])), Stmt::ReturnStmt(Some(Expr::IdentExpr("msg".to_string())))];
    let mut empty_sigs = collect_signatures(&Program::new(vec![]));
    let mut empty_dict1: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut empty_dict2: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut empty_dict3: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut empty_dict4: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut empty_vars: std::collections::HashMap<String, Option<TypeRef>> = std::collections::HashMap::new();
    let mut empty_dict5: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut empty_dict6: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut gen: CodeGen = CodeGen::new((empty_sigs).clone(), (empty_vars).clone(), (empty_dict1).clone(), (empty_dict2).clone(), (empty_dict3).clone(), (empty_dict4).clone(), vec![], Some(TypeRef::new("Str".to_string(), None, None)), vec![], 0, (empty_dict5).clone(), false, (empty_dict6).clone(), false, std::collections::HashMap::new());
    gen.gen_stmts(&body);
    let mut i: i64 = 0;
    let mut n = (gen.output.len() as i64);
    while (i < n) {
        println!("{}", gen.output[i as usize].clone());
        i = (i + 1);
    }
    let mut point_fields: Vec<Param> = vec![Param::new("x".to_string(), Some(TypeRef::new("Int".to_string(), None, None)), None), Param::new("y".to_string(), Some(TypeRef::new("Int".to_string(), None, None)), None)];
    let mut point_decl: Stmt = Stmt::StructDecl("Point".to_string(), (point_fields).clone(), vec![]);
    let mut sum_body: Vec<Stmt> = vec![Stmt::ReturnStmt(Some(Expr::BinOp("+".to_string(), Box::new(Expr::NumberLit("1".to_string())), Box::new(Expr::NumberLit("2".to_string())))))];
    let mut sum_fun: Stmt = Stmt::FunDecl("add_one_two".to_string(), vec![], Some(TypeRef::new("Int".to_string(), None, None)), (sum_body).clone(), false, vec![]);
    let mut demo_prog = Program::new(vec![point_decl, sum_fun]);
    let mut generated: Vec<String> = gen_program(&demo_prog);
    let mut gi: i64 = 0;
    let mut gn = (generated.len() as i64);
    while (gi < gn) {
        println!("{}", generated[gi as usize].clone());
        gi = (gi + 1);
    }
}

// ## Stan tej sesji (patrz docs/ROADMAP.md - "krok 6/N, W PELNI ZAMKNIETY")
// 
// `codegen.hcs` jest teraz W PELNI KOMPLETNY jako generator
// POJEDYNCZEGO pliku `.hcs` -> Rust: `rust_type_name` +
// `RecursionAnalyzer`/`build_recursion_info` (Box) + `MutTracker`/
// `SelfCallTracker`/`compute_mut_params`/`compute_method_mut_params`
// (auto-`&mut`, z punktem stalym) + `compute_no_default_structs`
// (`#[derive(Default)]`) + `CodeGen` (struct: `sigs`/`env_vars`/
// `variant_arity`/`boxed_fields`/`method_mut_params`/`mut_params`/
// `current_type_params`/`current_ret_type`/`output`/`indent`/
// `no_default_structs`/`needs_pyo3`/`direct_blocks`) + PELNE
// `gen_expr`/`gen_stmt`/`gen_struct`/`gen_enum`/`gen_fun`/`gen_impl`/
// `gen_method`/`gen_extern`/`gen_const`/`gen_get_import`/`gen_direct`
// (PyO3)/`gen_toplevel`/`gen_program`. `gen_toplevel` obsluguje
// WSZYSTKIE formy najwyzszego poziomu (nie tylko struct/enum/fun/impl -
// DOPISANE: `using`/`get <...>`/`gc:use::`/`extern`/`const`/komentarz
// dokumentacyjny modulu). `gen_program(prog: Program) -> List<Str>`
// spina WSZYSTKO w JEDEN dzialajacy generator - liczy wszystkie
// analizy raz, doklada NAGLOWEK pliku (`#![allow(...)]` + warunkowo
// `use pyo3::prelude::*;`), potem przechodzi `prog.body`.
// 
// **KRZYZOWO ZWERYFIKOWANY wobec PRAWDZIWEGO Pythonowego `hackerc`**
// na rownowaznym programie (`struct Point [...]` + prosta `fun`) -
// identyczny ksztalt wyjscia (patrz docs/ROADMAP.md po szczegoly).
// 
// **CALKOWICIE NIE ZROBIONE (naleza do INNYCH, przyszlych modulow, nie
// do tego pliku)**:
// - Wyciaganie surowego tekstu `__direct__(4)` ZE ZRODLA `.hcs` PRZED
// tokenizacja (`transpiler.py`/`_extract_direct_blocks`) - `gen_direct`
// w TYM pliku jest KOMPLETNE i czeka na `direct_blocks` jako dane
// wejsciowe, ktore dostarczy `transpiler.hcs` (przyszly krok).
// - Wielo-plikowe skladanie/system modulow/Cargo.toml (`project.hcs`).
// 
// ## Ograniczenia
// 
// - `rust_type_name` nie ma prawdziwego `CodegenError` - `log`-uje i
// zwraca przyblizenie (ta sama filozofia co `expect` w parser.hcs).
// - `RecursionInfo.boxed_fields` splaszczone do JEDNEGO `Dict<Str,Str>`
// (klucz laczy poziom struct+pole/enum+wariant#idx) zamiast
// Pythonowych dwoch osobnych, zagniezdzonych map - rownowazne
// informacyjnie, mniej wygodne w API.
// - `compute_mut_params`/`compute_method_mut_params` splaszczone
// analogicznie (`Dict<Str,Bool>` z kluczem laczonym `"::"` zamiast
// Pythonowych `dict[str, set[str]]`).
// - `MutTracker`/`SelfCallTracker` maja dodatkowe pole `order:
// List<Str>` (obok `Dict<Str,Bool>`) WYLACZNIE dlatego, ze ten
// bootstrap nie ma iteracji po Dict - Python zwraca zwykly `set`.
// - `type_ref_generic`/`type_ref_generic2` (proba "zwrocenia" TypeRef
// wyciagnietego z Box) zostaly CALKOWICIE USUNIETE jako
// fundamentalnie niewykonalne w tym bootstrapie (patrz
// docs/ROADMAP.md) - zastapione `type_ref_generic_name`/
// `rust_type_name_of_generic`/`_of_generic2`.
// - `CodeGen.env` NIE jest `Option<TypeEnv>` (jak pierwotnie w
// Pythonie) - splaszczone do `env_vars: Dict<Str, Option<TypeRef>>`
// BEZPOSREDNIO, z tego samego powodu co `FnChecker` w typecheck.hcs
// (`self.env.declare(...)` nie wymuszaloby `&mut self`).
// - `gen_let_stmt`'s `skip_hint` implementuje TYLKO jeden z trzech
// warunkow oryginalu (`_contains_any`) - reszta wymaga infrastruktury
// z `project.hcs` (jeszcze nieistniejacego).
// - Wielo-plikowe skladanie (naglowki `use`/system modulow) to
// zadanie `project.hcs` (kolejny krok) - `gen_program` obsluguje
// TYLKO pojedynczy plik.
// - `extern_libs` (Set nazw bibliotek do polaczenia) z oryginalu NIE
// zostal przepisany - w Pythonie i tak nic z nim nie robi poza
// iteracja+`pass` (linki emitowane bezposrednio przy kazdym
// `ExternFunDecl` przez `gen_extern`), wiec pominiecie go jest
// nieszkodliwe (brak roznicy w zachowaniu).
// - NIEPRZETESTOWANE na prawdziwym wejsciu w tym srodowisku (brak
// rustc) - zweryfikowane strukturalnie przez `hackerc check`/
// `build`, inspekcje wygenerowanego Rusta, I krzyzowe porownanie z
// PRAWDZIWYM Pythonowym `hackerc` na rownowaznym programie - patrz
// tests/test_hackerc.py.

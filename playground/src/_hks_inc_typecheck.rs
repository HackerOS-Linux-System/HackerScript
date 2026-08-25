#![allow(non_snake_case, unused_mut, dead_code)]

//! bootstrap/hackerc-self/typecheck.hcs
//! 
//! Krok 5/N przepisania calego hackerc na HackerScript (patrz
//! docs/ROADMAP.md, "W TOKU"). Parytet z hackerc/hackerc/typecheck.py
//! (253 linie) - przebieg statycznej analizy AST: E0001 (zla liczba
//! argumentow), E0002 (brak `end <wartosc>`), E0003 (nieznane zrodlo
//! `get <...>`), E0005 (niezgodny typ `let`), E0011 (`?` w funkcji
//! bez Result/Option), W0001 (nieuzywana zmienna), W0002 (nieznana
//! funkcja - tylko ostrzezenie, bootstrap nie widzi jeszcze innych
//! plikow).
//! 
//! Kompilowany dzis przez STAGE0 (Pythonowy hackerc) - patrz
//! bootstrap/README.md.
//! 
//! ## Uwaga projektowa - DODATKOWA pulapka odkryta w tej sesji (obok
//! tych z typeinfer.hcs)
//! 
//! `self.pole` (dostep do pola PRZEZ `self`, ktore w metodzie jest
//! ZAWSZE referencja `&Self`/`&mut Self`) NIE MOZE byc dopasowywane
//! (`match self.pole [...]`) bez `self.pole.clone()` NAJPIERW - w
//! przeciwienstwie do dopasowywania pola OWNED lokalnej zmiennej
//! (np. `ta.generic` w typeinfer.hcs, gdzie `ta` jest zwykla,
//! wlasna, NIE-referencyjna zmienna lokalna - tam klon NIE byl
//! potrzebny). Rust odrzucilby "cannot move out of `self.pole` which
//! is behind a reference" bez tego klonu. W tym pliku KAZDE
//! dopasowanie pola `self.*` jest wiec poprzedzone `.clone()` -
//! defensywnie, nawet tam gdzie czesciowy ruch OWNED lokalnej
//! zmiennej moglby teoretycznie zadzialac (nie chcemy polegac na
//! subtelnej analizie "czy ta konkretna metoda jest jeszcze potrzebna
//! PO tym dopasowaniu" bez `rustc` do zweryfikowania).
//! 
//! Podobnie: TypeEnv NIE jest polem struktury `FnChecker` (bo
//! wywolanie metody UZYTKOWNIKA (`self.env.declare(...)`) na POLU
//! `self` NIE JEST wykrywane przez mechanizm auto-`&mut self` tego
//! codegen - `_mutated_names_in_body`/`mark_base` rozpoznaje TYLKO
//! (a) bezposrednie `self.pole = ...`/`self.pole.WBUDOWANA_METODA()`
//! (push/pop/insert/remove/clear/sort/extend/truncate - TYLKO ta
//! stala lista, NIE dowolna metoda uzytkownika) NA POLU BEDACYM
//! BEZPOSREDNIM atrybutem `self`, oraz (b) `self.metoda()` (BEZPOSREDNIE
//! wywolanie INNEJ metody TEJ SAMEJ struktury). `self.env.declare(...)`
//! nie pasuje do ANI (a) (bo "declare" nie jest w liscie wbudowanych),
//! ANI (b) (bo cel wywolania to `self.env`, nie `self`) - takie
//! wywolanie NIE WYMUSZILOBY `&mut self` na metodzie, ktora by je
//! zawierala, co dalej skutkowaloby prawdziwym bledem kompilacji Rust
//! ("cannot borrow as mutable"). Rozwiazanie: `FnChecker` przechowuje
//! `vars: Dict<Str, Option<TypeRef>>`/`sigs: Signatures` BEZPOSREDNIO
//! (nie zapakowane w `TypeEnv`), mutuje `self.vars.insert(...)`
//! WPROST (co JEST na liscie (a)), i buduje SWIEZY `TypeEnv` (przez
//! `env_snapshot`) TYLKO gdy trzeba wywolac `infer_expr_type`
//! (ktora TYLKO CZYTA `env`, nigdy nie mutuje).
use crate::_hks_inc_ast_nodes::*;
use crate::_hks_inc_diagnostics::*;
use crate::_hks_inc_typeinfer::*;
pub fn is_known_get_source(s: &String) -> bool {
    return (((((s.to_string() == "pypi".to_string().to_string()) || (s.to_string() == "crates".to_string().to_string())) || (s.to_string() == "std".to_string().to_string())) || (s.to_string() == "core".to_string().to_string())) || (s.to_string() == "selfhost".to_string().to_string()));
}

pub fn is_builtin_func(name: &String) -> bool {
    return ((((((((((((name.to_string() == "log".to_string().to_string()) || (name.to_string() == "__direct__".to_string().to_string())) || (name.to_string() == "some".to_string().to_string())) || (name.to_string() == "none".to_string().to_string())) || (name.to_string() == "ok".to_string().to_string())) || (name.to_string() == "err".to_string().to_string())) || (name.to_string() == "read_file".to_string().to_string())) || (name.to_string() == "write_file".to_string().to_string())) || (name.to_string() == "dict".to_string().to_string())) || (name.to_string() == "env_var".to_string().to_string())) || (name.to_string() == "run_command".to_string().to_string())) || (name.to_string() == "http_get".to_string().to_string()));
}

pub fn str_starts_with_underscore(s: &String) -> bool {
    if ((s.len() as i64) == 0) {
        return false;
    }
    return ((s.chars().nth(0 as usize).map(|c| c.to_string()).unwrap_or_default()).to_string() == "_".to_string().to_string());
}

// -- Akcesory `Stmt::FunDecl` (rozszerzaja te z typeinfer.hcs) --------
pub fn fun_decl_name(f: &Stmt) -> String {
    match f {
        Stmt::FunDecl(name, params, ret_type, body, is_pub, type_params) => {
            return name.to_string();
        }
        _ => {
            return "".to_string();
        }
    }
}

pub fn fun_decl_params(f: &Stmt) -> Vec<Param> {
    match f {
        Stmt::FunDecl(name, params, ret_type, body, is_pub, type_params) => {
            return params.clone();
        }
        _ => {
            return vec![];
        }
    }
}

pub fn fun_decl_body(f: &Stmt) -> Vec<Stmt> {
    match f {
        Stmt::FunDecl(name, params, ret_type, body, is_pub, type_params) => {
            return body.clone();
        }
        _ => {
            return vec![];
        }
    }
}

// -- E0002: czy cialo funkcji ma GDZIEKOLWIEK `end <wartosc>` --------
// Analiza UPROSZCZONA (jak w Pythonie) - nie sledzi WSZYSTKICH sciezek
// wykonania, tylko OBECNOSC takiej instrukcji gdziekolwiek w drzewie.
pub fn has_value_return(stmts: &Vec<Stmt>) -> bool {
    let mut i: i64 = 0;
    let mut n = (stmts.len() as i64);
    while (i < n) {
        let mut s: Stmt = stmts[i as usize].clone();
        match s {
            Stmt::ReturnStmt(value) => {
                match value {
                    Some(v) => {
                        return true;
                    }
                    None => {
                    }
                }
            }
            Stmt::IfStmt(cond, body, elifs, else_body) => {
                if has_value_return(&body) {
                    return true;
                }
                let mut j: i64 = 0;
                let mut en = (elifs.len() as i64);
                while (j < en) {
                    let mut arm = elifs[j as usize].clone();
                    if has_value_return(&arm.body) {
                        return true;
                    }
                    j = (j + 1);
                }
                match else_body {
                    Some(eb) => {
                        if has_value_return(&eb) {
                            return true;
                        }
                    }
                    None => {
                    }
                }
            }
            Stmt::WhileStmt(cond, body) => {
                if has_value_return(&body) {
                    return true;
                }
            }
            Stmt::ForStmt(var, iterable, body) => {
                if has_value_return(&body) {
                    return true;
                }
            }
            Stmt::ManualBlock(body) => {
                if has_value_return(&body) {
                    return true;
                }
            }
            Stmt::MatchStmt(subject, arms) => {
                let mut k: i64 = 0;
                let mut an = (arms.len() as i64);
                while (k < an) {
                    let mut arm2 = arms[k as usize].clone();
                    if has_value_return(&arm2.body) {
                        return true;
                    }
                    k = (k + 1);
                }
            }
            _ => {
            }
        }
        i = (i + 1);
    }
    return false;
}

// -- FnChecker: sprawdza CIALO JEDNEJ funkcji/metody -------------------
// Patrz uwaga projektowa na gorze pliku co do tego, DLACZEGO to NIE
// opakowuje `TypeEnv` jako pole, tylko trzyma `sigs`/`vars` osobno.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FnChecker {
    pub sigs: Signatures,
    pub diags: Vec<Diagnostic>,
    pub used: std::collections::HashMap<String, bool>,
    pub declared_names: Vec<String>,
    pub vars: std::collections::HashMap<String, Option<TypeRef>>,
    pub ret_type: Option<TypeRef>,
    pub fn_name: String,
    pub imported_names: std::collections::HashMap<String, bool>,
    pub extra_variant_names: Vec<String>,
}

impl FnChecker {
    pub fn new(sigs: Signatures, diags: Vec<Diagnostic>, used: std::collections::HashMap<String, bool>, declared_names: Vec<String>, vars: std::collections::HashMap<String, Option<TypeRef>>, ret_type: Option<TypeRef>, fn_name: String, imported_names: std::collections::HashMap<String, bool>, extra_variant_names: Vec<String>) -> Self {
        FnChecker { sigs, diags, used, declared_names, vars, ret_type, fn_name, imported_names, extra_variant_names }
    }
}

impl FnChecker {
    pub fn mark_used(&mut self, name: &String) {
        self.used.insert((name).to_string(), true);
    }

    pub fn declare_var(&mut self, name: &String, t: Option<TypeRef>) {
        self.vars.insert((name).to_string(), t);
    }

    /// Buduje `TypeEnv` "na zawolanie" (klonujac `sigs`/`vars`) - TYLKO
    /// do przekazania do `infer_expr_type` (czysty odczyt, nigdy nie
    /// mutuje) - patrz uwaga projektowa na gorze pliku.
    pub fn env_snapshot(&self) -> TypeEnv {
        return TypeEnv::new(self.sigs.clone(), self.vars.clone());
    }

    /// E0001/W0002 - parytet z `_check_call` w typecheck.py. `name` to
    /// JUZ wyciagnieta nazwa identyfikatora calleego (patrz
    /// `expr_as_ident_name` w typeinfer.hcs).
    pub fn check_call(&mut self, name: &String, arg_count: i64) {
        if (((is_builtin_func(&name) || self.sigs.structs.contains_key(name.as_str())) || self.sigs.variant_owner.contains_key(name.as_str())) || self.imported_names.contains_key(name.as_str())) {
            return;
        }
        match self.sigs.functions.get(name.as_str()).cloned() {
            Some(f) => {
                let mut params: Vec<Param> = fun_decl_params(&f);
                if (arg_count != (params.len() as i64)) {
                    self.diags.push(Diagnostic::new("error".to_string(), "E0001".to_string(), format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "'".to_string(), name), "' oczekuje ".to_string()), ((params.len() as i64)).to_string()), " argument(ow), otrzymano ".to_string()), (arg_count).to_string()), 0, 0, 1, "".to_string()));
                }
            }
            None => {
                self.diags.push(Diagnostic::new("warning".to_string(), "W0002".to_string(), format!("{}{}", format!("{}{}", "wywolanie nieznanej funkcji '".to_string(), name), "' (jesli pochodzi z 'get <...>', to ograniczenie bootstrapu - patrz docs/ROADMAP.md)".to_string()), 0, 0, 1, "".to_string()));
            }
        }
    }

    /// Odwiedza wyrazenie - oznacza uzyte identyfikatory (`used`),
    /// sprawdza wywolania (`check_call`), sprawdza `?` (E0011).
    pub fn visit_expr(&mut self, e: &Expr) {
        let mut e2 = e.clone();
        match e2 {
            Expr::IdentExpr(name) => {
                self.mark_used(&name);
            }
            Expr::BinOp(op, left, right) => {
                self.visit_expr(&left);
                self.visit_expr(&right);
            }
            Expr::UnaryOp(op, operand) => {
                self.visit_expr(&operand);
            }
            Expr::Attr(target, name) => {
                self.visit_expr(&target);
            }
            Expr::Index(target, index) => {
                self.visit_expr(&target);
                self.visit_expr(&index);
            }
            Expr::ListLit(items) => {
                let mut i: i64 = 0;
                let mut n = (items.len() as i64);
                while (i < n) {
                    self.visit_expr(&items[i as usize]);
                    i = (i + 1);
                }
            }
            Expr::Call(callee, args) => {
                let mut ident_name: String = expr_as_ident_name(&callee);
                if (ident_name.to_string() != "".to_string().to_string()) {
                    self.mark_used(&ident_name);
                    self.check_call(&ident_name, (args.len() as i64));
                } else {
                    self.visit_expr(&callee);
                }
                let mut j: i64 = 0;
                let mut an = (args.len() as i64);
                while (j < an) {
                    self.visit_expr(&args[j as usize]);
                    j = (j + 1);
                }
            }
            Expr::Cast(target, type_ref) => {
                self.visit_expr(&target);
            }
            Expr::TryOp(target) => {
                self.visit_expr(&target);
                let mut rt = self.ret_type.clone();
                let mut ok: bool = false;
                match rt {
                    Some(rtv) => {
                        if ((rtv.name.to_string() == "Result".to_string().to_string()) || (rtv.name.to_string() == "Option".to_string().to_string())) {
                            ok = true;
                        }
                    }
                    None => {
                    }
                }
                if !(ok) {
                    let mut fname = self.fn_name.clone();
                    self.diags.push(Diagnostic::new("error".to_string(), "E0011".to_string(), format!("{}{}", format!("{}{}", "'?' uzyte w '".to_string(), fname), "', ktora nie zwraca Result<T,E> ani Option<T> - '?' propaguje Err/None do OTACZAJACEJ funkcji, wiec jej typ zwracany musi na to pozwalac".to_string()), 0, 0, 1, "".to_string()));
                }
            }
            _ => {
            }
        }
    }

    pub fn visit_stmts(&mut self, stmts: &Vec<Stmt>) {
        let mut i: i64 = 0;
        let mut n = (stmts.len() as i64);
        while (i < n) {
            self.visit_stmt(&stmts[i as usize].clone());
            i = (i + 1);
        }
    }

    /// Odwiedza jedna instrukcje - deklaruje zmienne (`vars`/
    /// `declared_names`), sprawdza E0005, rekuruje w bloki zagniezdzone.
    pub fn visit_stmt(&mut self, s: &Stmt) {
        let mut s2 = s.clone();
        match s2 {
            Stmt::LetStmt(name, type_ref, value, is_const) => {
                let mut inferred: Option<TypeRef> = None;
                match value {
                    Some(v) => {
                        self.visit_expr(&v.clone());
                        let mut env = self.env_snapshot();
                        inferred = infer_expr_type(&v, &env);
                    }
                    None => {
                    }
                }
                match type_ref {
                    Some(tref) => {
                        match inferred {
                            Some(inf) => {
                                if !(types_equal(Some(tref.clone()), Some(inf.clone()))) {
                                    self.diags.push(Diagnostic::new("error".to_string(), "E0005".to_string(), format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "zmienna '".to_string(), name), "' zadeklarowana jako ".to_string()), tref.name), ", ale przypisana wartosc ma wywnioskowany typ ".to_string()), inf.name), 0, 0, 1, "".to_string()));
                                }
                            }
                            None => {
                            }
                        }
                        self.declare_var(&name, Some(tref));
                    }
                    None => {
                        self.declare_var(&name, inferred);
                    }
                }
                self.declared_names.push(name);
            }
            Stmt::AssignStmt(target, op, value) => {
                self.visit_expr(&value);
                let mut target_ident: String = expr_as_ident_name(&target);
                if (target_ident.to_string() == "".to_string().to_string()) {
                    self.visit_expr(&target);
                }
            }
            Stmt::IfStmt(cond, body, elifs, else_body) => {
                self.visit_expr(&cond);
                self.visit_stmts(&body);
                let mut i: i64 = 0;
                let mut n = (elifs.len() as i64);
                while (i < n) {
                    let mut arm = elifs[i as usize].clone();
                    self.visit_expr(&arm.cond);
                    self.visit_stmts(&arm.body);
                    i = (i + 1);
                }
                match else_body {
                    Some(eb) => {
                        self.visit_stmts(&eb);
                    }
                    None => {
                    }
                }
            }
            Stmt::WhileStmt(cond, body) => {
                self.visit_expr(&cond);
                self.visit_stmts(&body);
            }
            Stmt::ForStmt(var, iterable, body) => {
                self.visit_expr(&iterable);
                self.declare_var(&var, None);
                self.visit_stmts(&body);
            }
            Stmt::ReturnStmt(value) => {
                match value {
                    Some(v) => {
                        self.visit_expr(&v);
                    }
                    None => {
                    }
                }
            }
            Stmt::ManualBlock(body) => {
                self.visit_stmts(&body);
            }
            Stmt::ExprStmt(expr) => {
                self.visit_expr(&expr);
            }
            Stmt::MatchStmt(subject, arms) => {
                self.visit_expr(&subject);
                let mut i2: i64 = 0;
                let mut n2 = (arms.len() as i64);
                while (i2 < n2) {
                    let mut arm = arms[i2 as usize].clone();
                    let mut j: i64 = 0;
                    let mut bn = (arm.binds.len() as i64);
                    while (j < bn) {
                        self.declare_var(&arm.binds[j as usize], None);
                        j = (j + 1);
                    }
                    self.visit_stmts(&arm.body);
                    i2 = (i2 + 1);
                }
            }
            _ => {
            }
        }
    }

}

// -- Checker: sprawdza CALY program -------------------------------------
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Checker {
    pub sigs: Signatures,
    pub diags: Vec<Diagnostic>,
    pub imported_names: std::collections::HashMap<String, bool>,
    pub extra_variant_names: Vec<String>,
}

impl Checker {
    pub fn new(sigs: Signatures, diags: Vec<Diagnostic>, imported_names: std::collections::HashMap<String, bool>, extra_variant_names: Vec<String>) -> Self {
        Checker { sigs, diags, imported_names, extra_variant_names }
    }
}

impl Checker {
    /// E0003 - parytet z `_check_get`.
    pub fn check_get(&mut self, source: &String, name: &String) {
        if !(is_known_get_source(&source)) {
            self.diags.push(Diagnostic::new("error".to_string(), "E0003".to_string(), format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "nieznane zrodlo '".to_string(), source), "' w 'get <".to_string()), source), ":".to_string()), name), ">' (dozwolone: core, crates, pypi, selfhost, std)".to_string()), 0, 0, 1, "".to_string()));
        }
    }

    /// Sprawdza jedna funkcje/metode - buduje `FnChecker`, odwiedza
    /// cale cialo, robi E0002/W0001, i doklada wynik do `self.diags`.
    pub fn check_fun(&mut self, fn_stmt: &Stmt, self_type: Option<TypeRef>) {
        let mut owned_fn = fn_stmt.clone();
        let mut fn_name: String = fun_decl_name(&owned_fn.clone());
        let mut params: Vec<Param> = fun_decl_params(&owned_fn.clone());
        let mut ret_type: Option<TypeRef> = fun_decl_ret_type(&owned_fn.clone());
        let mut body: Vec<Stmt> = fun_decl_body(&owned_fn);
        let mut checker: FnChecker = FnChecker::new(self.sigs.clone(), vec![], std::collections::HashMap::new(), vec![], std::collections::HashMap::new(), ret_type, (fn_name).to_string(), self.imported_names.clone(), self.extra_variant_names.clone());
        let mut i: i64 = 0;
        let mut n = (params.len() as i64);
        while (i < n) {
            let mut p = params[i as usize].clone();
            if (p.name.to_string() == "self".to_string().to_string()) {
                checker.declare_var(&"self".to_string(), self_type.clone());
            } else {
                checker.declare_var(&p.name, p.type_ref);
            }
            i = (i + 1);
        }
        checker.visit_stmts(&body.clone());
        let mut ret_type_for_e0002 = checker.ret_type.clone();
        match ret_type_for_e0002 {
            Some(rt) => {
                if !(has_value_return(&body)) {
                    let mut fname_for_e0002 = checker.fn_name.clone();
                    checker.diags.push(Diagnostic::new("error".to_string(), "E0002".to_string(), format!("{}{}", format!("{}{}", format!("{}{}", format!("{}{}", "funkcja '".to_string(), fname_for_e0002), "' deklaruje typ zwracany ".to_string()), rt.name), ", ale nigdzie nie ma 'end <wartosc>'".to_string()), 0, 0, 1, "".to_string()));
                }
            }
            None => {
            }
        }
        let mut j: i64 = 0;
        let mut dn = (checker.declared_names.len() as i64);
        while (j < dn) {
            let mut dname: String = checker.declared_names[j as usize].clone();
            if (!(checker.used.contains_key(dname.as_str())) && !(str_starts_with_underscore(&dname))) {
                checker.diags.push(Diagnostic::new("warning".to_string(), "W0001".to_string(), format!("{}{}", format!("{}{}", "zmienna '".to_string(), dname), "' jest zadeklarowana, ale nigdy nie uzyta".to_string()), 0, 0, 1, "".to_string()));
            }
            j = (j + 1);
        }
        self.diags.extend((checker.diags).clone());
    }

    /// Sprawdza CALY program - parytet z `Checker.check()`.
    pub fn check(&self) {
        return;
    }

}

// Buduje `imported_names` (suma `details` ze WSZYSTKICH
// `GetImportStmt` w programie) - parytet z petla w
// `Checker.__init__`.
pub fn collect_imported_names(prog: &Program) -> std::collections::HashMap<String, bool> {
    let mut out: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut i: i64 = 0;
    let mut n = (prog.body.len() as i64);
    while (i < n) {
        let mut s: Stmt = prog.body[i as usize].clone();
        match s {
            Stmt::GetImportStmt(source, name, version, details) => {
                let mut j: i64 = 0;
                let mut dn = (details.len() as i64);
                while (j < dn) {
                    out.insert(details[j as usize].clone(), true);
                    j = (j + 1);
                }
            }
            _ => {
            }
        }
        i = (i + 1);
    }
    return out;
}

// Punkt wejscia - parytet z `check_program(program, extra_variant_names)`
// w typecheck.py. `extra_variant_names` to warianty enumow
// ZAIMPORTOWANYCH z innych plikow (`get <selfhost:ast_nodes> import
// <Expr>`) - bez tego konstruktor wariantu z importowanego enuma
// dawalby spurious W0002, bo `Signatures` widzi TYLKO BIEZACY plik
// (parytet z komentarzem przy `extra_variant_names` w typecheck.py).
pub fn check_program(prog: &Program, extra_variant_names: &Vec<String>) -> Vec<Diagnostic> {
    let mut sigs = collect_signatures(&prog.clone());
    let mut imported_names: std::collections::HashMap<String, bool> = collect_imported_names(&prog.clone());
    let mut checker: Checker = Checker::new((sigs).clone(), vec![], (imported_names).clone(), extra_variant_names.clone());
    let mut i: i64 = 0;
    let mut n = (prog.body.len() as i64);
    while (i < n) {
        let mut s: Stmt = prog.body[i as usize].clone();
        match s {
            Stmt::GetImportStmt(source, name, version, details) => {
                checker.check_get(&source, &name);
            }
            Stmt::FunDecl(fname, fparams, fret, fbody, fis_pub, ftype_params) => {
                checker.check_fun(&prog.body[i as usize].clone(), None);
            }
            Stmt::ImplDecl(struct_name, methods, type_params) => {
                let mut j: i64 = 0;
                let mut mn = (methods.len() as i64);
                while (j < mn) {
                    checker.check_fun(&methods[j as usize], Some(TypeRef::new(struct_name.clone(), None, None)));
                    j = (j + 1);
                }
            }
            _ => {
            }
        }
        i = (i + 1);
    }
    return checker.diags.clone();
}

// Demonstracyjne uzycie - sprawdza reczne AST dla `fun bad() -> Int [
// let x = 1 ]` (brak `end` -> E0002, `x` nieuzyte -> W0001) i
// `fun ok() -> Int [ end 1 + 2 ]` (czyste).
pub fn main() {
    let mut bad_body: Vec<Stmt> = vec![Stmt::LetStmt("x".to_string(), None, Some(Expr::NumberLit("1".to_string())), false)];
    let mut bad_fn: Stmt = Stmt::FunDecl("bad".to_string(), vec![], Some(TypeRef::new("Int".to_string(), None, None)), (bad_body).clone(), false, vec![]);
    let mut ok_body: Vec<Stmt> = vec![Stmt::ReturnStmt(Some(Expr::BinOp("+".to_string(), Box::new(Expr::NumberLit("1".to_string())), Box::new(Expr::NumberLit("2".to_string())))))];
    let mut ok_fn: Stmt = Stmt::FunDecl("ok".to_string(), vec![], Some(TypeRef::new("Int".to_string(), None, None)), (ok_body).clone(), false, vec![]);
    let mut prog = Program::new(vec![bad_fn, ok_fn]);
    let mut diags: Vec<Diagnostic> = check_program(&prog, &vec![]);
    println!("{} {}", "liczba diagnostyk:".to_string(), (diags.len() as i64));
}

// ## Ograniczenia tej wersji (patrz docs/ROADMAP.md)
// 
// - Wszystkie `Diagnostic` maja `line=0, col=0, length=1` - brak
// pola `line: Int` na wezlach AST (patrz "Ograniczenia" w
// ast_nodes.hcs) - komunikaty sa poprawne tresciowo, ale bez
// lokalizacji w zrodle (parser.hcs rowniez tego dzis nie sledzi).
// - `Checker.check()` (bez argumentow, odpowiadajac
// `Checker.check(self) -> list[Diagnostic]` w Pythonie) jest tu
// PUSTE - prawdziwa logika zyje w wolnej funkcji `check_program`
// zamiast metody (matched Python semantycznie identycznie, ale
// `check_program` MUSI byc funkcja wolna, bo POTRZEBUJE zbudowac
// `Signatures`/`imported_names` PRZED skonstruowaniem `Checker`,
// co w Pythonie dzieje sie w `__init__` - HackerScript nie ma
// konstruktorow z logika, tylko plaskie pozycyjne wywolanie typu).
// - `used`/`imported_names` to `Dict<Str, Bool>` (nie prawdziwy Set -
// ten bootstrap go nie ma).
// - `declared_names` to `List<Str>` (nie `Dict<Str, LetStmt>` jak w
// Pythonie) - bez iteracji po Dict (`.items()`) nie dalo by sie
// tego przejrzec, a i tak nie mamy `line` do pokazania z LetStmt.
// - NIEPRZETESTOWANE na prawdziwym wejsciu w tym srodowisku (brak
// rustc) - zweryfikowane strukturalnie przez `hackerc check`/
// `build` i BARDZO dokladna inspekcje wygenerowanego Rusta (patrz
// docs/ROADMAP.md - uwaga projektowa o `self.pole` + `match`).

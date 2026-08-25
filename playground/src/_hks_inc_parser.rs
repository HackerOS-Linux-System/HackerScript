#![allow(non_snake_case, unused_mut, dead_code)]

//! bootstrap/hackerc-self/parser.hcs
//! 
//! Krok 3/N przepisania calego hackerc na HackerScript (patrz
//! docs/ROADMAP.md, "W TOKU"). PELNY parytet z hackerc/hackerc/parser.py
//! (618 linii) - JEDEN plik z CALA gramatyka (instrukcje najwyzszego
//! poziomu, blokowe, i precedence-climbing dla wyrazen), tak jak w
//! oryginale.
//! 
//! **TEN PLIK ZASTĘPUJE** poprzednie `expr_parser.hcs`/`stmt_parser.hcs`/
//! `decl_parser.hcs` (usuniete w tej sesji - ich `impl Parser`
//! rozbite na trzy pliki bylo rozsadnym krokiem PRZYROSTOWYM, ale
//! rozjezdzalo sie z prawdziwym parser.py, gdzie WSZYSTKO jest jedna
//! klasa `Parser`). Uzywa `Stmt`/`Expr`/`TypeRef`/`Param`/`ElifArm`/
//! `EnumVariant`/`MatchArm`/`Program` z NOWEGO, skonsolidowanego
//! `ast_nodes.hcs` (patrz tamten plik).
//! 
//! Kompilowany dzis przez STAGE0 (Pythonowy hackerc) - patrz
//! bootstrap/README.md.
//! 
//! Nazwa metody `match` (Pythonowe `Parser.match(...)`) KOLIDUJE z
//! `match` jako slowem kluczowym HackerScript (uzywanym przez
//! `Stmt::MatchStmt`) - PRZEMIANOWANA na `match_tok` w calym tym
//! pliku. To JEDYNA zmiana nazwy metody wobec parser.py.
use crate::_hks_inc_lexer::*;
use crate::_hks_inc_ast_nodes::*;
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parser {
    pub toks: Vec<Token>,
    pub pos: i64,
}

impl Parser {
    pub fn new(toks: Vec<Token>, pos: i64) -> Self {
        Parser { toks, pos }
    }
}

impl Parser {
    pub fn cur(&self) -> Token {
        return self.toks[self.pos as usize].clone();
    }

    pub fn at_end(&self) -> bool {
        return (self.cur().kind == TokKind::Eof);
    }

    pub fn advance(&mut self) -> Token {
        let mut t = self.toks[self.pos as usize].clone();
        if (self.pos < ((self.toks.len() as i64) - 1)) {
            self.pos = (self.pos + 1);
        }
        return t;
    }

    /// `value: Option<Str>` odpowiada Pythonowemu `value: str | None =
    /// None` - `none()` = "dopasuj tylko po `kind`", `some(x)` = "i
    /// po wartosci". `kind.clone()` PONIZEJ jest KONIECZNE, nie
    /// kosmetyczne: `kind: TokKind` to ZNANY enum, wiec ten codegen
    /// ZAWSZE przekazuje go jako `&TokKind` (patrz `_is_refable` w
    /// codegen.py - kazdy enum/struct/Str/List jest "refable"), a
    /// porownanie `t.kind != kind` bylo by wtedy `TokKind != &TokKind`
    /// - PRAWDZIWA niezgodnosc typow w Rust (derive'owany `PartialEq`
    /// daje TYLKO `PartialEq<Self>`, nie `PartialEq<&Self>`), znaleziona
    /// i naprawiona w TEJ sesji. `.clone()` na referencji auto-derefuje
    /// sie (w przeciwienstwie do operatorow `==`/`!=`, wywolania metod
    /// W RUST auto-derefuja odbiornik) i zwraca WLASCIWY, owned
    /// `TokKind`.
    pub fn check(&self, kind: &TokKind, value: Option<String>) -> bool {
        let mut t = self.cur();
        if (t.kind != kind.clone()) {
            return false;
        }
        match value {
            Some(v) => {
                return (t.value.to_string() == v.to_string());
            }
            None => {
                return true;
            }
        }
    }

    pub fn match_tok(&mut self, kind: &TokKind, value: Option<String>) -> Option<Token> {
        if self.check(&kind, value) {
            return Some(self.advance());
        }
        return None;
    }

    /// Bez prawdziwego `ParseError`/wyjatkow w tej wersji bootstrapu
    /// (patrz "Ograniczenia") - gdy oczekiwany token nie wystepuje,
    /// `log`-uje ostrzezenie i i tak KONSUMUJE biezacy token (best-effort,
    /// nie zatrzymuje parsowania) zamiast rzucac `ParseError` jak w
    /// parser.py.
    pub fn expect(&mut self, kind: &TokKind, value: Option<String>) -> Token {
        if !(self.check(&kind, value)) {
            println!("{} {}", "[hackerc-self] parser: nieoczekiwany token".to_string(), self.cur().value);
        }
        return self.advance();
    }

    pub fn skip_newlines(&mut self) {
        while (self.check(&TokKind::Newline, None) || self.check(&TokKind::LineComment, None)) {
            self.advance();
        }
    }

    /// Jak `skip_newlines`, ale ZWRACA tekst napotkanych komentarzy `!`
    /// (w kolejnosci wystapienia) zamiast je odrzucac - wywolujacy
    /// doczepia je do nastepnej instrukcji (patrz "Ograniczenia":
    /// `_leading_comments`/`hackerc fmt` nie sa jeszcze uzyte tutaj,
    /// sam mechanizm zbierania jest juz gotowy).
    pub fn skip_newlines_collect_comments(&mut self) -> Vec<String> {
        let mut comments: Vec<String> = vec![];
        while true {
            if self.check(&TokKind::Newline, None) {
                self.advance();
            } else if self.check(&TokKind::LineComment, None) {
                comments.push((self.advance().value).to_string());
            } else {
                break;
            }
        }
        return comments;
    }

    /// -- program --------------------------------------------------------
    pub fn parse_program(&mut self) -> Program {
        let mut body: Vec<Stmt> = vec![];
        self.skip_newlines_collect_comments();
        while !(self.at_end()) {
            let mut stmt: Stmt = self.parse_statement();
            body.push((stmt).clone());
            self.skip_newlines_collect_comments();
        }
        return Program::new((body).clone());
    }

    /// -- instrukcje -------------------------------------------------------
    pub fn parse_block(&mut self) -> Vec<Stmt> {
        self.expect(&TokKind::Open, None);
        self.skip_newlines_collect_comments();
        let mut stmts: Vec<Stmt> = vec![];
        while !(self.check(&TokKind::Close, None)) {
            let mut stmt: Stmt = self.parse_statement();
            stmts.push((stmt).clone());
            self.skip_newlines_collect_comments();
        }
        self.expect(&TokKind::Close, None);
        return stmts;
    }

    pub fn parse_statement(&mut self) -> Stmt {
        let mut t = self.cur();
        if (t.kind == TokKind::DocComment) {
            self.advance();
            return Stmt::ExprStmt(Expr::StringLit((t.value).to_string(), true));
        }
        if (t.kind == TokKind::Keyword) {
            let mut kw: String = (t.value).to_string();
            if (kw.to_string() == "using".to_string().to_string()) {
                return self.parse_using();
            }
            if (kw.to_string() == "get".to_string().to_string()) {
                return self.parse_get_import();
            }
            if (kw.to_string() == "include".to_string().to_string()) {
                return self.parse_include();
            }
            if (kw.to_string() == "let".to_string().to_string()) {
                return self.parse_let(false);
            }
            if (kw.to_string() == "const".to_string().to_string()) {
                return self.parse_let(true);
            }
            if (kw.to_string() == "pub".to_string().to_string()) {
                self.advance();
                let mut inner: Stmt = self.parse_statement();
                match inner {
                    Stmt::FunDecl(name, params, ret_type, body, is_pub, type_params) => {
                        return Stmt::FunDecl(name, params, ret_type, body, true, type_params);
                    }
                    _ => {
                        return inner;
                    }
                }
            }
            if (kw.to_string() == "extern".to_string().to_string()) {
                return self.parse_extern();
            }
            if (kw.to_string() == "fun".to_string().to_string()) {
                return self.parse_fun();
            }
            if (kw.to_string() == "if".to_string().to_string()) {
                return self.parse_if();
            }
            if (kw.to_string() == "while".to_string().to_string()) {
                return self.parse_while();
            }
            if (kw.to_string() == "for".to_string().to_string()) {
                return self.parse_for();
            }
            if (kw.to_string() == "end".to_string().to_string()) {
                self.advance();
                let mut value: Option<Expr> = None;
                if (!(self.check(&TokKind::Newline, None)) && !(self.check(&TokKind::Close, None))) {
                    value = Some(self.parse_expr());
                }
                return Stmt::ReturnStmt(value);
            }
            if (kw.to_string() == "break".to_string().to_string()) {
                self.advance();
                return Stmt::BreakStmt;
            }
            if (kw.to_string() == "continue".to_string().to_string()) {
                self.advance();
                return Stmt::ContinueStmt;
            }
            if (kw.to_string() == "direct".to_string().to_string()) {
                return self.parse_direct();
            }
            if (kw.to_string() == "manual".to_string().to_string()) {
                return self.parse_manual();
            }
            if (kw.to_string() == "gc".to_string().to_string()) {
                return self.parse_gc_pragma();
            }
            if (kw.to_string() == "struct".to_string().to_string()) {
                return self.parse_struct();
            }
            if (kw.to_string() == "enum".to_string().to_string()) {
                return self.parse_enum();
            }
            if (kw.to_string() == "impl".to_string().to_string()) {
                return self.parse_impl();
            }
            if (kw.to_string() == "match".to_string().to_string()) {
                return self.parse_match();
            }
        }
        let mut expr: Expr = self.parse_expr();
        if ((self.cur().kind == TokKind::Op) && (((((self.cur().value.to_string() == "=".to_string().to_string()) || (self.cur().value.to_string() == "+=".to_string().to_string())) || (self.cur().value.to_string() == "-=".to_string().to_string())) || (self.cur().value.to_string() == "*=".to_string().to_string())) || (self.cur().value.to_string() == "/=".to_string().to_string()))) {
            let mut op: String = (self.advance().value).to_string();
            let mut value: Expr = self.parse_expr();
            return Stmt::AssignStmt((expr).clone(), (op).to_string(), (value).clone());
        }
        return Stmt::ExprStmt((expr).clone());
    }

    pub fn parse_using(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("using".to_string()));
        self.expect(&TokKind::LAngle, None);
        let mut version_parts: String = "".to_string();
        while !(self.check(&TokKind::RAngle, None)) {
            version_parts = format!("{}{}", version_parts, self.advance().value);
        }
        self.expect(&TokKind::RAngle, None);
        return Stmt::UsingStmt((version_parts).to_string());
    }

    /// Czyta tekst az do napotkania '::', '>' lub ':' (na poziomie top) -
    /// parytet z `_read_angle_segment` w parser.py.
    pub fn read_angle_segment(&mut self) -> String {
        let mut parts: String = "".to_string();
        while !(((self.check(&TokKind::DColon, None) || self.check(&TokKind::RAngle, None)) || self.check(&TokKind::Colon, None))) {
            parts = format!("{}{}", parts, self.advance().value);
        }
        return parts.to_string();
    }

    pub fn parse_get_import(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("get".to_string()));
        self.expect(&TokKind::LAngle, None);
        let mut source: String = self.read_angle_segment();
        if (source.to_string() == "selfhost".to_string().to_string()) {
            /// `get <selfhost:...>` jest zablokowane (na wyrazne
            /// zyczenie uzytkownika) - uzyj `include <plik>` zamiast
            /// tego. `parser.hcs` (w odroznieniu od `parser.py`) NIE MA
            /// wyjatkow/`Result` dla bledow parsera (patrz "Ograniczenia"
            /// - best-effort, `log()` i kontynuacja) - to jedyny sposob
            /// zasygnalizowania tego bledu W TYM PLIKU. `parser.py` (uzywany
            /// dzis jako stage0) DAJE prawdziwy blad skladni (ParseError).
            println!("{}", "[hackerc-self] parser: 'get <selfhost:...>' jest zablokowane - uzyj 'include <plik>' zamiast tego".to_string());
        }
        self.expect(&TokKind::Colon, None);
        let mut name: String = self.read_angle_segment();
        let mut version: Option<String> = None;
        if self.check(&TokKind::DColon, None) {
            self.advance();
            version = Some(self.read_angle_segment());
        }
        self.expect(&TokKind::RAngle, None);
        let mut details: Vec<String> = vec![];
        if self.check(&TokKind::Keyword, Some("import".to_string())) {
            self.advance();
            self.expect(&TokKind::LAngle, None);
            details.push(self.read_angle_segment());
            while self.check(&TokKind::DColon, None) {
                self.advance();
                details.push(self.read_angle_segment());
            }
            self.expect(&TokKind::RAngle, None);
        }
        return Stmt::GetImportStmt((source).to_string(), (name).to_string(), version, (details).clone());
    }

    /// `include <sciezka>` - parytet z `parse_include()` (parser.py) -
    /// prostsze niz `get`: jeden segment, bez `source:`/`::wersja`/
    /// `import <details>`.
    pub fn parse_include(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("include".to_string()));
        self.expect(&TokKind::LAngle, None);
        let mut path: String = self.read_angle_segment();
        self.expect(&TokKind::RAngle, None);
        return Stmt::IncludeStmt((path).to_string());
    }

    pub fn parse_type(&mut self) -> TypeRef {
        let mut name_tok = self.advance();
        let mut generic: Option<TypeRef> = None;
        let mut generic2: Option<TypeRef> = None;
        if self.check(&TokKind::LAngle, None) {
            self.advance();
            generic = Some(self.parse_type());
            if self.check(&TokKind::Comma, None) {
                self.advance();
                generic2 = Some(self.parse_type());
            }
            self.expect(&TokKind::RAngle, None);
        }
        return TypeRef::new((name_tok.value).to_string(), generic, generic2);
    }

    pub fn parse_let(&mut self, is_const: bool) -> Stmt {
        self.advance();
        let mut name: String = (self.expect(&TokKind::Ident, None).value).to_string();
        let mut type_ref: Option<TypeRef> = None;
        if self.check(&TokKind::Colon, None) {
            self.advance();
            type_ref = Some(self.parse_type());
        }
        let mut value: Option<Expr> = None;
        if self.check(&TokKind::Op, Some("=".to_string())) {
            self.advance();
            value = Some(self.parse_expr());
        }
        return Stmt::LetStmt((name).to_string(), type_ref, value, is_const);
    }

    pub fn parse_params(&mut self) -> Vec<Param> {
        let mut params: Vec<Param> = vec![];
        self.expect(&TokKind::LParen, None);
        while !(self.check(&TokKind::RParen, None)) {
            if self.check(&TokKind::Keyword, Some("self".to_string())) {
                self.advance();
                params.push(Param::new("self".to_string(), None, None));
                if !(self.check(&TokKind::Comma, None)) {
                    break;
                }
                self.advance();
            } else {
                let mut pname: String = (self.expect(&TokKind::Ident, None).value).to_string();
                let mut ptype: Option<TypeRef> = None;
                if self.check(&TokKind::Colon, None) {
                    self.advance();
                    ptype = Some(self.parse_type());
                }
                let mut pdefault: Option<Expr> = None;
                if self.check(&TokKind::Op, Some("=".to_string())) {
                    self.advance();
                    pdefault = Some(self.parse_expr());
                }
                params.push(Param::new((pname).to_string(), ptype, pdefault));
                if !(self.check(&TokKind::Comma, None)) {
                    break;
                }
                self.advance();
            }
        }
        self.expect(&TokKind::RParen, None);
        return params;
    }

    /// Opcjonalna lista parametrow generycznych `<T, U>` po nazwie
    /// struct/fun/enum/impl.
    pub fn parse_type_params(&mut self) -> Vec<String> {
        let mut params: Vec<String> = vec![];
        if !(self.check(&TokKind::LAngle, None)) {
            return params;
        }
        self.advance();
        while !(self.check(&TokKind::RAngle, None)) {
            params.push((self.expect(&TokKind::Ident, None).value).to_string());
            if !(self.check(&TokKind::Comma, None)) {
                break;
            }
            self.advance();
        }
        self.expect(&TokKind::RAngle, None);
        return params;
    }

    pub fn parse_fun(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("fun".to_string()));
        let mut name: String = (self.expect(&TokKind::Ident, None).value).to_string();
        let mut type_params: Vec<String> = self.parse_type_params();
        let mut params: Vec<Param> = self.parse_params();
        let mut ret_type: Option<TypeRef> = None;
        if self.check(&TokKind::Op, Some("->".to_string())) {
            self.advance();
            ret_type = Some(self.parse_type());
        }
        let mut body: Vec<Stmt> = self.parse_block();
        return Stmt::FunDecl((name).to_string(), (params).clone(), ret_type, (body).clone(), false, (type_params).clone());
    }

    pub fn parse_if(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("if".to_string()));
        let mut cond: Expr = self.parse_expr();
        let mut body: Vec<Stmt> = self.parse_block();
        let mut elifs: Vec<ElifArm> = vec![];
        let mut else_body: Option<Vec<Stmt>> = None;
        self.skip_newlines_soft();
        while self.check(&TokKind::Keyword, Some("elif".to_string())) {
            self.advance();
            let mut econd: Expr = self.parse_expr();
            let mut ebody: Vec<Stmt> = self.parse_block();
            elifs.push(ElifArm::new((econd).clone(), (ebody).clone()));
            self.skip_newlines_soft();
        }
        if self.check(&TokKind::Keyword, Some("else".to_string())) {
            self.advance();
            else_body = Some(self.parse_block());
        }
        return Stmt::IfStmt((cond).clone(), (body).clone(), (elifs).clone(), else_body);
    }

    /// Podglada czy po newline'ach jest `elif`/`else` (zeby obsluzyc
    /// `else` na nowej linii po `]`) - cofa sie jesli nie.
    pub fn skip_newlines_soft(&mut self) {
        let mut save: i64 = self.pos;
        while (self.check(&TokKind::Newline, None) || self.check(&TokKind::LineComment, None)) {
            self.advance();
        }
        if !((self.check(&TokKind::Keyword, Some("elif".to_string())) || self.check(&TokKind::Keyword, Some("else".to_string())))) {
            self.pos = save;
        }
    }

    pub fn parse_while(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("while".to_string()));
        let mut cond: Expr = self.parse_expr();
        let mut body: Vec<Stmt> = self.parse_block();
        return Stmt::WhileStmt((cond).clone(), (body).clone());
    }

    pub fn parse_for(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("for".to_string()));
        let mut var: String = (self.expect(&TokKind::Ident, None).value).to_string();
        self.expect(&TokKind::Keyword, Some("in".to_string()));
        let mut iterable: Expr = self.parse_expr();
        let mut body: Vec<Stmt> = self.parse_block();
        return Stmt::ForStmt((var).to_string(), (iterable).clone(), (body).clone());
    }

    pub fn parse_extern(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("extern".to_string()));
        let mut lib_tok = self.expect(&TokKind::StrLit, None);
        self.expect(&TokKind::Keyword, Some("fun".to_string()));
        let mut name: String = (self.expect(&TokKind::Ident, None).value).to_string();
        let mut params: Vec<Param> = self.parse_params();
        let mut ret_type: Option<TypeRef> = None;
        if self.check(&TokKind::Op, Some("->".to_string())) {
            self.advance();
            ret_type = Some(self.parse_type());
        }
        return Stmt::ExternFunDecl((lib_tok.value).to_string(), (name).to_string(), (params).clone(), ret_type);
    }

    /// `__direct__(0)` jest wyjatkowe: w prawdziwym hackerc surowy tekst
    /// pomiedzy `[`/`]` jest wyciagany PRZED tokenizacja (patrz
    /// transpiler.py::`_extract_direct_blocks`) - Parser normalnie
    /// NIGDY nie widzi tokenow `direct` (parytet: `parse_direct` w
    /// parser.py TEZ tylko rzuca `ParseError` mowiacy "to powinno byc
    /// obsluzone na etapie preprocessing"). Ta metoda jest wiec
    /// siecia bezpieczenstwa, nie realna implementacja - patrz
    /// "Ograniczenia".
    pub fn parse_direct(&mut self) -> Stmt {
        self.advance();
        println!("{}", "[hackerc-self] parser: 'direct' powinno byc wyciagniete na etapie preprocessing, nie parsowane".to_string());
        return Stmt::DirectBlock(vec![]);
    }

    pub fn parse_manual(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("manual".to_string()));
        let mut body: Vec<Stmt> = self.parse_block();
        return Stmt::ManualBlock((body).clone());
    }

    pub fn parse_gc_pragma(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("gc".to_string()));
        self.expect(&TokKind::Colon, None);
        self.advance();
        self.expect(&TokKind::DColon, None);
        let mut mode: String = (self.advance().value).to_string();
        return Stmt::GcPragma((mode).to_string());
    }

    pub fn parse_struct(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("struct".to_string()));
        let mut name: String = (self.expect(&TokKind::Ident, None).value).to_string();
        let mut type_params: Vec<String> = self.parse_type_params();
        self.expect(&TokKind::Open, None);
        self.skip_newlines();
        let mut fields: Vec<Param> = vec![];
        while !(self.check(&TokKind::Close, None)) {
            let mut fname: String = (self.expect(&TokKind::Ident, None).value).to_string();
            self.expect(&TokKind::Colon, None);
            let mut ftype = self.parse_type();
            fields.push(Param::new((fname).to_string(), Some((ftype).clone()), None));
            if self.check(&TokKind::Comma, None) {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(&TokKind::Close, None);
        return Stmt::StructDecl((name).to_string(), (fields).clone(), (type_params).clone());
    }

    pub fn parse_enum(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("enum".to_string()));
        let mut name: String = (self.expect(&TokKind::Ident, None).value).to_string();
        let mut type_params: Vec<String> = self.parse_type_params();
        self.expect(&TokKind::Open, None);
        self.skip_newlines();
        let mut variants: Vec<EnumVariant> = vec![];
        while !(self.check(&TokKind::Close, None)) {
            let mut vname: String = (self.expect(&TokKind::Ident, None).value).to_string();
            let mut vfields: Vec<TypeRef> = vec![];
            if self.check(&TokKind::LParen, None) {
                self.advance();
                while !(self.check(&TokKind::RParen, None)) {
                    vfields.push(self.parse_type());
                    if !(self.check(&TokKind::Comma, None)) {
                        break;
                    }
                    self.advance();
                }
                self.expect(&TokKind::RParen, None);
            }
            variants.push(EnumVariant::new((vname).to_string(), (vfields).clone()));
            if self.check(&TokKind::Comma, None) {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(&TokKind::Close, None);
        return Stmt::EnumDecl((name).to_string(), (variants).clone(), (type_params).clone());
    }

    /// `impl Nazwa [ fun metoda(self, ...) -> Typ [ ... ] ... ]`.
    /// Komentarze `!!` przed metoda sa dziś ZBIERANE (`pending_doc`,
    /// parytet z `pending_doc` w parser.py), ale NIE MA gdzie ich
    /// doczepic (`FunDecl` w tym AST nie ma pola
    /// `_leading_doc_comments`, patrz "Ograniczenia" w ast_nodes.hcs) -
    /// sa wiec dzis odrzucane po zebraniu, TYLKO zeby nie wywolac
    /// `expect(Keyword, "fun")` na tokenie `DocComment` i nie
    /// wyprodukowac spurious ostrzezenia.
    pub fn parse_impl(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("impl".to_string()));
        let mut struct_name: String = (self.expect(&TokKind::Ident, None).value).to_string();
        let mut type_params: Vec<String> = self.parse_type_params();
        self.expect(&TokKind::Open, None);
        self.skip_newlines();
        let mut methods: Vec<Stmt> = vec![];
        while !(self.check(&TokKind::Close, None)) {
            if self.check(&TokKind::DocComment, None) {
                self.advance();
                self.skip_newlines();
            } else {
                let mut m: Stmt = self.parse_fun();
                methods.push((m).clone());
                self.skip_newlines();
            }
        }
        self.expect(&TokKind::Close, None);
        return Stmt::ImplDecl((struct_name).to_string(), (methods).clone(), (type_params).clone());
    }

    /// `match wyrazenie [ Wariant(bind, ...) -> [ ... ] ... _ -> [...] ]`.
    pub fn parse_match(&mut self) -> Stmt {
        self.expect(&TokKind::Keyword, Some("match".to_string()));
        let mut subject: Expr = self.parse_expr();
        self.expect(&TokKind::Open, None);
        self.skip_newlines();
        let mut arms: Vec<MatchArm> = vec![];
        while !(self.check(&TokKind::Close, None)) {
            let mut vname: String = (self.expect(&TokKind::Ident, None).value).to_string();
            let mut binds: Vec<String> = vec![];
            if self.check(&TokKind::LParen, None) {
                self.advance();
                while !(self.check(&TokKind::RParen, None)) {
                    binds.push((self.expect(&TokKind::Ident, None).value).to_string());
                    if !(self.check(&TokKind::Comma, None)) {
                        break;
                    }
                    self.advance();
                }
                self.expect(&TokKind::RParen, None);
            }
            self.expect(&TokKind::Op, Some("->".to_string()));
            let mut body: Vec<Stmt> = self.parse_block();
            arms.push(MatchArm::new((vname).to_string(), (binds).clone(), (body).clone()));
            self.skip_newlines();
        }
        self.expect(&TokKind::Close, None);
        return Stmt::MatchStmt((subject).clone(), (arms).clone());
    }

    /// -- wyrazenia (precedence climbing) --------------------------------
    pub fn parse_expr(&mut self) -> Expr {
        return self.parse_or();
    }

    pub fn parse_or(&mut self) -> Expr {
        let mut left: Expr = self.parse_and();
        while (self.check(&TokKind::Keyword, Some("or".to_string())) || self.check(&TokKind::Op, Some("||".to_string()))) {
            self.advance();
            let mut right: Expr = self.parse_and();
            left = Expr::BinOp("or".to_string(), Box::new((left).clone()), Box::new((right).clone()));
        }
        return left;
    }

    pub fn parse_and(&mut self) -> Expr {
        let mut left: Expr = self.parse_not();
        while (self.check(&TokKind::Keyword, Some("and".to_string())) || self.check(&TokKind::Op, Some("&&".to_string()))) {
            self.advance();
            let mut right: Expr = self.parse_not();
            left = Expr::BinOp("and".to_string(), Box::new((left).clone()), Box::new((right).clone()));
        }
        return left;
    }

    pub fn parse_not(&mut self) -> Expr {
        if (self.check(&TokKind::Keyword, Some("not".to_string())) || self.check(&TokKind::Op, Some("!".to_string()))) {
            self.advance();
            let mut operand: Expr = self.parse_not();
            return Expr::UnaryOp("not".to_string(), Box::new((operand).clone()));
        }
        return self.parse_comparison();
    }

    pub fn parse_comparison(&mut self) -> Expr {
        let mut left: Expr = self.parse_additive();
        while (((self.check(&TokKind::Op, None) && ((((self.cur().value.to_string() == "==".to_string().to_string()) || (self.cur().value.to_string() == "!=".to_string().to_string())) || (self.cur().value.to_string() == "<=".to_string().to_string())) || (self.cur().value.to_string() == ">=".to_string().to_string()))) || self.check(&TokKind::LAngle, None)) || self.check(&TokKind::RAngle, None)) {
            let mut op: String = (self.advance().value).to_string();
            let mut right: Expr = self.parse_additive();
            left = Expr::BinOp((op).to_string(), Box::new((left).clone()), Box::new((right).clone()));
        }
        return left;
    }

    pub fn parse_additive(&mut self) -> Expr {
        let mut left: Expr = self.parse_mult();
        while (self.check(&TokKind::Op, None) && ((self.cur().value.to_string() == "+".to_string().to_string()) || (self.cur().value.to_string() == "-".to_string().to_string()))) {
            let mut op: String = (self.advance().value).to_string();
            let mut right: Expr = self.parse_mult();
            left = Expr::BinOp((op).to_string(), Box::new((left).clone()), Box::new((right).clone()));
        }
        return left;
    }

    pub fn parse_mult(&mut self) -> Expr {
        let mut left: Expr = self.parse_unary();
        while (self.check(&TokKind::Op, None) && (((self.cur().value.to_string() == "*".to_string().to_string()) || (self.cur().value.to_string() == "/".to_string().to_string())) || (self.cur().value.to_string() == "%".to_string().to_string()))) {
            let mut op: String = (self.advance().value).to_string();
            let mut right: Expr = self.parse_unary();
            left = Expr::BinOp((op).to_string(), Box::new((left).clone()), Box::new((right).clone()));
        }
        return left;
    }

    pub fn parse_unary(&mut self) -> Expr {
        if (self.check(&TokKind::Op, None) && ((self.cur().value.to_string() == "-".to_string().to_string()) || (self.cur().value.to_string() == "+".to_string().to_string()))) {
            let mut op: String = (self.advance().value).to_string();
            let mut operand: Expr = self.parse_unary();
            return Expr::UnaryOp((op).to_string(), Box::new((operand).clone()));
        }
        return self.parse_postfix();
    }

    /// Lancuch postfiksowy: wywolania `f(...)`, `.pole`, `as Typ`, `?`,
    /// `[indeks]` - w DOWOLNEJ kolejnosci i liczbie, np. `f().pole?[0]`.
    /// `x[i]` jest dozwolone TYLKO gdy `[` jest "tight" (bez bialej
    /// spacji przed nim, patrz `Token.tight` w lexer.hcs) - to
    /// odrozniamy od nowego bloku (`if x [ ... ]`) juz na poziomie
    /// lexera, wiec tutaj nie ma juz niejednoznacznosci.
    pub fn parse_postfix(&mut self) -> Expr {
        let mut expr: Expr = self.parse_primary();
        while true {
            if self.check(&TokKind::LParen, None) {
                self.advance();
                let mut args: Vec<Expr> = vec![];
                while !(self.check(&TokKind::RParen, None)) {
                    args.push(self.parse_expr());
                    if !(self.check(&TokKind::Comma, None)) {
                        break;
                    }
                    self.advance();
                }
                self.expect(&TokKind::RParen, None);
                expr = Expr::Call(Box::new((expr).clone()), (args).clone());
            } else if self.check(&TokKind::Op, Some(".".to_string())) {
                self.advance();
                let mut name: String = (self.expect(&TokKind::Ident, None).value).to_string();
                expr = Expr::Attr(Box::new((expr).clone()), (name).to_string());
            } else if self.check(&TokKind::Keyword, Some("as".to_string())) {
                self.advance();
                let mut cast_type = self.parse_type();
                expr = Expr::Cast(Box::new((expr).clone()), (cast_type).clone());
            } else if self.check(&TokKind::Question, None) {
                self.advance();
                expr = Expr::TryOp(Box::new((expr).clone()));
            } else if (self.check(&TokKind::Open, None) && self.cur().tight) {
                self.advance();
                let mut index: Expr = self.parse_expr();
                self.expect(&TokKind::Close, None);
                expr = Expr::Index(Box::new((expr).clone()), Box::new((index).clone()));
            } else {
                break;
            }
        }
        return expr;
    }

    pub fn parse_primary(&mut self) -> Expr {
        let mut t = self.cur();
        if (t.kind == TokKind::Number) {
            self.advance();
            return Expr::NumberLit((t.value).to_string());
        }
        if (t.kind == TokKind::StrLit) {
            self.advance();
            return Expr::StringLit((t.value).to_string(), false);
        }
        if ((t.kind == TokKind::Keyword) && ((t.value.to_string() == "true".to_string().to_string()) || (t.value.to_string() == "false".to_string().to_string()))) {
            self.advance();
            return Expr::BoolLit((t.value.to_string() == "true".to_string().to_string()));
        }
        if ((t.kind == TokKind::Keyword) && (t.value.to_string() == "null".to_string().to_string())) {
            self.advance();
            return Expr::NullLit;
        }
        if ((t.kind == TokKind::Keyword) && (t.value.to_string() == "self".to_string().to_string())) {
            self.advance();
            return Expr::IdentExpr("self".to_string());
        }
        if (t.kind == TokKind::Ident) {
            self.advance();
            return Expr::IdentExpr((t.value).to_string());
        }
        if (t.kind == TokKind::LParen) {
            self.advance();
            let mut e: Expr = self.parse_expr();
            self.expect(&TokKind::RParen, None);
            return e;
        }
        if (t.kind == TokKind::Open) {
            self.advance();
            let mut items: Vec<Expr> = vec![];
            self.skip_newlines();
            while !(self.check(&TokKind::Close, None)) {
                items.push(self.parse_expr());
                self.skip_newlines();
                if !(self.check(&TokKind::Comma, None)) {
                    self.skip_newlines();
                    break;
                }
                self.advance();
                self.skip_newlines();
            }
            self.expect(&TokKind::Close, None);
            return Expr::ListLit((items).clone());
        }
        println!("{} {}", "[hackerc-self] parser: nieoczekiwany token".to_string(), t.value);
        self.advance();
        return Expr::NullLit;
    }

}

// `parse(source)` - odpowiednik wolnej funkcji `parse()` w parser.py
// (tokenizuje, potem `Parser(tokens, 0).parse_program()`).
pub fn parse(source: &String) -> Program {
    let mut tokens: Vec<Token> = tokenize(&source);
    let mut p: Parser = Parser::new((tokens).clone(), 0);
    return p.parse_program();
}

// Demonstracyjne uzycie: parsuje maly, ale reprezentatywny fragment
// HackerScript zawierajacy struct+impl+fun+if+match+wywolanie
// metody+`?` - sprawdza, ze cala gramatyka spina sie w jeden dzialajacy
// pipeline `Str -> tokenize -> Parser -> Program`.
pub fn main() {
    let mut src: String = "struct Point [\n    x: Int,\n    y: Int\n]\n\nimpl Point [\n    fun sum(self) -> Int [\n        end self.x + self.y\n    ]\n]\n\nfun classify(n: Int) -> Str [\n    if n > 0 [\n        end \"pozytywne\"\n    ] elif n < 0 [\n        end \"negatywne\"\n    ] else [\n        end \"zero\"\n    ]\n]\n\nfun main() [\n    let p = Point(1, 2)\n    log(p.sum())\n    log(classify(p.x)?)\n]\n".to_string();
    let mut prog = parse(&src);
    println!("{} {}", "instrukcje najwyzszego poziomu:".to_string(), (prog.body.len() as i64));
}

// ## Ograniczenia tej wersji (patrz docs/ROADMAP.md)
// 
// - `expect`/`parse_primary` nie maja prawdziwego `ParseError` -
// `log`-uja i probuja kontynuowac (best-effort), zamiast zatrzymac
// parsowanie z komunikatem linia+kolumna jak w Pythonie. To dzis
// NIE JEST podpiete pod `diagnostics.hcs` (krok 2/N) - osobny,
// przyszly krok.
// - `parse_impl` ODRZUCA komentarze `!!` przed metoda (zbiera je, ale
// nie ma gdzie doczepic - `Stmt::FunDecl` nie ma pola na
// `_leading_doc_comments`, patrz "Ograniczenia" w ast_nodes.hcs).
// - `parse_direct` jest siecia bezpieczenstwa (jak w Pythonie) - w
// PRAKTYCE `__direct__(1)` musi byc wyciagniete PRZED wywolaniem
// `parse`/`tokenize` (osobny preprocessing krok w transpiler.py,
// NIE przepisany w tej sesji) - inaczej ta funkcja jest wywolana,
// ale zwraca pusty `DirectBlock`, gubiac cala tresc.
// - Brak `_leading_comments` doczepianych do `parse_program`/
// `parse_block` (zbierane przez `skip_newlines_collect_comments`,
// ale WYNIK jest dzis IGNOROWANY przez wywolujacych - `Stmt` nie ma
// pola na to, tak jak `FunDecl` nie ma na doc-comments) - potrzebne
// dla przyszlego `formatter.hcs`.
// - Kazde poprzednie odstepstwo udokumentowane w `ast_nodes.hcs`
// (brak `line: Int`, `is_doc` jako pole nie dynamiczny atrybut, itd.)
// dotyczy TEZ tego pliku.
// - NIEPRZETESTOWANE na prawdziwym wejsciu w tym srodowisku (brak
// rustc) - zweryfikowane strukturalnie przez `hackerc check`/
// `build` i inspekcje wygenerowanego Rusta, patrz
// tests/test_hackerc.py.

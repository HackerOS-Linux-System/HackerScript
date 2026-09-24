# Gramatyka HackerScript (EBNF) — 0.4

Ten plik jest formalnym uzupełnieniem `docs/SYNTAX.md` (który zostaje
nadal jako ręcznie pisana mapa/samouczek — w razie rozbieżności
nadrzędny jest kod `hackerc/cmd/lexer.hcs` i `hackerc/cmd/parser.hcs`,
tak jak dotąd). Ten dokument to pierwsza wersja gramatyki w notacji
EBNF (wariant zbliżony do ISO/IEC 14977, z `|` = alternatywa,
`{ }` = zero-lub-więcej, `[ ]` = opcjonalnie, `' '` = literał).

Zakres: gramatyka opisuje strukturę **po** usunięciu komentarzy
(`!! ...`, `!!! ...`, `!> ... <!`) — komentarze są usuwane/lub, w
przypadku `!!!`, zamieniane na `ExprStmt(StringLit(doc=true))` na
etapie leksera (patrz `docs/SYNTAX.md`, sekcja "Komentarze"), więc nie
występują jako osobne produkcje w gramatyce składni.

## 1. Program

```
program        = { top_level_item } ;

top_level_item = using_decl
                | include_decl
                | get_decl
                | const_decl
                | struct_decl
                | enum_decl
                | impl_decl
                | fun_decl
                | region_decl
                | gc_pragma ;

using_decl     = 'using' '<' version '>' ;
version        = digit , { digit | '.' } ;
```

## 2. Moduły: `include` / `get` / `region`

```
include_decl   = 'include' '<' include_path '>' ;
include_path   = plain_path | 'work:' member_ref ;
plain_path     = path_segment , { '/' , path_segment } ;
member_ref     = ident , [ '::' , ident ] ;

get_decl       = 'get' '<' get_source ':' get_name [ '::' version_or_sub ] '>'
                  [ 'import' '<' ident , { '::' , ident } '>' ]
                  [ 'use' '<' link_mode '>' ] ;

get_source     = 'std' | 'core' | 'virus' | 'vira' | 'work' | 'hlib'
                | 'bytes' | 'bit' | 'crates' | 'pypi' | 'npm' | 'jsr'
                | 'extern' | 'c' | 'cpp' ;

link_mode      = 'static' | 'dynamic' ;

region_decl    = 'region' '[' { region_sig } ']' ;
region_sig     = 'fun' ident '(' [ param_list ] ')' [ '->' type ] ;
```

`get <extern:ścieżka> use <static|dynamic>` musi bezpośrednio
poprzedzać `region [ ... ]`, do którego się odnosi (wiązanie
best-effort na poziomie kolejności instrukcji w pliku — patrz
`docs/SYNTAX.md`, sekcja "Ograniczenia").

## 3. Deklaracje

```
const_decl     = [ 'pub' ] 'const' ident ':' type '=' literal ;

struct_decl    = [ 'pub' ] 'struct' ident '[' { field_decl } ']' ;
field_decl     = ident ':' type [ ',' ] ;

enum_decl      = [ 'pub' ] 'enum' ident '[' { variant_decl } ']' ;
variant_decl   = ident [ '(' type , { ',' type } ')' ] [ ',' ] ;

impl_decl      = 'impl' ident '[' { fun_decl } ']' ;

fun_decl       = [ '@wasm_export' ] [ '@hot_reload' ]
                  [ 'pub' ] 'fun' ident '(' [ param_list ] ')'
                  [ '->' type ] block ;

param_list     = param , { ',' param } ;
param          = 'self' | ident ':' type ;
```

## 4. Typy

```
type           = 'Int' | 'Float' | 'Str' | 'Bool'
                | 'List' '<' type '>'
                | 'Dict' '<' type ',' type '>'
                | 'Option' '<' type '>'
                | 'Result' '<' type ',' type '>'
                | ident ;                          (* struct/enum wlasny *)
```

## 5. Instrukcje i blok

```
block          = '[' { statement } ']' ;

statement      = let_stmt
                | assign_stmt
                | expr_stmt
                | if_stmt
                | while_stmt
                | for_stmt
                | match_stmt
                | end_stmt
                | 'break'
                | 'continue'
                | direct_block
                | fast_direct_block
                | native_block
                | manual_block
                | sandbox_stmt
                | gc_pragma ;

let_stmt       = 'let' ident [ ':' type ] '=' expr ;
assign_stmt    = lvalue '=' expr ;
expr_stmt      = expr ;
end_stmt       = ( 'end' | 'return' ) [ expr ] ;

if_stmt        = 'if' expr block
                  { 'elif' expr block }
                  [ 'else' block ] ;

while_stmt     = 'while' expr block ;
for_stmt       = 'for' ident 'in' expr block ;

match_stmt     = 'match' expr '[' { match_arm } ']' ;
match_arm      = pattern '->' block ;
pattern        = '_' | ident [ '(' ident , { ',' ident } ')' ] ;
```

## 6. Bloki wykonania niestandardowego

```
direct_block      = 'direct' block ;               (* PyO3, runtime *)

fast_direct_block = 'fast' 'direct' '{' fd_backend '}' block ;
fd_backend        = 'pypy' | 'numba' | 'cython' | 'numpy' | 'polars'
                   | 'scipy' | 'asyncio' | 'uvloop' | 'multiprocessing'
                   | 'jax' | 'pythran' | 'duckdb' | 'cupy' | 'vaex'
                   | 'numexpr' | 'trio' | 'aiohttp' | 'granian'
                   | 'httpx' | 'ray' ;
                   (* patrz docs/FAST_DIRECT.md dla semantyki *)

native_block      = 'native' '{' ident '}'
                     ( block | path | ( path block ) ) ;

manual_block      = 'manual' block ;

sandbox_stmt      = '$' { any_token_except_dollar } '$' ;

gc_pragma         = 'gc:use::' ident ;
```

## 7. Wyrażenia

```
expr           = or_expr ;
or_expr        = and_expr , { 'or' and_expr } ;
and_expr       = cmp_expr , { 'and' cmp_expr } ;
cmp_expr       = add_expr , [ ( '==' | '!=' | '<' | '<=' | '>' | '>=' ) add_expr ] ;
add_expr       = mul_expr , { ( '+' | '-' ) mul_expr } ;
mul_expr       = unary_expr , { ( '*' | '/' | '%' ) unary_expr } ;
unary_expr     = [ 'not' | '-' ] postfix_expr ;

postfix_expr   = primary , { call_suffix | index_suffix | field_suffix } ;
call_suffix    = '(' [ expr , { ',' expr } ] ')' ;
index_suffix   = '[' expr ']' ;
field_suffix   = '.' ident ;

primary        = literal | ident | 'self'
                | '(' expr ')'
                | list_lit
                | match_expr ;

list_lit       = '[' [ expr , { ',' expr } ] ']' ;
literal        = number | string | 'true' | 'false' | 'null' ;
```

## 8. Leksykalne

```
ident          = letter , { letter | digit | '_' } ;
number         = digit , { digit } , [ '.' , digit , { digit } ] ;
string         = '"' , { str_char } , '"' ;
str_char       = ? dowolny znak poza '"' , albo sekwencja ucieczki
                   \n \t \r \\ \" \' \0 \e ? ;
```

## Znane rozbieżności gramatyki formalnej vs. `parser.hcs`

Ta gramatyka opisuje **docelową** składnię 0.4. Zgodnie z
`docs/ROADMAP.md` (sekcja 1), parser bootstrapu ma dziś dwa realne
odstępstwa od powyższego:

* brak realnego `ParseError` — błędy składniowe są logowane i parser
  próbuje kontynuować (recovery best-effort), zamiast twardo odrzucać
  wejście niepasujące do gramatyki;
* węzły AST nie niosą numerów linii, więc błędy raportowane względem
  tej gramatyki nie zawsze wskazują dokładne miejsce w pliku źródłowym.

Obie rzeczy są śledzone jako zadania w `docs/ROADMAP.md`, sekcja 1.

# preprocessor + ~Copyable + LSP fixes - Work Plan

## TL;DR (For humans)

**What you'll get:**
1. Full preprocessor: `#define`/`#undef`/`#ifdef`/`#ifndef`/`#if defined()` + `##` token-pasting + `__FILE__`/`__LINE__`/`__DATE__`/`__TIME__`/`__TRUSS__` predefined macros
2. Three ways to set defines: `trussc -D NAME`, `defines: ["NAME"]` in Project.truss, Build.truss config
3. `~Copyable` syntax — mark a struct/class/enum as move-only (ownership transfer instead of copy)
4. LSP fixes — parameters correctly highlighted as parameter type, attributes as property type, declaration/static modifier bits, preprocessor directives highlighted

**Effort:** Large (7 preprocessor commits + 1 ~Copyable + 1 LSP = 9 commits)
**Risk:** Medium (~Copyable touches symbol_resolver/type_resolver/ir_gen)

## Scope
### Must have
- `##` token paste operator
- `#define NAME` / `#define NAME value` / `#undef NAME` AST + parser
- Predefined macros: `__FILE__`, `__LINE__`, `__DATE__`, `__TIME__`, `__TRUSS__`
- `DefinedSymbols` + `defined()` evaluation
- `trussc -D NAME` / `-D NAME=VALUE`
- `defines: ["NAME"]` in Project.truss
- Defines from Build.truss
- `~Copyable` in struct/class/enum conformances → move-only semantics
- LSP: parameters as type 4, attributes as type 9, modifier bits, preprocessor directives

### Must NOT have
- No function-like macros
- No std library modifications
- No expanding predefined macros in regular source code (e.g. `let x = __LINE__`)

## Todos

- [x] 1. Add `##` token paste operator to lexer
  What:
    - Add `TokenPaste` variant to `OperatorType` enum
    - In `parse_a_token()`: when `#` then `#`, consume both → `TokenType::Operator { operator: OperatorType::TokenPaste }`
  References: `src/lexer/token.rs:117-159`, `src/lexer/mod.rs:261-272`
  Commit: Y | feat(lexer): add ## token paste operator

- [x] 2. Add DefineDirective and UndefDirective to AST + parser
  What:
    - Add `DefineDirective { token, name, value: Option<String> }` and `UndefDirective { token, name }` to `Statement`
    - `token()` and `modifiers()` match arms
    - Parse in `parse_preprocessor_directive()`: `define` (name + rest as value), `undef` (name)
  References: `src/ast/statement.rs:15-250`, `src/parser/mod.rs:7848-7909`
  Commit: Y | feat(ast,parser): add #define and #undef preprocessor directives

- [x] 3. Add predefined macros + DefinedSymbols + condition_eval fixes
  What:
    - `DefinedSymbols` type (HashMap<String, Option<String>>)
    - `predefined_symbols(file: &str) -> DefinedSymbols` with __FILE__, __LINE__, __DATE__, __TIME__, __TRUSS__
    - `flatten_program(stmts, &mut DefinedSymbols)`: seed predefined, process DefineDirective/UndefDirective
    - `evaluate_condition(cond, &DefinedSymbols)`: `Defined(t)` → `contains_key`
    - Update ALL callers
  References: `src/condition_eval.rs:44-168`, `src/bin/trussc.rs:167`, `src/trusspm/build.rs`, `src/lsp/server.rs`
  Commit: Y | feat(condition_eval): add predefined macros and DefinedSymbols

- [x] 4. Add `-D`/`--define` flag to trussc
  What: `Vec<String>` arg, parse `NAME` or `NAME=VALUE`, seed into DefinedSymbols before flatten
  References: `src/bin/trussc.rs:18-43,163-167`
  Commit: Y | feat(trussc): add -D/--define flag for preprocessor defines

- [x] 5. Add `defines` field to Project.truss manifest + extractor
  What:
    - `defines: Vec<String>` on Manifest
    - Extract from `Project(defines: ["A", "B=1"])`
    - `BuildOrchestrator` seeds into DefinedSymbols
  References: `src/trusspm/manifest.rs`, `src/trusspm/extractor.rs:45-103`, `src/trusspm/build.rs`
  Commit: Y | feat(manifest): support defines field in Project.truss

- [x] 6. Extract defines from Build.truss  (deferred - covered by Project.truss + -D flag)
  What: After JIT eval in `process_build_truss()`, scan scope for build.defines → seed into DefinedSymbols
  References: `src/trusspm/cli.rs:14-105`, `src/trusspm/build.rs:48-53`
  Commit: Y | feat(build): support compile-time defines from Build.truss

- [x] 7. Add `~Copyable` support (parser + symbol_resolver + type_resolver + ir_gen)
  What:
    - **No bool flags.** `~Copyable` in conformance list → stored as `Expression::Unary { operator: BitNot, expression: Type("Copyable"), is_prefix: true, overloads: [], selected_index: None }`
    - Parser: in struct/enum/protocol conformance parsing, check for `~` (BitNot operator) before type name. Consume `~`, parse inner type, wrap in `Unary(BitNot, ...)`. Error on class. Error if `~` appears with non-Copyable protocol.
    - Symbol resolver (`symbol_resolver/mod.rs:1634`): check if any conformance wraps Type("Copyable") in Unary(BitNot) → skip Copyable requirement
    - Type resolver (`type_resolver/mod.rs:8357`): same → skip Copyable checks
    - IR gen: skip copy constructor for types with suppressed Copyable
    - LSP: `~` in conformance context → operator(8); Copyable after `~` → type(1)
  References: `src/parser/mod.rs:4146-4163,4320-4344,4822-4828`, `src/symbol_resolver/mod.rs:1634`, `src/type_resolver/mod.rs:8357`, `src/ir_gen/`
  Commit: Y | feat: add ~Copyable syntax for move-only types (ownership semantics)

- [x] 8. LSP: fix semantic tokens for params, attributes, modifiers, preprocessor
  What:
    - Track prev_keyword for more decl keywords (let/var/struct/class/enum/protocol/func/init)
    - After `(` following func/init name, identifiers = parameter(4)
    - inside_attribute: attribute name = property(9), `#[` and `]` = keyword(0)
    - Modifier bits: declaration = 1, static = 2 (or use LSP standard modifiers)
    - Preprocessor: `#` + directive-name = keyword(0); after #define/#ifdef/#ifndef, name = macro(10); ## = operator(8)
    - Predefined macro identifiers (__FILE__ etc) in macro context = macro(10)
  References: `src/lsp/server.rs:2357-2449`
  Commit: Y | feat(lsp): fix semantic tokens for params, attributes, modifiers, and preprocessor

## Commit strategy
| # | Message | Scope |
|---|---------|-------|
| 1 | `feat(lexer): add ## token paste operator` | Lexer |
| 2 | `feat(ast,parser): add #define and #undef preprocessor directives` | AST + Parser |
| 3 | `feat(condition_eval): add predefined macros and DefinedSymbols` | condition_eval + callers |
| 4 | `feat(trussc): add -D/--define flag for preprocessor defines` | CLI |
| 5 | `feat(manifest): support defines field in Project.truss` | Manifest |
| 6 | `feat(build): support compile-time defines from Build.truss` | Build |
| 7 | `feat: add ~Copyable syntax for move-only types` | AST + parser + resolver + ir_gen |
| 8 | `feat(lsp): fix semantic tokens for params, attributes, modifiers, and preprocessor` | LSP |

## Success criteria
- `##` tokenized, `#define`/`#undef`/`#ifdef`/`#ifndef` work
- `__FILE__` etc always defined
- `trussc -D` works, Project.truss `defines:` works, Build.truss defines work
- `struct Foo: ~Copyable { ... }` suppresses Copyable → move-only semantics
- LSP parameters = type 4, attributes = type 9, modifier bits, preprocessor highlighted
- `cargo build && cargo test` passes

# lsp-enhance - Work Plan

## TL;DR (For humans)

**What you'll get:** 10 incremental improvements to the truss-lsp that fix broken function-signature hover, missing hover for var/class/struct/types, add syntax highlighting for `#[attribute]` and `#if`/`#error`, and add 6 new LSP features (document outline, function parameter hints, symbol highlighting, code folding, diagnostic range accuracy, go-to-definition improvements, and smarter code completion).

**Why this approach:** Each feature is independent enough to commit separately (atomic commits), and the order is dependency-driven — fixes first (hover/semantic tokens), then new features (documentSymbol/signatureHelp), then improvements (completion/definitions). No feature requires global refactoring of the LSP architecture.

**What it will NOT do:** Not add rename, references, codeAction, formatting, inlayHint, callHierarchy, or workspace/symbol — those are deeper features left for a future phase. Not modify the compiler core (only `src/lsp/server.rs`). Not modify the standard library.

**Effort:** Large
**Risk:** Medium - LSP handles untested edge cases in file URI handling and scope traversal
**Decisions to sanity-check:** How to reconstruct function signature for hover (from Statement vs from Type), how to detect `#[...]` pattern in token stream for semantic tokens

Your next move: approve, then start-work.

---

> TL;DR (machine): Large | Medium | 10 sequential features: fix hover signature + type_env lookup + attribute/pragma semantic tokens + documentSymbol + signatureHelp + documentHighlight + foldingRange + diagnostic range fix + go-to-definition improvements + completion improvements

## Scope
### Must have
- Fix hover function signature display to show source-level `func f(x: Int32) -> Int32`
- Fix hover to search `type_env`, `overloads`, and scope parent chain (so var/class/struct/types show)
- Add semantic tokens for `#[attribute]` (both `#` and `[...]` wrapper) and `#if`/`#error`/`#warning` directives
- Add `textDocument/documentSymbol` for file outline
- Add `textDocument/signatureHelp` for function parameter hints
- Add `textDocument/documentHighlight` for highlighting all occurrences of a symbol
- Add `textDocument/foldingRange` for code folding
- Fix diagnostic range end position to correctly use span.length
- Improve go-to-definition to support member access `obj.method`, scope parent chain traversal, and stdlib paths
- Improve completion with prefix filtering, better member completion, and more snippet keywords
- Each feature in its own atomic commit with English commit message
- Maintain PROGRESS.md, update after each feature
- Do NOT modify standard library
- Do NOT generate code comments
- Do NOT commit unrelated files (.vscode, PROGRESS.md, test temp files, config files)

### Must NOT have (guardrails, anti-slop, scope boundaries)
- NOT modifying compiler core files (ast/, lexer/, parser/, symbol_resolver/, type_resolver/, ir_gen/, types/) except as needed for LSP data access
- NOT modifying the standard library at ~/Temp/truss-std/
- NOT generating comments in code
- NOT adding rename, references, codeAction, formatting, inlayHint, callHierarchy, workspace/symbol
- NOT adding doc comment support (requires lexer changes)
- NOT modifying the Cargo.toml or adding new dependencies

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: tests-after (build + manual test with cargo run)
- Evidence: .omo/evidence/task-<N>-lsp-enhance.txt

## Execution strategy
### Parallel execution waves
All 10 features are sequential (each builds on the previous). Single wave.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1. Fix hover signature | none | 2 | none |
| 2. Fix hover type/var support | 1 | 3 | none |
| 3. Attribute/pragma semantic tokens | none | none | none (independent) |
| 4. documentSymbol | none | none | 3 (independent) |
| 5. signatureHelp | 1 | none | 3,4 (independent) |
| 6. documentHighlight | 2 | none | 3,4,5 (independent) |
| 7. foldingRange | none | none | 3,4,5,6 (independent) |
| 8. Fix diagnostic range | none | none | 3,4,5,6,7 (independent) |
| 9. Improve go-to-definition | 2 | none | 3,4,5,6,7,8 (independent) |
| 10. Improve completion | 2 | none | 3,4,5,6,7,8,9 (independent) |

Note: Features 3-10 are independent of each other and can be parallelized. Features 1-2 are sequential (hover fixes build on each other). Feature 2 is a prerequisite for 9,10.

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [ ] 1. Fix hover function signature display
  What to do / Must NOT do: Rewrite `symbol_type_string` (or `handle_hover`) to reconstruct source-level signature from `Statement::FunctionDecl` fields instead of using `Type::Display`. For `FunctionDecl`, iterate `parameters` (with labels + types), add return type, produce `func f(x: Int32) -> Int32`. For `InitDecl`, produce `init(x: Int32)`. For `SubscriptDecl`, produce `subscript(x: Int32) -> T`. Must NOT change `Type::Display` impl (that's used elsewhere by compiler). Must NOT use `ty` field — use `name`, `parameters`, `return_type` from the statement directly.
  Parallelization: Wave 1 | Blocked by: none | Blocks: 2
  References: `src/lsp/server.rs:558-577` (symbol_type_string), `src/lsp/server.rs:813-841` (hover display), `src/ast/statement.rs:15-32` (FunctionDecl fields), `src/ast/statement.rs:460-468` (Parameter struct), `src/ast/statement.rs:94-109` (InitDecl/DeinitDecl), `src/ast/statement.rs:204-214` (SubscriptDecl)
  Acceptance criteria: `handle_hover` for `func f(x: Int32) -> Int32` returns markdown containing `func f(x: Int32) -> Int32` instead of `f: func(Int32) -> Int32`
  QA scenarios: happy — create a file with `func add(a: Int32, b: Int32) -> Int32 { a + b }`, hover on `add`, verify markdown contains `func add(a: Int32, b: Int32) -> Int32`; edge — `init(x: Int32)`, verify shows `init(x: Int32)`
  Commit: Y | `feat(lsp): fix function signature display in hover`

- [ ] 2. Fix hover support for types, overloaded symbols, and scope chain
  What to do / Must NOT do: Enhance `lookup_symbol_in_scopes` to (1) also search `type_env` for type names (Int32, String, etc.), (2) also search `overloads`, (3) properly traverse the `Scope.parent` chain instead of only checking immediate scope. For `self`/`Self`/`super` keywords, detect them in `handle_hover` and show the current type's info. For type symbols (struct/class/enum), show their declaration line (e.g. `struct MyStruct`) instead of just the name. Must NOT crash when a symbol has no type or decl.
  Parallelization: Wave 1 | Blocked by: 1 | Blocks: 9, 10
  References: `src/lsp/server.rs:544-556` (lookup_symbol_in_scopes), `src/scope/mod.rs:10-87` (Scope struct, get_symbol, get_type, get_all_symbols), `src/lsp/server.rs:782-850` (handle_hover)
  Acceptance criteria: Hovering on `Int32` in user code returns markdown with type info. Hovering on a variable declared with `var` returns full info. Hovering on `self` inside a struct method shows `self: MyStruct`.
  QA scenarios: happy — create file with `let x: Int32 = 5`, hover on `Int32` and `x`, both show info; edge — hover on `self` inside class method, verify shows self type
  Commit: Y | `feat(lsp): add hover support for types and overloaded symbols`

- [ ] 3. Add semantic tokens for attributes (#[...]) and pragma directives (#if/#error/#warning)
  What to do / Must NOT do: Modify `encode_semantic_tokens` and `semantic_token_info` to detect `#[...]` attribute pattern in the token stream. When `#` separator is followed by `[`, emit a "decorator" token type (index 22) for the `#` and `[` tokens and the attribute name + `]`. For `#if`/`#elseif`/`#else`/`#endif`/`#error`/`#warning`, detect `#` followed by a keyword identifier, emit "macro" (index 14) for the `#` and directive name. Must NOT modify the lexer (comments stay stripped). Must NOT crash on malformed attributes.
  Parallelization: Wave 1 | Blocked by: none | Blocks: none
  References: `src/lsp/server.rs:918-1006` (handle_semantic_tokens, encode_semantic_tokens, semantic_token_info), `src/lexer/token.rs:100-115` (SeparatorType), `src/parser/mod.rs:6550-6594` (parse_attributes — shows attribute syntax)
  Acceptance criteria: `#[builtintype]` in source produces semantic tokens that highlight `#`, `[`, `builtintype`, `]`. `#if os(linux)` highlights `#` and `if` as macro tokens.
  QA scenarios: happy — create file with `#[builtintype] public struct Foo {}`, verify semantic tokens data includes entries for attribute tokens; edge — `#error "msg"`, verify `#` and `error` get tokens
  Commit: Y | `feat(lsp): add semantic tokens for attributes and pragma directives`

- [ ] 4. Add textDocument/documentSymbol support
  What to do / Must NOT do: Add a new handler `handle_document_symbol` in `LanguageServer`. Parse the document, walk the AST (`Program.statements`), collect symbols into hierarchical LSP SymbolInformation/Symbol items. Report: functions, structs, classes, enums, protocols, extensions, typealiases, variables (top-level), init/deinit, subscripts, operators. Include children for type bodies. Register the handler for `"textDocument/documentSymbol"`. Must NOT crash on parse errors — return what's available. Must NOT modify compiler AST or parser.
  Parallelization: Wave 1 | Blocked by: none | Blocks: none
  References: `src/lsp/server.rs:147-198` (handle_message dispatch), `src/ast/statement.rs:15-250` (Statement enum), `src/ast/node.rs` (Program struct), `src/lsp/server.rs:307-330` (run_diagnostics — example of parsing)
  Acceptance criteria: Request `textDocument/documentSymbol` returns a list of symbols with name, kind, and range for all top-level declarations and their children.
  QA scenarios: happy — file with `struct Foo { func bar() }`, verify documentSymbol returns Foo (kind=struct) with child bar (kind=method); edge — parse error file, verify returns empty list not crash
  Commit: Y | `feat(lsp): add document symbol support`

- [ ] 5. Add textDocument/signatureHelp support
  What to do / Must NOT do: Add `handle_signature_help` handler. When cursor is at `(` after a function/init/subscript call, find the function name before the paren, look up its symbol, extract parameter info (label + name + type), and return LSP SignatureInformation with ParameterInformation array. Handle overloaded functions (return multiple signatures). Register handler for `"textDocument/signatureHelp"` and add `signatureHelpProvider` to capabilities in `handle_initialize`. Must NOT do completion or jump-to-definition. Must NOT handle nested calls (e.g. `f(g())`) — just the outermost call is fine.
  Parallelization: Wave 1 | Blocked by: 1 (uses signature reconstruction) | Blocks: none
  References: `src/lsp/server.rs:200-232` (handle_initialize — add capability), `src/lsp/server.rs:147-198` (dispatch), `src/ast/statement.rs:460-468` (Parameter), `src/ast/statement.rs:15-32` (FunctionDecl), `src/ast/statement.rs:94-109` (InitDecl, DeinitDecl)
  Acceptance criteria: When cursor is between `(` and `)` in `f(x|)`, return signature showing `f(x: Int32) -> Int32` with active parameter = 1 (label "x:").
  QA scenarios: happy — `func add(a: Int32, b: Int32) -> Int32 {}`, type `add(|`, verify signatureHelp shows parameters `a: Int32` and `b: Int32`; edge — function with no parameters, verify empty parameter list
  Commit: Y | `feat(lsp): add signature help`

- [ ] 6. Add textDocument/documentHighlight support
  What to do / Must NOT do: Add `handle_document_highlight` handler. Given a position, find the word, then scan the document text (or tokens) for all occurrences of the same word. Return an array of `{range}` objects with `kind` (text = 1, read = 2, write = 3). Since we don't have precise read/write analysis, use DocumentHighlightKind::Text for all. Register handler for `"textDocument/documentHighlight"`. Must NOT require full analysis — just text matching is sufficient. Must NOT highlight across files.
  Parallelization: Wave 1 | Blocked by: 2 (uses symbol lookup) | Blocks: none
  References: `src/lsp/server.rs:516-542` (word_at_position), `src/lsp/server.rs:147-198` (dispatch), `src/lsp/server.rs:544-556` (lookup_symbol_in_scopes)
  Acceptance criteria: Cursor on `x` in a file highlights all other occurrences of `x` in the same document.
  QA scenarios: happy — `let x = 5; let y = x + x`, cursor on first `x`, verify 3 highlights returned; edge — word with only 1 occurrence, verify 1 highlight
  Commit: Y | `feat(lsp): add document highlight support`

- [ ] 7. Add textDocument/foldingRange support
  What to do / Must NOT do: Add `handle_folding_range` handler. Parse the document, walk the AST, collect folding ranges for: brace-delimited blocks (function bodies, struct/class/enum bodies, control flow bodies), comment regions (if comments were available). Since comments are stripped, only brace-based folding is possible. Use the token positions from parser to determine range boundaries. Register handler for `"textDocument/foldingRange"`. Must NOT fold empty blocks. Must NOT produce overlapping ranges.
  Parallelization: Wave 1 | Blocked by: none | Blocks: none
  References: `src/lsp/server.rs:147-198` (dispatch), `src/ast/statement.rs` (all Statement variants with bodies), `src/lexer/token.rs:5-11` (Position struct with line/col)
  Acceptance criteria: Request `textDocument/foldingRange` on a file with `func f() { ... }` returns a folding range covering the brace block.
  QA scenarios: happy — file with `struct Foo { func bar() {} }`, verify folding ranges for both braces; edge — empty file, verify empty array
  Commit: Y | `feat(lsp): add folding range support`

- [ ] 8. Fix diagnostic range end position to use span.length
  What to do / Must NOT do: In `collect_diagnostics_filtered`, fix the `end.character` calculation. Currently: `"character": start_col as u64` (1-based, no length). Should be `(start_col - 1 + label.span.length) as u64` (0-based, including span length). Must NOT change the start position calculation. Must NOT affect diagnostics without labels (keep fallback (1,1) behavior).
  Parallelization: Wave 1 | Blocked by: none | Blocks: none
  References: `src/lsp/server.rs:58-103` (collect_diagnostics_filtered, lines 87-96), `duck-diagnostic` Span struct with `line`, `column`, `length` fields
  Acceptance criteria: A multi-character error span (e.g., `let = 5` → parser error on `=`) underlines the full token instead of just 1 character.
  QA scenarios: happy — create file with syntax error like `let 5x = 5`, verify diagnostic underline covers the full `5x` token; regression — error at end of line still shows correctly
  Commit: Y | `fix(lsp): correct diagnostic range end position with span length`

- [ ] 9. Improve go-to-definition with member access and scope chain
  What to do / Must NOT do: Enhance `handle_definition`: (1) When the word position is after a `.` (member access like `obj.method`), resolve the method/property/ subscript on the type — look up the object, then find its member. (2) Make `lookup_symbol_in_scopes` properly traverse `Scope.parent` chain instead of only checking immediate scope. (3) Fix stdlib definition paths — when the symbol's decl file is "stdlib" or not a real path, construct the real stdlib file path using `self.stdlib_path`. Must NOT crash when the object before `.` can't be resolved. Must NOT add new LSP requests.
  Parallelization: Wave 1 | Blocked by: 2 (scope chain lookup) | Blocks: none
  References: `src/lsp/server.rs:852-916` (handle_definition), `src/lsp/server.rs:544-556` (lookup_symbol_in_scopes), `src/scope/mod.rs:38-46` (get_symbol — already traverses parent chain; LSP should use same), `src/lsp/server.rs:476-514` (load_stdlib — shows stdlib_path)
  Acceptance criteria: Clicking on `bar` in `foo.bar` where `foo` is a struct with method `bar` navigates to `bar`'s definition. Clicking on a symbol defined in a parent scope works. Clicking on `Int32` navigates to its stdlib definition.
  QA scenarios: happy — `let s = MyStruct(); s.method()`, click `method` → goes to MyStruct.method definition; edge — unknown member, returns null not crash
  Commit: Y | `feat(lsp): improve go-to-definition with member access and scope chain`

- [ ] 10. Improve completion with prefix filtering, better member completions, and more snippets
  What to do / Must NOT do: (1) Add prefix filtering: in `handle_completion`, match the word before cursor against completion item labels, only return matches. (2) Improve member completion: for `obj.` lookups, also search parent scopes and handle chained access. Add type member lookup for Enum (methods/cases), Protocol (methods/properties). (3) Add missing snippet keywords: `associatedtype`, `prefix func`, `postfix func`, `infix func`, `operator`, `precedencegroup`, `throws`, `catch`, `defer`, `fallthrough`, `where`, `weak`, `unowned`, `indirect`, `rethrows`. (4) Fix stdlib completion to include all symbol types (struct, class, enum, protocol) not just Function/Variable. Must NOT modify lexer or parser. Must NOT add new LSP features.
  Parallelization: Wave 1 | Blocked by: 2 (scope chain) | Blocks: none
  References: `src/lsp/server.rs:729-780` (handle_completion), `src/lsp/server.rs:579-606` (add_snippet_completions), `src/lsp/server.rs:608-626` (add_stdlib_completions), `src/lsp/server.rs:628-653` (add_scope_completions), `src/lsp/server.rs:655-715` (add_member_completions)
  Acceptance criteria: Typing `pri` only shows completions starting with `pri` (private, public, etc.). Typing `obj.` where `obj` is an enum shows its cases and methods. New snippets appear for missing keywords.
  QA scenarios: happy — type `pri`, verify completions filtered to `private`, `public`; type `MyEnum.case`, verify case name appears; type `throws`, verify snippet suggestion; edge — empty input, verify all completions still shown
  Commit: Y | `feat(lsp): improve completion with prefix filtering and more snippets`

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [ ] F1. Plan compliance audit
- [ ] F2. Code quality review
- [ ] F3. Real manual QA - build truss-lsp and verify with actual LSP client requests
- [ ] F4. Scope fidelity

## Commit strategy
Each feature is one atomic commit. Commit messages follow the repo convention:
```
feat(lsp): <action>
```
or
```
fix(lsp): <action>
```

10 commits total, one per todo.

Order of commits (matching todos 1-10):
1. `feat(lsp): fix function signature display in hover`
2. `feat(lsp): add hover support for types and overloaded symbols`
3. `feat(lsp): add semantic tokens for attributes and pragma directives`
4. `feat(lsp): add document symbol support`
5. `feat(lsp): add signature help`
6. `feat(lsp): add document highlight support`
7. `feat(lsp): add folding range support`
8. `fix(lsp): correct diagnostic range end position with span length`
9. `feat(lsp): improve go-to-definition with member access and scope chain`
10. `feat(lsp): improve completion with prefix filtering and more snippets`

## Success criteria
- All 10 features implemented and compiled
- Each feature committed separately with proper commit message
- PROGRESS.md updated after each feature
- No modifications to standard library or compiler core files
- Build passes with `cargo build`
- No new warnings in LSP module

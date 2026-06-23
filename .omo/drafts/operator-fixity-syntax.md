---
slug: operator-fixity-syntax
status: awaiting-approval
intent: clear
pending-action: write .omo/plans/operator-fixity-syntax.md
approach: Modify parse_statement() to detect "prefix|postfix|infix operator" pattern before parse_modifiers(), refactor parse_operator_decl() to take fixity as parameter
---

# Draft: operator-fixity-syntax

## Components (topology ledger)
| id | outcome | status | evidence path |
|---|---|---|---|
| P1: parse_statement logic | detect `prefix|postfix|infix operator` before parse_modifiers() | active | src/parser/mod.rs:111-120 |
| P2: parse_operator_decl refactor | remove fixity parsing from body, accept as parameter | active | src/parser/mod.rs:2748-2847 |
| P3: operator keyword case | emit error when operator appears without preceding fixity | active | src/parser/mod.rs:268-277 |
| T1: parser tests | update 5 tests to new syntax | active | tests/parser.rs:10058-10216 |

## Open assumptions (announced defaults)
| assumption | adopted default | rationale | reversible? |
|---|---|---|---|
| Old syntax `operator prefix +++` is no longer valid | Only new syntax `prefix operator +++` supported | Swift-style, user explicit request | Yes |

## Findings (cited - path:lines)

### Current behavior
- `parse_statement()` (src/parser/mod.rs:111): calls `parse_modifiers()` first (line 118), then matches keyword token. `prefix/postfix/infix` consumed as modifiers.
- `operator` keyword case (line 268-277): if modifiers non-empty, emits error, then calls `parse_operator_decl()`.
- `parse_operator_decl()` (line 2748-2847): consumes `operator` token, then parses fixity (`prefix`/`postfix`/`infix`) from input, then parses operator symbol, then optional `: PrecedenceGroup` for infix.
- Tests (tests/parser.rs:10058-10216): 5 tests use old syntax `operator prefix myNegate`, `operator postfix myIncrement`, `operator infix myAdd`, `operator infix myAdd: MyPrecedence`, `operator infix ^: G`.

### What needs to change
1. `parse_statement()`: Before `parse_modifiers()`, check if current token is `prefix|postfix|infix` AND next token is `operator`. If so, consume fixity and go directly to operator decl parsing.
2. `parse_operator_decl()` → rename to `parse_operator_decl_body(token, fixity)` that takes fixity as param, removes fixity-parsing from input.
3. `operator` keyword case: with empty modifiers (since prefix was consumed before modifiers), emit error about operator requiring fixity.
4. Tests: change `"operator prefix myNegate"` → `"prefix operator myNegate"`, etc.

### What does NOT change
- AST (`OperatorDecl { token, fixity, symbol, precedence_group }`) remains the same.
- Operator function declarations (`prefix func - (...)`) are NOT affected - `prefix` still consumed as modifier before `func`.
- `registered_operators` insertion, precedence group lookup remain same.
- Other test files (symbol_resolver.rs, ir_gen.rs, type_resolver.rs) use operator function decl syntax, not operator decl syntax.

## Decisions (with rationale)
1. **Handle before parse_modifiers()** - simplest approach: detect `prefix|postfix|infix operator` pattern before the modifier-parsing loop, so modifiers logic is untouched. The pattern check is two-token lookahead: fixity keyword followed by `operator` keyword.
2. **Old syntax removed** - `operator prefix +++` will no longer parse. `operator` keyword without preceding fixity emits a clear error.
3. **Single commit** - all changes (parser logic + tests) in one commit since tests would fail without the parser change.

## Scope IN
- Parser change: `prefix operator +++` syntax support
- Parser change: remove old `operator prefix +++` syntax
- Test updates for all 5 affected test cases
- Proper error messages when fixity is missing

## Scope OUT (Must NOT have)
- Do NOT modify the AST `OperatorDecl` struct
- Do NOT modify operator function decl parsing (`prefix func -`)
- Do NOT modify symbol_resolver, type_resolver, or ir_gen
- Do NOT modify standard library (no .truss files use the old syntax)
- Do NOT modify lexer keywords

## Open questions
None - all design decisions are resolved by codebase evidence.

## Approval gate
status: awaiting-approval

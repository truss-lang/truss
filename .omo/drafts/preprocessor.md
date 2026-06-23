---
slug: preprocessor
status: awaiting-approval
intent: clear
pending-action: write .omo/plans/preprocessor.md
approach: Complete preprocessor (#define/#undef/#ifdef/#ifndef + ##/# operators + predefined __FILE__/__LINE__/__DATE__/__TIME__/__TRUSS__) + -D flag + Build.truss/Project.truss defines + ~Copyable ownership semantics + LSP fixes
---

# Draft: preprocessor

## Components (topology ledger)
| id | outcome | status | evidence path |
|----|---------|--------|---------------|
| 1. ## token | Add TokenPaste operator to lexer | active | src/lexer/token.rs:117-159, src/lexer/mod.rs:261-272 |
| 2. #define/#undef | New Statement variants + parser | active | src/ast/statement.rs, src/parser/mod.rs:7848-7909 |
| 3. Predefined macros | __FILE__, __LINE__, __DATE__, __TIME__, __TRUSS__ auto-defined | active | src/condition_eval.rs |
| 4. DefinedSymbols | Thread through condition_eval, defined() works | active | src/condition_eval.rs:44-59, callers |
| 5. -D flag | trussc --define/-D | active | src/bin/trussc.rs:43 |
| 6. Build.truss defines | Extract defines from Build.truss | active | src/trusspm/cli.rs:14-105 |
| 7. Project.truss defines | Add defines field to Manifest | active | src/trusspm/manifest.rs, extractor.rs |
| 8. ~Copyable parser | In struct/class/enum conformances, parse ~Protocol as suppression | active | src/parser/mod.rs:4146-4163 |
| 9. ~Copyable AST | Add suppress_copyable flag to StructDecl/ClassDecl/EnumDecl | active | src/ast/statement.rs:50-94 |
| 10. ~Copyable symbol_resolver | Skip Copyable protocol check for suppressed types | active | src/symbol_resolver/mod.rs:1634 |
| 11. ~Copyable type_resolver | Skip Copyable requirement check for suppressed types | active | src/type_resolver/mod.rs:8357 |
| 12. ~Copyable ir_gen | Skip copy constructor generation for suppressed types | active | src/ir_gen/ |
| 13. LSP parameters | Function/init params as parameter type (4) not variable (3) | active | src/lsp/server.rs:2406-2448 |
| 14. LSP attributes | #[attr] names as property (9) not type (1) | active | src/lsp/server.rs:2361-2404 |
| 15. LSP modifiers | Declaration/static modifier bits | active | src/lsp/server.rs:2357-2404 |
| 16. LSP preprocessor | Highlight all # directives | active | src/lsp/server.rs:2357-2449 |

## Findings (cited - path:lines)

**Existing preprocessor:**
- `#if`/`#elseif`/`#else`/`#endif`/`#ifdef`/`#ifndef`/`#error`/`#warning` parsed → `src/parser/mod.rs:7848-7909`
- `Condition::Defined` exists but always false → `src/condition_eval.rs:49`
- Build.truss returns empty Vec → `src/trusspm/cli.rs:104`
- `~Copyable` is NOT a special syntax currently; `~` = BitNot, `Copyable` = builtin protocol

**~Copyable in Swift:** `struct Foo: ~Copyable { }` means Foo does not conform to Copyable → move-only (ownership on assignment/parameter passing instead of copy). In Truss, Copyable is a built-in protocol checked by name → `symbol_resolver/mod.rs:1634`, `type_resolver/mod.rs:8357`.

**LSP semantic tokens:** `src/lsp/server.rs:2357-2449`
- Legend: keyword(0), type(1), function(2), variable(3), parameter(4), string(5), number(6), comment(7), operator(8), property(9), macro(10)
- Bugs: params = variable(3) should be parameter(4); #[attr] = type(1) should be property(9); no modifier bits; preprocessor dirs not differentiated

## Decisions (with rationale)

1. **`~Copyable` as `Expression::Unary(BitNot, ...)`** — no boolean flag. Parse `~Copyable` in conformance list as `Unary { operator: BitNot, expression: Type("Copyable"), is_prefix: true, .. }`. The Unary variant already exists in Expression, no new variant needed.
2. **Parser**: in conformance loop, when `~` token before type name → consume `~`, parse inner type, wrap in `Unary(BitNot, ...)`. Only valid on struct/enum/protocol; error on class.
3. **Symbol/Type resolver check sites**: check if any conformance is `Unary(BitNot, Type("Copyable"))` → skip Copyable requirements
4. **IR gen**: skip copy constructor for structs/enums where conformances include the suppression
5. **Protocol `~Copyable`**: means protocol does not require Copyable conformance from its adopters
6. **LSP fixes incremental**: fix parameter type, attribute type, modifier bits, preprocessor highlighting

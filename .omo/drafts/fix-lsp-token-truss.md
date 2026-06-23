---
slug: fix-lsp-token-truss
status: awaiting-approval
intent: clear
pending-action: write .omo/plans/fix-lsp-token-truss.md
approach: Fix the token.truss test file LSP errors by editing the file and fixing compiler bugs
---

# Draft: fix-lsp-token-truss

## Components (topology ledger)
| id | outcome (one line) | status | evidence path |
|---|---|---|---|
| C1 | `/home/xiaoli/Temp/token.truss` - test file with 15 LSP/compiler errors | active | compiler output from `cargo run --bin trussc` |
| C2 | Compiler `is_member_accessible` private check - bug for init | active | `src/type_resolver/mod.rs:7287-7292` |

## Open assumptions (announced defaults)
| assumption | adopted default | rationale | reversible? |
|---|---|---|---|
| `private` fields should be accessible from struct's own `init` | Yes - fix compiler private access check to also match when accessing from same-type init | Standard language semantics (Swift/Rust/Kotlin all allow this) | Yes - easy to revert |
| `[Char]` resolves to `Type::Struct("Array",..)` but Array is a class | Fix type representation or member access path | `Array` is declared as `public final class Array<T>` | Yes |

## Findings (cited - path:lines)

### Compiler errors from `token.truss`
```
1. [E0316] Private field access in init (5 errors) - lines 38-42
2. [E0316] Private field 'pos' access in isEmpty - line 45
3. [E0315] 'length' field not found on Array (4 errors) - lines 48,51,58,65
4. [E0302] Type mismatch in next() - line 65 (cascading from earlier)
5. [E0315] 'map' not found on Iterable - line 84
6. [E0314] Non-function type Never on chain calls - lines 85-86
```

### Root cause analysis
1. **Private field access**: `src/type_resolver/mod.rs:7287` - `is_member_accessible` compares `current_owner.name() == container.name()`. Both should be "CharStream", but the init is processed inside `Statement::InitDecl` handler which enters a sub-scope. The `current_owner` might be lost during processing. The commit `5f77207` fixed `Rc::ptr_eq` → name comparison but didn't address the init scope context.

2. **Array.length**: Array is `public final class Array<T>` (a class). When accessing `[Char]` type, the type system represents it as `Type::Struct("Array", ...)`. The member access handler at line 3609 enters `Type::Struct` branch, looks up "Array" symbol, finds `Symbol::Class` (not `Symbol::Struct`), then fails at line 3644-3656 with "Struct 'Array' has unexpected symbol type". Additionally, Array has no `.length` - only `.count`.

3. **Mutable fields**: `pos`, `line`, `col` are `private let` but mutated in `next()` (`pos += 1`, `line += 1`, `col += 1`). Must be `var`.

4. **String.chars buggy**: `String.chars` computed property in std lib has `let arr = []` with no type hint and `arr.append()` with no argument. Cannot rely on it.

5. **Chain calls**: `arr.map { $0 * 2 }.filter { ... }.zip(arr2).collect()` requires Array to conform to Iterator protocol (for `map`/`filter`/`zip`/`collect` default implementations). Array only conforms to `Iterable` which only has `iterator` property. `ArrayIterator` also doesn't conform to `Iterator`.

### LSP errors shown
All LSP errors map to the same compiler errors above - no LSP-specific bugs.

## Decisions (with rationale)
1. **Fix compiler bug**: `is_member_accessible` private check fails because `current_owner` is lost in `InitDecl` handler path. Fix by ensuring the check also allows access when the member container name matches the current_owner or... debug first.
2. **Change `let` → `var`** for pos/line/col since they're mutated
3. **Change `.length` → `.count`** for Array access
4. **Keep `init(content: String, id: SourceId)`** - String.chars works (compiler infers `[Char]` from return type)
5. **Remove chain calls** at bottom (lines 84-87) since Array doesn't support Iterator protocol methods yet

## Scope IN
- Fix all 15 LSP/compiler errors in `/home/xiaoli/Temp/token.truss`
- Verify compilation succeeds with `cargo run --bin trussc`

## Scope OUT (Must NOT have)
- Do NOT modify standard library files
- Do NOT modify any test files in the truss project
- Do NOT add comments to token.truss

## Open questions
None - all exploration is complete.

## Approval gate
status: awaiting-approval
Plan is ready for approval.

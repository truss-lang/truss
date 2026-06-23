# fix-lsp-token-truss - Work Plan

## TL;DR (For humans)

**What you'll get:** The `token.truss` test file compiles without any LSP errors。

**Why this approach:** There are 3 categories of errors:
1. Compiler bug: `private` fields can't be accessed from struct's own `init` (the `current_owner` is lost in the `InitDecl` handler)
2. File bugs: using `.length` (Array has `.count`), `let` where `var` is needed
3. API mismatch: chain calls assume Array has `map`/`filter`/`zip`/`collect` but Array only conforms to `Iterable`, not `Iterator`

**What it will NOT do:** Won't modify the Truss standard library (String.chars is actually fine, compiler infers `[Char]` from return type). Won't add comments.

**Effort:** Short
**Risk:** Low
**Decisions to sanity-check:** The compiler fix for `private` access in init needs a debug first (add eprintln! to see why current_owner doesn't match container name).

Your next move: Approve so I can start executing.

---

> TL;DR (machine): Short, Low - Fix 15 LSP errors: 1 compiler bug + file edits

## Scope
### Must have
- Fix all LSP errors in `/home/xiaoli/Temp/token.truss`
- File compiles with `cargo run --bin trussc -- /home/xiaoli/Temp/token.truss`
- Each fix committed separately

### Must NOT have
- No modifications to standard library
- No adding comments to the .truss file
- No modifying tests in truss project

## Verification strategy
- Compile the fixed file and verify zero errors

## Todos
- [ ] 1. Debug compiler's private access check for init - add eprintln! to `is_member_accessible` Private branch to see why current_owner != container
- [ ] 2. Fix the compiler bug - ensure `current_owner` is correctly set when members access fields from init
- [ ] 3. Fix token.truss: `.length`→`.count`, `let`→`var` for mutable fields, remove chain calls
- [ ] 4. Verify compilation succeeds
- [ ] 5. Commit each change

## Commit strategy
1. `fix(type_resolver): allow private field access from init`
2. `fix(token.truss): use Array.count instead of length, fix let->var for mutable fields`

## Success criteria
- `cargo run --bin trussc -- /home/xiaoli/Temp/token.truss` succeeds with no errors

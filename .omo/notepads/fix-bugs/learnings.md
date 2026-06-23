# Bug Fix Learning Log

## Phase 0 Results

### C2 - String.init? IR not emitted
- Root cause: In `ir_gen/mod.rs`, `resolve_statement` for `VariableDecl` at line 3826-3870 incorrectly generates auto-accessors for LOCAL variables inside computed property getter bodies.
- The error propagates up through ? and terminates class body iteration.
- `run_all_passes` swallows the error with `let _` (line 481).

### C7 - for-in loop never executes
- The for-in IR gen (lines 4120-4260) is structurally correct when `next_fn` is found.
- Lookup for `next_fn` searches by mangled name. Likely caused by C17 mangling bug.

### C9 - Array.iterator segfault
- Depends on C17 mangling fix.

### C10 - Closures RefCell panic
- Confirmed double-borrow bugs: Assignment handler (lines 6687, 6737, 6754), Call handler (line 9326).

### C14 - Case bindings not scoped
- `match` case binding is CORRECT. Bugs are in `guard case` (line 4926: bindings: _) and `if case` (line 7636: _ => {}).

### C16 - Generic + function type interaction
- First pass sets `closure_expected_type` to unsubstituted types with GenericParam, causing false positive diagnostics.


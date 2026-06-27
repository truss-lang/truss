# Truss Compiler Progress

## Current Phase: Operator定义语法：prefix operator +++

### Completed

5. ✅ **trussup: redesign update to rebuild current toolchain in-place**
   - `update` 现在感知当前版本，不再硬编码 `latest`
   - `nightly`/`latest` → 远程克隆构建；其他版本 → 本地源码重编译
   - 未激活 toolchain 时报错
   - 消除了与 `install_from_remote` 的重复，-59 行

4. ✅ **trussup: support install nightly**
   - `install nightly` clones from GitHub, builds from remote, installs
   - Extracted shared helpers: `download_stdlib()`, `install_from_local()`, `install_from_remote()`, `finalize_install()`
   - `cmd_update()` now reuses `download_stdlib()` instead of inline code

3. ✅ **trussup: auto-add installed toolchains to PATH via ~/.trussup/bin/**
   - `sync_bin_dir()` creates symlinks from `~/.trussup/toolchains/<version>/` to `~/.trussup/bin/`
   - `install` / `use` / `update` all sync binaries after activation
   - `remove` cleans up bin symlinks when removing current version
   - Prints PATH hint when not found in shell config

2. ✅ **trussup: add list-remote command**
   - New `list-remote` subcommand in `Commands` enum
   - `cmd_list_remote()` fetches tags from GitHub via `git ls-remote --tags --refs`
   - Displays versions sorted descending, handles no-tags gracefully

### Completed

1. ✅ **F2: Protocol static var and init constraints**
   - AST: `ProtocolMember::StaticVar` and `ProtocolMember::Init` variants
   - Parser: `static var NAME: TYPE { get set }` and `init(PARAMS) throws?` in protocol bodies
   - Symbol resolver: registers static vars as ProtocolProperty and inits as ProtocolMethod
   - Type resolver: resolves types for static vars, inits, and checks conformance

2. ✅ **F1: Auto-generate LLVM IR for #[autowired] BuiltinType methods**
   - Symbol resolver auto-generates method declarations for #[builtintype] structs conforming to protocols with #[autowired] methods
   - Type resolver accepts autowired requirements for builtin types
   - IR generator emits direct LLVM instructions (add, sub, mul, div, icmp, fcmp, bitwise ops) for builtin type arithmetic/comparison

3. ✅ **F3: Labeled subscript parameters**
   - Label-aware subscript resolution in type resolver
   - Label-aware key registration in IR generator for getter/setter lookup
   - Enables subscript overloads with different parameter labels

4. ✅ **F5+F6: Dynamic object fallback with subscript(dynamicMember:)**
   - Optional-aware fallback for #[dynamicMemberLookup] types
   - Call subscript getter first, check Optional.None, branch to property access
   - Assignment handler already has correct fallback behavior

5. ✅ **F9: Compile-time operator requirement checking**
   - Improved error messages for missing operator implementations
   - Builtin types get direct LLVM instructions; user types must implement methods

6. ✅ **F7: Backtrace for panic and exception propagation**
   - Emits calls to backtrace() and backtrace_symbols_fd() before panic() invocation
   - Stack backtrace printed to stderr before process exit

7. ✅ **F4+F8: LSP and VSCode extension improvements**
   - LSP runs full compiler pipeline (symbol resolve + type check) for diagnostics
   - LSP supports multi-file project analysis when Project.truss is detected
   - VSCode extension: truss.lsp.path setting for custom LSP binary path
   - VSCode extension: enhanced Run command (build + execute)

### LSP Enhancement Features (12 features completed)

1. ✅ **F1: Fix hover function signature display**
   - Reconstruct source-level signature from Statement::FunctionDecl fields
   - Shows `func f(x: Int32) -> Int32` instead of `f: func(Int32) -> Int32`
   - Support for init, subscript, deinit signatures

2. ✅ **F2: Fix hover type/var/class support**
   - lookup_type_in_scopes searches type_env and overloads for type names
   - Handle self/Self/super keywords with descriptive messages
   - Fall back to type_env lookup when symbol lookup fails

3. ✅ **F3: Add semantic tokens for attributes and pragma directives**
   - Detect #[...] attribute pattern and #if/#error pragma directives
   - Emit attribute brackets and names with proper token types
   - Add 'macro' type to semantic token legend

4. ✅ **F4: Add documentSymbol support**
   - Hierarchical DocumentSymbol[] return for file outline
   - Support all declaration types with children

5. ✅ **F5: Add signatureHelp support**
   - Function parameter hints triggered by '('
   - Support overloaded functions with multiple signatures
   - Active parameter calculation by comma counting

6. ✅ **F6: Add inlayHint support**
   - Type annotations for inferred variables (let x: Int32 = 1)
   - Position hints after variable names with resolved types

7. ✅ **F7: Add documentHighlight support**
   - Highlight all occurrences of symbol at cursor
   - Word boundary checking for accurate matching

8. ✅ **F8: Add foldingRange support**
   - Code folding for all brace-delimited blocks
   - Recursive child statement processing

9. ✅ **F9: Fix diagnostic range**
   - Correct end position using span.length
   - Multi-character error underlines

10. ✅ **F10: Add cross-file references support**
    - Find All References across open documents and project files
    - Canonical symbol name lookup for accurate matching

11. ✅ **F11: Improve go-to-definition**
    - Member access support (obj.method resolves to method)
    - Cross-file stdlib path resolution
    - Scope parent chain traversal

12. ✅ **F12: Improve completion**
    - Prefix filtering (type 'pri' shows only matching items)
    - 14 new snippet entries (associatedtype, prefix func, etc.)
    - All symbol types in stdlib completion
    - Protocol member completions

### Fixed
1. ✅ **Fix: Stdlib forward reference resolution crash**
   - Register protocol types in `type_env` during `register_symbols` for `ProtocolDecl` (was only in `process_decl`)
   - Ensures protocol types like `Iterable`/`Clonable` referenced across files resolve correctly regardless of processing order

2. ✅ **Fix: Iterator default implementation methods not generated**
   - Add `self`/`Self` type registration in type resolver's protocol method function scope
   - Iterator protocol default methods (`map`, `filter`, `zip`, `enumerated`, `collect`) now have their `ty` field properly filled
   - Verified via `--ir` output: all 6 default implementation functions are now generated

3. ✅ **Fix: Closure shorthand inference in generic method calls**
   - Set `closure_expected_type` before calling `infer_type`/`infer_expression_type` on closure args in generic mapping loop
   - Enables trailing closures with `$0`/`$1` shorthand syntax in protocol default method calls

4. ✅ **Fix: Protocol associated type resolution in protocol method lookup**
   - Modified `lookup_protocol_method` to find protocol conformance expression and substitute associated types
   - Uses conformance expressions' type parameters to map protocol generics to concrete types
   - Added `Type::Closure` variant handling in `substitute_generic_params`

5. ✅ **Fix: Generic type param propagation in member access**
   - Default generic type params filled when resolving types without explicit type arguments (e.g., `ArrayIterator` → `ArrayIterator<T>`)
   - Substitute concrete generic params during member access for properties, methods, and protocol methods
   - Fixed `infer_protocol_generic_params` to skip protocol default methods (only compare required methods)

6. ✅ **Fix: Method chain parsing after trailing closure**
   - Added `parse_trailing_chain` in parser to continue `.member` access after trailing closure
   - `arr.iterator.map { }.filter { }.zip().collect()` now parsed as single chain expression

7. ✅ **Fix: Return type struct generic params inference from call context**
   - Infer `_MapIterator<I,F,T>` type params from Self type and generic mapping in call handler
   - Fallback in `lookup_protocol_method` to `infer_protocol_generic_params` for resolving associated types

### Fixed
8. ✅ **Fix: Type resolver two-pass (build symbol table before resolving symbol references)**
   - Added `has_parameters` field to `Symbol::EnumCase` to detect when enum case parameter types haven't been populated yet
   - MemberAccess handler for `Type::Enum` returns `None` (defers to second pass) when `parameter_types` is empty but case actually has parameters
   - Moved closure shorthand parameter setup from `process_decl` to `resolve_statement`'s VariableDecl handler
   - Eliminates `[E0314] Cannot call non-function type E` for forward enum references

9. ✅ **Fix: AST dump infinite loop**
   - Custom `Debug` implementation for `Scope` that doesn't recursively follow the `parent` chain
   - Shows `parent` as `Some("..")` instead of infinite recursion
   - `cargo run -- -a file.truss` now terminates normally

10. ✅ **LSP: Enum case constructor hover and signatureHelp**
    - Hover over enum case with parameters shows function-like signature (e.g., `A(Int32) -> E`)
    - signatureHelp triggered by `Enum.Case(` shows parameter type hints
    - Stdlib type completions show "type" instead of "builtin type"

### Known Issues
- Std `Project.truss` needs `version: "1.0.0"` for project-structure detection to work
- Backtrace symbols show addresses when binary lacks debug info
- Optional type does not implement `==`/`!=` operators (affects `if let` comparisons)

# Truss

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![LLVM](https://img.shields.io/badge/LLVM-22.1-purple.svg?logo=llvm)](https://llvm.org/)

Truss is a general-purpose programming language built from scratch, implemented in Rust with an LLVM 22.1 backend.

Its syntax is inspired by Swift and Rust. The language supports structs, classes (single inheritance + vtable + reference counting), enums (with associated values / ADTs), protocols (existential containers, **no monomorphization**), generics, extensions, pattern matching, computed properties, access control, and more.

## Compiler Pipeline

```
Source  ─▶  Lexer  ─▶  Parser  ─▶  SymbolResolver  ─▶  TypeResolver  ─▶  IRGenerator  ─▶  LLVM IR
```

| Phase | Directory | Description |
|-------|-----------|-------------|
| **Lexer** | `src/lexer/` | `CharStream` → `Lexer` → `Vec<Token>` |
| **Parser** | `src/parser/` | Pratt + recursive descent parsing into AST (`Program`) |
| **SymbolResolver** | `src/symbol_resolver/` | Name resolution, scope hierarchy, duplicate/shadow detection |
| **TypeResolver** | `src/type_resolver/` | Type inference and checking, generic constraints, overload resolution |
| **IRGenerator** | `src/ir_gen/` | LLVM IR generation via Inkwell (multi-pass) |

## Features

- **Structs** — Mapped to LLVM opaque structs, field access via GEP
- **Classes** — Single inheritance, vtable + reference counting, `open` for subclassing
- **Enums** — Tagged unions with associated values
- **Protocols** — Existential containers (value pointer + protocol witness table + vtable), **no monomorphization**
- **Generics** — Generic functions and type parameters, `where` clauses (separated by `&&`)
- **Extensions** — Add methods to existing types
- **Computed Properties** — getter / setter / willSet / didSet
- **Pattern Matching** — `match` expressions, `if case` / `guard case` bindings
- **Access Control** — `open` / `public` / `package` / `internal` / `fileprivate` / `private`
- **FFI** — `extern "C"` for foreign function declarations
- **Existential Types** — `any Protocol` and compound `any P1 & P2`

## Example

```swift
protocol Drawable {
    func draw()
}

protocol Runnable {
    func run()
}

class Shape: Drawable, Runnable {
    func draw() { let a = 1 }
    func run()  { let b = 2 }
}

func main() {
    let s = Shape()
    s.draw()

    let d: any Drawable & Runnable = s
    d.run()
}
```

## Usage

```bash
# Compile a source file
cargo run -- path/to/source.truss

# Dump tokens
cargo run -- -t path/to/source.truss

# Dump AST
cargo run -- -a path/to/source.truss

# Dump tokens + AST
cargo run -- -i path/to/source.truss

# Output LLVM IR
cargo run -- --ir path/to/source.truss
```

## Running Tests

```bash
cargo test
```

The test suite covers all compiler phases (~18,000+ lines of test code):

| Test file | Scope |
|-----------|-------|
| `tests/lexer.rs` | Lexer: literals, identifiers, keywords |
| `tests/parser.rs` | Parser: functions, expressions, control flow, FFI |
| `tests/symbol_resolver.rs` | Symbol resolution: scoping, shadowing, Self |
| `tests/type_resolver.rs` | Type inference and checking, error diagnostics |
| `tests/ir_gen.rs` | LLVM IR generation: structs, classes, protocols, etc. |

## Project Structure

| Directory | Description |
|-----------|-------------|
| `src/ast/` | AST node definitions (Program, Expression, Statement) |
| `src/lexer/` | Lexer and token definitions |
| `src/parser/` | Recursive descent + Pratt parser |
| `src/symbol_resolver/` | Symbol resolution |
| `src/type_resolver/` | Type inference and checking |
| `src/ir_gen/` | LLVM IR code generation |
| `src/diag/` | Diagnostic system (error/warning codes and formatting) |
| `src/krate/` | Package / module system |
| `src/scope/` | Scope implementation (symbol table, type env, overloads) |
| `src/symbol/` | Symbol enum definitions |
| `src/types/` | Type system definitions |
| `tests/` | Integration tests |

## Tech Stack

- **Rust** edition 2024
- **LLVM 22.1** — via [Inkwell](https://github.com/TheDan64/inkwell) bindings
- **Diagnostics** — powered by [`duck-diagnostic`](https://crates.io/crates/duck-diagnostic)

## License

MIT

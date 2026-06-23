
# 2026-06-20: Fix hover function signature display

## Problem
`symbol_type_string` in `src/lsp/server.rs` used `Type::Display` to format function types, which produced `f: func(Int32) -> Int32` — dropping parameter labels and names.

## Solution
Rewrote `symbol_type_string` to reconstruct source-level signatures directly from `Statement` fields:

- **FunctionDecl**: `func name(label p1: T1, p2: T2) -> RetType` — uses `name.value`, iterates `parameters` for `label`+`name`+`ty`, extracts return type from `Type::Function(_, ret, _, _)` via the decl's `ty` field
- **InitDecl**: `init(label p1: T1, p2: T2)` — same parameter logic, no return type; handles `is_failable` for `init?`
- **SubscriptDecl**: `subscript(label p1: T1, p2: T2) -> RetType` — uses `subscript` keyword, same parameter/return logic
- **VariableDecl/StructDecl/ClassDecl/EnumDecl/ProtocolDecl**: keep using `ty` field (Type::Display) — unchanged behavior
- **DeinitDecl**: returns `"deinit"` directly

## handle_hover change
Added `is_func_like` check using `matches!` on `Symbol::Function | StructMethod | ClassMethod | ProtocolMethod | StructSubscript | ClassSubscript | ProtocolSubscript`. For function-like symbols, the type_str (now the full source signature) is used directly instead of `name: type` format. For other symbols, `name: type` is preserved.

## Key structures
- `Parameter { label: Option<Box<Token>>, name: Box<Token>, type_expression: Rc<RefCell<Expression>>, ty: Option<Rc<RefCell<Type>>> }`
- `Token { value: String, ... }` — simple `.value` yields the identifier name
- `Type::Display` for Function produces `func(ParamType) -> RetType` without labels — avoid for function-level signatures, OK for individual param types

## Type usage
- Imported `crate::types::Type` for pattern matching on `Type::Function(_, ret, _, _)`
- Parameter types displayed via `pty.borrow().to_string()` (individual Type Display is fine)
- Return types extracted from `Type::Function` inside the decl's `ty` field

---
name: compiler-new-syntax-phase
description: 在 Truss 编译器中新增语法功能的逐阶段实现方法（Parser → SymbolResolver → TypeResolver → IRGen），含 import/模块系统的实现模式、$0/$1 简写参数、递归表达式搜索
source: auto-skill
extracted_at: '2026-06-03T14:38:38.463Z'
---

# 新增语法功能的四阶段实现法

> 经验来源: 实现 extension 关键字（Swift-like）、Self 类型关键字、模式匹配语法（match/guard/fallthrough/break/.xxx shorthand）的完整过程。
> 覆盖了 Lexer/Token/AST → Parser → SymbolResolver → TypeResolver → IRGen 全链路。
> 更新于 2026-05-29：补充了目标符号 scope 查找、多 borrow 处理、witness table 扩展等实践经验。
> 更新于 2026-05-29：补充了模式匹配语法（Pattern 扩展、match 表达式、guard 语句、&& 条件连接、fallthrough/break 处理）的实践经验。
> 更新于 2026-05-29：补充了 match 标量类型匹配（整数/浮点数）、IR gen 错误处理、空 case body 的经验。
> 更新于 2026-05-30：补充了多模块支持（module 关键字、dotted path desugar、Crate/Module children）的实践经验。
> 更新于 2026-05-30：补充了 import 语句的四种形式（Module/Member/Wildcard）的解析与语义设计。

Truss 编译器采用多阶段架构：Lexer/Token → Parser → SymbolResolver → TypeResolver → IRGen。新增一个语法功能需要按顺序完成所有阶段。

## 阶段顺序

实现新功能时，一次只做一个阶段，完成后等待用户手动 commit 后再继续下一个。

1. **Phase 1: Parser** — 修改 lexer/token、AST nodes、parser
2. **Phase 2: Symbol Resolver** — symbol 注册与解析
3. **Phase 3: Type Resolver** — 类型检查与推导
4. **Phase 4: IR Gen** — LLVM IR 代码生成

## Phase 1 具体步骤

### 1. Lexer / Token
- `src/lexer/token.rs` 中 `KeywordType` enum 添加新关键字
- `code()` 方法中添加对应的字符串映射

### 2. AST — Statement
- `src/ast/statement.rs` 中 `Statement` enum 添加新变体
- `token()` 方法中添加对应 match arm
- `modifiers()` 方法中根据是否需要 modifier 添加或不加

### 3. AST — Expression（如果需要新的表达式类型）
- `src/ast/expression.rs` 中 `Expression` enum 添加新变体
- `get_ty()`、`get_ty_ref()`、`get_ty_mut_ref()`、`token()` 方法中添加 match arm
- 注意 `get_ty_ref()` 和 `get_ty_mut_ref()` 是两个独立的 match，都需要覆盖

### 4. Parser
- `src/parser/mod.rs` 中 `parse_statement` 方法添加 `KeywordType` 分支
- 实现对应的 `parse_xxx` 方法（参考已有 `parse_struct_decl`、`parse_class_decl` 等模式）
- 如果需要在类型表达式中使用新关键字，在 `parse_type_expression` 中添加对应处理（参考 `Any` 关键字的处理位置）
- `parse_brace_body()` 可用于复用花括号 body 的解析逻辑

### 5. 占位处理（让编译通过）

在其他阶段中添加默认/占位处理，确保编译通过：

- **type_resolver** 的 `infer_type` 中为新 `Expression` 变体添加 match arm
- 如果无法立即实现完整逻辑，返回一个合理的占位（如 `return None`）
- 其他 match-on-Expression 的地方如有编译错误也需要添加
- **注意**: `SelfType` 在 `infer_type` 中需要返回 `Rc<RefCell<Type>>`（而非 `Option`），无法直接 `return None`，需要用类似 `if let Some(t) = t { ... } else { return None }` 的模式

### 6. 测试
- 在 `tests/parser.rs` 中按相同模式编写解析测试

### 7. 模式匹配语法特殊模式

模式匹配（match/guard/case/.xxx shorthand）的 parser 实现有几个特殊模式：

#### 7.1 Pattern 扩展策略

Pattern 在当前 AST 中是独立于 Expression 的类型，需要新增变体时：

```rust
pub enum Pattern {
    Identifier(Box<Token>),
    Tuple(Vec<Pattern>),
    Ignore,
    ValueBinding(Box<Pattern>),   // let p
    EnumCase {                     // .caseName(p1, p2)
        case_name: Box<Token>,
        bindings: Vec<Pattern>,
    },
    Expr(Rc<RefCell<Expression>>), // literal: 1, "hello", true
}
```

`parse_pattern()` 的 dispatch 顺序需要注意：
1. 先检查 `let`/`var` 关键字 → `ValueBinding`
2. 再检查 `.` 操作符 → `EnumCase`
3. 再处理 `(` → `Tuple`
4. 再处理 identifier / `_` → `Identifier` / `Ignore`
5. 最后处理 literal tokens（`IntegerLiteral`、`BooleanLiteral` 等）→ `Expr`

**关键**：literal tokens 作为 Pattern 时不能直接 `self.next()` 消耗掉，需要 `self.index -= 1` 退回后通过 `parse_expression()` 重新解析为 Expression，否则 Token 类型枚举匹配优先级问题会导致解析错误。

#### 7.2 parse_case_expression 支持 .xxx 简写

`case` 表达式支持两种形式：
```swift
case TypeName.caseName(bindings) = expr   // 完整形式
case .caseName(bindings) = expr           // 简写（enum_type 为 None）
```

解析时先 peek 下一个 token：
- 如果是 `.` → 简写路径，enum_type = None
- 如果是 identifier → 完整路径，读 identifier + `.` + caseName

对应的 AST field 从 `enum_type: Box<Token>` 改为 `enum_type: Option<Box<Token>>`，这个变化会影响到所有下游阶段（symbol_resolver、type_resolver、ir_gen），需要逐一适配。

#### 7.3 && 条件连接与优先级控制

`if case .x = a && case .y = b { }` 需要被解析为 `(case .x = a) && (case .y = b)`。

**陷阱**：`parse_case_expression` 内部调用 `parse_expression()` 解析 `=` 右侧表达式时会**贪心地**把 `&&` 也吃掉，因为 `&&` 的优先级 `And > Assignment`。

**解决方案**：使用 `parse_binary(Precedence::And)` 替代 `parse_expression()`，让 `&&`（优先级 `And`）不被 `>` 条件吞入：

```rust
// 在 parse_case_expression 中:
let expression = self.parse_binary(Precedence::And)?;
// 而不是 self.parse_expression()?
```

这样 `case .x = a && case .y = b` 中：
- 第一个 case 的 RHS 只解析到 `a`
- `&&` 在外部 `parse_binary(Assignment)` 中作为条件连接符处理
- 第二个 `case .y = b` 作为 `&&` 的 RHS

#### 7.4 match 表达式（Expression 而非 Statement）

`match expr { cases }` 是表达式（类似 `if`），在 `parse_primary()` 中通过 `KeywordType::Match` 分派。

**match body 中的 case body 需要处理三种形式：**
1. 块表达式 `{ ... }` — 直接调用 `parse_block()`
2. 关键字语句 `fallthrough` / `break` — 解析为 Statement，包裹在 `Expression::Block` 中
3. 普通表达式 — 直接 `parse_expression()`

```rust
fn parse_match_case_body(&mut self) -> Result<Expression, ()> {
    if let Some(t) = self.peek()
        && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
    {
        return self.parse_block();
    }
    if let Some(t) = self.peek() {
        if let TokenType::Keyword { keyword } = &t.ty {
            match keyword {
                KeywordType::Fallthrough | KeywordType::Break => {
                    let stmt = self.parse_fallthrough()?; // 或 self.parse_break()
                    return Ok(Expression::Block {
                        statements: vec![Rc::new(RefCell::new(stmt))],
                        scope: None,
                    });
                }
                _ => {}
            }
        }
    }
    self.parse_expression()
}
```

#### 7.5 guard 语句（Statement 而非 Expression）

`guard condition else { body }` 是 statement（因为 else 必须退出作用域）。

解析非常简单：`condition` 作为表达式解析，`else` 关键字后跟 `else_body` 表达式。

#### 7.6 fallthrough / break 作为独立 Statement

`fallthrough` 和 `break` 是 `Statement::Fallthrough` 和 `Statement::Break` 变体，不需要参数。它们在 `parse_statement()` 中分派，在 match case body 中通过 `parse_match_case_body` 间接调用。

#### 7.7 Option<Box<Token>> 的连锁反应

当把 AST 字段从 `Box<Token>` 改为 `Option<Box<Token>>` 时，需要修改的**所有位置**：

- `src/ast/expression.rs` — 枚举变体定义、`get_ty()`、`get_ty_ref()`、`get_ty_mut_ref()`、`token()`
- `src/parser/mod.rs` — 解析代码构造 AST
- `src/symbol_resolver/mod.rs` — 所有 `Expression::Case { enum_type, .. }` 的 match 臂
- `src/type_resolver/mod.rs` — `infer_type` 和 `resolve_statement` 中的 match 臂
- `src/ir_gen/mod.rs` — 代码生成中的 match 臂
- `tests/parser.rs` — 所有断言 `enum_type.value` → `enum_type.as_ref().unwrap().value`
- 其他测试文件中的类似断言

#### 7.8 Pattern 在 symbol_resolver 中的默认处理

`resolve_pattern_bindings` 函数必须覆盖所有 Pattern 变体，新增变体时：

```rust
Pattern::ValueBinding(inner) => {
    Self::resolve_pattern_bindings(&[*(inner.clone())], resolver);
}
Pattern::EnumCase { bindings, .. } => {
    Self::resolve_pattern_bindings(bindings, resolver);
}
Pattern::Expr(_) => {} // 字面量模式不绑定符号
```

注意 `&[*(inner.clone())]`：`inner` 是 `Box<Pattern>`，解引用后需要创建单元素 slice。

### 7.9 Type Resolver 中模式匹配的特殊模式

#### 7.9.1 Expression::Case 简写的类型推断

当 `enum_type` 为 `None`（`.caseName` 简写）时，需要从 RHS 表达式的类型推断 enum 类型：

```rust
Expression::Case { enum_type: enum_type_opt, case_name, expression, ty, .. } => {
    let expr_ty = self.infer_type(expression.clone());

    if let Some(current_scope) = &self.current_scope {
        if let Some(enum_type) = enum_type_opt.as_ref() {
            // 完整路径：TypeName.caseName — 直接查找
            ...
        } else if let Some(expr_ty) = expr_ty.as_ref() {
            // 简写 .caseName — 从表达式类型推断
            if let Type::Enum(enum_name, _) = &*expr_ty.borrow() {
                // 在 scope 中查找 enum 定义，验证 case 存在
                ...
            }
        }
    }
}
```

#### 7.9.2 Expression::If 中简写 binding 的类型绑定

当 `if case .some(val) = x { ... }` 的 `enum_type` 为 `None` 时，`binding_types` 的提取也需要通过 RHS 表达式类型推断：

```rust
let binding_types = {
    let cond = condition.borrow();
    if let Expression::Case { enum_type, case_name, bindings, expression, .. } = &*cond {
        if !bindings.is_empty() {
            if let Some(type_name) = enum_type.as_ref() {
                // 完整路径
                self.get_enum_case_parameter_types(&type_name.value, &case_name.value)
            } else if let Some(expr_ty) = self.infer_type(expression.clone()) {
                // 简写：从表达式类型推断 enum → 获取 case parameter types
                self.resolve_enum_case_from_type(&expr_ty, None, case_name, bindings)
            } else {
                None
            }
        } else { None }
    } else { None }
};
```

需要辅助方法 `resolve_enum_case_from_type` 统一处理完整路径和简写路径：

```rust
fn resolve_enum_case_from_type(
    &self,
    expr_ty: &Rc<RefCell<Type>>,
    enum_name: Option<&str>,
    case_name: &Token,
    _bindings: &[Pattern],
) -> Option<Vec<Rc<RefCell<Type>>>> {
    if let Some(name) = enum_name {
        self.get_enum_case_parameter_types(name, &case_name.value)
    } else if let Type::Enum(enum_name, _) = &*expr_ty.borrow() {
        self.get_enum_case_parameter_types(enum_name, &case_name.value)
    } else {
        None
    }
}
```

#### 7.9.3 Statement::Guard 的类型检查

Guard 的 pattern binding 注册在**当前 scope**（而非新 scope），因为 binding 需要在 guard 之后的代码中可见。binding 注册发生在 else body 解析**之后**，确保 else body 内看不到 guard binding：

```rust
Statement::Guard { condition, else_body, .. } => {
    // 1. 解析条件
    let _cond_ty = self.infer_type(condition.clone());
    
    // 2. 提取 binding types（支持简写）
    let binding_types = { ... };
    
    // 3. 解析 else body — 此时 binding 尚未注册，else body 看不到它们
    self.resolve_block_expression(else_body.clone());
    
    // 4. 注册 bindings 到当前 scope — 后续语句可见
    if let Some(ref param_types) = binding_types {
        if let Expression::Case { bindings, .. } = &*condition.borrow() {
            let current_scope = self.current_scope.clone();
            if let Some(scope) = current_scope {
                Self::set_binding_types(bindings, param_types, &scope);
            }
        }
    }
}
```

关键点：else body 的 scope 是 `Expression::Block` 创建的独立 scope（子 scope），所以即使 binding 先注册，子 scope 也能看到父 scope 的绑定。但**为了避免 else body 意外访问 guard binding**，必须在 else body 解析**之后**再注册 binding。

#### 7.9.4 Expression::Match 的类型检查

Match 为每个 case 创建独立 scope。对于 `Pattern::EnumCase` pattern，需要从 subject 表达式的 enum 类型获取 case parameter types，然后通过 `set_binding_types` 注册到 case scope：

```rust
Expression::Match { value, cases, ty, .. } => {
    let subject_ty = self.infer_type(value.clone());
    let mut match_ty = Rc::new(RefCell::new(Type::Void));

    for case in cases {
        let case_scope = Rc::new(RefCell::new(Scope::new(self.current_scope.clone())));
        self.enter_scope(case_scope.clone());

        // EnumCase: 从 subject type 获取 case parameter types 并绑定
        if let Pattern::EnumCase { case_name, bindings, .. } = case.pattern.as_ref() {
            if !bindings.is_empty() {
                if let Some(ref subject_ty) = subject_ty {
                    if let Type::Enum(enum_name, _) = &*subject_ty.borrow() {
                        let param_types = self.get_enum_case_parameter_types(
                            enum_name, &case_name.value);
                        if let Some(ref param_types) = param_types {
                            Self::set_binding_types(bindings, param_types, &case_scope);
                        }
                    }
                }
            }
        }
        // ValueBinding: 绑定整个 subject type（如 `case let x:`）
        if let Pattern::ValueBinding(inner) = case.pattern.as_ref() {
            if let Pattern::Identifier(name) = inner.as_ref() {
                if let Some(ref subject_ty) = subject_ty {
                    case_scope.borrow_mut().set_type(
                        name.value.clone(), subject_ty.clone());
                }
            }
        }

        // 解析 guard condition
        if let Some(guard) = &case.guard { self.infer_type(guard.clone()); }

        // 解析 body，检查分支类型一致性
        let body_ty = self.infer_type(case.body.clone())
            .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));
        if *match_ty.borrow() == Type::Void {
            match_ty = body_ty;
        } else if match_ty.borrow().clone() != body_ty.borrow().clone() {
            self.emit_error(TrussDiagnosticCode::BranchTypeMismatch, ...);
        }

        self.leave_scope();
    }
    *ty = Some(match_ty.clone());
    match_ty
}
```

关键点：
- **每个 case 独立 scope** — 不同 case 中的 binding 不会互相干扰
- **EnumCase + ValueBinding 都要处理** — case pattern 可能同时包含两者
- **guard condition 在 body 之前解析** — guard 中的表达式可能引用 pattern binding
- **分支类型一致性** — match 作为表达式时，所有分支的类型应该一致（类似 if 表达式）

#### 7.9.5 set_binding_types 对 ValueBinding 的支持

`set_binding_types` 必须支持 `Pattern::ValueBinding(inner)`，因为 match/guard 中的 binding 可能是 `let x` 形式（以 ValueBinding 包裹 Identifier）：

```rust
fn set_binding_types(bindings: &[Pattern], param_types: &[Rc<RefCell<Type>>], scope: &Rc<RefCell<Scope>>) {
    let mut scope_ref = scope.borrow_mut();
    for (i, binding) in bindings.iter().enumerate() {
        if i >= param_types.len() { break; }
        match binding {
            Pattern::Identifier(name) => {
                if name.value != "_" {
                    scope_ref.set_type(name.value.clone(), param_types[i].clone());
                }
            }
            Pattern::ValueBinding(inner) => {
                if let Pattern::Identifier(name) = inner.as_ref() {
                    if name.value != "_" {
                        scope_ref.set_type(name.value.clone(), param_types[i].clone());
                    }
                }
            }
            _ => {}
        }
    }
}
```

## 后续阶段要点

### Phase 2: Symbol Resolver (`src/symbol_resolver/mod.rs`)

**`register_symbols` 中注册符号：**
- 通过 `self.current_scope.as_ref().and_then(|s| s.borrow().get_symbol(&name))` 查找已有的目标类型符号
- 如果目标未找到，emit 错误并 return
- 通过目标符号的 `get_decl()` 获取其 Statement，再从中拿到 `scope`（即 struct/class 内部作用域）
- 切换到目标 scope 后再创建方法符号并注册：
  - 方法符号使用 `Symbol::StructMethod` / `Symbol::ClassMethod`（带 `WeakSymbol` parent 引用指向目标）
  - 将方法符号 push 到目标的 `methods` 列表
  - 调用 `self.enter(symbol, token)` 进入当前 scope
  - 对方法 body 递归调用 `register_symbols` 注册内部符号
- **Struct/Class** 额外处理 `InitDecl`（push 到 `constructors` 列表）和 `DeinitDecl`（设置 `destrcutor`，检查重复）
- **Enum** 只处理方法（使用 `StructMethod` symbol，enum 的方法和 struct 共用同一种 symbol 类型）
- **Protocol** 使用 `ProtocolMethod` symbol
- 完成后恢复 `self.current_scope`
- **注意**: 外部 `if let` 中不要多余的 destructure（如 `body: method_body`），否则会产生 unused variable 警告 — 只需要 `name: method_name` 就够了，因为内层会重新 borrow

```rust
// 查找目标类型
let Some(target_sym) = self.current_scope
    .as_ref()
    .and_then(|scope| scope.borrow().get_symbol(&type_name.value))
else {
    self.emit_error(TrussDiagnosticCode::SymbolError, "Cannot extend undefined type", &type_name);
    return;
};

// 获取目标 scope
let target_scope = target_sym.borrow().get_decl().ok().flatten()
    .and_then(|d| {
        let stmt = d.borrow();
        match &*stmt {
            Statement::StructDecl { scope, .. }
            | Statement::ClassDecl { scope, .. }
            | Statement::EnumDecl { scope, .. }
            | Statement::ProtocolDecl { scope, .. } => scope.clone(),
            _ => None,
        }
    });

// 在目标 scope 下注册方法
let saved = self.current_scope.clone();
if let Some(ref ts) = target_scope {
    self.current_scope = Some(ts.clone());
}
match &mut *target_sym.borrow_mut() {
    Symbol::Struct { methods, .. } | Symbol::Class { methods, .. } => {
        for field_stmt in body {
            if let Statement::FunctionDecl { name, .. } = &*field_stmt.borrow() {
                let method_sym = Rc::new(RefCell::new(Symbol::StructMethod {
                    name: name.value.clone(),
                    parent: WeakSymbol(Rc::downgrade(&target_sym)),
                    decl: Some(field_stmt.clone()),
                }));
                methods.push(method_sym.clone());
                self.enter(method_sym, name);
                // 递归注册方法体内的符号
            }
        }
    }
    _ => {}
}
self.current_scope = saved;
```

**`resolve_statement` 中解析成员：**
- 同样查找目标 scope（通过 `get_symbol` → `get_decl` → 从 Statement 中取 scope）
- 切换到目标 scope 后，遍历 body 调用 `resolve_statement(stmt.clone())`
- 这使得 `self` 在方法体内可解析
- 如果目标 scope 不存在（如未找到类型），至少也要 resolve body 里已有的 statement，避免完全跳过

### Phase 3: Type Resolver (`src/type_resolver/mod.rs`)

**`process_decl` 中处理类型声明：**
- 查找目标符号 (`name_table.get(&type_name.value)`)
- 确定目标类型 Type（`Type::Struct`/`Type::Class`/`Type::Enum`/`Type::Protocol`）
- 解析 protocol conformances
- 获取目标 scope（通过符号的 `get_decl()` → Statement 的 scope 字段）
- 进入目标 scope，注入 `"Self"` 类型（`scope.set_type("Self".to_string(), target_ty)`）
- 遍历 extension body，对每个 FunctionDecl：
  - 处理方法参数类型推导（参考 StructDecl 中 process_decl 的 pattern）
  - 设置 `ty` 字段为推导出的 Function type
  - 递归调用 `process_decl(stmt.clone())`
- 离开 scope

**`resolve_statement`：**
- 查找目标 scope（方式同 symbol resolver）
- 进入目标 scope，resolve 所有 body 语句
- 离开 scope
- **注意**: 如果目标 scope 不存在，fallback 为不进入任何 scope，直接 resolve body 中的 statement

**`SelfType` 在 `infer_type` 中的解析：**
```rust
Expression::SelfType { ty, .. } => {
    let t = self.current_scope
        .as_ref()?
        .borrow()
        .get_type("Self");
    if let Some(t) = t {
        *ty = Some(t.clone());
        t
    } else {
        return None;
    }
}
```

### Phase 4: IR Gen (`src/ir_gen/mod.rs`)

- 扩展方法生成为 `<TypeName>.<methodName>` 格式的函数（与 struct/class 内定义的方法命名一致）
- 静态分发的函数直接 `call`
- 动态分发（protocol requirement）通过 witness table 实现
- `create_protocol_witness_tables` 需扩展为也扫描 ExtensionDecl（当前只扫描 StructDecl/ClassDecl）
- ExtensionDecl 的 `resolve_statement` 中设置 `current_struct` 为扩展目标类型名（参考 StructDecl/ClassDecl 的 pattern）

### 查找目标符号的 scope

跨阶段通用的模式：要从一个已知的 type symbol 拿到它的内部 scope（用于在其中注册成员/解析 self），需要：

```rust
// 1. 从 current_scope 中查找目标符号
let Some(target_sym) = self.current_scope
    .as_ref()
    .and_then(|scope| scope.borrow().get_symbol(&type_name.value))
else {
    // emit error 或 return
    return;
};

// 2. 从符号的 decl 中获取 Statement → 再取 scope
let target_scope = target_sym.borrow().get_decl().ok().flatten()
    .and_then(|d| {
        let stmt = d.borrow();
        match &*stmt {
            Statement::StructDecl { scope, .. }
            | Statement::ClassDecl { scope, .. }
            | Statement::EnumDecl { scope, .. }
            | Statement::ProtocolDecl { scope, .. } => scope.clone(),
            _ => None,
        }
    });
```

### 避免对 target_sym 的 double borrow

当需要同时 `&mut *target_sym.borrow_mut()`（如修改 methods 列表）和创建带 parent 引用的子符号时：

```rust
// 先 clone 一份，再用 clone 去做 mutable borrow
let target_symbol_rc = target_sym.clone();
match &mut *target_symbol_rc.borrow_mut() {
    Symbol::Struct { methods, .. } => {
        let method_sym = Rc::new(RefCell::new(Symbol::StructMethod {
            parent: WeakSymbol(Rc::downgrade(&target_sym)), // 用原始 target_sym
            ..
        }));
        methods.push(method_sym.clone());
    }
    _ => {}
}
```

关键点：`WeakSymbol(Rc::downgrade(&target_sym))` 用的是**未 clone 的原始引用**，而 mutable borrow 用的是 `target_symbol_rc.clone()`。这两者指向同一个 Rc，但引用计数层面不会冲突。

### 协议 Witness Table（跨阶段）

协议 witness table 的构建分三阶段：

1. **Symbol Resolver**: extension 中的方法注册为目标类型的 `StructMethod`/`ProtocolMethod`，与 struct/class body 内方法使用同样的 symbol 类型
2. **Type Resolver**: 方法类型推导与 struct/class 内方法一致
3. **IR Gen**:
   - `create_function_declarations`: 用 `<TypeName>.<methodName>` 命名（含 self 参数）
   - `create_protocol_witness_tables`: 当前只扫描 StructDecl/ClassDecl，需扩展为也扫描 ExtensionDecl（取 `type_name` + `conformances`）
   - `compute_protocol_witness_table_entries`: 从 Protocol 的 symbol 中获取 requirements 列表，返回 `Vec<(name, kind)>`
   - witness table 中是函数指针，通过 `format!("{}.{}", type_name, entry_name)` 在 module 中查找对应的 FunctionValue

### 模式匹配 IR 生成

#### Expression::Match 的 IR 生成

Match 是表达式（Expression），在 `resolve_expression` 中处理。核心模式是**多分支 tag 比较链**：

```rust
Expression::Match { value, cases, .. } => {
    let fn_val = self.builder.get_insert_block().unwrap().get_parent().unwrap();
    let exit_bb = self.context.append_basic_block(fn_val, "match_exit");

    // 1. Resolve subject 值并保存
    let subject_val = self.resolve_expression(value.clone())?.unwrap();
    let subject_alloca = self.builder.build_alloca(subject_val.get_type(), "")?;
    self.builder.build_store(subject_alloca, subject_val)?;

    // 2. 通过 subject 表达式推断 enum 类型名（type resolver 已标注在 ty 字段上）
    let enum_name = self.get_enum_name_from_expr_string(value.clone())
        .unwrap_or_default();

    // 3. 创建基本块
    for _ in cases.iter() {
        all_body_bbs.push(self.context.append_basic_block(fn_val, "case_body"));
        all_check_bbs.push(self.context.append_basic_block(fn_val, "case_check"));
    }

    // 4. 为每个 case 生成比较 + payload 提取
    for (i, case) in cases.iter().enumerate() {
        let body_bb = all_body_bbs[i];
        let next_bb = if i + 1 < all_body_bbs.len() { all_body_bbs[i + 1] } else { exit_bb };

        // EnumCase pattern → tag 比较
        if let Some(idx) = case_idx {
            // icmp eq tag, expected_tag
            self.builder.build_conditional_branch(match_result, body_bb, next_bb)?;
        } else {
            // Default/wildcard → 无条件匹配
            self.builder.build_unconditional_branch(body_bb)?;
        }

        // 在 body_bb 中提取 payload 并绑定变量
        self.builder.position_at_end(body_bb);
        self.enter_scope();

        // 为每个 binding 创建 alloca + store
        for (j, binding) in bindings.iter().enumerate() {
            let field_ptr = self.builder.build_struct_gep(case_payload_struct_ty, ...)?;
            let field_val = self.builder.build_load(field_ty, field_ptr, "")?;
            let var_ptr = self.builder.build_alloca(field_ty, &name)?;
            self.builder.build_store(var_ptr, field_val)?;
            self.declare_variable(name.clone(), var_ptr);
        }

        // 解析 body
        let _ = self.resolve_expression(case.body.clone())?;
        self.builder.build_unconditional_branch(exit_bb)?;
        self.exit_scope();
    }

    self.builder.position_at_end(exit_bb);
    Ok(None)
}
```

关键点：
- **EnumCase 和 Default 分支使用不同的比较策略**：EnumCase 用 `icmp eq` 比较 tag；default 用 `br` 无条件分支
- **Payload 提取分两步**：先 `struct_gep` 到 payload union 的对应 case 槽位，再逐个字段 `struct_gep` + `load` + `alloca` + `store`
- **每个 case body 后必须 `br exit_bb`**（除非 body 内部有 return/break）
- **check_bb 链**：每个 case 的检查结果 false → 进入下一个 case 的 `check_bb`，最终全部不匹配 → `exit_bb`

#### Statement::Guard 的 IR 生成

Guard 是 Statement（而非 Expression），在 `resolve_statement` 中处理：

```rust
Statement::Guard { condition, else_body, .. } => {
    // condition 是 Expression::Case 时提取 enum_name 和 case_name
    let enum_name = if let Some(name) = enum_type.as_ref().map(|t| t.value.as_str()) {
        name.to_string()
    } else if let Some(name) = self.get_enum_name_from_expr_string(expression.clone()) {
        name
    } else { String::new() };

    let check_bb = self.context.append_basic_block(fn_val, "guard_check");
    let else_bb = self.context.append_basic_block(fn_val, "guard_else");
    let exit_bb = self.context.append_basic_block(fn_val, "guard_exit");

    // 在 check_bb 中比较 tag
    // match → exit_bb (guard 通过), no match → else_bb (guard 失败)
    self.builder.build_conditional_branch(match_result, exit_bb, else_bb)?;

    // else 块：必须 exit scope（return/throw 等）
    self.builder.position_at_end(else_bb);
    self.resolve_block_expression(else_body.clone())?;

    // guard 通过后的代码
    self.builder.position_at_end(exit_bb);
}
```

注意分支方向：`build_conditional_branch(match_result, exit_bb, else_bb)` — match 为 true 时进入 exit（代码继续），false 时进入 else 块（必须退出作用域）。

#### 从表达式推断 enum 类型名

IR gen 阶段需要推断 enum 类型名（因为 match/guard 中使用 `.caseName` 简写时没有显式类型名）。从 `Expression::Variable` 的 `ty` 字段提取：

```rust
fn get_enum_name_from_expr_string(&self, expr: Rc<RefCell<Expression>>) -> Option<String> {
    let e = expr.borrow();
    match &*e {
        Expression::Variable { ty, .. } => {
            let ty = ty.as_ref()?;
            if let Type::Enum(name, _) = &*ty.borrow() {
                Some(name.clone())
            } else { None }
        }
        _ => None,
    }
}
```

需要其他表达式类型（如 Call、MemberAccess）的 enum 类型提取时，可以用类似 `extract_concrete_type_name` 的模式扩展 match 臂。

#### Statement::Fallthrough / Break 的处理

- **Fallthrough** 在 IR gen 中暂不完整支持（因为需要向前跟踪下一个 case body 的 basic block），抛出错误提示
- **Break** 同样不完整支持，抛出错误
- 表达式级 match body 中的 fallthrough/break 实际应翻译为对 exit_bb 或 next_body_bb 的 `br` 指令

#### Guard 在 resolve_statement 中的短路处理

Guard handler 的关键控制流特性：
1. 先 resolve condition（enum case 比较）
2. 执行 else block（需要 terminate，如 return）
3. 之后 builder 的插入点会留在 exit_bb（guard 通过后的代码会自然在 exit_bb 中继续生成）

不需要像 if-case 那样在 binding 后手动插入 scope，因为 guard 的 bindings 是通过 type resolver 在 scope 的 type_env 中注册的，IR gen 中后续对 binding 变量的引用会通过 `lookup_variable` 找到正确的位置。

### 测试 IR 输出

测试 LLVM IR 生成时，用 `module.print_to_string().to_string()` 获取 IR 文本，然后用 `assert!(llvm_ir.contains("..."))` 验证：

```rust
let context = Context::create();
let ir_gen = IRGenerator::new(&context, engine.clone());
let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
let llvm_ir = module.print_to_string().to_string();

assert!(llvm_ir.contains("define i32 @Foo.bar"),
    "expected function signature in IR:\n{}", llvm_ir);
assert!(llvm_ir.contains("__protocol_wt.P.Foo"),
    "expected witness table global:\n{}", llvm_ir);
```

### 7.10 defer 语句的特殊模式

`defer { body }` 是一种只有关键字 + 块的语句（无条件、无额外语法），实现模式与

## 8. 闭包（Closure）表达式特殊模式

闭包 `{ (params) -> Ret in body }` 与块表达式 `{ statements }` 共用 `{` token，需要在 parser 中通过 lookahead 消歧义。

### 8.1 闭包 vs 块消歧义

在 `parse_primary()` 中遇到 `{` 时，不能立即调用 `parse_block()`，需要通过 lookahead 判断：

```rust
// 读取 { 后的第一个 token：
// 1. `in` 关键字 → 无参闭包 { in body }
// 2. `(` → 扫描到匹配的 )，检查后续是否为 -> 或 in
//    如果是 → 闭包；否则 → 回退解析为 Block
// 3. 其他 → Block

SeparatorType::OpenBrace => {
    let closure_detected = self.index + 1 < self.tokens.len() && {
        let next = &self.tokens[self.index + 1];
        if KeywordType::is_keyword(&next, KeywordType::In) {
            true  // { in body } — 无参闭包
        } else if SeparatorType::is_separator(&next, SeparatorType::OpenParen) {
            // 扫描到匹配的 ) 看后面是不是 -> 或 in
            let mut depth = 1u32;
            let mut i = self.index + 2;
            while i < self.tokens.len() && depth > 0 {
                let t = &self.tokens[i];
                if SeparatorType::is_separator(t, SeparatorType::OpenParen) { depth += 1; }
                else if SeparatorType::is_separator(t, SeparatorType::CloseParen) { depth -= 1; }
                i += 1;
            }
            if depth == 0 && i < self.tokens.len() {
                let after = &self.tokens[i];
                OperatorType::is_operator(after, OperatorType::Arrow)
                    || KeywordType::is_keyword(after, KeywordType::In)
            } else { false }
        } else { false }
    };
    if closure_detected { self.parse_closure_expression() }
    else { self.parse_block() }
}
```

**关键原则**：lookahead 只读 `self.tokens[]` 不修改 `self.index`，失败后回退到 `parse_block()` 不需要恢复 index。

### 8.2 闭包参数解析模式

闭包参数与函数参数的关键差异：

| 特性 | FunctionDecl 参数 | 闭包参数 |
|------|------------------|---------|
| 外部标签 | ✅ 支持 `label name: Type` | ❌ 无外部标签 |
| 类型注解 | ✅ 必须 | ✅ 可选 |
| 变参 `...` | ✅ 支持 | ❌ 不支持 |
| `in` 分隔符 | ❌ 不需要 | ✅ `)` 后必须跟 `in` |
| 括号 | ✅ 必须 | ✅ `(params)` 形式必须 |

```rust
fn parse_closure_expression(&mut self) -> Result<Expression, ()> {
    self.index += 1; // 消费 {
    let parameters: Vec<Rc<RefCell<ClosureParameter>>>;
    let return_type: Option<Rc<RefCell<Expression>>>;

    // 无参闭包: { in body }
    if let Some(token) = self.peek()
        && KeywordType::is_keyword(&token, KeywordType::In)
    {
        parameters = Vec::new();
        return_type = None;
        self.index += 1; // 消费 in
    } else {
        // 有参闭包: { (params) ... in body }
        // 消费 (，解析参数序列，消费 )
        // 可选 -> RetType
        // 必须消费 in
    }

    // 解析 body statements 直到 }
    // 消费 }
    Ok(Expression::Closure { parameters, return_type, body, scope: None, ty: None })
}
```

### 8.3 ClosureParameter AST 设计

闭包参数结构体（独立于 `Parameter`）：

```rust
pub struct ClosureParameter {
    pub name: Box<Token>,
    pub type_annotation: Option<Rc<RefCell<Expression>>>,
}
```

`Expression::Closure` 枚举变体：
```rust
Closure {
    parameters: Vec<Rc<RefCell<ClosureParameter>>>,
    return_type: Option<Rc<RefCell<Expression>>>,
    body: Vec<Rc<RefCell<Statement>>>,
    scope: Option<Rc<RefCell<Scope>>>,
    ty: Option<Rc<RefCell<Type>>>,
}
```

### 8.4 函数类型语法 `(T...) -> R`

在 `parse_type_expression()` 中，解析完括号组 `(T1, T2)` 或 `(T)` 后，检查下一个 token 是否为 `->`：

**单参数组 `(T)`：**
```rust
let Some(right) = self.next() else { ... }; // 消费 )
// 在 unwrap 为分组类型之前检查
if let Some(token) = self.peek()
    && OperatorType::is_operator(&token, OperatorType::Arrow)
{
    self.index += 1;
    let return_type = self.parse_type_expression()?;
    return Ok(Expression::FunctionType {
        param_types: vec![first],  // first 是尚未 unwrap 的 Rc<RefCell<Expression>>
        return_type: Rc::new(RefCell::new(return_type)),
        ty: None,
    });
}
```

**多参数/命名组 `(T1, T2)` / `(name: T)`：**
```rust
// 创建 TupleType 之前检查
if let Some(token) = self.peek()
    && OperatorType::is_operator(&token, OperatorType::Arrow)
{
    self.index += 1;
    let return_type = self.parse_type_expression()?;
    return Ok(Expression::FunctionType {
        param_types: elements.into_iter().map(|(_, t)| t).collect(),
        return_type: Rc::new(RefCell::new(return_type)),
        ty: None,
    });
}
```

**空组 `()`：**
```rust
if SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
    let right = self.next().unwrap();
    if let Some(token) = self.peek()
        && OperatorType::is_operator(&token, OperatorType::Arrow)
    {
        // () -> R → 函数类型
        self.index += 1;
        let return_type = self.parse_type_expression()?;
        return Ok(Expression::FunctionType { param_types: vec![], return_type: ..., ty: None });
    }
    // 否则 () → Void
}
```

### 8.5 Symbol Resolver 中闭包作用域

闭包需要独立的 scope（类似函数），参数注册为 `Symbol::Variable`：

```rust
Expression::Closure { parameters, return_type, body, scope, .. } => {
    *scope = Some(self.enter_scope(None));
    for param in parameters {
        let name = param.borrow().name.value.clone();
        if name != "_" {
            let symbol = Rc::new(RefCell::new(Symbol::Variable {
                name, decl: None, parameter: None,
            }));
            self.enter(symbol, &param.borrow().name);
        }
        if let Some(type_annotation) = &param.borrow().type_annotation {
            self.resolve_expression(type_annotation.clone());
        }
    }
    if let Some(ret) = return_type { self.resolve_expression(ret.clone()); }
    for stmt in body { self.resolve_statement(stmt.clone()); }
    self.leave_scope();
}
```

### 8.6 Type Resolver 中闭包类型推断

**参数类型注册**：在进入闭包 scope 后，必须将参数类型通过 `set_type()` 注册到 scope 的 `type_env` 中，否则闭包体内的变量引用无法获得类型：

```rust
Expression::Closure { parameters, return_type, body, scope, ty, .. } => {
    let ret_type = return_type.as_ref()
        .and_then(|rt| self.infer_type(rt.clone()))
        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));
    let mut param_types = Vec::new();
    for param in parameters.iter() {
        let pt = param.borrow().type_annotation.as_ref()
            .and_then(|ta| self.infer_type(ta.clone()))
            .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));
        param_types.push(pt);
    }
    let fn_type = Rc::new(RefCell::new(Type::Function(param_types.clone(), ret_type, false)));
    *ty = Some(fn_type.clone());

    if let Some(sc) = scope {
        self.enter_scope(sc.clone());
        for (i, param) in parameters.iter().enumerate() {
            let p = param.borrow();
            let param_type = if i < param_types.len() { param_types[i].clone() }
                else { Rc::new(RefCell::new(Type::Void)) };
            self.current_scope.as_ref().unwrap().borrow_mut()
                .set_type(p.name.value.clone(), param_type);
        }
        // 处理 body statements (遍历 body，对 ExpressionStatement 直接 infer_type)
        for stmt in body.iter() {
            let s = stmt.borrow();
            if let Statement::ExpressionStatement { expression } = &*s {
                self.infer_type(expression.clone());
            } else {
                drop(s);
                self.process_decl(stmt.clone());
            }
        }
        self.leave_scope();
    }
    fn_type
}
```

**陷阱**：不能调用 `self.process_decl(stmt.clone())` 处理 body 中的 `ExpressionStatement`，因为这会导致 `process_decl` 内部 match 无法正确匹配 `ExpressionStatement` 变体。需要手动解构并调用 `self.infer_type(expression)`。

### 8.7 IRGen 中闭包代码生成

闭包需要生成独立的 LLVM function，需要用计数器生成唯一名称：

**IRGenerator 新增字段：**
```rust
closure_counter: Rc<RefCell<u32>>,
// 初始化: closure_counter: Rc::new(RefCell::new(0)),
```

**闭包代码生成模式：**
```rust
Expression::Closure { parameters, return_type, body, .. } => {
    // 1. 生成唯一函数名 __closure_{N}
    let counter = { let mut c = self.closure_counter.borrow_mut(); let val = *c; *c += 1; val };
    let fn_name = format!("__closure_{}", counter);

    // 2. 从参数和返回类型推断 LLVM 函数类型
    let fn_llvm_type = self.get_function_type(ret_type, param_types, false)?;
    let function = self.module.add_function(&fn_name, fn_llvm_type, None);

    // 3. 保存当前插入点，创建 entry block
    let current_block = self.builder.get_insert_block();
    let entry_block = self.context.append_basic_block(function, "entry");
    self.builder.position_at_end(entry_block);

    // 4. 进入新 scope，分配参数到 alloca
    self.enter_scope();
    for (i, param) in parameters.iter().enumerate() {
        let param_name = &param.borrow().name.value;
        let llvm_type = self.resolve_type(param_types[i].clone())?;
        let ptr = self.builder.build_alloca(llvm_type, &unique_name)?;
        let param_value = function.get_nth_param(i as u32).unwrap();
        self.builder.build_store(ptr, param_value)?;
        self.declare_variable(param_name.clone(), ptr);
    }

    // 5. 生成函数体（类似 FunctionDecl 的 body 生成）
    // 对于最后一条表达式语句，自动作为返回值
    // 对于普通语句，逐条 resolve

    // 6. 恢复插入点
    if let Some(block) = current_block { self.builder.position_at_end(block); }

    // 7. 返回函数指针（bitcast 到通用指针类型）
    let fn_ptr = function.as_global_value().as_pointer_value();
    let ptr_ty = self.context.ptr_type(AddressSpace::from(0));
    Ok(Some(self.builder.build_bit_cast(fn_ptr, ptr_ty, "")?.into()))
}
```

### 8.8 Type::Function 作为变量类型的处理

在 `resolve_type()` 中，遇到 `Type::Function(_, _, _)` 必须返回指针类型（而非报错），因为 LLVM 中函数不是值类型：

```rust
// 以前:
Type::Function(_, _, _) => {
    // ❌ Nested function types are not supported — 会报错
    anyhow::bail!("...");
}

// 改为:
Type::Function(_, _, _) => {
    // ✅ 返回通用指针类型
    self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
}
```

这样闭包变量（如 `let f = { ... }`）才能分配存储空间（alloca ptr）。

### 8.9 闭包实现的测试策略

四个阶段的测试关注点：

| 阶段 | 测试重点 |
|------|---------|
| Parser | 闭包 AST 结构（parameters/return_type/body），与 Block 的消歧义 |
| SymbolResolver | 参数 symbol 解析、外部变量捕获、scope 分配 |
| TypeResolver | `Type::Function` 构造、参数类型注册、body 内变量类型推断 |
| IRGen | `__closure_N` 函数定义、参数店分配、函数指针返回 |

IRGen 测试的典型模式：

```rust
let llvm_ir = module.print_to_string().to_string();
assert!(llvm_ir.contains("__closure_0"), "Should define closure function");
assert!(llvm_ir.contains("define i32 @__closure_0(i32 %"), "Closure signature");
assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
``` `loop` 相同但更简单。

#### 7.10.1 关键字 + 块语句的标准模式

```rust
// parse_statement 中的分发
KeywordType::Defer => {
    if !modifiers.is_empty() {
        self.emit_error(TrussDiagnosticCode::ModifierNotAllowedHere, ...);
    }
    self.parse_defer()
}

// parse_defer 实现
fn parse_defer(&mut self) -> Result<Statement, ()> {
    let token = self.next().unwrap();
    if let Some(t) = self.peek()
        && SeparatorType::is_separator(&t, SeparatorType::OpenBrace)
    {
        let body = self.parse_block()?;
        let body_rc = Rc::new(RefCell::new(body));
        
        // 可选：检查 body 中禁止的语句
        Ok(Statement::Defer {
            token: Box::new(token),
            body: body_rc,
        })
    } else {
        self.emit_error(
            TrussDiagnosticCode::ExpectedBlockAfterDefer,
            "Expected '{' after 'defer'",
            &self.tokens[self.index],
        );
        Err(())
    }
}
```

关键点：
- 先检查 `{`（调用 `SeparatorType::is_separator`），再调用 `parse_block()`
- 与 `loop` 完全一致的模式
- AST 中 body 的类型是 `Rc<RefCell<Expression>>`（因为 block 本身就是 `Expression::Block`）

#### 7.10.2 对语句体做语义检查

当需要在解析后对语句/表达式体做语义约束检查（如 defer body 中禁止控制流），有两种可行策略：

**Strategy A — 直接匹配 check（推荐，简单直接）：**

```rust
if let Expression::Block { ref statements, .. } = body {
    for stmt in statements {
        if Self::is_forbidden_in_defer(&*stmt.borrow()) {
            self.emit_error(
                TrussDiagnosticCode::ControlFlowNotAllowedInDefer,
                ...,
            );
        }
    }
}

fn is_forbidden_in_defer(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Return { .. }
            | Statement::Throw { .. }
            | Statement::Break { .. }
            | Statement::Fallthrough { .. }
    )
}
```

**Strategy B — 解析时设置 flag（适合更复杂的上下文敏感约束）：**

在 parser 中维护一个 `in_defer_body: bool` 标志，在 `parse_statement` 中检查。但 Strategy A 更简单。

#### 7.10.3 副作用：Expression 的 body 不可移动两次

在 `parse_defer()` 中，`body` 是 owned `Expression`。如果需要在 match 中检查其内部 statements，同时还要在后续使用它构造 Statement，注意：

```rust
// 正确：ref 借用，不移动 body
if let Expression::Block { ref statements, .. } = body {
    // 检查 statements...
}
// body 仍可用
Ok(Statement::Defer {
    token: Box::new(token),
    body: Rc::new(RefCell::new(body)),  // 没问题
})
```

`ref statements` 是借用，所以 `body` 不会被移动。

#### 7.10.4 测试模式

```rust
#[test]
fn test_parse_defer_statement() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func test() { defer { cleanup() } }".to_string(), ...),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    
    // 嵌套匹配：FunctionDecl → body → defer → block
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Defer { body: defer_body, .. } = &*statements[0].borrow()
        && let Expression::Block { statements: block_stmts, .. } = &*defer_body.borrow()
    {
        assert_eq!(block_stmts.len(), 1);
    } else {
        panic!("...");
    }
}

// 错误 case 测试：验证 engine 有错误
#[test]
fn test_parse_defer_with_return_error() {
    let engine = create_engine();
    // lexer 和 parser 使用 engine.clone()
    let mut lexer = Lexer::new(..., engine.clone());
    let mut parser = Parser::new(..., engine.clone());
    parser.parse();
    assert!(engine.borrow().has_errors());
}
```

检查错误时**必须 clone engine**（`Lexer::new` 和 `Parser::new` 都用 `engine.clone()`），否则 `engine` 所有权会被移入 parser，后续无法调用 `engine.borrow()`。

#### 7.10.5 Symbol Resolver 中对 defer body 的处理

在 symbol resolver 中，`defer { body }` 的 body 是 `Expression::Block`，需要：

**`register_symbols` 中** — 遍历 block 的 statements 递归注册（因为 defer body 内可能声明嵌套函数）：

```rust
Statement::Defer { body, .. } => {
    if let Expression::Block { statements, .. } = &*body.borrow() {
        for stmt in statements {
            self.register_symbols(stmt.clone());
        }
    }
}
```

**`resolve_statement` 中** — 直接将 body 作为表达式解析：

```rust
Statement::Defer { body, .. } => {
    self.resolve_expression(body.clone());
}
```

`resolve_expression` 对于 `Expression::Block` 的处理是：enter scope → resolve each statement → leave scope。defer body 中引用的外层变量通过 scope chain 自动可见。

#### 7.10.6 Type Resolver 中对 defer body 的处理

Type resolver 中同样只需要将 body 作为 block 表达式解析：

```rust
Statement::Defer { body, .. } => {
    self.resolve_block_expression(body.clone());
}
```

`resolve_block_expression` 遍历 block 中的语句调用 `resolve_statement`，对每个表达式做类型检查和推断。

不需要特殊处理 scope 进入/离开——`resolve_block_expression` 本身已经调用 `enter_scope`/`leave_scope`。

#### 7.10.7 IR Gen 中对 defer 的处理（最复杂）

IR gen 是 defer 最复杂的一环。核心思路：**保存 body 到当前 scope，在 scope exit 时按 LIFO 发射**。

**Step 1: Scope 结构体加字段**

```rust
struct Scope<'ctx> {
    variables: HashMap<String, PointerValue<'ctx>>,
    deferred_vars: Vec<(PointerValue<'ctx>, String)>,
    deferred_blocks: Vec<Rc<RefCell<Expression>>>,  // 新增
}
```

**Step 2: resolve_statement 中保存 defer body**

`defer` 不产生任何 IR 指令，只是把 body 存入当前 scope：

```rust
Statement::Defer { body, .. } => {
    self.scope_stack
        .borrow_mut()
        .last_mut()
        .unwrap()
        .deferred_blocks
        .push(body.clone());
    Ok(false)
}
```

**Step 3: exit_scope 和 emit_all_deinit_calls 中发射**

在离开作用域时，按 LIFO 顺序先发射 deferred blocks，再发射 deferred_vars deinit：

```rust
fn exit_scope(&self) {
    // 先 clone 出数据，避免 scope_stack 的 borrow 与 resolve_statement 冲突
    let (deferred_blocks, deferred_vars) = {
        let mut stack = self.scope_stack.borrow_mut();
        if let Some(scope) = stack.last() {
            (scope.deferred_blocks.clone(), scope.deferred_vars.clone())
        } else {
            return;
        }
    };

    let current_block = self.builder.get_insert_block();
    let can_emit = current_block.map_or(false, |b| !b.get_terminator().is_some());
    if can_emit {
        // LIFO: 逆序迭代
        for block_expr in deferred_blocks.iter().rev() {
            if let Expression::Block { statements, .. } = &*block_expr.borrow() {
                for stmt in statements {
                    let _ = self.resolve_statement(stmt.clone());
                }
            }
        }
        for (var_ptr, type_name) in &deferred_vars {
            // 原有的 struct/enum deinit 调用
        }
    }
    self.scope_stack.borrow_mut().pop();
}
```

**Step 4: emit_all_deinit_calls 同样处理**

在 return/throw 路径上，`emit_all_deinit_calls` 需要遍历所有 scope 的 deferred blocks：

```rust
fn emit_all_deinit_calls(&self) {
    let all_blocks: Vec<(Vec<Rc<RefCell<Expression>>>, Vec<(PointerValue<'ctx>, String)>)> = {
        let stack = self.scope_stack.borrow();
        stack.iter().rev()
            .map(|scope| (scope.deferred_blocks.clone(), scope.deferred_vars.clone()))
            .collect()
    };
    for (deferred_blocks, deferred_vars) in &all_blocks {
        for block_expr in deferred_blocks.iter().rev() { /* emit */ }
        for (var_ptr, type_name) in deferred_vars { /* deinit */ }
    }
}
```

#### 7.10.8 IR Gen 中 defer 的 borrow 冲突规避

`exit_scope` 和 `emit_all_deinit_calls` 都会调用 `self.resolve_statement()`，而 `resolve_statement` 可能再次访问 `self.scope_stack`。这会导致 **double borrow**：

```rust
// 错误：
fn exit_scope(&self) {
    let mut stack = self.scope_stack.borrow_mut();  // 1st borrow
    if let Some(scope) = stack.last() {
        for stmt in statements {
            let _ = self.resolve_statement(stmt.clone());  // 2nd borrow inside!
        }
    }
    stack.pop();
}
```

**解决方案**：先 clone 出需要的数据，释放 scope_stack 的 borrow，再处理：

```rust
let (deferred_blocks, deferred_vars) = {
    let mut stack = self.scope_stack.borrow_mut();
    if let Some(scope) = stack.last() {
        (scope.deferred_blocks.clone(), scope.deferred_vars.clone())
    } else {
        return;
    }
};  // stack borrow 在这里释放

// 现在可以安全调用 self.resolve_statement
for block_expr in deferred_blocks.iter().rev() { ... }
```

#### 7.10.9 完整阶段的 defer 测试

每阶段的测试模式：

```rust
// Parser 测试 — 检查 AST 结构
#[test]
fn test_parse_defer_statement() {
    // lexer + parser → 匹配 Statement::Defer { body } → Expression::Block { statements }
}

// Symbol Resolver 测试 — 检查无错误 / 未定义变量报错
#[test]
fn test_defer_body_variable_resolved() {
    // lexer + parser + symbol_resolver → 检查 engine 无错误
}
#[test]
fn test_defer_body_undefined_variable_error() {
    // lexer + parser + symbol_resolver → 检查 engine 有错误
}

// Type Resolver 测试 — 检查类型解析正确 / 类型错误报错
#[test]
fn test_defer_body_type_resolved() {
    // 完整 pipeline (含 type_resolver) → 检查无错误
}
#[test]
fn test_defer_body_type_error() {
    // 使用未定义类型 → 检查有错误
}

// IR Gen 测试 — 检查不 panic
#[test]
fn test_irgen_defer_basic() {
    // 完整 pipeline (含 ir_gen) → std::panic::catch_unwind 检查不崩溃
}
```

## 通用陷阱与注意事项

### 0. 泛型语法实现的特殊模式（parser + symbol_resolver 阶段经验）

泛型语法涉及多个 Decl 类型的统一改造，有固定的模式：

**添加 new keyword 到 `KeywordType` 后，必须同时更新 `code()` 方法**，否则 lexer 无法映射字符串到枚举变体。

**给多个 Decl 添加新字段时（如 `generic_parameters`、`where_clause`）**，统一在 `src/ast/statement.rs` 中修改所有相关变体，然后更新 `token()` 和 `modifiers()` 方法。

**`parse_generic_parameters` — 解析 `<T, U: Constraint>`：**
```rust
fn parse_generic_parameters(&mut self) -> Result<Option<Vec<GenericParameter>>, ()> {
    if let Some(token) = self.peek()
        && OperatorType::is_operator(&token, OperatorType::Less)
    {
        self.index += 1;
        let mut params = Vec::new();
        loop {
            let name = self.next()?;
            // 检查 name 是 Identifier
            let mut constraints = Vec::new();
            if let Some(t) = self.peek()
                && SeparatorType::is_separator(&t, SeparatorType::Colon)
            {
                self.index += 1;
                // 注意: parse_type_expression 已处理 & 作为复合类型，
                // 所以这里不需要手动循环 BitAnd
                constraints.push(Rc::new(RefCell::new(self.parse_type_expression()?)));
            }
            params.push(GenericParameter { name: Box::new(name), constraints });
            // 检查下一个是 >、, 还是其他
            // ...
        }
        // 消耗 >
        Ok(Some(params))
    } else {
        Ok(None)
    }
}
```

**`parse_where_clause` — 解析 `where T: P && U == V`：**
- 使用 `&&`（`OperatorType::And`）连接多个要求，而不是逗号
- 每个要求可以是 `T: Constraint`（Conformance）或 `T.Element == U`（Equality，用 `OperatorType::Equal`）

**`protocol<T>` 的语法糖：** 在 `parse_protocol_decl` 中，解析完 `<T, U>` 后生成对应的 `ProtocolMember::AssociatedType` 条目，合并到 members 列表头部。

**约束中的 `&` 由 `parse_type_expression` 自动处理：** `T: Equatable & Hashable` 中的 `&` 被 `parse_type_expression` 解析为 `Expression::CompoundType`，所以 `GenericParameter.constraints` 中只会有一个元素（复合类型），不需要手动处理 `BitAnd`。

**Where 子句在 parse_function_decl 中的位置：** 在 return_type 解析之后、body 解析之前插入 `self.parse_where_clause()?`。

**在 parse_statement 中分派新关键字：** 直接添加到 `match token.ty { TokenType::Keyword { keyword } => match keyword { ... } }` 中，与 `KeywordType::Struct`、`KeywordType::Class` 等并列。

**`Type::GenericParam` 添加后的连锁反应：**
- `src/types/mod.rs` 中添加新变体和对应的 `Display` impl
- `src/ir_gen/mod.rs` 的 `resolve_type` 中必须添加 match arm（返回 opaque `ptr` 类型）
- 如果其他模块中有完整的 `match &*ty.borrow() { ... }`（覆盖所有变体），也需要添加 stub

**Symbol Resolver 中泛型参数注册：**
```rust
// 在 register_symbols + resolve_statement 中为每个有 generic_parameters 的 Decl 注册
for gp in generic_parameters {
    let gp_type = Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())));
    scope.as_ref().unwrap().borrow_mut().set_type(gp.name.value.clone(), gp_type);
}
```
- **register_symbols**: StructDecl/ClassDecl/EnumDecl/ProtocolDecl 在创建 scope 后立即注册
- **resolve_statement**: FunctionDecl 在 `*scope = Some(self.enter_scope(None))` 后注册

**协议符合性中的 type_parameters 解析：** 当 struct 声明 `: Container<Int32>`，conformance 表达式是 `Expression::Type { name: "Container", type_parameters: Some([Int32]) }`。旧的解析只处理 `name`，需要扩展为同时解析 `type_parameters`：
```rust
fn resolve_conformance(&mut self, conformance: Rc<RefCell<Expression>>) {
    match &*conformance.borrow() {
        Expression::Type { name, type_parameters, .. } => {
            let _ = self.resolve_symbol(name);
            if let Some(type_parameters) = type_parameters {
                for tp in type_parameters {
                    self.resolve_expression(tp.clone());
                }
            }
        }
        Expression::CompoundType { types, .. } => {
            for t in types {
                self.resolve_conformance(t.clone());
            }
        }
        _ => { self.resolve_expression(conformance.clone()); }
    }
}
```

**Where clause 条件中的类型表达式解析：** 抽象为独立方法 `resolve_where_requirement` 以便在所有 Decl 的 resolve_statement 中复用：
```rust
fn resolve_where_requirement(&mut self, req: &WhereRequirement) {
    match &req.kind {
        WhereRequirementKind::Conformance { type_expr, constraint } => {
            self.resolve_expression(type_expr.clone());
            self.resolve_expression(constraint.clone());
        }
        WhereRequirementKind::Equality { left, right } => {
            self.resolve_expression(left.clone());
            self.resolve_expression(right.clone());
        }
    }
}
```

**TypeAlias 在 Struct/Class body 中：** `parse_statement` 中分派到 `parse_typealias`，返回 `Statement::TypeAlias`。在 `register_symbols` 中通过 StructDecl body 循环的 `else { self.register_symbols(field_stmt.clone()) }` 兜底处理。在 `resolve_statement` 中添加显式分支解析类型表达式。

**利用 `&&` 分隔 where 要求（而非逗号）：** 避免与泛型参数列表和类型参数列表中的逗号混淆。`parse_where_clause` 中用 `OperatorType::And`（`&&`）作为 separator。

**Type Resolver 中泛型参数注册的作用域问题：** 泛型参数必须在类型/函数 scope 中注册后，`resolve_type_name` 才能在 scope chain 中找到它们。但不同 Decl 的 scope 管理方式不同：

**FunctionDecl `process_decl` 必须先进入 scope：** 旧代码先解析参数/返回类型再进入函数 body scope，但泛型参数需要在解析参数类型时就可见。必须改为：
```rust
// 错误: 泛型参数不可见
let ret_type = self.infer_type(return_type_expr.clone())?;
self.enter_scope(scope.as_ref().unwrap().clone());

// 正确: 先进入 scope 注册泛型参数
self.enter_scope(scope.as_ref().unwrap().clone());
for gp in generic_parameters {
    let gp_type = Rc::new(RefCell::new(Type::GenericParam(gp.name.value.clone())));
    self.current_scope.as_ref().unwrap().borrow_mut().set_type(gp.name.value.clone(), gp_type);
}
// 再解析参数/返回类型（泛型参数已在 scope 中）
let ret_type = self.infer_type(return_type_expr.clone())?;
```
函数类型需要在父 scope 中注册，而非当前 scope：
```rust
// 注册 fn type 到父 scope
if let Some(parent) = self.current_scope.as_ref().unwrap().borrow().parent.clone() {
    parent.borrow_mut().set_type(name.value.clone(), fn_type);
}
```

**EnumDecl 需要先进入 scope 再处理 case 参数：** Enum case 参数的类型表达式（如 `some(T)` 中的 `T`）可能在 scope 注册泛型参数之前被推断。必须将 scope 进入和泛型参数注册移到 case 循环之前。

**resolve_statement 必须进入 body scope：** StructDecl/ClassDecl/EnumDecl 的 `resolve_statement` 中必须进入对应 scope（与 `process_decl` 一致），否则 body 中的类型引用找不到 scope 中的泛型参数：
```rust
// StructDecl resolve_statement 正确模式:
Statement::StructDecl { body, conformances, scope, .. } => {
    for conformance in conformances {
        self.infer_type(conformance.clone());
    }
    if let Some(s) = scope.as_ref() {
        self.enter_scope(s.clone());
        for stmt in body { self.resolve_statement(stmt.clone()); }
        self.leave_scope();
    } else {
        for stmt in body { self.resolve_statement(stmt.clone()); }
    }
}
```

**TypeAlias 必须注册到 scope：** 在 `process_decl` 和 `resolve_statement` 中，TypeAlias 不仅要推断类型表达式，还必须将结果注册到当前 scope：
```rust
Statement::TypeAlias { type_expression, name, .. } => {
    if let Some(ty) = self.infer_type(type_expression.clone()) {
        if let Some(scope) = self.current_scope.as_ref() {
            scope.borrow_mut().set_type(name.value.clone(), ty);
        }
    }
}
```

### 关联类型访问（`I.Item` 语法）

在类型位置支持 `I.Item` 语法，其中 `I` 是类型表达式（泛型参数、协议类型等），`Item` 是协议中声明的 `associatedtype` 或 `typealias`。

**语法范围：** 出现在所有调用 `parse_type_expression()` 的位置（参数类型、返回类型、where 子句、变量声明等），与值表达式的 `MemberAccess`（`.field`）完全不同。

**实现要点：**

1. **AST 新增 `Expression` 变体**（`src/ast/expression.rs`）：
   ```rust
   AssociatedTypeAccess {
       object: Rc<RefCell<Expression>>,
       member: Box<Token>,
       ty: Option<Rc<RefCell<Type>>>,
   }
   ```
   同时更新 `get_ty()`、`get_ty_ref()`、`get_ty_mut_ref()`、`token()` 四个方法。

2. **Type 新增变体**（`src/types/mod.rs`）：
   ```rust
   AssociatedType(Rc<RefCell<Type>>, String),  // (base_type, member_name)
   ```
   同时更新 `fmt::Display`（`"{}.{}", base.borrow(), name`）。

3. **Parser 中 `.` 的处理位置**（`src/parser/mod.rs`）：

   在 `parse_type_expression()` 中，`.` 必须在 `*`（指针类型）和 `&`（复合类型）**之前**处理。需要修改**两个路径**：

   **标识符路径**（默认分支）：
   ```
   1. 解析 name + type_parameters → Expression::Type
   2. while peek() is '.': → Expression::AssociatedTypeAccess (可链式)
   3. while peek() is '*': → Expression::PointerType
   4. while peek() is '&': → Expression::CompoundType
   ```

   **括号路径**（`(T)` 或 `(label: T, ...)`）：
   ```
   1. 解析括号内容 → type_expr
   2. while peek() is '.': → Expression::AssociatedTypeAccess (同上)
   3. while peek() is '*': → Expression::PointerType
   4. while peek() is '&': → Expression::CompoundType
   ```

   `.` 的解析模式：
   ```rust
   while let Some(token) = self.peek()
       && OperatorType::is_operator(&token, OperatorType::Dot)
   {
       self.index += 1;
       let Some(member_token) = self.next() else {
           // 错误：'.' 后需要标识符
           return Err(());
       };
       if TokenType::Identifier != member_token.ty {
           // 错误：期望关联类型名称
           return Err(());
       }
       type_expr = Expression::AssociatedTypeAccess {
           object: Rc::new(RefCell::new(type_expr)),
           member: Box::new(member_token),
           ty: None,
       };
   }
   ```

   **关键**：`.` 在 `*` 之前处理，这样 `I.Item*` 正确解析为 `PointerType(AssociatedTypeAccess(I, Item))`。

4. **链式支持**：`I.Item.SubItem` 自然支持，因为 `while` 循环会多次应用，产生嵌套的 `AssociatedTypeAccess`。

5. **区分值访问与类型访问**：`parse_type_expression()` 处理 `I.Item`（类型访问），`parse_primary()` 的 post-loop 处理 `obj.field`（值访问）。两者使用不同的 `Expression` 变体（`AssociatedTypeAccess` vs `MemberAccess`），下游阶段据此做不同的语义处理。

6. **分阶段 stub 策略**：Parser 阶段在 type_resolver 和 ir_gen 中添加 stub 让编译通过：
   - Type Resolver `infer_type`：发 "not yet supported" 错误并 `return None`
   - IR Gen `resolve_type`：返回 `ptr_type(0)`（不透明指针，同 GenericParam）

### 1. `edit` 工具容易把周围文本合并到一起
当用 `edit` 替换包含函数声明或 `match` 臂的大段代码时，确保 `old_string` 唯一且精确。如果 match 的最后一项是 `_ => {}`，替换时连 `fn xxx` 一起替换容易把下一个函数签名吃掉。**最佳实践**: 在 `_ => {}` 之前插入新 match 臂，而不是替换 `_ => {}`。

### 2. 在同一个 match 中同时 borrow 两个 Rc
```rust
// 错误: 两次 mutable borrow
match &mut *target_sym.borrow_mut() { ... }
// 同时还想用 target_sym 做其他事
```
**解决方案**: 先 clone target_sym，然后用 clone 后的引用做 mutable match。

### 3. SelfType 需要同时处理多个 match 位置
- `type_resolver::infer_type` — 主类型推导
- `ast/expression.rs` 中的 `get_ty()`、`get_ty_ref()`、`get_ty_mut_ref()`、`token()`
- `symbol_resolver` 的 `resolve_expression` 中的 `_ => {}` catch-all 也会覆盖

### 4. 方法 body 的多次 borrow
在 symbol resolver 的 `register_symbols` 中，先从外部 `if let` 匹配 `Statement::FunctionDecl { name, body, .. }`，然后在内部又用 `if let Statement::FunctionDecl { body, .. }` 重新匹配同一个 statement 来取 body，会导致外部的 body 变量 unused。**解决方案**: 外层只 `name: method_name` 就够了。

### 5. 恢复 `current_scope`
无论中间是否有错误，`register_symbols` 和 `resolve_statement` 中都要确保在函数退出前恢复 `self.current_scope = saved`。

### 6. Protocol 成员的特殊模式（不同于 struct/class body）

Protocol 体的解析不使用 `parse_brace_body()`（即不通过 `parse_statement` 通用分派），而是**在 `parse_protocol_decl` 内用自定义的 `match peek_token.ty` 循环**。这意味着：

**Protocol 成员之间用空格分隔，不是分号。** 例如：
```
protocol P { typealias Inner = Int32 func get() -> Inner }
//                                      ^^ 没有分号
```

**现有可被协议体解析的关键字**在 `parse_protocol_decl` 的 `match` 中逐一列出：
- `func` → `ProtocolMember::Method`
- `associatedtype` → `ProtocolMember::AssociatedType`
- `let` / `var` → `ProtocolMember::Property`
- `typealias` → `ProtocolMember::TypeAlias`（新增时需要添加）

**添加新的协议成员类型需要做的修改：**

1. **AST: `ProtocolMember` 枚举** (`src/ast/statement.rs`)
   ```rust
   pub enum ProtocolMember {
       Method { ... },
       Property { ... },
       AssociatedType { ... },
       TypeAlias { token, name, type_expression },  // 新增
   }
   ```

2. **Parser: `parse_protocol_decl` 中添加 match 分支** (`src/parser/mod.rs`)
   ```rust
   TokenType::Keyword { keyword }
       if keyword == KeywordType::Typealias =>
   {
       let token = self.next().unwrap();
       // 解析 name, =, type_expression...
       members.push(ProtocolMember::TypeAlias { ... });
   }
   ```
   **不需要**像 struct body 中那样消耗分号——协议成员之间没有分号。

3. **Symbol Resolver: 两个位置都加处理** (`src/symbol_resolver/mod.rs`)
   - `register_symbols` 中的 protocol member 循环
   - `resolve` 中的 protocol member 循环
   - 通常只需要 `self.resolve_expression(type_expression.clone())`

4. **Type Resolver: 两个位置都加处理** (`src/type_resolver/mod.rs`)
   - `process_decl` 中的 protocol member 循环
   - `resolve_statement` 中的 protocol member 循环
   - 通常是 infer type + 注册到 scope（同 `Statement::TypeAlias` 的通用处理）

5. **IR Gen: 两个位置都加** (`src/ir_gen/mod.rs`)
   - 协议成员迭代位置
   - 通常添加空 match arm 即可（typealias 不产生 IR）

**TypeAlias 在非 protocol 上下文中（top-level/struct/class/enum/func body）：** 这些上下文都走 `parse_statement` → `parse_typealias` 路径，生成 `Statement::TypeAlias`。解析器和下游阶段已经支持。不需要额外修改。

## 测试策略

| 阶段 | 测试文件 | 测试重点 |
|------|---------|---------|
| Parser | `tests/parser.rs` | AST 结构正确性 |
| Symbol Resolver | `tests/symbol_resolver.rs` | 符号注册成功、self 可解析、未定义类型报错 |
| Type Resolver | `tests/type_resolver.rs` | 类型推导正确、无类型错误 |
| IR Gen | `tests/ir_gen.rs` | 代码生成不崩溃 |

每个测试需要完整的 pipeline 链：`Lexer → Parser → SymbolResolver → TypeResolver`，然后检查 engine 的错误列表或 AST 字段。

## 参考模式

```rust
// Statement 变体添加
Statement::ExtensionDecl {
    token: Box<Token>,
    type_name: Box<Token>,
    conformances: Vec<Rc<RefCell<Expression>>>,
    body: Vec<Rc<RefCell<Statement>>>,
    scope: Option<Rc<RefCell<Scope>>>,
}

// Parser 中解析关键字
KeywordType::Extension => self.parse_extension_decl(modifiers),

// 类型表达式中解析关键字
if let Some(token) = self.peek()
    && KeywordType::is_keyword(&token, KeywordType::SelfType)
{
    self.index += 1;
    return Ok(Expression::SelfType { token: Box::new(token), ty: None });
}
```

### 13. 关联类型访问语法 `I.Item`（type-level member access）

与值成员访问（`obj.field` 在 `parse_primary()` 后处理循环中处理）不同，类型层级的 `.Item` 访问在 **`parse_type_expression()`** 中处理。

#### 13.1 Parser 中的插入位置

`parse_type_expression()` 有两条解析路径需要同时加 `.` 处理：

```
【标识符路径】                                【括号路径】
Parse identifier                            Parse (Type) or (label: T, ...)
Parse <T, U> generic args                   Unwrap inner expression
↓                                            ↓
【插入点】→ while token == '.' {            【插入点】→ while token == '.' {
     consume '.'                               consume '.'
     read identifier (must be ident)           read identifier (must be ident)
     wrap in AssociatedTypeAccess              wrap in AssociatedTypeAccess
}                                            }
↓                                            ↓
while token == '*' { ptr type }              while token == '*' { ptr type }
while token == '&' { compound }              while token == '&' { compound }
```

**关键点**：
- `.` 必须在 `*` 和 `&` **之前**处理，这样 `I.Item*` 的语义是 "pointer to I.Item"
- 支持链式：`I.Item.Sub` 产生嵌套 `AssociatedTypeAccess` 树
- `parse_primary()` 中的 `.` 处理保持不变（那是值级别的 MemberAccess）

```rust
// 标识符路径中的插入代码：
let mut type_expr = Expression::Type {
    name: Box::new(name),
    type_parameters,
    ty: None,
};

// ==== 新增：处理 .Item 语法 ====
while let Some(token) = self.peek()
    && OperatorType::is_operator(&token, OperatorType::Dot)
{
    self.index += 1;
    let Some(member_token) = self.next() else { /* error */ };
    if TokenType::Identifier != member_token.ty { /* error */ }
    type_expr = Expression::AssociatedTypeAccess {
        object: Rc::new(RefCell::new(type_expr)),
        member: Box::new(member_token),
        ty: None,
    };
}
// ===========================

// 原有的 * 和 & 处理：
while let Some(token) = self.peek() && OperatorType::is_operator(&token, OperatorType::Multiply) { ... }
```

**括号路径**完全相同的循环，插入在 `Rc::try_unwrap(first).ok().unwrap().into_inner()` 之后、`*` 循环之前。

#### 13.2 AST 结构

```rust
// src/ast/expression.rs
Expression::AssociatedTypeAccess {
    object: Rc<RefCell<Expression>>,   // 基础类型表达式
    member: Box<Token>,                 // 关联类型名称
    ty: Option<Rc<RefCell<Type>>>,
}

// src/types/mod.rs
Type::AssociatedType(Rc<RefCell<Type>>, String),  // (base_type, member_name)
```

必须在 `get_ty()`、`get_ty_ref()`、`get_ty_mut_ref()`、`token()` 四个方法中添加 match arm。

#### 13.3 Symbol Resolver 中的关联类型注册

`ProtocolMember::AssociatedType` 的名称必须注册到协议 scope 的 `type_env` 中，否则后续类型解析找不到关联类型名：

```rust
// in register_symbols, ProtocolDecl handling:
ProtocolMember::AssociatedType { name, .. } => {
    let at_type = Rc::new(RefCell::new(Type::GenericParam(name.value.clone())));
    scope.as_ref().unwrap().borrow_mut().set_type(name.value.clone(), at_type);
}
```

关联类型使用 `Type::GenericParam` 作为占位类型——它是一个"类型变量"，与泛型参数使用同一种表示。实际的类型解析在 type resolver 阶段完成。

在 `resolve_statement` 中，还需 resolve associatedtype 的约束表达式：
```rust
ProtocolMember::AssociatedType { constraints, .. } => {
    for constraint in constraints {
        self.resolve_expression(constraint.clone());
    }
}
```

#### 13.4 Type Resolver 中的处理

`Expression::AssociatedTypeAccess` 的 `infer_type` 处理流程：

1. 递归 `infer_type(object)` 获得 object 的类型  
2. 根据 object 类型分支处理：

**Protocol 类型** — 完整查找链：
```rust
// Type::Protocol(name, weak_sym)  → 通过 weak_sym 获取 symbol → decl → scope
Type::Protocol(protocol_name, weak_sym) => {
    if let Some(sym) = weak_sym.0.upgrade() {               // WeakSymbol 是 newtype，需要 .0
        if let Ok(Some(decl)) = sym.borrow().get_decl() {   // Symbol::Protocol { decl }
            if let Statement::ProtocolDecl { scope, .. } = &*decl.borrow() {
                if let Some(protocol_scope) = scope {
                    if let Some(found) = protocol_scope.borrow().get_type(&member.value) {
                        // 如果找到的是 Type::GenericParam (associatedtype)，返回 Type::AssociatedType
                        // 如果找到的是具体类型 (typealias)，直接返回
                    }
                }
            }
        }
    }
}
```

**Compound 类型** — 遍历每个协议查找：
```rust
Type::Compound(types) => {
    for t in types {
        if let Type::Protocol(_name, weak_sym) = &*t.borrow() {
            // 同 Protocol 的查找逻辑，找到第一个匹配就 break
        }
    }
}
```

**GenericParam 类型** — 直接产出 `Type::AssociatedType`：
```rust
Type::GenericParam(_) => {
    Rc::new(RefCell::new(Type::AssociatedType(object_ty.clone(), member.value.clone())))
}
```

#### 13.5 IR Gen 中的处理

`Type::AssociatedType(_, _)` 在不做 monomorphization 的架构下等同于 `GenericParam`——全擦除为不透明指针：

```rust
Type::AssociatedType(_, _) => {
    self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
}
```

#### 13.6 测试注意事项

Parser 测试使用 `Parameter.type_expression`（而非 `Parameter.ty`）获取类型表达式：
- `Parameter.type_expression: Rc<RefCell<Expression>>` — 由 parser **直接填充**
- `Parameter.ty: Option<Rc<RefCell<Type>>>` — 由 type resolver **后续填充**

```rust
// 正确的 parser 测试方式：
if let Statement::FunctionDecl { parameters, .. } = &*program.statements[1].borrow() {
    let ty_expr = parameters[0].borrow().type_expression.clone();
    let ty_expr_ref = ty_expr.borrow();
    assert!(matches!(&*ty_expr_ref, Expression::AssociatedTypeAccess { .. }));
}

// VariableDecl 同理：
Statement::VariableDecl { type_expression, .. } => {
    let ty_expr = type_expression.as_ref().unwrap().borrow();
    // 用 Expression::AssociatedTypeAccess 匹配
}
```

Symbol resolver 测试验证无错误即可（关联类型名称的可见性由 type resolver 验证）：

```rust
// 验证关联类型注册不会导致错误
resolver.resolve(&program, "test".to_string());
let errors = engine.borrow().get_errors();
assert_eq!(errors.len(), 0);
```

## 补充模式：if 表达式作为值（带 phi node）和隐式返回

### if 表达式作为值

Truss 的 `if` 在 parser 层已经是 `Expression::If`（不是 Statement），所以 `let x = if cond { 1 } else { 2 }` 在 parser 层面自然工作。关键在下游：

**Type Resolver**（`infer_type`）：
- `Expression::If` 已经推断 then 分支的类型并检查两个分支类型一致，返回 `then_ty`
- 用于 variable initializer 时（`resolve_statement` 中的 `VariableDecl`），如果无类型注解则 `infer_type` 推断类型，有注解则 `check_type_with_expected` 检查兼容性
- 这个流程在 if 表达式投入使用时无需额外修改

**IR Generator**（`resolve_expression`）：
- `Expression::If` 当前返回 `Ok(None)` — 不产生值。要使其作为表达式值，需要在 exit block 使用 **phi node**

```rust
Expression::If { condition, then, else_, .. } => {
    let fn_val = self.builder.get_insert_block().unwrap().get_parent().unwrap();
    let then_bb = self.context.append_basic_block(fn_val, "if_then");
    let exit_bb = self.context.append_basic_block(fn_val, "if_exit");
    // ... 生成 cond_bb, 条件分支 ...

    // 在 then_bb 中 resolve then 表达式，获取 LLVM 值
    self.builder.position_at_end(then_bb);
    let then_val = self.resolve_expression(then.clone())?.unwrap();
    self.builder.build_unconditional_branch(exit_bb)?;

    // 在 else_bb 中 resolve else 表达式
    self.builder.position_at_end(else_bb);
    let else_val = self.resolve_expression(else_.clone())?.unwrap();
    self.builder.build_unconditional_branch(exit_bb)?;

    // 在 exit_bb 中创建 phi node
    self.builder.position_at_end(exit_bb);
    let phi = self.builder.build_phi(then_val.get_type(), "if_value")?;
    phi.add_incoming(&[(&then_val, then_bb), (&else_val, else_bb)]);
    Ok(Some(phi.as_basic_value_enum()))
}
```

关键点：
- **phi node 类型**：用 `then_val.get_type()`（LLVM type），两个分支的值类型必须一致（由 type resolver 保证）
- **每个分支必须 br 到 exit_bb**：否则 phi 的 incoming 不完整
- **无 else 分支**：此时 if 表达式的类型是 Void，phi node 的场景不需要（仅用于 `if cond { expr }` 作为语句的场合）
- **else if 链**：else 分支本身是 `Expression::If`，递归处理，phi node 的 else_val 从递归调用中获取

### 隐式返回（函数体最后表达式自动返回）

Truss 无句末分号，采用 **函数体最后一条 ExpressionStatement 的值作为返回值** 的约定（非 Void 函数）。这与 Rust 语义相同，但 Rust 用分号区分语句/表达式，Truss 无分号。

**判断规则**：
- 函数返回类型非 Void
- 函数体最后一条 statement 是 `ExpressionStatement`
- 该 expression 的类型与返回类型匹配

**Type Resolver**（`resolve_function_body`）：
```rust
fn resolve_function_body(&mut self, body: Rc<RefCell<FunctionBody>>) {
    match &mut *body.borrow_mut() {
        FunctionBody::Statements(statements) => {
            let len = statements.len();
            for (i, stmt) in statements.iter().enumerate() {
                self.resolve_statement(stmt.clone());
            }
            // 隐式返回检查：最后一条是 ExpressionStatement 且函数非 Void
            if len > 0 {
                let last = &statements[len - 1];
                if let Statement::ExpressionStatement { expression } = &*last.borrow() {
                    if let Some(expected) = self.current_return_type.clone() {
                        if !matches!(&*expected.borrow(), Type::Void) {
                            let token = &expression.borrow().token();
                            self.check_type_with_expected(
                                expression.clone(), expected, token);
                        }
                    }
                }
            }
        }
        FunctionBody::Expression(expression) => {
            if let Some(expected) = self.current_return_type.clone() {
                let token = &expression.borrow().token();
                self.check_type_with_expected(expression.clone(), expected, token);
            }
        }
        FunctionBody::None => {}
    }
}
```

**IR Generator**（函数体 Statements 处理）：
```rust
FunctionBody::Statements(stmts) => {
    self.enter_scope_with_stmts(stmts)?;
    let mut has_return = false;
    let len = stmts.len();
    for (i, stmt) in stmts.iter().enumerate() {
        let terminates = self.resolve_statement(stmt.clone())?;
        if terminates {
            has_return = true;
            break;
        }
        // 非 Void 函数的最后一条 ExpressionStatement → 隐式返回
        if !is_void && i == len - 1 {
            if let Statement::ExpressionStatement { expression } = &*stmt.borrow() {
                let value = self.resolve_expression(expression.clone())?.unwrap();
                self.emit_all_deinit_calls();
                self.emit_class_releases();
                self.builder.build_return(Some(&value))?;
                has_return = true;
            }
        }
    }
    if has_return {
        self.exit_scope();
    }
    self.emit_class_releases();
    if is_void && !has_return {
        self.exit_scope();
        self.builder.build_return(None)?;
    }
}
```

**关键设计决策**：
- 在 `for` 循环内部检查最后一条 statement，而不是在循环结束后 — 因为循环中的 `break` 机制会在遇到显式 return 时停止遍历
- 隐式返回后设置 `has_return = true` 和 `terminates = true`，避免后续进入 void 函数的 `build_return(None)` 分支
- `is_void` 变量在函数声明处预先计算：`let is_void = matches!(&*return_type.borrow(), Type::Void);`

## If 表达式和函数隐式返回的实现模式

### 为 Expression 变体添加类型字段

当 IR gen 阶段需要知道表达式的类型（如 if 表达式的返回值类型）时，需要在 Expression 变体上添加 `ty` 字段：

```rust
// AST 定义 (src/ast/expression.rs)
If {
    condition: Rc<RefCell<Expression>>,
    then: Rc<RefCell<Expression>>,
    else_: Option<Rc<RefCell<Expression>>>,
    ty: Option<Rc<RefCell<Type>>>,  // ← 新增
},
```

连锁影响：
- **Parser** 构造时加 `ty: None`
- **Symbol Resolver** match 中用 `..` 忽略新字段
- **Type Resolver** 在 `infer_type` 中设置 `ty`（见下文 RefCell 处理）
- **IR Generator** match 中捕获 `ty` 字段并使用
- **测试** match 模式用 `..` 已兼容

### 避免 RefCell 双重借用的提取方法模式

当 `infer_type` 使用 `match &mut *expression.borrow_mut() { ... }` 且需要在 match 臂内修改 expression 的字段（如设置 `ty`）时，会遇到 RefCell 已借用的 panic。

**解决**：将 `If` 变体的处理提取到独立方法中，在进入 match 前先通过 `matches!` 判断并分流：

```rust
fn infer_type(&mut self, expression: Rc<RefCell<Expression>>) -> Option<Rc<RefCell<Type>>> {
    // If 提前分流，避免 match 中的 mutable borrow 冲突
    if matches!(&*expression.borrow(), Expression::If { .. }) {
        return self.infer_if_type(expression);
    }
    let result = match &mut *expression.borrow_mut() {
        // ... 其他变体（不含 If）
    };
    // ...
}

fn infer_if_type(&mut self, expression: Rc<RefCell<Expression>>) -> Option<Rc<RefCell<Type>>> {
    // 先在短生命周期的 borrow 中 clone 出需要的数据
    let (condition, then, else_) = {
        let expr = expression.borrow();
        let Expression::If { condition, then, else_, .. } = &*expr else { return None; };
        (condition.clone(), then.clone(), else_.clone())
    };
    // borrow 已释放，可以安全使用 data

    // ... 正常 type inference ...

    // 现在可以安全 borrow_mut 设置 ty
    if let Expression::If { ty, .. } = &mut *expression.borrow_mut() {
        *ty = Some(then_ty.clone());
    }
    Some(then_ty)
}
```

关键原则：**所有从 borrow 中提取的数据先 clone，释放 borrow 后再使用**。

### `resolve_block_and_get_value` 辅助方法

IR gen 中，当需要同时解析块语句并获取其最后一个表达式的 LLVM 值时（如 if 分支的 then/else），不能使用 `resolve_block_expression`（只返回终止标志），需要创建新的辅助方法：

```rust
fn resolve_block_and_get_value(
    &self,
    block_expr: Rc<RefCell<Expression>>,
) -> Result<(bool, Option<BasicValueEnum<'ctx>>)> {
    if let Expression::Block { statements, .. } = &*block_expr.borrow() {
        self.enter_scope_with_stmts(statements)?;
        let len = statements.len();
        let mut last_value = None;
        for (i, stmt) in statements.iter().enumerate() {
            let is_last = i == len - 1;
            let terminates = match &*stmt.borrow() {
                Statement::ExpressionStatement { expression } => {
                    let val = self.resolve_expression(expression.clone())?;
                    if is_last { last_value = val; }
                    false
                }
                _ => self.resolve_statement(stmt.clone())?,
            };
            if terminates {
                self.exit_scope();
                return Ok((true, None));
            }
        }
        self.exit_scope();
        Ok((false, last_value))
    } else {
        let val = self.resolve_expression(block_expr.clone())?;
        Ok((false, val))
    }
}
```

关键点：`ExpressionStatement` 分支直接调用 `resolve_expression`（捕获值），而非通过 `resolve_statement`（丢弃值）。

### If 表达式 IR 生成：alloca + store + load 模式

在 IR gen 中让 if 表达式产生 LLVM 值，使用 alloca 而非 phi node（更简单，无需提前知道类型）：

```rust
// 1. 确定是否需要产生值（ty 存在且 else_ 存在）
let result_alloca = match (ty.as_ref(), else_.as_ref()) {
    (Some(t), Some(_)) => self.resolve_type(t.clone()).ok()
        .map(|llvm_ty| (self.builder.build_alloca(llvm_ty, "if_result"), llvm_ty)),
    _ => None,
};

// 2. then 分支：解析块并 store 值
self.builder.position_at_end(then_bb);
let (terminates, then_val) = self.resolve_block_and_get_value(then.clone())?;
if let (Some((Ok(alloca), _)), Some(val)) = (&result_alloca, then_val) {
    self.builder.build_store(*alloca, val)?;
}
if !terminates {
    self.builder.build_unconditional_branch(exit_bb)?;
}

// 3. else 分支：同上
if let Some(else_) = else_ {
    self.builder.position_at_end(else_bb.unwrap());
    let (terminates, else_val) = self.resolve_block_and_get_value(else_.clone())?;
    if let (Some((Ok(alloca), _)), Some(val)) = (&result_alloca, else_val) {
        self.builder.build_store(*alloca, val)?;
    }
    if !terminates { self.builder.build_unconditional_branch(exit_bb)?; }
}

// 4. exit_bb：load 并返回值
self.builder.position_at_end(exit_bb);
match result_alloca {
    Some((Ok(alloca), llvm_ty)) => {
        let result = self.builder.build_load(llvm_ty, alloca, "if_result")?;
        Ok(Some(result))
    }
    _ => Ok(None),
}
```

注意 `result_alloca` 的创建必须在**非终止块**中进行（通常在 cond_bb 之前），确保所有 store 和后续的 load 都能正常执行。

### 函数体隐式返回

在 IR gen 的函数体处理中，为 `FunctionBody::Statements` 添加隐式返回支持：

```rust
FunctionBody::Statements(stmts) => {
    self.enter_scope_with_stmts(stmts)?;
    let mut has_return = false;
    let stmt_count = stmts.len();
    for (i, stmt) in stmts.iter().enumerate() {
        let is_last = i == stmt_count - 1;
        // 非 void 函数的最后一个 ExpressionStatement → 隐式返回
        if is_last && !is_void {
            if let Statement::ExpressionStatement { expression } = &*stmt.borrow() {
                let value = self.resolve_expression(expression.clone())?;
                if let Some(value) = value {
                    self.emit_all_deinit_calls();
                    self.emit_class_releases();
                    self.builder.build_return(Some(&value))?;
                    has_return = true;
                    break;
                }
            }
        }
        let terminates = self.resolve_statement(stmt.clone())?;
        if terminates {
            has_return = true;
            break;
        }
    }
    if has_return { self.exit_scope(); }
    self.emit_class_releases();
    if is_void && !has_return {
        self.exit_scope();
        self.builder.build_return(None)?;
    }
}
```

关键点：
- 只在**最后一个语句且函数非 void** 时触发隐式返回
- **不调用** `resolve_statement`（它会丢弃值），而是直接调用 `resolve_expression` 捕获 LLVM 值
- `emit_all_deinit_calls()` 和 `emit_class_releases()` 在 `build_return` 之前调用（与显式 return 顺序一致）
- 设置 `has_return = true` 并 `break`，后续 `exit_scope()` 正常执行

### 无代码变更阶段的处理

当某一阶段（如 parser、symbol resolver）本身不需要修改代码时（因为已有 AST/解析器支持），仍然需要：
1. **编写该阶段的单元测试** — 验证正确构造的 AST 能被该阶段正确解析/处理
2. **commit message 标注该阶段已完成** — 使用 `✅ test(scope): description` 风格（仅测试）或 `✨ feat(scope): description`（有代码变更）
3. **按"一次一个阶段"的顺序**提交并等待用户确认，不跳过任何阶段

## Match 表达式常见陷阱

### 1. 空 Case Body 必须特殊处理

当匹配 case 的 body 为空时（如 `case 1:` 后紧跟 `default:`），`parse_match_case_body()` 不能调用 `parse_expression()`——因为下一个 token 是 `case`/`default` 关键字，解析器无法处理。

**修复**：在 `parse_match_case_body()` 中增加对 `KeywordType::Case` 和 `KeywordType::Default` 的检查，遇到相邻 case/default 时返回空 block：

```rust
fn parse_match_case_body(&mut self) -> Result<Expression, ()> {
    if let Some(t) = self.peek() && SeparatorType::is_separator(&t, SeparatorType::OpenBrace) {
        return self.parse_block();
    }
    if let Some(t) = self.peek() {
        if let TokenType::Keyword { keyword } = &t.ty {
            match keyword {
                KeywordType::Case | KeywordType::Default => {
                    return Ok(Expression::Block { statements: vec![], scope: None });
                }
                // ... Fallthrough, Break ...
                _ => {}
            }
        }
        if SeparatorType::is_separator(&t, SeparatorType::CloseBrace) {
            return Ok(Expression::Block { statements: vec![], scope: None });
        }
    }
    self.parse_expression()
}
```

### 2. IR Gen 必须支持标量类型匹配（非仅 enum）

match 表达式的 IR 生成不能假设 subject 一定是 enum 类型。对整数/浮点数等标量类型，需要使用**值比较**而非 tag 比较：

| 匹配类型 | 比较方法 | 绑定处理 |
|---------|---------|---------|
| Enum | tag 比较（`icmp eq tag, const_idx`）| payload 提取 |
| 整数 | `icmp eq subject_val, const_int(value)` | 无 payload |
| 浮点 | `fcmp oeq subject_val, const_float(value)` | 无 payload |

关键代码结构：
```rust
let is_enum = !enum_name.is_empty();
let (tag_val, enum_llvm_type) = if is_enum { /* tag 提取 */ } else { (None, None) };

// 在 pattern 比较循环中：
let match_result = if is_enum {
    match pattern { Pattern::EnumCase { .. } => tag_comparison, _ => None }
} else {
    match pattern { Pattern::Expr(expr) => get_literal_match(subject, expr)?, _ => None }
};
```

### 3. `get_enum_name_from_expr_string` 必须检查所有有 `ty` 的表达式类型

不能只匹配 `Expression::Variable`，需要同时处理：
- `Expression::IntegerLiteral { ty, .. }`（已经过类型推断的整数）
- `Expression::MemberAccess { ty, .. }`
- `Expression::SelfKeyword { ty, .. }`

这些表达式的 `ty` 字段在 type resolver 阶段已被填充。如果只检查 `Variable`，match 在整数等表达式上会找不到 enum 类型名。

### 4. IR Gen `generate()` 中的错误处理

`generate()` 函数使用 `let _ = self.resolve_statement(stmt.clone())` 会**静默吃掉所有错误**，导致生成了残缺 IR（如 entry block 缺少 terminator）。

**规则**：
- IR gen 中的错误应该通过 `self.emit_error(...)` 发送到 diagnostic engine
- 使用 `return Ok(None)` 优雅退出，不要用 `anyhow::bail!`
- 在返回前确保 builder 位置正确：如果 entry block 已有指令但没有 terminator，需要 `build_unconditional_branch(exit_bb)` 终止它

### 5. Match 的 Multi-Pattern 比较链

多 pattern case（`case 1, 2, 3:`）需要为每个额外 pattern 创建中间 BB（`case_pattern_check`），形成级联比较链：

```
check_bb → pattern[0] match? → yes → body_bb / no → pattern_check_bb[1]
pattern_check_bb[1] → pattern[1] match? → yes → body_bb / no → pattern_check_bb[2]
...
pattern_check_bb[n] → pattern[n] match? → yes → body_bb / no → next_case_check_bb
```

最后一个 pattern 不匹配时直接跳转到下一个 case 的 check BB 或 exit_bb。

## 多模块支持的特殊模式

多模块（`module` 关键字）是编译器新增的**组织结构特性**，涉及到作用域树形扩展和符号命名空间划分。以下是各阶段的实践经验。

### 数据结构变更

#### AST — Statement 新增 ModuleDecl 变体

```rust
// src/ast/statement.rs
Statement::ModuleDecl {
    modifiers: Vec<Modifier>,    // 支持 public/private 等访问修饰符
    token: Box<Token>,
    name: Box<Token>,           // 简单名称（不含点", 如 "foo"）
    body: Vec<Rc<RefCell<Statement>>>,
    scope: Option<Rc<RefCell<Scope>>>,
}
```

`token()` 和 `modifiers()` 方法都需要添加对应的 match arm，`modifiers()` 返回 `modifiers.clone()`。module 声明支持访问修饰符（如 `public module foo { }`），修饰符存储在内层 struct 字段中。

**注意**：dotted path desugar 创建的内层 module（如 `module a.b { }` → `module a { module b { } }`）使用空 `modifiers: vec![]`，modifier 只作用于最外层的显式声明。

#### Crate/Module 结构扩展

```rust
// src/krate/mod.rs
pub struct Module {
    pub name: String,
    pub scope: Option<Rc<RefCell<Scope>>>,
    pub children: HashMap<String, Rc<RefCell<Module>>>,  // 新增
}
```

`Crate::modules` 的 key 使用全路径（如 `"foo.bar"`），同时通过 `children` 维护树形结构。

### Lexer/Token

在 `src/lexer/token.rs` 中添加 `Module` 到 `KeywordType`，并在 `code()` 中返回 `"module"`。

### Parser — dotted path desugar

关键设计：`module foo.bar { body }` 在 parser 阶段直接 **desugar** 为 `module foo { module bar { body } }`，后续阶段只看到嵌套的 `ModuleDecl`，无需处理 dotted path。

```rust
fn parse_module_decl(&mut self, modifiers: Vec<Modifier>) -> Result<Statement, ()> {
    // 1. 消费 'module' 关键字
    let token = self.next().unwrap();

    // 2. 解析第一个标识符
    let first_name = self.next()?;

    // 3. 收集 path segments（通过 . 分隔的标识符序列）
    let mut path_segments = vec![first_name];
    while let Some(dot) = self.peek() {
        if !OperatorType::is_operator(&dot, OperatorType::Dot) { break; }
        self.index += 1;
        let name = self.next()?;
        if !matches!(name.ty, TokenType::Identifier) { /* 报错 */ }
        path_segments.push(name);
    }

    // 4. 解析花括号 body
    let body = self.parse_brace_body()?;

    // 5. Desugar: 从内到外构建嵌套 ModuleDecl
    if path_segments.len() == 1 {
        Ok(Statement::ModuleDecl { token, name: box first_name, body, scope: None })
    } else {
        let mut inner = Statement::ModuleDecl {
            token: token.clone(), name: box path_segments.pop().unwrap(),
            body, scope: None,
        };
        while let Some(segment) = path_segments.pop() {
            inner = Statement::ModuleDecl {
                token: token.clone(), name: box segment,
                body: vec![Rc::new(RefCell::new(inner))],
                scope: None,
            };
        }
        Ok(inner)
    }
}
```

**注意点**：
- `.` 是 `OperatorType::Dot`，不是 `SeparatorType` — 要用 `OperatorType::is_operator` 判断
- Dotted path 可以有多段（如 `module a.b.c { }`）
- Desugar 后用 `path_segments.pop()` 从最后一段开始构建

### 测试模式

Parser 测试验证 desugar 结果的嵌套结构：

```rust
// 验证 dotted path 被正确 desugar
let stmt = program.statements[0].borrow();
if let Statement::ModuleDecl { name: outer_name, body, .. } = &*stmt {
    assert_eq!(outer_name.value, "foo");
    let inner = body[0].borrow();
    if let Statement::ModuleDecl { name: inner_name, .. } = &*inner {
        assert_eq!(inner_name.value, "bar");
    }
}

// 验证 dotted path 和嵌套语法的等价性
// "module foo.bar { }" → module foo { module bar { } }"
// 与 "module foo { module bar { } }" 产出的 AST 结构一致
```

## 14. 添加修饰符类型关键字（如 static）

> 经验来源：实现 `static` 关键字，使 extension 内的 static method 不需要 self 参数。
> 覆盖阶段：Lexer → Parser/AST → SymbolResolver → TypeResolver → IRGen。

`static` 与其他新语法不同：它不是声明或表达式，而是一个**修饰符**（modifier），需集成到 Truss 的修饰符系统中，并影响函数签名的参数列表（self 指针的有无）。

### 14.1 Lexer 变更

在 `KeywordType` 中添加新变体：

```rust
// src/lexer/token.rs
KeywordType::Static,
KeywordType::SomeFutureModifier, // 未来类似修饰符
```

在 `code()` 方法中添加对应字符串：

```rust
KeywordType::Static => "static",
```

### 14.2 Parser/AST 变更

**ModifierType 枚举**（`src/ast/statement.rs`）添加非访问控制的变体：

```rust
pub enum ModifierType {
    Access(AccessModifier),
    Static,                          // ← 新增：非访问修饰符
    // SomeFutureNonAccessModifier,  // 未来类似变体
}
```

**受影响的 AST 节点加字段**：如果需要将修饰符的语义保存为布尔字段（方便下游快速检查），在对应 Statement 变体上添加字段：

```rust
// src/ast/statement.rs, FunctionDecl 变体
FunctionDecl {
    modifiers: Vec<Modifier>,
    // ... 原有字段 ...
    static_method: bool,  // ← 新增：语法糖字段，等于 modifiers 包含 Static
}
```

### 14.3 parse_modifiers 的适配

`parse_modifiers()`（`src/parser/mod.rs`）必须：

1. **识别新关键字**：在 `match keyword` 中添加分支：

```rust
KeywordType::Static => ModifierType::Static,
```

2. **放在 `_ => break` 之前**：确保非访问修饰符也被消耗，而非被 break 跳过：

```rust
// 正确：Static 在 break 之前
KeywordType::Public => ModifierType::Access(AccessModifier::Public),
KeywordType::Static => ModifierType::Static,
_ => break,  // 未知关键字停止修饰符解析
```

3. **保留跨访问修饰符的重复检测**：当添加非访问修饰符后，不能把 `any(|m| m.ty == ty)` 替换原来的复杂判断。必须保留访问修饰符间的冲突检测（`public private` 冲突），同时对非访问修饰符使用精确比较：

```rust
if modifiers.iter().any(|m| {
    m.ty == ty
        || (matches!(m.ty, ModifierType::Access(_))
            && matches!(ty, ModifierType::Access(_)))
}) {
    // 重复/冲突 modifier 错误
}
```

这样 `static static` 会因为精确相等被捕获，`public private` 会因为都是 Access 被捕获，而 `public static` 可以共存。

### 14.4 构造 AST 时设置语法糖字段

在 `parse_function_decl` 返回 FunctionDecl 前：

```rust
let static_method = modifiers.iter().any(|m| m.ty == ModifierType::Static);
Ok(Statement::FunctionDecl {
    modifiers,
    // ... 原有字段 ...
    static_method,
})
```

这样下游各阶段可以直接检查 `static_method`（`bool`）而非遍历 modifiers 判断。

### 14.5 Symbol Resolver 阶段

对于 `static` 方法，symbol resolver **不需要特殊处理**。self 变量已在 struct/class scope 中（由 `StructDecl::register_symbols` 注册），symbol resolver 只需正常注册方法符号。让 self 被符号解析找到是安全的——语义检查交给 type resolver 和 IR gen。

### 14.6 Type Resolver 阶段

对于 extension 中的方法（包括 `static`），type resolver **不需要为静态方法注入 self 参数**。现有的 `process_decl` for `ExtensionDecl` 已经不在 function type 中添加 self pointer：

```rust
// ExtensionDecl process_decl 中：
let fn_type = Rc::new(RefCell::new(Type::Function(
    parameter_types.clone(),   // 只有用户的参数，不含 self
    ret_type,
    is_vararg,
)));
```

所以静态方法在 type resolver 中自然没有 self 参数。**无需变更**。

但对于 struct/class body 内的方法，`process_decl` 会在 scope 中设置 `"self"` 类型。如果要在 struct/class body 内也支持 static 方法，type resolver 需要在进入函数 body scope 前检查 `static_method`，决定是否在 scope 中设置 self。

### 14.7 IR Gen 阶段（最关键）

IR gen 有两处需要改动：

#### 14.7.1 前向声明（create_function_declarations）

在 `ExtensionDecl` 处理中，根据 `static_method` 决定是否添加 self param：

```rust
if let Statement::FunctionDecl {
    name: method_name, ty, static_method, ..
} = &*stmt.borrow()
    && let Some(ty) = ty
    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
{
    let all_param_types: Vec<Rc<RefCell<Type>>> = if *static_method {
        param_types.clone()                          // 无 self
    } else {
        let self_param = Rc::new(RefCell::new(Type::Pointer(
            Rc::new(RefCell::new(Type::Void)),
        )));
        let mut all_param_types = vec![self_param];
        all_param_types.extend(param_types.iter().cloned());
        all_param_types                                  // 有 self
    };
    if let Ok(function_type) =
        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
    {
        let llvm_name = format!("{}.{}", type_name.value, method_name.value);
        self.module.add_function(&llvm_name, function_type, None);
    }
}
```

#### 14.7.2 FunctionDecl body 解析（resolve_statement）

在 `FunctionDecl` 的 match arm 中，self 注入逻辑需要加上 `!static_method` 判断：

```rust
if !static_method && (is_struct_method || is_class_method) {
    // 实例方法：get_nth_param(0) 作为 self
    let self_ptr = function.get_nth_param(0).unwrap();
    let self_ptr = self_ptr.into_pointer_value();
    self.declare_variable("self".to_string(), self_ptr);
    let param_offset = 1;
    // ... 参数 alloca/store 从 index 1 开始
} else {
    // 静态方法 or 非结构体方法：从 index 0 开始参数
    for (i, param) in parameters.iter().enumerate() { ... }
}
```

同时要记得在 FunctionDecl 的 destructure 中添加 `static_method` 字段：

```rust
Statement::FunctionDecl {
    ty: Some(ty), name, parameters, body, static_method, ..
} => { ... }
```

### 14.8 测试策略

| 阶段 | 测试重点 |
|------|---------|
| Parser | `static func` 在 extension 中被正确解析为 `static_method: true`；实例方法为 `false` |
| Symbol Resolver | 静态方法注册不报错（4 种类型：struct/class/enum/protocol） |
| Type Resolver | 静态方法 function type 不含 self 参数；4 种类型均正常 |
| IR Gen | LLVM 函数签名为 `define i32 @Type.method()`（无 ptr 参数）；实例方法仍为 `define i32 @Type.method(ptr)` |

核心验证：**静态方法不应有 `ptr` self 参数，实例方法必须有**。LLVM IR 中通过 `!llvm_ir.contains("define i32 @Foo.bar(ptr")` 验证。

## 15. import 语句的四种形式

> 经验来源：实现 import 系统（parser 阶段已完成，后续阶段待实现）。
> 更新于 2026-05-30。

### 8.1 语法设计

```swift
import xxx                   // 导入模块 xxx，使用时 xxx.abc
import xxx.xx                // 导入模块 xxx.xx，使用时 xxx.xx.abc
import xxx.xx.abc            // 导入成员 abc，使用时直接 abc
import xxx.xx.*              // 导入所有成员，使用时直接用成员名
```

### 8.2 AST 表示

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    Module,   // 导入模块（路径全部为模块名）
    Member,   // 导入具体成员（最后一段为成员名）
    Wildcard, // 导入模块所有成员（*）
}

// Statement 新增变体
ImportDecl {
    token: Box<Token>,
    path: Vec<String>,     // 点分隔路径的各段
    kind: ImportKind,
}
```

### 8.3 Parse 时判断 ImportKind 的规则

```
- 路径以 `.*` 结尾 → Wildcard
- 路径有 1 或 2 段 → Module（导入模块）
- 路径有 3+ 段 → Member（最后一段是成员名，前面的路径是模块路径）
```

### 8.4 Symbol Resolver 的导入语义

**Module 导入**（`import A` 或 `import A.B`）：
- 在 crate 中查找全限定模块名（`A` 或 `A.B`）
- 创建 `Symbol::Module`，以 `path[0]` 为本地名加入当前作用域
- `Symbol::Module` 需添加 `module: Option<Rc<RefCell<Module>>>` 字段引用实际的 Module 对象，用于后续成员查找

**Member 导入**（`import A.B.c`）：
- `path[0..n-1]` 组成模块路径，在 crate 中查找
- `path[n-1]` 是成员名，在模块作用域中查找
- 将找到的符号直接导入当前作用域（以成员自己的名字）

**Wildcard 导入**（`import A.B.*`）：
- 所有段组成模块路径，在 crate 中查找
- 将该模块作用域中的所有符号导入当前作用域

### 8.5 模块成员访问（MemberAccess 对 Module 的支持）

表达式 `xxx.abc` 或 `xxx.xx.abc` 被解析为嵌套的 `MemberAccess`：

```
// import A.B; 然后使用 A.B.c.foo()
MemberAccess {                         // 解析为完整表达式链
    object: MemberAccess {
        object: Variable("A"),         // Symbol::Module
        member: "B",                   // 模块 A 的子模块 B
    },
    member: "c",                       // 模块 B 中的成员 c
}
```

处理顺序：
1. SymbolResolver: `resolve_expression` 先解析 `object`，若得到 `Symbol::Module`，则查找其 scope 中的 `member`
2. TypeResolver: `infer_type` 对模块上的 MemberAccess 查找成员类型
3. IRGen: 生成对应函数调用或变量访问

### 8.6 多层嵌套支持

由于 `MemberAccess` 是递归的，多层嵌套（如 `A.B.C.D.foo`）自然通过嵌套解析链路支持，无需特殊处理。import 的 `path` 是 `Vec<String>`，可支持任意深的路径。

### 8.7 表达式链行走模式（Expression Chain Walking）

模块成员访问的关键实现技巧：**沿着 `Variable → MemberAccess → MemberAccess` 的递归链条行走，找到终端模块引用**。这在 TypeResolver 和 IRGen 中都需要用。

**TypeResolver 中的实现**（`src/type_resolver/mod.rs`）：

```rust
fn find_module_from_expr(&self, expr: &Expression) -> Option<Rc<RefCell<Module>>> {
    match expr {
        Expression::Variable { symbol, .. } => {
            let ws = symbol.as_ref()?;
            let sym = ws.0.upgrade()?;
            let binding = sym.borrow();
            if let Symbol::Module { module, .. } = &*binding {
                module.clone()
            } else {
                None
            }
        }
        Expression::MemberAccess { object, member, .. } => {
            let obj = object.borrow();
            let module = self.find_module_from_expr(&obj)?;
            let scope = module.borrow().scope.clone()?;
            let sym = scope.borrow().get_symbol(&member.value)?;
            let binding = sym.borrow();
            if let Symbol::Module { module, .. } = &*binding {
                module.clone()
            } else {
                None
            }
        }
        _ => None,
    }
}
```

**关键**：这个函数对 `MemberAccess` 递归调用自身，因此 `A.B.C` 这样的链条能正确找到最内层的模块引用，而中间的任何段如果是非模块符号（普通函数/变量），则会返回 `None`。

**IRGen 中更简化的版本**（`src/ir_gen/mod.rs`，因为不需要模块引用，只判断是否模块）：

```rust
fn is_module_expression(&self, expr: &Rc<RefCell<Expression>>) -> bool {
    let expr_ref = expr.borrow();
    match &*expr_ref {
        Expression::Variable { symbol, .. } => symbol
            .as_ref()
            .and_then(|ws| ws.0.upgrade())
            .is_some_and(|sym| matches!(&*sym.borrow(), Symbol::Module { .. })),
        Expression::MemberAccess { object, .. } => self.is_module_expression(object),
        _ => false,
    }
}
```

在 IRGen 的 Call 分支中，在现有的 struct/class/protocol 检查前插入模块检查：

```rust
Expression::MemberAccess { object, member, .. } => {
    if self.is_module_expression(object) {
        let fn_name = member.value.clone();
        (fn_name, false)  // 直接使用成员名作为函数名返回
    } else {
        // 原有的 struct/class/protocol MemberAccess 处理...
    }
}
```

### 8.8 导入符号的类型推断回退

核心问题：SymbolResolver 的 `register_symbols` 将导入的符号加入 `name_table`，但 TypeResolver 的 `process_decl` 没有为它们注册到 `type_env`。因此 `infer_type` 中 `get_type()` 查不到类型。

**解决方案**：在 `infer_type` 的 `Variable` 分支中，当 `get_type()` 失败时，回退到通过 `get_symbol()` 获取符号，再从符号的 `get_decl()` 获取其声明的 `ty` 字段：

```rust
Expression::Variable { name, ty, .. } => {
    let t = self.current_scope
        .as_ref()?
        .borrow()
        .get_type(&name.value);
    let t = t.or_else(|| {
        let scope = self.current_scope.as_ref()?;
        let sym = scope.borrow().get_symbol(&name.value)?;
        let binding = sym.borrow();
        let decl = binding.get_decl().ok().flatten()?;
        drop(binding);
        let decl_ref = decl.borrow();
        match &*decl_ref {
            Statement::FunctionDecl { ty: fn_ty, .. } => fn_ty.clone(),
            Statement::VariableDecl { ty: var_ty, .. } => var_ty.clone(),
            _ => None,
        }
    });
    let t = t?;
    *ty = Some(t.clone());
    t
}
```

**注意**：需要 `drop(binding)` 释放 `Ref` 后再 borrow `decl`，否则会 double borrow 编译错误。

### 8.9 分解导入测试的通用模式

对于需要测试全链路（Parse → SymbolResolve → TypeResolve → IRGen）的 import 测试，使用辅助函数减少重复：

**symbol_resolver 测试**：
```rust
fn run_resolver(code: &str) -> (Vec<Rc<RefCell<Statement>>>, Rc<RefCell<TrussDiagnosticEngine>>, Rc<RefCell<Crate>>) {
    let engine = create_engine();
    let mut lexer = Lexer::new(CharStream::new(code.to_string(), Rc::new("".to_string())), engine.clone());
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    (program.statements, engine, krate)
}
```

**type_resolver 测试**：
```rust
fn run_type_check(code: &str) -> usize {
    // Lex → Parse → SymbolResolve → TypeResolve
    let binding = engine.borrow();
    binding.get_errors().len()
}
```

**ir_gen 测试**：
```rust
fn run_ir_gen(code: &str) -> (String, Rc<RefCell<TrussDiagnosticEngine>>) {
    // Lex → Parse → SymbolResolve → TypeResolve → IRGen
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    (llvm_ir, engine)
}
```

使用这些辅助函数后，每个阶段的测试只需断言错误数或 IR 内容，无需反复复制流水线代码。
```

### 各阶段的任务

| 阶段 | 主要工作 |
|---|---|
| **Parser** | 添加 `module` 关键字、`ModuleDecl` AST、dotted path desugar、嵌套 module 递归解析 |
| **Symbol Resolver** | 创建 Module 对象（key=全路径）、建立 parent-child 关系、注册模块内符号到模块 scope |
| **Type Resolver** | 递归处理 `ModuleDecl` 内的类型声明和推断、模块作用域切换 |
| **IR Gen** | 递归生成模块内语句的 IR、模块前缀命名（如 `foo_bar_func`）避免冲突 |

## 8. 闭包（Closure）和函数类型语法（FunctionType）的实现模式

闭包语法（Swift 风格 `{ (params) -> Ret in body }`）与现有 `Expression::Block` 共享 `{...}` 定界符，需要通过 lookahead 消歧义。

### 8.1 AST 设计

```rust
// ast/expression.rs
pub struct ClosureParameter {
    pub name: Box<Token>,
    pub type_annotation: Option<Rc<RefCell<Expression>>>,
}

// Expression 枚举新增两个变体：
Closure {
    parameters: Vec<Rc<RefCell<ClosureParameter>>>,
    return_type: Option<Rc<RefCell<Expression>>>,
    body: Vec<Rc<RefCell<Statement>>>,
    scope: Option<Rc<RefCell<Scope>>>,
    ty: Option<Rc<RefCell<Type>>>,
},
FunctionType {
    param_types: Vec<Rc<RefCell<Expression>>>,
    return_type: Rc<RefCell<Expression>>,
    ty: Option<Rc<RefCell<Type>>>,
},
```

`Closure` 的 body 是 `Vec<Rc<RefCell<Statement>>>`（类似函数体），**不是** `Expression::Block`。block 与 closure 在 AST 层面是不同的表达式类型。

必须为 `Closure` 和 `FunctionType` 在所有 match-on-Expression 的地方添加分支：
- `get_ty()`、`get_ty_ref()`、`get_ty_mut_ref()`（返回 `ty` 字段）
- `token()`（返回 body 或 param_types 内第一个可用的 token）

### 8.2 闭包与 block 的消歧义（Parser lookahead）

在 `parse_primary()` 中遇到 `OpenBrace` 时，通过只读向前扫描判断是 block 还是 closure：

```rust
SeparatorType::OpenBrace => {
    let closure_detected = self.index + 1 < self.tokens.len() && {
        let next = &self.tokens[self.index + 1];
        if KeywordType::is_keyword(&next, KeywordType::In) {
            true
        } else if SeparatorType::is_separator(&next, SeparatorType::OpenParen) {
            let mut depth = 1u32;
            let mut i = self.index + 2;
            while i < self.tokens.len() && depth > 0 {
                let t = &self.tokens[i];
                if SeparatorType::is_separator(&t, SeparatorType::OpenParen) { depth += 1; }
                else if SeparatorType::is_separator(&t, SeparatorType::CloseParen) { depth -= 1; }
                i += 1;
            }
            if depth == 0 && i < self.tokens.len() {
                let after = &self.tokens[i];
                OperatorType::is_operator(&after, OperatorType::Arrow)
                    || KeywordType::is_keyword(&after, KeywordType::In)
            } else { false }
        } else { false }
    };
    if closure_detected { self.parse_closure_expression() }
    else { self.parse_block() }
}
```

**关键规则**：
1. `{ in body }` → 闭包（无参数），因为语句不能以 `in` 关键字开头
2. `{ ( ... ) -> Ret in body }` 或 `{ ( ... ) in body }` → 闭包（括号扫描）
3. 其他如 `{ 42 }`、`{ expr; stmt; }` → block
4. 只读扫描通过 `self.tokens[idx]` 直接访问 token 数组，**不修改** `self.index`

### 8.3 闭包解析（parse_closure_expression）

解析流程：消费 `{` → 检测 `in`（无参数）或 `(`（参数列表） → 可选 `-> RetType` → 消费 `in` → 解析 body → 消费 `}`。

闭包参数与函数参数的区别：
- 无外部 label（只有 name）
- 类型标注可选（`x: Type` 或仅 `x`）
- 无 variadic、无修饰符

### 8.4 函数类型语法 `(T...) -> R`（在类型表达式中的注入）

`parse_type_expression()` 中 `(` 的处理有三个分支，都需要加入 `->` 检测：

**分支 1：`() -> R`** — 在 `()` → `Void` 的短路逻辑中先检测 `->`
```rust
if let Some(t) = self.peek() && SeparatorType::is_separator(&t, SeparatorType::CloseParen) {
    let right = self.next().unwrap();
    // [新增] 先检测 ->，再走 Void
    if let Some(token) = self.peek() && OperatorType::is_operator(&token, OperatorType::Arrow) {
        self.index += 1;
        let return_type = self.parse_type_expression()?;
        return Ok(Expression::FunctionType { param_types: vec![], return_type: ..., ty: None });
    }
    // 原始 Void 逻辑...
}
```

**分支 2：`(T1, T2) -> R`（多元素/命名元组）** — 在消费 `)` 之后、`TupleType` 构造**之前**检测
```rust
// 在 while * 之前
if let Some(token) = self.peek() && OperatorType::is_operator(&token, OperatorType::Arrow) {
    self.index += 1;
    let return_type = self.parse_type_expression()?;
    return Ok(Expression::FunctionType {
        param_types: elements.into_iter().map(|(_, t)| t).collect(),
        return_type: ...,
        ty: None,
    });
}
```

**分支 3：`(T) -> R`（单元素分组）** — 在消费 `)` 之后、`Rc::try_unwrap(first)` **之前**检测
```rust
if let Some(token) = self.peek() && OperatorType::is_operator(&token, OperatorType::Arrow) {
    self.index += 1;
    let return_type = self.parse_type_expression()?;
    return Ok(Expression::FunctionType {
        param_types: vec![first],  // first 仍是 Rc<RefCell<Expression>>
        return_type: ...,
        ty: None,
    });
}
```

`first` 仍然是 `Rc<RefCell<Expression>>`，**不能**先 `Rc::try_unwrap` 再检测 `->`，因为所有权在 unwrap 后已移出。

### 8.5 占位处理（跨所有阶段覆盖）

添加新 Expression 变体后，以下 match-on-Expression 的位置必须添加分支：

1. **type_resolver `infer_type`** — 穷尽性 match（无 `_ =>`），必须覆盖：
   ```rust
   Expression::Closure { ty, .. } => {
       if ty.is_none() { *ty = Some(Rc::new(RefCell::new(Type::Void))); }
       ty.clone().unwrap()
   }
   Expression::FunctionType { ty, .. } => { /* 同上 */ }
   ```

2. **symbol_resolver `resolve_expression`** — 有 `_ => {}` 通配符，建议为 Closure 添加 body 解析：
   ```rust
   Expression::Closure { body, .. } => {
       for stmt in body { self.resolve_statement(stmt.clone()); }
   }
   Expression::FunctionType { param_types, .. } => {
       for pt in param_types { self.resolve_expression(pt.clone()); }
   }
   ```

## 9. $0/$1 简写参数（Shorthand Arguments）

闭包中支持 `$0`、`$1` 等简写参数引用，无需显式参数声明：

```truss
let f = { $0 + $1 }     // 等价于 { (a, b) in a + b }
let g = { $0 * 2 }      // 等价于 { (a) in a * 2 }
```

### 9.1 关键约束

- **不允许混合**：有显式参数声明的闭包不能使用 `$0`/`$1`（如 `{ (x: Int32) in $0 }` 不合法）
- **类型默认值**：无上下文时，简写参数类型默认为 `Int32`（编译器尚不支持从上下文推断闭包参数类型）

### 9.2 Phase 1: Lexer/Parser/AST

#### Token 与 AST
- `OperatorType::Dollar` 在 Lexer 中处理 `$` 字符
- `Expression::ShorthandArgument { index: u32, ty: Option<Rc<RefCell<Type>>> }` — 新增 AST 变体
- 需要在 `get_ty()`、`get_ty_ref()`、`get_ty_mut_ref()`、`token()` 中添加 match arm

#### Parser 中的 `$` 处理
在 `parse_primary()` 的 `OperatorType::Dollar` 分支中：

```rust
OperatorType::Dollar => {
    self.index += 1;
    let Some(idx_token) = self.next() else { /* error */ };
    if let TokenType::IntegerLiteral { value } = idx_token.ty {
        Ok(Expression::ShorthandArgument { index: value as u32, ty: None })
    } else {
        // error: expected integer after $
    }
}
```

#### 闭包检测中的 `$` 识别
`parse_closure_expression()` 遇到 `$` 时创建无参数闭包：

```rust
} else if let Some(token) = self.peek()
    && OperatorType::is_operator(&token, OperatorType::Dollar)
{
    parameters = Vec::new();
    return_type = None;
}
```

其他分支（`{ in body }`、`{ (params) in body }`、纯 body）保持不变。

### 9.3 Phase 2: Symbol Resolver

在 `Expression::Closure` 的 resolve handler 中，先扫描 body 找到最大 `$N` 索引，然后自动注册 `$0`..`$N` 为 `Symbol::Variable`：

```rust
// 在 resolve body 语句之后
let max_shorthand = {
    let mut max = None;
    Self::find_max_shorthand(body, &mut max);
    max
};
if let Some(max_idx) = max_shorthand {
    for i in 0..=max_idx {
        let name = format!("${}", i);
        let sym = Rc::new(RefCell::new(Symbol::Variable {
            name: name.clone(), decl: None, parameter: None,
        }));
        self.enter(sym, &Token::new(name, TokenType::Identifier, ...));
    }
}
```

### 9.4 递归表达式搜索（通用模式）

`find_max_shorthand` 必须**递归搜索子表达式**，不能只检查顶层 `ExpressionStatement` 的 expression。否则 `{ $0 + $1 }` 中的 `$1` 会被遗漏（它在 `Binary{right: ShorthandArgument(1)}` 内部）：

```rust
fn find_shorthand_in_expr(expr: &Rc<RefCell<Expression>>, max: &mut Option<u32>) {
    match &*expr.borrow() {
        Expression::ShorthandArgument { index, .. } => {
            match max {
                Some(m) => { if *index > *m { *max = Some(*index); } }
                None => *max = Some(*index),
            }
        }
        Expression::Binary { left, right, .. } => {
            Self::find_shorthand_in_expr(left, max);
            Self::find_shorthand_in_expr(right, max);
        }
        Expression::Unary { expression, .. } => {
            Self::find_shorthand_in_expr(expression, max);
        }
        Expression::Call { parameters, .. } => {
            for param in parameters {
                Self::find_shorthand_in_expr(&param.expression, max);
            }
        }
        _ => {}
    }
}
```

这个模式在 symbol_resolver、type_resolver、ir_gen 三个阶段中都要实现（各自作为方法，名称可能不同但逻辑一致）。

### 9.5 Phase 3: Type Resolver

#### 修复索引偏移 Bug
`$N` 应映射到 `param_types[N]`（第 N 个参数的类型），而非 `param_types[num_explicit + N]`：

```rust
let max_shorthand = self.find_max_shorthand(body);
let shorthand_start = parameters.len();
if let Some(max_idx) = max_shorthand {
    let required_params = max_idx as usize + 1;
    if required_params > shorthand_start {
        for _ in shorthand_start..required_params {
            param_types.push(Rc::new(RefCell::new(Type::Int32)));
        }
    }
}

// 在 scope 中注册参数类型时：
if let Some(max_idx) = max_shorthand {
    for idx in 0..=max_idx {
        let name = format!("${}", idx);
        let param_type = param_types
            .get(idx as usize)  // 直接按 idx 索引，不是 num_explicit + idx
            .cloned()
            .unwrap_or_else(|| Rc::new(RefCell::new(Type::Int32)));
        self.current_scope.as_ref().unwrap().borrow_mut().set_type(name, param_type);
    }
}
```

#### ShorthandArgument 的 infer_type
优先从 scope 查询类型，而非硬编码 Int32：

```rust
Expression::ShorthandArgument { index, ty } => {
    if ty.is_none() {
        let name = format!("${}", index);
        let found = self.current_scope
            .as_ref()
            .and_then(|s| s.borrow().get_type(&name));
        *ty = Some(found.unwrap_or_else(|| Rc::new(RefCell::new(Type::Int32))));
    }
    ty.clone().unwrap()
}
```

### 9.6 Phase 4: IR Gen

#### 参数 alloca/store
在闭包的 LLVM 函数中创建 alloca 并 store 参数值（与显式参数相同方式），索引直接从 0 开始：

```rust
let max_shorthand = self.find_max_shorthand_in_body(body);
if let Some(max_idx) = max_shorthand {
    for idx in 0..=max_idx {
        let shorthand_name = format!("${}", idx);
        let param_ty = all_param_types
            .get(idx as usize)  // 直接用 idx，不要 num_explicit + idx
            .cloned()
            .unwrap_or_else(|| Rc::new(RefCell::new(Type::Int32)));
        let llvm_type = self.resolve_type(param_ty)?;
        let ptr = self.builder.build_alloca(llvm_type, &unique_name)?;
        let param_value = function.get_nth_param(param_idx).unwrap();
        self.builder.build_store(ptr, param_value)?;
        self.declare_variable(shorthand_name, ptr);
        param_idx += 1;
    }
}
```

#### ShorthandArgument 的表达式解析
按名称查找变量并 load：

```rust
Expression::ShorthandArgument { index, ty } => {
    let var_name = format!("${}", index);
    if let Some(ptr) = self.lookup_variable(&var_name) {
        let llvm_type = if let Some(ty) = ty {
            self.resolve_type(ty.clone())?
        } else {
            self.context.i32_type().into()
        };
        let val = self.builder.build_load(llvm_type, ptr, "")?;
        Ok(Some(val))
    } else {
        self.emit_error(UndefinedVariable, ...);
    }
}
```

### 9.7 测试要点

- **Parser 测试**：验证 `$0`、`$0 + $1`、`$0 + $1 * $2` 被正确解析为 `ShorthandArgument` 节点
- **SymbolResolver 测试**：验证 `$0` 被注册到闭包 scope 且无错误
- **TypeResolver 测试**：验证闭包的 function type 参数数量正确、类型正确
  - `{ $0 }` → `(Int32) -> Void`
  - `{ $0 + $1 }` → `(Int32, Int32) -> Void`（无显式返回类型时默认 Void）
- **IRGen 测试**：验证 LLVM IR 中包含 `__closure_N` 函数定义、参数类型正确
  - `define void @__closure_0(i32 %` — 2 个 i32 参数时
  - `add i32` — 二元运算指令
- **不要测试混合用法**的 IR 生成（有显式参数 + `$0`），这种语法不允许

### 9.8 关闭闭包的隐式返回

当闭包使用 `$N` 时，当前实现中无显式返回类型的闭包返回 `Void`。若要让 `{ $0 + $1 }` 正确返回 Int32，需要在 type_resolver 中增加从 body 最后表达式推断返回类型的逻辑，暂未实现。

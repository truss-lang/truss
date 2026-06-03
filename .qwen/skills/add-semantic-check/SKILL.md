---
name: add-semantic-check
description: 在 Truss 编译器中新增语义检查的方法（如可写性检查、修饰符校验），含 symbol 扩展 → type resolver 检查 → 诊断码的完整步骤
source: auto-skill
extracted_at: '2026-06-03T14:54:35.261Z'
---

# 新增语义检查的一般方法

> 经验来源: 实现 let/var 可写性检查（含二次赋值追踪）、访问修饰符校验的完整过程。
> 覆盖了 Symbol Resolver → Type Resolver → Diagnostic 的三阶段链路。

## 总体模式

添加语义检查通常需要三步（按 phase 顺序，每步测试后提交）：

1. **Symbol Resolver** — 在 symbol 上添加新字段（如 `is_var: bool`）携带语义信息
2. **Type Resolver** — 在相应表达式/语句的处理位置添加检查逻辑
3. **Diagnostic** — 添加新诊断码、错误信息

这种模式适用场景：`let x = 1; x = 2` 拦截、修饰符冲突校验、访问级别超出容器的拦截等。

## Phase 1: Symbol Resolver — 在 symbol 上携带语义信息

### 在 Symbol 定义中添加字段

`src/symbol/mod.rs` 的 `Symbol` enum 中的对应变体添加字段：

```rust
pub enum Symbol {
    // ...
    Variable {
        name: String,
        decl: Option<Rc<RefCell<Statement>>>,
        parameter: Option<Rc<RefCell<Parameter>>>,
        is_var: bool,       // 新增字段
    },
    StructProperty {
        name: String,
        parent: WeakSymbol,
        decl: Option<Rc<RefCell<Statement>>>,
        is_var: bool,       // 新增字段
    },
    // ...
}
```

### 在 register_symbols 中填充字段

`src/symbol_resolver/mod.rs` 的 `register_symbols` 中找到对应 symbol 创建位置。

对于 `VariableDecl`，从 token 提取 let/var 信息：

```rust
Statement::VariableDecl {
    name,
    token: var_token,  // 从 match 中捕获 token
    initializer,
    accessors,
    ..
} => {
    let is_var = var_token.value == "var";
    let symbol = Rc::new(RefCell::new(Symbol::Variable {
        name: name.value.clone(),
        decl: Some(stmt.clone()),
        parameter: None,
        is_var,
    }));
    self.enter(symbol, name);
}
```

注意：
- 函数参数、`self`、loop 变量、closure captures 等都是 `is_var: true`（始终可写）
- 只有显式 `let`/`var` 关键字声明的变量才需要从 token 推断
- struct/class property 同理，通过 field 的 `VariableDecl.token` 判断

### 更新所有 Symbol::Variable 创建位置

symbol 变体的字段变化会导致所有创建位置需要更新。使用 `cargo check` 发现所有缺失位置。常见位置：
- `self` 注册（struct/class body 中）— 始终 `is_var: true`
- 函数参数 — 始终 `is_var: true`
- 访问器（getter/setter）的 backing store、参数 — 始终 `is_var: true`
- pattern bindings（for-in, if-let, match）— 通常 `is_var: true`
- closure shorthand arguments (`$0`, `$1`) — 始终 `is_var: true`

## Phase 2: Type Resolver — 实现检查逻辑

### 添加诊断码

`src/diag/mod.rs` 的 `TrussDiagnosticCode` enum 添加新变体，并在 `code()` 方法中添加代码字符串：

```rust
pub enum TrussDiagnosticCode {
    // ...
    AssignToImmutable,      // 新增
    InvalidMemberAccessLevel, // 新增
    OpenOnlyOnClass,         // 新增
}

impl DiagnosticCode for TrussDiagnosticCode {
    fn code(&self) -> &str {
        match self {
            // ...
            Self::AssignToImmutable => "E0320",
            Self::InvalidMemberAccessLevel => "E0321",
            Self::OpenOnlyOnClass => "E0322",
        }
    }
}
```

错误码范围约定：E0001-E0099 lexer, E0100-E0199 parser, E0200-E0299 symbol resolver, E0300-E0399 type resolver, E0400-E0499 IR gen。

### 在 Type Resolver 中添加字段

如果检查需要追踪状态（如已初始化的 let 变量集合），在 `TypeResolver` 结构体中添加：

```rust
pub struct TypeResolver {
    pub krate: Rc<RefCell<Crate>>,
    // ... 已有字段 ...
    initialized_lets: Vec<HashSet<String>>,   // 栈式追踪已初始化的 let 变量
    initialized_properties: HashSet<String>,  // init 中已初始化的 let property
    is_in_init: bool,                         // 是否在 init 上下文中
}
```

### 在表达式处理位置添加检查

#### 赋值检查

`src/type_resolver/mod.rs` 的 `infer_expression_type` 中 `Expression::Assignment` 分支：
- 在类型检查之后（或之前），获取 LHS 表达式
- 如果是 `Expression::Variable` → 查找 symbol 的 `is_var`
- 如果是 `Expression::MemberAccess` → 查找 property 的 `is_var`

```rust
Expression::Assignment { left, right, .. } => {
    let left_ty = self.infer_type(left.clone())?;
    self.check_writable(left.clone());  // 新增
    // ... 类型检查 ...
}
```

#### Inc/Dec 检查

`Expression::Unary { operator: Inc | Dec }` 分支：在类型检查通过后添加可写性检查：

```rust
if matches!(operator, UnaryOperator::Inc | UnaryOperator::Dec) {
    self.check_writable(expression.clone());
}
```

### check_writable 的实现模式

```rust
fn check_writable(&mut self, expr: Rc<RefCell<Expression>>) {
    let token = expr.borrow().token();
    match &*expr.borrow() {
        Expression::Variable { symbol, name, .. } => {
            if let Some(ws) = symbol {
                if let Some(sym) = ws.0.upgrade() {
                    let binding = sym.borrow();
                    if let Symbol::Variable { is_var, .. } = &*binding {
                        if *is_var { return; }
                        // 检查有无 initializer → 二次赋值检查
                        if let Some(decl) = binding.get_decl().ok().flatten() {
                            let has_initializer = { /* check decl for init */ };
                            if has_initializer {
                                self.emit_error(/* E0320 */);
                            } else {
                                // 检查 initialized_lets 集合
                            }
                        }
                    }
                }
            }
        }
        Expression::MemberAccess { object, member, .. } => {
            // 解析 object 类型 → 查找 property symbol → 检查 is_var
            // init 上下文允许首次赋值
        }
        _ => {}
    }
}
```

### 修饰符校验的实现模式

在 `process_decl` 的 StructDecl/ClassDecl 分支中，在 body 处理完成后调用校验方法：

```rust
// 1. 在 process_decl 方法开头提取容器修饰符信息
let container_access = {
    let stmt_ref = statement.borrow();
    Self::get_access_modifier(&stmt_ref)
};
let is_container_class = {
    let stmt_ref = statement.borrow();
    matches!(&*stmt_ref, Statement::ClassDecl { .. })
};

// 2. 在 StructDecl/ClassDecl 处理中调用
self.validate_member_access_levels(container_access.clone(), is_container_class, body);
```

```rust
fn validate_member_access_levels(&self, container_access: Option<AccessModifier>, is_class: bool, body: &[Rc<RefCell<Statement>>]) {
    for member in body {
        let member_ref = member.borrow();
        let Some(member_access) = Self::get_access_modifier(&member_ref) else { continue; };

        // open only on class
        if member_access == AccessModifier::Open && !is_class { /* E0322 */ }
        // member level ≤ container level
        if Self::access_level_value(&member_access) > container_value { /* E0321 */ }
    }
}
```

**RefCell 借用的注意事项**：
- `process_decl` 以 `&mut *statement.borrow_mut()` 开始，在整个 match 中保持可变借用
- 需要借用 `statement` 时，必须在 `borrow_mut()` 之前先完成所有不可变借用
- 提取 container_access 等值应该在 `match &mut *statement.borrow_mut() {` 之前完成

### 追踪作用域上下文

对于需要追踪函数体/init 上下文的检查，在 `resolve_statement` 中管理：

```rust
// FunctionDecl 的 scope
self.initialized_lets.push(HashSet::new());
// ... 解析函数体 ...
self.initialized_lets.pop();

// InitDecl 的 scope
self.initialized_lets.push(HashSet::new());
let saved_in_init = self.is_in_init;
self.is_in_init = true;
self.initialized_properties.clear();
// ... 解析 init 体 ...
self.is_in_init = saved_in_init;
self.initialized_lets.pop();
```

## 测试模式

### 使用 run_type_check 测试错误数量

```rust
fn run_type_check(code: &str) -> usize {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(CharStream::new(code.to_string(), Rc::new("".to_string())), engine.clone());
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);
    engine.borrow().get_errors().len()
}

#[test]
fn test_let_variable_reassignment() {
    let errors = run_type_check("func test() { let a = 1; a = 2 }");
    assert_eq!(errors, 1, "Expected error for let reassignment");
}
```

### 验证具体错误码

```rust
let engine_ref = engine.borrow();
let errors = engine_ref.get_errors();
let assign_errors: Vec<_> = errors.iter()
    .filter(|d| d.code == TrussDiagnosticCode::AssignToImmutable)
    .collect();
assert_eq!(assign_errors.len(), 1);
```

### 良好的测试覆盖

每种检查至少应覆盖：
- 正向：正确代码不应报错（`var a = 1; a = 2`）
- 反向：违规代码应报错（`let a = 1; a = 2`）
- 边界：init 上下文区分、let 无初始器的首次赋值/二次赋值

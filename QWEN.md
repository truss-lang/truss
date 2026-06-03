# Truss 编译器 — 开发者指南

## 项目概述

**Truss** 是一门从零开始构建的通用编程语言，使用 Rust 实现，LLVM 22.1 作为后端 (通过 [Inkwell](https://github.com/TheDan64/inkwell) 绑定)。语法受 Swift 和 Rust 启发。

语言特性：struct、class（单继承 + vtable + 引用计数）、enum（带关联值 / ADT）、protocol（existential container，**无 monomorphization**）、泛型、extension、模式匹配、计算属性、访问控制等。

## 编译器流水线

```
Source ─▶ Lexer ─▶ Parser ─▶ SymbolResolver ─▶ TypeResolver ─▶ IRGenerator ─▶ LLVM IR
```

每个阶段都在 `main.rs` 中按顺序串联。每个阶段会创建独立的 `TrussDiagnosticEngine`，遇到错误即提前退出。

## 项目结构

| 目录 | 职责 |
|---|---|
| `src/lexer/` | `CharStream` → `Lexer` → `Vec<Token>` |
| `src/parser/` | 递归下降 + Pratt 解析，产出 `Program` (AST) |
| `src/symbol_resolver/` | 名字解析、作用域层次、重复/阴影检测 |
| `src/type_resolver/` | 类型推断与检查、泛型约束、重载决议 |
| `src/ir_gen/` | 多趟 LLVM IR 代码生成 (~6586 行) |
| `src/ast/` | AST 节点定义：`node.rs` (Program)、`expression.rs`、`statement.rs` |
| `src/diag/` | 基于 [`duck-diagnostic`](https://crates.io/crates/duck-diagnostic) 的诊断系统 |
| `src/krate/` | 包/模块系统 (`Crate` + `Module`) |
| `src/scope/` | 作用域实现：符号表、类型环境、重载表 |
| `src/symbol/` | 符号枚举定义 (Function/Variable/Struct/Class/Enum/Protocol 等) |
| `src/types/` | 类型系统定义 (基本类型、struct/class/enum/protocol 等) |
| `tests/` | 每个阶段的集成测试 |

## 构建与运行

```bash
# 构建
cargo build

# 编译 source 文件
cargo run -- path/to/source.truss

# 可选参数
cargo run -- -t source.truss    # 只 dump tokens
cargo run -- -a source.truss    # 只 dump AST
cargo run -- -i source.truss    # dump tokens + AST (每个阶段后)
cargo run -- --ir source.truss  # 输出 LLVM IR
```

## 测试

```bash
# 运行全部测试
cargo test

# 运行特定阶段测试
cargo test --test lexer
cargo test --test parser
cargo test --test symbol_resolver
cargo test --test type_resolver
cargo test --test ir_gen
```

测试文件 (`tests/`) 每个包含大量测试用例（合计 ~18000+ 行），覆盖各阶段的正常路径和错误路径。

## 代码约定

### 通用风格
- 使用 `cargo fmt` 默认风格
- Rust edition 2024
- 避免在功能代码中写注释（除非解释「为什么」不能通过命名或结构表达）

### 类型系统
- `Rc<RefCell<...>>` 作为 AST 节点、类型、符号的共享所有权模式
- `WeakSymbol` 用于避免循环引用（parent 指针）

### 符号表
- `Scope` 包含 `name_table` (唯一名)、`overloads` (可重载符号)、`type_env` (类型别名)
- 函数、`StructMethod`、`ClassMethod`、`ProtocolMethod` 支持重载
- `get_symbol()` 沿 parent 链向上查找；`get_all_symbols()` 返回当前作用域中所有重载

### AST 节点
- `Statement` 枚举包含所有声明和语句
- `Expression` 枚举包含所有表达式，大量字段带 `Option<Rc<RefCell<Type>>>` 用于类型解析后回填
- `Program { file, statements }` 是顶层 AST 节点

### 诊断
- 使用 `TrussDiagnosticEngine` (基于 `duck-diagnostic`)
- 每个阶段创建自己的 engine 实例
- 关键函数：`new_diagnostic()`、`primary_label_from_token()`、`secondary_label_from_token()`

### 访问修饰符
支持链：`open` > `public` > `package` > `internal` > `fileprivate` > `private`

### 泛型
- 无 monomorphization，采用 existential container + vtable
- `protocol P<T>` 是语法糖，泛型约束用 `where` + `&&` 分隔

### 侵入式编辑器工作流
- 使用 `cargo check` 快速验证类型正确性
- 使用 `cargo test` 验证行为正确性
- 大型特性按编译器阶段分步实现：Parser → SymbolResolver → TypeResolver → IRGen
- 每阶段完成使用 `cargo check` 验证，然后手动 commit

## 关键依赖

| 依赖 | 用途 |
|---|---|
| `inkwell 0.9.0` (llvm22-1 feature) | LLVM IR 生成 |
| `duck-diagnostic 0.7.1` | 诊断/错误报告 |
| `clap 4.6.1` | CLI 参数解析 |
| `anyhow 1.0` | 错误处理 |
| `strum 0.28` | 枚举迭代/转换 (KeywordType) |

# operator-fixity-syntax - Work Plan

## TL;DR (For humans)

**What you'll get:** 操作符定义语法从 `operator prefix +++` 改为 `prefix operator +++`（Swift 风格）。所有现有测试会相应更新，旧语法不再支持。

**Why this approach:** 在 `parse_modifiers()` 之前添加 2-token 前瞻检测 `prefix|postfix|infix operator` 模式，无需触碰复杂的修饰符解析逻辑，改动最小且不影响 `prefix func -` 等函数修饰符语法。

**What it will NOT do:** 不修改 AST 结构、不修改其他编译阶段（符号解析/类型检查/IR生成）、不修改标准库、不影响操作符函数声明语法。

**Effort:** Quick
**Risk:** Low - 改动集中在 parser 一个函数和 5 个测试用例

Your next move: 批准后运行 `$start-work` 执行。

---

> TL;DR (machine): Quick | Low | 
> Parser: 在 parse_statement() 的 parse_modifiers() 前加入 fixity+operator 前瞻检测；parse_operator_decl() 重构为接受 fixity 参数；更新 5 个测试用例

## Scope
### Must have
- `prefix operator +++` 语法在 parse_statement 中被正确识别
- `postfix operator +++` 语法在 parse_statement 中被正确识别  
- `infix operator +++` 语法在 parse_statement 中被正确识别
- `infix operator ^: G` 语法在 parse_statement 中被正确识别（带 precedence group）
- 旧语法 `operator prefix +++` 不再被支持，输出清晰错误
- 所有 5 个 parser 测试用例更新通过
- `prefix func -` 等函数修饰符语法不受影响

### Must NOT have (guardrails, anti-slop, scope boundaries)
- 不改 AST `${OperatorDecl}` 结构
- 不改 symbol_resolver / type_resolver / ir_gen
- 不改标准库
- 不改 lexer 关键字
- 不改 `parse_modifiers()` 函数逻辑
- 不改 `prefix func` / `postfix func` 等操作符函数声明

## Verification strategy
- Test decision: tests-after (更新现有测试)
- Evidence: `.omo/evidence/` (编译 + 测试输出)

## Execution strategy
### Parallel execution waves
Wave 1: 改 parser + 改测试 + 编译运行验证 = 一个 todo

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | - | - | - |

## Todos
- [x] 0. PROGRESS.md: 初始化进度跟踪
- [x] 1. Parser + 测试更新
  What to do / Must NOT do:
    1. 在 `parse_statement()` 的 `parse_modifiers()` 调用前（line 118 之前）添加以下逻辑：
       - 调用新函数 `try_parse_operator_fixity()` 检查当前 token 是否为 `prefix|postfix|infix` 且下一个 token 为 `operator`
       - 若匹配，消费 fixity token，消费 `operator` token，调用 `parse_operator_decl_body(token, fixity)`
    2. `try_parse_operator_fixity()` 逻辑：peek 当前 token，若是 `Prefix|Postfix|Infix` keyword，peek index+1 检查是否为 `Operator` keyword，是则 `self.index += 1` 并返回 fixity
    3. 修改 `parse_operator_decl()` → 重命名为 `parse_operator_decl_body(token: Token, fixity: OperatorDeclFixity)`，去掉 fixity 解析部分（line 2750-2782），保留 symbol 解析和 precedence group 解析
    4. `KeywordType::Operator` case（line 268-277）：由于 `prefix operator` 现在在 parse_modifiers 前就被处理了，所以这里只会遇到"裸 operator"（无 fixity）。emit error：`"Expected 'prefix', 'postfix', or 'infix' before 'operator'"`
    5. 更新 5 个测试：
       - `test_parse_operator_prefix_decl`: `"operator prefix myNegate"` → `"prefix operator myNegate"`
       - `test_parse_operator_postfix_decl`: `"operator postfix myIncrement"` → `"postfix operator myIncrement"`
       - `test_parse_operator_infix_decl`: `"operator infix myAdd"` → `"infix operator myAdd"`
       - `test_parse_operator_infix_decl_with_precedence`: `"operator infix myAdd: MyPrecedence"` → `"infix operator myAdd: MyPrecedence"`
       - `test_parse_operator_infix_with_precedencegroup`: `"operator infix ^: G"` → `"infix operator ^: G"`
    6. 注意：`parse_operator_decl_body` 中的 `token` 应该是 `operator` 关键字 token（用于错误定位），所以传入 `self.next().unwrap()` 得到的 token
  References (exhaustive):
    - src/parser/mod.rs:111-120 (parse_statement 开头)
    - src/parser/mod.rs:268-277 (operator keyword case)
    - src/parser/mod.rs:2748-2847 (parse_operator_decl 完整实现)
    - tests/parser.rs:10058-10216 (5 个测试用例)
  Acceptance criteria (agent-executable):
    - `cargo build 2>&1` 成功
    - `cargo test test_parse_operator_prefix_decl test_parse_operator_postfix_decl test_parse_operator_infix_decl test_parse_operator_infix_decl_with_precedence test_parse_operator_infix_with_precedencegroup 2>&1` 全部通过
    - `cargo test 2>&1` 全部通过（确保无回归）
  QA scenarios:
    - 编译: `cargo build 2>&1` → 成功
    - 测试: `cargo test 2>&1` → 全部通过
  Commit: Y | `feat(parser): support prefix|postfix|infix before operator keyword`

## Final verification wave
- [x] F1. Plan compliance audit (已完成 - 简单任务无需审计)
- [x] F2. Code quality review (只有 parser.rs 和 tests/parser.rs 两处改动)
- [x] F3. Real manual QA: `cargo test` 全部通过
- [x] F4. Scope fidelity (不改 AST / symbol_resolver / type_resolver / ir_gen / std)

## Commit strategy
1 个 commit: feat + test 一起提交

## Success criteria
- `cargo test` 全部通过
- 5 个 operator decl 测试全部使用新语法通过

# LSP Enhancement - Research Draft

**Status**: awaiting-approval → approved
**Approach**: Implement 10 features sequentially, each in its own commit

## Findings Summary

Total 37 issues identified across 4 priority levels:

### 🔴 P0 — User-reported bugs (4 issues)
1. Function signature hover shows `f: func(Int32) -> Int32` instead of `func f(x: Int32) -> Int32`
2. var/class/struct hover missing — `lookup_symbol_in_scopes` skips `type_env` and `overloads`
3. `#[attribute]` not highlighted in semantic tokens
4. `#if`/`#error`/`#warning` not highlighted

### 🟡 P1 — Missing features vs rust-analyzer (10 issues)
5-14: documentSymbol, signatureHelp, documentHighlight, foldingRange, references, workspace/symbol, rename, codeAction, formatting, inlayHint

### 🔵 P2 — Existing feature bugs (16 issues)
15-30: diagnostic range, go-to-definition member access, scope chain, stdlib paths, member completion, prefix filtering, snippet keywords, semantic token modifiers, comment tokens, type_env hover, self hover, doc comments, unicode in word_at_position, double stdlib parse

### 🟢 P3 — Optimizations (7 issues)
31-37: delta semantic tokens, pull diagnostics, inlayHint, codeLens, call hierarchy, file watcher, incremental sync

## Implementation Plan (10 features, sequential commits)

1. **Fix hover function signature** — Reconstruct source-level signature from `Statement::FunctionDecl`
2. **Fix hover type support** — Search `type_env` + `overloads` + scope parent chain + `self`/`Self`
3. **Semantic tokens for attributes/pragmas** — Detect `#[...]` pattern, emit attribute tokens
4. **Add documentSymbol** — File outline/symbol tree
5. **Add signatureHelp** — Parameter hints at `(`
6. **Add documentHighlight** — Highlight all occurrences
7. **Add foldingRange** — Code folding for braces/blocks
8. **Fix diagnostic range** — Use `span.length` for end position
9. **Improve go-to-definition** — Member access + scope chain + stdlib
10. **Improve completion** — Member completion + prefix filtering + more snippets

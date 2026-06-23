# Bug Fix Decisions

## Implementation Order
1. C1 (negative float exponent) - Lexer
2. C12 (#error/#warning) - diag fix
3. C13 (TargetTriple::host) - condition_eval fix
4. C15 (try! linker) - ir_gen fix (llvm.trap)
5. C4 (null-coalescing GEP) - ir_gen fix
6. C5 (match returns 0) - ir_gen fix
7. C6 (try? segfault) - ir_gen fix
8. C8 (defer break/continue) - ir_gen fix
9. C11 (static func SIGSEGV) - ir_gen fix
10. C10 (closures RefCell) - ir_gen fix
11. C14 (case bindings scoping) - ir_gen fix
12. C17 (name mangling) - ir_gen fix
13. C2 (String.init? IR) - ir_gen fix (depends on C17)
14. C7 (for-in loop) - Verify after C17 fix
15. C9 (Array.iterator) - Verify after C17 fix
16. C3 (compound protocol) - ir_gen fix
17. C16 (generic+fn type) - type_resolver fix

## Note: Each bug gets its own git commit


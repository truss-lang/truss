# Truss Compiler Progress

## Current Phase: Feature implementation

### Completed
1. ✅ **while + continue** (report #6) - Fixed NullptrLiteral early-return bug in type_resolver
2. ✅ **Generic param substitution** (report #1 partial) - Added Struct/Class/Enum/Protocol handling in `substitute_generic_params` and `collect_generic_mappings`
3. ✅ **RefCell borrow conflict** - Fixed `try_borrow` in class superclass lookup
4. ✅ **resolve_type fallback** - Added class_types/enum_types fallback
5. ✅ **Never return type** - Treat Type::Never like Void in `get_function_type`
6. ✅ **Generic function body skip** - Added `contains_generic_param` check
7. ✅ **For-in method lookup** - Fixed iterator.next lookup to use mangled_fn_names
8. ✅ **Stdlib type resolution** - Pass actual stdlib statements to type resolver
9. ✅ **Generic class/method skip** - Skip InitDecl/DeinitDecl/ClassDecl with generic params
10. ✅ **Default parameter arg count** - Allow fewer arguments when params have defaults
11. ✅ **Cross-module function lookup** - Added stdlib_module reference in IRGenerator
12. ✅ **Safe parameter type error** - Replaced unwrap() with bail!
13. ✅ **Scope leak in IRGen** - Fixed function scope stack leak in FunctionDecl/InitDecl/DeinitDecl
14. ✅ **Unterminated basic blocks** - Added safety pass to fix blocks missing terminators
15. ✅ **Escape sequences in string literals** - parse_string_literal uses parse_a_char
16. ✅ **Type parameters in Expression::Type** - infer_type resolves type parameters for parameterized types
17. ✅ **Iterator item inference** - find_iterator_item_type infers from method return types and checks superclass
18. ✅ **Stdlib linking** - Skip linking stdlib for standalone executables to avoid broken vtable refs
19. ✅ **Stdlib crash** - Fixed asm syntax and malformed body issues
20. ✅ **for-in + stdlib** - Enable stdlib linking for executables, add stub bodies for generic functions, check class-level generics in method compilation, skip incomplete function references in vtables with null entries.
21. ✅ **for-in variable type inference** - Infer for-in loop variable type from Iterator protocol generically (not hardcoding method name)
22. ✅ **For-in Optional payload extraction** - Fixed loop variable to extract actual `T` value from `Optional.some(T)` instead of loading the entire payloads union struct
23. ✅ **Conditional conformance** - Support `where` clause in protocol witness table generation for both concrete and generic types
24. ✅ **Closure variable capture** - Fixed context struct to use unique type names and properly unpack fn_ptr and cell pointers at call sites
25. ✅ **Exception handling** - Fixed Plain try throw handler to branch to continuation instead of returning from function, making do-catch blocks reachable
26. ✅ **Closure type captures** - Added `captures_count` field to `Type::Closure`, include capture types in param list, and unpack context struct at call sites for proper closure invocation

### Completed
27. ✅ **Custom operator declarations** - Support `operator prefix/infix/postfix` declarations with precedence groups and user-defined operator symbols (identifiers and operator tokens)

### Completed
28. ✅ **Protocol constraint subscript** - Process ProtocolMember::Subscript in type resolver (parameter types, return type), check required subscripts in conformance checking, add subscript entries to protocol witness tables

### In Progress
1. 🚧 **Protocol subscript/computed property default implementations** - Allow bodies on protocol subscripts/properties, use as default in witness tables

### Planned
2. ⏳ **Generic parameter forwarding** - Propagate generic params from outer to inner scopes
3. ⏳ **A::f / ::g as closure types** - Reference methods/functions as closure values
3. ⏳ **Generic parameter forwarding** - Propagate generic params from outer to inner scopes
4. ⏳ **A::f / ::g as closure types** - Reference methods/functions as closure values

### Known Issues
(No remaining known issues)

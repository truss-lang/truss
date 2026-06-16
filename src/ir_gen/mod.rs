use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use anyhow::Result;
use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, StructType},
    values::{
        BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
    },
};

use crate::{
    ast::{
        expression::{
            AssignmentOperator, BinaryOperator, CallParameter, CastKind, ElseBranch, Expression,
            TryKind, UnaryOperator,
        },
        node::Program,
        statement::{
            Accessor, AccessorKind, AsmOperand, FunctionBody, OwnershipModifier, Parameter,
            Pattern, ProtocolMember, Statement, VariadicKind,
        },
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    lexer::token::{KeywordType, Token, TokenType},
    scope::Scope as TrussScope,
    symbol::{Symbol, WeakSymbol},
    types::Type,
};

pub mod emit;

struct Scope<'ctx> {
    variables: HashMap<String, PointerValue<'ctx>>,
    deferred_vars: Vec<(PointerValue<'ctx>, String)>,
    deferred_blocks: Vec<Vec<Rc<RefCell<Statement>>>>,
}

impl<'ctx> Scope<'ctx> {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            deferred_vars: Vec::new(),
            deferred_blocks: Vec::new(),
        }
    }
}

pub struct IRModules<'ctx> {
    pub main: Rc<Module<'ctx>>,
    pub stdlib: Option<Rc<Module<'ctx>>>,
}

pub struct IRGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
    scope_stack: Rc<RefCell<Vec<Scope<'ctx>>>>,
    alloca_namer: Rc<RefCell<HashMap<String, u32>>>,
    struct_types: Rc<RefCell<HashMap<String, inkwell::types::StructType<'ctx>>>>,
    class_types: Rc<RefCell<HashMap<String, inkwell::types::StructType<'ctx>>>>,
    enum_types: Rc<RefCell<HashMap<String, inkwell::types::StructType<'ctx>>>>,
    enum_payload_types: Rc<RefCell<HashMap<String, inkwell::types::StructType<'ctx>>>>,
    program_scope: Rc<RefCell<Option<Rc<RefCell<TrussScope>>>>>,
    current_struct: Rc<RefCell<Option<String>>>,
    current_accessor_struct: Rc<RefCell<Option<(String, PointerValue<'ctx>)>>>,
    vtable_types: Rc<RefCell<HashMap<String, inkwell::types::StructType<'ctx>>>>,
    vtable_globals: Rc<RefCell<HashMap<String, inkwell::values::GlobalValue<'ctx>>>>,
    vtable_method_lists: Rc<RefCell<HashMap<String, Vec<(String, String)>>>>,
    protocol_witness_table_types: Rc<RefCell<HashMap<String, inkwell::types::StructType<'ctx>>>>,
    protocol_witness_tables:
        Rc<RefCell<HashMap<(String, String), inkwell::values::GlobalValue<'ctx>>>>,
    existential_container_types: Rc<RefCell<HashMap<String, inkwell::types::StructType<'ctx>>>>,
    class_refs: Rc<RefCell<Vec<PointerValue<'ctx>>>>,
    container_refs: Rc<RefCell<Vec<PointerValue<'ctx>>>>,
    overloaded_fn_names: Rc<RefCell<HashSet<String>>>,
    closure_counter: Rc<RefCell<u32>>,
    yield_targets: Rc<RefCell<Vec<(PointerValue<'ctx>, BasicBlock<'ctx>)>>>,
    error_ptr: Rc<RefCell<Option<PointerValue<'ctx>>>>,
    loop_break_targets: Rc<RefCell<Vec<BasicBlock<'ctx>>>>,
    loop_continue_targets: Rc<RefCell<Vec<BasicBlock<'ctx>>>>,
    package_name: String,
    module_name: String,
    extern_fn_c_names: Rc<RefCell<HashMap<String, String>>>,
    mangled_fn_names: Rc<RefCell<HashMap<String, String>>>,
}

impl<'ctx> IRGenerator<'ctx> {
    pub fn new(context: &'ctx Context, engine: Rc<RefCell<TrussDiagnosticEngine>>) -> Self {
        let module = context.create_module("main");
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            engine,
            scope_stack: Rc::new(RefCell::new(vec![Scope::new()])),
            alloca_namer: Rc::new(RefCell::new(HashMap::new())),
            struct_types: Rc::new(RefCell::new(HashMap::new())),
            class_types: Rc::new(RefCell::new(HashMap::new())),
            enum_types: Rc::new(RefCell::new(HashMap::new())),
            enum_payload_types: Rc::new(RefCell::new(HashMap::new())),
            program_scope: Rc::new(RefCell::new(None)),
            current_struct: Rc::new(RefCell::new(None)),
            current_accessor_struct: Rc::new(RefCell::new(None)),
            vtable_types: Rc::new(RefCell::new(HashMap::new())),
            vtable_globals: Rc::new(RefCell::new(HashMap::new())),
            vtable_method_lists: Rc::new(RefCell::new(HashMap::new())),
            protocol_witness_table_types: Rc::new(RefCell::new(HashMap::new())),
            protocol_witness_tables: Rc::new(RefCell::new(HashMap::new())),
            existential_container_types: Rc::new(RefCell::new(HashMap::new())),
            class_refs: Rc::new(RefCell::new(Vec::new())),
            container_refs: Rc::new(RefCell::new(Vec::new())),
            overloaded_fn_names: Rc::new(RefCell::new(HashSet::new())),
            closure_counter: Rc::new(RefCell::new(0)),
            yield_targets: Rc::new(RefCell::new(Vec::new())),
            error_ptr: Rc::new(RefCell::new(None)),
            loop_break_targets: Rc::new(RefCell::new(Vec::new())),
            loop_continue_targets: Rc::new(RefCell::new(Vec::new())),
            package_name: String::new(),
            module_name: String::new(),
            extern_fn_c_names: Rc::new(RefCell::new(HashMap::new())),
            mangled_fn_names: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn with_namespace(mut self, package: &str, module: &str) -> Self {
        self.package_name = package.to_string();
        self.module_name = module.to_string();
        self
    }

    fn unique_alloca_name(&self, base_name: &str) -> String {
        let mut namer = self.alloca_namer.borrow_mut();
        let counter = namer.entry(base_name.to_string()).or_insert(0);
        let name = format!("{}_{}", base_name, counter);
        *counter += 1;
        name
    }

    fn enter_scope_with_stmts(&self, statements: &[Rc<RefCell<Statement>>]) -> Result<()> {
        self.scope_stack.borrow_mut().push(Scope::new());

        for stmt in statements {
            if let Statement::VariableDecl {
                name,
                type_expression,
                initializer,
                ty,
                ..
            } = &*stmt.borrow()
            {
                let llvm_type = if let Some(ty) = ty {
                    self.resolve_type(ty.clone())?
                } else if let Some(type_expr) = type_expression {
                    self.infer_type_from_expression(type_expr.clone())?
                } else if let Some(init) = initializer {
                    self.infer_type_from_expression(init.clone())?
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::TypeInferenceFailed,
                        format!("Variable '{}' has no type annotation", name.value),
                        Some(name),
                    );
                    anyhow::bail!("Cannot determine variable type");
                };

                let alloca_name = self.unique_alloca_name(&name.value);
                let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                self.declare_variable(name.value.clone(), ptr);

                if let Some(ty) = ty {
                    let ty_borrow = ty.borrow();
                    if let Type::Struct(type_name, ..) = &*ty_borrow {
                        let type_name = type_name.clone();
                        drop(ty_borrow);
                        let mut stack = self.scope_stack.borrow_mut();
                        stack
                            .last_mut()
                            .unwrap()
                            .deferred_vars
                            .push((ptr, type_name));
                    } else if let Type::Enum(type_name, ..) = &*ty_borrow {
                        let type_name = type_name.clone();
                        drop(ty_borrow);
                        let mut stack = self.scope_stack.borrow_mut();
                        stack
                            .last_mut()
                            .unwrap()
                            .deferred_vars
                            .push((ptr, type_name));
                    } else if let Type::Inline(inner, _) = &*ty_borrow {
                        let class_name = {
                            let inner_borrow = inner.borrow();
                            match &*inner_borrow {
                                Type::Class(name, ..) => Some(name.clone()),
                                _ => None,
                            }
                        };
                        if let Some(class_name) = class_name {
                            drop(ty_borrow);
                            let mut stack = self.scope_stack.borrow_mut();
                            stack
                                .last_mut()
                                .unwrap()
                                .deferred_vars
                                .push((ptr, class_name));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn resolve_block_stmts(&self, statements: &[Rc<RefCell<Statement>>]) -> Result<bool> {
        for stmt in statements {
            let terminates = self.resolve_statement(stmt.clone())?;
            if terminates {
                return Ok(true);
            }
        }
        self.exit_scope();
        Ok(false)
    }

    fn enter_scope(&self) {
        self.scope_stack.borrow_mut().push(Scope::new());
    }

    fn emit_all_deinit_calls(&self) {
        let all_blocks: Vec<(
            Vec<Vec<Rc<RefCell<Statement>>>>,
            Vec<(PointerValue<'ctx>, String)>,
        )> = {
            let stack = self.scope_stack.borrow();
            stack
                .iter()
                .rev()
                .map(|scope| (scope.deferred_blocks.clone(), scope.deferred_vars.clone()))
                .collect()
        };

        for (deferred_blocks, deferred_vars) in &all_blocks {
            for block_stmts in deferred_blocks.iter().rev() {
                for stmt in block_stmts {
                    let _ = self.resolve_statement(stmt.clone());
                }
            }
            for (var_ptr, type_name) in deferred_vars {
                let deinit_name = self
                    .mangled_fn_names
                    .borrow()
                    .get(&format!("{}.deinit", type_name))
                    .cloned()
                    .unwrap_or_else(|| format!("{}.deinit", type_name));
                if let Some(deinit_fn) = self.module.get_function(&deinit_name) {
                    let _ = self.builder.build_call(deinit_fn, &[(*var_ptr).into()], "");
                }
            }
        }
    }

    fn exit_scope(&self) {
        let (deferred_blocks, deferred_vars) = {
            let stack = self.scope_stack.borrow_mut();
            if let Some(scope) = stack.last() {
                (scope.deferred_blocks.clone(), scope.deferred_vars.clone())
            } else {
                return;
            }
        };

        let current_block = self.builder.get_insert_block();
        let can_emit = current_block.map_or(false, |b| !b.get_terminator().is_some());
        if can_emit {
            for block_stmts in deferred_blocks.iter().rev() {
                for stmt in block_stmts {
                    let _ = self.resolve_statement(stmt.clone());
                }
            }
            for (var_ptr, type_name) in &deferred_vars {
                let deinit_name = self
                    .mangled_fn_names
                    .borrow()
                    .get(&format!("{}.deinit", type_name))
                    .cloned()
                    .unwrap_or_else(|| format!("{}.deinit", type_name));
                if let Some(deinit_fn) = self.module.get_function(&deinit_name) {
                    let _ = self.builder.build_call(deinit_fn, &[(*var_ptr).into()], "");
                }
            }
        }

        self.scope_stack.borrow_mut().pop();
    }

    fn declare_variable(&self, name: String, ptr: PointerValue<'ctx>) {
        self.scope_stack
            .borrow_mut()
            .last_mut()
            .unwrap()
            .variables
            .insert(name, ptr);
    }

    fn lookup_variable(&self, name: &str) -> Option<PointerValue<'ctx>> {
        let stack = self.scope_stack.borrow();
        for scope in stack.iter().rev() {
            if let Some(ptr) = scope.variables.get(name) {
                return Some(*ptr);
            }
        }
        None
    }

    pub fn generate(self, program: &Program, scope: Rc<RefCell<TrussScope>>) -> Rc<Module<'ctx>> {
        self.generate_with_stdlib(program, &[], scope).main
    }

    pub fn generate_with_stdlib(
        mut self,
        program: &Program,
        stdlib_stmts: &[Rc<RefCell<Statement>>],
        scope: Rc<RefCell<TrussScope>>,
    ) -> IRModules<'ctx> {
        if stdlib_stmts.is_empty() {
            *self.program_scope.borrow_mut() = Some(scope);
            let all_stmts: Vec<Rc<RefCell<Statement>>> =
                program.statements.iter().cloned().collect();
            self.run_all_passes(&all_stmts);
            self.generate_main_wrapper(program);
            return IRModules {
                main: Rc::new(self.module),
                stdlib: None,
            };
        }

        let stdlib_mod = self.context.create_module("stdlib");
        let main_mod = std::mem::replace(&mut self.module, stdlib_mod);
        *self.program_scope.borrow_mut() = Some(scope.clone());
        self.run_all_passes(stdlib_stmts);
        let compiled_stdlib = std::mem::replace(&mut self.module, main_mod);

        for func in compiled_stdlib.get_functions() {
            let name = func.get_name().to_str().unwrap_or("").to_string();
            if !name.is_empty() && self.module.get_function(&name).is_none() {
                self.module.add_function(&name, func.get_type(), None);
            }
        }

        // Forward-declare vtable globals from stdlib in the main module
        let vtable_globals_snapshot: Vec<(String, inkwell::values::GlobalValue<'ctx>)> = self
            .vtable_globals
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (class_name, _) in &vtable_globals_snapshot {
            let vtable_global_name = format!("__vtable.{}", class_name);
            if self.module.get_global(&vtable_global_name).is_none() {
                if let Some(t) = self.vtable_types.borrow().get(class_name).copied() {
                    let gv =
                        self.module
                            .add_global(t.as_basic_type_enum(), None, &vtable_global_name);
                    gv.set_linkage(inkwell::module::Linkage::External);
                    self.vtable_globals
                        .borrow_mut()
                        .insert(class_name.clone(), gv);
                }
            }
        }

        // Forward-declare protocol witness tables from stdlib in the main module
        let wt_snapshot: Vec<((String, String), inkwell::values::GlobalValue<'ctx>)> = self
            .protocol_witness_tables
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for ((protocol_name, type_suffix), old_gv) in &wt_snapshot {
            let wt_global_name = format!("__protocol_wt.{}.{}", protocol_name, type_suffix);
            if self.module.get_global(&wt_global_name).is_none() {
                let wt_type = old_gv.get_value_type().into_struct_type();
                let gv = self.module.add_global(wt_type, None, &wt_global_name);
                gv.set_linkage(inkwell::module::Linkage::External);
                self.protocol_witness_tables
                    .borrow_mut()
                    .insert((protocol_name.clone(), type_suffix.clone()), gv);
            }
        }

        *self.program_scope.borrow_mut() = Some(scope);
        let all_main: Vec<Rc<RefCell<Statement>>> = program.statements.iter().cloned().collect();
        self.run_all_passes(&all_main);

        self.generate_main_wrapper(program);

        IRModules {
            main: Rc::new(std::mem::replace(
                &mut self.module,
                self.context.create_module("done"),
            )),
            stdlib: Some(Rc::new(compiled_stdlib)),
        }
    }

    fn run_all_passes(&self, stmts: &[Rc<RefCell<Statement>>]) {
        for stmt in stmts {
            self.declare_struct_types(stmt.clone());
        }
        for stmt in stmts {
            self.declare_class_types(stmt.clone());
        }
        for stmt in stmts {
            self.declare_enum_types(stmt.clone());
        }
        for stmt in stmts {
            self.create_vtable_types(stmt.clone());
        }
        for stmt in stmts {
            self.create_protocol_witness_table_types(stmt.clone());
        }
        for stmt in stmts {
            self.create_struct_type_bodies(stmt.clone());
        }
        for stmt in stmts {
            self.create_class_type_bodies(stmt.clone());
        }
        for stmt in stmts {
            self.create_existential_container_types(stmt.clone());
        }
        for stmt in stmts {
            self.create_enum_type_bodies(stmt.clone());
        }

        {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for stmt in stmts {
                self.count_fn_name_frequencies(stmt, &mut counts);
            }
            *self.overloaded_fn_names.borrow_mut() = counts
                .into_iter()
                .filter(|(_, c)| *c > 1)
                .map(|(n, _)| n)
                .collect();
        }

        for stmt in stmts {
            self.create_function_declarations(stmt.clone());
        }
        for stmt in stmts {
            self.create_vtable_instances(stmt.clone());
        }
        for stmt in stmts {
            self.create_protocol_witness_tables(stmt.clone());
        }
        for stmt in stmts {
            let _ = self.resolve_statement(stmt.clone());
        }
    }

    fn declare_struct_types(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::StructDecl {
            name, attributes, ..
        } = &*statement.borrow()
        {
            if attributes.iter().any(|a| a.name == "builtintype") {
                return;
            }
            let struct_name = &name.value;
            if !self.struct_types.borrow().contains_key(struct_name) {
                let mangled =
                    Self::mangle_type_name("S", &self.package_name, &self.module_name, struct_name);
                let struct_type = self.context.opaque_struct_type(&mangled);
                self.struct_types
                    .borrow_mut()
                    .insert(struct_name.clone(), struct_type);
            }
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.declare_struct_types(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.declare_struct_types(stmt.clone());
                }
            }
        }
    }

    fn is_stored_field(&self, stmt: &Rc<RefCell<Statement>>) -> bool {
        if let Statement::VariableDecl { accessors, .. } = &*stmt.borrow() {
            let has_get_set = accessors
                .iter()
                .any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
            !has_get_set
        } else {
            false
        }
    }

    fn create_struct_type_bodies(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::StructDecl {
            name,
            body,
            attributes,
            ..
        } = &*statement.borrow()
        {
            if attributes.iter().any(|a| a.name == "builtintype") {
                return;
            }
            let struct_name = &name.value;
            if let Some(struct_type) = self.struct_types.borrow().get(struct_name).cloned() {
                let field_types: Vec<inkwell::types::BasicTypeEnum<'ctx>> = body
                    .iter()
                    .filter(|stmt| self.is_stored_field(stmt))
                    .filter_map(|stmt| {
                        if let Statement::VariableDecl { ty, .. } = &*stmt.borrow()
                            && let Some(ty) = ty
                        {
                            match self.resolve_type(ty.clone()) {
                                Ok(llvm_ty) => return Some(llvm_ty),
                                Err(_) => return None,
                            }
                        }
                        None
                    })
                    .collect();

                struct_type.set_body(&field_types, false);
            }
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_struct_type_bodies(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_struct_type_bodies(stmt.clone());
                }
            }
        }
    }

    fn declare_class_types(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::ClassDecl { name, .. } = &*statement.borrow() {
            let class_name = &name.value;
            if !self.class_types.borrow().contains_key(class_name) {
                let mangled =
                    Self::mangle_type_name("C", &self.package_name, &self.module_name, class_name);
                let class_type = self.context.opaque_struct_type(&mangled);
                self.class_types
                    .borrow_mut()
                    .insert(class_name.clone(), class_type);
            }
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.declare_class_types(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.declare_class_types(stmt.clone());
                }
            }
        }
    }

    fn create_class_type_bodies(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::ClassDecl { name, .. } = &*statement.borrow() {
            let class_name = &name.value;
            if let Some(class_type) = self.class_types.borrow().get(class_name).cloned() {
                let ref_count_ty = self.context.i64_type().into();
                let vtable_ptr_ty: BasicTypeEnum<'ctx> =
                    self.context.ptr_type(inkwell::AddressSpace::from(0)).into();
                let mut field_types: Vec<BasicTypeEnum<'ctx>> = vec![vtable_ptr_ty, ref_count_ty];

                field_types.extend(self.collect_class_stored_field_types(class_name));

                class_type.set_body(&field_types, false);
            }
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_class_type_bodies(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_class_type_bodies(stmt.clone());
                }
            }
        }
    }

    fn collect_class_stored_field_types(&self, class_name: &str) -> Vec<BasicTypeEnum<'ctx>> {
        let binding = self.program_scope.borrow();
        let Some(scope) = binding.as_ref() else {
            return vec![];
        };
        let Some(symbol) = scope.borrow().get_symbol(class_name) else {
            return vec![];
        };

        let sym_borrow = symbol.borrow();
        let (decl, properties) = match &*sym_borrow {
            Symbol::Class {
                decl, properties, ..
            } => (decl.clone(), properties.clone()),
            _ => return vec![],
        };
        drop(sym_borrow);

        let mut field_types = Vec::new();

        if let Statement::ClassDecl {
            superclass: Some(super_expr),
            ..
        } = &*decl.borrow()
        {
            if let Expression::Type {
                name: super_name, ..
            } = &*super_expr.borrow()
            {
                field_types.extend(self.collect_class_stored_field_types(&super_name.value));
            }
        }

        for field in properties.iter() {
            if let Ok(Some(field_decl)) = field.borrow().get_decl() {
                if let Statement::VariableDecl {
                    accessors,
                    ty: Some(ty),
                    ..
                } = &*field_decl.borrow()
                {
                    let has_get_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
                    if !has_get_set {
                        if let Ok(llvm_ty) = self.resolve_type(ty.clone()) {
                            field_types.push(llvm_ty);
                        }
                    }
                }
            }
        }

        field_types
    }

    fn auto_assign_init_fields(
        &self,
        struct_name: &str,
        self_ptr: PointerValue<'ctx>,
        parameters: &[Rc<RefCell<Parameter>>],
    ) {
        let is_class = self.class_types.borrow().contains_key(struct_name);
        let binding = self.program_scope.borrow();
        let Some(scope) = binding.as_ref() else {
            return;
        };
        let Some(symbol) = scope.borrow().get_symbol(struct_name) else {
            return;
        };
        let sym_borrow = symbol.borrow();
        let (decl, properties) = match &*sym_borrow {
            Symbol::Struct {
                decl, properties, ..
            } => (decl.clone(), properties.clone()),
            Symbol::Class {
                decl, properties, ..
            } => (decl.clone(), properties.clone()),
            _ => return,
        };
        drop(sym_borrow);

        let superclass_name = if is_class {
            if let Statement::ClassDecl {
                superclass: Some(super_expr),
                ..
            } = &*decl.borrow()
            {
                if let Expression::Type {
                    name: super_name, ..
                } = &*super_expr.borrow()
                {
                    Some(super_name.value.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let superclass_field_count = if let Some(ref super_name) = superclass_name {
            self.get_class_stored_field_count(super_name)
        } else {
            0
        };

        for param in parameters {
            let param_name = param.borrow().name.value.clone();
            let mut stored_idx = 0usize;
            for field in properties.iter() {
                let Ok(field_name) = field.borrow().name() else {
                    continue;
                };
                let Ok(Some(field_decl)) = field.borrow().get_decl() else {
                    continue;
                };
                let decl_ref = field_decl.borrow();
                let Statement::VariableDecl {
                    accessors,
                    ty: field_ty,
                    ..
                } = &*decl_ref
                else {
                    continue;
                };
                let has_get_set = accessors
                    .iter()
                    .any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
                if has_get_set {
                    stored_idx += 1;
                    continue;
                }
                let Some(field_ty) = field_ty else {
                    stored_idx += 1;
                    continue;
                };
                if field_name == param_name {
                    let gep_idx = if is_class {
                        stored_idx + 2 + superclass_field_count
                    } else {
                        stored_idx
                    };
                    if is_class {
                        if let Some(class_type) =
                            self.class_types.borrow().get(struct_name).copied()
                        {
                            if let Ok(field_ptr) = self.builder.build_struct_gep(
                                class_type,
                                self_ptr,
                                gep_idx as u32,
                                "",
                            ) {
                                if let Some(param_ptr) = self.lookup_variable(&param_name) {
                                    if let Ok(llvm_ty) = self.resolve_type(field_ty.clone()) {
                                        if let Ok(param_val) =
                                            self.builder.build_load(llvm_ty, param_ptr, "")
                                        {
                                            let _ = self.builder.build_store(field_ptr, param_val);
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(struct_type) =
                        self.struct_types.borrow().get(struct_name).copied()
                    {
                        if let Ok(field_ptr) =
                            self.builder
                                .build_struct_gep(struct_type, self_ptr, gep_idx as u32, "")
                        {
                            if let Some(param_ptr) = self.lookup_variable(&param_name) {
                                if let Ok(llvm_ty) = self.resolve_type(field_ty.clone()) {
                                    if let Ok(param_val) =
                                        self.builder.build_load(llvm_ty, param_ptr, "")
                                    {
                                        let _ = self.builder.build_store(field_ptr, param_val);
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
                stored_idx += 1;
            }
        }
    }

    fn build_optional_some_return(
        &self,
        self_ptr: PointerValue<'ctx>,
        ret_ty: Rc<RefCell<Type>>,
    ) -> Result<()> {
        let enum_name = match &*ret_ty.borrow() {
            Type::Enum(name, ..) => name.clone(),
            _ => anyhow::bail!("Expected Enum type for Optional return"),
        };
        let enum_type = self
            .enum_types
            .borrow()
            .get(&enum_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
        let payloads_type = self
            .enum_payload_types
            .borrow()
            .get(&enum_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Enum payload type '{}' not found", enum_name))?;
        let struct_ty = self_ptr.get_type();
        let struct_val = self.builder.build_load(struct_ty, self_ptr, "")?;
        let some_payload_type = payloads_type
            .get_field_type_at_index(1)
            .ok_or_else(|| anyhow::anyhow!("Some payload slot not found"))?;
        let some_payload_struct = some_payload_type.into_struct_type();
        let some_payload = some_payload_struct.const_named_struct(&[struct_val.into()]);
        let tag = self.context.i8_type().const_int(1, false);
        let optional_val = enum_type.const_named_struct(&[tag.into(), some_payload.into()]);
        self.emit_all_deinit_calls();
        self.emit_class_releases();
        self.builder.build_return(Some(&optional_val))?;
        Ok(())
    }

    fn declare_enum_types(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::EnumDecl { name, .. } = &*statement.borrow() {
            let enum_name = &name.value;
            if !self.enum_types.borrow().contains_key(enum_name) {
                let mangled =
                    Self::mangle_type_name("E", &self.package_name, &self.module_name, enum_name);
                let enum_type = self.context.opaque_struct_type(&mangled);
                self.enum_types
                    .borrow_mut()
                    .insert(enum_name.clone(), enum_type);

                let mangled_payloads = Self::mangle_type_name(
                    "E",
                    &self.package_name,
                    &self.module_name,
                    &format!("{}.payloads", enum_name),
                );
                let payload_type = self.context.opaque_struct_type(&mangled_payloads);
                self.enum_payload_types
                    .borrow_mut()
                    .insert(enum_name.clone(), payload_type);
            }
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.declare_enum_types(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.declare_enum_types(stmt.clone());
                }
            }
        }
    }

    fn create_enum_type_bodies(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::EnumDecl {
            name,
            cases,
            raw_value_type,
            ..
        } = &*statement.borrow()
        {
            let enum_name = &name.value;
            if let Some(enum_type) = self.enum_types.borrow().get(enum_name).cloned() {
                if let Some(raw_type) = raw_value_type {
                    if let Ok(raw_llvm_type) = self.resolve_type(raw_type.clone()) {
                        enum_type.set_body(&[raw_llvm_type], false);
                    }
                    return;
                }

                let mut case_types: Vec<inkwell::types::BasicTypeEnum<'ctx>> = Vec::new();
                for case in cases {
                    let param_types: Vec<inkwell::types::BasicTypeEnum<'ctx>> = case
                        .parameters
                        .iter()
                        .filter_map(|param| {
                            let expr = param.type_expression.borrow();
                            let ty = expr.get_ty().ok().flatten();
                            ty.and_then(|ty| self.resolve_type(ty).ok())
                        })
                        .collect();
                    if param_types.is_empty() {
                        case_types.push(self.context.struct_type(&[], false).as_basic_type_enum());
                    } else {
                        case_types.push(
                            self.context
                                .struct_type(&param_types, false)
                                .as_basic_type_enum(),
                        );
                    }
                }

                if let Some(payload_type) = self.enum_payload_types.borrow().get(enum_name).cloned()
                {
                    payload_type.set_body(&case_types, false);
                }

                let tag_type = self.context.i8_type();
                if case_types.is_empty() {
                    enum_type.set_body(&[], false);
                } else if let Some(payload_type) =
                    self.enum_payload_types.borrow().get(enum_name).cloned()
                {
                    let field_types = vec![
                        tag_type.as_basic_type_enum(),
                        payload_type.as_basic_type_enum(),
                    ];
                    enum_type.set_body(&field_types, false);
                }
            }
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_enum_type_bodies(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_enum_type_bodies(stmt.clone());
                }
            }
        }
    }

    fn get_stored_struct_field_index(&self, struct_name: &str, field_name: &str) -> Result<usize> {
        if let Some(scope) = self.program_scope.borrow().as_ref()
            && let Some(symbol) = scope.borrow().get_symbol(struct_name)
            && let Symbol::Struct { properties, .. } = &*symbol.borrow()
        {
            let mut stored_idx = 0;
            for field in properties.iter() {
                if let Some(decl) = field.borrow().get_decl().ok().flatten()
                    && let Statement::VariableDecl { accessors, .. } = &*decl.borrow()
                {
                    let has_get_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
                    if has_get_set {
                        continue;
                    }
                    if field.borrow().name().as_ref().ok() == Some(&field_name.to_string()) {
                        return Ok(stored_idx);
                    }
                    stored_idx += 1;
                }
            }
        }
        anyhow::bail!(
            "Stored field '{}' not found in struct '{}'",
            field_name,
            struct_name
        )
    }

    fn get_stored_class_field_index(&self, class_name: &str, field_name: &str) -> Result<usize> {
        if let Some(scope) = self.program_scope.borrow().as_ref()
            && let Some(symbol) = scope.borrow().get_symbol(class_name)
        {
            let binding = symbol.borrow();
            let (decl, properties) = match &*binding {
                Symbol::Struct {
                    decl, properties, ..
                } => (decl.clone(), properties.clone()),
                Symbol::Class {
                    decl, properties, ..
                } => (decl.clone(), properties.clone()),
                _ => {
                    return Err(anyhow::anyhow!(
                        "Symbol '{}' is not a struct or class",
                        class_name
                    ));
                }
            };
            drop(binding);

            let superclass_name = if let Statement::ClassDecl {
                superclass: Some(super_expr),
                ..
            } = &*decl.borrow()
            {
                if let Expression::Type {
                    name: super_name, ..
                } = &*super_expr.borrow()
                {
                    Some(super_name.value.clone())
                } else {
                    None
                }
            } else {
                None
            };

            let superclass_field_count = if let Some(ref super_name) = superclass_name {
                self.get_class_stored_field_count(super_name)
            } else {
                0
            };

            let mut stored_idx = 0;
            for field in properties.iter() {
                if let Some(field_decl) = field.borrow().get_decl().ok().flatten()
                    && let Statement::VariableDecl { accessors, .. } = &*field_decl.borrow()
                {
                    let has_get_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
                    if has_get_set {
                        continue;
                    }
                    if field.borrow().name().as_ref().ok() == Some(&field_name.to_string()) {
                        return Ok(stored_idx + 2 + superclass_field_count);
                    }
                    stored_idx += 1;
                }
            }

            if let Some(super_name) = superclass_name {
                return self.get_stored_class_field_index(&super_name, field_name);
            }
        }
        anyhow::bail!(
            "Stored field '{}' not found in class '{}'",
            field_name,
            class_name
        )
    }

    fn get_class_stored_field_count(&self, class_name: &str) -> usize {
        let binding = self.program_scope.borrow();
        let Some(scope) = binding.as_ref() else {
            return 0;
        };
        let Some(symbol) = scope.borrow().get_symbol(class_name) else {
            return 0;
        };

        let sym_borrow = symbol.borrow();
        let (decl, properties) = match &*sym_borrow {
            Symbol::Class {
                decl, properties, ..
            } => (decl.clone(), properties.clone()),
            _ => return 0,
        };
        drop(sym_borrow);

        let mut count = 0;

        if let Statement::ClassDecl {
            superclass: Some(super_expr),
            ..
        } = &*decl.borrow()
        {
            if let Expression::Type {
                name: super_name, ..
            } = &*super_expr.borrow()
            {
                count += self.get_class_stored_field_count(&super_name.value);
            }
        }

        for field in properties.iter() {
            if let Ok(Some(field_decl)) = field.borrow().get_decl() {
                if let Statement::VariableDecl { accessors, .. } = &*field_decl.borrow() {
                    let has_get_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
                    if !has_get_set {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    fn get_enum_case_index(&self, enum_name: &str, case_name: &str) -> Result<usize> {
        if let Some(scope) = self.program_scope.borrow().as_ref()
            && let Some(symbol) = scope.borrow().get_symbol(enum_name)
            && let Symbol::Enum { cases, .. } = &*symbol.borrow()
        {
            for (i, case) in cases.iter().enumerate() {
                if case.borrow().name().as_ref().ok() == Some(&case_name.to_string()) {
                    return Ok(i);
                }
            }
        }
        anyhow::bail!("Case '{}' not found in enum '{}'", case_name, enum_name)
    }

    fn get_enum_raw_llvm_type(&self, enum_name: &str) -> Option<BasicTypeEnum<'ctx>> {
        if let Some(scope) = self.program_scope.borrow().as_ref() {
            if let Some(symbol) = scope.borrow().get_symbol(enum_name) {
                if let Symbol::Enum { decl, .. } = &*symbol.borrow() {
                    if let Statement::EnumDecl { raw_value_type, .. } = &*decl.borrow() {
                        if let Some(raw_type) = raw_value_type {
                            return self.resolve_type(raw_type.clone()).ok();
                        }
                    }
                }
            }
        }
        None
    }

    fn count_fn_name_frequencies(
        &self,
        stmt: &Rc<RefCell<Statement>>,
        counts: &mut HashMap<String, usize>,
    ) {
        let s = stmt.borrow();
        match &*s {
            Statement::FunctionDecl { name, body, .. } => {
                *counts.entry(name.value.clone()).or_insert(0) += 1;
                if let FunctionBody::Statements(stmts) = &*body.borrow() {
                    for s in stmts {
                        self.count_fn_name_frequencies(s, counts);
                    }
                }
            }
            Statement::StructDecl { name, body, .. } => {
                for s in body {
                    if let Statement::FunctionDecl { name: mname, .. } = &*s.borrow() {
                        let key = format!("{}.{}", name.value, mname.value);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                    self.count_fn_name_frequencies(s, counts);
                }
            }
            Statement::ClassDecl { name, body, .. } => {
                for s in body {
                    if let Statement::FunctionDecl { name: mname, .. } = &*s.borrow() {
                        let key = format!("{}.{}", name.value, mname.value);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                    self.count_fn_name_frequencies(s, counts);
                }
            }
            Statement::EnumDecl { name, body, .. } => {
                for s in body {
                    if let Statement::FunctionDecl { name: mname, .. } = &*s.borrow() {
                        let key = format!("{}.{}", name.value, mname.value);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                    self.count_fn_name_frequencies(s, counts);
                }
            }
            Statement::ExtensionDecl {
                type_name,
                body,
                type_arguments,
                ..
            } => {
                let prefix = if !self
                    .type_args_to_abbreviation(type_arguments.as_ref())
                    .is_empty()
                {
                    format!(
                        "{}.{}",
                        type_name.value,
                        self.type_args_to_abbreviation(type_arguments.as_ref())
                    )
                } else {
                    type_name.value.clone()
                };
                for s in body {
                    if let Statement::FunctionDecl { name: mname, .. } = &*s.borrow() {
                        let key = format!("{}.{}", prefix, mname.value);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                    self.count_fn_name_frequencies(s, counts);
                }
            }
            Statement::ConditionalBlock { clauses } => {
                for clause in clauses {
                    for stmt in &clause.body {
                        self.count_fn_name_frequencies(stmt, counts);
                    }
                }
            }
            _ => {}
        }
    }

    fn type_to_abbreviation(&self, ty: &Type) -> String {
        match ty {
            Type::Void => "V".into(),
            Type::Never => "N".into(),
            Type::Struct(name, ..)
            | Type::Class(name, ..)
            | Type::Enum(name, ..)
            | Type::Protocol(name, ..) => name.clone(),
            Type::Pointer(_) => "P".into(),
            Type::NonNullPointer(_) => "NP".into(),
            Type::Tuple(_) => "T".into(),
            Type::GenericParam(name) => name.clone(),
            Type::ConstGeneric(name, _) => format!("cg{}", name),
            Type::AssociatedType(_, name) => name.clone(),
            Type::Compound(_) => "C".into(),
            Type::Function(_, _, _, _) => "F".into(),
            Type::Closure(_, _) => "CC".into(),
            Type::Inline(inner, _) => {
                format!("inline{}", self.type_to_abbreviation(&inner.borrow()))
            }
        }
    }

    fn type_args_to_abbreviation(
        &self,
        type_arguments: Option<&Vec<Rc<RefCell<Expression>>>>,
    ) -> String {
        match type_arguments {
            Some(args) => {
                let parts: Vec<String> = args
                    .iter()
                    .filter_map(|ta| {
                        let expr = ta.borrow();
                        let ty = match &*expr {
                            Expression::Type { ty, .. } => ty.clone(),
                            Expression::Variable { ty, .. } => ty.clone(),
                            _ => None,
                        };
                        drop(expr);
                        ty.and_then(|t| {
                            let ty = t.borrow();
                            let s = self.type_to_abbreviation(&ty);
                            if s.is_empty() { None } else { Some(s) }
                        })
                    })
                    .collect();
                if parts.is_empty() {
                    String::new()
                } else {
                    parts.join(".")
                }
            }
            None => String::new(),
        }
    }

    fn mangle_type_name(kind: &str, package: &str, module: &str, name: &str) -> String {
        format!("_T${}${}${}${}", kind, package, module, name)
    }

    fn mangle_fn_name(&self, base_name: &str, params: &[Rc<RefCell<Parameter>>]) -> String {
        let labels: Vec<String> = params
            .iter()
            .map(|p| {
                let pb = p.borrow();
                if let Some(label) = &pb.label {
                    label.value.clone()
                } else {
                    pb.name.value.clone()
                }
            })
            .collect();
        let types: Vec<String> = params
            .iter()
            .map(|p| {
                let pb = p.borrow();
                pb.ty
                    .as_ref()
                    .map(|t| self.type_to_abbreviation(&t.borrow()))
                    .unwrap_or_else(|| "?".into())
            })
            .collect();
        let base = base_name.replace('.', "$");
        format!("_T${}${}${}", base, labels.join("_"), types.join("_"))
    }

    fn types_compatible(a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Void, Type::Void) | (Type::Never, Type::Never) => true,
            (Type::Struct(n1, _, _), Type::Struct(n2, _, _)) if n1 == n2 => true,
            (Type::Class(n1, ..), Type::Class(n2, ..))
            | (Type::Enum(n1, ..), Type::Enum(n2, ..))
            | (Type::Protocol(n1, ..), Type::Protocol(n2, ..)) => n1 == n2,
            (Type::Pointer(t1), Type::Pointer(t2)) => {
                Self::types_compatible(&t1.borrow(), &t2.borrow())
            }
            (Type::NonNullPointer(t1), Type::NonNullPointer(t2)) => {
                Self::types_compatible(&t1.borrow(), &t2.borrow())
            }
            (Type::NonNullPointer(t1), Type::Pointer(t2))
            | (Type::Pointer(t1), Type::NonNullPointer(t2)) => {
                Self::types_compatible(&t1.borrow(), &t2.borrow())
            }
            (Type::Tuple(e1), Type::Tuple(e2)) => {
                e1.len() == e2.len()
                    && e1.iter().zip(e2.iter()).all(|((_, t1), (_, t2))| {
                        Self::types_compatible(&t1.borrow(), &t2.borrow())
                    })
            }
            (Type::GenericParam(n1), Type::GenericParam(n2)) => n1 == n2,
            (Type::AssociatedType(t1, n1), Type::AssociatedType(t2, n2)) => {
                n1 == n2 && Self::types_compatible(&t1.borrow(), &t2.borrow())
            }
            (Type::Inline(a_inner, _), Type::Inline(b_inner, _)) => {
                Self::types_compatible(&a_inner.borrow(), &b_inner.borrow())
            }
            (Type::Inline(a_inner, _), b) => Self::types_compatible(&a_inner.borrow(), b),
            (a, Type::Inline(b_inner, _)) => Self::types_compatible(a, &b_inner.borrow()),
            _ => false,
        }
    }

    fn mangle_from_overload(
        &self,
        base_name: &str,
        sym: &Rc<RefCell<Symbol>>,
        call_params: &[CallParameter],
    ) -> Option<String> {
        if let Ok(Some(decl)) = sym.borrow().get_decl()
            && let Statement::FunctionDecl {
                parameters,
                attributes,
                ..
            } = &*decl.borrow()
        {
            if let Some(cname) = attributes
                .iter()
                .find(|a| a.name == "cname")
                .and_then(|a| a.value.as_deref())
            {
                return Some(cname.to_string());
            }
            if self.extern_fn_c_names.borrow().contains_key(base_name) {
                if let Some(c_name) = self.extern_fn_c_names.borrow().get(base_name) {
                    return Some(c_name.clone());
                }
            }
            Some(self.mangle_fn_name(base_name, parameters))
        } else {
            let labels: Vec<String> = call_params
                .iter()
                .map(|p| {
                    p.label
                        .as_ref()
                        .map(|t| t.value.clone())
                        .unwrap_or_else(|| "_".into())
                })
                .collect();
            let types: Vec<String> = (0..call_params.len()).map(|_| "?".into()).collect();
            Some(format!(
                "{}${}${}",
                base_name,
                labels.join("_"),
                types.join("_")
            ))
        }
    }

    fn create_function_declarations(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::FunctionDecl {
            name,
            ty,
            parameters,
            body,
            attributes,
            ..
        } = &*statement.borrow()
        {
            if let Some(ty) = ty {
                if let Some(cname) = attributes
                    .iter()
                    .find(|a| a.name == "cname")
                    .and_then(|a| a.value.as_deref())
                {
                    if let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                    {
                        let mut all_params = param_types.clone();
                        if throws_types.is_some() {
                            let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(
                                RefCell::new(Type::Struct(
                                    "Int8".to_string(),
                                    WeakSymbol(std::rc::Weak::new()),
                                    vec![],
                                )),
                            ))));
                            all_params.insert(0, err_ty);
                        }
                        if let Ok(function_type) =
                            self.get_function_type(return_type.clone(), all_params, *is_vararg)
                        {
                            self.module.add_function(cname, function_type, None);
                        }
                    }
                } else {
                    self.create_mangled_function_declaration(name, parameters, ty);
                }
            }
            match &*body.borrow() {
                FunctionBody::Statements(stmts) => {
                    for s in stmts {
                        self.create_function_declarations(s.clone());
                    }
                }
                FunctionBody::Expression(expr) => {
                    self.create_function_declarations_in_expr(expr.clone());
                }
                FunctionBody::None => {}
            }
        }
        if let Statement::VariableDecl { accessors, .. } = &*statement.borrow() {
            for accessor in accessors {
                for stmt in &accessor.body {
                    self.create_function_declarations(stmt.clone());
                }
            }
        }
        if let Statement::ExternBlock { items, .. } = &*statement.borrow() {
            for item in items {
                let _ = self.create_extern_declaration(item.clone());
            }
        }
        if let Statement::ExternDecl { statement, .. } = &*statement.borrow() {
            let _ = self.create_extern_declaration(statement.clone());
        }
        if let Statement::StructDecl { name, body, .. } = &*statement.borrow() {
            for stmt in body {
                if let Statement::FunctionDecl {
                    name: method_name,
                    ty,
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Some(ty) = ty
                    && let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    if throws_types.is_some() {
                        let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                            Type::Struct(
                                "Int8".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            ),
                        )))));
                        all_param_types.push(err_ty);
                    }
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let base_name = format!("{}.{}", name.value, method_name.value);
                        let mangled = self.mangle_fn_name(&base_name, parameters);
                        self.module.add_function(&mangled, function_type, None);
                        self.register_mangled_name(&base_name, &mangled);
                    }
                }
                if let Statement::InitDecl {
                    ty: Some(ty),
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    if throws_types.is_some() {
                        let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                            Type::Struct(
                                "Int8".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            ),
                        )))));
                        all_param_types.push(err_ty);
                    }
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let base = format!("{}.init", name.value);
                        let mangled = self.mangle_fn_name(&base, parameters);
                        self.module.add_function(&mangled, function_type, None);
                        self.register_mangled_name(&base, &mangled);
                    }
                }
                if let Statement::DeinitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(_, return_type, _, None) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), vec![self_param], false)
                    {
                        let llvm_name = self.mangle_fn_name(&format!("{}.deinit", name.value), &[]);
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::SubscriptDecl {
                    accessors,
                    ty: Some(ty),
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, _, None) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    for pt in param_types {
                        all_param_types.push(pt.clone());
                    }
                    let has_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Set));
                    if let Ok(getter_type) =
                        self.get_function_type(return_type.clone(), all_param_types.clone(), false)
                    {
                        let sub_base = format!("{}.subscript", name.value);
                        let getter_mangled = self.mangle_fn_name(&format!("{}.getter", sub_base), parameters);
                        self.module.add_function(
                            &getter_mangled,
                            getter_type,
                            None,
                        );
                        self.register_mangled_name(&format!("{}.subscript.getter", name.value), &getter_mangled);
                    }
                    if has_set {
                        let void_ty = Rc::new(RefCell::new(Type::Void));
                        let mut setter_param_types = all_param_types.clone();
                        setter_param_types.push(return_type.clone());
                        if let Ok(setter_type) =
                            self.get_function_type(void_ty, setter_param_types, false)
                        {
                            let sub_base = format!("{}.subscript", name.value);
                            let setter_mangled = self.mangle_fn_name(&format!("{}.setter", sub_base), parameters);
                            self.module.add_function(
                                &setter_mangled,
                                setter_type,
                                None,
                            );
                            self.register_mangled_name(&format!("{}.subscript.setter", name.value), &setter_mangled);
                        }
                    }
                }
            }
            let deinit_name = self.mangle_fn_name(&format!("{}.deinit", name.value), &[]);
            self.mangled_fn_names
                .borrow_mut()
                .insert(format!("{}.deinit", name.value), deinit_name.clone());
            if self.module.get_function(&deinit_name).is_none() {
                let void_ty = Rc::new(RefCell::new(Type::Void));
                let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                    Type::Void,
                )))));
                if let Ok(fn_type) = self.get_function_type(void_ty, vec![self_param], false) {
                    let f = self.module.add_function(&deinit_name, fn_type, None);
                    let entry = self.context.append_basic_block(f, "entry");
                    let current_block = self.builder.get_insert_block();
                    self.builder.position_at_end(entry);
                    let _ = self.builder.build_return(None);
                    if let Some(block) = current_block {
                        self.builder.position_at_end(block);
                    }
                }
            }
        }
        if let Statement::ClassDecl { name, body, .. } = &*statement.borrow() {
            for stmt in body {
                if let Statement::FunctionDecl {
                    name: method_name,
                    ty,
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Some(ty) = ty
                    && let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    if throws_types.is_some() {
                        let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                            Type::Struct(
                                "Int8".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            ),
                        )))));
                        all_param_types.push(err_ty);
                    }
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let base_name = format!("{}.{}", name.value, method_name.value);
                        let mangled = self.mangle_fn_name(&base_name, parameters);
                        self.module.add_function(&mangled, function_type, None);
                        self.register_mangled_name(&base_name, &mangled);
                    }
                }
                if let Statement::InitDecl {
                    ty: Some(ty),
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    if throws_types.is_some() {
                        let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                            Type::Struct(
                                "Int8".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            ),
                        )))));
                        all_param_types.push(err_ty);
                    }
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let base = format!("{}.init", name.value);
                        let mangled = self.mangle_fn_name(&base, parameters);
                        self.module.add_function(&mangled, function_type, None);
                        self.register_mangled_name(&base, &mangled);
                    }
                }
                if let Statement::VariableDecl {
                    name: field_name,
                    accessors,
                    ty: Some(ty),
                    token,
                    ..
                } = &*stmt.borrow()
                {
                    let has_explicit_get = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Get));
                    let has_explicit_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Set));
                    if let Ok(llvm_ty) = self.resolve_type(ty.clone()) {
                        if has_explicit_get || has_explicit_set {
                            self.declare_accessor_functions(
                                &name.value,
                                &field_name.value,
                                accessors,
                                llvm_ty,
                            );
                        } else {
                            let is_var = matches!(
                                &token.ty,
                                TokenType::Keyword {
                                    keyword: KeywordType::Var
                                }
                            );
                            self.declare_accessor_fn(&name.value, &field_name.value, true, llvm_ty);
                            if is_var {
                                self.declare_accessor_fn(
                                    &name.value,
                                    &field_name.value,
                                    false,
                                    llvm_ty,
                                );
                            }
                        }
                    }
                }
                if let Statement::DeinitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(_, return_type, _, None) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), vec![self_param], false)
                    {
                        let llvm_name = self.mangle_fn_name(&format!("{}.deinit", name.value), &[]);
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::SubscriptDecl {
                    accessors,
                    ty: Some(ty),
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, _, None) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    for pt in param_types {
                        all_param_types.push(pt.clone());
                    }
                    let has_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Set));
                    if let Ok(getter_type) =
                        self.get_function_type(return_type.clone(), all_param_types.clone(), false)
                    {
                        let sub_base = format!("{}.subscript", name.value);
                        let getter_mangled = self.mangle_fn_name(&format!("{}.getter", sub_base), parameters);
                        self.module.add_function(
                            &getter_mangled,
                            getter_type,
                            None,
                        );
                        self.register_mangled_name(&format!("{}.subscript.getter", name.value), &getter_mangled);
                    }
                    if has_set {
                        let void_ty = Rc::new(RefCell::new(Type::Void));
                        let mut setter_param_types = all_param_types.clone();
                        setter_param_types.push(return_type.clone());
                        if let Ok(setter_type) =
                            self.get_function_type(void_ty, setter_param_types, false)
                        {
                            let sub_base = format!("{}.subscript", name.value);
                            let setter_mangled = self.mangle_fn_name(&format!("{}.setter", sub_base), parameters);
                            self.module.add_function(
                                &setter_mangled,
                                setter_type,
                                None,
                            );
                            self.register_mangled_name(&format!("{}.subscript.setter", name.value), &setter_mangled);
                        }
                    }
                }
            }
        }
        if let Statement::EnumDecl { name, body, .. } = &*statement.borrow() {
            for stmt in body {
                if let Statement::FunctionDecl {
                    name: method_name,
                    ty,
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Some(ty) = ty
                    && let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                {
                    let mut all_params = param_types.clone();
                    if throws_types.is_some() {
                        let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                            Type::Struct(
                                "Int8".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            ),
                        )))));
                        all_params.insert(0, err_ty);
                    }
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_params, *is_vararg)
                    {
                        let base_name = format!("{}.{}", name.value, method_name.value);
                        let mangled = self.mangle_fn_name(&base_name, parameters);
                        self.module.add_function(&mangled, function_type, None);
                        self.register_mangled_name(&base_name, &mangled);
                    }
                }
                if let Statement::DeinitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(_, return_type, _, None) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), vec![self_param], false)
                    {
                        let llvm_name = self.mangle_fn_name(&format!("{}.deinit", name.value), &[]);
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::SubscriptDecl {
                    accessors,
                    ty: Some(ty),
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, _, None) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    for pt in param_types {
                        all_param_types.push(pt.clone());
                    }
                    let has_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Set));
                    if let Ok(getter_type) =
                        self.get_function_type(return_type.clone(), all_param_types.clone(), false)
                    {
                        let sub_base = format!("{}.subscript", name.value);
                        let getter_mangled = self.mangle_fn_name(&format!("{}.getter", sub_base), parameters);
                        self.module.add_function(
                            &getter_mangled,
                            getter_type,
                            None,
                        );
                        self.register_mangled_name(&format!("{}.subscript.getter", name.value), &getter_mangled);
                    }
                    if has_set {
                        let void_ty = Rc::new(RefCell::new(Type::Void));
                        let mut setter_param_types = all_param_types.clone();
                        setter_param_types.push(return_type.clone());
                        if let Ok(setter_type) =
                            self.get_function_type(void_ty, setter_param_types, false)
                        {
                            let sub_base = format!("{}.subscript", name.value);
                            let setter_mangled = self.mangle_fn_name(&format!("{}.setter", sub_base), parameters);
                            self.module.add_function(
                                &setter_mangled,
                                setter_type,
                                None,
                            );
                            self.register_mangled_name(&format!("{}.subscript.setter", name.value), &setter_mangled);
                        }
                    }
                }
            }
            let deinit_name = self.mangle_fn_name(&format!("{}.deinit", name.value), &[]);
            if self.module.get_function(&deinit_name).is_none() {
                let void_ty = Rc::new(RefCell::new(Type::Void));
                let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                    Type::Void,
                )))));
                if let Ok(fn_type) = self.get_function_type(void_ty, vec![self_param], false) {
                    let f = self.module.add_function(&deinit_name, fn_type, None);
                    let entry = self.context.append_basic_block(f, "entry");
                    let current_block = self.builder.get_insert_block();
                    self.builder.position_at_end(entry);
                    let _ = self.builder.build_return(None);
                    if let Some(block) = current_block {
                        self.builder.position_at_end(block);
                    }
                }
            }
        }
        if let Statement::ExtensionDecl {
            type_name,
            body,
            type_arguments,
            ..
        } = &*statement.borrow()
        {
            let type_prefix = if !self
                .type_args_to_abbreviation(type_arguments.as_ref())
                .is_empty()
            {
                format!(
                    "{}.{}",
                    type_name.value,
                    self.type_args_to_abbreviation(type_arguments.as_ref())
                )
            } else {
                type_name.value.clone()
            };
            for stmt in body {
                if let Statement::FunctionDecl {
                    name: method_name,
                    ty,
                    static_method,
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Some(ty) = ty
                    && let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                {
                    let all_param_types: Vec<Rc<RefCell<Type>>> = if *static_method {
                        let mut params = param_types.clone();
                        if throws_types.is_some() {
                            let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(
                                RefCell::new(Type::Struct(
                                    "Int8".to_string(),
                                    WeakSymbol(std::rc::Weak::new()),
                                    vec![],
                                )),
                            ))));
                            params.insert(0, err_ty);
                        }
                        params
                    } else {
                        let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(
                            RefCell::new(Type::Void),
                        ))));
                        let mut all_param_types = vec![self_param];
                        if throws_types.is_some() {
                            let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(
                                RefCell::new(Type::Struct(
                                    "Int8".to_string(),
                                    WeakSymbol(std::rc::Weak::new()),
                                    vec![],
                                )),
                            ))));
                            all_param_types.push(err_ty);
                        }
                        all_param_types.extend(param_types.iter().cloned());
                        all_param_types
                    };
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let base = format!("{}.{}", type_prefix, method_name.value);
                        let mangled = self.mangle_fn_name(&base, parameters);
                        self.module.add_function(&mangled, function_type, None);
                        self.register_mangled_name(&base, &mangled);
                    }
                }
                if let Statement::InitDecl {
                    ty: Some(ty),
                    parameters,
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    if throws_types.is_some() {
                        let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                            Type::Struct(
                                "Int8".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            ),
                        )))));
                        all_param_types.push(err_ty);
                    }
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let base = format!("{}.{}", type_prefix, "init");
                        let mangled = self.mangle_fn_name(&base, parameters);
                        self.module.add_function(&mangled, function_type, None);
                        self.register_mangled_name(&base, &mangled);
                    }
                }
                if let Statement::DeinitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(_, return_type, _, None) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), vec![self_param], false)
                    {
                        let mangled = self.mangle_fn_name(&format!("{}.deinit", type_prefix), &[]);
                        self.module.add_function(&mangled, function_type, None);
                    }
                }
            }
        }
        if let Statement::ProtocolDecl { name, members, .. } = &*statement.borrow() {
            for member in members {
                if let ProtocolMember::Method { decl, .. } = member
                    && let Statement::FunctionDecl {
                        name: method_name,
                        ty,
                        parameters,
                        body,
                        ..
                    } = &*decl.borrow()
                    && let Some(ty) = ty
                    && let Type::Function(param_types, return_type, is_vararg, throws_types) =
                        &*ty.borrow()
                    && !matches!(&*body.borrow(), FunctionBody::None)
                {
                    let mut all_params = param_types.clone();
                    if throws_types.is_some() {
                        let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                            Type::Struct(
                                "Int8".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            ),
                        )))));
                        all_params.insert(0, err_ty);
                    }
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_params, *is_vararg)
                    {
                        let base_name = format!("{}.{}", name.value, method_name.value);
                        let mangled = self.mangle_fn_name(&base_name, parameters);
                        self.module.add_function(&mangled, function_type, None);
                        self.register_mangled_name(&base_name, &mangled);
                    }
                }
            }
        }
        if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_function_declarations(stmt.clone());
            }
        }
        if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_function_declarations(stmt.clone());
                }
            }
        }
    }

    fn create_extern_declaration(&self, statement: Rc<RefCell<Statement>>) -> Result<()> {
        if let Statement::FunctionDecl {
            name,
            ty,
            attributes,
            ..
        } = &*statement.borrow()
            && let Some(ty) = ty
            && let Type::Function(param_types, return_type, is_vararg, throws_types) = &*ty.borrow()
        {
            let c_name = attributes
                .iter()
                .find(|a| a.name == "cname")
                .and_then(|a| a.value.as_deref())
                .unwrap_or(&name.value);
            let mut all_params = param_types.clone();
            if throws_types.is_some() {
                let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                    Type::Struct("Int8".to_string(), WeakSymbol(std::rc::Weak::new()), vec![]),
                )))));
                all_params.insert(0, err_ty);
            }
            let function_type =
                self.get_function_type(return_type.clone(), all_params, *is_vararg)?;
            self.module.add_function(c_name, function_type, None);
            self.extern_fn_c_names
                .borrow_mut()
                .insert(name.value.clone(), c_name.to_string());
        }
        if let Statement::VariableDecl { name, ty, .. } = &*statement.borrow()
            && let Some(ty) = ty
        {
            let llvm_type = self.resolve_type(ty.clone())?;
            self.module.add_global(llvm_type, None, &name.value);
        }
        Ok(())
    }

    fn create_function_declarations_in_expr(&self, expr: Rc<RefCell<Expression>>) {
        if let Expression::Closure { body, .. } = &*expr.borrow() {
            for stmt in body {
                self.create_function_declarations(stmt.clone());
            }
        }
    }

    fn create_mangled_function_declaration(
        &self,
        name: &Token,
        parameters: &[Rc<RefCell<Parameter>>],
        ty: &Rc<RefCell<Type>>,
    ) {
        if let Type::Function(param_types, return_type, is_vararg, throws_types) = &*ty.borrow() {
            let mut all_params = param_types.clone();
            if throws_types.is_some() {
                let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                    Type::Struct("Int8".to_string(), WeakSymbol(std::rc::Weak::new()), vec![]),
                )))));
                all_params.insert(0, err_ty);
            }
            if let Ok(function_type) =
                self.get_function_type(return_type.clone(), all_params, *is_vararg)
            {
                let mangled = self.mangle_fn_name(&name.value, parameters);
                self.module.add_function(&mangled, function_type, None);
                self.mangled_fn_names
                    .borrow_mut()
                    .insert(name.value.clone(), mangled);
            }
        }
    }

    fn register_mangled_name(&self, src_name: &str, mangled: &str) {
        self.mangled_fn_names
            .borrow_mut()
            .insert(src_name.to_string(), mangled.to_string());
    }

    fn compute_vtable_method_list(&self, class_name: &str) -> Vec<(String, String)> {
        if let Some(cached) = self.vtable_method_lists.borrow().get(class_name) {
            return cached.clone();
        }
        let scope = self.program_scope.borrow();
        let Some(scope_ref) = scope.as_ref() else {
            return vec![];
        };
        let Some(symbol) = scope_ref.borrow().get_symbol(class_name) else {
            return vec![];
        };
        let sym_borrow = symbol.borrow();
        let Symbol::Class {
            methods,
            properties,
            subscripts,
            superclass,
            destrcutor,
            ..
        } = &*sym_borrow
        else {
            return vec![];
        };
        let own_method_names: Vec<String> = methods
            .iter()
            .filter_map(|m| m.borrow().name().ok())
            .collect();
        let _has_destrcutor = destrcutor.is_some();

        let mut own_property_entry_names: Vec<String> = Vec::new();
        for field in properties {
            if let Ok(Some(field_decl)) = field.borrow().get_decl()
                && let Ok(field_name) = field.borrow().name()
                && let Statement::VariableDecl {
                    accessors, token, ..
                } = &*field_decl.borrow()
            {
                let has_explicit_get = accessors
                    .iter()
                    .any(|a| matches!(a.kind, AccessorKind::Get));
                let has_explicit_set = accessors
                    .iter()
                    .any(|a| matches!(a.kind, AccessorKind::Set));

                if has_explicit_get || has_explicit_set {
                    if has_explicit_get {
                        own_property_entry_names.push(format!("{}.getter", field_name));
                    }
                    if has_explicit_set {
                        own_property_entry_names.push(format!("{}.setter", field_name));
                    }
                } else {
                    let is_var = matches!(
                        &token.ty,
                        TokenType::Keyword {
                            keyword: KeywordType::Var
                        }
                    );
                    own_property_entry_names.push(format!("{}.getter", field_name));
                    if is_var {
                        own_property_entry_names.push(format!("{}.setter", field_name));
                    }
                }
            }
        }
        for sub in subscripts {
            if let Ok(Some(decl)) = sub.borrow().get_decl()
                && let Statement::SubscriptDecl { accessors, .. } = &*decl.borrow()
            {
                if accessors
                    .iter()
                    .any(|a| matches!(a.kind, AccessorKind::Get))
                {
                    own_property_entry_names.push("subscript.getter".to_string());
                }
                if accessors
                    .iter()
                    .any(|a| matches!(a.kind, AccessorKind::Set))
                {
                    own_property_entry_names.push("subscript.setter".to_string());
                }
            }
        }

        let result: Vec<(String, String)> = if let Some(super_weak) = superclass {
            if let Some(super_sym) = super_weak.0.upgrade() {
                let super_borrow = super_sym.borrow();
                if let Symbol::Class {
                    name: super_name, ..
                } = &*super_borrow
                {
                    let super_entries = self.compute_vtable_method_list(&super_name);
                    let mut merged: Vec<(String, String)> = Vec::new();
                    for (entry_name, owner) in super_entries {
                        if own_method_names.contains(&entry_name)
                            || own_property_entry_names.contains(&entry_name)
                        {
                            merged.push((entry_name.clone(), class_name.to_string()));
                        } else {
                            merged.push((entry_name, owner));
                        }
                    }
                    for method_name in &own_method_names {
                        if !merged.iter().any(|(n, _)| n == method_name) {
                            merged.push((method_name.clone(), class_name.to_string()));
                        }
                    }
                    for prop_entry in &own_property_entry_names {
                        if !merged.iter().any(|(n, _)| n == prop_entry) {
                            merged.push((prop_entry.clone(), class_name.to_string()));
                        }
                    }
                    merged
                } else {
                    let mut all: Vec<(String, String)> = own_method_names
                        .iter()
                        .map(|n| (n.clone(), class_name.to_string()))
                        .collect();
                    for pe in &own_property_entry_names {
                        all.push((pe.clone(), class_name.to_string()));
                    }
                    all
                }
            } else {
                let mut all: Vec<(String, String)> = own_method_names
                    .iter()
                    .map(|n| (n.clone(), class_name.to_string()))
                    .collect();
                for pe in &own_property_entry_names {
                    all.push((pe.clone(), class_name.to_string()));
                }
                all
            }
        } else {
            let mut all: Vec<(String, String)> = own_method_names
                .iter()
                .map(|n| (n.clone(), class_name.to_string()))
                .collect();
            for pe in &own_property_entry_names {
                all.push((pe.clone(), class_name.to_string()));
            }
            all
        };

        drop(sym_borrow);

        let mut result = result;
        result.insert(0, ("deinit".to_string(), class_name.to_string()));

        self.vtable_method_lists
            .borrow_mut()
            .insert(class_name.to_string(), result.clone());
        result
    }

    fn class_has_vtable_methods(&self, class_name: &str) -> bool {
        !self.compute_vtable_method_list(class_name).is_empty()
    }

    fn create_vtable_types(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::ClassDecl { name, .. } = &*statement.borrow() {
            let class_name = &name.value;
            if !self.class_has_vtable_methods(class_name) {
                return;
            }
            let method_list = self.compute_vtable_method_list(class_name);
            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
            let mut field_types: Vec<BasicTypeEnum<'ctx>> = Vec::new();

            for _ in &method_list {
                field_types.push(ptr_ty.into());
            }

            let vtable_name = format!("vtable.{}", class_name);
            let vtable_type = self.context.opaque_struct_type(&vtable_name);
            vtable_type.set_body(&field_types, false);
            self.vtable_types
                .borrow_mut()
                .insert(class_name.clone(), vtable_type);
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_vtable_types(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_vtable_types(stmt.clone());
                }
            }
        }
    }

    fn get_vtable_slot_index(&self, class_name: &str, method_name: &str) -> Option<u32> {
        let method_list = self.compute_vtable_method_list(class_name);
        method_list
            .iter()
            .position(|(name, _)| name == method_name)
            .map(|i| i as u32)
    }

    fn is_final_class(&self, class_name: &str) -> bool {
        if let Some(scope) = self.program_scope.borrow().as_ref() {
            if let Some(sym) = scope.borrow().get_symbol(class_name) {
                let binding = sym.borrow();
                if let Symbol::Class { is_final, .. } = &*binding {
                    return *is_final;
                }
            }
        }
        false
    }

    fn is_final_method(&self, class_name: &str, method_name: &str) -> bool {
        if let Some(scope) = self.program_scope.borrow().as_ref() {
            if let Some(sym) = scope.borrow().get_symbol(class_name) {
                let binding = sym.borrow();
                if let Symbol::Class { methods, .. } = &*binding {
                    for m in methods {
                        let mb = m.borrow();
                        if let Symbol::ClassMethod { name, is_final, .. } = &*mb {
                            if name == method_name {
                                return *is_final;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn create_vtable_instances(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::ClassDecl { name, .. } = &*statement.borrow() {
            let class_name = &name.value;
            let method_list = self.compute_vtable_method_list(class_name);
            if method_list.is_empty() {
                return;
            }
            let Some(vtable_type) = self.vtable_types.borrow().get(class_name).copied() else {
                return;
            };

            let vtable_name = format!("__vtable.{}", class_name);
            if self.module.get_global(&vtable_name).is_some() {
                return;
            }

            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
            let mut const_vals: Vec<BasicValueEnum<'ctx>> = Vec::new();

            for (method_name, owner) in &method_list {
                let fn_name = format!("{}.{}", owner, method_name);
                if let Some(func) = self.module.get_function(&fn_name) {
                    let fn_ptr = func.as_global_value().as_pointer_value();
                    const_vals.push(fn_ptr.as_basic_value_enum());
                } else {
                    const_vals.push(ptr_ty.const_null().as_basic_value_enum());
                }
            }

            let init_val = vtable_type.const_named_struct(&const_vals);
            let global = self.module.add_global(vtable_type, None, &vtable_name);
            global.set_initializer(&init_val);
            global.set_constant(true);
            global.set_linkage(inkwell::module::Linkage::Internal);

            self.vtable_globals
                .borrow_mut()
                .insert(class_name.clone(), global);
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_vtable_instances(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_vtable_instances(stmt.clone());
                }
            }
        }
    }

    fn compute_protocol_witness_table_entries(
        &self,
        protocol_name: &str,
    ) -> Vec<(String, &'static str)> {
        let scope = self.program_scope.borrow();
        let Some(scope_ref) = scope.as_ref() else {
            return vec![];
        };
        let Some(symbol) = scope_ref.borrow().get_symbol(protocol_name) else {
            return vec![];
        };
        let sym_borrow = symbol.borrow();
        let Symbol::Protocol {
            methods,
            properties,
            ..
        } = &*sym_borrow
        else {
            return vec![];
        };
        let mut entries = Vec::new();
        for m in methods {
            if let Ok(name) = m.borrow().name() {
                entries.push((name, "method"));
            }
        }
        for p in properties {
            if let Ok(name) = p.borrow().name() {
                if let Some(decl) = p.borrow().get_decl().ok().flatten() {
                    let decl_borrow = decl.borrow();
                    if let Statement::VariableDecl { accessors, .. } = &*decl_borrow {
                        let has_get = accessors
                            .iter()
                            .any(|a| matches!(a.kind, AccessorKind::Get));
                        let has_set = accessors
                            .iter()
                            .any(|a| matches!(a.kind, AccessorKind::Set));
                        if has_get {
                            entries.push((format!("{}.getter", name), "getter"));
                        }
                        if has_set {
                            entries.push((format!("{}.setter", name), "setter"));
                        }
                    }
                }
            }
        }
        drop(sym_borrow);
        entries
    }

    fn get_compound_protocol_names(&self, types: &[Rc<RefCell<Type>>]) -> Vec<String> {
        types
            .iter()
            .filter_map(|t| match &*t.borrow() {
                Type::Protocol(name, ..) => Some(name.clone()),
                _ => None,
            })
            .collect()
    }

    fn build_compound_protocol_key(&self, names: &[String]) -> String {
        names.join(" & ")
    }

    fn find_overloaded_witness_fn(
        &self,
        type_name: &str,
        entry_name: &str,
        _entry_kind: &str,
        protocol_name: &str,
    ) -> Option<String> {
        let scope = self.program_scope.borrow();
        let scope_ref = scope.as_ref()?;
        let symbol = scope_ref.borrow().get_symbol(type_name)?;
        let sym_borrow = symbol.borrow();
        let methods = match &*sym_borrow {
            Symbol::Struct { methods, .. }
            | Symbol::Class { methods, .. }
            | Symbol::Enum { methods, .. } => methods.clone(),
            _ => return None,
        };
        drop(sym_borrow);
        drop(symbol);
        let proto_symbol = scope_ref.borrow().get_symbol(protocol_name)?;
        let proto_borrow = proto_symbol.borrow();
        let proto_methods = match &*proto_borrow {
            Symbol::Protocol { methods, .. } => methods.clone(),
            _ => return None,
        };
        drop(proto_borrow);
        drop(proto_symbol);

        let actual_entry_name = entry_name
            .strip_suffix(".getter")
            .or_else(|| entry_name.strip_suffix(".setter"))
            .unwrap_or(entry_name);

        let proto_params = proto_methods
            .iter()
            .find(|m| m.borrow().name().as_ref().ok() == Some(&actual_entry_name.to_string()))
            .and_then(|m| {
                let decl = m.borrow().get_decl().ok().flatten()?;
                let decl_ref = decl.borrow();
                if let Statement::FunctionDecl { parameters, ty, .. } = &*decl_ref {
                    let fn_ty = ty.as_ref()?;
                    let fn_borrow = fn_ty.borrow();
                    if let Type::Function(param_tys, _, _, None) = &*fn_borrow {
                        Some((parameters.clone(), param_tys.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        if let Some((_proto_params, proto_param_tys)) = proto_params {
            for method in &methods {
                if method.borrow().name().as_ref().ok() != Some(&actual_entry_name.to_string()) {
                    continue;
                }
                let decl = method.borrow().get_decl().ok().flatten()?;
                let decl_ref = decl.borrow();
                if let Statement::FunctionDecl { parameters, ty, .. } = &*decl_ref {
                    let fn_ty = ty.as_ref()?;
                    let fn_borrow = fn_ty.borrow();
                    if let Type::Function(param_tys, _, _, None) = &*fn_borrow {
                        if param_tys.len() == proto_param_tys.len()
                            && param_tys
                                .iter()
                                .zip(proto_param_tys.iter())
                                .all(|(a, b)| Self::types_compatible(&a.borrow(), &b.borrow()))
                        {
                            let base = format!("{}.{}", type_name, actual_entry_name);
                            return Some(self.mangle_fn_name(&base, &parameters));
                        }
                    }
                }
            }
        }
        None
    }

    fn create_protocol_witness_table_types(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::ProtocolDecl { name, .. } = &*statement.borrow() {
            let protocol_name = &name.value;
            if self
                .protocol_witness_table_types
                .borrow()
                .contains_key(protocol_name)
            {
                return;
            }
            let entries = self.compute_protocol_witness_table_entries(protocol_name);
            if entries.is_empty() {
                return;
            }
            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
            let mut field_types: Vec<BasicTypeEnum<'ctx>> = Vec::new();
            for _ in &entries {
                field_types.push(ptr_ty.into());
            }
            let wt_name = format!("protocol_wt.{}", protocol_name);
            let wt_type = self.context.opaque_struct_type(&wt_name);
            wt_type.set_body(&field_types, false);
            self.protocol_witness_table_types
                .borrow_mut()
                .insert(protocol_name.clone(), wt_type);
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_protocol_witness_table_types(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_protocol_witness_table_types(stmt.clone());
                }
            }
        }
    }

    fn create_existential_container_types(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::ProtocolDecl { name, .. } = &*statement.borrow() {
            let protocol_name = &name.value;
            if self
                .existential_container_types
                .borrow()
                .contains_key(protocol_name)
            {
                return;
            }
            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
            let field_types: Vec<BasicTypeEnum<'ctx>> = vec![ptr_ty.into(), ptr_ty.into()];
            let container_name = format!("existential.{}", protocol_name);
            let container_type = self.context.opaque_struct_type(&container_name);
            container_type.set_body(&field_types, false);
            self.existential_container_types
                .borrow_mut()
                .insert(protocol_name.clone(), container_type);
        } else if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_existential_container_types(stmt.clone());
            }
        } else if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_existential_container_types(stmt.clone());
                }
            }
        }
    }

    fn get_or_create_existential_container_for_compound(
        &self,
        compound_ty: &Type,
    ) -> Option<inkwell::types::StructType<'ctx>> {
        if let Type::Compound(types) = compound_ty {
            let names = self.get_compound_protocol_names(types);
            if names.is_empty() {
                return None;
            }
            let key = self.build_compound_protocol_key(&names);
            if let Some(ct) = self.existential_container_types.borrow().get(&key).copied() {
                return Some(ct);
            }
            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
            let n_fields = 1 + names.len();
            let mut field_types = Vec::with_capacity(n_fields);
            field_types.push(ptr_ty.into());
            for _ in &names {
                field_types.push(ptr_ty.into());
            }
            let container_name = format!("existential.{}", key);
            let container_type = self.context.opaque_struct_type(&container_name);
            container_type.set_body(&field_types, false);
            self.existential_container_types
                .borrow_mut()
                .insert(key.clone(), container_type);
            Some(container_type)
        } else {
            None
        }
    }

    fn create_protocol_witness_tables(&self, statement: Rc<RefCell<Statement>>) {
        let (type_name, type_arguments_opt) =
            if let Statement::ClassDecl {
                name, conformances, ..
            } = &*statement.borrow()
            {
                if conformances.is_empty() {
                    return;
                }
                (name.value.clone(), None)
            } else if let Statement::StructDecl {
                name, conformances, ..
            } = &*statement.borrow()
            {
                if conformances.is_empty() {
                    return;
                }
                (name.value.clone(), None)
            } else if let Statement::ExtensionDecl {
                type_name,
                conformances,
                type_arguments,
                ..
            } = &*statement.borrow()
            {
                if conformances.is_empty() {
                    return;
                }
                (type_name.value.clone(), type_arguments.clone())
            } else {
                return;
            };

        let type_suffix = if type_arguments_opt.is_some()
            && !self
                .type_args_to_abbreviation(type_arguments_opt.as_ref())
                .is_empty()
        {
            format!(
                "{}.{}",
                type_name,
                self.type_args_to_abbreviation(type_arguments_opt.as_ref())
            )
        } else {
            type_name.clone()
        };

        let conformances: Vec<Rc<RefCell<Expression>>> =
            if let Statement::ClassDecl { conformances, .. } = &*statement.borrow() {
                conformances.clone()
            } else if let Statement::StructDecl { conformances, .. } = &*statement.borrow() {
                conformances.clone()
            } else if let Statement::ExtensionDecl { conformances, .. } = &*statement.borrow() {
                conformances.clone()
            } else {
                return;
            };

        for conformance in &conformances {
            let conformance_expr = conformance.borrow();
            let protocol_name = if let Expression::Type { name: pn, .. } = &*conformance_expr {
                pn.value.clone()
            } else if let Expression::AnyType { inner, .. } = &*conformance_expr {
                if let Expression::Type { name: pn, .. } = &*inner.borrow() {
                    pn.value.clone()
                } else {
                    continue;
                }
            } else {
                continue;
            };
            drop(conformance_expr);

            let wt_lookup_key = if type_suffix != type_name {
                let key = format!("{}.{}", protocol_name, type_suffix);
                if let Some(wt_type) = self
                    .protocol_witness_table_types
                    .borrow()
                    .get(&key)
                    .copied()
                {
                    Some(wt_type)
                } else {
                    None
                }
            } else {
                self.protocol_witness_table_types
                    .borrow()
                    .get(&protocol_name)
                    .copied()
            };

            let Some(wt_type) = wt_lookup_key else {
                continue;
            };
            let entries = self.compute_protocol_witness_table_entries(&protocol_name);
            if entries.is_empty() {
                continue;
            }

            let key = (protocol_name.clone(), type_suffix.clone());
            let wt_global_name = format!("__protocol_wt.{}.{}", protocol_name, type_suffix);
            if self.module.get_global(&wt_global_name).is_some() {
                continue;
            }

            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
            let mut const_vals: Vec<BasicValueEnum<'ctx>> = Vec::new();
            for (entry_name, entry_kind) in &entries {
                let base_name = format!("{}.{}", type_suffix, entry_name);
                let fn_name = if self.overloaded_fn_names.borrow().contains(&base_name) {
                    self.find_overloaded_witness_fn(
                        &type_suffix,
                        entry_name,
                        entry_kind,
                        &protocol_name,
                    )
                    .unwrap_or_else(|| {
                        let generic_base = format!("{}.{}", type_name, entry_name);
                        if self.overloaded_fn_names.borrow().contains(&generic_base) {
                            self.find_overloaded_witness_fn(
                                &type_name,
                                entry_name,
                                entry_kind,
                                &protocol_name,
                            )
                            .unwrap_or(generic_base)
                        } else {
                            generic_base
                        }
                    })
                } else if self.module.get_function(&base_name).is_some() {
                    base_name
                } else {
                    format!("{}.{}", type_name, entry_name)
                };
                if let Some(func) = self.module.get_function(&fn_name) {
                    const_vals.push(
                        func.as_global_value()
                            .as_pointer_value()
                            .as_basic_value_enum(),
                    );
                } else {
                    const_vals.push(ptr_ty.const_null().as_basic_value_enum());
                }
            }
            let init_val = wt_type.const_named_struct(&const_vals);
            let global = self.module.add_global(wt_type, None, &wt_global_name);
            global.set_initializer(&init_val);
            global.set_constant(true);
            global.set_linkage(inkwell::module::Linkage::Internal);
            self.protocol_witness_tables
                .borrow_mut()
                .insert(key, global);
        }
        if let Statement::ModuleDecl { body, .. } = &*statement.borrow() {
            for stmt in body {
                self.create_protocol_witness_tables(stmt.clone());
            }
        }
        if let Statement::ConditionalBlock { clauses } = &*statement.borrow() {
            for clause in clauses {
                for stmt in &clause.body {
                    self.create_protocol_witness_tables(stmt.clone());
                }
            }
        }
    }

    fn declare_accessor_functions(
        &self,
        struct_name: &str,
        field_name: &str,
        accessors: &[Accessor],
        llvm_field_type: BasicTypeEnum<'ctx>,
    ) {
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::from(0));
        let fn_prefix = format!("{}.{}", struct_name, field_name);

        for accessor in accessors {
            let fn_name = match accessor.kind {
                AccessorKind::Get => self.mangle_fn_name(&format!("{}.getter", fn_prefix), &[]),
                AccessorKind::Set => self.mangle_fn_name(&format!("{}.setter", fn_prefix), &[]),
                AccessorKind::WillSet => {
                    self.mangle_fn_name(&format!("{}.willSet", fn_prefix), &[])
                }
                AccessorKind::DidSet => self.mangle_fn_name(&format!("{}.didSet", fn_prefix), &[]),
            };

            if self.module.get_function(&fn_name).is_some() {
                continue;
            }

            let is_getter = matches!(accessor.kind, AccessorKind::Get);
            let has_param = !is_getter;

            let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> = vec![ptr_type.into()];
            if has_param {
                param_types.push(llvm_field_type.into());
            }

            let fn_type = if is_getter {
                llvm_field_type.fn_type(&param_types, false)
            } else {
                self.context.void_type().fn_type(&param_types, false)
            };

            self.module.add_function(&fn_name, fn_type, None);
        }
    }

    fn declare_accessor_fn(
        &self,
        struct_name: &str,
        field_name: &str,
        is_getter: bool,
        llvm_field_type: BasicTypeEnum<'ctx>,
    ) {
        let fn_name = if is_getter {
            self.mangle_fn_name(&format!("{}.{}.getter", struct_name, field_name), &[])
        } else {
            self.mangle_fn_name(&format!("{}.{}.setter", struct_name, field_name), &[])
        };
        if self.module.get_function(&fn_name).is_some() {
            return;
        }
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::from(0));
        let param_types: Vec<BasicMetadataTypeEnum<'ctx>> = if is_getter {
            vec![ptr_type.into()]
        } else {
            vec![ptr_type.into(), llvm_field_type.into()]
        };
        let fn_type = if is_getter {
            llvm_field_type.fn_type(&param_types, false)
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };
        self.module.add_function(&fn_name, fn_type, None);
    }

    fn generate_accessor_function(
        &self,
        fn_prefix: &str,
        backing_var_name: &str,
        accessor: &Accessor,
        llvm_var_type: BasicTypeEnum<'ctx>,
        struct_name: Option<&str>,
    ) -> Result<()> {
        let (fn_name, param_names, is_getter): (String, Vec<Option<String>>, bool) =
            match accessor.kind {
                AccessorKind::Get => (
                    self.mangle_fn_name(&format!("{}.getter", fn_prefix), &[]),
                    vec![],
                    true,
                ),
                AccessorKind::Set => {
                    let param_name = accessor
                        .parameter
                        .as_ref()
                        .map(|t| t.value.clone())
                        .unwrap_or_else(|| "newValue".to_string());
                    (
                        self.mangle_fn_name(&format!("{}.setter", fn_prefix), &[]),
                        vec![Some(param_name)],
                        false,
                    )
                }
                AccessorKind::WillSet => {
                    let param_name = accessor
                        .parameter
                        .as_ref()
                        .map(|t| t.value.clone())
                        .unwrap_or_else(|| "newValue".to_string());
                    (
                        self.mangle_fn_name(&format!("{}.willSet", fn_prefix), &[]),
                        vec![Some(param_name)],
                        false,
                    )
                }
                AccessorKind::DidSet => {
                    let param_name = accessor
                        .parameter
                        .as_ref()
                        .map(|t| t.value.clone())
                        .unwrap_or_else(|| "oldValue".to_string());
                    (
                        self.mangle_fn_name(&format!("{}.didSet", fn_prefix), &[]),
                        vec![Some(param_name)],
                        false,
                    )
                }
            };

        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::from(0));
        let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> = vec![ptr_type.into()];
        let mut all_param_names: Vec<String> = vec!["__struct_ptr".to_string()];
        for pn in &param_names {
            if let Some(name) = pn {
                param_types.push(llvm_var_type.into());
                all_param_names.push(name.clone());
            }
        }

        let fn_type = if is_getter {
            llvm_var_type.fn_type(&param_types, false)
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };

        let function: FunctionValue<'ctx> =
            if let Some(existing_fn) = self.module.get_function(&fn_name) {
                if existing_fn.get_first_basic_block().is_some() {
                    return Ok(());
                }
                existing_fn
            } else {
                self.module.add_function(&fn_name, fn_type, None)
            };
        let current_block = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let ptr_param = function.get_nth_param(0).unwrap();
        let ptr_var = ptr_param.into_pointer_value();

        self.enter_scope();

        if struct_name.is_some() {
            *self.current_accessor_struct.borrow_mut() =
                Some((struct_name.unwrap().to_string(), ptr_var));
            self.declare_variable("self".to_string(), ptr_var);
        } else {
            self.declare_variable(format!("_{}", backing_var_name), ptr_var);
        }

        for (i, param_name) in all_param_names.iter().enumerate().skip(1) {
            let param_val = function.get_nth_param(i as u32).unwrap();
            let alloca_name = self.unique_alloca_name(param_name);
            let ptr = self.builder.build_alloca(llvm_var_type, &alloca_name)?;
            self.builder.build_store(ptr, param_val)?;
            self.declare_variable(param_name.clone(), ptr);
        }

        self.enter_scope_with_stmts(&accessor.body)?;
        let mut has_return = false;
        let mut last_expr_value = None;
        for (i, stmt) in accessor.body.iter().enumerate() {
            let is_last = i == accessor.body.len() - 1;
            if is_last && is_getter {
                if let Statement::ExpressionStatement { expression } = &*stmt.borrow() {
                    let val = self.resolve_expression(expression.clone())?;
                    last_expr_value = val;
                    continue;
                }
            }
            let terminates = self.resolve_statement(stmt.clone())?;
            if terminates {
                has_return = true;
                break;
            }
        }
        if is_getter && !has_return {
            if let Some(val) = last_expr_value {
                self.builder.build_return(Some(&val))?;
            } else if let Some(ptr) = self.lookup_variable(&format!("_{}", backing_var_name)) {
                let val = self.builder.build_load(llvm_var_type, ptr, "")?;
                self.builder.build_return(Some(&val))?;
            } else {
                self.builder.build_return(None)?;
            }
        } else if !is_getter && !has_return {
            self.builder.build_return(None)?;
        }
        self.exit_scope();

        self.exit_scope();

        *self.current_accessor_struct.borrow_mut() = None;

        if let Some(block) = current_block {
            self.builder.position_at_end(block);
        }
        Ok(())
    }

    fn generate_auto_accessor(
        &self,
        fn_prefix: &str,
        field_name: &str,
        is_getter: bool,
        llvm_var_type: BasicTypeEnum<'ctx>,
        struct_name: &str,
        accessors: &[Accessor],
    ) -> Result<()> {
        let fn_name = if is_getter {
            self.mangle_fn_name(&format!("{}.getter", fn_prefix), &[])
        } else {
            self.mangle_fn_name(&format!("{}.setter", fn_prefix), &[])
        };

        let function: FunctionValue<'ctx> =
            if let Some(existing_fn) = self.module.get_function(&fn_name) {
                if existing_fn.get_first_basic_block().is_some() {
                    return Ok(());
                }
                existing_fn
            } else {
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::from(0));
                let param_types: Vec<BasicMetadataTypeEnum<'ctx>> = if is_getter {
                    vec![ptr_type.into()]
                } else {
                    vec![ptr_type.into(), llvm_var_type.into()]
                };
                let fn_type = if is_getter {
                    llvm_var_type.fn_type(&param_types, false)
                } else {
                    self.context.void_type().fn_type(&param_types, false)
                };
                self.module.add_function(&fn_name, fn_type, None)
            };
        let current_block = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let ptr_param = function.get_nth_param(0).unwrap();
        let class_ptr = ptr_param.into_pointer_value();

        let body_result = (|| -> Result<()> {
            let field_index = self.get_stored_class_field_index(struct_name, field_name)?;
            let class_type = *self.class_types.borrow().get(struct_name).unwrap();
            let field_ptr =
                self.builder
                    .build_struct_gep(class_type, class_ptr, field_index as u32, "")?;

            if is_getter {
                let val = self.builder.build_load(llvm_var_type, field_ptr, "")?;
                self.builder.build_return(Some(&val))?;
            } else {
                let new_val = function.get_nth_param(1).unwrap();

                let has_willset = accessors
                    .iter()
                    .any(|a| matches!(a.kind, AccessorKind::WillSet));
                let has_didset = accessors
                    .iter()
                    .any(|a| matches!(a.kind, AccessorKind::DidSet));

                let old_val = if has_didset {
                    Some(self.builder.build_load(llvm_var_type, field_ptr, "")?)
                } else {
                    None
                };

                if has_willset {
                    let willset_name = format!("{}.willSet", fn_prefix);
                    if let Some(willset_fn) = self.module.get_function(&willset_name) {
                        self.builder.build_call(
                            willset_fn,
                            &[class_ptr.into(), new_val.into()],
                            "",
                        )?;
                    }
                }

                self.builder.build_store(field_ptr, new_val)?;

                if let Some(old) = old_val {
                    let didset_name = format!("{}.didSet", fn_prefix);
                    if let Some(didset_fn) = self.module.get_function(&didset_name) {
                        self.builder
                            .build_call(didset_fn, &[class_ptr.into(), old.into()], "")?;
                    }
                }

                self.builder.build_return(None)?;
            }
            Ok(())
        })();

        if body_result.is_err() {
            self.builder.build_unreachable()?;
        }
        body_result?;

        if let Some(block) = current_block {
            self.builder.position_at_end(block);
        }
        Ok(())
    }

    fn compute_compound_assign(
        &self,
        left_val: BasicValueEnum<'ctx>,
        right_val: BasicValueEnum<'ctx>,
        operator: AssignmentOperator,
    ) -> Result<BasicValueEnum<'ctx>> {
        match operator {
            AssignmentOperator::PlusAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_int_add(l, r, "")?.into())
                } else if let (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_float_add(l, r, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for += assignment");
                }
            }
            AssignmentOperator::MinusAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_int_sub(l, r, "")?.into())
                } else if let (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_float_sub(l, r, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for -= assignment");
                }
            }
            AssignmentOperator::MultiplyAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_int_mul(l, r, "")?.into())
                } else if let (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_float_mul(l, r, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for *= assignment");
                }
            }
            AssignmentOperator::DivideAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_int_signed_div(l, r, "")?.into())
                } else if let (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_float_div(l, r, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for /= assignment");
                }
            }
            AssignmentOperator::ModulusAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_int_signed_rem(l, r, "")?.into())
                } else if let (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_float_rem(l, r, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for %= assignment");
                }
            }
            AssignmentOperator::BitAndAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_and(l, r, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for &= assignment");
                }
            }
            AssignmentOperator::BitOrAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_or(l, r, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for |= assignment");
                }
            }
            AssignmentOperator::LeftShiftAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_left_shift(l, r, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for <<= assignment");
                }
            }
            AssignmentOperator::RightShiftAssign => {
                if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                    (left_val, right_val)
                {
                    Ok(self.builder.build_right_shift(l, r, false, "")?.into())
                } else {
                    anyhow::bail!("Invalid types for >>= assignment");
                }
            }
            _ => anyhow::bail!("Unsupported compound assignment"),
        }
    }

    fn resolve_block_expression(&self, body: &[Rc<RefCell<Statement>>]) -> Result<bool> {
        self.enter_scope_with_stmts(body)?;
        self.resolve_block_stmts(body)
    }

    fn resolve_block_and_get_value(
        &self,
        body: &[Rc<RefCell<Statement>>],
    ) -> Result<(bool, Option<BasicValueEnum<'ctx>>)> {
        self.enter_scope_with_stmts(body)?;
        let len = body.len();
        let mut last_value = None;
        for (i, stmt) in body.iter().enumerate() {
            let is_last = i == len - 1;
            let terminates = match &*stmt.borrow() {
                Statement::ExpressionStatement { expression } => {
                    let val = self.resolve_expression(expression.clone())?;
                    if is_last {
                        last_value = val;
                    }
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
    }

    fn resolve_statement(&self, statement: Rc<RefCell<Statement>>) -> Result<bool> {
        match &*statement.borrow() {
            Statement::VariableDecl {
                name,
                pattern: decl_pattern,
                initializer,
                ty,
                accessors,
                token,
                ownership,
                ..
            } => {
                if let Some(Pattern::Tuple(items)) = decl_pattern {
                    return self.resolve_tuple_pattern_decl(items, initializer, ty);
                }
                let llvm_var_type = if let Some(ty) = ty {
                    self.resolve_type(ty.clone())?
                } else {
                    self.context.i32_type().into()
                };

                if let Some(ptr) = self.lookup_variable(&name.value) {
                    let is_existential_ty = ty.as_ref().map_or(false, |t| {
                        matches!(&*t.borrow(), Type::Protocol(..))
                            || matches!(&*t.borrow(), Type::Compound(..))
                    });

                    if is_existential_ty {
                        if let Some(init) = initializer
                            && let Some(init_val) = self.resolve_expression(init.clone())?
                        {
                            if let Some(ty) = ty {
                                let ty_borrow = ty.borrow();
                                match &*ty_borrow {
                                    Type::Protocol(protocol_name, ..) => {
                                        let protocol_name = protocol_name.clone();
                                        drop(ty_borrow);
                                        if let Some(ct) = self
                                            .existential_container_types
                                            .borrow()
                                            .get(&protocol_name)
                                            .copied()
                                        {
                                            let (data_ptr, is_class_ref) = match init_val {
                                                BasicValueEnum::PointerValue(p) => (p, true),
                                                val => {
                                                    let heap =
                                                        self.heap_allocate(val.get_type())?;
                                                    self.builder.build_store(heap, val)?;
                                                    (heap, false)
                                                }
                                            };
                                            let value_field =
                                                self.builder.build_struct_gep(ct, ptr, 0, "")?;
                                            self.builder.build_store(value_field, data_ptr)?;
                                            if is_class_ref {
                                                self.emit_retain(data_ptr)?;
                                                self.container_refs.borrow_mut().push(ptr);
                                            }

                                            if let Some(init_expr) = initializer {
                                                let concrete_name =
                                                    self.extract_concrete_type_name(init_expr);
                                                if let Some(ref concrete_name) = concrete_name {
                                                    let key = (
                                                        protocol_name.clone(),
                                                        concrete_name.clone(),
                                                    );
                                                    if let Some(wt_global) = self
                                                        .protocol_witness_tables
                                                        .borrow()
                                                        .get(&key)
                                                        .copied()
                                                    {
                                                        let wt_field = self
                                                            .builder
                                                            .build_struct_gep(ct, ptr, 1, "")?;
                                                        self.builder.build_store(
                                                            wt_field,
                                                            wt_global.as_pointer_value(),
                                                        )?;
                                                    }
                                                }
                                            }
                                        } else {
                                            self.builder.build_store(ptr, init_val)?;
                                        }
                                    }
                                    Type::Compound(types) => {
                                        let names = self.get_compound_protocol_names(types);
                                        if !names.is_empty() {
                                            let compound_key =
                                                self.build_compound_protocol_key(&names);
                                            if let Some(ct) = self
                                                .existential_container_types
                                                .borrow()
                                                .get(&compound_key)
                                                .copied()
                                            {
                                                let (data_ptr, is_class_ref) = match init_val {
                                                    BasicValueEnum::PointerValue(p) => (p, true),
                                                    val => {
                                                        let heap =
                                                            self.heap_allocate(val.get_type())?;
                                                        self.builder.build_store(heap, val)?;
                                                        (heap, false)
                                                    }
                                                };
                                                let value_field = self
                                                    .builder
                                                    .build_struct_gep(ct, ptr, 0, "")?;
                                                self.builder.build_store(value_field, data_ptr)?;
                                                if is_class_ref {
                                                    self.emit_retain(data_ptr)?;
                                                    self.container_refs.borrow_mut().push(ptr);
                                                }

                                                if let Some(init_expr) = initializer {
                                                    let concrete_name =
                                                        self.extract_concrete_type_name(init_expr);
                                                    if let Some(ref concrete_name) = concrete_name {
                                                        for (i, pname) in names.iter().enumerate() {
                                                            let key = (
                                                                pname.clone(),
                                                                concrete_name.clone(),
                                                            );
                                                            if let Some(wt_global) = self
                                                                .protocol_witness_tables
                                                                .borrow()
                                                                .get(&key)
                                                                .copied()
                                                            {
                                                                let wt_field =
                                                                    self.builder.build_struct_gep(
                                                                        ct,
                                                                        ptr,
                                                                        (i + 1) as u32,
                                                                        "",
                                                                    )?;
                                                                self.builder.build_store(
                                                                    wt_field,
                                                                    wt_global.as_pointer_value(),
                                                                )?;
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                self.builder.build_store(ptr, init_val)?;
                                            }
                                        } else {
                                            self.builder.build_store(ptr, init_val)?;
                                        }
                                    }
                                    _ => {
                                        self.builder.build_store(ptr, init_val)?;
                                    }
                                }
                            }
                        }
                    } else if let Some(init) = initializer
                        && let Expression::Call {
                            callee, parameters, ..
                        } = &*init.borrow()
                        && let Expression::Variable {
                            name: callee_name, ..
                        } = &*callee.borrow()
                        && self.module.get_function(&callee_name.value).is_none()
                    {
                        let type_name = &callee_name.value;
                        let fn_name = self
                            .mangled_fn_names
                            .borrow()
                            .get(&format!("{}.init", type_name))
                            .cloned()
                            .unwrap_or_else(|| format!("{}.init", type_name));
                        if let Some(function) = self.module.get_function(&fn_name) {
                            let is_inline = ty
                                .as_ref()
                                .map_or(false, |t| matches!(&*t.borrow(), Type::Inline(_, _)));
                            if let Some(class_type) =
                                self.class_types.borrow().get(type_name).cloned()
                            {
                                let obj_ptr = if is_inline {
                                    ptr
                                } else {
                                    self.heap_allocate(class_type.as_basic_type_enum())?
                                };
                                let vtable_global =
                                    self.vtable_globals.borrow().get(type_name).copied();
                                if let Some(vt_global) = vtable_global {
                                    let vtable_ptr_gep = self
                                        .builder
                                        .build_struct_gep(class_type, obj_ptr, 0, "")?;
                                    self.builder.build_store(
                                        vtable_ptr_gep,
                                        vt_global.as_pointer_value(),
                                    )?;
                                }
                                let i64_ty = self.context.i64_type();
                                let rc_ptr =
                                    self.builder.build_struct_gep(class_type, obj_ptr, 1, "")?;
                                let rc_val = if is_inline { 0 } else { 1 };
                                self.builder
                                    .build_store(rc_ptr, i64_ty.const_int(rc_val, false))?;
                                let mut args = Vec::new();
                                args.push(obj_ptr.into());
                                for param in parameters {
                                    let arg_val =
                                        self.resolve_expression(param.expression.clone())?.unwrap();
                                    args.push(arg_val.into());
                                }
                                self.builder.build_call(function, &args, "")?;
                                if !is_inline {
                                    self.builder.build_store(ptr, obj_ptr)?;
                                    if !matches!(
                                        ownership,
                                        OwnershipModifier::Weak | OwnershipModifier::Unowned
                                    ) {
                                        self.class_refs.borrow_mut().push(ptr);
                                    }
                                }
                            } else {
                                let fn_ret_type = function.get_type().get_return_type();
                                if fn_ret_type.is_some() {
                                    let st = self
                                        .struct_types
                                        .borrow()
                                        .get(type_name)
                                        .cloned()
                                        .ok_or_else(|| {
                                            anyhow::anyhow!("Struct type '{}' not found", type_name)
                                        })?;
                                    let struct_ptr = self.builder.build_alloca(st, "")?;
                                    let mut args = Vec::new();
                                    args.push(struct_ptr.into());
                                    for param in parameters {
                                        let arg_val = self
                                            .resolve_expression(param.expression.clone())?
                                            .unwrap();
                                        args.push(arg_val.into());
                                    }
                                    let call_result =
                                        self.builder.build_call(function, &args, "")?;
                                    let ret_val = match call_result.try_as_basic_value() {
                                        inkwell::values::ValueKind::Basic(val) => val,
                                        _ => return Ok(false),
                                    };
                                    self.builder.build_store(ptr, ret_val)?;
                                } else {
                                    let mut args = Vec::new();
                                    args.push(ptr.into());
                                    for param in parameters {
                                        let arg_val = self
                                            .resolve_expression(param.expression.clone())?
                                            .unwrap();
                                        args.push(arg_val.into());
                                    }
                                    self.builder.build_call(function, &args, "")?;
                                }
                            }
                        } else if let Some(init_val) = self.resolve_expression(init.clone())? {
                            self.builder.build_store(ptr, init_val)?;
                        }
                    } else if let Some(init) = initializer
                        && let Some(init_val) = self.resolve_expression(init.clone())?
                    {
                        self.builder.build_store(ptr, init_val)?;
                    }
                }

                if let Some(ref sname) = *self.current_struct.borrow() {
                    let is_class = self.class_types.borrow().contains_key(sname);
                    let fn_prefix = format!("{}.{}", sname, name.value);
                    let has_explicit_get = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Get));
                    let has_explicit_set = accessors
                        .iter()
                        .any(|a| matches!(a.kind, AccessorKind::Set));

                    if has_explicit_get || has_explicit_set {
                        for accessor in accessors {
                            self.generate_accessor_function(
                                &fn_prefix,
                                &name.value,
                                accessor,
                                llvm_var_type,
                                Some(sname),
                            )?;
                        }
                    } else if is_class {
                        let is_var = matches!(
                            &token.ty,
                            TokenType::Keyword {
                                keyword: KeywordType::Var
                            }
                        );
                        self.generate_auto_accessor(
                            &fn_prefix,
                            &name.value,
                            true,
                            llvm_var_type,
                            sname,
                            accessors,
                        )?;
                        if is_var {
                            self.generate_auto_accessor(
                                &fn_prefix,
                                &name.value,
                                false,
                                llvm_var_type,
                                sname,
                                accessors,
                            )?;
                        }
                    } else if !accessors.is_empty() {
                        for accessor in accessors {
                            self.generate_accessor_function(
                                &fn_prefix,
                                &name.value,
                                accessor,
                                llvm_var_type,
                                Some(sname),
                            )?;
                        }
                    }
                } else if !accessors.is_empty() {
                    let fn_prefix = name.value.clone();
                    for accessor in accessors {
                        self.generate_accessor_function(
                            &fn_prefix,
                            &name.value,
                            accessor,
                            llvm_var_type,
                            None,
                        )?;
                    }
                }
                Ok(false)
            }
            Statement::SubscriptDecl {
                parameters,
                accessors,
                ty: Some(ty),
                ..
            } => {
                if let Some(ref sname) = *self.current_struct.borrow() {
                    for accessor in accessors {
                        let fn_name = match accessor.kind {
                            AccessorKind::Get => {
                                self.mangle_fn_name(&format!("{}.subscript.getter", sname), parameters)
                            }
                            AccessorKind::Set => {
                                self.mangle_fn_name(&format!("{}.subscript.setter", sname), parameters)
                            }
                            _ => continue,
                        };
                        let function = if let Some(f) = self.module.get_function(&fn_name) {
                            if f.get_first_basic_block().is_some() {
                                continue;
                            }
                            f
                        } else {
                            continue;
                        };
                        let current_block = self.builder.get_insert_block();
                        let entry = self.context.append_basic_block(function, "entry");
                        self.builder.position_at_end(entry);
                        let ptr_param = function.get_nth_param(0).unwrap().into_pointer_value();
                        self.enter_scope();
                        self.declare_variable("self".to_string(), ptr_param);
                        *self.current_accessor_struct.borrow_mut() =
                            Some((sname.clone(), ptr_param));
                        let mut param_idx = 1u32;
                        for param in parameters {
                            let param_name = param.borrow().name.value.clone();
                            if param_name != "_" {
                                if let Some(param_val) = function.get_nth_param(param_idx) {
                                    if let Some(pt) = param
                                        .borrow()
                                        .ty
                                        .clone()
                                        .and_then(|t| self.resolve_type(t).ok())
                                    {
                                        let alloca_name = self.unique_alloca_name(&param_name);
                                        let ptr = self.builder.build_alloca(pt, &alloca_name)?;
                                        self.builder.build_store(ptr, param_val)?;
                                        self.declare_variable(param_name, ptr);
                                    }
                                }
                                param_idx += 1;
                            }
                        }
                        if matches!(accessor.kind, AccessorKind::Set) {
                            let setter_param_name = accessor
                                .parameter
                                .as_ref()
                                .map(|t| t.value.clone())
                                .unwrap_or_else(|| "newValue".to_string());
                            if let Some(new_val) = function.get_nth_param(param_idx) {
                                let (_, return_type, _) = match &*ty.borrow() {
                                    Type::Function(_, ret, _, _) => ((), ret.clone(), false),
                                    _ => continue,
                                };
                                if let Ok(ret_ty) = self.resolve_type(return_type) {
                                    let alloca_name = self.unique_alloca_name(&setter_param_name);
                                    let ptr = self.builder.build_alloca(ret_ty, &alloca_name)?;
                                    self.builder.build_store(ptr, new_val)?;
                                    self.declare_variable(setter_param_name, ptr);
                                }
                            }
                        }
                        self.enter_scope_with_stmts(&accessor.body)?;
                        let mut has_return = false;
                        for s in &accessor.body {
                            if self.resolve_statement(s.clone())? {
                                has_return = true;
                                break;
                            }
                        }
                        if !has_return {
                            self.builder.build_return(None)?;
                        }
                        self.exit_scope();
                        self.exit_scope();
                        *self.current_accessor_struct.borrow_mut() = None;
                        if let Some(block) = current_block {
                            self.builder.position_at_end(block);
                        }
                    }
                }
                Ok(false)
            }
            Statement::While {
                condition, body, ..
            } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let while_bb = self.context.append_basic_block(fn_val, "while_cond");
                let body_bb = self.context.append_basic_block(fn_val, "while_body");
                let exit_bb = self.context.append_basic_block(fn_val, "while_exit");

                self.loop_break_targets.borrow_mut().push(exit_bb);
                self.loop_continue_targets.borrow_mut().push(while_bb);

                self.builder.build_unconditional_branch(while_bb)?;
                self.builder.position_at_end(while_bb);

                let cond_val = self.resolve_expression(condition.clone())?.unwrap();
                let cond_int = cond_val.into_int_value();
                self.builder
                    .build_conditional_branch(cond_int, body_bb, exit_bb)?;

                self.builder.position_at_end(body_bb);
                let terminates = self.resolve_block_expression(body)?;

                if !terminates {
                    self.builder.build_unconditional_branch(while_bb)?;
                }

                self.loop_break_targets.borrow_mut().pop();
                self.loop_continue_targets.borrow_mut().pop();

                self.builder.position_at_end(exit_bb);
                Ok(false)
            }
            Statement::Loop { body, .. } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let body_bb = self.context.append_basic_block(fn_val, "loop_body");
                let exit_bb = self.context.append_basic_block(fn_val, "loop_exit");

                self.loop_break_targets.borrow_mut().push(exit_bb);
                self.loop_continue_targets.borrow_mut().push(body_bb);

                self.builder.build_unconditional_branch(body_bb)?;

                self.builder.position_at_end(body_bb);
                let terminates = self.resolve_block_expression(body)?;

                if !terminates {
                    self.builder.build_unconditional_branch(body_bb)?;
                }

                self.loop_break_targets.borrow_mut().pop();
                self.loop_continue_targets.borrow_mut().pop();

                self.builder.position_at_end(exit_bb);
                Ok(false)
            }
            Statement::RepeatWhile {
                body, condition, ..
            } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let body_bb = self.context.append_basic_block(fn_val, "repeat_body");
                let cond_bb = self.context.append_basic_block(fn_val, "repeat_cond");
                let exit_bb = self.context.append_basic_block(fn_val, "repeat_exit");

                self.loop_break_targets.borrow_mut().push(exit_bb);
                self.loop_continue_targets.borrow_mut().push(cond_bb);

                self.builder.build_unconditional_branch(body_bb)?;

                self.builder.position_at_end(body_bb);
                let terminates = self.resolve_block_expression(body)?;

                if !terminates {
                    self.builder.build_unconditional_branch(cond_bb)?;
                }

                self.loop_break_targets.borrow_mut().pop();
                self.loop_continue_targets.borrow_mut().pop();

                self.builder.position_at_end(cond_bb);
                let cond_val = self.resolve_expression(condition.clone())?.unwrap();
                let cond_int = cond_val.into_int_value();
                self.builder
                    .build_conditional_branch(cond_int, body_bb, exit_bb)?;

                self.builder.position_at_end(exit_bb);
                Ok(false)
            }
            Statement::For {
                pattern,
                iterator,
                body,
                ..
            } => {
                let iter_val = self.resolve_expression(iterator.clone())?.unwrap();
                let iter_ptr = if let BasicValueEnum::PointerValue(p) = iter_val {
                    p
                } else {
                    let p = self.builder.build_alloca(iter_val.get_type(), "")?;
                    self.builder.build_store(p, iter_val)?;
                    p
                };

                let function = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                let cond_bb = self.context.append_basic_block(function, "for_cond");
                let body_bb = self.context.append_basic_block(function, "for_body");
                let exit_bb = self.context.append_basic_block(function, "for_exit");

                self.loop_break_targets.borrow_mut().push(exit_bb);
                self.loop_continue_targets.borrow_mut().push(cond_bb);

                let _ = self.builder.build_unconditional_branch(cond_bb);
                self.builder.position_at_end(cond_bb);

                let iter_ty = iterator.borrow().get_ty().ok().flatten();
                let next_fn = iter_ty.as_ref().and_then(|ty| {
                    let type_name = match &*ty.borrow() {
                        Type::Struct(n, _, _) | Type::Class(n, _, _) | Type::Enum(n, _, _) => {
                            n.clone()
                        }
                        _ => return None,
                    };
                    self.module.get_function(&format!("{}.next", type_name))
                });

                if let Some(next_fn) = next_fn {
                    let result = self.builder.build_call(next_fn, &[iter_ptr.into()], "")?;
                    let result_val = match result.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(val) => val,
                        _ => return Ok(false),
                    };
                    let enum_llvm_type = result_val.get_type();
                    let alloca = self.builder.build_alloca(enum_llvm_type, "")?;
                    self.builder.build_store(alloca, result_val)?;
                    let tag_ptr = self.builder.build_struct_gep(
                        enum_llvm_type.into_struct_type(),
                        alloca,
                        0,
                        "",
                    )?;
                    let tag = self
                        .builder
                        .build_load(self.context.i8_type(), tag_ptr, "")?;
                    let some_tag = self.context.i8_type().const_int(1, false);
                    let cond = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        tag.into_int_value(),
                        some_tag,
                        "",
                    )?;
                    self.builder
                        .build_conditional_branch(cond, body_bb, exit_bb)?;

                    self.builder.position_at_end(body_bb);
                    let payload_ptr = self.builder.build_struct_gep(
                        enum_llvm_type.into_struct_type(),
                        alloca,
                        1,
                        "",
                    )?;
                    let payload_ty = enum_llvm_type
                        .into_struct_type()
                        .get_field_type_at_index(1)
                        .unwrap();
                    let payload_val = self.builder.build_load(payload_ty, payload_ptr, "")?;
                    if let Pattern::Identifier(name) = pattern.as_ref() {
                        if name.value != "_" {
                            let var_ptr = self.builder.build_alloca(payload_ty, &name.value)?;
                            self.builder.build_store(var_ptr, payload_val)?;
                            self.declare_variable(name.value.clone(), var_ptr);
                        }
                    } else if let Pattern::ValueBinding(inner) = pattern.as_ref() {
                        if let Pattern::Identifier(name) = inner.as_ref() {
                            if name.value != "_" {
                                let var_ptr = self.builder.build_alloca(payload_ty, &name.value)?;
                                self.builder.build_store(var_ptr, payload_val)?;
                                self.declare_variable(name.value.clone(), var_ptr);
                            }
                        }
                    }
                    self.resolve_block_expression(body)?;
                    self.builder.build_unconditional_branch(cond_bb)?;
                    self.builder.position_at_end(exit_bb);
                } else {
                    self.resolve_block_expression(body)?;
                }

                self.loop_break_targets.borrow_mut().pop();
                self.loop_continue_targets.borrow_mut().pop();

                Ok(false)
            }
            Statement::FunctionDecl {
                ty: Some(ty),
                name,
                parameters,
                body,
                static_method,
                attributes,
                ..
            } => {
                if let Type::Function(_parameter_types, return_type, _, throws_types) =
                    &*ty.borrow()
                {
                    let is_throwing = throws_types.is_some();
                    let saved_struct = self.current_struct.borrow_mut().take();
                    self.class_refs.borrow_mut().clear();
                    self.container_refs.borrow_mut().clear();
                    let fn_name = if let Some(cname) = attributes
                        .iter()
                        .find(|a| a.name == "cname")
                        .and_then(|a| a.value.as_deref())
                    {
                        cname.to_string()
                    } else if let Some(struct_name) = &saved_struct {
                        let base = format!("{}.{}", struct_name, name.value);
                        self.mangle_fn_name(&base, parameters)
                    } else {
                        self.mangle_fn_name(&name.value, parameters)
                    };
                    let function = self.module.get_function(&fn_name).unwrap();

                    let current_block = self.builder.get_insert_block();

                    let entry_block = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry_block);

                    self.enter_scope();
                    let is_class_method;
                    let is_struct_method;
                    if let Some(struct_name) = &saved_struct {
                        is_class_method = self.class_types.borrow().contains_key(struct_name);
                        is_struct_method = self.struct_types.borrow().contains_key(struct_name);
                    } else {
                        is_class_method = false;
                        is_struct_method = false;
                    }
                    if is_struct_method || is_class_method {
                        if !static_method {
                            let self_ptr = function.get_nth_param(0).unwrap();
                            let self_ptr = self_ptr.into_pointer_value();
                            self.declare_variable("self".to_string(), self_ptr);
                            let mut param_offset = 1u32;
                            if is_throwing {
                                let err_ptr = function
                                    .get_nth_param(param_offset)
                                    .unwrap()
                                    .into_pointer_value();
                                *self.error_ptr.borrow_mut() = Some(err_ptr);
                                param_offset += 1;
                            }
                            for (i, param) in parameters.iter().enumerate() {
                                if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                    continue;
                                }
                                let param_name = &param.borrow().name.value;
                                let llvm_type =
                                    self.resolve_type(param.borrow().ty.clone().unwrap())?;
                                let alloca_name = self.unique_alloca_name(param_name);
                                let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                                let param_value =
                                    function.get_nth_param(i as u32 + param_offset).unwrap();
                                self.builder.build_store(ptr, param_value)?;
                                self.declare_variable(param_name.clone(), ptr);
                            }
                        } else {
                            let mut param_offset = 0u32;
                            if is_throwing {
                                let err_ptr = function
                                    .get_nth_param(param_offset)
                                    .unwrap()
                                    .into_pointer_value();
                                *self.error_ptr.borrow_mut() = Some(err_ptr);
                                param_offset += 1;
                            }
                            for (i, param) in parameters.iter().enumerate() {
                                if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                    continue;
                                }
                                let param_name = &param.borrow().name.value;
                                let llvm_type =
                                    self.resolve_type(param.borrow().ty.clone().unwrap())?;
                                let alloca_name = self.unique_alloca_name(param_name);
                                let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                                let param_value =
                                    function.get_nth_param(i as u32 + param_offset).unwrap();
                                self.builder.build_store(ptr, param_value)?;
                                self.declare_variable(param_name.clone(), ptr);
                            }
                        }
                    } else {
                        let mut param_offset = 0u32;
                        if is_throwing {
                            let err_ptr = function
                                .get_nth_param(param_offset)
                                .unwrap()
                                .into_pointer_value();
                            *self.error_ptr.borrow_mut() = Some(err_ptr);
                            param_offset += 1;
                        }
                        for (i, param) in parameters.iter().enumerate() {
                            if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                continue;
                            }
                            let param_name = &param.borrow().name.value;
                            let llvm_type =
                                self.resolve_type(param.borrow().ty.clone().unwrap())?;
                            let alloca_name = self.unique_alloca_name(param_name);
                            let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                            let param_value =
                                function.get_nth_param(i as u32 + param_offset).unwrap();
                            self.builder.build_store(ptr, param_value)?;
                            self.declare_variable(param_name.clone(), ptr);
                        }
                    }

                    let is_void = matches!(&*return_type.borrow(), Type::Void);

                    match &*body.borrow() {
                        FunctionBody::Statements(stmts) => {
                            self.enter_scope_with_stmts(stmts)?;
                            let mut has_return = false;
                            let stmt_count = stmts.len();
                            for (i, stmt) in stmts.iter().enumerate() {
                                let is_last = i == stmt_count - 1;
                                if is_last && !is_void {
                                    if let Statement::ExpressionStatement { expression } =
                                        &*stmt.borrow()
                                    {
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
                            if has_return {
                                self.exit_scope();
                            } else {
                                self.emit_class_releases();
                            }
                            if is_void && !has_return {
                                self.exit_scope();
                                self.builder.build_return(None)?;
                            }
                        }
                        FunctionBody::Expression(expr) => {
                            let value = self.resolve_expression(expr.clone())?.unwrap();
                            self.emit_all_deinit_calls();
                            self.emit_class_releases();
                            self.builder.build_return(Some(&value))?;
                        }
                        FunctionBody::None => {
                            self.emit_class_releases();
                        }
                    }

                    if let Some(block) = current_block {
                        self.builder.position_at_end(block);
                    }
                    if let Some(sname) = saved_struct {
                        self.current_struct.borrow_mut().replace(sname);
                    }
                }
                Ok(false)
            }
            Statement::InitDecl {
                ty: Some(ty),
                parameters,
                body,
                is_failable,
                ..
            } => {
                if let Type::Function(_parameter_types, return_type, _, throws_types) =
                    &*ty.borrow()
                {
                    let is_throwing = throws_types.is_some();
                    let struct_name = self.current_struct.borrow().clone().unwrap();
                    let fn_name = self
                        .mangled_fn_names
                        .borrow()
                        .get(&format!("{}.init", struct_name))
                        .cloned()
                        .unwrap_or_else(|| format!("{}.init", struct_name));
                    let function = self.module.get_function(&fn_name).unwrap();

                    let current_block = self.builder.get_insert_block();
                    let entry_block = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry_block);

                    self.enter_scope();
                    let self_ptr = function.get_nth_param(0).unwrap();
                    let self_ptr = self_ptr.into_pointer_value();
                    self.declare_variable("self".to_string(), self_ptr);
                    let mut param_offset = 1u32;
                    if is_throwing {
                        let err_ptr = function
                            .get_nth_param(param_offset)
                            .unwrap()
                            .into_pointer_value();
                        *self.error_ptr.borrow_mut() = Some(err_ptr);
                        param_offset += 1;
                    }
                    for (i, param) in parameters.iter().enumerate() {
                        let param_name = &param.borrow().name.value;
                        let llvm_type = self.resolve_type(param.borrow().ty.clone().unwrap())?;
                        let alloca_name = self.unique_alloca_name(param_name);
                        let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                        let param_value = function.get_nth_param(i as u32 + param_offset).unwrap();
                        self.builder.build_store(ptr, param_value)?;
                        self.declare_variable(param_name.clone(), ptr);
                    }

                    let is_class_init = self.class_types.borrow().contains_key(&struct_name);
                    if !is_class_init {
                        self.auto_assign_init_fields(&struct_name, self_ptr, parameters);
                    }

                    let is_failable = *is_failable;
                    let optional_ret_ty = if is_failable {
                        Some(return_type.clone())
                    } else {
                        None
                    };

                    match &*body.borrow() {
                        FunctionBody::Statements(stmts) => {
                            self.enter_scope_with_stmts(stmts)?;
                            let mut has_return = false;
                            for stmt in stmts {
                                let terminates = self.resolve_statement(stmt.clone())?;
                                if terminates {
                                    has_return = true;
                                    break;
                                }
                            }
                            if !has_return {
                                if let Some(ret_ty) = optional_ret_ty {
                                    self.build_optional_some_return(self_ptr, ret_ty)?;
                                } else {
                                    self.builder.build_return(None)?;
                                }
                            }
                            self.exit_scope();
                        }
                        FunctionBody::Expression(expr) => {
                            self.resolve_expression(expr.clone())?;
                            if let Some(ret_ty) = optional_ret_ty {
                                self.build_optional_some_return(self_ptr, ret_ty)?;
                            } else {
                                self.builder.build_return(None)?;
                            }
                        }
                        FunctionBody::None => {
                            if let Some(ret_ty) = optional_ret_ty {
                                self.build_optional_some_return(self_ptr, ret_ty)?;
                            } else {
                                self.builder.build_return(None)?;
                            }
                        }
                    }

                    if let Some(block) = current_block {
                        self.builder.position_at_end(block);
                    }
                }
                Ok(false)
            }
            Statement::DeinitDecl {
                ty: Some(ty), body, ..
            } => {
                if let Type::Function(_, _return_type, _, None) = &*ty.borrow() {
                    let struct_name = self.current_struct.borrow().clone().unwrap();
                    let fn_name = self.mangle_fn_name(&format!("{}.deinit", struct_name), &[]);
                    let function = self.module.get_function(&fn_name).unwrap();

                    let current_block = self.builder.get_insert_block();
                    let entry_block = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry_block);

                    self.enter_scope();
                    let self_ptr = function.get_nth_param(0).unwrap();
                    let self_ptr = self_ptr.into_pointer_value();
                    self.declare_variable("self".to_string(), self_ptr);

                    match &*body.borrow() {
                        FunctionBody::Statements(stmts) => {
                            self.enter_scope_with_stmts(stmts)?;
                            for stmt in stmts {
                                let terminates = self.resolve_statement(stmt.clone())?;
                                if terminates {
                                    break;
                                }
                            }
                            self.builder.build_return(None)?;
                            self.exit_scope();
                        }
                        FunctionBody::Expression(expr) => {
                            self.resolve_expression(expr.clone())?;
                            self.builder.build_return(None)?;
                        }
                        FunctionBody::None => {
                            self.builder.build_return(None)?;
                        }
                    }

                    if let Some(block) = current_block {
                        self.builder.position_at_end(block);
                    }
                }
                Ok(false)
            }
            Statement::Return { value, .. } => {
                match value {
                    Some(value) if !matches!(&*value.borrow(), Expression::VoidLiteral { .. }) => {
                        let val = self.resolve_expression(value.clone())?.unwrap();
                        self.emit_all_deinit_calls();
                        self.emit_class_releases();
                        self.builder.build_return(Some(&val))?;
                    }
                    _ => {
                        self.emit_all_deinit_calls();
                        self.emit_class_releases();
                        self.builder.build_return(None)?;
                    }
                }
                Ok(true)
            }
            Statement::Throw { exception, .. } => {
                let val = self.resolve_expression(exception.clone())?.unwrap();
                self.emit_all_deinit_calls();
                if let Some(err_ptr) = *self.error_ptr.borrow() {
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                    let err_obj_ptr = match val {
                        BasicValueEnum::PointerValue(p) => {
                            self.builder.build_pointer_cast(p, ptr_ty, "")?
                        }
                        _ => {
                            let alloca = self.builder.build_alloca(val.get_type(), "")?;
                            self.builder.build_store(alloca, val)?;
                            self.builder.build_pointer_cast(alloca, ptr_ty, "")?
                        }
                    };
                    self.builder.build_store(err_ptr, err_obj_ptr)?;
                }
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let ret_type = fn_val.get_type().get_return_type();
                if let Some(ret_ty) = ret_type {
                    let zero_val = ret_ty.const_zero();
                    self.builder.build_return(Some(&zero_val))?;
                } else {
                    self.builder.build_return(None)?;
                }
                Ok(true)
            }
            Statement::Yield { value, .. } => {
                if self.yield_targets.borrow().is_empty() {
                    match value {
                        Some(value)
                            if !matches!(&*value.borrow(), Expression::VoidLiteral { .. }) =>
                        {
                            let value = self.resolve_expression(value.clone())?.unwrap();
                            self.emit_all_deinit_calls();
                            self.builder.build_return(Some(&value))?;
                        }
                        _ => {
                            self.emit_all_deinit_calls();
                            self.builder.build_return(None)?;
                        }
                    }
                } else {
                    let target = self.yield_targets.borrow().last().copied().unwrap();
                    let (result_alloca, exit_bb) = target;
                    match value {
                        Some(value)
                            if !matches!(&*value.borrow(), Expression::VoidLiteral { .. }) =>
                        {
                            let value = self.resolve_expression(value.clone())?.unwrap();
                            self.builder.build_store(result_alloca, value)?;
                        }
                        _ => {}
                    }
                    self.emit_all_deinit_calls();
                    self.builder.build_unconditional_branch(exit_bb)?;
                }
                Ok(true)
            }
            Statement::ExpressionStatement { expression } => {
                self.resolve_expression(expression.clone())?;
                Ok(false)
            }
            Statement::ExternBlock { items, .. } => {
                for item in items {
                    if let Statement::FunctionDecl { .. } = &*item.borrow() {
                        continue;
                    }
                    let _ = self.resolve_statement(item.clone());
                }
                Ok(false)
            }
            Statement::ExternDecl { .. } => Ok(false),
            Statement::StructDecl { name, body, .. } => {
                let prev = self.current_struct.borrow_mut().take();
                self.current_struct.borrow_mut().replace(name.value.clone());
                let result = (|| -> Result<bool> {
                    for stmt in body {
                        self.resolve_statement(stmt.clone())?;
                    }
                    Ok(false)
                })();
                *self.current_struct.borrow_mut() = prev;
                result
            }
            Statement::ClassDecl { name, body, .. } => {
                let prev = self.current_struct.borrow_mut().take();
                self.current_struct.borrow_mut().replace(name.value.clone());
                let result = (|| -> Result<bool> {
                    for stmt in body {
                        self.resolve_statement(stmt.clone())?;
                    }
                    Ok(false)
                })();
                *self.current_struct.borrow_mut() = prev;
                result
            }
            Statement::EnumDecl { name, body, .. } => {
                let prev = self.current_struct.borrow_mut().take();
                self.current_struct.borrow_mut().replace(name.value.clone());
                let result = (|| -> Result<bool> {
                    for stmt in body {
                        self.resolve_statement(stmt.clone())?;
                    }
                    Ok(false)
                })();
                *self.current_struct.borrow_mut() = prev;
                result
            }
            Statement::ProtocolDecl { name, members, .. } => {
                let prev = self.current_struct.borrow_mut().take();
                self.current_struct.borrow_mut().replace(name.value.clone());
                let result = (|| -> Result<bool> {
                    for member in members {
                        if let ProtocolMember::Method { decl, .. } = member
                            && let Statement::FunctionDecl { body, .. } = &*decl.borrow()
                            && !matches!(&*body.borrow(), FunctionBody::None)
                        {
                            self.resolve_statement(decl.clone())?;
                        }
                    }
                    Ok(false)
                })();
                *self.current_struct.borrow_mut() = prev;
                result
            }
            Statement::ExtensionDecl {
                type_name,
                body,
                type_arguments,
                ..
            } => {
                let prev = self.current_struct.borrow_mut().take();
                let struct_name = if type_arguments.is_some()
                    && !self
                        .type_args_to_abbreviation(type_arguments.as_ref())
                        .is_empty()
                {
                    format!(
                        "{}.{}",
                        type_name.value,
                        self.type_args_to_abbreviation(type_arguments.as_ref())
                    )
                } else {
                    type_name.value.clone()
                };
                self.current_struct.borrow_mut().replace(struct_name);
                let result = (|| -> Result<bool> {
                    for stmt in body {
                        self.resolve_statement(stmt.clone())?;
                    }
                    Ok(false)
                })();
                *self.current_struct.borrow_mut() = prev;
                result
            }
            Statement::Guard {
                condition,
                else_body,
                ..
            } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                if let Expression::Case {
                    enum_type,
                    case_name,
                    bindings: _,
                    expression,
                    ..
                } = &*condition.borrow()
                {
                    let enum_name = if let Some(name) = enum_type.as_ref().map(|t| t.value.as_str())
                    {
                        name.to_string()
                    } else if let Some(name) =
                        self.get_enum_name_from_expr_string(expression.clone())
                    {
                        name
                    } else {
                        String::new()
                    };
                    let check_bb = self.context.append_basic_block(fn_val, "guard_check");
                    let else_bb = self.context.append_basic_block(fn_val, "guard_else");
                    let exit_bb = self.context.append_basic_block(fn_val, "guard_exit");

                    self.builder.build_unconditional_branch(check_bb)?;
                    self.builder.position_at_end(check_bb);

                    let subject_val = self.resolve_expression(expression.clone())?.unwrap();
                    let subject_alloca = self.builder.build_alloca(subject_val.get_type(), "")?;
                    self.builder.build_store(subject_alloca, subject_val)?;

                    let case_idx = self.get_enum_case_index(&enum_name, &case_name.value)?;

                    let enum_types = self.enum_types.borrow();
                    let enum_llvm_type = enum_types
                        .get(&enum_name)
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
                    drop(enum_types);

                    let (match_result, exit_bb) = if let Some(raw_llvm_type) =
                        self.get_enum_raw_llvm_type(&enum_name)
                    {
                        let raw_ptr =
                            self.builder
                                .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                        let tag_val = self.builder.build_load(raw_llvm_type, raw_ptr, "")?;
                        let expected_tag = raw_llvm_type
                            .into_int_type()
                            .const_int(case_idx as u64, false);
                        let match_result = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            tag_val.into_int_value(),
                            expected_tag,
                            "",
                        )?;
                        (match_result, exit_bb)
                    } else {
                        let tag_ptr =
                            self.builder
                                .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                        let tag_val =
                            self.builder
                                .build_load(self.context.i8_type(), tag_ptr, "")?;
                        let expected_tag = self.context.i8_type().const_int(case_idx as u64, false);
                        let match_result = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            tag_val.into_int_value(),
                            expected_tag,
                            "",
                        )?;
                        (match_result, exit_bb)
                    };

                    self.builder
                        .build_conditional_branch(match_result, exit_bb, else_bb)?;

                    self.builder.position_at_end(else_bb);
                    self.resolve_block_expression(else_body)?;
                    self.builder.build_unconditional_branch(exit_bb)?;

                    self.builder.position_at_end(exit_bb);
                }
                Ok(false)
            }
            Statement::Fallthrough { .. } => {
                anyhow::bail!("fallthrough outside of match is not supported");
            }
            Statement::Break { .. } => {
                let targets = self.loop_break_targets.borrow();
                if let Some(exit_bb) = targets.last() {
                    self.builder.build_unconditional_branch(*exit_bb)?;
                    Ok(true)
                } else {
                    anyhow::bail!("break outside of loop is not supported");
                }
            }
            Statement::Continue { .. } => {
                let targets = self.loop_continue_targets.borrow();
                if let Some(cont_bb) = targets.last() {
                    self.builder.build_unconditional_branch(*cont_bb)?;
                    Ok(true)
                } else {
                    anyhow::bail!("continue outside of loop is not supported");
                }
            }
            Statement::Defer { body, .. } => {
                self.scope_stack
                    .borrow_mut()
                    .last_mut()
                    .unwrap()
                    .deferred_blocks
                    .push(body.clone());
                Ok(false)
            }
            Statement::ModuleDecl { body, .. } => {
                for stmt in body {
                    self.resolve_statement(stmt.clone())?;
                }
                Ok(false)
            }
            Statement::MacroDecl { .. } => Ok(false),
            Statement::ConditionalBlock { clauses } => {
                for clause in clauses {
                    for stmt in &clause.body {
                        self.resolve_statement(stmt.clone())?;
                    }
                }
                Ok(false)
            }
            Statement::PragmaError { .. } | Statement::PragmaWarning { .. } => Ok(false),
            Statement::AsmBlock {
                instructions,
                outputs,
                inputs,
                clobbers,
                ..
            } => self.resolve_asm_block(instructions, outputs, inputs, clobbers),
            _ => Ok(false),
        }
    }

    fn resolve_tuple_pattern_decl(
        &self,
        items: &[Pattern],
        initializer: &Option<Rc<RefCell<Expression>>>,
        ty: &Option<Rc<RefCell<Type>>>,
    ) -> Result<bool> {
        let Some(init) = initializer else {
            return Err(anyhow::anyhow!("Tuple pattern needs initializer"));
        };
        let Some(init_val) = self.resolve_expression(init.clone())? else {
            return Err(anyhow::anyhow!("Cannot resolve tuple initializer"));
        };
        let tuple_type = ty
            .as_ref()
            .map(|t| t.clone())
            .or_else(|| init.borrow().get_ty_ref().ok().and_then(|t| t.clone()));
        let Some(tuple_type) = tuple_type else {
            return Err(anyhow::anyhow!("Cannot determine tuple type"));
        };
        let elements = match &*tuple_type.borrow() {
            Type::Tuple(e) => e.clone(),
            _ => return Err(anyhow::anyhow!("Expected tuple type for pattern")),
        };
        let llvm_tuple_ty = self.resolve_type(tuple_type)?.into_struct_type();
        let struct_ptr = match init_val {
            BasicValueEnum::PointerValue(ptr) => ptr,
            val => {
                let ptr = self.builder.build_alloca(val.get_type(), "")?;
                self.builder.build_store(ptr, val)?;
                ptr
            }
        };
        for (i, item) in items.iter().enumerate() {
            if i >= elements.len() {
                break;
            }
            self.resolve_tuple_pattern_item(item, struct_ptr, llvm_tuple_ty, i)?;
        }
        Ok(false)
    }

    fn resolve_tuple_pattern_item(
        &self,
        pattern: &Pattern,
        struct_ptr: PointerValue<'ctx>,
        tuple_llvm: StructType<'ctx>,
        index: usize,
    ) -> Result<()> {
        match pattern {
            Pattern::Identifier(tok) => {
                if tok.value == "_" {
                    return Ok(());
                }
                let field_ptr =
                    self.builder
                        .build_struct_gep(tuple_llvm, struct_ptr, index as u32, "")?;
                let field_ty = tuple_llvm
                    .get_field_type_at_index(index as u32)
                    .ok_or_else(|| anyhow::anyhow!("Field not found at idx {}", index))?;
                let field_val = self.builder.build_load(field_ty, field_ptr, "")?;
                let var_ptr = self.builder.build_alloca(field_ty, &tok.value)?;
                self.builder.build_store(var_ptr, field_val)?;
                self.declare_variable(tok.value.clone(), var_ptr);
            }
            Pattern::ValueBinding(inner) => {
                if let Pattern::Identifier(tok) = inner.as_ref() {
                    if tok.value == "_" {
                        return Ok(());
                    }
                    let field_ptr =
                        self.builder
                            .build_struct_gep(tuple_llvm, struct_ptr, index as u32, "")?;
                    let field_ty = tuple_llvm
                        .get_field_type_at_index(index as u32)
                        .ok_or_else(|| anyhow::anyhow!("Field not found at idx {}", index))?;
                    let field_val = self.builder.build_load(field_ty, field_ptr, "")?;
                    let var_ptr = self.builder.build_alloca(field_ty, &tok.value)?;
                    self.builder.build_store(var_ptr, field_val)?;
                    self.declare_variable(tok.value.clone(), var_ptr);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn match_tuple_elements(
        &self,
        items: &[Pattern],
        subject_alloca: PointerValue<'ctx>,
        subject_expr: Rc<RefCell<Expression>>,
    ) -> Result<Option<IntValue<'_>>> {
        let subject_ty = subject_expr
            .borrow()
            .get_ty_ref()?
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine match subject type"))?;
        let elements = match &*subject_ty.borrow() {
            Type::Tuple(e) => e.clone(),
            _ => return Err(anyhow::anyhow!("Expected tuple type for match pattern")),
        };
        let llvm_tuple_ty = self.resolve_type(subject_ty)?.into_struct_type();
        let fn_val = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        for (i, item) in items.iter().enumerate() {
            if i >= elements.len() {
                break;
            }
            if let Pattern::Expr(expr) = item {
                let element_ptr =
                    self.builder
                        .build_struct_gep(llvm_tuple_ty, subject_alloca, i as u32, "")?;
                let element_ty = llvm_tuple_ty
                    .get_field_type_at_index(i as u32)
                    .ok_or_else(|| anyhow::anyhow!("Field not found at idx {}", i))?;
                let element_val = self.builder.build_load(element_ty, element_ptr, "")?;
                let next_bb = self.context.append_basic_block(fn_val, "tuple_elem_check");
                if let Some(cmp) = self.get_literal_match(element_val, expr.clone())? {
                    let body_bb = self.builder.get_insert_block().unwrap();
                    self.builder
                        .build_conditional_branch(cmp, body_bb, next_bb)?;
                    self.builder.position_at_end(next_bb);
                }
            }
        }
        Ok(None)
    }

    fn resolve_match_tuple_bindings(
        &self,
        items: &[Pattern],
        subject_alloca: PointerValue<'ctx>,
    ) -> Result<()> {
        let loaded = self
            .builder
            .build_load(subject_alloca.get_type(), subject_alloca, "")?;
        let basic_ty = loaded.get_type();
        let struct_ty = basic_ty.into_struct_type();
        for (i, item) in items.iter().enumerate() {
            self.resolve_tuple_pattern_item(item, subject_alloca, struct_ty, i)?;
        }
        Ok(())
    }

    fn resolve_asm_block(
        &self,
        instructions: &[Token],
        outputs: &[AsmOperand],
        inputs: &[AsmOperand],
        clobbers: &[Token],
    ) -> Result<bool> {
        let asm_body: Vec<String> = instructions
            .iter()
            .map(|t| t.value.trim_matches('"').to_string())
            .collect();
        let asm_str = asm_body.join("\n");
        let mut label_to_idx: HashMap<&str, u32> = HashMap::new();
        for (i, op) in outputs.iter().enumerate() {
            label_to_idx.insert(&op.label.value, i as u32);
        }
        for (i, op) in inputs.iter().enumerate() {
            label_to_idx.insert(&op.label.value, (outputs.len() + i) as u32);
        }
        let mut final_asm = String::new();
        let mut chars = asm_str.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                let mut label = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next();
                        break;
                    }
                    label.push(next);
                    chars.next();
                }
                if let Some(idx) = label_to_idx.get(label.as_str()) {
                    final_asm.push_str(&format!("${}", idx));
                } else {
                    final_asm.push('{');
                    final_asm.push_str(&label);
                    final_asm.push('}');
                }
            } else {
                final_asm.push(c);
            }
        }
        let mut constraints = String::new();
        let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
        let mut param_values: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
        for op in outputs {
            if !constraints.is_empty() {
                constraints.push(',');
            }
            let prefix = "=";
            match op.constraint.value.as_str() {
                "reg" => constraints.push_str(&format!("{}r", prefix)),
                "imm" => constraints.push_str(&format!("{}i", prefix)),
                "mem" => constraints.push_str(&format!("{}m", prefix)),
                other => constraints.push_str(&format!("{}{}", prefix, other)),
            }
            let expr_type = match &*op.expression.borrow() {
                Expression::Variable { ty, .. }
                | Expression::IntegerLiteral { ty, .. }
                | Expression::DecimalLiteral { ty, .. }
                | Expression::NullptrLiteral { ty, .. }
                | Expression::MemberAccess { ty, .. }
                | Expression::AssociatedTypeAccess { ty, .. }
                | Expression::If { ty, .. }
                | Expression::Case { ty, .. }
                | Expression::Match { ty, .. }
                | Expression::ShorthandArgument { ty, .. }
                | Expression::Cast { ty, .. }
                | Expression::TupleLiteral { ty, .. }
                | Expression::ArrayLiteral { ty, .. }
                | Expression::TupleIndexAccess { ty, .. }
                | Expression::SelfKeyword { ty, .. }
                | Expression::SuperKeyword { ty, .. }
                | Expression::SelfType { ty, .. }
                | Expression::AnyType { ty, .. }
                | Expression::SomeType { ty, .. }
                | Expression::CompoundType { ty, .. }
                | Expression::Closure { ty, .. }
                | Expression::ClosureType { ty, .. }
                | Expression::FunctionType { ty, .. }
                | Expression::SubscriptAccess { ty, .. }
                | Expression::MacroInvocation { ty, .. }
                | Expression::SizeOf { ty, .. }
                | Expression::PointerType { ty, .. }
                | Expression::Type { ty, .. }
                | Expression::Do { ty, .. }
                | Expression::OptionalType { ty, .. }
                | Expression::ArrayType { ty, .. } => ty.clone(),
                _ => None,
            };
            let ty = self.resolve_type(
                expr_type.ok_or_else(|| anyhow::anyhow!("Missing type on asm output operand"))?,
            )?;
            param_types.push(ty.as_basic_type_enum().into());
        }
        for op in inputs {
            if !constraints.is_empty() {
                constraints.push(',');
            }
            match op.constraint.value.as_str() {
                "reg" => constraints.push('r'),
                "imm" => constraints.push('i'),
                "mem" => constraints.push('m'),
                other => constraints.push_str(other),
            }
            if let Some(val) = self.resolve_expression(op.expression.clone())? {
                param_types.push(val.get_type().into());
                param_values.push(val.into());
            }
        }
        for clobber in clobbers {
            let name = clobber.value.trim_matches('"');
            if !constraints.is_empty() {
                constraints.push(',');
            }
            constraints.push_str(&format!("~{{{}}}", name));
        }
        let fn_type: FunctionType<'ctx> = if outputs.is_empty() {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            let out_expr = &*outputs[0].expression.borrow();
            let out_expr_type = match out_expr {
                Expression::Variable { ty, .. }
                | Expression::IntegerLiteral { ty, .. }
                | Expression::DecimalLiteral { ty, .. }
                | Expression::NullptrLiteral { ty, .. }
                | Expression::MemberAccess { ty, .. }
                | Expression::AssociatedTypeAccess { ty, .. }
                | Expression::If { ty, .. }
                | Expression::Case { ty, .. }
                | Expression::Match { ty, .. }
                | Expression::ShorthandArgument { ty, .. }
                | Expression::Cast { ty, .. }
                | Expression::TupleLiteral { ty, .. }
                | Expression::ArrayLiteral { ty, .. }
                | Expression::TupleIndexAccess { ty, .. }
                | Expression::SelfKeyword { ty, .. }
                | Expression::SuperKeyword { ty, .. }
                | Expression::SelfType { ty, .. }
                | Expression::AnyType { ty, .. }
                | Expression::SomeType { ty, .. }
                | Expression::CompoundType { ty, .. }
                | Expression::Closure { ty, .. }
                | Expression::ClosureType { ty, .. }
                | Expression::FunctionType { ty, .. }
                | Expression::SubscriptAccess { ty, .. }
                | Expression::MacroInvocation { ty, .. }
                | Expression::SizeOf { ty, .. }
                | Expression::PointerType { ty, .. }
                | Expression::Type { ty, .. }
                | Expression::Do { ty, .. }
                | Expression::OptionalType { ty, .. }
                | Expression::ArrayType { ty, .. } => ty.clone(),
                _ => None,
            };
            let out_ty = self.resolve_type(
                out_expr_type
                    .ok_or_else(|| anyhow::anyhow!("Missing type on asm output operand"))?,
            )?;
            out_ty.fn_type(&param_types, false)
        };
        let asm_ptr = self.context.create_inline_asm(
            fn_type,
            final_asm,
            constraints,
            true,
            false,
            None,
            false,
        );
        let result = self
            .builder
            .build_indirect_call(fn_type, asm_ptr, &param_values, "asm")?;
        if !outputs.is_empty() {
            let result_val = match result.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(val) => val,
                _ => anyhow::bail!("Inline asm did not return a value"),
            };
            if let Expression::Variable { name, .. } = &*outputs[0].expression.borrow() {
                if let Some(ptr) = self.lookup_variable(&name.value) {
                    self.builder.build_store(ptr, result_val)?;
                }
            }
        }
        Ok(false)
    }

    fn resolve_expression(
        &self,
        expr: Rc<RefCell<Expression>>,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        match &*expr.borrow() {
            Expression::IntegerLiteral { value, ty, .. } => {
                let llvm_type = match ty {
                    Some(t) => self.resolve_type(t.clone())?,
                    None => self.context.i32_type().into(),
                };
                let is_optional = matches!(ty, Some(t) if matches!(&*t.borrow(), Type::Enum(name, ..) if name == "Optional"));
                if is_optional {
                    let t_ref = ty.as_ref().unwrap();
                    let enum_name = match &*t_ref.borrow() {
                        Type::Enum(name, ..) => name.clone(),
                        _ => unreachable!(),
                    };
                    let enum_type = self
                        .enum_types
                        .borrow()
                        .get(&enum_name)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
                    let payloads_type = self
                        .enum_payload_types
                        .borrow()
                        .get(&enum_name)
                        .cloned()
                        .ok_or_else(|| {
                        anyhow::anyhow!("Enum payload type '{}' not found", enum_name)
                    })?;
                    let int_val = self.context.i32_type().const_int(*value as u64, false);
                    let int_alloca = self.builder.build_alloca(self.context.i32_type(), "")?;
                    self.builder.build_store(int_alloca, int_val)?;
                    let loaded =
                        self.builder
                            .build_load(self.context.i32_type(), int_alloca, "")?;
                    let payload_alloca = self
                        .builder
                        .build_alloca(payloads_type.as_basic_type_enum(), "")?;
                    if payloads_type.get_field_type_at_index(1).is_none() {
                        anyhow::bail!("Some payload slot not found");
                    }
                    let payload_ptr =
                        self.builder
                            .build_struct_gep(payloads_type, payload_alloca, 1, "")?;
                    self.builder.build_store(payload_ptr, loaded)?;
                    let payload_val = self.builder.build_load(
                        payloads_type.as_basic_type_enum(),
                        payload_alloca,
                        "",
                    )?;
                    let enum_alloca = self
                        .builder
                        .build_alloca(enum_type.as_basic_type_enum(), "")?;
                    let tag_ptr = self
                        .builder
                        .build_struct_gep(enum_type, enum_alloca, 0, "")?;
                    self.builder
                        .build_store(tag_ptr, self.context.i8_type().const_int(1, false))?;
                    let payload_store_ptr =
                        self.builder
                            .build_struct_gep(enum_type, enum_alloca, 1, "")?;
                    self.builder.build_store(payload_store_ptr, payload_val)?;
                    let result =
                        self.builder
                            .build_load(enum_type.as_basic_type_enum(), enum_alloca, "")?;
                    Ok(Some(result))
                } else {
                    Ok(Some(
                        llvm_type
                            .into_int_type()
                            .const_int(*value as u64, false)
                            .into(),
                    ))
                }
            }
            Expression::BooleanLiteral { token } => {
                let value = match &token.ty {
                    TokenType::BooleanLiteral { value } => *value,
                    _ => false,
                };
                Ok(Some(
                    self.context
                        .bool_type()
                        .const_int(value as u64, false)
                        .into(),
                ))
            }
            Expression::DecimalLiteral { value, ty, .. } => {
                let llvm_type = match ty {
                    Some(t) => self.resolve_type(t.clone())?,
                    None => self.context.f64_type().into(),
                };
                Ok(Some(llvm_type.into_float_type().const_float(*value).into()))
            }
            Expression::CharLiteral { .. } => {
                Ok(Some(self.context.i8_type().const_int(0, false).into()))
            }
            Expression::NullLiteral { ty, .. } => {
                if let Some(t) = ty.as_ref()
                    && let Type::Enum(name, ..) = &*t.borrow()
                {
                    let enum_type =
                        self.enum_types.borrow().get(name).cloned().ok_or_else(|| {
                            anyhow::anyhow!("Enum type '{}' not found in IR generation", name)
                        })?;
                    let payloads_type = self
                        .enum_payload_types
                        .borrow()
                        .get(name)
                        .cloned()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Enum payload type '{}' not found in IR generation",
                                name
                            )
                        })?;
                    let none_payload_type = payloads_type
                        .get_field_type_at_index(0)
                        .ok_or_else(|| anyhow::anyhow!("None payload not found"))?;
                    let none_payload = none_payload_type.const_zero();
                    let tag = self.context.i8_type().const_zero();
                    let enum_val = enum_type.const_named_struct(&[tag.into(), none_payload.into()]);
                    Ok(Some(enum_val.into()))
                } else {
                    Ok(Some(
                        self.context
                            .ptr_type(inkwell::AddressSpace::from(0))
                            .const_null()
                            .into(),
                    ))
                }
            }
            Expression::StringLiteral { value, .. } => {
                let string_data = unsafe { self.builder.build_global_string(value, ".str")? };
                let raw_src = string_data.as_pointer_value();
                let i8_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                let i64_ty = self.context.i64_type();
                let len = value.len() as u64;

                if let Some(str_type) = self.class_types.borrow().get("String").copied() {
                    let null_i8 =
                        self.builder
                            .build_int_to_ptr(i64_ty.const_int(0, false), i8_ty, "")?;
                    let size_val = unsafe {
                        let gep = self.builder.build_gep(
                            str_type.as_basic_type_enum(),
                            null_i8,
                            &[i64_ty.const_int(1, false)],
                            "",
                        )?;
                        self.builder.build_ptr_to_int(gep, i64_ty, "")?
                    };
                    let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                        let fn_ty = i8_ty.fn_type(&[i64_ty.into()], false);
                        self.module.add_function("malloc", fn_ty, None)
                    });
                    let malloc_class =
                        self.builder.build_call(malloc_fn, &[size_val.into()], "")?;
                    let class_ptr = match malloc_class.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(
                            inkwell::values::BasicValueEnum::PointerValue(p),
                        ) => p,
                        _ => anyhow::bail!("malloc expected to return pointer"),
                    };

                    let rc_ptr = self.builder.build_struct_gep(str_type, class_ptr, 1, "")?;
                    self.builder
                        .build_store(rc_ptr, i64_ty.const_int(1, false))?;

                    let init_fn = self.module.get_function("String.init").unwrap_or_else(|| {
                        let fn_ty =
                            i8_ty.fn_type(&[i8_ty.into(), i8_ty.into(), i64_ty.into()], false);
                        self.module.add_function("String.init", fn_ty, None)
                    });

                    let raw_src_i8 = self.builder.build_pointer_cast(raw_src, i8_ty, "")?;
                    self.builder.build_call(
                        init_fn,
                        &[
                            class_ptr.into(),
                            raw_src_i8.into(),
                            i64_ty.const_int(len + 1, false).into(),
                        ],
                        "",
                    )?;

                    Ok(Some(class_ptr.into()))
                } else {
                    let bitcast = self.builder.build_pointer_cast(raw_src, i8_ty, "")?;
                    Ok(Some(bitcast.into()))
                }
            }
            Expression::ArrayLiteral { elements, .. } => {
                for element in elements {
                    self.resolve_expression(element.clone())?;
                }
                Ok(Some(
                    self.context
                        .ptr_type(inkwell::AddressSpace::from(0))
                        .const_null()
                        .into(),
                ))
            }
            Expression::NullptrLiteral { .. } => Ok(Some(
                self.context
                    .ptr_type(inkwell::AddressSpace::from(0))
                    .const_null()
                    .into(),
            )),
            Expression::SizeOf { argument, .. } => {
                let arg_ty = argument
                    .borrow()
                    .get_ty_ref()
                    .ok()
                    .and_then(|t| t.clone())
                    .ok_or_else(|| {
                        self.emit_error(
                            TrussDiagnosticCode::TypeInferenceFailed,
                            "Cannot determine type for sizeof".to_string(),
                            None,
                        );
                        anyhow::anyhow!("Cannot determine type for sizeof")
                    })?;
                let llvm_type = self.resolve_type(arg_ty)?;
                let i64_ty = self.context.i64_type();
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                let null_ptr =
                    self.builder
                        .build_int_to_ptr(i64_ty.const_int(0, false), ptr_ty, "")?;
                let size_val = unsafe {
                    let gep = self.builder.build_gep(
                        llvm_type,
                        null_ptr,
                        &[i64_ty.const_int(1, false)],
                        "",
                    )?;
                    self.builder.build_ptr_to_int(gep, i64_ty, "")?
                };
                Ok(Some(size_val.into()))
            }
            Expression::Variable { name, ty, .. } => {
                let getter_name = self.mangle_fn_name(&format!("{}.getter", name.value), &[]);
                if let Some(getter_fn) = self.module.get_function(&getter_name) {
                    if let Some(ptr) = self.lookup_variable(&name.value) {
                        let args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                            vec![ptr.into()];
                        let result = self.builder.build_call(getter_fn, &args, "")?;
                        match result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(val) => Ok(Some(val)),
                            _ => Ok(None),
                        }
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::UndefinedVariable,
                            format!("Undefined variable: '{}'", name.value),
                            Some(name),
                        );
                        anyhow::bail!("Undefined variable: {}", name.value);
                    }
                } else if let Some(ptr) = self.lookup_variable(&name.value) {
                    let llvm_type = if let Some(ty) = ty {
                        self.resolve_type(ty.clone())?
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::TypeInferenceFailed,
                            "Cannot infer type from variable without type annotation",
                            Some(name),
                        );
                        anyhow::bail!("Cannot infer type from variable")
                    };
                    let val = self.builder.build_load(llvm_type, ptr, "")?;
                    Ok(Some(val))
                } else if let Some((sname, struct_ptr)) = &*self.current_accessor_struct.borrow()
                    && let Ok(idx) = self.get_stored_struct_field_index(sname, &name.value)
                    && let Some(stype) = self.struct_types.borrow().get(sname).copied()
                    && let Ok(field_ptr) =
                        self.builder
                            .build_struct_gep(stype, *struct_ptr, idx as u32, "")
                {
                    self.declare_variable(name.value.clone(), field_ptr);
                    let llvm_type = if let Some(ty) = ty {
                        self.resolve_type(ty.clone())?
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::TypeInferenceFailed,
                            "Cannot infer type from variable without type annotation",
                            Some(name),
                        );
                        anyhow::bail!("Cannot infer type from variable")
                    };
                    let val = self.builder.build_load(llvm_type, field_ptr, "")?;
                    Ok(Some(val))
                } else if let Some(fn_val) = self.module.get_function(&name.value).or_else(|| {
                    self.mangled_fn_names
                        .borrow()
                        .get(&name.value)
                        .and_then(|mangled| self.module.get_function(mangled))
                }) {
                    let fn_ptr = fn_val.as_global_value().as_pointer_value();
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                    Ok(Some(
                        self.builder.build_bit_cast(fn_ptr, ptr_ty, "")?.into(),
                    ))
                } else if let Some(struct_type) =
                    self.struct_types.borrow().get(&name.value).copied()
                {
                    let zero = struct_type.const_zero();
                    let ptr = self.builder.build_alloca(struct_type, "")?;
                    self.builder.build_store(ptr, zero)?;
                    Ok(Some(self.builder.build_load(struct_type, ptr, "")?))
                } else if let Some(enum_type) = self.enum_types.borrow().get(&name.value).copied() {
                    let zero = enum_type.const_zero();
                    let ptr = self.builder.build_alloca(enum_type, "")?;
                    self.builder.build_store(ptr, zero)?;
                    Ok(Some(self.builder.build_load(enum_type, ptr, "")?))
                } else if self.class_types.borrow().contains_key(&name.value) {
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                    Ok(Some(ptr_ty.const_null().into()))
                } else if let Some(self_ptr) = self.lookup_variable("self") {
                    let struct_names: Vec<String> =
                        self.struct_types.borrow().keys().cloned().collect();
                    let mut found_sname = None;
                    for n in &struct_names {
                        if self.get_stored_struct_field_index(n, &name.value).is_ok() {
                            found_sname = Some(n.clone());
                            break;
                        }
                    }
                    if let Some(ref sname) = found_sname {
                        if let Ok(idx) = self.get_stored_struct_field_index(sname, &name.value) {
                            let stype = *self.struct_types.borrow().get(sname).unwrap();
                            let field_ptr = self
                                .builder
                                .build_struct_gep(stype, self_ptr, idx as u32, "")?;
                            self.declare_variable(name.value.clone(), field_ptr);
                            let field_ty = self.get_struct_field_type(sname, &name.value)?;
                            let val = self.builder.build_load(field_ty, field_ptr, "")?;
                            return Ok(Some(val));
                        }
                    }
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                    Ok(Some(ptr_ty.const_null().into()))
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        format!("Undefined variable: '{}'", name.value),
                        Some(name),
                    );
                    anyhow::bail!("Undefined variable: {}", name.value);
                }
            }
            Expression::SelfKeyword { ty, token, .. } => {
                if let Some(ptr) = self.lookup_variable("self") {
                    if let Some(ty) = ty {
                        if matches!(&*ty.borrow(), Type::Class(..)) {
                            return Ok(Some(ptr.into()));
                        }
                        if matches!(&*ty.borrow(), Type::Struct(..)) {
                            return Ok(Some(ptr.into()));
                        }
                        let llvm_type = self.resolve_type(ty.clone())?;
                        let val = self.builder.build_load(llvm_type, ptr, "")?;
                        Ok(Some(val))
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::TypeInferenceFailed,
                            "Cannot infer type for 'self'",
                            Some(token),
                        );
                        anyhow::bail!("Cannot infer type for 'self'");
                    }
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        "'self' is only available inside methods",
                        Some(token),
                    );
                    anyhow::bail!("'self' is only available inside methods");
                }
            }
            Expression::SuperKeyword { ty, token, .. } => {
                let self_ptr = self
                    .builder
                    .get_insert_block()
                    .and_then(|block| block.get_parent())
                    .and_then(|func| func.get_nth_param(0))
                    .map(|val| val.into_pointer_value());
                if let Some(ptr) = self_ptr {
                    let llvm_type = if let Some(ty) = ty {
                        self.resolve_type(ty.clone())?
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::TypeInferenceFailed,
                            "Cannot infer type for 'super'",
                            Some(token),
                        );
                        anyhow::bail!("Cannot infer type for 'super'");
                    };
                    let val = self.builder.build_load(llvm_type, ptr, "")?;
                    Ok(Some(val))
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        "'super' is only available inside class methods",
                        Some(token),
                    );
                    anyhow::bail!("'super' is only available inside class methods");
                }
            }
            Expression::Binary {
                left,
                operator,
                right,
                overloads,
                selected_index,
                ..
            } => {
                if let Some(idx) = *selected_index
                    && idx < overloads.len()
                {
                    let sym = &overloads[idx];
                    let op_name = operator.operator_name().to_string();
                    let is_static = sym.borrow().get_decl().ok().flatten().is_some_and(|d| {
                        if let Statement::FunctionDecl { static_method, .. } = &*d.borrow() {
                            *static_method
                        } else {
                            false
                        }
                    });

                    let params = vec![
                        CallParameter {
                            label: None,
                            expression: left.clone(),
                        },
                        CallParameter {
                            label: None,
                            expression: right.clone(),
                        },
                    ];
                    let fn_name = if is_static {
                        self.mangle_from_overload(&op_name, sym, &params)
                            .unwrap_or(op_name.clone())
                    } else {
                        let left_ty_name = {
                            let left_ty = left.borrow().get_ty().ok().flatten();
                            match left_ty.and_then(|t| {
                                let tb = t.borrow();
                                match &*tb {
                                    Type::Struct(n, ..)
                                    | Type::Class(n, ..)
                                    | Type::Enum(n, ..) => Some(n.clone()),
                                    _ => None,
                                }
                            }) {
                                Some(n) => n,
                                None => op_name.clone(),
                            }
                        };
                        let base = format!("{}.{}", left_ty_name, op_name);
                        self.mangle_from_overload(&base, sym, &params)
                            .unwrap_or(base)
                    };

                    if let Some(f) = self.module.get_function(&fn_name) {
                        let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                            Vec::new();
                        if is_static {
                            let left_val = self.resolve_expression(left.clone())?.unwrap();
                            args.push(left_val.into());
                        }
                        let right_val = self.resolve_expression(right.clone())?.unwrap();
                        args.push(right_val.into());
                        let call_result = self.builder.build_call(f, &args, "")?;
                        match call_result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(val) => return Ok(Some(val)),
                            _ => return Ok(None),
                        }
                    }
                }

                let left_val = self.resolve_expression(left.clone())?.unwrap();
                let right_val = self.resolve_expression(right.clone())?.unwrap();
                let left_ty = left.borrow().get_ty().ok().flatten();
                let is_unsigned = matches!(left_ty, Some(ref ty) if matches!(&*ty.borrow(), Type::Struct(name, _, _) if name == "UInt8" || name == "UInt16" || name == "UInt32" || name == "UInt64" || name == "UInt128"));

                match operator {
                    BinaryOperator::Plus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_int_add(l, r, "")?.into()))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_add(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for addition");
                        }
                    }
                    BinaryOperator::Minus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_int_sub(l, r, "")?.into()))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_sub(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for subtraction");
                        }
                    }
                    BinaryOperator::Multiply => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_int_mul(l, r, "")?.into()))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_mul(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for multiplication");
                        }
                    }
                    BinaryOperator::Divide => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            if is_unsigned {
                                Ok(Some(self.builder.build_int_unsigned_div(l, r, "")?.into()))
                            } else {
                                Ok(Some(self.builder.build_int_signed_div(l, r, "")?.into()))
                            }
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_div(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for division");
                        }
                    }
                    BinaryOperator::Equal => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_int_compare(inkwell::IntPredicate::EQ, l, r, "")?
                                    .into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OEQ, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for equality comparison");
                        }
                    }
                    BinaryOperator::NotEqual => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_int_compare(inkwell::IntPredicate::NE, l, r, "")?
                                    .into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::ONE, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for inequality comparison");
                        }
                    }
                    BinaryOperator::Less => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            let predicate = if is_unsigned {
                                inkwell::IntPredicate::ULT
                            } else {
                                inkwell::IntPredicate::SLT
                            };
                            Ok(Some(
                                self.builder.build_int_compare(predicate, l, r, "")?.into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OLT, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for less than comparison");
                        }
                    }
                    BinaryOperator::LessEqual => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            let predicate = if is_unsigned {
                                inkwell::IntPredicate::ULE
                            } else {
                                inkwell::IntPredicate::SLE
                            };
                            Ok(Some(
                                self.builder.build_int_compare(predicate, l, r, "")?.into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OLE, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for less than or equal comparison");
                        }
                    }
                    BinaryOperator::Greater => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            let predicate = if is_unsigned {
                                inkwell::IntPredicate::UGT
                            } else {
                                inkwell::IntPredicate::SGT
                            };
                            Ok(Some(
                                self.builder.build_int_compare(predicate, l, r, "")?.into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OGT, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for greater than comparison");
                        }
                    }
                    BinaryOperator::GreaterEqual => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            let predicate = if is_unsigned {
                                inkwell::IntPredicate::UGE
                            } else {
                                inkwell::IntPredicate::SGE
                            };
                            Ok(Some(
                                self.builder.build_int_compare(predicate, l, r, "")?.into(),
                            ))
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder
                                    .build_float_compare(inkwell::FloatPredicate::OGE, l, r, "")?
                                    .into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for greater than or equal comparison");
                        }
                    }
                    BinaryOperator::And => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_and(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for logical and");
                        }
                    }
                    BinaryOperator::Or => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_or(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for logical or");
                        }
                    }
                    BinaryOperator::BitAnd => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_and(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for bitwise and");
                        }
                    }
                    BinaryOperator::BitOr => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_or(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for bitwise or");
                        }
                    }
                    BinaryOperator::BitXor => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_xor(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for bitwise xor");
                        }
                    }
                    BinaryOperator::LeftShift => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_left_shift(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for left shift");
                        }
                    }
                    BinaryOperator::RightShift => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            Ok(Some(
                                self.builder.build_right_shift(l, r, false, "")?.into(),
                            ))
                        } else {
                            anyhow::bail!("Invalid types for right shift");
                        }
                    }
                    BinaryOperator::Modulus => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (left_val, right_val)
                        {
                            if is_unsigned {
                                Ok(Some(self.builder.build_int_unsigned_rem(l, r, "")?.into()))
                            } else {
                                Ok(Some(self.builder.build_int_signed_rem(l, r, "")?.into()))
                            }
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (left_val, right_val)
                        {
                            Ok(Some(self.builder.build_float_rem(l, r, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid types for modulus");
                        }
                    }
                    BinaryOperator::RangeTo
                    | BinaryOperator::RangeUntil
                    | BinaryOperator::OpenRange => {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Range expressions are not yet supported in IR generation",
                            None,
                        );
                        anyhow::bail!("Range expressions not implemented");
                    }
                    BinaryOperator::NullCoalescing => {
                        if let Some(ty) = left_ty.clone()
                            && let Type::Enum(enum_name, _, _) = &*ty.borrow()
                            && enum_name == "Optional"
                        {
                            let enum_type = self
                                .enum_types
                                .borrow()
                                .get(enum_name)
                                .copied()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Enum type '{}' not found", enum_name)
                                })?;
                            let payloads_type = self
                                .enum_payload_types
                                .borrow()
                                .get(enum_name)
                                .copied()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Enum payload type '{}' not found", enum_name)
                                })?;
                            let alloca = self
                                .builder
                                .build_alloca(enum_type.as_basic_type_enum(), "")?;
                            self.builder.build_store(alloca, left_val)?;
                            let tag_ptr =
                                self.builder.build_struct_gep(enum_type, alloca, 0, "")?;
                            let tag =
                                self.builder
                                    .build_load(self.context.i8_type(), tag_ptr, "")?;
                            let none_tag = self.context.i8_type().const_zero();
                            let is_none = self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                tag.into_int_value(),
                                none_tag,
                                "",
                            )?;
                            let fn_val = self
                                .builder
                                .get_insert_block()
                                .unwrap()
                                .get_parent()
                                .unwrap();
                            let left_some_bb =
                                self.context.append_basic_block(fn_val, "coalesce_some");
                            let left_none_bb =
                                self.context.append_basic_block(fn_val, "coalesce_none");
                            let cont_bb = self.context.append_basic_block(fn_val, "coalesce_cont");
                            self.builder.build_conditional_branch(
                                is_none,
                                left_none_bb,
                                left_some_bb,
                            )?;
                            self.builder.position_at_end(left_some_bb);
                            let payload_union_ptr =
                                self.builder.build_struct_gep(enum_type, alloca, 1, "")?;
                            let some_payload_ptr = self.builder.build_struct_gep(
                                payloads_type,
                                payload_union_ptr,
                                1,
                                "",
                            )?;
                            let some_payload_ty = payloads_type
                                .get_field_type_at_index(1)
                                .ok_or_else(|| anyhow::anyhow!("Some payload field not found"))?
                                .into_struct_type();
                            let payload_val =
                                self.builder
                                    .build_load(some_payload_ty, some_payload_ptr, "")?;
                            self.builder.build_unconditional_branch(cont_bb)?;
                            self.builder.position_at_end(left_none_bb);
                            let none_val = right_val;
                            self.builder.build_unconditional_branch(cont_bb)?;
                            self.builder.position_at_end(cont_bb);
                            let result_ty =
                                payloads_type.get_field_type_at_index(1).ok_or_else(|| {
                                    anyhow::anyhow!("Some payload field type not found")
                                })?;
                            let phi = self.builder.build_phi(result_ty, "")?;
                            phi.add_incoming(&[
                                (&payload_val, left_some_bb),
                                (&none_val, left_none_bb),
                            ]);
                            let result = phi.as_basic_value();
                            Ok(Some(result))
                        } else {
                            anyhow::bail!("Null-coalescing requires Optional type");
                        }
                    }
                }
            }
            Expression::Unary {
                expression: expr,
                operator,
                overloads,
                selected_index,
                ..
            } => {
                if let Some(idx) = *selected_index
                    && idx < overloads.len()
                {
                    let sym = &overloads[idx];
                    let op_name = operator.operator_name().to_string();
                    let is_static = sym.borrow().get_decl().ok().flatten().is_some_and(|d| {
                        if let Statement::FunctionDecl { static_method, .. } = &*d.borrow() {
                            *static_method
                        } else {
                            false
                        }
                    });

                    let params = vec![CallParameter {
                        label: None,
                        expression: expr.clone(),
                    }];
                    let fn_name = if is_static {
                        self.mangle_from_overload(&op_name, sym, &params)
                            .unwrap_or(op_name.clone())
                    } else {
                        let ty_name = {
                            let e = expr.borrow();
                            match e.get_ty_ref().ok() {
                                Some(Some(ty)) => {
                                    let tb = ty.borrow();
                                    match &*tb {
                                        Type::Struct(n, ..)
                                        | Type::Class(n, ..)
                                        | Type::Enum(n, ..) => n.clone(),
                                        _ => op_name.clone(),
                                    }
                                }
                                _ => op_name.clone(),
                            }
                        };
                        let base = format!("{}.{}", ty_name, op_name);
                        self.mangle_from_overload(&base, sym, &params)
                            .unwrap_or(base)
                    };

                    if let Some(f) = self.module.get_function(&fn_name) {
                        let expr_val = self.resolve_expression(expr.clone())?.unwrap();
                        let args = if is_static {
                            vec![expr_val.into()]
                        } else {
                            vec![]
                        };
                        let call_result = self.builder.build_call(f, &args, "")?;
                        match call_result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(val) => return Ok(Some(val)),
                            _ => return Ok(None),
                        }
                    }
                }

                let expr_val = self.resolve_expression(expr.clone())?.unwrap();
                match operator {
                    UnaryOperator::Plus => Ok(Some(expr_val)),
                    UnaryOperator::Minus => {
                        if let BasicValueEnum::IntValue(v) = expr_val {
                            Ok(Some(self.builder.build_int_neg(v, "")?.into()))
                        } else if let BasicValueEnum::FloatValue(v) = expr_val {
                            Ok(Some(self.builder.build_float_neg(v, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid type for unary minus");
                        }
                    }
                    UnaryOperator::Not => {
                        if let BasicValueEnum::IntValue(v) = expr_val {
                            let one = self.context.bool_type().const_int(1, false);
                            Ok(Some(self.builder.build_xor(v, one, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid type for logical not");
                        }
                    }
                    UnaryOperator::BitNot => {
                        if let BasicValueEnum::IntValue(v) = expr_val {
                            Ok(Some(self.builder.build_not(v, "")?.into()))
                        } else {
                            anyhow::bail!("Invalid type for bitwise not");
                        }
                    }
                    UnaryOperator::Inc => {
                        let one = if let BasicValueEnum::IntValue(val) = expr_val {
                            val.get_type().const_int(1, false).into()
                        } else if let BasicValueEnum::FloatValue(val) = expr_val {
                            val.get_type().const_float(1.0).into()
                        } else {
                            anyhow::bail!("Invalid type for increment");
                        };
                        let result = if let (
                            BasicValueEnum::IntValue(v),
                            BasicValueEnum::IntValue(one_val),
                        ) = (expr_val, one)
                        {
                            self.builder.build_int_add(v, one_val, "")?.into()
                        } else if let (
                            BasicValueEnum::FloatValue(v),
                            BasicValueEnum::FloatValue(one_val),
                        ) = (expr_val, one)
                        {
                            self.builder.build_float_add(v, one_val, "")?.into()
                        } else {
                            anyhow::bail!("Invalid type for increment");
                        };
                        if let Expression::Variable { name, .. } = &*expr.borrow()
                            && let Some(ptr) = self.lookup_variable(&name.value)
                        {
                            self.builder.build_store(ptr, result)?;
                        }
                        Ok(Some(result))
                    }
                    UnaryOperator::Dec => {
                        let one = if let BasicValueEnum::IntValue(val) = expr_val {
                            val.get_type().const_int(1, false).into()
                        } else if let BasicValueEnum::FloatValue(val) = expr_val {
                            val.get_type().const_float(1.0).into()
                        } else {
                            anyhow::bail!("Invalid type for decrement");
                        };
                        let result = if let (
                            BasicValueEnum::IntValue(v),
                            BasicValueEnum::IntValue(one_val),
                        ) = (expr_val, one)
                        {
                            self.builder.build_int_sub(v, one_val, "")?.into()
                        } else if let (
                            BasicValueEnum::FloatValue(v),
                            BasicValueEnum::FloatValue(one_val),
                        ) = (expr_val, one)
                        {
                            self.builder.build_float_sub(v, one_val, "")?.into()
                        } else {
                            anyhow::bail!("Invalid type for decrement");
                        };
                        if let Expression::Variable { name, .. } = &*expr.borrow()
                            && let Some(ptr) = self.lookup_variable(&name.value)
                        {
                            self.builder.build_store(ptr, result)?;
                        }
                        Ok(Some(result))
                    }
                    UnaryOperator::NotNullAssertation => {
                        let expr_ty = expr.borrow().get_ty().ok().flatten();
                        if let Some(ty) = expr_ty
                            && let Type::Enum(enum_name, _, params) = &*ty.borrow()
                            && enum_name == "Optional"
                            && !params.is_empty()
                        {
                            let enum_type = self
                                .enum_types
                                .borrow()
                                .get(enum_name)
                                .copied()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Enum type '{}' not found", enum_name)
                                })?;
                            let payloads_type = self
                                .enum_payload_types
                                .borrow()
                                .get(enum_name)
                                .copied()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Enum payload type '{}' not found", enum_name)
                                })?;
                            let alloca = self
                                .builder
                                .build_alloca(enum_type.as_basic_type_enum(), "")?;
                            self.builder.build_store(alloca, expr_val)?;
                            let tag_ptr =
                                self.builder.build_struct_gep(enum_type, alloca, 0, "")?;
                            let tag =
                                self.builder
                                    .build_load(self.context.i8_type(), tag_ptr, "")?;
                            let none_tag = self.context.i8_type().const_zero();
                            let is_none = self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                tag.into_int_value(),
                                none_tag,
                                "",
                            )?;
                            let fn_val = self
                                .builder
                                .get_insert_block()
                                .unwrap()
                                .get_parent()
                                .unwrap();
                            let some_bb = self.context.append_basic_block(fn_val, "unwrap_some");
                            let trap_bb = self.context.append_basic_block(fn_val, "unwrap_trap");
                            let cont_bb = self.context.append_basic_block(fn_val, "unwrap_cont");
                            self.builder
                                .build_conditional_branch(is_none, trap_bb, some_bb)?;
                            self.builder.position_at_end(trap_bb);
                            let trap_fn =
                                self.module.get_function("llvm.trap").unwrap_or_else(|| {
                                    self.module.add_function(
                                        "llvm.trap",
                                        self.context.void_type().fn_type(&[], false),
                                        None,
                                    )
                                });
                            self.builder.build_call(trap_fn, &[], "")?;
                            self.builder.build_unreachable()?;
                            self.builder.position_at_end(some_bb);
                            let payload_union_ptr =
                                self.builder.build_struct_gep(enum_type, alloca, 1, "")?;
                            let some_payload_ptr = self.builder.build_struct_gep(
                                payloads_type,
                                payload_union_ptr,
                                1,
                                "",
                            )?;
                            let some_payload_ty = payloads_type
                                .get_field_type_at_index(1)
                                .ok_or_else(|| anyhow::anyhow!("Some payload field not found"))?
                                .into_struct_type();
                            let payload_val =
                                self.builder
                                    .build_load(some_payload_ty, some_payload_ptr, "")?;
                            self.builder.build_unconditional_branch(cont_bb)?;
                            self.builder.position_at_end(cont_bb);
                            let result_ty =
                                payloads_type.get_field_type_at_index(1).ok_or_else(|| {
                                    anyhow::anyhow!("Some payload field type not found")
                                })?;
                            let phi = self.builder.build_phi(result_ty, "")?;
                            phi.add_incoming(&[(&payload_val, some_bb)]);
                            let result = phi.as_basic_value();
                            Ok(Some(result))
                        } else {
                            Ok(Some(expr_val))
                        }
                    }
                    UnaryOperator::OpenRange => {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Open range operator is not yet supported in IR generation",
                            None,
                        );
                        anyhow::bail!("Open range not implemented");
                    }
                    UnaryOperator::Deref => {
                        if let BasicValueEnum::PointerValue(ptr) = expr_val {
                            let expr_borrowed = expr.borrow();
                            let ty_opt = expr_borrowed.get_ty_ref()?;
                            let ty = ty_opt.as_ref().ok_or_else(|| anyhow::anyhow!("No type"))?;
                            let llvm_ty = self.resolve_type(ty.clone())?;
                            drop(expr_borrowed);
                            Ok(Some(self.builder.build_load(llvm_ty, ptr, "")?))
                        } else {
                            anyhow::bail!("Invalid type for dereference");
                        }
                    }
                    UnaryOperator::AddressOf => {
                        let inner = expr.borrow();
                        if let Expression::Variable { name, .. } = &*inner {
                            if let Some(ptr) = self.lookup_variable(&name.value) {
                                return Ok(Some(ptr.into()));
                            }
                            if let Some(f) = self.module.get_function(&name.value) {
                                let fn_ptr = f.as_global_value().as_pointer_value();
                                return Ok(Some(fn_ptr.into()));
                            }
                        }
                        if let Expression::Unary {
                            expression: deref_target,
                            operator: UnaryOperator::Deref,
                            ..
                        } = &*inner
                        {
                            let ptr_val = self.resolve_expression(deref_target.clone())?;
                            return Ok(ptr_val);
                        }
                        if let Expression::MemberAccess { object, member, .. } = &*inner {
                            if let Expression::Variable { name, .. } = &*object.borrow() {
                                let fn_name = format!("{}.{}", name.value, member.value);
                                if let Some(f) = self.module.get_function(&fn_name) {
                                    let fn_ptr = f.as_global_value().as_pointer_value();
                                    return Ok(Some(fn_ptr.into()));
                                }
                            }
                            let object_expr = object.borrow();
                            let object_ty = object_expr.get_ty_ref()?.clone();
                            drop(object_expr);
                            if let Some(ty) = &object_ty {
                                let (type_name, class_types) = match &*ty.borrow() {
                                    Type::Struct(n, _, _) => (n.clone(), false),
                                    Type::Class(n, _, _) => (n.clone(), true),
                                    _ => (String::new(), false),
                                };
                                if !type_name.is_empty() {
                                    let field_name = member.value.clone();
                                    let object_val =
                                        self.resolve_expression(object.clone())?.unwrap();
                                    let ptr = if let BasicValueEnum::PointerValue(p) = object_val {
                                        p
                                    } else {
                                        let p =
                                            self.builder.build_alloca(object_val.get_type(), "")?;
                                        self.builder.build_store(p, object_val)?;
                                        p
                                    };
                                    let field_index = self
                                        .get_stored_struct_field_index(&type_name, &field_name)?;
                                    let llvm_type = if class_types {
                                        *self.class_types.borrow().get(&type_name).unwrap()
                                    } else {
                                        *self.struct_types.borrow().get(&type_name).unwrap()
                                    };
                                    let field_ptr = self.builder.build_struct_gep(
                                        llvm_type,
                                        ptr,
                                        field_index as u32,
                                        "",
                                    )?;
                                    return Ok(Some(field_ptr.into()));
                                }
                            }
                        }
                        anyhow::bail!(
                            "AddressOf operator not yet supported for this expression in IR generation"
                        );
                    }
                }
            }
            Expression::Assignment {
                left,
                operator,
                right,
            } => {
                let right_val = self.resolve_expression(right.clone())?.unwrap();

                if let Expression::Variable { name, .. } = &*left.borrow() {
                    let setter_name = self.mangle_fn_name(&format!("{}.setter", name.value), &[]);
                    let willset_name = self.mangle_fn_name(&format!("{}.willSet", name.value), &[]);
                    let didset_name = self.mangle_fn_name(&format!("{}.didSet", name.value), &[]);
                    let has_setter = self.module.get_function(&setter_name).is_some();
                    let has_willset = self.module.get_function(&willset_name).is_some();
                    let has_didset = self.module.get_function(&didset_name).is_some();
                    if has_setter || has_willset || has_didset {
                        let ty_opt = left.borrow().get_ty()?;
                        let ty = if let Some(ty_rc) = ty_opt {
                            self.resolve_type(ty_rc)?
                        } else {
                            self.context.i32_type().into()
                        };
                        let ptr = self.lookup_variable(&name.value).ok_or_else(|| {
                            self.emit_error(
                                TrussDiagnosticCode::UndefinedVariable,
                                format!("Undefined variable: '{}'", name.value),
                                Some(name),
                            );
                            anyhow::anyhow!("Undefined variable")
                        })?;
                        let current_val = self.builder.build_load(ty, ptr, "")?;
                        let store_val = match operator {
                            AssignmentOperator::Assign => right_val,
                            _ => {
                                let op = *operator;
                                let left_val = current_val;
                                self.compute_compound_assign(left_val, right_val, op)?
                            }
                        };
                        if has_willset {
                            let willset_fn = self.module.get_function(&willset_name).unwrap();
                            let args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                                vec![ptr.into(), store_val.into()];
                            self.builder.build_call(willset_fn, &args, "")?;
                        }
                        if has_setter {
                            let setter_fn = self.module.get_function(&setter_name).unwrap();
                            let args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                                vec![ptr.into(), store_val.into()];
                            self.builder.build_call(setter_fn, &args, "")?;
                        } else {
                            self.builder.build_store(ptr, store_val)?;
                        }
                        if has_didset {
                            let didset_fn = self.module.get_function(&didset_name).unwrap();
                            let args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                                vec![ptr.into(), current_val.into()];
                            self.builder.build_call(didset_fn, &args, "")?;
                        }
                        return Ok(Some(store_val));
                    }
                }
                let (var_ptr, current_val) = if let Expression::Variable { name, .. } =
                    &*left.borrow()
                {
                    if let Some(ptr) = self.lookup_variable(&name.value) {
                        let ty_opt = left.borrow().get_ty()?;
                        let ty = if let Some(ty_rc) = ty_opt {
                            self.resolve_type(ty_rc)?
                        } else {
                            self.context.i32_type().into()
                        };
                        let val = self.builder.build_load(ty, ptr, "")?;
                        (ptr, Some(val))
                    } else if let Some((sname, struct_ptr)) =
                        &*self.current_accessor_struct.borrow()
                        && let Ok(idx) = self.get_stored_struct_field_index(sname, &name.value)
                        && let Some(stype) = self.struct_types.borrow().get(sname).copied()
                        && let Ok(field_ptr) =
                            self.builder
                                .build_struct_gep(stype, *struct_ptr, idx as u32, "")
                    {
                        self.declare_variable(name.value.clone(), field_ptr);
                        let ty_opt = left.borrow().get_ty()?;
                        let ty = if let Some(ty_rc) = ty_opt {
                            self.resolve_type(ty_rc)?
                        } else {
                            self.context.i32_type().into()
                        };
                        let val = self.builder.build_load(ty, field_ptr, "")?;
                        (field_ptr, Some(val))
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::UndefinedVariable,
                            format!("Undefined variable: '{}'", name.value),
                            Some(name),
                        );
                        anyhow::bail!("Undefined variable");
                    }
                } else if let Expression::MemberAccess { object, member, .. } = &*left.borrow() {
                    let object_expr = object.borrow();
                    let object_ty = object_expr.get_ty_ref()?.clone();
                    drop(object_expr);

                    if let Some(ty) = object_ty {
                        if let Type::Struct(struct_name, ..) = &*ty.borrow() {
                            let struct_name = struct_name.clone();
                            let field_name = member.value.clone();

                            let object_val = self.resolve_expression(object.clone())?.unwrap();

                            let struct_ptr = if let BasicValueEnum::PointerValue(ptr) = object_val {
                                ptr
                            } else {
                                let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                                self.builder.build_store(ptr, object_val)?;
                                ptr
                            };

                            let setter_name = self.mangle_fn_name(
                                &format!("{}.{}.setter", struct_name, field_name),
                                &[],
                            );
                            let willset_name = self.mangle_fn_name(
                                &format!("{}.{}.willSet", struct_name, field_name),
                                &[],
                            );
                            let didset_name = self.mangle_fn_name(
                                &format!("{}.{}.didSet", struct_name, field_name),
                                &[],
                            );

                            let has_setter = self.module.get_function(&setter_name).is_some();
                            let has_willset = self.module.get_function(&willset_name).is_some();
                            let has_didset = self.module.get_function(&didset_name).is_some();

                            if has_setter {
                                if has_willset {
                                    let willset_fn =
                                        self.module.get_function(&willset_name).unwrap();
                                    self.builder.build_call(
                                        willset_fn,
                                        &[struct_ptr.into(), right_val.into()],
                                        "",
                                    )?;
                                }
                                let setter_fn = self.module.get_function(&setter_name).unwrap();
                                self.builder.build_call(
                                    setter_fn,
                                    &[struct_ptr.into(), right_val.into()],
                                    "",
                                )?;
                                if has_didset {
                                    let didset_fn = self.module.get_function(&didset_name).unwrap();
                                    self.builder.build_call(
                                        didset_fn,
                                        &[struct_ptr.into(), right_val.into()],
                                        "",
                                    )?;
                                }
                                return Ok(Some(right_val));
                            }

                            if has_willset || has_didset {
                                let field_index =
                                    self.get_stored_struct_field_index(&struct_name, &field_name)?;
                                let field_ptr = self.builder.build_struct_gep(
                                    *self.struct_types.borrow().get(&struct_name).unwrap(),
                                    struct_ptr,
                                    field_index as u32,
                                    "",
                                )?;
                                let field_ty =
                                    self.get_struct_field_type(&struct_name, &field_name)?;
                                let current_val = if has_didset {
                                    Some(self.builder.build_load(field_ty, field_ptr, "")?)
                                } else {
                                    None
                                };

                                if has_willset {
                                    self.builder.build_call(
                                        self.module.get_function(&willset_name).unwrap(),
                                        &[struct_ptr.into(), right_val.into()],
                                        "",
                                    )?;
                                }
                                self.builder.build_store(field_ptr, right_val)?;
                                if let Some(old_val) = current_val {
                                    self.builder.build_call(
                                        self.module.get_function(&didset_name).unwrap(),
                                        &[struct_ptr.into(), old_val.into()],
                                        "",
                                    )?;
                                }
                                return Ok(Some(right_val));
                            }

                            let field_index =
                                self.get_stored_struct_field_index(&struct_name, &field_name)?;
                            let field_ptr = self.builder.build_struct_gep(
                                *self.struct_types.borrow().get(&struct_name).unwrap(),
                                struct_ptr,
                                field_index as u32,
                                "",
                            )?;
                            let field_ty = self.get_struct_field_type(&struct_name, &field_name)?;
                            let val = self.builder.build_load(field_ty, field_ptr, "")?;
                            (field_ptr, Some(val))
                        } else if let Type::Class(class_name, ..) = &*ty.borrow() {
                            let class_name = class_name.clone();
                            let field_name = member.value.clone();

                            let object_val = self.resolve_expression(object.clone())?.unwrap();

                            let class_ptr = if let BasicValueEnum::PointerValue(ptr) = object_val {
                                ptr
                            } else {
                                let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                                self.builder.build_store(ptr, object_val)?;
                                ptr
                            };

                            let setter_entry = format!("{}.setter", field_name);
                            let is_super =
                                matches!(&*object.borrow(), Expression::SuperKeyword { .. });
                            if !is_super {
                                if let Some(slot_idx) =
                                    self.get_vtable_slot_index(&class_name, &setter_entry)
                                {
                                    let class_type =
                                        *self.class_types.borrow().get(&class_name).unwrap();
                                    let vtable_ptr_ptr = self
                                        .builder
                                        .build_struct_gep(class_type, class_ptr, 0, "")?;
                                    let vtable_ptr = self
                                        .builder
                                        .build_load(
                                            self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                            vtable_ptr_ptr,
                                            "",
                                        )?
                                        .into_pointer_value();

                                    let vtable_type =
                                        *self.vtable_types.borrow().get(&class_name).unwrap();
                                    let fn_ptr_ptr = self.builder.build_struct_gep(
                                        vtable_type,
                                        vtable_ptr,
                                        slot_idx,
                                        "",
                                    )?;
                                    let fn_ptr_val = self
                                        .builder
                                        .build_load(
                                            self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                            fn_ptr_ptr,
                                            "",
                                        )?
                                        .into_pointer_value();

                                    let method_list = self.compute_vtable_method_list(&class_name);
                                    let (_, owner) = method_list
                                        .iter()
                                        .find(|(n, _)| n == &setter_entry)
                                        .unwrap();
                                    let declared_fn_name = self.mangle_fn_name(
                                        &format!("{}.{}.setter", owner, field_name),
                                        &[],
                                    );
                                    let declared_fn = self
                                        .module
                                        .get_function(&declared_fn_name)
                                        .ok_or_else(|| {
                                        anyhow::anyhow!(
                                            "Setter function {} not found",
                                            declared_fn_name
                                        )
                                    })?;
                                    let fn_type = declared_fn.get_type();

                                    self.builder.build_indirect_call(
                                        fn_type,
                                        fn_ptr_val,
                                        &[class_ptr.into(), right_val.into()],
                                        "",
                                    )?;
                                    return Ok(Some(right_val));
                                }
                            }

                            let field_index =
                                self.get_stored_class_field_index(&class_name, &field_name)?;
                            let field_ptr = self.builder.build_struct_gep(
                                *self.class_types.borrow().get(&class_name).unwrap(),
                                class_ptr,
                                field_index as u32,
                                "",
                            )?;
                            let field_ty = self.get_struct_field_type(&class_name, &field_name)?;
                            let val = self.builder.build_load(field_ty, field_ptr, "")?;
                            (field_ptr, Some(val))
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::UnsupportedFeature,
                                "Member access on non-struct/class type",
                                Some(member.as_ref()),
                            );
                            anyhow::bail!("Member access on non-struct type");
                        }
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Cannot infer type for member access",
                            Some(member.as_ref()),
                        );
                        anyhow::bail!("Cannot infer type");
                    }
                } else if let Expression::SubscriptAccess {
                    object: sub_object,
                    parameters: sub_params,
                    ..
                } = &*left.borrow()
                {
                    let sub_ty = {
                        let obj = sub_object.borrow();
                        obj.get_ty_ref()?.clone()
                    };
                    if let Some(ty) = &sub_ty
                        && let Type::Struct(struct_name, ..) = &*ty.borrow()
                    {
                        let struct_name = struct_name.clone();
                        let object_val = self.resolve_expression(sub_object.clone())?.unwrap();
                        let struct_ptr = if let Expression::Variable { name, .. } =
                            &*sub_object.borrow()
                        {
                            if let Some(ptr) = self.lookup_variable(&name.value) {
                                ptr
                            } else {
                                if let BasicValueEnum::PointerValue(ptr) = object_val {
                                    ptr
                                } else {
                                    let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                                    self.builder.build_store(ptr, object_val)?;
                                    ptr
                                }
                            }
                        } else {
                            if let BasicValueEnum::PointerValue(ptr) = object_val {
                                ptr
                            } else {
                                let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                                self.builder.build_store(ptr, object_val)?;
                                ptr
                            }
                        };
                        let setter_name = self
                            .mangled_fn_names
                            .borrow()
                            .get(&format!("{}.subscript.setter", struct_name))
                            .cloned()
                            .unwrap_or_else(|| {
                                self.mangle_fn_name(
                                    &format!("{}.subscript.setter", struct_name),
                                    &[],
                                )
                            });
                        if let Some(setter_fn) = self.module.get_function(&setter_name) {
                            let mut args = vec![struct_ptr.into()];
                            for p in sub_params {
                                let arg_val =
                                    self.resolve_expression(p.expression.clone())?.unwrap();
                                args.push(arg_val.into());
                            }
                            args.push(right_val.into());
                            self.builder.build_call(setter_fn, &args, "")?;
                            return Ok(Some(right_val));
                        }
                    }
                    if let Some(ty) = &sub_ty
                        && let Type::Class(class_name, ..) = &*ty.borrow()
                    {
                        let class_name = class_name.clone();
                        let object_val = self.resolve_expression(sub_object.clone())?.unwrap();
                        let class_ptr = if let BasicValueEnum::PointerValue(ptr) = object_val {
                            ptr
                        } else {
                            let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                            self.builder.build_store(ptr, object_val)?;
                            ptr
                        };
                        let setter_entry = "subscript.setter";
                        if let Some(slot_idx) =
                            self.get_vtable_slot_index(&class_name, setter_entry)
                        {
                            let class_type = *self.class_types.borrow().get(&class_name).unwrap();
                            let vtable_ptr_ptr = self
                                .builder
                                .build_struct_gep(class_type, class_ptr, 0, "")?;
                            let vtable_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                    vtable_ptr_ptr,
                                    "",
                                )?
                                .into_pointer_value();
                            let vtable_type = *self.vtable_types.borrow().get(&class_name).unwrap();
                            let fn_ptr_ptr = self.builder.build_struct_gep(
                                vtable_type,
                                vtable_ptr,
                                slot_idx,
                                "",
                            )?;
                            let fn_ptr_val = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                    fn_ptr_ptr,
                                    "",
                                )?
                                .into_pointer_value();
                            let method_list = self.compute_vtable_method_list(&class_name);
                            let (_, owner) = method_list
                                .iter()
                                .find(|(n, _)| n == &setter_entry)
                                .unwrap();
                            let declared_fn_name =
                                self.mangle_fn_name(&format!("{}.subscript.setter", owner), &[]);
                            let declared_fn =
                                self.module.get_function(&declared_fn_name).ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Subscript setter function {} not found",
                                        declared_fn_name
                                    )
                                })?;
                            let fn_type = declared_fn.get_type();
                            let mut args = vec![class_ptr.into()];
                            for p in sub_params {
                                let arg_val =
                                    self.resolve_expression(p.expression.clone())?.unwrap();
                                args.push(arg_val.into());
                            }
                            args.push(right_val.into());
                            self.builder
                                .build_indirect_call(fn_type, fn_ptr_val, &args, "")?;
                            return Ok(Some(right_val));
                        }
                    }
                    anyhow::bail!("Subscript assignment requires struct or class type")
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UnsupportedFeature,
                        "Invalid assignment target",
                        None,
                    );
                    anyhow::bail!("Invalid assignment target");
                };

                let result = match operator {
                    AssignmentOperator::Assign => {
                        self.builder.build_store(var_ptr, right_val)?;
                        right_val
                    }
                    AssignmentOperator::PlusAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_add(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_add(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for += assignment");
                        }
                    }
                    AssignmentOperator::MinusAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_sub(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_sub(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for -= assignment");
                        }
                    }
                    AssignmentOperator::MultiplyAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_mul(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_mul(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for *= assignment");
                        }
                    }
                    AssignmentOperator::DivideAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_signed_div(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_div(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for /= assignment");
                        }
                    }
                    AssignmentOperator::ModulusAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_int_signed_rem(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else if let (
                            BasicValueEnum::FloatValue(l),
                            BasicValueEnum::FloatValue(r),
                        ) = (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_float_rem(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for %= assignment");
                        }
                    }
                    AssignmentOperator::BitAndAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_and(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for &= assignment");
                        }
                    }
                    AssignmentOperator::BitOrAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_or(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for |= assignment");
                        }
                    }
                    AssignmentOperator::LeftShiftAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_left_shift(l, r, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for <<= assignment");
                        }
                    }
                    AssignmentOperator::RightShiftAssign => {
                        if let (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) =
                            (current_val.unwrap(), right_val)
                        {
                            let result = self.builder.build_right_shift(l, r, false, "")?;
                            self.builder.build_store(var_ptr, result)?;
                            result.into()
                        } else {
                            anyhow::bail!("Invalid types for >>= assignment");
                        }
                    }
                };

                Ok(Some(result))
            }
            Expression::If {
                condition,
                then,
                else_,
                ty,
            } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                if let Expression::Case {
                    enum_type,
                    case_name,
                    bindings,
                    expression,
                    ..
                } = &*condition.borrow()
                {
                    let then_bb = self.context.append_basic_block(fn_val, "if_then");
                    let else_bb = if else_.is_some() {
                        Some(self.context.append_basic_block(fn_val, "if_else"))
                    } else {
                        None
                    };
                    let exit_bb = self.context.append_basic_block(fn_val, "if_exit");
                    let case_match_bb = self.context.append_basic_block(fn_val, "case_match");

                    self.builder.build_unconditional_branch(case_match_bb)?;
                    self.builder.position_at_end(case_match_bb);

                    let subject_val = self.resolve_expression(expression.clone())?.unwrap();
                    let subject_alloca = self.builder.build_alloca(subject_val.get_type(), "")?;
                    self.builder.build_store(subject_alloca, subject_val)?;

                    let enum_name = &enum_type.as_ref().unwrap().value;
                    let case_idx = self.get_enum_case_index(enum_name, &case_name.value)?;

                    let enum_types = self.enum_types.borrow();
                    let enum_llvm_type = enum_types
                        .get(enum_name)
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
                    drop(enum_types);

                    let match_result = if let Some(raw_ty) = self.get_enum_raw_llvm_type(enum_name)
                    {
                        let tag_ptr =
                            self.builder
                                .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                        let tag_val = self.builder.build_load(raw_ty, tag_ptr, "")?;
                        let expected_tag = raw_ty.into_int_type().const_int(case_idx as u64, false);
                        self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            tag_val.into_int_value(),
                            expected_tag,
                            "",
                        )?
                    } else {
                        let tag_ptr =
                            self.builder
                                .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                        let tag_val =
                            self.builder
                                .build_load(self.context.i8_type(), tag_ptr, "")?;
                        let expected_tag = self.context.i8_type().const_int(case_idx as u64, false);
                        self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            tag_val.into_int_value(),
                            expected_tag,
                            "",
                        )?
                    };

                    self.builder.build_conditional_branch(
                        match_result,
                        then_bb,
                        if let Some(else_bb) = else_bb {
                            else_bb
                        } else {
                            exit_bb
                        },
                    )?;

                    self.builder.position_at_end(then_bb);

                    if !bindings.is_empty() {
                        let enum_payloads = self.enum_payload_types.borrow();
                        if let Some(payload_type) = enum_payloads.get(enum_name) {
                            let payload_union_ptr = self.builder.build_struct_gep(
                                enum_llvm_type,
                                subject_alloca,
                                1,
                                "",
                            )?;
                            let case_payload_ptr = self.builder.build_struct_gep(
                                *payload_type,
                                payload_union_ptr,
                                case_idx as u32,
                                "",
                            )?;
                            let case_payload_ty = payload_type
                                .get_field_type_at_index(case_idx as u32)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Case payload field not found at index {}",
                                        case_idx
                                    )
                                })?;
                            let case_payload_struct_ty = case_payload_ty.into_struct_type();

                            self.enter_scope();
                            for (i, binding) in bindings.iter().enumerate() {
                                match binding {
                                    Pattern::Identifier(token) => {
                                        let field_ptr = self.builder.build_struct_gep(
                                            case_payload_struct_ty,
                                            case_payload_ptr,
                                            i as u32,
                                            "",
                                        )?;
                                        let field_ty = case_payload_struct_ty
                                            .get_field_type_at_index(i as u32)
                                            .ok_or_else(|| {
                                                anyhow::anyhow!(
                                                    "Binding field not found at index {}",
                                                    i
                                                )
                                            })?;
                                        let field_val =
                                            self.builder.build_load(field_ty, field_ptr, "")?;
                                        let var_ptr =
                                            self.builder.build_alloca(field_ty, &token.value)?;
                                        self.builder.build_store(var_ptr, field_val)?;
                                        self.declare_variable(token.value.clone(), var_ptr);
                                    }
                                    Pattern::Ignore => {}
                                    _ => {}
                                }
                            }
                            let terminates = self.resolve_block_expression(then)?;
                            if !terminates {
                                self.builder.build_unconditional_branch(exit_bb)?;
                            }
                            self.exit_scope();
                        } else {
                            let terminates = self.resolve_block_expression(then)?;
                            if !terminates {
                                self.builder.build_unconditional_branch(exit_bb)?;
                            }
                        }
                    } else {
                        let terminates = self.resolve_block_expression(then)?;
                        if !terminates {
                            self.builder.build_unconditional_branch(exit_bb)?;
                        }
                    }

                    if let Some(else_) = else_ {
                        self.builder.position_at_end(else_bb.unwrap());
                        let terminates = match else_ {
                            ElseBranch::Block(body) => self.resolve_block_expression(body)?,
                            ElseBranch::If(if_expr) => {
                                self.resolve_expression(if_expr.clone())?;
                                false
                            }
                        };
                        if !terminates {
                            self.builder.build_unconditional_branch(exit_bb)?;
                        }
                    }

                    self.builder.position_at_end(exit_bb);

                    Ok(None)
                } else {
                    let cond_bb = self.context.append_basic_block(fn_val, "if_cond");
                    let then_bb = self.context.append_basic_block(fn_val, "if_then");
                    let else_bb = if else_.is_some() {
                        Some(self.context.append_basic_block(fn_val, "if_else"))
                    } else {
                        None
                    };
                    let exit_bb = self.context.append_basic_block(fn_val, "if_exit");

                    let result_alloca = match (ty.as_ref(), else_.as_ref()) {
                        (Some(t), Some(_)) => self.resolve_type(t.clone()).ok().map(|llvm_ty| {
                            (self.builder.build_alloca(llvm_ty, "if_result"), llvm_ty)
                        }),
                        _ => None,
                    };

                    self.builder.build_unconditional_branch(cond_bb)?;
                    self.builder.position_at_end(cond_bb);

                    let cond_val = self
                        .resolve_expression(condition.clone())?
                        .unwrap()
                        .into_int_value();
                    self.builder.build_conditional_branch(
                        cond_val,
                        then_bb,
                        if let Some(else_bb) = else_bb {
                            else_bb
                        } else {
                            exit_bb
                        },
                    )?;

                    self.builder.position_at_end(then_bb);
                    if let Some((Ok(alloca), _)) = result_alloca.as_ref() {
                        self.yield_targets.borrow_mut().push((*alloca, exit_bb));
                    }
                    let (terminates, then_val) = self.resolve_block_and_get_value(then)?;
                    if let Some((Ok(_alloca), _)) = result_alloca.as_ref() {
                        self.yield_targets.borrow_mut().pop();
                    }
                    if let (Some((Ok(alloca), _)), Some(val)) = (&result_alloca, then_val) {
                        self.builder.build_store(*alloca, val)?;
                    }
                    if !terminates {
                        self.builder.build_unconditional_branch(exit_bb)?;
                    }

                    if let Some(else_) = else_ {
                        self.builder.position_at_end(else_bb.unwrap());
                        if let Some((Ok(alloca), _)) = result_alloca.as_ref() {
                            self.yield_targets.borrow_mut().push((*alloca, exit_bb));
                        }
                        let (terminates, else_val) = match else_ {
                            ElseBranch::Block(body) => self.resolve_block_and_get_value(body)?,
                            ElseBranch::If(if_expr) => {
                                let val = self.resolve_expression(if_expr.clone())?;
                                (false, val)
                            }
                        };
                        if let Some((Ok(_alloca), _)) = result_alloca.as_ref() {
                            self.yield_targets.borrow_mut().pop();
                        }
                        if let (Some((Ok(alloca), _)), Some(val)) = (&result_alloca, else_val) {
                            self.builder.build_store(*alloca, val)?;
                        }
                        if !terminates {
                            self.builder.build_unconditional_branch(exit_bb)?;
                        }
                    }

                    self.builder.position_at_end(exit_bb);
                    match result_alloca {
                        Some((Ok(alloca), llvm_ty)) => {
                            let result = self.builder.build_load(llvm_ty, alloca, "if_result")?;
                            Ok(Some(result))
                        }
                        _ => Ok(None),
                    }
                }
            }
            Expression::Do {
                body,
                catch_clauses,
                finally_body,
                ty,
                ..
            } => {
                let saved_error_ptr = *self.error_ptr.borrow();
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                let do_error = self.builder.build_alloca(ptr_ty, "do_err")?;
                self.builder.build_store(do_error, ptr_ty.const_null())?;
                *self.error_ptr.borrow_mut() = Some(do_error);

                let result_alloca = ty.as_ref().and_then(|t| {
                    if matches!(&*t.borrow(), Type::Void) {
                        return None;
                    }
                    self.resolve_type(t.clone())
                        .ok()
                        .map(|llvm_ty| (self.builder.build_alloca(llvm_ty, "do_result"), llvm_ty))
                });

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|bb| Some(bb.get_parent().unwrap()));
                let exit_bb = current_fn.map(|f| self.context.append_basic_block(f, "do_exit"));

                let has_yield_target = result_alloca.is_some() && exit_bb.is_some();
                if has_yield_target {
                    if let Some((Ok(alloca), _)) = result_alloca.as_ref() {
                        self.yield_targets
                            .borrow_mut()
                            .push((*alloca, exit_bb.unwrap()));
                    }
                }

                self.enter_scope_with_stmts(body)?;
                let len = body.len();
                let mut last_value = None;
                let mut terminated_by_return = false;
                let mut terminated_by_yield = false;
                for (i, stmt) in body.iter().enumerate() {
                    let is_last = i == len - 1;
                    let terminates = match &*stmt.borrow() {
                        Statement::ExpressionStatement { expression } => {
                            let val = self.resolve_expression(expression.clone())?;
                            if is_last {
                                last_value = val;
                            }
                            false
                        }
                        _ => self.resolve_statement(stmt.clone())?,
                    };
                    if terminates {
                        if matches!(&*stmt.borrow(), Statement::Yield { .. }) {
                            terminated_by_yield = true;
                            break;
                        }
                        terminated_by_return = true;
                        break;
                    }
                }

                if has_yield_target {
                    self.yield_targets.borrow_mut().pop();
                }

                let has_catch = !catch_clauses.is_empty();
                let has_finally = !finally_body.is_empty();

                if has_catch || has_finally {
                    let do_err_val = self.builder.build_load(ptr_ty, do_error, "do_err_val")?;
                    let err_int = self.builder.build_ptr_to_int(
                        do_err_val.into_pointer_value(),
                        self.context.i64_type(),
                        "do_err_int",
                    )?;
                    let err_is_null = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        err_int,
                        self.context.i64_type().const_zero(),
                        "do_err_is_null",
                    )?;

                    let fn_val = current_fn.unwrap();
                    let catch_bb = if has_catch {
                        Some(self.context.append_basic_block(fn_val, "do_catch"))
                    } else {
                        None
                    };
                    let finally_bb = if has_finally {
                        Some(self.context.append_basic_block(fn_val, "do_finally"))
                    } else {
                        None
                    };
                    let after_do_bb = self.context.append_basic_block(fn_val, "do_after");

                    if !terminated_by_return && !terminated_by_yield {
                        if has_catch {
                            self.builder.build_conditional_branch(
                                err_is_null,
                                after_do_bb,
                                catch_bb.unwrap(),
                            )?;
                        } else {
                            self.builder.build_unconditional_branch(after_do_bb)?;
                        }
                    }

                    if let Some(catch_bb) = catch_bb {
                        self.builder.position_at_end(catch_bb);
                        for clause in catch_clauses {
                            if let Some(guard) = &clause.guard {
                                let guard_val = self.resolve_expression(guard.clone())?.unwrap();
                                let guard_bool = match guard_val {
                                    BasicValueEnum::IntValue(i) => i,
                                    _ => self.builder.build_int_z_extend(
                                        guard_val.into_int_value(),
                                        self.context.bool_type(),
                                        "guard_bool",
                                    )?,
                                };
                                let clause_cont =
                                    self.context.append_basic_block(fn_val, "catch_clause_cont");
                                let clause_body_bb =
                                    self.context.append_basic_block(fn_val, "catch_clause_body");
                                self.builder.build_conditional_branch(
                                    guard_bool,
                                    clause_body_bb,
                                    clause_cont,
                                )?;
                                self.builder.position_at_end(clause_body_bb);
                            }
                            self.enter_scope_with_stmts(&clause.body)?;
                            for s in &clause.body {
                                let _ = self.resolve_statement(s.clone());
                            }
                            self.builder.build_unconditional_branch(after_do_bb)?;
                        }
                        if !saved_error_ptr.is_none() {
                            if let Some(outer_err) = saved_error_ptr {
                                self.builder.build_store(outer_err, do_err_val)?;
                            }
                        }
                        let ret_type = fn_val.get_type().get_return_type();
                        if let Some(ret_ty) = ret_type {
                            self.builder.build_return(Some(&ret_ty.const_zero()))?;
                        } else {
                            self.builder.build_return(None)?;
                        }
                    }

                    if let Some(finally_bb) = finally_bb {
                        self.builder.position_at_end(finally_bb);
                        self.enter_scope_with_stmts(finally_body)?;
                        for s in finally_body {
                            let _ = self.resolve_statement(s.clone());
                        }
                        if has_catch {
                            let finally_err_val =
                                self.builder
                                    .build_load(ptr_ty, do_error, "finally_err_val")?;
                            let fin_err_int = self.builder.build_ptr_to_int(
                                finally_err_val.into_pointer_value(),
                                self.context.i64_type(),
                                "finally_err_int",
                            )?;
                            let fin_err_is_null = self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                fin_err_int,
                                self.context.i64_type().const_zero(),
                                "finally_err_is_null",
                            )?;
                            let fin_nocatch_bb =
                                self.context.append_basic_block(fn_val, "finally_nocatch");
                            self.builder.build_conditional_branch(
                                fin_err_is_null,
                                after_do_bb,
                                fin_nocatch_bb,
                            )?;
                            self.builder.position_at_end(fin_nocatch_bb);
                            if let Some(outer_err) = saved_error_ptr {
                                self.builder.build_store(outer_err, finally_err_val)?;
                            }
                            let ret_type = fn_val.get_type().get_return_type();
                            if let Some(ret_ty) = ret_type {
                                self.builder.build_return(Some(&ret_ty.const_zero()))?;
                            } else {
                                self.builder.build_return(None)?;
                            }
                        } else {
                            self.builder.build_unconditional_branch(after_do_bb)?;
                        }
                    }

                    self.builder.position_at_end(after_do_bb);
                }

                *self.error_ptr.borrow_mut() = saved_error_ptr;

                if !terminated_by_return
                    && !terminated_by_yield
                    && !has_yield_target
                    && !has_catch
                    && !has_finally
                {
                    self.exit_scope();
                    return Ok(None);
                }

                if has_yield_target {
                    let (alloca_result, llvm_ty) = result_alloca.unwrap();
                    let exit = exit_bb.unwrap();
                    if !terminated_by_return && !terminated_by_yield {
                        if let Some(val) = last_value {
                            if let Ok(ref alloca_ptr) = alloca_result {
                                self.builder.build_store(*alloca_ptr, val)?;
                            }
                        }
                        self.builder.build_unconditional_branch(exit)?;
                    }
                    if terminated_by_return {
                        self.exit_scope();
                        return Ok(None);
                    }
                    self.builder.position_at_end(exit);
                    match alloca_result {
                        Ok(alloca_ptr) => {
                            let result =
                                self.builder.build_load(llvm_ty, alloca_ptr, "do_result")?;
                            self.exit_scope();
                            Ok(Some(result))
                        }
                        _ => {
                            self.exit_scope();
                            Ok(None)
                        }
                    }
                } else {
                    self.exit_scope();
                    Ok(None)
                }
            }
            Expression::Cast {
                expression,
                target_type,
                kind,
                ..
            } => {
                let source_val = self.resolve_expression(expression.clone())?.unwrap();
                let target_ty = target_type
                    .borrow()
                    .get_ty_ref()?
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No target type"))?
                    .clone();

                let target_llvm_ty = self.resolve_type(target_ty.clone())?;

                let result = match kind {
                    CastKind::ForceBitcast => {
                        self.builder
                            .build_bit_cast(source_val, target_llvm_ty, "")?
                    }
                    _ => match source_val {
                        BasicValueEnum::IntValue(src) => match target_llvm_ty {
                            BasicTypeEnum::IntType(dst_ty) => {
                                if src.get_type().get_bit_width() < dst_ty.get_bit_width() {
                                    self.builder.build_int_z_extend(src, dst_ty, "")?.into()
                                } else if src.get_type().get_bit_width() > dst_ty.get_bit_width() {
                                    self.builder.build_int_truncate(src, dst_ty, "")?.into()
                                } else {
                                    src.into()
                                }
                            }
                            BasicTypeEnum::FloatType(dst_ty) => self
                                .builder
                                .build_signed_int_to_float(src, dst_ty, "")?
                                .into(),
                            _ => src.into(),
                        },
                        BasicValueEnum::FloatValue(src) => match target_llvm_ty {
                            BasicTypeEnum::IntType(dst_ty) => self
                                .builder
                                .build_float_to_signed_int(src, dst_ty, "")?
                                .into(),
                            BasicTypeEnum::FloatType(dst_ty) => {
                                if src.get_type() == dst_ty {
                                    src.into()
                                } else {
                                    self.builder.build_float_trunc(src, dst_ty, "")?.into()
                                }
                            }
                            _ => src.into(),
                        },
                        _ => source_val,
                    },
                };

                Ok(Some(result))
            }
            Expression::OptionalChain {
                token: _,
                object,
                member,
                ..
            } => {
                let object_val = self.resolve_expression(object.clone())?.unwrap();
                let object_ty = object.borrow().get_ty_ref()?.clone();
                let enum_name = match &*object_ty
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No type for optional chain object"))?
                    .borrow()
                {
                    Type::Enum(name, _, _) => name.clone(),
                    _ => anyhow::bail!("Optional chain requires enum type"),
                };
                let enum_type = self
                    .enum_types
                    .borrow()
                    .get(&enum_name)
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
                let payloads_type = self
                    .enum_payload_types
                    .borrow()
                    .get(&enum_name)
                    .copied()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Enum payload type '{}' not found", enum_name)
                    })?;
                let alloca = self
                    .builder
                    .build_alloca(enum_type.as_basic_type_enum(), "")?;
                self.builder.build_store(alloca, object_val)?;
                let tag_ptr = self.builder.build_struct_gep(enum_type, alloca, 0, "")?;
                let tag = self
                    .builder
                    .build_load(self.context.i8_type(), tag_ptr, "")?;
                let none_tag = self.context.i8_type().const_zero();
                let is_none = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag.into_int_value(),
                    none_tag,
                    "",
                )?;
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let some_bb = self.context.append_basic_block(fn_val, "chain_some");
                let none_bb = self.context.append_basic_block(fn_val, "chain_none");
                let cont_bb = self.context.append_basic_block(fn_val, "chain_cont");
                self.builder
                    .build_conditional_branch(is_none, none_bb, some_bb)?;
                self.builder.position_at_end(none_bb);
                let none_payload_type = payloads_type
                    .get_field_type_at_index(0)
                    .ok_or_else(|| anyhow::anyhow!("None payload not found"))?;
                let none_payload = none_payload_type.const_zero();
                let none_enum_val =
                    enum_type.const_named_struct(&[none_tag.into(), none_payload.into()]);
                self.builder.build_unconditional_branch(cont_bb)?;
                self.builder.position_at_end(some_bb);
                let payload_union_ptr = self.builder.build_struct_gep(enum_type, alloca, 1, "")?;
                let some_payload_ptr =
                    self.builder
                        .build_struct_gep(payloads_type, payload_union_ptr, 1, "")?;
                let some_payload_ty = payloads_type
                    .get_field_type_at_index(1)
                    .ok_or_else(|| anyhow::anyhow!("Some payload field not found"))?
                    .into_struct_type();
                let struct_val = self
                    .builder
                    .build_load(some_payload_ty, some_payload_ptr, "")?;
                let struct_alloca = self.builder.build_alloca(some_payload_ty, "")?;
                self.builder.build_store(struct_alloca, struct_val)?;
                let inner_struct_name = match &*object_ty
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No type"))?
                    .borrow()
                {
                    Type::Enum(_, _, params) if !params.is_empty() => match &*params[0].borrow() {
                        Type::Struct(n, _, _) | Type::Class(n, _, _) => n.clone(),
                        _ => anyhow::bail!("Unsupported inner type for optional chaining"),
                    },
                    _ => anyhow::bail!("Optional without params"),
                };
                let field_index =
                    self.get_stored_struct_field_index(&inner_struct_name, &member.value)?;
                let struct_llvm_type = *self.struct_types.borrow().get(&inner_struct_name).unwrap();
                let field_ptr = self.builder.build_struct_gep(
                    struct_llvm_type,
                    struct_alloca,
                    field_index as u32,
                    "",
                )?;
                let field_ty = self.get_struct_field_type(&inner_struct_name, &member.value)?;
                let field_val = self.builder.build_load(field_ty, field_ptr, "")?;
                let field_alloca = self.builder.build_alloca(field_ty, "")?;
                self.builder.build_store(field_alloca, field_val)?;
                let field_loaded = self.builder.build_load(field_ty, field_alloca, "")?;
                let some_payload_struct_for_result =
                    some_payload_ty.const_named_struct(&[field_loaded.into()]);
                let some_tag = self.context.i8_type().const_int(1, false);
                let some_enum_val = enum_type
                    .const_named_struct(&[some_tag.into(), some_payload_struct_for_result.into()]);
                self.builder.build_unconditional_branch(cont_bb)?;
                self.builder.position_at_end(cont_bb);
                let phi = self.builder.build_phi(enum_type.as_basic_type_enum(), "")?;
                phi.add_incoming(&[(&none_enum_val, none_bb), (&some_enum_val, some_bb)]);
                let result = phi.as_basic_value();
                Ok(Some(result))
            }
            Expression::MemberAccess { object, member, .. } => {
                let object_expr = object.borrow();
                let object_ty = object_expr.get_ty_ref()?.clone();
                drop(object_expr);

                if let Some(ty) = &object_ty
                    && let Type::Struct(struct_name, ..) = &*ty.borrow()
                {
                    let struct_name = struct_name.clone();
                    let field_name = member.value.clone();

                    let object_val = self.resolve_expression(object.clone())?.unwrap();

                    let struct_ptr = if let BasicValueEnum::PointerValue(ptr) = object_val {
                        ptr
                    } else {
                        let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                        self.builder.build_store(ptr, object_val)?;
                        ptr
                    };

                    let getter_name =
                        self.mangle_fn_name(&format!("{}.{}.getter", struct_name, field_name), &[]);
                    if let Some(getter_fn) = self.module.get_function(&getter_name) {
                        let result =
                            self.builder
                                .build_call(getter_fn, &[struct_ptr.into()], "")?;
                        let result_val = match result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(val) => val,
                            _ => anyhow::bail!("Getter call did not return a value"),
                        };
                        return Ok(Some(result_val));
                    }

                    let field_index =
                        self.get_stored_struct_field_index(&struct_name, &field_name)?;

                    let field_ptr = self.builder.build_struct_gep(
                        *self.struct_types.borrow().get(&struct_name).unwrap(),
                        struct_ptr,
                        field_index as u32,
                        "",
                    )?;

                    let field_ty = self.get_struct_field_type(&struct_name, &field_name)?;
                    let field_val = self.builder.build_load(field_ty, field_ptr, "")?;
                    return Ok(Some(field_val));
                }

                if let Some(ty) = &object_ty
                    && let Type::Class(class_name, ..) = &*ty.borrow()
                {
                    let class_name = class_name.clone();
                    let field_name = member.value.clone();

                    let object_val = self.resolve_expression(object.clone())?.unwrap();

                    let class_ptr = if let BasicValueEnum::PointerValue(ptr) = object_val {
                        ptr
                    } else {
                        let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                        self.builder.build_store(ptr, object_val)?;
                        ptr
                    };

                    let getter_entry = format!("{}.getter", field_name);
                    let is_super = matches!(&*object.borrow(), Expression::SuperKeyword { .. });
                    if !is_super {
                        if let Some(slot_idx) =
                            self.get_vtable_slot_index(&class_name, &getter_entry)
                        {
                            let class_type = *self.class_types.borrow().get(&class_name).unwrap();
                            let vtable_ptr_ptr = self
                                .builder
                                .build_struct_gep(class_type, class_ptr, 0, "")?;
                            let vtable_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                    vtable_ptr_ptr,
                                    "",
                                )?
                                .into_pointer_value();

                            let vtable_type = *self.vtable_types.borrow().get(&class_name).unwrap();
                            let fn_ptr_ptr = self.builder.build_struct_gep(
                                vtable_type,
                                vtable_ptr,
                                slot_idx,
                                "",
                            )?;
                            let fn_ptr_val = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                    fn_ptr_ptr,
                                    "",
                                )?
                                .into_pointer_value();

                            let method_list = self.compute_vtable_method_list(&class_name);
                            let (_, owner) = method_list
                                .iter()
                                .find(|(n, _)| n == &getter_entry)
                                .unwrap();
                            let declared_fn_name = self
                                .mangle_fn_name(&format!("{}.{}.getter", owner, field_name), &[]);
                            let declared_fn =
                                self.module.get_function(&declared_fn_name).ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Getter function {} not found",
                                        declared_fn_name
                                    )
                                })?;
                            let fn_type = declared_fn.get_type();

                            let result = self.builder.build_indirect_call(
                                fn_type,
                                fn_ptr_val,
                                &[class_ptr.into()],
                                "",
                            )?;
                            let result_val = match result.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(val) => val,
                                _ => anyhow::bail!("Getter call did not return a value"),
                            };
                            return Ok(Some(result_val));
                        }
                    }

                    let field_index =
                        self.get_stored_class_field_index(&class_name, &field_name)?;

                    let field_ptr = self.builder.build_struct_gep(
                        *self.class_types.borrow().get(&class_name).unwrap(),
                        class_ptr,
                        field_index as u32,
                        "",
                    )?;

                    let field_ty = self.get_struct_field_type(&class_name, &field_name)?;
                    let field_val = self.builder.build_load(field_ty, field_ptr, "")?;
                    return Ok(Some(field_val));
                }

                if let Some(ty) = &object_ty {
                    let is_compound_protocol = matches!(&*ty.borrow(), Type::Compound(types) if types.iter().all(|t| matches!(&*t.borrow(), Type::Protocol(..))));
                    if is_compound_protocol {
                        if let Type::Compound(types) = &*ty.borrow() {
                            let names = self.get_compound_protocol_names(types);
                            let compound_key = self.build_compound_protocol_key(&names);
                            let field_name = member.value.clone();
                            let object_val = self.resolve_expression(object.clone())?.unwrap();

                            let ptr = if let BasicValueEnum::PointerValue(p) = object_val {
                                p
                            } else {
                                let p = self.builder.build_alloca(object_val.get_type(), "")?;
                                self.builder.build_store(p, object_val)?;
                                p
                            };

                            if let Some(ct) = self
                                .existential_container_types
                                .borrow()
                                .get(&compound_key)
                                .copied()
                            {
                                let value_ptr_ptr =
                                    self.builder.build_struct_gep(ct, ptr, 0, "")?;
                                let value_ptr = self
                                    .builder
                                    .build_load(
                                        self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                        value_ptr_ptr,
                                        "",
                                    )?
                                    .into_pointer_value();

                                for (slot, pname) in names.iter().enumerate() {
                                    let entries =
                                        self.compute_protocol_witness_table_entries(pname);
                                    let getter_entry = format!("{}.getter", field_name);
                                    if let Some(slot_idx) = entries
                                        .iter()
                                        .position(|(n, _)| n == &getter_entry)
                                        .map(|i| i as u32)
                                    {
                                        if let Some(wt) = self
                                            .protocol_witness_table_types
                                            .borrow()
                                            .get(pname)
                                            .copied()
                                        {
                                            let wt_ptr_ptr = self.builder.build_struct_gep(
                                                ct,
                                                ptr,
                                                (slot + 1) as u32,
                                                "",
                                            )?;
                                            let wt_ptr = self
                                                .builder
                                                .build_load(
                                                    self.context
                                                        .ptr_type(inkwell::AddressSpace::from(0)),
                                                    wt_ptr_ptr,
                                                    "",
                                                )?
                                                .into_pointer_value();

                                            let fn_ptr_ptr = self
                                                .builder
                                                .build_struct_gep(wt, wt_ptr, slot_idx, "")?;
                                            let fn_ptr_val = self
                                                .builder
                                                .build_load(
                                                    self.context
                                                        .ptr_type(inkwell::AddressSpace::from(0)),
                                                    fn_ptr_ptr,
                                                    "",
                                                )?
                                                .into_pointer_value();

                                            let getter_name = self.mangle_fn_name(
                                                &format!("{}.{}.getter", pname, field_name),
                                                &[],
                                            );
                                            if let Some(getter_fn) = self
                                                .module
                                                .get_function(&getter_name)
                                                .or_else(|| {
                                                    self.module.get_function(&format!(
                                                        "some.{}.getter",
                                                        getter_name
                                                    ))
                                                })
                                            {
                                                let fn_type = getter_fn.get_type();
                                                let result = self.builder.build_indirect_call(
                                                    fn_type,
                                                    fn_ptr_val,
                                                    &[value_ptr.into()],
                                                    "",
                                                )?;
                                                let result_val = match result.try_as_basic_value() {
                                                    inkwell::values::ValueKind::Basic(val) => val,
                                                    _ => anyhow::bail!(
                                                        "Getter call did not return a value"
                                                    ),
                                                };
                                                return Ok(Some(result_val));
                                            }
                                        }
                                    }
                                }
                            }
                            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                            return Ok(Some(ptr_ty.const_null().into()));
                        }
                    }
                }

                if let Some(ty) = &object_ty
                    && let Type::Protocol(protocol_name, ..) = &*ty.borrow()
                {
                    let protocol_name = protocol_name.clone();
                    let field_name = member.value.clone();
                    let object_val = self.resolve_expression(object.clone())?.unwrap();

                    let ptr = if let BasicValueEnum::PointerValue(p) = object_val {
                        p
                    } else {
                        let p = self.builder.build_alloca(object_val.get_type(), "")?;
                        self.builder.build_store(p, object_val)?;
                        p
                    };

                    let ct = self
                        .existential_container_types
                        .borrow()
                        .get(&protocol_name)
                        .copied();
                    let wt_type = self
                        .protocol_witness_table_types
                        .borrow()
                        .get(&protocol_name)
                        .copied();
                    let entries = self.compute_protocol_witness_table_entries(&protocol_name);

                    if let (Some(ct), Some(wt)) = (ct, wt_type) {
                        let value_ptr_ptr = self.builder.build_struct_gep(ct, ptr, 0, "")?;
                        let value_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                value_ptr_ptr,
                                "",
                            )?
                            .into_pointer_value();

                        let wt_ptr_ptr = self.builder.build_struct_gep(ct, ptr, 1, "")?;
                        let wt_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                wt_ptr_ptr,
                                "",
                            )?
                            .into_pointer_value();

                        let getter_entry = format!("{}.getter", field_name);
                        if let Some(slot_idx) = entries
                            .iter()
                            .position(|(n, _)| n == &getter_entry)
                            .map(|i| i as u32)
                        {
                            let fn_ptr_ptr =
                                self.builder.build_struct_gep(wt, wt_ptr, slot_idx, "")?;
                            let fn_ptr_val = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                    fn_ptr_ptr,
                                    "",
                                )?
                                .into_pointer_value();

                            let getter_name = self.mangle_fn_name(
                                &format!("{}.{}.getter", protocol_name, field_name),
                                &[],
                            );
                            if let Some(getter_fn) =
                                self.module.get_function(&getter_name).or_else(|| {
                                    self.module
                                        .get_function(&format!("some.{}.getter", getter_name))
                                })
                            {
                                let fn_type = getter_fn.get_type();
                                let result = self.builder.build_indirect_call(
                                    fn_type,
                                    fn_ptr_val,
                                    &[value_ptr.into()],
                                    "",
                                )?;
                                let result_val = match result.try_as_basic_value() {
                                    inkwell::values::ValueKind::Basic(val) => val,
                                    _ => anyhow::bail!("Getter call did not return a value"),
                                };
                                return Ok(Some(result_val));
                            }
                        }
                    }

                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                    return Ok(Some(ptr_ty.const_null().into()));
                }

                if let Some(ty) = &object_ty
                    && let Type::Enum(enum_name, ..) = &*ty.borrow()
                {
                    let case_name = member.value.clone();
                    let enum_name = enum_name.clone();
                    let case_index = self.get_enum_case_index(&enum_name, &case_name)?;

                    let enum_types = self.enum_types.borrow();
                    let enum_llvm_type = enum_types
                        .get(&enum_name)
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
                    drop(enum_types);

                    if let Some(raw_llvm_type) = self.get_enum_raw_llvm_type(&enum_name) {
                        let raw_val = raw_llvm_type
                            .into_int_type()
                            .const_int(case_index as u64, false);
                        let alloca = self
                            .builder
                            .build_alloca(enum_llvm_type.as_basic_type_enum(), "")?;
                        let raw_ptr =
                            self.builder
                                .build_struct_gep(enum_llvm_type, alloca, 0, "")?;
                        self.builder.build_store(raw_ptr, raw_val)?;
                        let val = self.builder.build_load(
                            enum_llvm_type.as_basic_type_enum(),
                            alloca,
                            "",
                        )?;
                        return Ok(Some(val));
                    }

                    let alloca = self
                        .builder
                        .build_alloca(enum_llvm_type.as_basic_type_enum(), "")?;

                    let tag_ptr = self
                        .builder
                        .build_struct_gep(enum_llvm_type, alloca, 0, "")?;
                    let tag_val = self.context.i8_type().const_int(case_index as u64, false);
                    self.builder.build_store(tag_ptr, tag_val)?;

                    let val =
                        self.builder
                            .build_load(enum_llvm_type.as_basic_type_enum(), alloca, "")?;
                    return Ok(Some(val));
                }

                if let Some(ty) = &object_ty
                    && let Type::Tuple(elements) = &*ty.borrow()
                {
                    let object_val = self.resolve_expression(object.clone())?.unwrap();
                    let tuple_llvm = self
                        .resolve_type(object_ty.clone().unwrap())?
                        .into_struct_type();

                    let struct_ptr = match object_val {
                        BasicValueEnum::PointerValue(ptr) => ptr,
                        val => {
                            let ptr = self.builder.build_alloca(val.get_type(), "")?;
                            self.builder.build_store(ptr, val)?;
                            ptr
                        }
                    };

                    let member_name = &member.value;
                    let idx = if let Some(idx) = elements
                        .iter()
                        .position(|(n, _)| n.as_ref().map_or(false, |name| name == member_name))
                    {
                        idx
                    } else if let Ok(numeric_idx) = member_name.parse::<usize>() {
                        if numeric_idx < elements.len() {
                            numeric_idx
                        } else {
                            anyhow::bail!("Tuple index {} out of bounds", numeric_idx);
                        }
                    } else {
                        anyhow::bail!("Field '{}' not found on tuple", member_name);
                    };

                    let element_ty = self.resolve_type(elements[idx].1.clone())?;
                    let field_ptr = self
                        .builder
                        .build_struct_gep(tuple_llvm, struct_ptr, idx as u32, "")?;
                    let val = self.builder.build_load(element_ty, field_ptr, "")?;
                    return Ok(Some(val));
                }

                if let Some(ty) = &object_ty
                    && let Type::Inline(inner, _) = &*ty.borrow()
                    && let Type::Class(class_name, ..) = &*inner.borrow()
                {
                    let object_val = self.resolve_expression(object.clone())?.unwrap();
                    let class_type = self.class_types.borrow().get(class_name).copied();
                    if let Some(class_type) = class_type {
                        let struct_ptr = match object_val {
                            BasicValueEnum::PointerValue(p) => p,
                            val => {
                                let p = self.builder.build_alloca(val.get_type(), "")?;
                                self.builder.build_store(p, val)?;
                                p
                            }
                        };
                        let field_index =
                            self.get_stored_class_field_index(class_name, &member.value);
                        if let Ok(field_index) = field_index {
                            let field_llvm_ptr = self.builder.build_struct_gep(
                                class_type,
                                struct_ptr,
                                field_index as u32,
                                "",
                            )?;
                            let field_llvm_type =
                                self.get_struct_field_type(class_name, &member.value)?;
                            let val =
                                self.builder
                                    .build_load(field_llvm_type, field_llvm_ptr, "")?;
                            return Ok(Some(val));
                        }
                    }
                    self.emit_error(
                        TrussDiagnosticCode::FieldNotFound,
                        format!(
                            "Field '{}' not found on inline class '{}'",
                            member.value, class_name
                        ),
                        Some(member.as_ref()),
                    );
                    anyhow::bail!(
                        "Field '{}' not found on inline class '{}'",
                        member.value,
                        class_name
                    );
                }

                self.emit_error(
                    TrussDiagnosticCode::UnsupportedFeature,
                    "Member access on non-struct type",
                    Some(member.as_ref()),
                );
                anyhow::bail!("Member access on non-struct type");
            }
            Expression::SubscriptAccess {
                object,
                parameters,
                ..
            } => {
                let object_ty = {
                    let obj = object.borrow();
                    obj.get_ty_ref()?.clone()
                };
                if let Some(ty) = &object_ty
                    && let Type::Struct(struct_name, ..) = &*ty.borrow()
                {
                    let struct_name = struct_name.clone();
                    let object_val = self.resolve_expression(object.clone())?.unwrap();
                    let struct_ptr = if let BasicValueEnum::PointerValue(ptr) = object_val {
                        ptr
                    } else {
                        let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                        self.builder.build_store(ptr, object_val)?;
                        ptr
                    };
                    let getter_name = self
                        .mangled_fn_names
                        .borrow()
                        .get(&format!("{}.subscript.getter", struct_name))
                        .cloned()
                        .unwrap_or_else(|| {
                            self.mangle_fn_name(
                                &format!("{}.subscript.getter", struct_name),
                                &[],
                            )
                        });
                    if let Some(getter_fn) = self.module.get_function(&getter_name) {
                        let mut args = vec![struct_ptr.into()];
                        for p in parameters {
                            let arg_val = self.resolve_expression(p.expression.clone())?.unwrap();
                            args.push(arg_val.into());
                        }
                        let result = self.builder.build_call(getter_fn, &args, "")?;
                        let result_val = match result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(val) => val,
                            _ => anyhow::bail!("Subscript getter call did not return a value"),
                        };
                        return Ok(Some(result_val));
                    }
                }
                if let Some(ty) = &object_ty
                    && let Type::Class(class_name, ..) = &*ty.borrow()
                {
                    let class_name = class_name.clone();
                    let object_val = self.resolve_expression(object.clone())?.unwrap();
                    let class_ptr = if let BasicValueEnum::PointerValue(ptr) = object_val {
                        ptr
                    } else {
                        let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                        self.builder.build_store(ptr, object_val)?;
                        ptr
                    };
                    let getter_entry = "subscript.getter";
                    if let Some(slot_idx) =
                        self.get_vtable_slot_index(&class_name, getter_entry)
                    {
                        let class_type = *self.class_types.borrow().get(&class_name).unwrap();
                        let vtable_ptr_ptr = self
                            .builder
                            .build_struct_gep(class_type, class_ptr, 0, "")?;
                        let vtable_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                vtable_ptr_ptr,
                                "",
                            )?
                            .into_pointer_value();
                        let vtable_type = *self.vtable_types.borrow().get(&class_name).unwrap();
                        let fn_ptr_ptr = self.builder.build_struct_gep(
                            vtable_type,
                            vtable_ptr,
                            slot_idx,
                            "",
                        )?;
                        let fn_ptr_val = self
                            .builder
                            .build_load(
                                self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                fn_ptr_ptr,
                                "",
                            )?
                            .into_pointer_value();
                        let method_list = self.compute_vtable_method_list(&class_name);
                        let (_, owner) = method_list
                            .iter()
                            .find(|(n, _)| n == &getter_entry)
                            .unwrap();
                        let declared_fn_name = self
                            .mangled_fn_names
                            .borrow()
                            .get(&format!("{}.subscript.getter", owner))
                            .cloned()
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Subscript getter function '{}' not found",
                                    format!("{}.subscript.getter", owner)
                                )
                            })?;
                        let declared_fn =
                            self.module.get_function(&declared_fn_name).ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Subscript getter LLVM function {} not found",
                                    declared_fn_name
                                )
                            })?;
                        let fn_type = declared_fn.get_type();
                        let mut args = vec![class_ptr.into()];
                        for p in parameters {
                            let arg_val = self.resolve_expression(p.expression.clone())?.unwrap();
                            args.push(arg_val.into());
                        }
                        let result = self.builder.build_indirect_call(fn_type, fn_ptr_val, &args, "")?;
                        let result_val = match result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(val) => val,
                            _ => anyhow::bail!("Subscript getter call did not return a value"),
                        };
                        return Ok(Some(result_val));
                    }
                }
                Ok(None)
            }
            Expression::Call {
                callee,
                parameters,
                overloads,
                selected_index,
                ..
            } => {
                if let Expression::ImplicitMemberAccess { member, .. } = &*callee.borrow() {
                    let callee_ty = callee.borrow().get_ty_ref()?.clone();
                    let enum_type = if let Some(t) = &callee_ty
                        && let Type::Function(_, ret_ty, _, _) = &*t.borrow()
                    {
                        Some(ret_ty.clone())
                    } else if let Some(t) = &callee_ty {
                        Some(t.clone())
                    } else {
                        None
                    };
                    if let Some(et) = &enum_type
                        && let Type::Enum(enum_name, ..) = &*et.borrow()
                    {
                        let case_name = member.value.clone();
                        let enum_name = enum_name.clone();
                        let case_index = self.get_enum_case_index(&enum_name, &case_name)?;
                        let enum_types = self.enum_types.borrow();
                        let case_llvm_type =
                            enum_types.get(&enum_name).copied().ok_or_else(|| {
                                anyhow::anyhow!("Enum type '{}' not found", enum_name)
                            })?;
                        drop(enum_types);
                        let alloca = self
                            .builder
                            .build_alloca(case_llvm_type.as_basic_type_enum(), "")?;
                        let tag_ptr =
                            self.builder
                                .build_struct_gep(case_llvm_type, alloca, 0, "")?;
                        let tag_val = self.context.i8_type().const_int(case_index as u64, false);
                        self.builder.build_store(tag_ptr, tag_val)?;
                        if !parameters.is_empty() {
                            let payload_ptr =
                                self.builder
                                    .build_struct_gep(case_llvm_type, alloca, 1, "")?;
                            let enum_payloads = self.enum_payload_types.borrow();
                            if let Some(payload_type) = enum_payloads.get(&enum_name) {
                                for (i, param) in parameters.iter().enumerate() {
                                    let field_ptr = self.builder.build_struct_gep(
                                        *payload_type,
                                        payload_ptr,
                                        i as u32,
                                        "",
                                    )?;
                                    let arg_val =
                                        self.resolve_expression(param.expression.clone())?.unwrap();
                                    self.builder.build_store(field_ptr, arg_val)?;
                                }
                            }
                        }
                        let val = self.builder.build_load(
                            case_llvm_type.as_basic_type_enum(),
                            alloca,
                            "",
                        )?;
                        return Ok(Some(val));
                    }
                }
                let mut method_self_ptr: Option<PointerValue<'ctx>> = None;
                let (function_name, is_init_call) = match &*callee.borrow() {
                    Expression::Variable { name, .. } => {
                        let name = name.value.clone();
                        if let Some(idx) = *selected_index
                            && idx < overloads.len()
                        {
                            if let Some(mangled) =
                                self.mangle_from_overload(&name, &overloads[idx], parameters)
                            {
                                (mangled, false)
                            } else if self.module.get_function(&name).is_some() {
                                (name, false)
                            } else {
                                (self.mangle_fn_name(&format!("{}.init", name), &[]), true)
                            }
                        } else if self.module.get_function(&name).is_some()
                            || self.mangled_fn_names.borrow().contains_key(&name)
                        {
                            let mangled = self
                                .mangled_fn_names
                                .borrow()
                                .get(&name)
                                .cloned()
                                .unwrap_or(name);
                            (mangled, false)
                        } else {
                            let init_name = format!("{}.init", name);
                            let mangled = self
                                .mangled_fn_names
                                .borrow()
                                .get(&init_name)
                                .cloned()
                                .unwrap_or_else(|| self.mangle_fn_name(&init_name, &[]));
                            (mangled, true)
                        }
                    }
                    Expression::MemberAccess { object, member, .. } => {
                        if self.is_module_expression(object) {
                            let fn_name = member.value.clone();
                            (fn_name, false)
                        } else {
                            let object_expr = object.borrow();
                            let object_ty = object_expr.get_ty_ref()?.clone();
                            drop(object_expr);

                            if let Some(ty) = &object_ty
                                && let Type::Struct(struct_name, ..) = &*ty.borrow()
                            {
                                let object_val = self.resolve_expression(object.clone())?.unwrap();
                                let ptr = if let Expression::Variable { name, .. } =
                                    &*object.borrow()
                                {
                                    if let Some(var_ptr) = self.lookup_variable(&name.value) {
                                        var_ptr
                                    } else {
                                        if let BasicValueEnum::PointerValue(p) = object_val {
                                            p
                                        } else {
                                            let p = self.builder.build_alloca(object_val.get_type(), "")?;
                                            self.builder.build_store(p, object_val)?;
                                            p
                                        }
                                    }
                                } else if let BasicValueEnum::PointerValue(p) = object_val {
                                    p
                                } else {
                                    let p = self.builder.build_alloca(object_val.get_type(), "")?;
                                    self.builder.build_store(p, object_val)?;
                                    p
                                };
                                method_self_ptr = Some(ptr);
                                let base_name = format!("{}.{}", struct_name, member.value);
                                let fn_name = if let Some(idx) = *selected_index
                                    && idx < overloads.len()
                                {
                                    self.mangle_from_overload(
                                        &base_name,
                                        &overloads[idx],
                                        parameters,
                                    )
                                    .unwrap_or({
                                        let mangled = self
                                            .mangled_fn_names
                                            .borrow()
                                            .get(&base_name)
                                            .cloned()
                                            .unwrap_or_else(|| {
                                                self.mangle_fn_name(&base_name, &[])
                                            });
                                        mangled
                                    })
                                } else {
                                    self.mangled_fn_names
                                        .borrow()
                                        .get(&base_name)
                                        .cloned()
                                        .unwrap_or_else(|| self.mangle_fn_name(&base_name, &[]))
                                };
                                (fn_name, false)
                            } else if let Some(ty) = &object_ty
                                && let Type::Class(class_name, ..) = &*ty.borrow()
                            {
                                let object_val = self.resolve_expression(object.clone())?.unwrap();
                                let ptr = if let Expression::Variable { name, .. } =
                                    &*object.borrow()
                                {
                                    if let Some(var_ptr) = self.lookup_variable(&name.value) {
                                        var_ptr
                                    } else {
                                        if let BasicValueEnum::PointerValue(p) = object_val {
                                            p
                                        } else {
                                            let p = self.builder.build_alloca(object_val.get_type(), "")?;
                                            self.builder.build_store(p, object_val)?;
                                            p
                                        }
                                    }
                                } else if let BasicValueEnum::PointerValue(p) = object_val {
                                    p
                                } else {
                                    let p = self.builder.build_alloca(object_val.get_type(), "")?;
                                    self.builder.build_store(p, object_val)?;
                                    p
                                };
                                let method_name = &member.value;

                                let is_super =
                                    matches!(&*object.borrow(), Expression::SuperKeyword { .. });

                                if is_super {
                                    let mangled = self.mangle_fn_name(
                                        &format!("{}.{}", class_name, method_name),
                                        &[],
                                    );
                                    let declared_fn =
                                        self.module.get_function(&mangled).ok_or_else(|| {
                                            self.emit_error(
                                                TrussDiagnosticCode::UndefinedFunction,
                                                format!("Undefined function: '{}'", mangled),
                                                None,
                                            );
                                            anyhow::anyhow!("Undefined function: {}", mangled)
                                        })?;

                                    let mut args: Vec<
                                        inkwell::values::BasicMetadataValueEnum<'ctx>,
                                    > = Vec::new();
                                    args.push(ptr.into());
                                    for param in parameters {
                                        let arg_val = self
                                            .resolve_expression(param.expression.clone())?
                                            .unwrap();
                                        args.push(arg_val.into());
                                    }
                                    let call_result =
                                        self.builder.build_call(declared_fn, &args, "")?;
                                    match call_result.try_as_basic_value() {
                                        inkwell::values::ValueKind::Basic(val) => {
                                            return Ok(Some(val));
                                        }
                                        _ => return Ok(None),
                                    }
                                }

                                // Static dispatch for final class or final method
                                if self.is_final_class(class_name)
                                    || self.is_final_method(class_name, method_name)
                                {
                                    let owner = self
                                        .compute_vtable_method_list(class_name)
                                        .iter()
                                        .find(|(n, _)| n == method_name)
                                        .map(|(_, owner)| owner.clone())
                                        .unwrap_or_else(|| class_name.clone());
                                    let fn_name = self
                                        .mangle_fn_name(&format!("{}.{}", owner, method_name), &[]);
                                    if let Some(declared_fn) = self.module.get_function(&fn_name) {
                                        let mut args: Vec<
                                            inkwell::values::BasicMetadataValueEnum<'ctx>,
                                        > = Vec::new();
                                        args.push(ptr.into());
                                        for param in parameters {
                                            let arg_val = self
                                                .resolve_expression(param.expression.clone())?
                                                .unwrap();
                                            args.push(arg_val.into());
                                        }
                                        let call_result =
                                            self.builder.build_call(declared_fn, &args, "")?;
                                        match call_result.try_as_basic_value() {
                                            inkwell::values::ValueKind::Basic(val) => {
                                                return Ok(Some(val));
                                            }
                                            _ => return Ok(None),
                                        }
                                    }
                                }

                                if let Some(slot_idx) =
                                    self.get_vtable_slot_index(class_name, method_name)
                                {
                                    let class_type =
                                        *self.class_types.borrow().get(class_name).unwrap();
                                    let vtable_ptr_ptr =
                                        self.builder.build_struct_gep(class_type, ptr, 0, "")?;
                                    let vtable_ptr = self
                                        .builder
                                        .build_load(
                                            self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                            vtable_ptr_ptr,
                                            "",
                                        )?
                                        .into_pointer_value();

                                    let vtable_type =
                                        *self.vtable_types.borrow().get(class_name).unwrap();
                                    let fn_ptr_ptr = self.builder.build_struct_gep(
                                        vtable_type,
                                        vtable_ptr,
                                        slot_idx,
                                        "",
                                    )?;
                                    let fn_ptr_val = self
                                        .builder
                                        .build_load(
                                            self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                            fn_ptr_ptr,
                                            "",
                                        )?
                                        .into_pointer_value();

                                    let Some(owner) = self
                                        .compute_vtable_method_list(class_name)
                                        .iter()
                                        .find(|(n, _)| n == method_name)
                                        .map(|(_, owner)| owner.clone())
                                    else {
                                        anyhow::bail!(
                                            "Method {} not found in vtable for {}",
                                            method_name,
                                            class_name
                                        );
                                    };
                                    let fn_name = self
                                        .mangle_fn_name(&format!("{}.{}", owner, method_name), &[]);
                                    let declared_fn =
                                        self.module.get_function(&fn_name).ok_or_else(|| {
                                            self.emit_error(
                                                TrussDiagnosticCode::UndefinedFunction,
                                                format!("Undefined function: '{}'", fn_name),
                                                None,
                                            );
                                            anyhow::anyhow!("Undefined function: {}", fn_name)
                                        })?;
                                    let fn_type = declared_fn.get_type();

                                    let mut args: Vec<
                                        inkwell::values::BasicMetadataValueEnum<'ctx>,
                                    > = Vec::new();
                                    args.push(ptr.into());
                                    for param in parameters {
                                        let arg_val = self
                                            .resolve_expression(param.expression.clone())?
                                            .unwrap();
                                        args.push(arg_val.into());
                                    }
                                    let call_result = self
                                        .builder
                                        .build_indirect_call(fn_type, fn_ptr_val, &args, "")?;
                                    match call_result.try_as_basic_value() {
                                        inkwell::values::ValueKind::Basic(val) => {
                                            return Ok(Some(val));
                                        }
                                        _ => return Ok(None),
                                    }
                                }

                                method_self_ptr = Some(ptr);
                                let base_name = format!("{}.{}", class_name, method_name);
                                let fn_name = if let Some(idx) = *selected_index
                                    && idx < overloads.len()
                                {
                                    self.mangle_from_overload(
                                        &base_name,
                                        &overloads[idx],
                                        parameters,
                                    )
                                    .unwrap_or({
                                        let mangled = self
                                            .mangled_fn_names
                                            .borrow()
                                            .get(&base_name)
                                            .cloned()
                                            .unwrap_or_else(|| {
                                                self.mangle_fn_name(&base_name, &[])
                                            });
                                        mangled
                                    })
                                } else {
                                    self.mangled_fn_names
                                        .borrow()
                                        .get(&base_name)
                                        .cloned()
                                        .unwrap_or_else(|| self.mangle_fn_name(&base_name, &[]))
                                };
                                (fn_name, false)
                            } else if let Some(ty) = &object_ty
                                && (matches!(&*ty.borrow(), Type::Compound(..))
                                    || matches!(&*ty.borrow(), Type::Protocol(..)))
                            {
                                let is_compound = matches!(&*ty.borrow(), Type::Compound(types) if types.iter().all(|t| matches!(&*t.borrow(), Type::Protocol(..))));
                                if is_compound {
                                    if let Type::Compound(types) = &*ty.borrow() {
                                        let names = self.get_compound_protocol_names(types);
                                        let compound_key = self.build_compound_protocol_key(&names);
                                        let object_val =
                                            self.resolve_expression(object.clone())?.unwrap();
                                        let ptr =
                                            if let BasicValueEnum::PointerValue(p) = object_val {
                                                p
                                            } else {
                                                let alloca = self
                                                    .builder
                                                    .build_alloca(object_val.get_type(), "")?;
                                                self.builder.build_store(alloca, object_val)?;
                                                alloca
                                            };
                                        let method_name = &member.value;

                                        if let Some(ct) = self
                                            .existential_container_types
                                            .borrow()
                                            .get(&compound_key)
                                            .copied()
                                        {
                                            let value_ptr_ptr =
                                                self.builder.build_struct_gep(ct, ptr, 0, "")?;
                                            let value_ptr = self
                                                .builder
                                                .build_load(
                                                    self.context
                                                        .ptr_type(inkwell::AddressSpace::from(0)),
                                                    value_ptr_ptr,
                                                    "",
                                                )?
                                                .into_pointer_value();

                                            for (slot, pname) in names.iter().enumerate() {
                                                if let Some(wt) = self
                                                    .protocol_witness_table_types
                                                    .borrow()
                                                    .get(pname)
                                                    .copied()
                                                {
                                                    let entries = self
                                                        .compute_protocol_witness_table_entries(
                                                            pname,
                                                        );
                                                    if let Some(slot_idx) = entries
                                                        .iter()
                                                        .position(|(n, k)| {
                                                            *k == "method" && n == method_name
                                                        })
                                                        .map(|i| i as u32)
                                                    {
                                                        let wt_ptr_ptr =
                                                            self.builder.build_struct_gep(
                                                                ct,
                                                                ptr,
                                                                (slot + 1) as u32,
                                                                "",
                                                            )?;
                                                        let wt_ptr = self
                                                            .builder
                                                            .build_load(
                                                                self.context.ptr_type(
                                                                    inkwell::AddressSpace::from(0),
                                                                ),
                                                                wt_ptr_ptr,
                                                                "",
                                                            )?
                                                            .into_pointer_value();

                                                        let fn_ptr_ptr =
                                                            self.builder.build_struct_gep(
                                                                wt, wt_ptr, slot_idx, "",
                                                            )?;
                                                        let fn_ptr_val = self
                                                            .builder
                                                            .build_load(
                                                                self.context.ptr_type(
                                                                    inkwell::AddressSpace::from(0),
                                                                ),
                                                                fn_ptr_ptr,
                                                                "",
                                                            )?
                                                            .into_pointer_value();

                                                        let method_fn = self
                                                            .get_protocol_method_fn_type(
                                                                pname,
                                                                method_name,
                                                            );
                                                        if let Some(fn_type) = method_fn {
                                                            let is_throwing = self
                                                                .is_protocol_method_throwing(
                                                                    pname,
                                                                    method_name,
                                                                );
                                                            let mut args: Vec<
                                                            inkwell::values::BasicMetadataValueEnum<
                                                                'ctx,
                                                            >,
                                                        > = Vec::new();
                                                            args.push(value_ptr.into());
                                                            let err_alloca = if is_throwing {
                                                                let ptr_ty = self.context.ptr_type(
                                                                    inkwell::AddressSpace::from(0),
                                                                );
                                                                let alloca =
                                                                    self.builder.build_alloca(
                                                                        ptr_ty, "call_err",
                                                                    )?;
                                                                self.builder.build_store(
                                                                    alloca,
                                                                    ptr_ty.const_null(),
                                                                )?;
                                                                Some(alloca)
                                                            } else {
                                                                None
                                                            };
                                                            if let Some(alloca) = err_alloca {
                                                                args.push(alloca.into());
                                                            }
                                                            for param in parameters {
                                                                let arg_val = self
                                                                    .resolve_expression(
                                                                        param.expression.clone(),
                                                                    )?
                                                                    .unwrap();
                                                                args.push(arg_val.into());
                                                            }
                                                            let call_result =
                                                                self.builder.build_indirect_call(
                                                                    fn_type, fn_ptr_val, &args, "",
                                                                )?;
                                                            if is_throwing {
                                                                let ptr_ty = self.context.ptr_type(
                                                                    inkwell::AddressSpace::from(0),
                                                                );
                                                                let err_val =
                                                                    self.builder.build_load(
                                                                        ptr_ty,
                                                                        err_alloca.unwrap(),
                                                                        "call_err_val",
                                                                    )?;
                                                                if let Some(outer_err_ptr) =
                                                                    *self.error_ptr.borrow()
                                                                {
                                                                    self.builder.build_store(
                                                                        outer_err_ptr,
                                                                        err_val,
                                                                    )?;
                                                                }
                                                            }
                                                            match call_result.try_as_basic_value() {
                                                            inkwell::values::ValueKind::Basic(
                                                                val,
                                                            ) => return Ok(Some(val)),
                                                            _ => return Ok(None),
                                                        }
                                                        }
                                                    }
                                                }
                                            }
                                            method_self_ptr = Some(value_ptr);
                                            (format!("{}.{}", compound_key, method_name), false)
                                        } else {
                                            method_self_ptr = Some(ptr);
                                            (format!("{}.{}", compound_key, method_name), false)
                                        }
                                    } else {
                                        anyhow::bail!("Internal error: expected Compound type");
                                    }
                                } else if let Type::Protocol(protocol_name, ..) = &*ty.borrow() {
                                    let protocol_name = protocol_name.clone();
                                    let object_val =
                                        self.resolve_expression(object.clone())?.unwrap();
                                    let ptr = if let BasicValueEnum::PointerValue(p) = object_val {
                                        p
                                    } else {
                                        let alloca =
                                            self.builder.build_alloca(object_val.get_type(), "")?;
                                        self.builder.build_store(alloca, object_val)?;
                                        alloca
                                    };
                                    let method_name = &member.value;

                                    let container_type = self
                                        .existential_container_types
                                        .borrow()
                                        .get(&protocol_name)
                                        .copied();
                                    let wt_type = self
                                        .protocol_witness_table_types
                                        .borrow()
                                        .get(&protocol_name)
                                        .copied();
                                    let entries =
                                        self.compute_protocol_witness_table_entries(&protocol_name);

                                    if let (Some(ct), Some(wt)) = (container_type, wt_type) {
                                        let value_ptr_ptr =
                                            self.builder.build_struct_gep(ct, ptr, 0, "")?;
                                        let value_ptr = self
                                            .builder
                                            .build_load(
                                                self.context
                                                    .ptr_type(inkwell::AddressSpace::from(0)),
                                                value_ptr_ptr,
                                                "",
                                            )?
                                            .into_pointer_value();

                                        let wt_ptr_ptr =
                                            self.builder.build_struct_gep(ct, ptr, 1, "")?;
                                        let wt_ptr = self
                                            .builder
                                            .build_load(
                                                self.context
                                                    .ptr_type(inkwell::AddressSpace::from(0)),
                                                wt_ptr_ptr,
                                                "",
                                            )?
                                            .into_pointer_value();

                                        if let Some(slot_idx) = entries
                                            .iter()
                                            .position(|(n, k)| *k == "method" && n == method_name)
                                            .map(|i| i as u32)
                                        {
                                            let fn_ptr_ptr = self
                                                .builder
                                                .build_struct_gep(wt, wt_ptr, slot_idx, "")?;
                                            let fn_ptr_val = self
                                                .builder
                                                .build_load(
                                                    self.context
                                                        .ptr_type(inkwell::AddressSpace::from(0)),
                                                    fn_ptr_ptr,
                                                    "",
                                                )?
                                                .into_pointer_value();

                                            let method_fn = self.get_protocol_method_fn_type(
                                                &protocol_name,
                                                &method_name,
                                            );
                                            if let Some(fn_type) = method_fn {
                                                let is_throwing = self.is_protocol_method_throwing(
                                                    &protocol_name,
                                                    &method_name,
                                                );
                                                let mut args: Vec<
                                                    inkwell::values::BasicMetadataValueEnum<'ctx>,
                                                > = Vec::new();
                                                args.push(value_ptr.into());
                                                let err_alloca = if is_throwing {
                                                    let ptr_ty = self
                                                        .context
                                                        .ptr_type(inkwell::AddressSpace::from(0));
                                                    let alloca = self
                                                        .builder
                                                        .build_alloca(ptr_ty, "call_err")?;
                                                    self.builder
                                                        .build_store(alloca, ptr_ty.const_null())?;
                                                    Some(alloca)
                                                } else {
                                                    None
                                                };
                                                if let Some(alloca) = err_alloca {
                                                    args.push(alloca.into());
                                                }
                                                for param in parameters {
                                                    let arg_val = self
                                                        .resolve_expression(
                                                            param.expression.clone(),
                                                        )?
                                                        .unwrap();
                                                    args.push(arg_val.into());
                                                }
                                                let call_result =
                                                    self.builder.build_indirect_call(
                                                        fn_type, fn_ptr_val, &args, "",
                                                    )?;
                                                if is_throwing {
                                                    let ptr_ty = self
                                                        .context
                                                        .ptr_type(inkwell::AddressSpace::from(0));
                                                    let err_val = self.builder.build_load(
                                                        ptr_ty,
                                                        err_alloca.unwrap(),
                                                        "call_err_val",
                                                    )?;
                                                    if let Some(outer_err_ptr) =
                                                        *self.error_ptr.borrow()
                                                    {
                                                        self.builder
                                                            .build_store(outer_err_ptr, err_val)?;
                                                    }
                                                }
                                                match call_result.try_as_basic_value() {
                                                    inkwell::values::ValueKind::Basic(val) => {
                                                        return Ok(Some(val));
                                                    }
                                                    _ => return Ok(None),
                                                }
                                            }
                                        }

                                        method_self_ptr = Some(value_ptr);
                                        (format!("{}.{}", protocol_name, method_name), false)
                                    } else {
                                        method_self_ptr = Some(ptr);
                                        (format!("{}.{}", protocol_name, method_name), false)
                                    }
                                } else {
                                    anyhow::bail!("Unsupported object type for method call");
                                }
                            } else if let Some(ty) = &object_ty
                                && let Type::Enum(enum_name, ..) = &*ty.borrow()
                            {
                                let case_name = member.value.clone();
                                let enum_name = enum_name.clone();
                                let case_index =
                                    self.get_enum_case_index(&enum_name, &case_name)?;

                                let enum_types = self.enum_types.borrow();
                                let case_llvm_type =
                                    enum_types.get(&enum_name).copied().ok_or_else(|| {
                                        anyhow::anyhow!("Enum type '{}' not found", enum_name)
                                    })?;
                                drop(enum_types);

                                let alloca = self
                                    .builder
                                    .build_alloca(case_llvm_type.as_basic_type_enum(), "")?;

                                let tag_ptr =
                                    self.builder
                                        .build_struct_gep(case_llvm_type, alloca, 0, "")?;
                                let tag_val =
                                    self.context.i8_type().const_int(case_index as u64, false);
                                self.builder.build_store(tag_ptr, tag_val)?;

                                if !parameters.is_empty() {
                                    let payload_ptr = self.builder.build_struct_gep(
                                        case_llvm_type,
                                        alloca,
                                        1,
                                        "",
                                    )?;
                                    let enum_payloads = self.enum_payload_types.borrow();
                                    if let Some(payload_type) = enum_payloads.get(&enum_name) {
                                        for (i, param) in parameters.iter().enumerate() {
                                            let field_ptr = self.builder.build_struct_gep(
                                                *payload_type,
                                                payload_ptr,
                                                i as u32,
                                                "",
                                            )?;
                                            let arg_val = self
                                                .resolve_expression(param.expression.clone())?
                                                .unwrap();
                                            self.builder.build_store(field_ptr, arg_val)?;
                                        }
                                    }
                                }

                                let val = self.builder.build_load(
                                    case_llvm_type.as_basic_type_enum(),
                                    alloca,
                                    "",
                                )?;
                                return Ok(Some(val));
                            } else {
                                self.emit_error(
                                    TrussDiagnosticCode::UnsupportedFeature,
                                    "Method call on non-struct/enum type",
                                    Some(member.as_ref()),
                                );
                                anyhow::bail!("Method call on non-struct/enum type");
                            }
                        }
                    }
                    _ => {
                        if let Expression::SelfType { ty, .. } = &*callee.borrow() {
                            if let Some(ty) = ty {
                                let type_name = match &*ty.borrow() {
                                    Type::Struct(name, ..) => name.clone(),
                                    Type::Class(name, ..) => name.clone(),
                                    _ => {
                                        self.emit_error(
                                            TrussDiagnosticCode::UnsupportedFeature,
                                            "Self(...) is only supported for struct and class types",
                                            None,
                                        );
                                        anyhow::bail!("Self(...) on non-struct/class type");
                                    }
                                };
                                (
                                    self.mangle_fn_name(&format!("{}.init", type_name), &[]),
                                    true,
                                )
                            } else {
                                self.emit_error(
                                    TrussDiagnosticCode::TypeInferenceFailed,
                                    "Self type not resolved",
                                    None,
                                );
                                anyhow::bail!("Self type not resolved");
                            }
                        } else {
                            self.emit_error(
                                TrussDiagnosticCode::UnsupportedFeature,
                                "Only simple function calls and method calls are supported",
                                None,
                            );
                            anyhow::bail!("Unsupported callee");
                        }
                    }
                };

                let callee_ty = match &*callee.borrow() {
                    Expression::Variable { ty, .. } => ty.clone(),
                    Expression::MemberAccess { ty, .. } => ty.clone(),
                    Expression::SelfType { ty, .. } => ty.clone(),
                    _ => None,
                };
                let is_throwing_call = callee_ty.as_ref().map_or(false, |t| {
                    matches!(&*t.borrow(), Type::Function(_, _, _, Some(_)))
                });
                if let Some(ty) = callee_ty
                    && self.module.get_function(&function_name).is_none()
                {
                    let (param_tys, ret_ty, is_vararg, throws_types) = match &*ty.borrow() {
                        Type::Function(pt, rt, iv, tt) => (pt.clone(), rt.clone(), *iv, tt.clone()),
                        Type::Closure(pt, rt) => (pt.clone(), rt.clone(), false, None),
                        _ => {
                            anyhow::bail!("Unsupported callee type for indirect call");
                        }
                    };
                    let fn_ptr_val = self.resolve_expression(callee.clone())?.unwrap();
                    let mut all_param_tys = param_tys.clone();
                    if throws_types.is_some() {
                        let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                            Type::Struct(
                                "Int8".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            ),
                        )))));
                        all_param_tys.insert(0, err_ty);
                    }
                    let fn_llvm_type =
                        self.get_function_type(ret_ty.clone(), all_param_tys, is_vararg)?;
                    let fn_ptr = fn_ptr_val.into_pointer_value();

                    let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
                    let err_slot_ptr = if is_throwing_call {
                        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                        let err_slot = self.builder.build_alloca(ptr_ty, "call_err")?;
                        self.builder.build_store(err_slot, ptr_ty.const_null())?;
                        args.push(err_slot.into());
                        Some(err_slot)
                    } else {
                        None
                    };
                    for param in parameters {
                        let arg_val = self.resolve_expression(param.expression.clone())?.unwrap();
                        args.push(arg_val.into());
                    }

                    let call_result =
                        self.builder
                            .build_indirect_call(fn_llvm_type, fn_ptr, &args, "")?;
                    if let Some(err_slot) = err_slot_ptr {
                        if let Some(err_ptr) = *self.error_ptr.borrow() {
                            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                            let err_val =
                                self.builder.build_load(ptr_ty, err_slot, "call_err_val")?;
                            self.builder.build_store(err_ptr, err_val)?;
                        }
                    }
                    let call_val = match call_result.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(val) => val,
                        _ => return Ok(None),
                    };
                    if let BasicValueEnum::PointerValue(ptr_val) = call_val {
                        if let Ok(call_ty) = expr.borrow().get_ty_ref().map(|ty| ty.clone()) {
                            if let Some(call_ty) = call_ty {
                                let is_void = matches!(&*call_ty.borrow(), Type::Void);
                                if !is_void {
                                    if let Ok(expected_llvm_ty) = self.resolve_type(call_ty) {
                                        if expected_llvm_ty != ptr_val.get_type().into() {
                                            let loaded = self.builder.build_load(
                                                expected_llvm_ty,
                                                ptr_val,
                                                "",
                                            )?;
                                            return Ok(Some(loaded));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    return Ok(Some(call_val));
                }

                let function = self.module.get_function(&function_name).ok_or_else(|| {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedFunction,
                        format!("Undefined function: '{}'", function_name),
                        None,
                    );
                    anyhow::anyhow!("Undefined function: {}", function_name)
                })?;

                let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();

                let callee_struct_name = match &*callee.borrow() {
                    Expression::Variable { name, .. } => Some(name.value.clone()),
                    _ => None,
                };
                let instantiation_ptr: Option<(BasicTypeEnum<'ctx>, PointerValue<'ctx>)> =
                    if is_init_call {
                        if let Some(ref struct_name) = callee_struct_name {
                            if let Some(class_type) =
                                self.class_types.borrow().get(struct_name).cloned()
                            {
                                let i64_ty = self.context.i64_type();
                                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                                let null_i8 = self.builder.build_int_to_ptr(
                                    i64_ty.const_int(0, false),
                                    ptr_ty,
                                    "",
                                )?;
                                let class_ptr_ty =
                                    self.context.ptr_type(inkwell::AddressSpace::from(0));
                                let null_class_ptr =
                                    self.builder.build_pointer_cast(null_i8, class_ptr_ty, "")?;
                                let size_val = unsafe {
                                    let gep = self.builder.build_gep(
                                        class_type.as_basic_type_enum(),
                                        null_class_ptr,
                                        &[i64_ty.const_int(1, false)],
                                        "",
                                    )?;
                                    self.builder.build_ptr_to_int(gep, i64_ty, "")?
                                };
                                let malloc_fn =
                                    self.module.get_function("malloc").unwrap_or_else(|| {
                                        let fn_ty = self
                                            .context
                                            .i64_type()
                                            .fn_type(&[self.context.i64_type().into()], false);
                                        self.module.add_function("malloc", fn_ty, None)
                                    });
                                let malloc_result =
                                    self.builder.build_call(malloc_fn, &[size_val.into()], "")?;
                                let heap_ptr = match malloc_result.try_as_basic_value() {
                                    inkwell::values::ValueKind::Basic(
                                        inkwell::values::BasicValueEnum::PointerValue(p),
                                    ) => p,
                                    _ => anyhow::bail!("malloc expected to return a pointer"),
                                };
                                let class_ptr =
                                    self.builder
                                        .build_pointer_cast(heap_ptr, class_ptr_ty, "")?;
                                let vtable_global =
                                    self.vtable_globals.borrow().get(struct_name).copied();
                                if let Some(vt_global) = vtable_global {
                                    let vtable_ptr_gep = self
                                        .builder
                                        .build_struct_gep(class_type, class_ptr, 0, "")?;
                                    self.builder.build_store(
                                        vtable_ptr_gep,
                                        vt_global.as_pointer_value(),
                                    )?;
                                }

                                let rc_ptr = self
                                    .builder
                                    .build_struct_gep(class_type, class_ptr, 1, "")?;
                                self.builder
                                    .build_store(rc_ptr, i64_ty.const_int(1, false))?;

                                args.push(class_ptr.into());
                                Some((class_type.as_basic_type_enum(), class_ptr))
                            } else {
                                self.struct_types
                                    .borrow()
                                    .get(struct_name)
                                    .cloned()
                                    .map(|st| {
                                        let ptr = self.builder.build_alloca(st, "").unwrap();
                                        args.push(ptr.into());
                                        (st.as_basic_type_enum(), ptr)
                                    })
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                if let Some(ptr) = method_self_ptr {
                    args.push(ptr.into());
                }

                if is_throwing_call {
                    if let Some(err_ptr) = *self.error_ptr.borrow() {
                        args.push(err_ptr.into());
                    } else {
                        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                        let dummy_err = self.builder.build_alloca(ptr_ty, "dummy_err")?;
                        self.builder.build_store(dummy_err, ptr_ty.const_null())?;
                        args.push(dummy_err.into());
                    }
                }

                for param in parameters {
                    let arg_val = self.resolve_expression(param.expression.clone())?.unwrap();
                    let fn_param_types = function.get_type().get_param_types();
                    let arg_idx = if is_init_call {
                        0
                    } else {
                        method_self_ptr.is_some() as usize
                    } + args.len();
                    if arg_idx < fn_param_types.len()
                        && fn_param_types[arg_idx] != arg_val.get_type().into()
                        && fn_param_types[arg_idx].is_pointer_type()
                    {
                        let alloca = self.builder.build_alloca(arg_val.get_type(), "")?;
                        self.builder.build_store(alloca, arg_val)?;
                        let ptr_val = self.builder.build_pointer_cast(
                            alloca,
                            fn_param_types[arg_idx].into_pointer_type(),
                            "",
                        )?;
                        args.push(ptr_val.into());
                    } else {
                        args.push(arg_val.into());
                    }
                }

                let call_result = self.builder.build_call(function, &args, "")?;

                if let Some((_, ptr)) = instantiation_ptr {
                    if self
                        .class_types
                        .borrow()
                        .contains_key(callee_struct_name.as_deref().unwrap_or(""))
                    {
                        let val: BasicValueEnum<'ctx> = ptr.into();
                        Ok(Some(val))
                    } else {
                        let fn_ret_type = function.get_type().get_return_type();
                        let is_void_return = fn_ret_type.is_none();
                        if is_void_return {
                            let val = self.builder.build_load(ptr.get_type(), ptr, "")?;
                            Ok(Some(val))
                        } else {
                            let call_val = match call_result.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(val) => val,
                                _ => return Ok(None),
                            };
                            Ok(Some(call_val))
                        }
                    }
                } else {
                    let call_val = match call_result.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(val) => val,
                        _ => return Ok(None),
                    };
                    let fn_ret_type = function.get_type().get_return_type();
                    if let Some(inkwell::types::BasicTypeEnum::PointerType(_)) = fn_ret_type {
                        if let BasicValueEnum::PointerValue(ptr_val) = call_val {
                            let expected_ty = expr.borrow().get_ty_ref().ok().and_then(|ty| ty.clone()).or_else(|| {
                                match &*callee.borrow() {
                                    Expression::Variable { ty, .. } | Expression::MemberAccess { ty, .. } => {
                                        ty.as_ref().and_then(|ct| {
                                            match &*ct.borrow() {
                                                Type::Function(_, ret_ty, _, _) => Some(ret_ty.clone()),
                                                _ => None,
                                            }
                                        })
                                    }
                                    _ => None,
                                }
                            });
                            if let Some(call_ty) = expected_ty {
                                let is_void = matches!(&*call_ty.borrow(), Type::Void);
                                if !is_void {
                                    if let Ok(expected_llvm_ty) = self.resolve_type(call_ty) {
                                        if expected_llvm_ty != ptr_val.get_type().into() {
                                            let loaded = self.builder.build_load(
                                                expected_llvm_ty,
                                                ptr_val,
                                                "",
                                            )?;
                                            return Ok(Some(loaded));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(Some(call_val))
                }
            }
            Expression::TupleLiteral { elements, .. } => {
                let ty_ref = expr.borrow().get_ty_ref()?.clone();
                if let Some(ty) = ty_ref
                    && let Type::Tuple(_) = &*ty.borrow()
                {
                    let tuple_llvm = self.resolve_type(ty.clone())?.into_struct_type();
                    let alloca = self.builder.build_alloca(tuple_llvm, "")?;
                    for (i, (_, element)) in elements.iter().enumerate() {
                        let val = self.resolve_expression(element.clone())?.unwrap();
                        let field_ptr = self
                            .builder
                            .build_struct_gep(tuple_llvm, alloca, i as u32, "")?;
                        self.builder.build_store(field_ptr, val)?;
                    }
                    let val =
                        self.builder
                            .build_load(tuple_llvm.as_basic_type_enum(), alloca, "")?;
                    Ok(Some(val))
                } else {
                    anyhow::bail!("Tuple literal has no type info");
                }
            }
            Expression::TupleIndexAccess {
                object,
                index_value,
                ..
            } => {
                let object_ty_ref = object.borrow().get_ty_ref()?.clone();
                if let Some(object_ty) = object_ty_ref
                    && let Type::Tuple(elements) = &*object_ty.borrow()
                {
                    let idx = *index_value as usize;
                    if idx >= elements.len() {
                        anyhow::bail!("Tuple index {} out of bounds", idx);
                    }
                    let element_ty = self.resolve_type(elements[idx].1.clone())?;
                    let object_val = self.resolve_expression(object.clone())?.unwrap();
                    let tuple_llvm = self.resolve_type(object_ty.clone())?.into_struct_type();
                    let struct_ptr = match object_val {
                        BasicValueEnum::PointerValue(ptr) => ptr,
                        val => {
                            let ptr = self.builder.build_alloca(val.get_type(), "")?;
                            self.builder.build_store(ptr, val)?;
                            ptr
                        }
                    };
                    let field_ptr = self
                        .builder
                        .build_struct_gep(tuple_llvm, struct_ptr, idx as u32, "")?;
                    let val = self.builder.build_load(element_ty, field_ptr, "")?;
                    Ok(Some(val))
                } else {
                    anyhow::bail!("TupleIndexAccess on non-tuple type");
                }
            }
            Expression::Match { value, cases, .. } => {
                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let exit_bb = self.context.append_basic_block(fn_val, "match_exit");

                let subject_val = self.resolve_expression(value.clone())?.unwrap();
                let subject_alloca = self.builder.build_alloca(subject_val.get_type(), "")?;
                self.builder.build_store(subject_alloca, subject_val)?;

                let enum_name = self
                    .get_enum_name_from_expr_string(value.clone())
                    .unwrap_or_default();

                let is_enum = !enum_name.is_empty();

                let (tag_val, enum_llvm_type, raw_llvm_type) = if is_enum {
                    let enum_types = self.enum_types.borrow();
                    let enum_llvm_type = enum_types
                        .get(&enum_name)
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
                    drop(enum_types);

                    if let Some(raw_ty) = self.get_enum_raw_llvm_type(&enum_name) {
                        let raw_ptr =
                            self.builder
                                .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                        let tag_val = self.builder.build_load(raw_ty, raw_ptr, "")?;
                        (
                            Some(tag_val.into_int_value()),
                            Some(enum_llvm_type),
                            Some(raw_ty),
                        )
                    } else {
                        let tag_ptr =
                            self.builder
                                .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                        let tag_val =
                            self.builder
                                .build_load(self.context.i8_type(), tag_ptr, "")?;
                        (Some(tag_val.into_int_value()), Some(enum_llvm_type), None)
                    }
                } else {
                    (None, None, None)
                };

                let mut all_body_bbs = Vec::new();
                let mut all_check_bbs = Vec::new();

                for _ in cases.iter() {
                    all_body_bbs.push(self.context.append_basic_block(fn_val, "case_body"));
                    all_check_bbs.push(self.context.append_basic_block(fn_val, "case_check"));
                }

                if !all_check_bbs.is_empty() {
                    self.builder
                        .build_unconditional_branch(all_check_bbs[0])?;
                } else {
                    self.builder.build_unconditional_branch(exit_bb)?;
                }

                for (i, case) in cases.iter().enumerate() {
                    let body_bb = all_body_bbs[i];
                    let next_check_or_exit_bb = if i + 1 < all_check_bbs.len() {
                        all_check_bbs[i + 1]
                    } else {
                        exit_bb
                    };

                    let mut prev_check_bb = all_check_bbs[i];
                    let pattern_count = case.patterns.len();

                    for (pi, pattern) in case.patterns.iter().enumerate() {
                        let is_last_pattern = pi == pattern_count - 1;
                        let next_bb = if is_last_pattern {
                            next_check_or_exit_bb
                        } else {
                            self.context
                                .append_basic_block(fn_val, "case_pattern_check")
                        };

                        self.builder.position_at_end(prev_check_bb);

                        let match_result = if is_enum {
                            match pattern.as_ref() {
                                Pattern::EnumCase { case_name, .. } => {
                                    let idx =
                                        self.get_enum_case_index(&enum_name, &case_name.value)?;
                                    let expected_tag = if let Some(raw_ty) = raw_llvm_type {
                                        raw_ty.into_int_type().const_int(idx as u64, false)
                                    } else {
                                        self.context.i8_type().const_int(idx as u64, false)
                                    };
                                    Some(self.builder.build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        tag_val.unwrap(),
                                        expected_tag,
                                        "",
                                    )?)
                                }
                                _ => None,
                            }
                        } else if let Pattern::Tuple(tuple_items) = pattern.as_ref() {
                            self.match_tuple_elements(tuple_items, subject_alloca, value.clone())?
                        } else {
                            match pattern.as_ref() {
                                Pattern::Expr(expr) => {
                                    self.get_literal_match(subject_val, expr.clone())?
                                }
                                _ => None,
                            }
                        };

                        if let Some(result) = match_result {
                            self.builder
                                .build_conditional_branch(result, body_bb, next_bb)?;
                        } else {
                            self.builder.build_unconditional_branch(body_bb)?;
                        }

                        prev_check_bb = next_bb;
                    }

                    self.builder.position_at_end(body_bb);
                    self.enter_scope();

                    if is_enum {
                        for pattern in &case.patterns {
                            if let Pattern::EnumCase { bindings, .. } = pattern.as_ref() {
                                if !bindings.is_empty() {
                                    let idx = match pattern.as_ref() {
                                        Pattern::EnumCase { case_name, .. } => {
                                            self.get_enum_case_index(&enum_name, &case_name.value)?
                                        }
                                        _ => unreachable!(),
                                    };
                                    let enum_payloads = self.enum_payload_types.borrow();
                                    if let Some(payload_type) = enum_payloads.get(&enum_name) {
                                        let payload_union_ptr = self.builder.build_struct_gep(
                                            enum_llvm_type.unwrap(),
                                            subject_alloca,
                                            1,
                                            "",
                                        )?;
                                        let case_payload_ptr = self.builder.build_struct_gep(
                                            *payload_type,
                                            payload_union_ptr,
                                            idx as u32,
                                            "",
                                        )?;
                                        let case_payload_ty = payload_type
                                            .get_field_type_at_index(idx as u32)
                                            .ok_or_else(|| {
                                                anyhow::anyhow!("Payload not found at idx {}", idx)
                                            })?;
                                        let case_payload_struct_ty =
                                            case_payload_ty.into_struct_type();

                                        for (j, binding) in bindings.iter().enumerate() {
                                            match binding {
                                                Pattern::Identifier(tok) => {
                                                    let field_ptr = self.builder.build_struct_gep(
                                                        case_payload_struct_ty,
                                                        case_payload_ptr,
                                                        j as u32,
                                                        "",
                                                    )?;
                                                    let field_ty = case_payload_struct_ty
                                                        .get_field_type_at_index(j as u32)
                                                        .ok_or_else(|| {
                                                            anyhow::anyhow!(
                                                                "Field not found at idx {}",
                                                                j
                                                            )
                                                        })?;
                                                    let field_val = self
                                                        .builder
                                                        .build_load(field_ty, field_ptr, "")?;
                                                    let var_ptr = self
                                                        .builder
                                                        .build_alloca(field_ty, &tok.value)?;
                                                    self.builder.build_store(var_ptr, field_val)?;
                                                    self.declare_variable(
                                                        tok.value.clone(),
                                                        var_ptr,
                                                    );
                                                }
                                                Pattern::ValueBinding(inner) => {
                                                    if let Pattern::Identifier(tok) = inner.as_ref()
                                                    {
                                                        let field_ptr =
                                                            self.builder.build_struct_gep(
                                                                case_payload_struct_ty,
                                                                case_payload_ptr,
                                                                j as u32,
                                                                "",
                                                            )?;
                                                        let field_ty = case_payload_struct_ty
                                                            .get_field_type_at_index(j as u32)
                                                            .ok_or_else(|| {
                                                                anyhow::anyhow!(
                                                                    "Field not found at idx {}",
                                                                    j
                                                                )
                                                            })?;
                                                        let field_val = self
                                                            .builder
                                                            .build_load(field_ty, field_ptr, "")?;
                                                        let var_ptr = self
                                                            .builder
                                                            .build_alloca(field_ty, &tok.value)?;
                                                        self.builder
                                                            .build_store(var_ptr, field_val)?;
                                                        self.declare_variable(
                                                            tok.value.clone(),
                                                            var_ptr,
                                                        );
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                            if let Pattern::ValueBinding(inner) = pattern.as_ref() {
                                if let Pattern::Identifier(tok) = inner.as_ref() {
                                    let var_ptr = self
                                        .builder
                                        .build_alloca(subject_val.get_type(), &tok.value)?;
                                    self.builder.build_store(var_ptr, subject_val)?;
                                    self.declare_variable(tok.value.clone(), var_ptr);
                                }
                                break;
                            }
                        }
                    }

                    if !is_enum {
                        for pattern in &case.patterns {
                            if let Pattern::Tuple(tuple_items) = pattern.as_ref() {
                                self.resolve_match_tuple_bindings(tuple_items, subject_alloca)?;
                            }
                        }
                    }

                    let mut case_terminated = false;
                    for stmt in &case.body {
                        if let Ok(terminates) = self.resolve_statement(stmt.clone()) {
                            if terminates {
                                case_terminated = true;
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    if case.guard.is_some() {
                        let _ = self.resolve_expression(case.guard.clone().unwrap());
                    }

                    if !case_terminated {
                        self.builder.build_unconditional_branch(exit_bb)?;
                    }
                    self.exit_scope();

                    if i + 1 < all_check_bbs.len() {
                        self.builder.position_at_end(all_check_bbs[i + 1]);
                    }
                }

                self.builder.position_at_end(exit_bb);
                Ok(None)
            }
            Expression::Closure {
                captures,
                parameters,
                return_type,
                body,
                scope,
                ty,
                ..
            } => {
                let counter = {
                    let mut c = self.closure_counter.borrow_mut();
                    let val = *c;
                    *c += 1;
                    val
                };
                let fn_name = format!("_T$CC${}", counter);

                let ret_type = return_type
                    .as_ref()
                    .and_then(|rt| {
                        let expr = rt.borrow();
                        expr.get_ty().ok().flatten()
                    })
                    .or_else(|| {
                        ty.as_ref().and_then(|t| {
                            let t_borrow = t.borrow();
                            match &*t_borrow {
                                Type::Function(_, ret_ty, _, None) | Type::Closure(_, ret_ty) => {
                                    Some(ret_ty.clone())
                                }
                                _ => None,
                            }
                        })
                    })
                    .unwrap_or_else(|| Rc::new(RefCell::new(Type::Void)));

                let mut param_types: Vec<Rc<RefCell<Type>>> = Vec::new();
                for param in parameters {
                    let pt = param
                        .borrow()
                        .type_annotation
                        .as_ref()
                        .and_then(|ta| ta.borrow().get_ty().ok().flatten())
                        .unwrap_or_else(|| {
                            Rc::new(RefCell::new(Type::Struct(
                                "Int32".to_string(),
                                WeakSymbol(std::rc::Weak::new()),
                                vec![],
                            )))
                        });
                    param_types.push(pt);
                }

                let all_param_types: Vec<Rc<RefCell<Type>>> = ty
                    .as_ref()
                    .and_then(|t| {
                        let t_borrow = t.borrow();
                        match &*t_borrow {
                            Type::Function(pts, _, _, None) | Type::Closure(pts, _) => {
                                Some(pts.clone())
                            }
                            _ => None,
                        }
                    })
                    .unwrap_or_else(|| param_types.clone());

                // Resolve capture types from the closure's parent scope (set by TypeResolver)
                let capture_llvm_types: Vec<BasicTypeEnum> = if !captures.is_empty() {
                    let parent_scope = scope.as_ref().and_then(|s| s.borrow().parent.clone());
                    captures
                        .iter()
                        .filter_map(|cap| {
                            let cap_name = &cap.name.value;
                            let var_ty = parent_scope
                                .as_ref()
                                .and_then(|s| s.borrow().get_type(cap_name))?;
                            self.resolve_type(var_ty).ok()
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let has_captures = !captures.is_empty();

                let fn_llvm_type = if has_captures {
                    // Closure function takes captured values (by value, using their actual types)
                    // as extra parameters before explicit params
                    let mut fn_param_types = Vec::new();
                    for ct in &capture_llvm_types {
                        fn_param_types.push((*ct).into());
                    }
                    for pt in &all_param_types {
                        let llvm_pt = self.resolve_type(pt.clone())?;
                        fn_param_types.push(llvm_pt.as_basic_type_enum().into());
                    }
                    let ret_llvm_ty = self.resolve_type(ret_type.clone())?;
                    let fn_ty: FunctionType = ret_llvm_ty.fn_type(&fn_param_types, false);
                    fn_ty
                } else {
                    self.get_function_type(ret_type.clone(), all_param_types.clone(), false)?
                };
                let function = self.module.add_function(&fn_name, fn_llvm_type, None);

                let current_block = self.builder.get_insert_block();
                let entry_block = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry_block);

                self.enter_scope_with_stmts(body)?;
                let mut param_idx = 0u32;
                // Closure function: first capture params (by value), then explicit params
                if has_captures {
                    for (i, cap) in captures.iter().enumerate() {
                        let cap_name = &cap.name.value;
                        let llvm_ty = if i < capture_llvm_types.len() {
                            capture_llvm_types[i]
                        } else {
                            self.context.i32_type().into()
                        };
                        let alloca_name = self.unique_alloca_name(cap_name);
                        let ptr = self.builder.build_alloca(llvm_ty, &alloca_name)?;
                        let param_value = function.get_nth_param(param_idx).unwrap();
                        self.builder.build_store(ptr, param_value)?;
                        self.declare_variable(cap_name.clone(), ptr);
                        param_idx += 1;
                    }
                }
                for (i, param) in parameters.iter().enumerate() {
                    let param_name = &param.borrow().name.value;
                    let llvm_type = self.resolve_type(param_types[i].clone())?;
                    let alloca_name = self.unique_alloca_name(param_name);
                    let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                    let param_value = function.get_nth_param(param_idx).unwrap();
                    self.builder.build_store(ptr, param_value)?;
                    self.declare_variable(param_name.clone(), ptr);
                    param_idx += 1;
                }

                let max_shorthand = self.find_max_shorthand_in_body(body);
                if let Some(max_idx) = max_shorthand {
                    for idx in 0..=max_idx {
                        let shorthand_name = format!("${}", idx);
                        let param_ty =
                            all_param_types
                                .get(idx as usize)
                                .cloned()
                                .unwrap_or_else(|| {
                                    Rc::new(RefCell::new(Type::Struct(
                                        "Int32".to_string(),
                                        WeakSymbol(std::rc::Weak::new()),
                                        vec![],
                                    )))
                                });
                        let llvm_type = self.resolve_type(param_ty)?;
                        let alloca_name = self.unique_alloca_name(&shorthand_name);
                        let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                        let param_value = function.get_nth_param(param_idx).unwrap();
                        self.builder.build_store(ptr, param_value)?;
                        self.declare_variable(shorthand_name, ptr);
                        param_idx += 1;
                    }
                }

                let is_void = matches!(&*ret_type.borrow(), Type::Void);
                match &body[..] {
                    [] if is_void => {
                        self.builder.build_return(None)?;
                    }
                    stmts => {
                        let stmt_count = stmts.len();
                        let mut has_return = false;
                        for (i, stmt) in stmts.iter().enumerate() {
                            let is_last = i == stmt_count - 1;
                            if is_last && !is_void {
                                if let Statement::ExpressionStatement { expression } =
                                    &*stmt.borrow()
                                {
                                    let value = self.resolve_expression(expression.clone())?;
                                    if let Some(value) = value {
                                        self.builder.build_return(Some(&value))?;
                                        has_return = true;
                                        break;
                                    }
                                }
                            }
                            if self.resolve_statement(stmt.clone())? {
                                has_return = true;
                                break;
                            }
                        }
                        if !has_return {
                            if is_void {
                                self.builder.build_return(None)?;
                            }
                        }
                    }
                }

                self.exit_scope();

                if let Some(block) = current_block {
                    self.builder.position_at_end(block);
                }

                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                if has_captures {
                    // Create context struct with fn_ptr and captured values
                    // For each capture, allocate heap cell, copy value, store cell ptr in context
                    let context_struct_type = self.context.opaque_struct_type("__closure_ctx");
                    let mut field_types: Vec<BasicTypeEnum> = Vec::new();
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                    field_types.push(ptr_ty.into()); // fn_ptr
                    for _ in captures.iter() {
                        field_types.push(ptr_ty.into()); // cell ptr (i8*)
                    }
                    context_struct_type.set_body(&field_types, false);

                    // Allocate context struct on heap
                    let ctx_ptr = self.heap_allocate(context_struct_type.as_basic_type_enum())?;

                    // Store function pointer
                    let fn_ptr = function.as_global_value().as_pointer_value();
                    let fn_field_ptr =
                        self.builder
                            .build_struct_gep(context_struct_type, ctx_ptr, 0, "")?;
                    let fn_bitcast = self.builder.build_bit_cast(fn_ptr, ptr_ty, "")?;
                    self.builder.build_store(fn_field_ptr, fn_bitcast)?;

                    // For each capture: heap-allocate a cell, copy value, store cell ptr
                    let closure_scope_parent =
                        scope.as_ref().and_then(|s| s.borrow().parent.clone());
                    for (i, cap) in captures.iter().enumerate() {
                        // Get the LLVM type of the captured variable
                        let var_ty = closure_scope_parent
                            .as_ref()
                            .and_then(|s| s.borrow().get_type(&cap.name.value));
                        let llvm_cell_ty = if let Some(ty) = var_ty {
                            self.resolve_type(ty)
                                .unwrap_or_else(|_| self.context.i32_type().into())
                        } else {
                            self.context.i32_type().into()
                        };

                        // Look up the variable in enclosing scope
                        if let Some(stack_ptr) = self.lookup_variable(&cap.name.value) {
                            // Allocate heap cell of the variable's type
                            let heap_cell = self.heap_allocate(llvm_cell_ty)?;
                            // Copy value from stack to heap
                            let val = self.builder.build_load(llvm_cell_ty, stack_ptr, "")?;
                            self.builder.build_store(heap_cell, val)?;
                            // Store cell ptr (bitcast to i8*) in context struct
                            let cell_field_ptr = self.builder.build_struct_gep(
                                context_struct_type,
                                ctx_ptr,
                                (i + 1) as u32,
                                "",
                            )?;
                            let cell_ptr_i8 = self.builder.build_bit_cast(heap_cell, ptr_ty, "")?;
                            self.builder.build_store(cell_field_ptr, cell_ptr_i8)?;
                        }
                    }

                    // Return context struct pointer as i8*
                    let ctx_ptr_i8 = self.builder.build_bit_cast(ctx_ptr, ptr_ty, "")?;
                    Ok(Some(ctx_ptr_i8))
                } else {
                    let fn_ptr = function.as_global_value().as_pointer_value();
                    Ok(Some(
                        self.builder.build_bit_cast(fn_ptr, ptr_ty, "")?.into(),
                    ))
                }
            }
            Expression::ClosureType { .. } => Ok(None),
            Expression::FunctionType { .. } => Ok(None),
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
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedVariable,
                        format!("Undefined shorthand argument '${}'", index),
                        None,
                    );
                    anyhow::bail!("Undefined shorthand argument: ${}", index)
                }
            }
            Expression::MacroInvocation { name, .. } => {
                anyhow::bail!(
                    "Unexpected macro invocation '{}' - macros should be expanded before IR generation",
                    name.value
                )
            }
            Expression::Try {
                kind,
                expression: try_expr,
                ty,
                ..
            } => {
                let saved_error_ptr = *self.error_ptr.borrow();

                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                let local_error = self.builder.build_alloca(ptr_ty, "try_err")?;
                self.builder.build_store(local_error, ptr_ty.const_null())?;
                *self.error_ptr.borrow_mut() = Some(local_error);

                let result = self.resolve_expression(try_expr.clone())?;

                *self.error_ptr.borrow_mut() = saved_error_ptr;

                let fn_val = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let continue_bb = self.context.append_basic_block(fn_val, "try_cont");

                let err_val = self
                    .builder
                    .build_load(ptr_ty, local_error, "try_err_val")?;
                let err_int = self.builder.build_ptr_to_int(
                    err_val.into_pointer_value(),
                    self.context.i64_type(),
                    "try_err_int",
                )?;
                let err_is_null = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    err_int,
                    self.context.i64_type().const_zero(),
                    "try_err_is_null",
                )?;

                match kind {
                    TryKind::Plain => {
                        let throw_bb = self.context.append_basic_block(fn_val, "try_throw");
                        self.builder.build_conditional_branch(
                            err_is_null,
                            continue_bb,
                            throw_bb,
                        )?;

                        self.builder.position_at_end(throw_bb);
                        if let Some(outer_err) = saved_error_ptr {
                            self.builder.build_store(outer_err, err_val)?;
                        }
                        let ret_type = fn_val.get_type().get_return_type();
                        if let Some(ret_ty) = ret_type {
                            self.builder.build_return(Some(&ret_ty.const_zero()))?;
                        } else {
                            self.builder.build_return(None)?;
                        }

                        self.builder.position_at_end(continue_bb);
                        if let Some(val) = result {
                            Ok(Some(val))
                        } else {
                            Ok(None)
                        }
                    }
                    TryKind::Force => {
                        let panic_bb = self.context.append_basic_block(fn_val, "try_panic");
                        self.builder.build_conditional_branch(
                            err_is_null,
                            continue_bb,
                            panic_bb,
                        )?;

                        self.builder.position_at_end(panic_bb);
                        let panic_fn = self.module.get_function("panic").unwrap_or_else(|| {
                            let void_ty = self.context.void_type();
                            let fn_ty = void_ty.fn_type(&[ptr_ty.into()], false);
                            self.module.add_function("panic", fn_ty, None)
                        });
                        let err_str = self.builder.build_pointer_cast(local_error, ptr_ty, "")?;
                        let _ = self.builder.build_call(panic_fn, &[err_str.into()], "");
                        self.builder.build_unreachable()?;

                        self.builder.position_at_end(continue_bb);
                        if let Some(val) = result {
                            Ok(Some(val))
                        } else {
                            Ok(None)
                        }
                    }
                    TryKind::Optional => {
                        let none_bb = self.context.append_basic_block(fn_val, "try_none");
                        self.builder
                            .build_conditional_branch(err_is_null, continue_bb, none_bb)?;

                        self.builder.position_at_end(none_bb);
                        if let Some(t) = ty.as_ref()
                            && let Type::Enum(enum_name, ..) = &*t.borrow()
                        {
                            let enum_type = self
                                .enum_types
                                .borrow()
                                .get(enum_name)
                                .cloned()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Enum type '{}' not found", enum_name)
                                })?;
                            let payloads_type = self
                                .enum_payload_types
                                .borrow()
                                .get(enum_name)
                                .cloned()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Enum payload type '{}' not found", enum_name)
                                })?;
                            let none_payload_type = payloads_type
                                .get_field_type_at_index(0)
                                .ok_or_else(|| anyhow::anyhow!("None payload slot not found"))?;
                            let none_payload = none_payload_type.const_zero();
                            let tag = self.context.i8_type().const_zero();
                            let none_val =
                                enum_type.const_named_struct(&[tag.into(), none_payload.into()]);
                            self.builder.build_unconditional_branch(continue_bb)?;
                            self.builder.position_at_end(continue_bb);
                            let phi = self
                                .builder
                                .build_phi(enum_type.as_basic_type_enum(), "try_opt_phi")?;
                            phi.add_incoming(&[(&none_val as &dyn BasicValue, none_bb)]);
                            if let Some(val) = result {
                                let result_alloca =
                                    self.builder.build_alloca(val.get_type(), "")?;
                                self.builder.build_store(result_alloca, val)?;
                                let payload_type =
                                    payloads_type.get_field_type_at_index(1).ok_or_else(|| {
                                        anyhow::anyhow!("Some payload slot not found")
                                    })?;
                                let payload_struct = payload_type.into_struct_type();
                                let loaded =
                                    self.builder.build_load(val.get_type(), result_alloca, "")?;
                                let some_payload =
                                    payload_struct.const_named_struct(&[loaded.into()]);
                                let some_tag = self.context.i8_type().const_int(1, false);
                                let some_val = enum_type
                                    .const_named_struct(&[some_tag.into(), some_payload.into()]);
                                phi.add_incoming(&[(&some_val as &dyn BasicValue, continue_bb)]);
                            } else {
                                self.builder.build_unconditional_branch(continue_bb)?;
                                self.builder.position_at_end(continue_bb);
                            }
                            let phi_val = phi.as_basic_value();
                            Ok(Some(phi_val))
                        } else {
                            self.builder.build_unconditional_branch(continue_bb)?;
                            self.builder.position_at_end(continue_bb);
                            if let Some(val) = result {
                                Ok(Some(val))
                            } else {
                                Ok(None)
                            }
                        }
                    }
                }
            }
            Expression::ImplicitMemberAccess { member, ty } => {
                if let Some(t) = ty
                    && let Type::Enum(enum_name, ..) = &*t.borrow()
                {
                    let case_name = member.value.clone();
                    let enum_name = enum_name.clone();
                    let case_index = self.get_enum_case_index(&enum_name, &case_name)?;
                    let enum_types = self.enum_types.borrow();
                    let enum_llvm_type = enum_types
                        .get(&enum_name)
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
                    drop(enum_types);
                    let alloca = self
                        .builder
                        .build_alloca(enum_llvm_type.as_basic_type_enum(), "")?;
                    let tag_ptr = self
                        .builder
                        .build_struct_gep(enum_llvm_type, alloca, 0, "")?;
                    let tag_val = self.context.i8_type().const_int(case_index as u64, false);
                    self.builder.build_store(tag_ptr, tag_val)?;
                    let val =
                        self.builder
                            .build_load(enum_llvm_type.as_basic_type_enum(), alloca, "")?;
                    Ok(Some(val))
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::UnsupportedFeature,
                        format!(
                            "Implicit member '{}' requires enum type context",
                            member.value
                        ),
                        Some(member),
                    );
                    anyhow::bail!(
                        "Implicit member '{}' requires enum type context",
                        member.value
                    );
                }
            }
            _ => anyhow::bail!("Expression type not implemented"),
        }
    }

    fn find_max_shorthand_in_body(&self, body: &[Rc<RefCell<Statement>>]) -> Option<u32> {
        let mut max: Option<u32> = None;
        for stmt in body {
            match &*stmt.borrow() {
                Statement::ExpressionStatement { expression } => {
                    self.find_shorthand_in_expr(expression, &mut max);
                }
                Statement::Return {
                    value: Some(val), ..
                } => {
                    self.find_shorthand_in_expr(val, &mut max);
                }
                _ => {}
            }
        }
        max
    }

    fn find_shorthand_in_expr(&self, expr: &Rc<RefCell<Expression>>, max: &mut Option<u32>) {
        match &*expr.borrow() {
            Expression::ShorthandArgument { index, .. } => match max {
                Some(m) if *index > *m => *max = Some(*index),
                None => *max = Some(*index),
                _ => {}
            },
            Expression::Binary { left, right, .. } => {
                self.find_shorthand_in_expr(left, max);
                self.find_shorthand_in_expr(right, max);
            }
            Expression::Unary { expression, .. } => {
                self.find_shorthand_in_expr(expression, max);
            }
            Expression::Call { parameters, .. } => {
                for param in parameters {
                    self.find_shorthand_in_expr(&param.expression, max);
                }
            }
            Expression::SizeOf { argument, .. } => {
                self.find_shorthand_in_expr(argument, max);
            }
            _ => {}
        }
    }

    fn get_function_type(
        &self,
        return_type: Rc<RefCell<Type>>,
        param_types: Vec<Rc<RefCell<Type>>>,
        is_vararg: bool,
    ) -> Result<inkwell::types::FunctionType<'ctx>> {
        let mut param_basic_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
        for param_type in param_types {
            param_basic_types.push(self.resolve_type(param_type.clone())?.into());
        }

        let is_void = matches!(&*return_type.borrow(), Type::Void);
        if is_void {
            let void_type = self.context.void_type();
            Ok(void_type.fn_type(&param_basic_types, is_vararg))
        } else {
            let return_basic_type = self.resolve_type(return_type.clone())?;
            Ok(return_basic_type.fn_type(&param_basic_types, is_vararg))
        }
    }

    fn get_protocol_method_fn_type(
        &self,
        protocol_name: &str,
        method_name: &str,
    ) -> Option<inkwell::types::FunctionType<'ctx>> {
        let scope = self.program_scope.borrow();
        let scope_ref = scope.as_ref()?;
        let symbol = scope_ref.borrow().get_symbol(protocol_name)?;
        let Symbol::Protocol { methods, .. } = &*symbol.borrow() else {
            return None;
        };

        let method_types: Vec<_> = methods
            .iter()
            .filter_map(|m| {
                let m_borrow = m.borrow();
                let Ok(name) = m_borrow.name() else {
                    return None;
                };
                if name != method_name {
                    return None;
                }
                let Ok(Some(decl)) = m_borrow.get_decl() else {
                    return None;
                };
                drop(m_borrow);
                let decl_borrow = decl.borrow();
                let Statement::FunctionDecl { ty, .. } = &*decl_borrow else {
                    return None;
                };
                let Some(ty) = ty else { return None };
                let Type::Function(param_types, return_type, is_vararg, throws_types) =
                    &*ty.borrow()
                else {
                    return None;
                };
                Some((
                    return_type.clone(),
                    param_types.clone(),
                    *is_vararg,
                    throws_types.clone(),
                ))
            })
            .collect();

        let (return_type, param_types, is_vararg, throws_types) =
            method_types.into_iter().next()?;
        let mut all_params = param_types;
        if throws_types.is_some() {
            let err_ty = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                Type::Struct("Int8".to_string(), WeakSymbol(std::rc::Weak::new()), vec![]),
            )))));
            let mut new_params = vec![err_ty];
            new_params.extend(all_params);
            all_params = new_params;
        }
        self.get_function_type(return_type, all_params, is_vararg)
            .ok()
    }

    fn is_protocol_method_throwing(&self, protocol_name: &str, method_name: &str) -> bool {
        let scope = self.program_scope.borrow();
        let Some(scope_ref) = scope.as_ref() else {
            return false;
        };
        let Some(symbol) = scope_ref.borrow().get_symbol(protocol_name) else {
            return false;
        };
        let Symbol::Protocol { methods, .. } = &*symbol.borrow() else {
            return false;
        };
        methods.iter().any(|m| {
            let m_borrow = m.borrow();
            let Ok(name) = m_borrow.name() else {
                return false;
            };
            if name != method_name {
                return false;
            }
            let Ok(Some(decl)) = m_borrow.get_decl() else {
                return false;
            };
            drop(m_borrow);
            let decl_borrow = decl.borrow();
            let Statement::FunctionDecl { ty, .. } = &*decl_borrow else {
                return false;
            };
            let Some(ty) = ty else { return false };
            matches!(&*ty.borrow(), Type::Function(_, _, _, Some(_)))
        })
    }

    fn extract_concrete_type_name(&self, expr: &Rc<RefCell<Expression>>) -> Option<String> {
        let e = expr.borrow();
        match &*e {
            Expression::Call { callee, .. } => {
                if let Expression::Variable {
                    name: callee_name, ..
                } = &*callee.borrow()
                {
                    Some(callee_name.value.clone())
                } else {
                    None
                }
            }
            Expression::Variable { ty, .. } => {
                let ty = ty.as_ref()?;
                match &*ty.borrow() {
                    Type::Struct(name, ..) | Type::Class(name, ..) | Type::Enum(name, ..) => {
                        Some(name.clone())
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn get_literal_match(
        &self,
        subject_val: BasicValueEnum<'ctx>,
        pattern_expr: Rc<RefCell<Expression>>,
    ) -> Result<Option<IntValue<'ctx>>> {
        let expr_ref = pattern_expr.borrow();
        match &*expr_ref {
            Expression::IntegerLiteral { value, .. } => {
                let int_val = subject_val.into_int_value();
                let int_ty = int_val.get_type();
                let lit_val = int_ty.const_int(*value as u64, *value < 0);
                Ok(Some(self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    int_val,
                    lit_val,
                    "",
                )?))
            }
            Expression::DecimalLiteral { value, .. } => {
                let float_val = subject_val.into_float_value();
                let float_ty = float_val.get_type();
                let lit_val = float_ty.const_float(*value);
                Ok(Some(self.builder.build_float_compare(
                    inkwell::FloatPredicate::OEQ,
                    float_val,
                    lit_val,
                    "",
                )?))
            }
            _ => Ok(None),
        }
    }

    fn get_enum_name_from_expr_string(&self, expr: Rc<RefCell<Expression>>) -> Option<String> {
        let e = expr.borrow();
        let ty = match &*e {
            Expression::Variable { ty, .. } => ty.as_ref()?,
            Expression::IntegerLiteral { ty, .. } => ty.as_ref()?,
            Expression::MemberAccess { ty, .. } => ty.as_ref()?,
            Expression::SelfKeyword { ty, .. } => ty.as_ref()?,
            Expression::Call { .. } => {
                drop(e);
                if let Ok(Some(val)) = self.resolve_expression(expr.clone()) {
                    let ty = val.get_type();
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                    if ty == ptr_ty.into() {
                        return None;
                    }
                }
                return None;
            }
            _ => return None,
        };
        if let Type::Enum(name, ..) = &*ty.borrow() {
            Some(name.clone())
        } else {
            None
        }
    }

    fn emit_class_releases(&self) {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
        for &alloca in self.class_refs.borrow().iter() {
            let val = self.builder.build_load(ptr_ty, alloca, "").unwrap();
            if let BasicValueEnum::PointerValue(p) = val {
                let _ = self.emit_release(p);
            }
        }
        for &container in self.container_refs.borrow().iter() {
            let ptr_ptr = unsafe {
                self.builder
                    .build_gep(
                        ptr_ty,
                        container,
                        &[self.context.i64_type().const_int(0, false)],
                        "",
                    )
                    .unwrap()
            };
            let val = self.builder.build_load(ptr_ty, ptr_ptr, "").unwrap();
            if let BasicValueEnum::PointerValue(p) = val {
                let _ = self.emit_release(p);
            }
        }
    }

    fn heap_allocate(&self, llvm_type: BasicTypeEnum<'ctx>) -> Result<PointerValue<'ctx>> {
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
        let null_ptr = self
            .builder
            .build_int_to_ptr(i64_ty.const_int(0, false), ptr_ty, "")?;
        let size_val = unsafe {
            let gep =
                self.builder
                    .build_gep(llvm_type, null_ptr, &[i64_ty.const_int(1, false)], "")?;
            self.builder.build_ptr_to_int(gep, i64_ty, "")?
        };
        let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
            let fn_ty = ptr_ty.fn_type(&[i64_ty.into()], false);
            self.module.add_function("malloc", fn_ty, None)
        });
        let result = self.builder.build_call(malloc_fn, &[size_val.into()], "")?;
        match result.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(inkwell::values::BasicValueEnum::PointerValue(p)) => {
                Ok(p)
            }
            _ => anyhow::bail!("malloc expected to return a pointer"),
        }
    }

    fn emit_retain(&self, obj_ptr: PointerValue<'ctx>) -> Result<()> {
        let saved_block = self.builder.get_insert_block();
        let fn_val = self.get_or_create_retain_fn();
        self.builder.build_call(fn_val, &[obj_ptr.into()], "")?;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }
        Ok(())
    }

    fn emit_release(&self, obj_ptr: PointerValue<'ctx>) -> Result<()> {
        let saved_block = self.builder.get_insert_block();
        let fn_val = self.get_or_create_release_fn();
        self.builder.build_call(fn_val, &[obj_ptr.into()], "")?;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }
        Ok(())
    }

    fn get_or_create_retain_fn(&self) -> FunctionValue<'ctx> {
        let fn_name = "truss_retain";
        if let Some(f) = self.module.get_function(fn_name) {
            return f;
        }
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
        let i64_ty = self.context.i64_type();
        let i8_ty = self.context.i8_type();
        let fn_ty = self.context.void_type().fn_type(&[ptr_ty.into()], false);
        let f = self.module.add_function(fn_name, fn_ty, None);
        let entry = self.context.append_basic_block(f, "entry");
        let saved = self.builder.get_insert_block();
        self.builder.position_at_end(entry);
        let obj = f.get_nth_param(0).unwrap().into_pointer_value();
        let rc_ptr = unsafe {
            self.builder
                .build_gep(i8_ty, obj, &[i64_ty.const_int(8, false)], "")
                .unwrap()
        };
        let rc = self
            .builder
            .build_load(i64_ty, rc_ptr, "")
            .unwrap()
            .into_int_value();
        let new_rc = self
            .builder
            .build_int_add(rc, i64_ty.const_int(1, false), "")
            .unwrap();
        self.builder.build_store(rc_ptr, new_rc).unwrap();
        self.builder.build_return(None).unwrap();
        if let Some(block) = saved {
            self.builder.position_at_end(block);
        }
        f
    }

    fn get_or_create_release_fn(&self) -> FunctionValue<'ctx> {
        let fn_name = "truss_release";
        if let Some(f) = self.module.get_function(fn_name) {
            return f;
        }
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
        let i64_ty = self.context.i64_type();
        let i8_ty = self.context.i8_type();
        let void_fn_ty = self.context.void_type().fn_type(&[ptr_ty.into()], false);
        let f = self.module.add_function(fn_name, void_fn_ty, None);

        let saved = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(f, "entry");
        let call_deinit = self.context.append_basic_block(f, "call_deinit");
        let free_block = self.context.append_basic_block(f, "free");
        let done_block = self.context.append_basic_block(f, "done");

        self.builder.position_at_end(entry);
        let obj = f.get_nth_param(0).unwrap().into_pointer_value();
        let rc_ptr = unsafe {
            self.builder
                .build_gep(i8_ty, obj, &[i64_ty.const_int(8, false)], "")
                .unwrap()
        };
        let rc = self
            .builder
            .build_load(i64_ty, rc_ptr, "")
            .unwrap()
            .into_int_value();
        let new_rc = self
            .builder
            .build_int_sub(rc, i64_ty.const_int(1, false), "")
            .unwrap();
        self.builder.build_store(rc_ptr, new_rc).unwrap();
        let is_zero = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                new_rc,
                i64_ty.const_int(0, false),
                "",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(is_zero, call_deinit, done_block)
            .unwrap();

        self.builder.position_at_end(call_deinit);
        let vtable_ptr = self
            .builder
            .build_load(ptr_ty, obj, "")
            .unwrap()
            .into_pointer_value();
        let deinit_fn = self
            .builder
            .build_load(ptr_ty, vtable_ptr, "")
            .unwrap()
            .into_pointer_value();
        self.builder
            .build_indirect_call(void_fn_ty, deinit_fn, &[obj.into()], "")
            .unwrap();
        self.builder.build_unconditional_branch(free_block).unwrap();

        self.builder.position_at_end(free_block);
        self.builder
            .build_call(
                self.module.get_function("free").unwrap_or_else(|| {
                    let free_ty = self.context.void_type().fn_type(&[ptr_ty.into()], false);
                    self.module.add_function("free", free_ty, None)
                }),
                &[obj.into()],
                "",
            )
            .unwrap();
        self.builder.build_return(None).unwrap();

        self.builder.position_at_end(done_block);
        self.builder.build_return(None).unwrap();

        if let Some(block) = saved {
            self.builder.position_at_end(block);
        }
        f
    }

    fn resolve_type(&self, ty: Rc<RefCell<Type>>) -> Result<BasicTypeEnum<'ctx>> {
        let resolved = match &*ty.borrow() {
            Type::Struct(name, _, _) if name == "Int8" => self.context.i8_type().into(),
            Type::Struct(name, _, _) if name == "Int16" => self.context.i16_type().into(),
            Type::Struct(name, _, _) if name == "Int32" => self.context.i32_type().into(),
            Type::Struct(name, _, _) if name == "Int64" => self.context.i64_type().into(),
            Type::Struct(name, _, _) if name == "Int128" => self.context.i128_type().into(),
            Type::Struct(name, _, _) if name == "UInt8" => self.context.i8_type().into(),
            Type::Struct(name, _, _) if name == "UInt16" => self.context.i16_type().into(),
            Type::Struct(name, _, _) if name == "UInt32" => self.context.i32_type().into(),
            Type::Struct(name, _, _) if name == "UInt64" => self.context.i64_type().into(),
            Type::Struct(name, _, _) if name == "UInt128" => self.context.i128_type().into(),
            Type::Struct(name, _, _) if name == "Float" => self.context.f32_type().into(),
            Type::Struct(name, _, _) if name == "Double" => self.context.f64_type().into(),
            Type::Struct(name, _, _) if name == "Bool" => self.context.bool_type().into(),
            Type::Struct(name, _, _) if name == "Char" => self.context.i8_type().into(),
            Type::Never => {
                self.emit_error(
                    TrussDiagnosticCode::NeverTypeConversion,
                    "Never type cannot be converted to LLVM type",
                    None,
                );
                anyhow::bail!("Never type cannot be converted to LLVM type");
            }
            Type::Void => {
                self.emit_error(
                    TrussDiagnosticCode::VoidTypeConversion,
                    "Void type is handled specially as void return type",
                    None,
                );
                anyhow::bail!("Void type is handled specially as void return type");
            }
            Type::Function(_, _, _, _) => {
                self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
            }
            Type::Closure(_, _) => self.context.ptr_type(inkwell::AddressSpace::from(0)).into(),
            Type::Pointer(_) | Type::NonNullPointer(_) => {
                self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
            }
            Type::Struct(name, ..) => {
                if let Some(struct_type) = self.struct_types.borrow().get(name) {
                    struct_type.as_basic_type_enum()
                } else if name == "Array" || name == "String" {
                    self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::StructTypeNotSupported,
                        format!("Struct type '{}' not found in IR generation", name),
                        None,
                    );
                    anyhow::bail!("Struct type not found");
                }
            }
            Type::Class(name, ..) => {
                let ptr_type: BasicTypeEnum<'ctx> =
                    self.context.ptr_type(inkwell::AddressSpace::from(0)).into();
                if self.class_types.borrow().contains_key(name) {
                    ptr_type
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::StructTypeNotSupported,
                        format!("Class type '{}' not found in IR generation", name),
                        None,
                    );
                    anyhow::bail!("Class type not found");
                }
            }
            Type::Enum(name, ..) => {
                if let Some(enum_type) = self.enum_types.borrow().get(name) {
                    enum_type.as_basic_type_enum()
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::EnumTypeNotSupported,
                        format!("Enum type '{}' not found in IR generation", name),
                        None,
                    );
                    anyhow::bail!("Enum type not found");
                }
            }
            Type::Protocol(name, ..) => {
                if let Some(container_type) = self.existential_container_types.borrow().get(name) {
                    container_type.as_basic_type_enum()
                } else {
                    self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
                }
            }
            Type::Compound(types) => {
                let all_protocols = types
                    .iter()
                    .all(|t| matches!(&*t.borrow(), Type::Protocol(..)));
                if all_protocols {
                    let compound = Type::Compound(types.clone());
                    if let Some(ct) =
                        self.get_or_create_existential_container_for_compound(&compound)
                    {
                        ct.as_basic_type_enum()
                    } else {
                        self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
                    }
                } else {
                    self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
                }
            }
            Type::Tuple(elements) => {
                let type_key = self.get_tuple_struct_key(elements);
                if let Some(struct_type) = self.struct_types.borrow().get(&type_key) {
                    struct_type.as_basic_type_enum()
                } else {
                    let field_types: Vec<BasicTypeEnum<'ctx>> = elements
                        .iter()
                        .map(|(_, e)| self.resolve_type(e.clone()))
                        .collect::<Result<Vec<_>>>()?;
                    let struct_type = self
                        .context
                        .opaque_struct_type(&format!("tuple.{}", type_key));
                    struct_type.set_body(&field_types, false);
                    self.struct_types.borrow_mut().insert(type_key, struct_type);
                    struct_type.as_basic_type_enum()
                }
            }
            Type::GenericParam(_) => self.context.ptr_type(inkwell::AddressSpace::from(0)).into(),
            Type::ConstGeneric(_, ct) => self.resolve_type(ct.clone())?,
            Type::AssociatedType(_, _) => {
                self.context.ptr_type(inkwell::AddressSpace::from(0)).into()
            }
            Type::Inline(inner, _) => {
                let inner_borrow = inner.borrow();
                match &*inner_borrow {
                    Type::Class(name, ..) => {
                        if let Some(class_type) = self.class_types.borrow().get(name).cloned() {
                            class_type.into()
                        } else {
                            self.context.i8_type().array_type(8).into()
                        }
                    }
                    _ => self.context.i8_type().array_type(8).into(),
                }
            }
        };
        Ok(resolved)
    }

    fn get_tuple_struct_key(&self, elements: &[(Option<String>, Rc<RefCell<Type>>)]) -> String {
        let mut key = String::from("__tuple_");
        for (i, (_, elem)) in elements.iter().enumerate() {
            if i > 0 {
                key.push('_');
            }
            key.push_str(&elem.borrow().to_string());
        }
        key
    }

    fn get_struct_field_type(
        &self,
        struct_name: &str,
        field_name: &str,
    ) -> Result<BasicTypeEnum<'ctx>> {
        if let Some(scope) = self.program_scope.borrow().as_ref()
            && let Some(symbol) = scope.borrow().get_symbol(struct_name)
        {
            let binding = symbol.borrow();
            let (decl, properties) = match &*binding {
                Symbol::Struct {
                    decl, properties, ..
                } => (decl.clone(), properties.clone()),
                Symbol::Class {
                    decl, properties, ..
                } => (decl.clone(), properties.clone()),
                _ => {
                    return Err(anyhow::anyhow!(
                        "Symbol '{}' is not a struct or class",
                        struct_name
                    ));
                }
            };
            drop(binding);

            for field in properties.iter() {
                if field.borrow().name().as_ref().ok() == Some(&field_name.to_string())
                    && let Some(field_decl) = field.borrow().get_decl().ok().flatten()
                    && let Statement::VariableDecl { ty: Some(ty), .. } = &*field_decl.borrow()
                {
                    return self.resolve_type(ty.clone());
                }
            }

            if let Statement::ClassDecl {
                superclass: Some(super_expr),
                ..
            } = &*decl.borrow()
            {
                if let Expression::Type {
                    name: super_name, ..
                } = &*super_expr.borrow()
                {
                    return self.get_struct_field_type(&super_name.value, field_name);
                }
            }
        }
        self.emit_error(
            TrussDiagnosticCode::UnsupportedFeature,
            format!(
                "Cannot get type of field '{}' in struct '{}'",
                field_name, struct_name
            ),
            None,
        );
        anyhow::bail!("Cannot get field type")
    }

    fn infer_type_from_expression(
        &self,
        expr: Rc<RefCell<Expression>>,
    ) -> Result<BasicTypeEnum<'ctx>> {
        match &*expr.borrow() {
            Expression::IntegerLiteral { ty, .. } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty.clone())
                } else {
                    Ok(self.context.i32_type().into())
                }
            }
            Expression::DecimalLiteral { ty, .. } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty.clone())
                } else {
                    Ok(self.context.f64_type().into())
                }
            }
            Expression::BooleanLiteral { .. } => Ok(self.context.bool_type().into()),
            Expression::CharLiteral { .. } => Ok(self.context.i8_type().into()),
            Expression::Variable { ty, name, .. } => {
                if let Some(ty) = ty {
                    self.resolve_type(ty.clone())
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::TypeInferenceFailed,
                        "Cannot infer type from variable without type annotation",
                        Some(name),
                    );
                    anyhow::bail!("Cannot infer type from variable")
                }
            }
            Expression::Unary { expression, .. } => {
                self.infer_type_from_expression(expression.clone())
            }
            Expression::Binary { left, right, .. } => self
                .infer_type_from_expression(left.clone())
                .or_else(|_| self.infer_type_from_expression(right.clone())),
            Expression::SizeOf { .. } => self.resolve_type(Rc::new(RefCell::new(Type::Struct(
                "UInt64".to_string(),
                WeakSymbol(std::rc::Weak::new()),
                vec![],
            )))),
            _ => {
                self.emit_error(
                    TrussDiagnosticCode::TypeInferenceFailed,
                    "Cannot infer type from this expression",
                    None,
                );
                anyhow::bail!("Cannot infer type")
            }
        }
    }

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

    fn emit_error(
        &self,
        code: TrussDiagnosticCode,
        message: impl Into<String>,
        token: Option<&Token>,
    ) {
        let msg = message.into();
        let diag = if let Some(token) = token {
            new_diagnostic(code, &msg).with_label(primary_label_from_token(token, &msg))
        } else {
            new_diagnostic(code, msg)
        };
        self.engine.borrow_mut().emit(diag);
    }

    fn generate_main_wrapper(&self, program: &Program) {
        let main_fn_name = self.find_main_function(program);
        if let Some(name) = main_fn_name {
            if self.module.get_function("main").is_some() {
                return;
            }
            if let Some(main_fn) = self.module.get_function(&name) {
                let i32_type = self.context.i32_type();
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let main_type = i32_type.fn_type(&[i32_type.into(), ptr_type.into()], false);
                let c_main = self.module.add_function("main", main_type, None);
                let entry = self.context.append_basic_block(c_main, "entry");
                self.builder.position_at_end(entry);
                let fn_type = main_fn.get_type();
                let ret_type = fn_type.get_return_type();
                if ret_type.is_none() {
                    self.builder.build_call(main_fn, &[], "call_main").ok();
                    let _ = self.builder.build_return(Some(&i32_type.const_zero()));
                } else {
                    let result = self.builder.build_call(main_fn, &[], "call_main");
                    if let Ok(call_val) = result {
                        match call_val.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(val) => {
                                let _ = self.builder.build_return(Some(&val));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn find_main_function(&self, program: &Program) -> Option<String> {
        for stmt in &program.statements {
            let result = self.find_main_in_stmt(stmt);
            if result.is_some() {
                return result;
            }
        }
        None
    }

    fn find_main_in_stmt(&self, stmt: &Rc<RefCell<Statement>>) -> Option<String> {
        let s = stmt.borrow();
        match &*s {
            Statement::FunctionDecl {
                attributes,
                name,
                parameters,
                ..
            } => {
                if attributes.iter().any(|a| a.name == "main") {
                    Some(self.mangle_fn_name(&name.value, parameters))
                } else {
                    None
                }
            }
            Statement::ModuleDecl { body, .. } => {
                for child in body {
                    let result = self.find_main_in_stmt(child);
                    if result.is_some() {
                        return result;
                    }
                }
                None
            }
            _ => None,
        }
    }
}

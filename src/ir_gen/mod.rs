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
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType},
    values::{
        BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
    },
};

use crate::{
    ast::{
        expression::{
            AssignmentOperator, BinaryOperator, CallParameter, CastKind, ElseBranch, Expression,
            UnaryOperator,
        },
        node::Program,
        statement::{
            Accessor, AccessorKind, AsmOperand, FunctionBody, Parameter, Pattern, ProtocolMember,
            Statement, VariadicKind,
        },
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    lexer::token::{KeywordType, Token, TokenType},
    scope::Scope as TrussScope,
    symbol::Symbol,
    types::Type,
};

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

pub struct IRGenerator<'ctx> {
    context: &'ctx Context,
    module: Rc<Module<'ctx>>,
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
}

impl<'ctx> IRGenerator<'ctx> {
    pub fn new(context: &'ctx Context, engine: Rc<RefCell<TrussDiagnosticEngine>>) -> Self {
        let module = Rc::new(context.create_module("main"));
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
        }
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
                    if let Type::Struct(type_name, _) = &*ty_borrow {
                        let type_name = type_name.clone();
                        drop(ty_borrow);
                        let mut stack = self.scope_stack.borrow_mut();
                        stack
                            .last_mut()
                            .unwrap()
                            .deferred_vars
                            .push((ptr, type_name));
                    } else if let Type::Enum(type_name, _) = &*ty_borrow {
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
                                Type::Class(name, _) => Some(name.clone()),
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
                let deinit_name = format!("{}.deinit", type_name);
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
                let deinit_name = format!("{}.deinit", type_name);
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

    pub fn generate(&self, program: &Program, scope: Rc<RefCell<TrussScope>>) -> Rc<Module<'ctx>> {
        *self.program_scope.borrow_mut() = Some(scope);

        for stmt in &program.statements {
            self.declare_struct_types(stmt.clone());
        }

        for stmt in &program.statements {
            self.declare_class_types(stmt.clone());
        }

        for stmt in &program.statements {
            self.declare_enum_types(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_vtable_types(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_protocol_witness_table_types(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_struct_type_bodies(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_class_type_bodies(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_existential_container_types(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_enum_type_bodies(stmt.clone());
        }

        {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for stmt in &program.statements {
                Self::count_fn_name_frequencies(stmt, &mut counts);
            }
            *self.overloaded_fn_names.borrow_mut() = counts
                .into_iter()
                .filter(|(_, c)| *c > 1)
                .map(|(n, _)| n)
                .collect();
        }

        for stmt in &program.statements {
            self.create_function_declarations(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_vtable_instances(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_protocol_witness_tables(stmt.clone());
        }

        for stmt in &program.statements {
            let _ = self.resolve_statement(stmt.clone());
        }
        self.module.clone()
    }

    fn declare_struct_types(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::StructDecl { name, .. } = &*statement.borrow() {
            let struct_name = &name.value;
            if !self.struct_types.borrow().contains_key(struct_name) {
                let struct_type = self
                    .context
                    .opaque_struct_type(&format!("struct.{}", struct_name));
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
        if let Statement::StructDecl { name, body, .. } = &*statement.borrow() {
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
                let class_type = self
                    .context
                    .opaque_struct_type(&format!("class.{}", class_name));
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

    fn declare_enum_types(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::EnumDecl { name, .. } = &*statement.borrow() {
            let enum_name = &name.value;
            if !self.enum_types.borrow().contains_key(enum_name) {
                let enum_type = self
                    .context
                    .opaque_struct_type(&format!("enum.{}", enum_name));
                self.enum_types
                    .borrow_mut()
                    .insert(enum_name.clone(), enum_type);

                let payload_type = self
                    .context
                    .opaque_struct_type(&format!("enum.{}.payloads", enum_name));
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
        if let Statement::EnumDecl { name, cases, .. } = &*statement.borrow() {
            let enum_name = &name.value;
            if let Some(enum_type) = self.enum_types.borrow().get(enum_name).cloned() {
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

    fn count_fn_name_frequencies(
        stmt: &Rc<RefCell<Statement>>,
        counts: &mut HashMap<String, usize>,
    ) {
        let s = stmt.borrow();
        match &*s {
            Statement::FunctionDecl { name, body, .. } => {
                *counts.entry(name.value.clone()).or_insert(0) += 1;
                if let FunctionBody::Statements(stmts) = &*body.borrow() {
                    for s in stmts {
                        Self::count_fn_name_frequencies(s, counts);
                    }
                }
            }
            Statement::StructDecl { name, body, .. } => {
                for s in body {
                    if let Statement::FunctionDecl { name: mname, .. } = &*s.borrow() {
                        let key = format!("{}.{}", name.value, mname.value);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                    Self::count_fn_name_frequencies(s, counts);
                }
            }
            Statement::ClassDecl { name, body, .. } => {
                for s in body {
                    if let Statement::FunctionDecl { name: mname, .. } = &*s.borrow() {
                        let key = format!("{}.{}", name.value, mname.value);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                    Self::count_fn_name_frequencies(s, counts);
                }
            }
            Statement::EnumDecl { name, body, .. } => {
                for s in body {
                    if let Statement::FunctionDecl { name: mname, .. } = &*s.borrow() {
                        let key = format!("{}.{}", name.value, mname.value);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                    Self::count_fn_name_frequencies(s, counts);
                }
            }
            Statement::ExtensionDecl {
                type_name, body, ..
            } => {
                for s in body {
                    if let Statement::FunctionDecl { name: mname, .. } = &*s.borrow() {
                        let key = format!("{}.{}", type_name.value, mname.value);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                    Self::count_fn_name_frequencies(s, counts);
                }
            }
            Statement::ConditionalBlock { clauses } => {
                for clause in clauses {
                    for stmt in &clause.body {
                        Self::count_fn_name_frequencies(stmt, counts);
                    }
                }
            }
            _ => {}
        }
    }

    fn type_to_abbreviation(ty: &Type) -> String {
        match ty {
            Type::Int8 => "I8".into(),
            Type::Int16 => "I16".into(),
            Type::Int32 => "I32".into(),
            Type::Int64 => "I64".into(),
            Type::Int128 => "I128".into(),
            Type::UInt8 => "U8".into(),
            Type::UInt16 => "U16".into(),
            Type::UInt32 => "U32".into(),
            Type::UInt64 => "U64".into(),
            Type::UInt128 => "U128".into(),
            Type::Float32 => "F32".into(),
            Type::Float64 => "F64".into(),
            Type::Bool => "B".into(),
            Type::Char => "C".into(),
            Type::Void => "V".into(),
            Type::Never => "N".into(),
            Type::Struct(name, _)
            | Type::Class(name, _)
            | Type::Enum(name, _)
            | Type::Protocol(name, _) => name.clone(),
            Type::Pointer(_) => "P".into(),
            Type::NonNullPointer(_) => "NP".into(),
            Type::Tuple(_) => "T".into(),
            Type::GenericParam(name) => name.clone(),
            Type::ConstGeneric(name, _) => format!("cg{}", name),
            Type::AssociatedType(_, name) => name.clone(),
            Type::Compound(_) => "C".into(),
            Type::Function(_, _, _) => "F".into(),
            Type::Inline(inner, _) => format!("inline{}", Self::type_to_abbreviation(&inner.borrow())),
        }
    }

    fn mangle_fn_name(base_name: &str, params: &[Rc<RefCell<Parameter>>]) -> String {
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
                    .map(|t| IRGenerator::type_to_abbreviation(&t.borrow()))
                    .unwrap_or_else(|| "?".into())
            })
            .collect();
        format!("{}${}${}", base_name, labels.join("_"), types.join("_"))
    }

    fn types_compatible(a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Int8, Type::Int8)
            | (Type::Int16, Type::Int16)
            | (Type::Int32, Type::Int32)
            | (Type::Int64, Type::Int64)
            | (Type::Int128, Type::Int128)
            | (Type::UInt8, Type::UInt8)
            | (Type::UInt16, Type::UInt16)
            | (Type::UInt32, Type::UInt32)
            | (Type::UInt64, Type::UInt64)
            | (Type::UInt128, Type::UInt128)
            | (Type::Float32, Type::Float32)
            | (Type::Float64, Type::Float64)
            | (Type::Bool, Type::Bool)
            | (Type::Char, Type::Char)
            | (Type::Void, Type::Void)
            | (Type::Never, Type::Never) => true,
            (Type::Struct(n1, _), Type::Struct(n2, _))
            | (Type::Class(n1, _), Type::Class(n2, _))
            | (Type::Enum(n1, _), Type::Enum(n2, _))
            | (Type::Protocol(n1, _), Type::Protocol(n2, _)) => n1 == n2,
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
            (Type::Inline(a_inner, _), b) => {
                Self::types_compatible(&a_inner.borrow(), b)
            }
            (a, Type::Inline(b_inner, _)) => {
                Self::types_compatible(a, &b_inner.borrow())
            }
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
            && let Statement::FunctionDecl { parameters, .. } = &*decl.borrow()
        {
            Some(Self::mangle_fn_name(base_name, parameters))
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
            ..
        } = &*statement.borrow()
        {
            if let Some(ty) = ty {
                if self.overloaded_fn_names.borrow().contains(&name.value) {
                    self.create_mangled_function_declaration(name, parameters, ty);
                } else {
                    let _ = self.create_function_declaration(name, ty);
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
                    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let base_name = format!("{}.{}", name.value, method_name.value);
                        if self.overloaded_fn_names.borrow().contains(&base_name) {
                            let mangled = Self::mangle_fn_name(&base_name, parameters);
                            self.module.add_function(&mangled, function_type, None);
                        } else {
                            self.module.add_function(&base_name, function_type, None);
                        }
                    }
                }
                if let Statement::InitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let llvm_name = format!("{}.{}", name.value, "init");
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::DeinitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(_, return_type, _) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), vec![self_param], false)
                    {
                        let llvm_name = format!("{}.deinit", name.value);
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::SubscriptDecl {
                    accessors,
                    ty: Some(ty),
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, _) = &*ty.borrow()
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
                        self.module.add_function(
                            &format!("{}.subscript.getter", name.value),
                            getter_type,
                            None,
                        );
                    }
                    if has_set {
                        let void_ty = Rc::new(RefCell::new(Type::Void));
                        let mut setter_param_types = all_param_types.clone();
                        setter_param_types.push(return_type.clone());
                        if let Ok(setter_type) =
                            self.get_function_type(void_ty, setter_param_types, false)
                        {
                            self.module.add_function(
                                &format!("{}.subscript.setter", name.value),
                                setter_type,
                                None,
                            );
                        }
                    }
                }
            }
            let deinit_name = format!("{}.deinit", name.value);
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
                    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let base_name = format!("{}.{}", name.value, method_name.value);
                        if self.overloaded_fn_names.borrow().contains(&base_name) {
                            let mangled = Self::mangle_fn_name(&base_name, parameters);
                            self.module.add_function(&mangled, function_type, None);
                        } else {
                            self.module.add_function(&base_name, function_type, None);
                        }
                    }
                }
                if let Statement::InitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let llvm_name = format!("{}.{}", name.value, "init");
                        self.module.add_function(&llvm_name, function_type, None);
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
                    && let Type::Function(_, return_type, _) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), vec![self_param], false)
                    {
                        let llvm_name = format!("{}.deinit", name.value);
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::SubscriptDecl {
                    accessors,
                    ty: Some(ty),
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, _) = &*ty.borrow()
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
                        self.module.add_function(
                            &format!("{}.subscript.getter", name.value),
                            getter_type,
                            None,
                        );
                    }
                    if has_set {
                        let void_ty = Rc::new(RefCell::new(Type::Void));
                        let mut setter_param_types = all_param_types.clone();
                        setter_param_types.push(return_type.clone());
                        if let Ok(setter_type) =
                            self.get_function_type(void_ty, setter_param_types, false)
                        {
                            self.module.add_function(
                                &format!("{}.subscript.setter", name.value),
                                setter_type,
                                None,
                            );
                        }
                    }
                }
            }
            let init_name = format!("{}.init", name.value);
            if self.module.get_function(&init_name).is_none() {
                let void_ty = Rc::new(RefCell::new(Type::Void));
                let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                    Type::Void,
                )))));
                if let Ok(fn_type) = self.get_function_type(void_ty, vec![self_param], false) {
                    let f = self.module.add_function(&init_name, fn_type, None);
                    let entry = self.context.append_basic_block(f, "entry");
                    let current_block = self.builder.get_insert_block();
                    self.builder.position_at_end(entry);
                    let _ = self.builder.build_return(None);
                    if let Some(block) = current_block {
                        self.builder.position_at_end(block);
                    }
                }
            }
            let deinit_name = format!("{}.deinit", name.value);
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
        if let Statement::EnumDecl { name, body, .. } = &*statement.borrow() {
            for stmt in body {
                if let Statement::FunctionDecl {
                    name: method_name,
                    ty,
                    ..
                } = &*stmt.borrow()
                    && let Some(ty) = ty
                    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
                    && let Ok(function_type) =
                        self.get_function_type(return_type.clone(), param_types.clone(), *is_vararg)
                {
                    let llvm_name = format!("{}.{}", name.value, method_name.value);
                    self.module.add_function(&llvm_name, function_type, None);
                }
                if let Statement::DeinitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(_, return_type, _) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), vec![self_param], false)
                    {
                        let llvm_name = format!("{}.deinit", name.value);
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::SubscriptDecl {
                    accessors,
                    ty: Some(ty),
                    ..
                } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, _) = &*ty.borrow()
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
                        self.module.add_function(
                            &format!("{}.subscript.getter", name.value),
                            getter_type,
                            None,
                        );
                    }
                    if has_set {
                        let void_ty = Rc::new(RefCell::new(Type::Void));
                        let mut setter_param_types = all_param_types.clone();
                        setter_param_types.push(return_type.clone());
                        if let Ok(setter_type) =
                            self.get_function_type(void_ty, setter_param_types, false)
                        {
                            self.module.add_function(
                                &format!("{}.subscript.setter", name.value),
                                setter_type,
                                None,
                            );
                        }
                    }
                }
            }
            let deinit_name = format!("{}.deinit", name.value);
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
            type_name, body, ..
        } = &*statement.borrow()
        {
            for stmt in body {
                if let Statement::FunctionDecl {
                    name: method_name,
                    ty,
                    static_method,
                    ..
                } = &*stmt.borrow()
                    && let Some(ty) = ty
                    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
                {
                    let all_param_types: Vec<Rc<RefCell<Type>>> = if *static_method {
                        param_types.clone()
                    } else {
                        let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(
                            RefCell::new(Type::Void),
                        ))));
                        let mut all_param_types = vec![self_param];
                        all_param_types.extend(param_types.iter().cloned());
                        all_param_types
                    };
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let llvm_name = format!("{}.{}", type_name.value, method_name.value);
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::InitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    let mut all_param_types = vec![self_param];
                    all_param_types.extend(param_types.iter().cloned());
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), all_param_types, *is_vararg)
                    {
                        let llvm_name = format!("{}.{}", type_name.value, "init");
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
                if let Statement::DeinitDecl { ty: Some(ty), .. } = &*stmt.borrow()
                    && let Type::Function(_, return_type, _) = &*ty.borrow()
                {
                    let self_param = Rc::new(RefCell::new(Type::Pointer(Rc::new(RefCell::new(
                        Type::Void,
                    )))));
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), vec![self_param], false)
                    {
                        let llvm_name = format!("{}.deinit", type_name.value);
                        self.module.add_function(&llvm_name, function_type, None);
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
                        body,
                        ..
                    } = &*decl.borrow()
                    && let Some(ty) = ty
                    && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
                    && !matches!(&*body.borrow(), FunctionBody::None)
                {
                    if let Ok(function_type) =
                        self.get_function_type(return_type.clone(), param_types.clone(), *is_vararg)
                    {
                        let llvm_name = format!("{}.{}", name.value, method_name.value);
                        self.module.add_function(&llvm_name, function_type, None);
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
        if let Statement::FunctionDecl { name, ty, .. } = &*statement.borrow()
            && let Some(ty) = ty
            && let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow()
        {
            let function_type =
                self.get_function_type(return_type.clone(), param_types.clone(), *is_vararg)?;
            self.module.add_function(&name.value, function_type, None);
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

    fn create_function_declaration(&self, name: &Token, ty: &Rc<RefCell<Type>>) -> Result<()> {
        if let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow() {
            let function_type =
                self.get_function_type(return_type.clone(), param_types.clone(), *is_vararg)?;
            self.module.add_function(&name.value, function_type, None);
        }
        Ok(())
    }

    fn create_mangled_function_declaration(
        &self,
        name: &Token,
        parameters: &[Rc<RefCell<Parameter>>],
        ty: &Rc<RefCell<Type>>,
    ) {
        if let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow() {
            if let Ok(function_type) =
                self.get_function_type(return_type.clone(), param_types.clone(), *is_vararg)
            {
                let mangled = Self::mangle_fn_name(&name.value, parameters);
                self.module.add_function(&mangled, function_type, None);
            }
        }
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
                Type::Protocol(name, _) => Some(name.clone()),
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
                    if let Type::Function(param_tys, _, _) = &*fn_borrow {
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
                    if let Type::Function(param_tys, _, _) = &*fn_borrow {
                        if param_tys.len() == proto_param_tys.len()
                            && param_tys
                                .iter()
                                .zip(proto_param_tys.iter())
                                .all(|(a, b)| Self::types_compatible(&a.borrow(), &b.borrow()))
                        {
                            let base = format!("{}.{}", type_name, actual_entry_name);
                            return Some(Self::mangle_fn_name(&base, &parameters));
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
        let type_name = if let Statement::ClassDecl {
            name, conformances, ..
        } = &*statement.borrow()
        {
            if conformances.is_empty() {
                return;
            }
            name.value.clone()
        } else if let Statement::StructDecl {
            name, conformances, ..
        } = &*statement.borrow()
        {
            if conformances.is_empty() {
                return;
            }
            name.value.clone()
        } else if let Statement::ExtensionDecl {
            type_name,
            conformances,
            ..
        } = &*statement.borrow()
        {
            if conformances.is_empty() {
                return;
            }
            type_name.value.clone()
        } else {
            return;
        };
        let type_name = &type_name;
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

            let Some(wt_type) = self
                .protocol_witness_table_types
                .borrow()
                .get(&protocol_name)
                .copied()
            else {
                continue;
            };
            let entries = self.compute_protocol_witness_table_entries(&protocol_name);
            if entries.is_empty() {
                continue;
            }

            let key = (protocol_name.clone(), type_name.clone());
            let wt_global_name = format!("__protocol_wt.{}.{}", protocol_name, type_name);
            if self.module.get_global(&wt_global_name).is_some() {
                continue;
            }

            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
            let mut const_vals: Vec<BasicValueEnum<'ctx>> = Vec::new();
            for (entry_name, entry_kind) in &entries {
                let base_name = format!("{}.{}", type_name, entry_name);
                let fn_name = if self.overloaded_fn_names.borrow().contains(&base_name) {
                    self.find_overloaded_witness_fn(
                        type_name,
                        entry_name,
                        entry_kind,
                        &protocol_name,
                    )
                    .unwrap_or(base_name)
                } else {
                    base_name
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
                AccessorKind::Get => format!("{}.getter", fn_prefix),
                AccessorKind::Set => format!("{}.setter", fn_prefix),
                AccessorKind::WillSet => format!("{}.willSet", fn_prefix),
                AccessorKind::DidSet => format!("{}.didSet", fn_prefix),
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
            format!("{}.{}.getter", struct_name, field_name)
        } else {
            format!("{}.{}.setter", struct_name, field_name)
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
                AccessorKind::Get => (format!("{}.getter", fn_prefix), vec![], true),
                AccessorKind::Set => {
                    let param_name = accessor
                        .parameter
                        .as_ref()
                        .map(|t| t.value.clone())
                        .unwrap_or_else(|| "newValue".to_string());
                    (
                        format!("{}.setter", fn_prefix),
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
                        format!("{}.willSet", fn_prefix),
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
                        format!("{}.didSet", fn_prefix),
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
        for stmt in &accessor.body {
            let terminates = self.resolve_statement(stmt.clone())?;
            if terminates {
                has_return = true;
                break;
            }
        }
        if is_getter && !has_return {
            if let Some(ptr) = self.lookup_variable(&format!("_{}", backing_var_name)) {
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
            format!("{}.getter", fn_prefix)
        } else {
            format!("{}.setter", fn_prefix)
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
                    self.builder
                        .build_call(willset_fn, &[class_ptr.into(), new_val.into()], "")?;
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
                initializer,
                ty,
                accessors,
                token,
                ..
            } => {
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
                                    Type::Protocol(protocol_name, _) => {
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
                        let fn_name = format!("{}.init", type_name);
                        if let Some(function) = self.module.get_function(&fn_name) {
                            let is_inline = ty.as_ref().map_or(false, |t| {
                                matches!(&*t.borrow(), Type::Inline(_, _))
                            });
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
                                let rc_ptr = self
                                    .builder
                                    .build_struct_gep(class_type, obj_ptr, 1, "")?;
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
                                    self.class_refs.borrow_mut().push(ptr);
                                }
                            } else {
                                let mut args = Vec::new();
                                args.push(ptr.into());
                                for param in parameters {
                                    let arg_val =
                                        self.resolve_expression(param.expression.clone())?.unwrap();
                                    args.push(arg_val.into());
                                }
                                self.builder.build_call(function, &args, "")?;
                            }
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
                            AccessorKind::Get => format!("{}.subscript.getter", sname),
                            AccessorKind::Set => format!("{}.subscript.setter", sname),
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
                                    Type::Function(_, ret, _) => ((), ret.clone(), false),
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
                let _ = self.context.append_basic_block(fn_val, "loop_exit");

                self.builder.build_unconditional_branch(body_bb)?;

                self.builder.position_at_end(body_bb);
                let terminates = self.resolve_block_expression(body)?;

                if !terminates {
                    self.builder.build_unconditional_branch(body_bb)?;
                }

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

                self.builder.build_unconditional_branch(body_bb)?;

                self.builder.position_at_end(body_bb);
                let terminates = self.resolve_block_expression(body)?;

                if !terminates {
                    self.builder.build_unconditional_branch(cond_bb)?;
                }

                self.builder.position_at_end(cond_bb);
                let cond_val = self.resolve_expression(condition.clone())?.unwrap();
                let cond_int = cond_val.into_int_value();
                self.builder
                    .build_conditional_branch(cond_int, body_bb, exit_bb)?;

                self.builder.position_at_end(exit_bb);
                Ok(false)
            }
            Statement::For { iterator, body, .. } => {
                let _ = self.resolve_expression(iterator.clone());
                self.resolve_block_expression(body)?;
                Ok(false)
            }
            Statement::FunctionDecl {
                ty: Some(ty),
                name,
                parameters,
                body,
                static_method,
                ..
            } => {
                if let Type::Function(_parameter_types, return_type, _) = &*ty.borrow() {
                    let saved_struct = self.current_struct.borrow_mut().take();
                    self.class_refs.borrow_mut().clear();
                    self.container_refs.borrow_mut().clear();
                    let fn_name = if let Some(struct_name) = &saved_struct {
                        let base = format!("{}.{}", struct_name, name.value);
                        if self.overloaded_fn_names.borrow().contains(&base) {
                            Self::mangle_fn_name(&base, parameters)
                        } else {
                            base
                        }
                    } else if self.overloaded_fn_names.borrow().contains(&name.value) {
                        Self::mangle_fn_name(&name.value, parameters)
                    } else {
                        name.value.clone()
                    };
                    let function = self.module.get_function(&fn_name).unwrap();

                    let current_block = self.builder.get_insert_block();

                    let entry_block = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry_block);

                    self.enter_scope();
                    if let Some(struct_name) = &*self.current_struct.borrow() {
                        let is_class_method = self.class_types.borrow().contains_key(struct_name);
                        let is_struct_method = self.struct_types.borrow().contains_key(struct_name);
                        if !static_method && (is_struct_method || is_class_method) {
                            let self_ptr = function.get_nth_param(0).unwrap();
                            let self_ptr = self_ptr.into_pointer_value();
                            self.declare_variable("self".to_string(), self_ptr);
                            let param_offset = 1;
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
                                    function.get_nth_param((i + param_offset) as u32).unwrap();
                                self.builder.build_store(ptr, param_value)?;
                                self.declare_variable(param_name.clone(), ptr);
                            }
                        } else {
                            for (i, param) in parameters.iter().enumerate() {
                                if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                    continue;
                                }
                                let param_name = &param.borrow().name.value;
                                let llvm_type =
                                    self.resolve_type(param.borrow().ty.clone().unwrap())?;
                                let alloca_name = self.unique_alloca_name(param_name);
                                let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                                let param_value = function.get_nth_param(i as u32).unwrap();
                                self.builder.build_store(ptr, param_value)?;
                                self.declare_variable(param_name.clone(), ptr);
                            }
                        }
                    } else {
                        for (i, param) in parameters.iter().enumerate() {
                            if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                continue;
                            }
                            let param_name = &param.borrow().name.value;
                            let llvm_type =
                                self.resolve_type(param.borrow().ty.clone().unwrap())?;
                            let alloca_name = self.unique_alloca_name(param_name);
                            let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                            let param_value = function.get_nth_param(i as u32).unwrap();
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
                            }
                            self.emit_class_releases();
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
                ..
            } => {
                if let Type::Function(_parameter_types, _return_type, _) = &*ty.borrow() {
                    let struct_name = self.current_struct.borrow().clone().unwrap();
                    let fn_name = format!("{}.init", struct_name);
                    let function = self.module.get_function(&fn_name).unwrap();

                    let current_block = self.builder.get_insert_block();
                    let entry_block = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry_block);

                    self.enter_scope();
                    let self_ptr = function.get_nth_param(0).unwrap();
                    let self_ptr = self_ptr.into_pointer_value();
                    self.declare_variable("self".to_string(), self_ptr);
                    for (i, param) in parameters.iter().enumerate() {
                        let param_name = &param.borrow().name.value;
                        let llvm_type = self.resolve_type(param.borrow().ty.clone().unwrap())?;
                        let alloca_name = self.unique_alloca_name(param_name);
                        let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                        let param_value = function.get_nth_param((i + 1) as u32).unwrap();
                        self.builder.build_store(ptr, param_value)?;
                        self.declare_variable(param_name.clone(), ptr);
                    }

                    let struct_name_clone = struct_name.clone();
                    self.auto_assign_init_fields(&struct_name_clone, self_ptr, parameters);

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
            Statement::DeinitDecl {
                ty: Some(ty), body, ..
            } => {
                if let Type::Function(_, _return_type, _) = &*ty.borrow() {
                    let struct_name = self.current_struct.borrow().clone().unwrap();
                    let fn_name = format!("{}.deinit", struct_name);
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
                        let value = self.resolve_expression(value.clone())?.unwrap();
                        self.emit_all_deinit_calls();
                        self.builder.build_return(Some(&value))?;
                    }
                    _ => {
                        self.emit_all_deinit_calls();
                        self.builder.build_return(None)?;
                    }
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
                type_name, body, ..
            } => {
                let prev = self.current_struct.borrow_mut().take();
                self.current_struct
                    .borrow_mut()
                    .replace(type_name.value.clone());
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

                    let tag_ptr =
                        self.builder
                            .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                    let tag_val = self
                        .builder
                        .build_load(self.context.i8_type(), tag_ptr, "")?;
                    let expected_tag = self.context.i8_type().const_int(case_idx as u64, false);
                    let match_result = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        tag_val.into_int_value(),
                        expected_tag,
                        "",
                    )?;

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
                anyhow::bail!("break outside of match is not supported");
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
                | Expression::TupleIndexAccess { ty, .. }
                | Expression::SelfKeyword { ty, .. }
                | Expression::SuperKeyword { ty, .. }
                | Expression::SelfType { ty, .. }
                | Expression::AnyType { ty, .. }
                | Expression::CompoundType { ty, .. }
                | Expression::Closure { ty, .. }
                | Expression::FunctionType { ty, .. }
                | Expression::SubscriptAccess { ty, .. }
                | Expression::MacroInvocation { ty, .. }
                | Expression::SizeOf { ty, .. }
                | Expression::PointerType { ty, .. }
                | Expression::Type { ty, .. }
                | Expression::Do { ty, .. } => ty.clone(),
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
                | Expression::TupleIndexAccess { ty, .. }
                | Expression::SelfKeyword { ty, .. }
                | Expression::SuperKeyword { ty, .. }
                | Expression::SelfType { ty, .. }
                | Expression::AnyType { ty, .. }
                | Expression::CompoundType { ty, .. }
                | Expression::Closure { ty, .. }
                | Expression::FunctionType { ty, .. }
                | Expression::SubscriptAccess { ty, .. }
                | Expression::MacroInvocation { ty, .. }
                | Expression::SizeOf { ty, .. }
                | Expression::PointerType { ty, .. }
                | Expression::Type { ty, .. }
                | Expression::Do { ty, .. } => ty.clone(),
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
                Ok(Some(
                    llvm_type
                        .into_int_type()
                        .const_int(*value as u64, false)
                        .into(),
                ))
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
                let getter_name = format!("{}.getter", name.value);
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
                } else if let Some(fn_val) = self.module.get_function(&name.value) {
                    let fn_ptr = fn_val.as_global_value().as_pointer_value();
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                    Ok(Some(
                        self.builder.build_bit_cast(fn_ptr, ptr_ty, "")?.into(),
                    ))
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
                    let llvm_type = if let Some(ty) = ty {
                        self.resolve_type(ty.clone())?
                    } else {
                        self.emit_error(
                            TrussDiagnosticCode::TypeInferenceFailed,
                            "Cannot infer type for 'self'",
                            Some(token),
                        );
                        anyhow::bail!("Cannot infer type for 'self'");
                    };
                    let val = self.builder.build_load(llvm_type, ptr, "")?;
                    Ok(Some(val))
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
                                    Type::Struct(n, _) | Type::Class(n, _) | Type::Enum(n, _) => {
                                        Some(n.clone())
                                    }
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
                let is_unsigned = matches!(left_ty, Some(ty) if matches!(&*ty.borrow(), Type::UInt8 | Type::UInt16 | Type::UInt32 | Type::UInt64 | Type::UInt128));

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
                    BinaryOperator::RangeTo | BinaryOperator::RangeUntil => {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Range expressions are not yet supported in IR generation",
                            None,
                        );
                        anyhow::bail!("Range expressions not implemented");
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
                                        Type::Struct(n, _)
                                        | Type::Class(n, _)
                                        | Type::Enum(n, _) => n.clone(),
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
                        // TODO: for now, just pass through the value
                        // In a full implementation, this would check for null/none
                        Ok(Some(expr_val))
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
                    let setter_name = format!("{}.setter", name.value);
                    let willset_name = format!("{}.willSet", name.value);
                    let didset_name = format!("{}.didSet", name.value);
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
                        if let Type::Struct(struct_name, _) = &*ty.borrow() {
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

                            let setter_name = format!("{}.{}.setter", struct_name, field_name);
                            let willset_name = format!("{}.{}.willSet", struct_name, field_name);
                            let didset_name = format!("{}.{}.didSet", struct_name, field_name);

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
                        } else if let Type::Class(class_name, _) = &*ty.borrow() {
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
                                    let declared_fn_name =
                                        format!("{}.{}.setter", owner, field_name);
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
                        && let Type::Struct(struct_name, _) = &*ty.borrow()
                    {
                        let struct_name = struct_name.clone();
                        let object_val = self.resolve_expression(sub_object.clone())?.unwrap();
                        let struct_ptr = if let BasicValueEnum::PointerValue(ptr) = object_val {
                            ptr
                        } else {
                            let ptr = self.builder.build_alloca(object_val.get_type(), "")?;
                            self.builder.build_store(ptr, object_val)?;
                            ptr
                        };
                        let setter_name = format!("{}.subscript.setter", struct_name);
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
                        && let Type::Class(class_name, _) = &*ty.borrow()
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
                            let declared_fn_name = format!("{}.subscript.setter", owner);
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

                    let tag_ptr =
                        self.builder
                            .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                    let tag_val = self
                        .builder
                        .build_load(self.context.i8_type(), tag_ptr, "")?;
                    let expected_tag = self.context.i8_type().const_int(case_idx as u64, false);
                    let match_result = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        tag_val.into_int_value(),
                        expected_tag,
                        "",
                    )?;

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
                    if let Some((Ok(alloca), _)) = result_alloca.as_ref() {
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
                        if let Some((Ok(alloca), _)) = result_alloca.as_ref() {
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
            Expression::Do { body, ty, .. } => {
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

                if !terminated_by_return && !terminated_by_yield && !has_yield_target {
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
            Expression::MemberAccess { object, member, .. } => {
                let object_expr = object.borrow();
                let object_ty = object_expr.get_ty_ref()?.clone();
                drop(object_expr);

                if let Some(ty) = &object_ty
                    && let Type::Struct(struct_name, _) = &*ty.borrow()
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

                    let getter_name = format!("{}.{}.getter", struct_name, field_name);
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
                    && let Type::Class(class_name, _) = &*ty.borrow()
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
                            let declared_fn_name = format!("{}.{}.getter", owner, field_name);
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

                                            let getter_name =
                                                format!("{}.{}.getter", pname, field_name);
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
                    && let Type::Protocol(protocol_name, _) = &*ty.borrow()
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

                            let getter_name = format!("{}.{}.getter", protocol_name, field_name);
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
                    && let Type::Enum(enum_name, _) = &*ty.borrow()
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
                    && let Type::Class(class_name, _) = &*inner.borrow()
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
                        let field_index = self.get_stored_class_field_index(class_name, &member.value);
                        if let Ok(field_index) = field_index {
                            let field_llvm_ptr = self
                                .builder
                                .build_struct_gep(class_type, struct_ptr, field_index as u32, "")?;
                            let field_llvm_type = self
                                .get_struct_field_type(class_name, &member.value)?;
                            let val = self
                                .builder
                                .build_load(field_llvm_type, field_llvm_ptr, "")?;
                            return Ok(Some(val));
                        }
                    }
                    self.emit_error(
                        TrussDiagnosticCode::FieldNotFound,
                        format!("Field '{}' not found on inline class '{}'", member.value, class_name),
                        Some(member.as_ref()),
                    );
                    anyhow::bail!("Field '{}' not found on inline class '{}'", member.value, class_name);
                }

                self.emit_error(
                    TrussDiagnosticCode::UnsupportedFeature,
                    "Member access on non-struct type",
                    Some(member.as_ref()),
                );
                anyhow::bail!("Member access on non-struct type");
            }
            Expression::Call {
                callee,
                parameters,
                overloads,
                selected_index,
                ..
            } => {
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
                                (format!("{}.init", name), true)
                            }
                        } else if self.module.get_function(&name).is_some() {
                            (name, false)
                        } else {
                            (format!("{}.init", name), true)
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
                                && let Type::Struct(struct_name, _) = &*ty.borrow()
                            {
                                let object_val = self.resolve_expression(object.clone())?.unwrap();
                                let ptr = if let BasicValueEnum::PointerValue(p) = object_val {
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
                                    .unwrap_or(base_name.clone())
                                } else {
                                    base_name.clone()
                                };
                                (fn_name, false)
                            } else if let Some(ty) = &object_ty
                                && let Type::Class(class_name, _) = &*ty.borrow()
                            {
                                let object_val = self.resolve_expression(object.clone())?.unwrap();
                                let ptr = if let BasicValueEnum::PointerValue(p) = object_val {
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
                                    let fn_name = format!("{}.{}", class_name, method_name);
                                    let declared_fn =
                                        self.module.get_function(&fn_name).ok_or_else(|| {
                                            self.emit_error(
                                                TrussDiagnosticCode::UndefinedFunction,
                                                format!("Undefined function: '{}'", fn_name),
                                                None,
                                            );
                                            anyhow::anyhow!("Undefined function: {}", fn_name)
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
                                    let fn_name = format!("{}.{}", owner, method_name);
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
                                    .unwrap_or(base_name.clone())
                                } else {
                                    base_name.clone()
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
                                                            let mut args: Vec<
                                                            inkwell::values::BasicMetadataValueEnum<
                                                                'ctx,
                                                            >,
                                                        > = Vec::new();
                                                            args.push(value_ptr.into());
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
                                } else if let Type::Protocol(protocol_name, _) = &*ty.borrow() {
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
                                                let mut args: Vec<
                                                    inkwell::values::BasicMetadataValueEnum<'ctx>,
                                                > = Vec::new();
                                                args.push(value_ptr.into());
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
                                && let Type::Enum(enum_name, _) = &*ty.borrow()
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
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Only simple function calls and method calls are supported",
                            None,
                        );
                        anyhow::bail!("Unsupported callee");
                    }
                };

                let callee_ty = match &*callee.borrow() {
                    Expression::Variable { ty, .. } => ty.clone(),
                    Expression::MemberAccess { ty, .. } => ty.clone(),
                    _ => None,
                };
                if let Some(ty) = callee_ty
                    && let Type::Function(param_tys, ret_ty, is_vararg) = &*ty.borrow()
                    && self.module.get_function(&function_name).is_none()
                {
                    let fn_ptr_val = self.resolve_expression(callee.clone())?.unwrap();
                    let fn_llvm_type =
                        self.get_function_type(ret_ty.clone(), param_tys.clone(), *is_vararg)?;
                    let fn_ptr = fn_ptr_val.into_pointer_value();

                    let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
                    for param in parameters {
                        let arg_val = self.resolve_expression(param.expression.clone())?.unwrap();
                        args.push(arg_val.into());
                    }

                    let call_result =
                        self.builder
                            .build_indirect_call(fn_llvm_type, fn_ptr, &args, "")?;
                    match call_result.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(val) => return Ok(Some(val)),
                        _ => return Ok(None),
                    }
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

                let instantiation_ptr: Option<(BasicTypeEnum<'ctx>, PointerValue<'ctx>)> =
                    if is_init_call {
                        if let Some(struct_name) = function_name.strip_suffix(".init") {
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
                        .contains_key(function_name.strip_suffix(".init").unwrap_or(""))
                    {
                        let val: BasicValueEnum<'ctx> = ptr.into();
                        Ok(Some(val))
                    } else {
                        let val = self.builder.build_load(ptr.get_type(), ptr, "")?;
                        Ok(Some(val))
                    }
                } else {
                    let call_val = match call_result.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(val) => val,
                        _ => return Ok(None),
                    };
                    let fn_ret_type = function.get_type().get_return_type();
                    if let Some(inkwell::types::BasicTypeEnum::PointerType(_)) = fn_ret_type {
                        if let BasicValueEnum::PointerValue(ptr_val) = call_val {
                            if let Ok(Some(call_ty)) = expr.borrow().get_ty_ref() {
                                let t_borrow = call_ty.borrow();
                                if !matches!(&*t_borrow, Type::Void) {
                                    if let Ok(expected_ty) = self.resolve_type(call_ty.clone()) {
                                        if expected_ty != ptr_val.get_type().into() {
                                            let loaded = self.builder.build_load(
                                                expected_ty,
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

                let (tag_val, enum_llvm_type) = if is_enum {
                    let enum_types = self.enum_types.borrow();
                    let enum_llvm_type = enum_types
                        .get(&enum_name)
                        .copied()
                        .ok_or_else(|| anyhow::anyhow!("Enum type '{}' not found", enum_name))?;
                    drop(enum_types);

                    let tag_ptr =
                        self.builder
                            .build_struct_gep(enum_llvm_type, subject_alloca, 0, "")?;
                    let tag_val = self
                        .builder
                        .build_load(self.context.i8_type(), tag_ptr, "")?;
                    (Some(tag_val.into_int_value()), Some(enum_llvm_type))
                } else {
                    (None, None)
                };

                let mut all_body_bbs = Vec::new();
                let mut all_check_bbs = Vec::new();

                for _ in cases.iter() {
                    all_body_bbs.push(self.context.append_basic_block(fn_val, "case_body"));
                    all_check_bbs.push(self.context.append_basic_block(fn_val, "case_check"));
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
                                    let expected_tag =
                                        self.context.i8_type().const_int(idx as u64, false);
                                    Some(self.builder.build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        tag_val.unwrap(),
                                        expected_tag,
                                        "",
                                    )?)
                                }
                                _ => None,
                            }
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

                    for stmt in &case.body {
                        let _ = self.resolve_statement(stmt.clone())?;
                    }
                    if case.guard.is_some() {
                        let _ = self.resolve_expression(case.guard.clone().unwrap());
                    }

                    self.builder.build_unconditional_branch(exit_bb)?;
                    self.exit_scope();

                    if i + 1 < all_check_bbs.len() {
                        self.builder.position_at_end(all_check_bbs[i + 1]);
                    }
                }

                self.builder.position_at_end(exit_bb);
                Ok(None)
            }
            Expression::Closure {
                parameters,
                return_type,
                body,
                ty,
                ..
            } => {
                let counter = {
                    let mut c = self.closure_counter.borrow_mut();
                    let val = *c;
                    *c += 1;
                    val
                };
                let fn_name = format!("__closure_{}", counter);

                let ret_type = return_type
                    .as_ref()
                    .and_then(|rt| {
                        let expr = rt.borrow();
                        expr.get_ty().ok().flatten()
                    })
                    .or_else(|| {
                        ty.as_ref().and_then(|t| {
                            if let Type::Function(_, ret_ty, _) = &*t.borrow() {
                                Some(ret_ty.clone())
                            } else {
                                None
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
                        .unwrap_or_else(|| Rc::new(RefCell::new(Type::Int32)));
                    param_types.push(pt);
                }

                let all_param_types: Vec<Rc<RefCell<Type>>> = ty
                    .as_ref()
                    .and_then(|t| {
                        if let Type::Function(pts, _, _) = &*t.borrow() {
                            Some(pts.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| param_types.clone());

                let fn_llvm_type =
                    self.get_function_type(ret_type.clone(), all_param_types.clone(), false)?;
                let function = self.module.add_function(&fn_name, fn_llvm_type, None);

                let current_block = self.builder.get_insert_block();
                let entry_block = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry_block);

                self.enter_scope_with_stmts(body)?;
                let mut param_idx = 0u32;
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
                        let param_ty = all_param_types
                            .get(idx as usize)
                            .cloned()
                            .unwrap_or_else(|| Rc::new(RefCell::new(Type::Int32)));
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

                let fn_ptr = function.as_global_value().as_pointer_value();
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                Ok(Some(
                    self.builder.build_bit_cast(fn_ptr, ptr_ty, "")?.into(),
                ))
            }
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
                let Type::Function(param_types, return_type, is_vararg) = &*ty.borrow() else {
                    return None;
                };
                Some((return_type.clone(), param_types.clone(), *is_vararg))
            })
            .collect();

        let (return_type, param_types, is_vararg) = method_types.into_iter().next()?;
        self.get_function_type(return_type, param_types, is_vararg)
            .ok()
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
                    Type::Struct(name, _) | Type::Class(name, _) | Type::Enum(name, _) => {
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
        if let Type::Enum(name, _) = &*ty.borrow() {
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
            Type::Int8 => self.context.i8_type().into(),
            Type::Int16 => self.context.i16_type().into(),
            Type::Int32 => self.context.i32_type().into(),
            Type::Int64 => self.context.i64_type().into(),
            Type::Int128 => self.context.i128_type().into(),
            Type::UInt8 => self.context.i8_type().into(),
            Type::UInt16 => self.context.i16_type().into(),
            Type::UInt32 => self.context.i32_type().into(),
            Type::UInt64 => self.context.i64_type().into(),
            Type::UInt128 => self.context.i128_type().into(),
            Type::Float32 => self.context.f32_type().into(),
            Type::Float64 => self.context.f64_type().into(),
            Type::Bool => self.context.bool_type().into(),
            Type::Char => self.context.i8_type().into(),
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
            Type::Function(_, _, _) => self.context.ptr_type(inkwell::AddressSpace::from(0)).into(),
            Type::Pointer(_) | Type::NonNullPointer(_) => self.context.ptr_type(inkwell::AddressSpace::from(0)).into(),
            Type::Struct(name, _) => {
                if let Some(struct_type) = self.struct_types.borrow().get(name) {
                    struct_type.as_basic_type_enum()
                } else {
                    self.emit_error(
                        TrussDiagnosticCode::StructTypeNotSupported,
                        format!("Struct type '{}' not found in IR generation", name),
                        None,
                    );
                    anyhow::bail!("Struct type not found");
                }
            }
            Type::Class(name, _) => {
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
            Type::Enum(name, _) => {
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
            Type::Protocol(name, _) => {
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
                    Type::Class(name, _) => {
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
            Expression::SizeOf { .. } => self.resolve_type(Rc::new(RefCell::new(Type::UInt64))),
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
}

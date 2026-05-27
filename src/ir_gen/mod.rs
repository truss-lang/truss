use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::Result;
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum},
    values::{BasicValue, BasicValueEnum, PointerValue},
};

use crate::{
    ast::{
        expression::{AssignmentOperator, BinaryOperator, CastKind, Expression, UnaryOperator},
        node::Program,
        statement::{Accessor, AccessorKind, FunctionBody, Parameter, Pattern, Statement, VariadicKind},
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine, new_diagnostic, primary_label_from_token},
    lexer::token::{Token, TokenType},
    scope::Scope as TrussScope,
    symbol::Symbol,
    types::Type,
};

struct Scope<'ctx> {
    variables: HashMap<String, PointerValue<'ctx>>,
}

impl<'ctx> Scope<'ctx> {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
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

    fn exit_scope(&self) {
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
            self.create_struct_type_bodies(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_class_type_bodies(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_enum_type_bodies(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_function_declarations(stmt.clone());
        }

        for stmt in &program.statements {
            self.create_vtable_instances(stmt.clone());
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
        }
    }

    fn is_stored_field(&self, stmt: &Rc<RefCell<Statement>>) -> bool {
        if let Statement::VariableDecl { accessors, .. } = &*stmt.borrow() {
            let has_get_set = accessors.iter().any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
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
        }
    }

    fn create_class_type_bodies(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::ClassDecl { name, .. } = &*statement.borrow() {
            let class_name = &name.value;
            if let Some(class_type) = self.class_types.borrow().get(class_name).cloned() {
                let ref_count_ty = self.context.i64_type().into();
                let vtable_ptr_ty: BasicTypeEnum<'ctx> =
                    self.context.ptr_type(inkwell::AddressSpace::from(0)).into();
                let mut field_types: Vec<BasicTypeEnum<'ctx>> = vec![ref_count_ty, vtable_ptr_ty];

                field_types.extend(self.collect_class_stored_field_types(class_name));

                class_type.set_body(&field_types, false);
            }
        }
    }

    fn collect_class_stored_field_types(&self, class_name: &str) -> Vec<BasicTypeEnum<'ctx>> {
        let binding = self.program_scope.borrow();
        let Some(scope) = binding.as_ref() else { return vec![] };
        let Some(symbol) = scope.borrow().get_symbol(class_name) else { return vec![] };

        let sym_borrow = symbol.borrow();
        let (decl, fields) = match &*sym_borrow {
            Symbol::Class { decl, fields, .. } => (decl.clone(), fields.clone()),
            _ => return vec![],
        };
        drop(sym_borrow);

        let mut field_types = Vec::new();

        if let Statement::ClassDecl { superclass: Some(super_expr), .. } = &*decl.borrow() {
            if let Expression::Type { name: super_name, .. } = &*super_expr.borrow() {
                field_types.extend(self.collect_class_stored_field_types(&super_name.value));
            }
        }

        for field in fields.iter() {
            if let Ok(Some(field_decl)) = field.borrow().get_decl() {
                if let Statement::VariableDecl { accessors, ty: Some(ty), .. } = &*field_decl.borrow() {
                    let has_get_set = accessors.iter().any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
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
        let Some(scope) = binding.as_ref() else { return };
        let Some(symbol) = scope.borrow().get_symbol(struct_name) else { return };
        let sym_borrow = symbol.borrow();
        let (decl, fields) = match &*sym_borrow {
            Symbol::Struct { decl, fields, .. } => (decl.clone(), fields.clone()),
            Symbol::Class { decl, fields, .. } => (decl.clone(), fields.clone()),
            _ => return,
        };
        drop(sym_borrow);

        let superclass_name = if is_class {
            if let Statement::ClassDecl { superclass: Some(super_expr), .. } = &*decl.borrow() {
                if let Expression::Type { name: super_name, .. } = &*super_expr.borrow() {
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
            for field in fields.iter() {
                let Ok(field_name) = field.borrow().name() else { continue };
                let Ok(Some(field_decl)) = field.borrow().get_decl() else { continue };
                let decl_ref = field_decl.borrow();
                let Statement::VariableDecl { accessors, ty: field_ty, .. } = &*decl_ref else { continue };
                let has_get_set = accessors.iter().any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
                if has_get_set {
                    stored_idx += 1;
                    continue;
                }
                let Some(field_ty) = field_ty else { stored_idx += 1; continue };
                if field_name == param_name {
                    let gep_idx = if is_class { stored_idx + 2 + superclass_field_count } else { stored_idx };
                    if is_class {
                        if let Some(class_type) = self.class_types.borrow().get(struct_name).copied() {
                            if let Ok(field_ptr) = self.builder.build_struct_gep(class_type, self_ptr, gep_idx as u32, "") {
                                if let Some(param_ptr) = self.lookup_variable(&param_name) {
                                    if let Ok(llvm_ty) = self.resolve_type(field_ty.clone()) {
                                        if let Ok(param_val) = self.builder.build_load(llvm_ty, param_ptr, "") {
                                            let _ = self.builder.build_store(field_ptr, param_val);
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(struct_type) = self.struct_types.borrow().get(struct_name).copied() {
                        if let Ok(field_ptr) = self.builder.build_struct_gep(struct_type, self_ptr, gep_idx as u32, "") {
                            if let Some(param_ptr) = self.lookup_variable(&param_name) {
                                if let Ok(llvm_ty) = self.resolve_type(field_ty.clone()) {
                                    if let Ok(param_val) = self.builder.build_load(llvm_ty, param_ptr, "") {
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

                if let Some(payload_type) = self.enum_payload_types.borrow().get(enum_name).cloned() {
                    payload_type.set_body(&case_types, false);
                }

                let tag_type = self.context.i8_type();
                if case_types.is_empty() {
                    enum_type.set_body(&[], false);
                } else if let Some(payload_type) =
                    self.enum_payload_types.borrow().get(enum_name).cloned()
                {
                    let field_types = vec![tag_type.as_basic_type_enum(), payload_type.as_basic_type_enum()];
                    enum_type.set_body(&field_types, false);
                }
            }
        }
    }

    fn get_stored_struct_field_index(&self, struct_name: &str, field_name: &str) -> Result<usize> {
        if let Some(scope) = self.program_scope.borrow().as_ref()
            && let Some(symbol) = scope.borrow().get_symbol(struct_name)
            && let Symbol::Struct { fields, .. } = &*symbol.borrow()
        {
            let mut stored_idx = 0;
            for field in fields.iter() {
                if let Some(decl) = field.borrow().get_decl().ok().flatten()
                    && let Statement::VariableDecl { accessors, .. } = &*decl.borrow()
                {
                    let has_get_set = accessors.iter().any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
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
        anyhow::bail!("Stored field '{}' not found in struct '{}'", field_name, struct_name)
    }

    fn get_stored_class_field_index(&self, class_name: &str, field_name: &str) -> Result<usize> {
        if let Some(scope) = self.program_scope.borrow().as_ref()
            && let Some(symbol) = scope.borrow().get_symbol(class_name)
        {
            let binding = symbol.borrow();
            let (decl, fields) = match &*binding {
                Symbol::Struct { decl, fields, .. } => (decl.clone(), fields.clone()),
                Symbol::Class { decl, fields, .. } => (decl.clone(), fields.clone()),
                _ => return Err(anyhow::anyhow!("Symbol '{}' is not a struct or class", class_name)),
            };
            drop(binding);

            let superclass_name = if let Statement::ClassDecl { superclass: Some(super_expr), .. } = &*decl.borrow() {
                if let Expression::Type { name: super_name, .. } = &*super_expr.borrow() {
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
            for field in fields.iter() {
                if let Some(field_decl) = field.borrow().get_decl().ok().flatten()
                    && let Statement::VariableDecl { accessors, .. } = &*field_decl.borrow()
                {
                    let has_get_set = accessors.iter().any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
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
        anyhow::bail!("Stored field '{}' not found in class '{}'", field_name, class_name)
    }

    fn get_class_stored_field_count(&self, class_name: &str) -> usize {
        let binding = self.program_scope.borrow();
        let Some(scope) = binding.as_ref() else { return 0 };
        let Some(symbol) = scope.borrow().get_symbol(class_name) else { return 0 };

        let sym_borrow = symbol.borrow();
        let (decl, fields) = match &*sym_borrow {
            Symbol::Class { decl, fields, .. } => (decl.clone(), fields.clone()),
            _ => return 0,
        };
        drop(sym_borrow);

        let mut count = 0;

        if let Statement::ClassDecl { superclass: Some(super_expr), .. } = &*decl.borrow() {
            if let Expression::Type { name: super_name, .. } = &*super_expr.borrow() {
                count += self.get_class_stored_field_count(&super_name.value);
            }
        }

        for field in fields.iter() {
            if let Ok(Some(field_decl)) = field.borrow().get_decl() {
                if let Statement::VariableDecl { accessors, .. } = &*field_decl.borrow() {
                    let has_get_set = accessors.iter().any(|a| matches!(a.kind, AccessorKind::Get | AccessorKind::Set));
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

    fn create_function_declarations(&self, statement: Rc<RefCell<Statement>>) {
        if let Statement::FunctionDecl { name, ty, body, .. } = &*statement.borrow() {
            if let Some(ty) = ty {
                let _ = self.create_function_declaration(name, ty);
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
                        let llvm_name = format!("{}.{}", name.value, method_name.value);
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
                        let llvm_name = format!("{}.{}", name.value, "init");
                        self.module.add_function(&llvm_name, function_type, None);
                    }
                }
            }
        }
        if let Statement::ClassDecl { name, body, .. } = &*statement.borrow() {
            for stmt in body {
                if let Statement::FunctionDecl {
                    name: method_name,
                    ty,
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
                        let llvm_name = format!("{}.{}", name.value, method_name.value);
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
                        let llvm_name = format!("{}.{}", name.value, "init");
                        self.module.add_function(&llvm_name, function_type, None);
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
        if let Expression::Block { statements, .. } = &*expr.borrow() {
            for stmt in statements {
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

    fn compute_vtable_method_list(&self, class_name: &str) -> Vec<(String, String)> {
        if let Some(cached) = self.vtable_method_lists.borrow().get(class_name) {
            return cached.clone();
        }
        let scope = self.program_scope.borrow();
        let Some(scope_ref) = scope.as_ref() else { return vec![] };
        let Some(symbol) = scope_ref.borrow().get_symbol(class_name) else { return vec![] };
        let sym_borrow = symbol.borrow();
        let Symbol::Class { methods, superclass, .. } = &*sym_borrow else { return vec![] };
        let own_method_names: Vec<String> = methods.iter().filter_map(|m| m.borrow().name().ok()).collect();

        let result = if let Some(super_weak) = superclass {
            if let Some(super_sym) = super_weak.0.upgrade() {
                let super_borrow = super_sym.borrow();
                if let Symbol::Class { name: super_name, .. } = &*super_borrow {
                    let super_methods = self.compute_vtable_method_list(&super_name);
                    let mut merged: Vec<(String, String)> = Vec::new();
                    for (method_name, owner) in super_methods {
                        if own_method_names.contains(&method_name) {
                            merged.push((method_name.clone(), class_name.to_string()));
                        } else {
                            merged.push((method_name, owner));
                        }
                    }
                    for method_name in &own_method_names {
                        if !merged.iter().any(|(n, _)| n == method_name) {
                            merged.push((method_name.clone(), class_name.to_string()));
                        }
                    }
                    merged
                } else {
                    own_method_names.iter().map(|n| (n.clone(), class_name.to_string())).collect()
                }
            } else {
                own_method_names.iter().map(|n| (n.clone(), class_name.to_string())).collect()
            }
        } else {
            own_method_names.iter().map(|n| (n.clone(), class_name.to_string())).collect()
        };

        drop(sym_borrow);

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
        }
    }

    fn generate_accessor_function(
        &self,
        fn_prefix: &str,
        backing_var_name: &str,
        accessor: &Accessor,
        llvm_var_type: BasicTypeEnum<'ctx>,
        struct_name: Option<&str>,
    ) -> Result<()> {
        let (fn_name, param_names, is_getter): (
            String,
            Vec<Option<String>>,
            bool,
        ) = match accessor.kind {
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

        if self.module.get_function(&fn_name).is_some() {
            return Ok(());
        }
        let function = self.module.add_function(&fn_name, fn_type, None);
        let current_block = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let ptr_param = function.get_nth_param(0).unwrap();
        let ptr_var = ptr_param.into_pointer_value();

        self.enter_scope();

        if struct_name.is_some() {
            *self.current_accessor_struct.borrow_mut() = Some((struct_name.unwrap().to_string(), ptr_var));
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

    fn resolve_block_expression(&self, block_expr: Rc<RefCell<Expression>>) -> Result<bool> {
        if let Expression::Block { statements, .. } = &*block_expr.borrow() {
            self.enter_scope_with_stmts(statements)?;
            self.resolve_block_stmts(statements)
        } else {
            Ok(false)
        }
    }

    fn resolve_statement(&self, statement: Rc<RefCell<Statement>>) -> Result<bool> {
        match &*statement.borrow() {
            Statement::VariableDecl {
                name, initializer, ty, accessors, ..
            } => {
                let llvm_var_type = if let Some(ty) = ty {
                    self.resolve_type(ty.clone())?
                } else {
                    self.context.i32_type().into()
                };

                if let Some(ptr) = self.lookup_variable(&name.value) {
                    if let Some(init) = initializer
                        && let Expression::Call {
                            callee, parameters, ..
                        } = &*init.borrow()
                        && let Expression::Variable {
                            name: callee_name, ..
                        } = &*callee.borrow()
                        && self.module.get_function(&callee_name.value).is_none()
                    {
                        let struct_name = &callee_name.value;
                        let fn_name = format!("{}.init", struct_name);
                        if let Some(function) = self.module.get_function(&fn_name) {
                            let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                                Vec::new();
                            args.push(ptr.into());
                            for param in parameters {
                                let arg_val =
                                    self.resolve_expression(param.expression.clone())?.unwrap();
                                args.push(arg_val.into());
                            }
                            self.builder.build_call(function, &args, "")?;
                        }
                    } else if let Some(init) = initializer
                        && let Some(init_val) = self.resolve_expression(init.clone())?
                    {
                        self.builder.build_store(ptr, init_val)?;
                    }
                }

                if !accessors.is_empty() {
                    let struct_name_opt = self.current_struct.borrow().clone();
                    let (fn_prefix, backing_name) = if let Some(ref sname) = struct_name_opt {
                        (format!("{}.{}", sname, name.value), name.value.clone())
                    } else {
                        (name.value.clone(), name.value.clone())
                    };
                    for accessor in accessors {
                        self.generate_accessor_function(&fn_prefix, &backing_name, accessor, llvm_var_type, struct_name_opt.as_deref())?;
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
                let terminates = self.resolve_block_expression(body.clone())?;

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
                let terminates = self.resolve_block_expression(body.clone())?;

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
                let terminates = self.resolve_block_expression(body.clone())?;

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
                self.resolve_block_expression(body.clone())?;
                Ok(false)
            }
            Statement::FunctionDecl {
                ty: Some(ty),
                name,
                parameters,
                body,
                ..
            } => {
                if let Type::Function(_parameter_types, return_type, _) = &*ty.borrow() {
                    let fn_name = if let Some(struct_name) = &*self.current_struct.borrow() {
                        format!("{}.{}", struct_name, name.value)
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
                        if is_struct_method || is_class_method {
                            let self_ptr = function.get_nth_param(0).unwrap();
                            let self_ptr = self_ptr.into_pointer_value();
                            self.declare_variable("self".to_string(), self_ptr);
                            let param_offset = 1;
                            for (i, param) in parameters.iter().enumerate() {
                                if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                    continue;
                                }
                                let param_name = &param.borrow().name.value;
                                let llvm_type = self.resolve_type(param.borrow().ty.clone().unwrap())?;
                                let alloca_name = self.unique_alloca_name(param_name);
                                let ptr = self.builder.build_alloca(llvm_type, &alloca_name)?;
                                let param_value = function.get_nth_param((i + param_offset) as u32).unwrap();
                                self.builder.build_store(ptr, param_value)?;
                                self.declare_variable(param_name.clone(), ptr);
                            }
                        } else {
                            for (i, param) in parameters.iter().enumerate() {
                                if param.borrow().variadic_kind == VariadicKind::BareVariadic {
                                    continue;
                                }
                                let param_name = &param.borrow().name.value;
                                let llvm_type = self.resolve_type(param.borrow().ty.clone().unwrap())?;
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
                            let llvm_type = self.resolve_type(param.borrow().ty.clone().unwrap())?;
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
                            for stmt in stmts {
                                let terminates = self.resolve_statement(stmt.clone())?;
                                if terminates {
                                    has_return = true;
                                    break;
                                }
                            }
                            if is_void && !has_return {
                                self.builder.build_return(None)?;
                            }
                            self.exit_scope();
                        }
                        FunctionBody::Expression(expr) => {
                            let value = self.resolve_expression(expr.clone())?.unwrap();
                            self.builder.build_return(Some(&value))?;
                        }
                        FunctionBody::None => {}
                    }

                    if let Some(block) = current_block {
                        self.builder.position_at_end(block);
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
            Statement::Return { value, .. } => {
                match value {
                    Some(value) if !matches!(&*value.borrow(), Expression::VoidLiteral { .. }) => {
                        let value = self.resolve_expression(value.clone())?.unwrap();
                        self.builder.build_return(Some(&value))?;
                    }
                    _ => {
                        self.builder.build_return(None)?;
                    }
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
            _ => Ok(false),
        }
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
            Expression::Block { statements, .. } => {
                self.enter_scope_with_stmts(statements)?;
                self.resolve_block_stmts(statements)?;

                Ok(Some(self.context.i32_type().const_int(0, false).into()))
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
                    && let Ok(field_ptr) = self.builder.build_struct_gep(stype, *struct_ptr, idx as u32, "")
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
            Expression::Binary {
                left,
                operator,
                right,
            } => {
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
                ..
            } => {
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
                            let willset_fn =
                                self.module.get_function(&willset_name).unwrap();
                            let args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                                vec![ptr.into(), store_val.into()];
                            self.builder.build_call(willset_fn, &args, "")?;
                        }
                        if has_setter {
                            let setter_fn =
                                self.module.get_function(&setter_name).unwrap();
                            let args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                                vec![ptr.into(), store_val.into()];
                            self.builder.build_call(setter_fn, &args, "")?;
                        } else {
                            self.builder.build_store(ptr, store_val)?;
                        }
                        if has_didset {
                            let didset_fn =
                                self.module.get_function(&didset_name).unwrap();
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
                    } else if let Some((sname, struct_ptr)) = &*self.current_accessor_struct.borrow()
                        && let Ok(idx) = self.get_stored_struct_field_index(sname, &name.value)
                        && let Some(stype) = self.struct_types.borrow().get(sname).copied()
                        && let Ok(field_ptr) = self.builder.build_struct_gep(stype, *struct_ptr, idx as u32, "")
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

                            let setter_name =
                                format!("{}.{}.setter", struct_name, field_name);
                            let willset_name =
                                format!("{}.{}.willSet", struct_name, field_name);
                            let didset_name =
                                format!("{}.{}.didSet", struct_name, field_name);

                            let has_setter =
                                self.module.get_function(&setter_name).is_some();
                            let has_willset =
                                self.module.get_function(&willset_name).is_some();
                            let has_didset =
                                self.module.get_function(&didset_name).is_some();

                            if has_setter {
                                if has_willset {
                                    let willset_fn =
                                        self.module.get_function(&willset_name).unwrap();
                                    self.builder.build_call(willset_fn, &[struct_ptr.into(), right_val.into()], "")?;
                                }
                                let setter_fn =
                                    self.module.get_function(&setter_name).unwrap();
                                self.builder.build_call(setter_fn, &[struct_ptr.into(), right_val.into()], "")?;
                                if has_didset {
                                    let didset_fn =
                                        self.module.get_function(&didset_name).unwrap();
                                    self.builder.build_call(didset_fn, &[struct_ptr.into(), right_val.into()], "")?;
                                }
                                return Ok(Some(right_val));
                            }

                            if has_willset || has_didset {
                                let field_index = self.get_stored_struct_field_index(&struct_name, &field_name)?;
                                let field_ptr = self.builder.build_struct_gep(
                                    *self.struct_types.borrow().get(&struct_name).unwrap(),
                                    struct_ptr,
                                    field_index as u32,
                                    "",
                                )?;
                                let field_ty = self.get_struct_field_type(&struct_name, &field_name)?;
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

                            let field_index = self.get_stored_struct_field_index(&struct_name, &field_name)?;
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

                            let field_index = self.get_stored_class_field_index(&class_name, &field_name)?;
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
                    let case_match_bb =
                        self.context.append_basic_block(fn_val, "case_match");

                    self.builder.build_unconditional_branch(case_match_bb)?;
                    self.builder.position_at_end(case_match_bb);

                    let subject_val =
                        self.resolve_expression(expression.clone())?.unwrap();
                    let subject_alloca = self
                        .builder
                        .build_alloca(subject_val.get_type(), "")?;
                    self.builder.build_store(subject_alloca, subject_val)?;

                    let enum_name = &enum_type.value;
                    let case_idx =
                        self.get_enum_case_index(enum_name, &case_name.value)?;

                    let enum_types = self.enum_types.borrow();
                    let enum_llvm_type =
                        enum_types.get(enum_name).copied().ok_or_else(|| {
                            anyhow::anyhow!("Enum type '{}' not found", enum_name)
                        })?;
                    drop(enum_types);

                    let tag_ptr = self.builder.build_struct_gep(
                        enum_llvm_type,
                        subject_alloca,
                        0,
                        "",
                    )?;
                    let tag_val =
                        self.builder.build_load(self.context.i8_type(), tag_ptr, "")?;
                    let expected_tag = self
                        .context
                        .i8_type()
                        .const_int(case_idx as u64, false);
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
                        if let Some(payload_type) =
                            enum_payloads.get(enum_name)
                        {
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
                            let case_payload_struct_ty =
                                case_payload_ty.into_struct_type();

                            self.enter_scope();
                            for (i, binding) in bindings.iter().enumerate() {
                                match binding {
                                    Pattern::Identifier(token) => {
                                        let field_ptr = self
                                            .builder
                                            .build_struct_gep(
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
                                        let field_val = self.builder.build_load(
                                            field_ty,
                                            field_ptr,
                                            "",
                                        )?;
                                        let var_ptr = self.builder.build_alloca(
                                            field_ty,
                                            &token.value,
                                        )?;
                                        self.builder
                                            .build_store(var_ptr, field_val)?;
                                        self.declare_variable(
                                            token.value.clone(),
                                            var_ptr,
                                        );
                                    }
                                    Pattern::Ignore => {}
                                    _ => {}
                                }
                            }
                            let terminates =
                                self.resolve_block_expression(then.clone())?;
                            if !terminates {
                                self.builder
                                    .build_unconditional_branch(exit_bb)?;
                            }
                            self.exit_scope();
                        } else {
                            let terminates =
                                self.resolve_block_expression(then.clone())?;
                            if !terminates {
                                self.builder
                                    .build_unconditional_branch(exit_bb)?;
                            }
                        }
                    } else {
                        let terminates =
                            self.resolve_block_expression(then.clone())?;
                        if !terminates {
                            self.builder.build_unconditional_branch(exit_bb)?;
                        }
                    }

                    if let Some(else_) = else_ {
                        self.builder.position_at_end(else_bb.unwrap());
                        let terminates = match &*else_.borrow() {
                            Expression::Block { .. } => {
                                self.resolve_block_expression(else_.clone())?
                            }
                            _ => {
                                self.resolve_expression(else_.clone())?;
                                false
                            }
                        };
                        if !terminates {
                            self.builder
                                .build_unconditional_branch(exit_bb)?;
                        }
                    }

                    self.builder.position_at_end(exit_bb);

                    Ok(None)
                } else {
                    let cond_bb =
                        self.context.append_basic_block(fn_val, "if_cond");
                    let then_bb =
                        self.context.append_basic_block(fn_val, "if_then");
                    let else_bb = if else_.is_some() {
                        Some(self.context.append_basic_block(fn_val, "if_else"))
                    } else {
                        None
                    };
                    let exit_bb =
                        self.context.append_basic_block(fn_val, "if_exit");

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
                    let terminates =
                        self.resolve_block_expression(then.clone())?;
                    if !terminates {
                        self.builder.build_unconditional_branch(exit_bb)?;
                    }

                    if let Some(else_) = else_ {
                        self.builder.position_at_end(else_bb.unwrap());
                        let terminates =
                            self.resolve_block_expression(else_.clone())?;
                        if !terminates {
                            self.builder
                                .build_unconditional_branch(exit_bb)?;
                        }
                    }

                    self.builder.position_at_end(exit_bb);

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
                        let result = self.builder.build_call(
                            getter_fn,
                            &[struct_ptr.into()],
                            "",
                        )?;
                        let result_val = match result.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(val) => val,
                            _ => anyhow::bail!("Getter call did not return a value"),
                        };
                        return Ok(Some(result_val));
                    }

                    let field_index = self.get_stored_struct_field_index(&struct_name, &field_name)?;

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

                    let field_index = self.get_stored_class_field_index(&class_name, &field_name)?;

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

                if let Some(ty) = &object_ty
                    && let Type::Enum(enum_name, _) = &*ty.borrow()
                {
                    let case_name = member.value.clone();
                    let enum_name = enum_name.clone();
                    let case_index = self.get_enum_case_index(&enum_name, &case_name)?;

                    let enum_types = self.enum_types.borrow();
                    let enum_llvm_type = enum_types.get(&enum_name).copied().ok_or_else(|| {
                        anyhow::anyhow!("Enum type '{}' not found", enum_name)
                    })?;
                    drop(enum_types);

                    let alloca = self.builder.build_alloca(enum_llvm_type.as_basic_type_enum(), "")?;

                    let tag_ptr = self.builder.build_struct_gep(enum_llvm_type, alloca, 0, "")?;
                    let tag_val = self.context.i8_type().const_int(case_index as u64, false);
                    self.builder.build_store(tag_ptr, tag_val)?;

                    let val = self.builder.build_load(enum_llvm_type.as_basic_type_enum(), alloca, "")?;
                    return Ok(Some(val));
                }

                if let Some(ty) = &object_ty
                    && let Type::Tuple(elements) = &*ty.borrow()
                {
                    let object_val = self.resolve_expression(object.clone())?.unwrap();
                    let tuple_llvm = self.resolve_type(object_ty.clone().unwrap())?.into_struct_type();

                    let struct_ptr = match object_val {
                        BasicValueEnum::PointerValue(ptr) => ptr,
                        val => {
                            let ptr = self.builder.build_alloca(val.get_type(), "")?;
                            self.builder.build_store(ptr, val)?;
                            ptr
                        }
                    };

                    // Try to find by name first
                    let member_name = &member.value;
                    let idx = if let Some(idx) = elements.iter().position(|(n, _)| {
                        n.as_ref().map_or(false, |name| name == member_name)
                    }) {
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
                    let field_ptr = self.builder.build_struct_gep(tuple_llvm, struct_ptr, idx as u32, "")?;
                    let val = self.builder.build_load(element_ty, field_ptr, "")?;
                    return Ok(Some(val));
                }

                self.emit_error(
                    TrussDiagnosticCode::UnsupportedFeature,
                    "Member access on non-struct type",
                    Some(member.as_ref()),
                );
                anyhow::bail!("Member access on non-struct type");
            }
            Expression::Call {
                callee, parameters, ..
            } => {
                let mut method_self_ptr: Option<PointerValue<'ctx>> = None;
                let (function_name, is_init_call) = match &*callee.borrow() {
                    Expression::Variable { name, .. } => {
                        let name = name.value.clone();
                        if self.module.get_function(&name).is_some() {
                            (name, false)
                        } else {
                            (format!("{}.init", name), true)
                        }
                    }
                    Expression::MemberAccess { object, member, .. } => {
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
                            (format!("{}.{}", struct_name, member.value), false)
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

                            if let Some(slot_idx) = self.get_vtable_slot_index(class_name, method_name) {
                                let class_type = *self.class_types.borrow().get(class_name).unwrap();
                                let vtable_ptr_ptr = self.builder.build_struct_gep(
                                    class_type, ptr, 1, "",
                                )?;
                                let vtable_ptr = self.builder.build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                    vtable_ptr_ptr, "",
                                )?.into_pointer_value();

                                let vtable_type = *self.vtable_types.borrow().get(class_name).unwrap();
                                let fn_ptr_ptr = self.builder.build_struct_gep(
                                    vtable_type, vtable_ptr, slot_idx, "",
                                )?;
                                let fn_ptr_val = self.builder.build_load(
                                    self.context.ptr_type(inkwell::AddressSpace::from(0)),
                                    fn_ptr_ptr, "",
                                )?.into_pointer_value();

                                let Some(owner) = self.compute_vtable_method_list(class_name)
                                    .iter()
                                    .find(|(n, _)| n == method_name)
                                    .map(|(_, owner)| owner.clone())
                                else {
                                    anyhow::bail!("Method {} not found in vtable for {}", method_name, class_name);
                                };
                                let fn_name = format!("{}.{}", owner, method_name);
                                let declared_fn = self.module.get_function(&fn_name).ok_or_else(|| {
                                    self.emit_error(
                                        TrussDiagnosticCode::UndefinedFunction,
                                        format!("Undefined function: '{}'", fn_name),
                                        None,
                                    );
                                    anyhow::anyhow!("Undefined function: {}", fn_name)
                                })?;
                                let fn_type = declared_fn.get_type();

                                let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
                                args.push(ptr.into());
                                for param in parameters {
                                    let arg_val = self.resolve_expression(param.expression.clone())?.unwrap();
                                    args.push(arg_val.into());
                                }
                                let call_result = self.builder.build_indirect_call(
                                    fn_type, fn_ptr_val, &args, "",
                                )?;
                                match call_result.try_as_basic_value() {
                                    inkwell::values::ValueKind::Basic(val) => return Ok(Some(val)),
                                    _ => return Ok(None),
                                }
                            }

                            method_self_ptr = Some(ptr);
                            (format!("{}.{}", class_name, method_name), false)
                        } else if let Some(ty) = &object_ty
                            && let Type::Enum(enum_name, _) = &*ty.borrow()
                        {
                            let case_name = member.value.clone();
                            let enum_name = enum_name.clone();
                            let case_index = self.get_enum_case_index(&enum_name, &case_name)?;

                            let enum_types = self.enum_types.borrow();
                            let case_llvm_type = enum_types.get(&enum_name).copied().ok_or_else(|| {
                                anyhow::anyhow!("Enum type '{}' not found", enum_name)
                            })?;
                            drop(enum_types);

                            let alloca = self.builder.build_alloca(case_llvm_type.as_basic_type_enum(), "")?;

                            let tag_ptr = self.builder.build_struct_gep(case_llvm_type, alloca, 0, "")?;
                            let tag_val = self.context.i8_type().const_int(case_index as u64, false);
                            self.builder.build_store(tag_ptr, tag_val)?;

                            if !parameters.is_empty() {
                                let payload_ptr = self.builder.build_struct_gep(case_llvm_type, alloca, 1, "")?;
                                let enum_payloads = self.enum_payload_types.borrow();
                                if let Some(payload_type) = enum_payloads.get(&enum_name) {
                                    for (i, param) in parameters.iter().enumerate() {
                                        let field_ptr = self.builder.build_struct_gep(*payload_type, payload_ptr, i as u32, "")?;
                                        let arg_val = self.resolve_expression(param.expression.clone())?.unwrap();
                                        self.builder.build_store(field_ptr, arg_val)?;
                                    }
                                }
                            }

                            let val = self.builder.build_load(case_llvm_type.as_basic_type_enum(), alloca, "")?;
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
                    _ => {
                        self.emit_error(
                            TrussDiagnosticCode::UnsupportedFeature,
                            "Only simple function calls and method calls are supported",
                            None,
                        );
                        anyhow::bail!("Unsupported callee");
                    }
                };

                let function = self.module.get_function(&function_name).ok_or_else(|| {
                    self.emit_error(
                        TrussDiagnosticCode::UndefinedFunction,
                        format!("Undefined function: '{}'", function_name),
                        None,
                    );
                    anyhow::anyhow!("Undefined function: {}", function_name)
                })?;

                let mut args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();

                let instantiation_ptr: Option<(BasicTypeEnum<'ctx>, PointerValue<'ctx>)> = if is_init_call {
                    if let Some(struct_name) = function_name.strip_suffix(".init") {
                        if let Some(class_type) = self.class_types.borrow().get(struct_name).cloned() {
                            let i64_ty = self.context.i64_type();
                            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                            let null_i8 = self.builder.build_int_to_ptr(
                                i64_ty.const_int(0, false),
                                ptr_ty,
                                "",
                            )?;
                            let class_ptr_ty = self.context.ptr_type(inkwell::AddressSpace::from(0));
                            let null_class_ptr = self.builder.build_pointer_cast(null_i8, class_ptr_ty, "")?;
                            let size_val = unsafe {
                                let gep = self.builder.build_gep(
                                    class_type.as_basic_type_enum(),
                                    null_class_ptr,
                                    &[i64_ty.const_int(1, false)],
                                    "",
                                )?;
                                self.builder.build_ptr_to_int(gep, i64_ty, "")?
                            };
                            let malloc_fn = self.module.get_function("malloc").unwrap_or_else(|| {
                                let fn_ty = self.context.i64_type().fn_type(&[self.context.i64_type().into()], false);
                                self.module.add_function("malloc", fn_ty, None)
                            });
                            let malloc_result = self.builder.build_call(
                                malloc_fn,
                                &[size_val.into()],
                                "",
                            )?;
                            let heap_ptr = match malloc_result.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(inkwell::values::BasicValueEnum::PointerValue(p)) => p,
                                _ => anyhow::bail!("malloc expected to return a pointer"),
                            };
                            let class_ptr = self.builder.build_pointer_cast(
                                heap_ptr,
                                class_ptr_ty,
                                "",
                            )?;
                            let rc_ptr = self.builder.build_struct_gep(class_type, class_ptr, 0, "")?;
                            self.builder.build_store(rc_ptr, i64_ty.const_int(1, false))?;

                            let vtable_global = self.vtable_globals.borrow().get(struct_name).copied();
                            if let Some(vt_global) = vtable_global {
                                let vtable_ptr_gep = self.builder.build_struct_gep(
                                    class_type, class_ptr, 1, "",
                                )?;
                                self.builder.build_store(
                                    vtable_ptr_gep,
                                    vt_global.as_pointer_value(),
                                )?;
                            }

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
                    args.push(arg_val.into());
                }

                let call_result = self.builder.build_call(function, &args, "")?;

                if let Some((_, ptr)) = instantiation_ptr {
                    if self.class_types.borrow().contains_key(
                        function_name.strip_suffix(".init").unwrap_or("")
                    ) {
                        let val: BasicValueEnum<'ctx> = ptr.into();
                        Ok(Some(val))
                    } else {
                        let val = self.builder.build_load(ptr.get_type(), ptr, "")?;
                        Ok(Some(val))
                    }
                } else {
                    match call_result.try_as_basic_value() {
                        inkwell::values::ValueKind::Basic(val) => Ok(Some(val)),
                        _ => Ok(None),
                    }
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
                        let field_ptr =
                            self.builder.build_struct_gep(tuple_llvm, alloca, i as u32, "")?;
                        self.builder.build_store(field_ptr, val)?;
                    }
                    let val = self.builder.build_load(tuple_llvm.as_basic_type_enum(), alloca, "")?;
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
                    let field_ptr =
                        self.builder.build_struct_gep(tuple_llvm, struct_ptr, idx as u32, "")?;
                    let val = self.builder.build_load(element_ty, field_ptr, "")?;
                    Ok(Some(val))
                } else {
                    anyhow::bail!("TupleIndexAccess on non-tuple type");
                }
            }
            _ => anyhow::bail!("Expression type not implemented"),
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
            Type::Function(_, _, _) => {
                self.emit_error(
                    TrussDiagnosticCode::NestedFunctionType,
                    "Nested function types are not supported",
                    None,
                );
                anyhow::bail!("Nested function types are not supported");
            }
            Type::Pointer(_) => self.context.ptr_type(inkwell::AddressSpace::from(0)).into(),
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
                self.emit_error(
                    TrussDiagnosticCode::UnsupportedFeature,
                    format!("Protocol type '{}' cannot be used directly in IR generation", name),
                    None,
                );
                anyhow::bail!("Protocol type not supported");
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
                    let struct_type =
                        self.context.opaque_struct_type(&format!("tuple.{}", type_key));
                    struct_type.set_body(&field_types, false);
                    self.struct_types
                        .borrow_mut()
                        .insert(type_key, struct_type);
                    struct_type.as_basic_type_enum()
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
            let (decl, fields) = match &*binding {
                Symbol::Struct { decl, fields, .. } => (decl.clone(), fields.clone()),
                Symbol::Class { decl, fields, .. } => (decl.clone(), fields.clone()),
                _ => return Err(anyhow::anyhow!("Symbol '{}' is not a struct or class", struct_name)),
            };
            drop(binding);

            for field in fields.iter() {
                if field.borrow().name().as_ref().ok() == Some(&field_name.to_string())
                    && let Some(field_decl) = field.borrow().get_decl().ok().flatten()
                    && let Statement::VariableDecl { ty: Some(ty), .. } = &*field_decl.borrow()
                {
                    return self.resolve_type(ty.clone());
                }
            }

            if let Statement::ClassDecl { superclass: Some(super_expr), .. } = &*decl.borrow() {
                if let Expression::Type { name: super_name, .. } = &*super_expr.borrow() {
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

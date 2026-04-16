use std::collections::HashMap;

use crate::ast::{
    BinaryOp, Block, BlockItem, Expression, ExternalDeclaration, Statement, TranslationUnit, Type,
};
use crate::error::CompilerError;

/// Backend interface for emitting target assembly from AST.
pub trait Codegen {
    /// Emits target output text from a semantic-validated AST.
    fn emit(&self, ast: &TranslationUnit) -> Result<String, CompilerError>;
}

/// Target-specific validation for unsupported features.
pub trait TargetValidation {
    /// Validates the AST contains only features supported by this target.
    fn validate(&self, _ast: &TranslationUnit) -> Result<(), CompilerError> {
        Ok(())
    }

    /// Binary operations not supported by this target.
    fn unsupported_binary_ops(&self) -> &'static [BinaryOp] {
        &[]
    }

    /// Statement types not supported by this target.
    fn unsupported_statement_kinds(&self) -> &'static [fn() -> Statement] {
        &[]
    }

    /// Checks for unsupported storage classes in declarations.
    fn unsupported_storage_classes(&self) -> &'static [fn() -> crate::ast::StorageClass] {
        &[|| crate::ast::StorageClass::Static]
    }
}

fn validate_ast(
    ast: &TranslationUnit,
    unsupported_ops: &[BinaryOp],
    _unsupported_stmts: &[fn() -> Statement],
    unsupported_storage: &[fn() -> crate::ast::StorageClass],
) -> Result<(), CompilerError> {
    for item in &ast.top_level_items {
        match item {
            ExternalDeclaration::Function(f) => validate_block(&f.body, unsupported_ops)?,
            ExternalDeclaration::GlobalDeclaration(d) => {
                if let Some(sc) = &d.storage_class {
                    for unsupported in unsupported_storage {
                        if *sc == unsupported() {
                            return Err(CompilerError::unsupported_at(
                                "static storage class not supported by target",
                                crate::error::SourceLocation { line: 1, column: 1 },
                            ));
                        }
                    }
                }
                for decl in &d.declarators {
                    if let Some(init) = &decl.initializer {
                        validate_expression(init, unsupported_ops)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_block(block: &Block, unsupported_ops: &[BinaryOp]) -> Result<(), CompilerError> {
    for item in &block.items {
        match item {
            BlockItem::Declaration(d) => {
                for decl in &d.declarators {
                    if let Some(init) = &decl.initializer {
                        validate_expression(init, unsupported_ops)?;
                    }
                }
            }
            BlockItem::Statement(s) => validate_statement(s, unsupported_ops)?,
        }
    }
    Ok(())
}

fn validate_statement(stmt: &Statement, unsupported_ops: &[BinaryOp]) -> Result<(), CompilerError> {
    match stmt {
        Statement::Block(b) => validate_block(b, unsupported_ops)?,
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            validate_expression(condition, unsupported_ops)?;
            validate_statement(then_branch, unsupported_ops)?;
            if let Some(else_b) = else_branch {
                validate_statement(else_b, unsupported_ops)?;
            }
        }
        Statement::While { condition, body } => {
            validate_expression(condition, unsupported_ops)?;
            validate_statement(body, unsupported_ops)?;
        }
        Statement::For {
            init,
            condition,
            update,
            body,
        } => {
            if let Some(i) = init {
                validate_expression(i, unsupported_ops)?;
            }
            if let Some(c) = condition {
                validate_expression(c, unsupported_ops)?;
            }
            if let Some(u) = update {
                validate_expression(u, unsupported_ops)?;
            }
            validate_statement(body, unsupported_ops)?;
        }
        Statement::Return(e) => {
            if let Some(expr) = e {
                validate_expression(expr, unsupported_ops)?;
            }
        }
        Statement::Expression(e) => {
            if let Some(expr) = e {
                validate_expression(expr, unsupported_ops)?;
            }
        }
        Statement::InlineAsm(_) => {}
    }
    Ok(())
}

fn validate_expression(
    expr: &Expression,
    unsupported_ops: &[BinaryOp],
) -> Result<(), CompilerError> {
    match expr {
        Expression::Binary {
            op,
            lhs,
            rhs,
            location,
        } => {
            if unsupported_ops.contains(op) {
                return Err(CompilerError::unsupported_with_location(
                    format!("operator {:?} not supported by target", op),
                    *location,
                ));
            }
            validate_expression(lhs, unsupported_ops)?;
            validate_expression(rhs, unsupported_ops)?;
        }
        Expression::Unary { expr: e, .. } => validate_expression(e, unsupported_ops)?,
        Expression::Assignment { target, value, .. } => {
            validate_expression(target, unsupported_ops)?;
            validate_expression(value, unsupported_ops)?;
        }
        Expression::Call { callee, args, .. } => {
            validate_expression(callee, unsupported_ops)?;
            for arg in args {
                validate_expression(arg, unsupported_ops)?;
            }
        }
        Expression::Index { base, index, .. } => {
            validate_expression(base, unsupported_ops)?;
            validate_expression(index, unsupported_ops)?;
        }
        Expression::ArrayInitializer { elements, .. } => {
            for elem in elements {
                validate_expression(elem, unsupported_ops)?;
            }
        }
        Expression::Identifier { .. } | Expression::IntegerLiteral { .. } => {}
    }
    Ok(())
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarieCodegen;

impl Codegen for MarieCodegen {
    /// Emits Marie assembly text from the provided AST.
    fn emit(&self, ast: &TranslationUnit) -> Result<String, CompilerError> {
        let mut emitter = MarieEmitter::default();
        emitter.emit_translation_unit(ast)?;
        Ok(emitter.finish())
    }
}

impl TargetValidation for MarieCodegen {
    fn unsupported_binary_ops(&self) -> &'static [BinaryOp] {
        &[
            BinaryOp::Divide,
            BinaryOp::ShiftLeft,
            BinaryOp::ShiftRight,
            BinaryOp::BitwiseAnd,
            BinaryOp::BitwiseOr,
            BinaryOp::BitwiseXor,
        ]
    }

    fn validate(&self, ast: &TranslationUnit) -> Result<(), CompilerError> {
        validate_ast(
            ast,
            self.unsupported_binary_ops(),
            self.unsupported_statement_kinds(),
            self.unsupported_storage_classes(),
        )
    }
}

#[derive(Debug, Default)]
struct MarieEmitter {
    instructions: Vec<String>,
    data: Vec<String>,
    functions: HashMap<String, PlannedFunction>,
    globals: HashMap<String, String>,
    int_consts: HashMap<i64, String>,
    addr_consts: HashMap<String, String>,
    label_counter: usize,
    has_zero_const: bool,
    has_one_const: bool,
    needs_mul_helper: bool,
    needs_mod_helper: bool,
}

#[derive(Debug, Clone)]
struct FunctionLabels {
    entry: String,
    body: String,
    end: String,
    ret: String,
}

#[derive(Debug, Clone)]
struct PlannedFunction {
    labels: FunctionLabels,
    param_labels: Vec<String>,
}

#[derive(Debug)]
struct FunctionEmitContext<'a> {
    function_name: &'a str,
    labels: &'a FunctionLabels,
    scopes: Vec<HashMap<String, String>>,
}

impl MarieEmitter {
    fn emit_translation_unit(&mut self, ast: &TranslationUnit) -> Result<(), CompilerError> {
        self.plan_symbols(ast);
        self.emit_start_entry();

        for item in &ast.top_level_items {
            match item {
                ExternalDeclaration::GlobalDeclaration(_) => {}
                ExternalDeclaration::Function(function) => self.emit_function(function)?,
            }
        }

        for item in &ast.top_level_items {
            if let ExternalDeclaration::GlobalDeclaration(declaration) = item {
                for declarator in &declaration.declarators {
                    let Some(label) = self.globals.get(&declarator.name).cloned() else {
                        continue;
                    };
                    self.emit_storage_for_declarator(
                        &label,
                        &declarator.ty,
                        declarator.initializer.as_ref(),
                    );
                }
            }
        }

        Ok(())
    }

    fn plan_symbols(&mut self, ast: &TranslationUnit) {
        for item in &ast.top_level_items {
            match item {
                ExternalDeclaration::GlobalDeclaration(declaration) => {
                    for declarator in &declaration.declarators {
                        self.globals
                            .insert(declarator.name.clone(), format!("g_{}", declarator.name));
                    }
                }
                ExternalDeclaration::Function(function) => {
                    let labels = FunctionLabels {
                        entry: format!("fn_{}", function.name),
                        body: format!("fn_{}_body", function.name),
                        end: format!("fn_{}_end", function.name),
                        ret: format!("fn_{}_ret", function.name),
                    };

                    let param_count = normalized_parameter_count(function);
                    let param_labels = (0..param_count)
                        .map(|index| format!("{}_param_{}", labels.entry, index))
                        .collect();

                    self.functions.insert(
                        function.name.clone(),
                        PlannedFunction {
                            labels,
                            param_labels,
                        },
                    );
                }
            }
        }
    }

    fn emit_start_entry(&mut self) {
        self.push_instructions([
            "_start, Clear".to_string(),
            "JnS fn_main".to_string(),
            "Load fn_main_ret".to_string(),
            "Output".to_string(),
            "Halt".to_string(),
        ]);
    }

    fn push_instructions<I>(&mut self, instructions: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.instructions.extend(instructions);
    }

    fn emit_function(
        &mut self,
        function: &crate::ast::FunctionDeclaration,
    ) -> Result<(), CompilerError> {
        let Some(planned) = self.functions.get(&function.name).cloned() else {
            return Err(CompilerError::semantic(format!(
                "missing function labels for '{}'",
                function.name
            )));
        };

        self.push_instructions([
            format!("{}, HEX 000", planned.labels.entry),
            format!("{}, Clear", planned.labels.body),
        ]);

        let mut context = FunctionEmitContext {
            function_name: &function.name,
            labels: &planned.labels,
            scopes: vec![HashMap::default()],
        };

        for (index, parameter) in function.params.iter().enumerate() {
            if let Some(name) = &parameter.name
                && let Some(param_label) = planned.param_labels.get(index)
            {
                context
                    .scopes
                    .last_mut()
                    .expect("function scope should exist")
                    .insert(name.clone(), param_label.clone());
            }
        }

        self.emit_block(&function.body, &mut context)?;

        self.push_instructions([
            format!("{}, Store {}", planned.labels.end, planned.labels.ret),
            format!("JumpI {}", planned.labels.entry),
        ]);

        self.data.push(format!("{}, DEC 0", planned.labels.ret));
        for param_label in &planned.param_labels {
            self.data.push(format!("{}, DEC 0", param_label));
        }

        Ok(())
    }

    fn emit_block(
        &mut self,
        block: &Block,
        context: &mut FunctionEmitContext<'_>,
    ) -> Result<(), CompilerError> {
        for item in &block.items {
            match item {
                crate::ast::BlockItem::Declaration(declaration) => {
                    for declarator in &declaration.declarators {
                        let local_label = format!(
                            "v_{}_{}_{}",
                            context.function_name,
                            self.next_label_id(),
                            declarator.name
                        );
                        self.emit_storage_for_declarator(
                            &local_label,
                            &declarator.ty,
                            declarator.initializer.as_ref(),
                        );
                        context
                            .scopes
                            .last_mut()
                            .expect("scope should exist")
                            .insert(declarator.name.clone(), local_label.clone());

                        if let Some(initializer) = &declarator.initializer {
                            if !matches!(initializer, Expression::ArrayInitializer { .. }) {
                                self.emit_expression(initializer, context)?;
                                self.instructions.push(format!("Store {}", local_label));
                            }
                        }
                    }
                }
                crate::ast::BlockItem::Statement(statement) => {
                    self.emit_statement(statement, context)?;
                }
            }
        }

        Ok(())
    }

    fn emit_statement(
        &mut self,
        statement: &Statement,
        context: &mut FunctionEmitContext<'_>,
    ) -> Result<(), CompilerError> {
        match statement {
            Statement::Block(block) => {
                context.scopes.push(HashMap::default());
                self.emit_block(block, context)?;
                context.scopes.pop();
                Ok(())
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let branch_id = self.next_label_id();
                let else_label = format!("if_{}_else", branch_id);
                let end_label = format!("if_{}_end", branch_id);

                self.emit_expression(condition, context)?;
                self.push_instructions([
                    "Skipcond 0C00".to_string(),
                    format!("Jump {}", else_label),
                ]);
                self.emit_statement(then_branch, context)?;
                self.push_instructions([
                    format!("Jump {}", end_label),
                    format!("{}, Clear", else_label),
                ]);
                if let Some(else_branch) = else_branch {
                    self.emit_statement(else_branch, context)?;
                }
                self.instructions
                    .push(format!("{}, Add const_zero", end_label));
                Ok(())
            }
            Statement::Return(expression) => {
                if let Some(expression) = expression {
                    self.emit_expression(expression, context)?;
                } else {
                    self.instructions.push("Clear".to_string());
                }
                self.push_instructions([format!("Jump {}", context.labels.end)]);
                Ok(())
            }
            Statement::Expression(expression) => {
                if let Some(expression) = expression {
                    self.emit_expression(expression, context)?;
                }
                Ok(())
            }
            Statement::InlineAsm(instructions) => {
                self.emit_inline_asm(instructions, context)?;
                Ok(())
            }
            Statement::While { condition, body } => {
                let loop_id = self.next_label_id();
                let cond_label = format!("while_cond_{}", loop_id);
                let end_label = format!("while_end_{}", loop_id);

                self.push_instructions([format!("Jump {}", cond_label)]);
                self.instructions.push(format!("{}, Clear", cond_label));
                self.emit_expression(condition, context)?;
                self.push_instructions([
                    "Skipcond 0C00".to_string(),
                    format!("Jump {}", end_label),
                ]);
                self.emit_statement(body, context)?;
                self.push_instructions([format!("Jump {}", cond_label)]);
                self.instructions.push(format!("{}, Clear", end_label));
                Ok(())
            }
            Statement::For {
                init,
                condition,
                update,
                body,
            } => {
                let loop_id = self.next_label_id();
                let cond_label = format!("for_cond_{}", loop_id);
                let end_label = format!("for_end_{}", loop_id);

                if let Some(init) = init {
                    self.emit_expression(init, context)?;
                }
                if let Some(cond) = condition {
                    self.push_instructions([format!("Jump {}", cond_label)]);
                    self.instructions.push(format!("{}, Clear", cond_label));
                    self.emit_expression(cond, context)?;
                    self.push_instructions([
                        "Skipcond 0C00".to_string(),
                        format!("Jump {}", end_label),
                    ]);
                } else {
                    self.push_instructions([format!("Jump {}", cond_label)]);
                    self.instructions.push(format!("{}, Clear", cond_label));
                }
                self.emit_statement(body, context)?;
                if let Some(upd) = update {
                    self.emit_expression(upd, context)?;
                }
                self.push_instructions([format!("Jump {}", cond_label)]);
                self.instructions.push(format!("{}, Clear", end_label));
                Ok(())
            }
        }
    }

    fn emit_expression(
        &mut self,
        expression: &Expression,
        context: &mut FunctionEmitContext<'_>,
    ) -> Result<(), CompilerError> {
        match expression {
            Expression::Identifier { name, .. } => {
                let label = self.resolve_symbol_label(context, name)?;
                self.instructions.push(format!("Load {}", label));
                Ok(())
            }
            Expression::IntegerLiteral { value, .. } => {
                let label = self.ensure_int_const(*value);
                self.instructions.push(format!("Load {}", label));
                Ok(())
            }
            Expression::Unary { op, expr, .. } => {
                use crate::ast::UnaryOp;
                match op {
                    UnaryOp::LogicalNot => {
                        self.emit_expression(expr, context)?;
                        let zero_label = self.ensure_int_const(0);
                        let one_label = self.ensure_int_const(1);
                        let true_label = format!("unary_not_{}_true", self.next_label_id());
                        let end_label = format!("unary_not_{}_end", self.next_label_id());
                        self.push_instructions([
                            "Skipcond 400".to_string(),
                            format!("Jump {}", true_label),
                            format!("Load {}", one_label),
                            format!("Jump {}", end_label),
                            format!("{}, Load {}", true_label, zero_label),
                            format!("{}, Add const_zero", end_label),
                        ]);
                    }
                    UnaryOp::AddressOf => {
                        self.emit_address_of(expr, context)?;
                    }
                    UnaryOp::Dereference => {
                        self.ensure_index_cells();
                        self.emit_expression(expr, context)?;
                        self.push_instructions([
                            "Store helper_addr".to_string(),
                            "LoadI helper_addr".to_string(),
                        ]);
                    }
                    UnaryOp::Plus => {
                        self.emit_expression(expr, context)?;
                    }
                    UnaryOp::Minus => {
                        self.emit_expression(expr, context)?;
                        self.ensure_zero_const();
                        self.ensure_index_cells();
                        self.instructions
                            .push("Store helper_store_value".to_string());
                        self.push_instructions([
                            "Load const_zero".to_string(),
                            "Subt helper_store_value".to_string(),
                        ]);
                    }
                }
                Ok(())
            }
            Expression::Binary { lhs, rhs, op, .. } => {
                self.emit_expression(lhs, context)?;
                let lhs_temp = format!("tmp_{}", self.next_label_id());
                self.data.push(format!("{}, DEC 0", lhs_temp));
                self.instructions.push(format!("Store {}", lhs_temp));

                self.emit_expression(rhs, context)?;
                let rhs_temp = format!("tmp_{}", self.next_label_id());
                self.data.push(format!("{}, DEC 0", rhs_temp));
                self.instructions.push(format!("Store {}", rhs_temp));

                self.instructions.push(format!("Load {}", lhs_temp));

                use crate::ast::BinaryOp;
                match op {
                    BinaryOp::Add => self.instructions.push(format!("Add {}", rhs_temp)),
                    BinaryOp::Subtract => self.instructions.push(format!("Subt {}", rhs_temp)),
                    BinaryOp::Equal => self.emit_compare_equal(&lhs_temp, &rhs_temp),
                    BinaryOp::NotEqual => self.emit_compare_not_equal(&lhs_temp, &rhs_temp),
                    BinaryOp::Less => self.emit_compare_less(&lhs_temp, &rhs_temp),
                    BinaryOp::LessEqual => self.emit_compare_less_equal(&lhs_temp, &rhs_temp),
                    BinaryOp::Greater => self.emit_compare_greater(&lhs_temp, &rhs_temp),
                    BinaryOp::GreaterEqual => self.emit_compare_greater_equal(&lhs_temp, &rhs_temp),
                    BinaryOp::LogicalAnd => self.emit_logical_and(&lhs_temp, &rhs_temp),
                    BinaryOp::LogicalOr => self.emit_logical_or(&lhs_temp, &rhs_temp),
                    BinaryOp::Multiply => self.emit_mul_call(&lhs_temp, &rhs_temp),
                    BinaryOp::Modulo => self.emit_mod_call(&lhs_temp, &rhs_temp),
                    BinaryOp::Divide
                    | BinaryOp::ShiftLeft
                    | BinaryOp::ShiftRight
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseOr
                    | BinaryOp::BitwiseXor => {
                        return Err(CompilerError::unsupported(format!(
                            "binary operator {:?} not supported by target",
                            op
                        )));
                    }
                }
                Ok(())
            }
            Expression::Assignment { target, value, .. } => {
                self.emit_expression(value, context)?;
                match &**target {
                    Expression::Identifier { name, .. } => {
                        let label = self.resolve_symbol_label(context, name)?;
                        self.instructions.push(format!("Store {}", label));
                    }
                    Expression::Index { base, index, .. } => {
                        self.ensure_index_cells();
                        self.instructions
                            .push("Store helper_store_value".to_string());
                        self.emit_index_address(base, index, context)?;
                        self.push_instructions([
                            "Load helper_store_value".to_string(),
                            "StoreI helper_addr".to_string(),
                        ]);
                    }
                    Expression::Unary {
                        op: crate::ast::UnaryOp::Dereference,
                        expr,
                        ..
                    } => {
                        self.ensure_index_cells();
                        self.instructions
                            .push("Store helper_store_value".to_string());
                        self.emit_expression(expr, context)?;
                        self.push_instructions([
                            "Store helper_addr".to_string(),
                            "Load helper_store_value".to_string(),
                            "StoreI helper_addr".to_string(),
                        ]);
                    }
                    _ => {
                        return Err(CompilerError::semantic(
                            "unsupported assignment target in codegen",
                        ));
                    }
                }
                Ok(())
            }
            Expression::Call { callee, args, .. } => {
                let function_name = if let Expression::Identifier { name, .. } = &**callee {
                    name
                } else {
                    return Err(CompilerError::semantic(
                        "call target must be a function identifier",
                    ));
                };

                let Some(callee_plan) = self.functions.get(function_name).cloned() else {
                    return Err(CompilerError::semantic(format!(
                        "missing planned function '{}' in codegen",
                        function_name
                    )));
                };

                for (index, argument) in args.iter().enumerate() {
                    self.emit_expression(argument, context)?;
                    if let Some(param_label) = callee_plan.param_labels.get(index) {
                        self.instructions.push(format!("Store {}", param_label));
                    }
                }

                self.instructions
                    .push(format!("JnS {}", callee_plan.labels.entry));
                self.instructions
                    .push(format!("Load {}", callee_plan.labels.ret));
                Ok(())
            }
            Expression::Index { base, index, .. } => {
                self.emit_index_address(base, index, context)?;
                self.instructions.push("LoadI helper_addr".to_string());
                Ok(())
            }
            Expression::ArrayInitializer { .. } => {
                Err(CompilerError::semantic(
                    "array initializer should not be emitted directly".to_string(),
                ))
            }
        }
    }

    fn emit_storage_for_declarator(
        &mut self,
        label: &str,
        ty: &Type,
        initializer: Option<&Expression>,
    ) {
        if let Type::Array { size, .. } = ty {
            let count = size
                .and_then(|const_expr| usize::try_from(const_expr.value).ok())
                .filter(|value| *value > 0)
                .unwrap_or(1);
            let first_elem_label = format!("{}_elem_0", label);
            self.data
                .push(format!("{}, ADR {}", label, first_elem_label));

            let mut initial_values: Vec<i64> = Vec::new();
            if let Some(Expression::ArrayInitializer { elements, .. }) = initializer {
                for element in elements {
                    if let Expression::IntegerLiteral { value, .. } = element {
                        initial_values.push(*value);
                    } else {
                        break;
                    }
                }
            }

            for index in 0..count {
                let element_label = format!("{}_elem_{}", label, index);
                let value = initial_values.get(index).copied().unwrap_or(0);
                self.data.push(format!("{}, DEC {}", element_label, value));
            }
            return;
        }

        let value = if let Some(Expression::IntegerLiteral { value, .. }) = initializer {
            *value
        } else {
            0
        };

        self.data.push(format!("{}, DEC {}", label, value));
    }

    fn ensure_zero_const(&mut self) {
        if !self.has_zero_const {
            self.data.push("const_zero, DEC 0".to_string());
            self.has_zero_const = true;
        }
    }

    fn ensure_one_const(&mut self) {
        if !self.has_one_const {
            self.data.push("const_one, DEC 1".to_string());
            self.has_one_const = true;
        }
    }

    fn ensure_int_const(&mut self, value: i64) -> String {
        if value == 0 {
            self.ensure_zero_const();
            return "const_zero".to_string();
        }

        if value == 1 {
            self.ensure_one_const();
            return "const_one".to_string();
        }

        if let Some(label) = self.int_consts.get(&value) {
            return label.clone();
        }

        let label = if value < 0 {
            format!("const_int_neg_{}", value.abs())
        } else {
            format!("const_int_{}", value)
        };

        self.data.push(format!("{}, DEC {}", label, value));
        self.int_consts.insert(value, label.clone());
        label
    }

    fn ensure_addr_const(&mut self, label: &str) -> String {
        if let Some(existing) = self.addr_consts.get(label) {
            return existing.clone();
        }

        let addr_label = format!("addr_{}", label);
        self.data.push(format!("{}, ADR {}", addr_label, label));
        self.addr_consts
            .insert(label.to_string(), addr_label.clone());
        addr_label
    }

    fn ensure_index_cells(&mut self) {
        if !self
            .data
            .iter()
            .any(|line| line.starts_with("helper_addr,"))
        {
            self.data.push("helper_addr, DEC 0".to_string());
            self.data.push("helper_index, DEC 0".to_string());
            self.data.push("helper_store_value, DEC 0".to_string());
        }
    }

    fn emit_index_address(
        &mut self,
        base: &Expression,
        index: &Expression,
        context: &mut FunctionEmitContext<'_>,
    ) -> Result<(), CompilerError> {
        self.ensure_index_cells();

        self.emit_expression(index, context)?;
        self.instructions.push("Store helper_index".to_string());

        self.emit_expression(base, context)?;
        self.push_instructions([
            "Add helper_index".to_string(),
            "Store helper_addr".to_string(),
        ]);
        Ok(())
    }

    fn emit_address_of(
        &mut self,
        expression: &Expression,
        context: &mut FunctionEmitContext<'_>,
    ) -> Result<(), CompilerError> {
        use crate::ast::UnaryOp;

        match expression {
            Expression::Identifier { name, .. } => {
                let label = self.resolve_symbol_label(context, name)?;
                let addr_label = self.ensure_addr_const(&label);
                self.instructions.push(format!("Load {}", addr_label));
                Ok(())
            }
            Expression::Index { base, index, .. } => {
                self.emit_index_address(base, index, context)?;
                self.instructions.push("Load helper_addr".to_string());
                Ok(())
            }
            Expression::Unary {
                op: UnaryOp::Dereference,
                expr,
                ..
            } => {
                self.emit_expression(expr, context)?;
                Ok(())
            }
            _ => Err(CompilerError::semantic(
                "unsupported address-of target in codegen",
            )),
        }
    }

    fn emit_mul_call(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.needs_mul_helper = true;
        self.instructions.push(format!("Load {}", lhs_temp));
        self.instructions.push("Store helper_mul_lhs".to_string());
        self.instructions.push(format!("Load {}", rhs_temp));
        self.instructions.push("Store helper_mul_rhs".to_string());
        self.instructions.push("JnS helper_mul".to_string());
        self.instructions.push("Load helper_mul_ret".to_string());
    }

    fn emit_mod_call(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.needs_mod_helper = true;
        self.instructions.push(format!("Load {}", lhs_temp));
        self.instructions.push("Store helper_mod_lhs".to_string());
        self.instructions.push(format!("Load {}", rhs_temp));
        self.instructions.push("Store helper_mod_rhs".to_string());
        self.instructions.push("JnS helper_mod".to_string());
        self.instructions.push("Load helper_mod_ret".to_string());
    }

    fn emit_compare_equal(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.ensure_zero_const();
        self.ensure_one_const();
        let false_label = format!("cmp_eq_{}_false", self.next_label_id());
        let end_label = format!("cmp_eq_{}_end", self.next_label_id());

        self.push_instructions([
            format!("Load {}", lhs_temp),
            format!("Subt {}", rhs_temp),
            "Skipcond 400".to_string(),
            format!("Jump {}", false_label),
            "Load const_one".to_string(),
            format!("Jump {}", end_label),
            format!("{}, Load const_zero", false_label),
            format!("{}, Add const_zero", end_label),
        ]);
    }

    fn emit_compare_not_equal(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.ensure_zero_const();
        self.ensure_one_const();
        let true_label = format!("cmp_ne_{}_true", self.next_label_id());
        let end_label = format!("cmp_ne_{}_end", self.next_label_id());

        self.push_instructions([
            format!("Load {}", lhs_temp),
            format!("Subt {}", rhs_temp),
            "Skipcond 400".to_string(),
            format!("Jump {}", true_label),
            "Load const_zero".to_string(),
            format!("Jump {}", end_label),
            format!("{}, Load const_one", true_label),
            format!("{}, Add const_zero", end_label),
        ]);
    }

    fn emit_compare_less(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.ensure_zero_const();
        self.ensure_one_const();
        let false_label = format!("cmp_lt_{}_false", self.next_label_id());
        let end_label = format!("cmp_lt_{}_end", self.next_label_id());

        self.push_instructions([
            format!("Load {}", lhs_temp),
            format!("Subt {}", rhs_temp),
            "Skipcond 000".to_string(),
            format!("Jump {}", false_label),
            "Load const_one".to_string(),
            format!("Jump {}", end_label),
            format!("{}, Load const_zero", false_label),
            format!("{}, Add const_zero", end_label),
        ]);
    }

    fn emit_compare_less_equal(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.ensure_zero_const();
        self.ensure_one_const();
        let true_label = format!("cmp_le_{}_true", self.next_label_id());
        let end_label = format!("cmp_le_{}_end", self.next_label_id());

        self.push_instructions([
            format!("Load {}", lhs_temp),
            format!("Subt {}", rhs_temp),
            "Skipcond 000".to_string(),
            format!("Jump {}", true_label),
            "Skipcond 400".to_string(),
            format!("Jump {}", true_label),
            "Load const_zero".to_string(),
            format!("Jump {}", end_label),
            format!("{}, Load const_one", true_label),
            format!("{}, Add const_zero", end_label),
        ]);
    }

    fn emit_compare_greater(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.ensure_zero_const();
        self.ensure_one_const();
        let false_label = format!("cmp_gt_{}_false", self.next_label_id());
        let end_label = format!("cmp_gt_{}_end", self.next_label_id());

        self.push_instructions([
            format!("Load {}", lhs_temp),
            format!("Subt {}", rhs_temp),
            "Skipcond 800".to_string(),
            format!("Jump {}", false_label),
            "Load const_one".to_string(),
            format!("Jump {}", end_label),
            format!("{}, Load const_zero", false_label),
            format!("{}, Add const_zero", end_label),
        ]);
    }

    fn emit_compare_greater_equal(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.ensure_zero_const();
        self.ensure_one_const();
        let true_label = format!("cmp_ge_{}_true", self.next_label_id());
        let end_label = format!("cmp_ge_{}_end", self.next_label_id());

        self.push_instructions([
            format!("Load {}", lhs_temp),
            format!("Subt {}", rhs_temp),
            "Skipcond 800".to_string(),
            format!("Jump {}", true_label),
            "Skipcond 400".to_string(),
            format!("Jump {}", true_label),
            "Load const_zero".to_string(),
            format!("Jump {}", end_label),
            format!("{}, Load const_one", true_label),
            format!("{}, Add const_zero", end_label),
        ]);
    }

    fn emit_logical_and(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.ensure_zero_const();
        self.ensure_one_const();
        let true_label = format!("logic_and_{}_true", self.next_label_id());
        let end_label = format!("logic_and_{}_end", self.next_label_id());

        self.push_instructions([
            format!("Load {}", lhs_temp),
            "Skipcond 0C00".to_string(),
            format!("Jump {}", end_label),
            format!("Load {}", rhs_temp),
            "Skipcond 0C00".to_string(),
            format!("Jump {}", end_label),
            format!("{}, Load const_one", true_label),
            format!("Jump {}", end_label),
            format!("{}, Load const_zero", end_label),
        ]);
    }

    fn emit_logical_or(&mut self, lhs_temp: &str, rhs_temp: &str) {
        self.ensure_zero_const();
        self.ensure_one_const();
        let true_label = format!("logic_or_{}_true", self.next_label_id());
        let end_label = format!("logic_or_{}_end", self.next_label_id());

        self.push_instructions([
            format!("Load {}", lhs_temp),
            "Skipcond 0C00".to_string(),
            format!("Jump {}", true_label),
            format!("Load {}", rhs_temp),
            "Skipcond 0C00".to_string(),
            format!("Jump {}", true_label),
            "Load const_zero".to_string(),
            format!("Jump {}", end_label),
            format!("{}, Load const_one", true_label),
            format!("{}, Add const_zero", end_label),
        ]);
    }

    fn emit_inline_asm(
        &mut self,
        instructions: &[String],
        context: &FunctionEmitContext<'_>,
    ) -> Result<(), CompilerError> {
        for instruction in instructions {
            for line in instruction.lines() {
                let rendered = self.render_inline_asm_line(line, context)?;
                if !rendered.trim().is_empty() {
                    self.instructions.push(rendered);
                }
            }
        }

        Ok(())
    }

    fn render_inline_asm_line(
        &self,
        line: &str,
        context: &FunctionEmitContext<'_>,
    ) -> Result<String, CompilerError> {
        let mut output = String::with_capacity(line.len());
        let bytes = line.as_bytes();
        let mut index = 0;

        while index < bytes.len() {
            if bytes[index] == b'%' {
                let start = index + 1;
                if start < bytes.len() && is_identifier_start_byte(bytes[start]) {
                    let mut end = start + 1;
                    while end < bytes.len() && is_identifier_continue_byte(bytes[end]) {
                        end += 1;
                    }

                    let name = &line[start..end];
                    let label = self.resolve_symbol_label(context, name)?;
                    output.push_str(&label);
                    index = end;
                    continue;
                }
            }

            output.push(bytes[index] as char);
            index += 1;
        }

        Ok(output)
    }

    fn resolve_symbol_label(
        &self,
        context: &FunctionEmitContext<'_>,
        name: &str,
    ) -> Result<String, CompilerError> {
        for scope in context.scopes.iter().rev() {
            if let Some(label) = scope.get(name) {
                return Ok(label.clone());
            }
        }

        if let Some(global_label) = self.globals.get(name) {
            return Ok(global_label.clone());
        }

        Err(CompilerError::semantic(format!(
            "unresolved symbol '{}' during codegen",
            name
        )))
    }

    fn next_label_id(&mut self) -> usize {
        let current = self.label_counter;
        self.label_counter += 1;
        current
    }

    fn emit_helpers(&mut self) {
        if self.needs_mul_helper {
            self.emit_mul_helper();
        }
        if self.needs_mod_helper {
            self.emit_mod_helper();
        }
    }

    fn emit_mul_helper(&mut self) {
        self.ensure_zero_const();
        self.ensure_one_const();

        if self
            .instructions
            .iter()
            .any(|line| line.starts_with("helper_mul,"))
        {
            return;
        }

        self.instructions.push("helper_mul, HEX 000".to_string());
        self.instructions.push("helper_mul_body, Clear".to_string());
        self.instructions.push("Store helper_mul_acc".to_string());
        self.instructions
            .push("helper_mul_loop, Load helper_mul_rhs".to_string());
        self.instructions.push("Skipcond 400".to_string());
        self.instructions
            .push("Jump helper_mul_continue".to_string());
        self.instructions.push("Jump helper_mul_done".to_string());
        self.instructions
            .push("helper_mul_continue, Load helper_mul_acc".to_string());
        self.instructions.push("Add helper_mul_lhs".to_string());
        self.instructions.push("Store helper_mul_acc".to_string());
        self.instructions.push("Load helper_mul_rhs".to_string());
        self.instructions.push("Subt const_one".to_string());
        self.instructions.push("Store helper_mul_rhs".to_string());
        self.instructions.push("Jump helper_mul_loop".to_string());
        self.instructions
            .push("helper_mul_done, Load helper_mul_acc".to_string());
        self.instructions.push("Store helper_mul_ret".to_string());
        self.instructions.push("JumpI helper_mul".to_string());

        if !self
            .data
            .iter()
            .any(|line| line.starts_with("helper_mul_lhs,"))
        {
            self.data.push("helper_mul_lhs, DEC 0".to_string());
            self.data.push("helper_mul_rhs, DEC 0".to_string());
            self.data.push("helper_mul_acc, DEC 0".to_string());
            self.data.push("helper_mul_ret, DEC 0".to_string());
        }
    }

    fn emit_mod_helper(&mut self) {
        self.ensure_zero_const();

        if self
            .instructions
            .iter()
            .any(|line| line.starts_with("helper_mod,"))
        {
            return;
        }

        self.instructions.push("helper_mod, HEX 000".to_string());
        self.instructions
            .push("helper_mod_body, Load helper_mod_lhs".to_string());
        self.instructions.push("Store helper_mod_work".to_string());
        self.instructions
            .push("helper_mod_loop, Load helper_mod_work".to_string());
        self.instructions.push("Subt helper_mod_rhs".to_string());
        self.instructions.push("Skipcond 000".to_string());
        self.instructions.push("Jump helper_mod_store".to_string());
        self.instructions.push("Jump helper_mod_done".to_string());
        self.instructions
            .push("helper_mod_store, Store helper_mod_work".to_string());
        self.instructions.push("Jump helper_mod_loop".to_string());
        self.instructions
            .push("helper_mod_done, Load helper_mod_work".to_string());
        self.instructions.push("Store helper_mod_ret".to_string());
        self.instructions.push("JumpI helper_mod".to_string());

        if !self
            .data
            .iter()
            .any(|line| line.starts_with("helper_mod_lhs,"))
        {
            self.data.push("helper_mod_lhs, DEC 0".to_string());
            self.data.push("helper_mod_rhs, DEC 0".to_string());
            self.data.push("helper_mod_work, DEC 0".to_string());
            self.data.push("helper_mod_ret, DEC 0".to_string());
        }
    }

    fn finish(mut self) -> String {
        let mut lines = Vec::new();
        self.emit_helpers();

        lines.push("/ marie-c-compiler output".to_string());
        lines.extend(self.instructions);
        lines.push("/ data".to_string());
        lines.extend(self.data);
        lines.join("\n")
    }
}

fn normalized_parameter_count(function: &crate::ast::FunctionDeclaration) -> usize {
    if function.params.len() == 1
        && function.params[0].name.is_none()
        && matches!(
            function.params[0].ty,
            crate::ast::Type::Builtin(crate::ast::BuiltinType::Void)
        )
    {
        0
    } else {
        function.params.len()
    }
}

fn is_identifier_start_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn is_identifier_continue_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        Block, BlockItem, BuiltinType, Expression, ExternalDeclaration, FunctionDeclaration,
        Parameter, Statement, TranslationUnit, Type,
    };

    use super::{Codegen, MarieCodegen};

    /// Confirms the emitter returns a MARIE skeleton with runtime entry.
    #[test]
    fn emits_placeholder_marie_program() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![BlockItem::Statement(Statement::Return(Some(
                        Expression::IntegerLiteral {
                            value: 0,
                            location: None,
                        },
                    )))],
                },
            })],
        };

        let output = MarieCodegen
            .emit(&unit)
            .expect("codegen should produce placeholder output");

        assert!(output.contains("_start, Clear"));
        assert!(output.contains("JnS fn_main"));
        assert!(output.contains("fn_main, HEX 000"));
    }

    #[test]
    fn emits_comparison_and_logical_patterns() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "a".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: None,
                            }],
                        }),
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "b".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: None,
                            }],
                        }),
                        BlockItem::Statement(Statement::Expression(Some(Expression::Binary {
                            op: crate::ast::BinaryOp::Equal,
                            lhs: Box::new(Expression::Identifier {
                                name: "a".to_string(),
                                location: None,
                            }),
                            rhs: Box::new(Expression::Identifier {
                                name: "b".to_string(),
                                location: None,
                            }),
                            location: None,
                        }))),
                        BlockItem::Statement(Statement::Expression(Some(Expression::Binary {
                            op: crate::ast::BinaryOp::LogicalAnd,
                            lhs: Box::new(Expression::Identifier {
                                name: "a".to_string(),
                                location: None,
                            }),
                            rhs: Box::new(Expression::Identifier {
                                name: "b".to_string(),
                                location: None,
                            }),
                            location: None,
                        }))),
                        BlockItem::Statement(Statement::Expression(Some(Expression::Unary {
                            op: crate::ast::UnaryOp::LogicalNot,
                            expr: Box::new(Expression::Identifier {
                                name: "a".to_string(),
                                location: None,
                            }),
                            location: None,
                        }))),
                        BlockItem::Statement(Statement::Return(Some(Expression::IntegerLiteral {
                            value: 0,
                            location: None,
                        }))),
                    ],
                },
            })],
        };

        let output = MarieCodegen
            .emit(&unit)
            .expect("codegen should emit comparison/logical scaffolding");

        assert!(output.contains("Skipcond 400"));
        assert!(output.contains("Skipcond 0C00"));
        assert!(output.contains("cmp_eq_"));
        assert!(output.contains("logic_and_"));
        assert!(output.contains("unary_not_"));
    }

    #[test]
    fn emits_address_of_and_dereference() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "x".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: Some(Expression::IntegerLiteral {
                                    value: 5,
                                    location: None,
                                }),
                            }],
                        }),
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "ptr".to_string(),
                                ty: Type::Pointer(Box::new(Type::Builtin(BuiltinType::Int))),
                                initializer: Some(Expression::Unary {
                                    op: crate::ast::UnaryOp::AddressOf,
                                    expr: Box::new(Expression::Identifier {
                                        name: "x".to_string(),
                                        location: None,
                                    }),
                                    location: None,
                                }),
                            }],
                        }),
                        BlockItem::Statement(Statement::Return(Some(Expression::Unary {
                            op: crate::ast::UnaryOp::Dereference,
                            expr: Box::new(Expression::Identifier {
                                name: "ptr".to_string(),
                                location: None,
                            }),
                            location: None,
                        }))),
                    ],
                },
            })],
        };

        let output = MarieCodegen
            .emit(&unit)
            .expect("codegen should emit address-of and dereference");

        assert!(output.contains("LoadI helper_addr"));
        assert!(output.contains("addr_v_"));
    }

    #[test]
    fn emits_mul_mod_and_index_paths() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "arr".to_string(),
                                ty: Type::Array {
                                    element: Box::new(Type::Builtin(BuiltinType::Int)),
                                    size: Some(crate::ast::ConstExpr { value: 4 }),
                                },
                                initializer: None,
                            }],
                        }),
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "x".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: Some(Expression::Binary {
                                    op: crate::ast::BinaryOp::Multiply,
                                    lhs: Box::new(Expression::IntegerLiteral {
                                        value: 6,
                                        location: None,
                                    }),
                                    rhs: Box::new(Expression::IntegerLiteral {
                                        value: 7,
                                        location: None,
                                    }),
                                    location: None,
                                }),
                            }],
                        }),
                        BlockItem::Statement(Statement::Expression(Some(Expression::Assignment {
                            target: Box::new(Expression::Index {
                                base: Box::new(Expression::Identifier {
                                    name: "arr".to_string(),
                                    location: None,
                                }),
                                index: Box::new(Expression::IntegerLiteral {
                                    value: 1,
                                    location: None,
                                }),
                                location: None,
                            }),
                            value: Box::new(Expression::Binary {
                                op: crate::ast::BinaryOp::Modulo,
                                lhs: Box::new(Expression::Identifier {
                                    name: "x".to_string(),
                                    location: None,
                                }),
                                rhs: Box::new(Expression::IntegerLiteral {
                                    value: 5,
                                    location: None,
                                }),
                                location: None,
                            }),
                            location: None,
                        }))),
                        BlockItem::Statement(Statement::Return(Some(Expression::Index {
                            base: Box::new(Expression::Identifier {
                                name: "arr".to_string(),
                                location: None,
                            }),
                            index: Box::new(Expression::IntegerLiteral {
                                value: 1,
                                location: None,
                            }),
                            location: None,
                        }))),
                    ],
                },
            })],
        };

        let output = MarieCodegen
            .emit(&unit)
            .expect("codegen should emit helper/index scaffolding");

        assert!(output.contains("JnS helper_mul"));
        assert!(output.contains("JnS helper_mod"));
        assert!(output.contains("helper_mul, HEX 000"));
        assert!(output.contains("helper_mod, HEX 000"));
        assert!(output.contains("StoreI helper_addr"));
        assert!(output.contains("LoadI helper_addr"));
    }

    #[test]
    fn emits_int_and_addr_constants_and_array_storage() {
        let unit = TranslationUnit {
            top_level_items: vec![
                ExternalDeclaration::GlobalDeclaration(crate::ast::Declaration {
                    storage_class: None,
                    declarators: vec![crate::ast::Declarator {
                        name: "garr".to_string(),
                        ty: Type::Array {
                            element: Box::new(Type::Builtin(BuiltinType::Int)),
                            size: Some(crate::ast::ConstExpr { value: 3 }),
                        },
                        initializer: None,
                    }],
                }),
                ExternalDeclaration::Function(FunctionDeclaration {
                    name: "main".to_string(),
                    return_type: Type::Builtin(BuiltinType::Int),
                    params: vec![Parameter {
                        name: None,
                        ty: Type::Builtin(BuiltinType::Void),
                        location: None,
                    }],
                    body: Block {
                        items: vec![BlockItem::Statement(Statement::Return(Some(
                            Expression::IntegerLiteral {
                                value: 0,
                                location: None,
                            },
                        )))],
                    },
                }),
            ],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

        // globals are emitted with a `g_` prefix
        assert!(output.contains("g_garr, ADR g_garr_elem_0"));
        assert!(output.contains("g_garr_elem_0, DEC 0"));
        assert!(output.contains("g_garr_elem_1, DEC 0"));
        assert!(output.contains("g_garr_elem_2, DEC 0"));
        assert!(output.contains("/ data"));
    }

    #[test]
    fn emits_pointer_add_and_deref() {
        // Build a unit where a local pointer p is initialized to &arr and we return *(p + 1)
        let decl_arr = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "arr".to_string(),
                ty: Type::Array {
                    element: Box::new(Type::Builtin(BuiltinType::Int)),
                    size: Some(crate::ast::ConstExpr { value: 4 }),
                },
                initializer: None,
            }],
        };

        let decl_p = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "p".to_string(),
                ty: Type::Pointer(Box::new(Type::Builtin(BuiltinType::Int))),
                initializer: Some(Expression::Unary {
                    op: crate::ast::UnaryOp::AddressOf,
                    expr: Box::new(Expression::Identifier {
                        name: "arr".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
            }],
        };

        let ret_expr = Expression::Unary {
            op: crate::ast::UnaryOp::Dereference,
            expr: Box::new(Expression::Binary {
                op: crate::ast::BinaryOp::Add,
                lhs: Box::new(Expression::Identifier {
                    name: "p".to_string(),
                    location: None,
                }),
                rhs: Box::new(Expression::IntegerLiteral {
                    value: 1,
                    location: None,
                }),
                location: None,
            }),
            location: None,
        };

        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(decl_arr),
                        BlockItem::Declaration(decl_p),
                        BlockItem::Statement(Statement::Return(Some(ret_expr))),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

        // Expect constant 1 to be emitted (may be emitted as the shared const_one)
        assert!(output.contains("const_one") || output.contains("const_int_1"));
        assert!(output.contains("Store helper_addr"));
        assert!(output.contains("LoadI helper_addr"));
    }

    #[test]
    fn emits_pointer_subtraction_pattern() {
        // Build a unit performing q = p + 2; return q - p;
        let decl_arr = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "arr".to_string(),
                ty: Type::Array {
                    element: Box::new(Type::Builtin(BuiltinType::Int)),
                    size: Some(crate::ast::ConstExpr { value: 4 }),
                },
                initializer: None,
            }],
        };

        let decl_p = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "p".to_string(),
                ty: Type::Pointer(Box::new(Type::Builtin(BuiltinType::Int))),
                initializer: Some(Expression::Unary {
                    op: crate::ast::UnaryOp::AddressOf,
                    expr: Box::new(Expression::Identifier {
                        name: "arr".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
            }],
        };

        let decl_q = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "q".to_string(),
                ty: Type::Pointer(Box::new(Type::Builtin(BuiltinType::Int))),
                initializer: Some(Expression::Binary {
                    op: crate::ast::BinaryOp::Add,
                    lhs: Box::new(Expression::Identifier {
                        name: "p".to_string(),
                        location: None,
                    }),
                    rhs: Box::new(Expression::IntegerLiteral {
                        value: 2,
                        location: None,
                    }),
                    location: None,
                }),
            }],
        };

        let ret_expr = Expression::Binary {
            op: crate::ast::BinaryOp::Subtract,
            lhs: Box::new(Expression::Identifier {
                name: "q".to_string(),
                location: None,
            }),
            rhs: Box::new(Expression::Identifier {
                name: "p".to_string(),
                location: None,
            }),
            location: None,
        };

        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(decl_arr),
                        BlockItem::Declaration(decl_p),
                        BlockItem::Declaration(decl_q),
                        BlockItem::Statement(Statement::Return(Some(ret_expr))),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

        // The subtraction should produce a Subt instruction between temporaries
        assert!(output.contains("Subt ") || output.contains("Subt"));
    }

    #[test]
    fn emits_pointer_add_exact_sequence() {
        // Reuse the earlier build for p + 1 then deref and assert the Add tmp occurs before storing
        // into helper_addr and the LoadI helper_addr follows.
        let decl_arr = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "arr".to_string(),
                ty: Type::Array {
                    element: Box::new(Type::Builtin(BuiltinType::Int)),
                    size: Some(crate::ast::ConstExpr { value: 4 }),
                },
                initializer: None,
            }],
        };

        let decl_p = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "p".to_string(),
                ty: Type::Pointer(Box::new(Type::Builtin(BuiltinType::Int))),
                initializer: Some(Expression::Unary {
                    op: crate::ast::UnaryOp::AddressOf,
                    expr: Box::new(Expression::Identifier {
                        name: "arr".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
            }],
        };

        let ret_expr = Expression::Unary {
            op: crate::ast::UnaryOp::Dereference,
            expr: Box::new(Expression::Binary {
                op: crate::ast::BinaryOp::Add,
                lhs: Box::new(Expression::Identifier {
                    name: "p".to_string(),
                    location: None,
                }),
                rhs: Box::new(Expression::IntegerLiteral {
                    value: 1,
                    location: None,
                }),
                location: None,
            }),
            location: None,
        };

        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(decl_arr),
                        BlockItem::Declaration(decl_p),
                        BlockItem::Statement(Statement::Return(Some(ret_expr))),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

        let lines: Vec<&str> = output.lines().collect();
        let mut add_idx = None;
        let mut store_helper_idx = None;
        let mut loadi_idx = None;

        for (i, l) in lines.iter().enumerate() {
            if add_idx.is_none() && l.trim_start().starts_with("Add tmp_") {
                add_idx = Some(i);
            }
            if store_helper_idx.is_none() && l.contains("Store helper_addr") {
                store_helper_idx = Some(i);
            }
            if loadi_idx.is_none() && l.contains("LoadI helper_addr") {
                loadi_idx = Some(i);
            }
        }

        assert!(add_idx.is_some(), "expected an Add tmp_ instruction");
        assert!(store_helper_idx.is_some(), "expected Store helper_addr");
        assert!(loadi_idx.is_some(), "expected LoadI helper_addr");

        assert!(
            add_idx.unwrap() < store_helper_idx.unwrap(),
            "Add tmp_ should occur before Store helper_addr"
        );
        assert!(
            store_helper_idx.unwrap() < loadi_idx.unwrap(),
            "Store helper_addr should occur before LoadI helper_addr"
        );
    }

    #[test]
    fn emits_pointer_subtract_exact_sequence() {
        // Build unit q = p + 2; return q - p; ensure subtraction occurs as Subt tmp_
        let decl_arr = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "arr".to_string(),
                ty: Type::Array {
                    element: Box::new(Type::Builtin(BuiltinType::Int)),
                    size: Some(crate::ast::ConstExpr { value: 4 }),
                },
                initializer: None,
            }],
        };

        let decl_p = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "p".to_string(),
                ty: Type::Pointer(Box::new(Type::Builtin(BuiltinType::Int))),
                initializer: Some(Expression::Unary {
                    op: crate::ast::UnaryOp::AddressOf,
                    expr: Box::new(Expression::Identifier {
                        name: "arr".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
            }],
        };

        let decl_q = crate::ast::Declaration {
            storage_class: None,
            declarators: vec![crate::ast::Declarator {
                name: "q".to_string(),
                ty: Type::Pointer(Box::new(Type::Builtin(BuiltinType::Int))),
                initializer: Some(Expression::Binary {
                    op: crate::ast::BinaryOp::Add,
                    lhs: Box::new(Expression::Identifier {
                        name: "p".to_string(),
                        location: None,
                    }),
                    rhs: Box::new(Expression::IntegerLiteral {
                        value: 2,
                        location: None,
                    }),
                    location: None,
                }),
            }],
        };

        let ret_expr = Expression::Binary {
            op: crate::ast::BinaryOp::Subtract,
            lhs: Box::new(Expression::Identifier {
                name: "q".to_string(),
                location: None,
            }),
            rhs: Box::new(Expression::Identifier {
                name: "p".to_string(),
                location: None,
            }),
            location: None,
        };

        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(decl_arr),
                        BlockItem::Declaration(decl_p),
                        BlockItem::Declaration(decl_q),
                        BlockItem::Statement(Statement::Return(Some(ret_expr))),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
        let lines: Vec<&str> = output.lines().collect();

        // find first Subt line that references a tmp_ temporary
        let mut subt_idx = None;
        for (i, l) in lines.iter().enumerate() {
            if l.trim_start().starts_with("Subt tmp_") {
                subt_idx = Some(i);
                break;
            }
        }

        assert!(
            subt_idx.is_some(),
            "expected a Subt tmp_ instruction for pointer subtraction"
        );

        // Ensure there are at least two Store tmp_ lines before that Subt (lhs/rhs temps were stored)
        let count_store_tmp_before = lines[..subt_idx.unwrap()]
            .iter()
            .filter(|l| l.trim_start().starts_with("Store tmp_"))
            .count();

        assert!(
            count_store_tmp_before >= 1,
            "expected at least one Store tmp_ before Subt (lhs/rhs temps)"
        );
    }

    /// Verifies while loop generates proper MARIE labels.
    /// Expected output contains: Jump while_cond_X, while_cond_X, Clear, while_end_X, Clear
    #[test]
    fn emits_while_loop_labels() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![BlockItem::Statement(Statement::While {
                        condition: Expression::IntegerLiteral {
                            value: 1,
                            location: None,
                        },
                        body: Box::new(Statement::Return(Some(Expression::IntegerLiteral {
                            value: 42,
                            location: None,
                        }))),
                    })],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
        assert!(output.contains("while_cond_"));
        assert!(output.contains("while_end_"));
        assert!(output.contains("Jump while_cond_"));
    }

    /// Verifies while loop with condition generates Skipcond instruction.
    #[test]
    fn emits_while_with_condition_check() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "x".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: None,
                            }],
                        }),
                        BlockItem::Statement(Statement::While {
                            condition: Expression::Binary {
                                op: crate::ast::BinaryOp::Less,
                                lhs: Box::new(Expression::Identifier {
                                    name: "x".to_string(),
                                    location: None,
                                }),
                                rhs: Box::new(Expression::IntegerLiteral {
                                    value: 5,
                                    location: None,
                                }),
                                location: None,
                            },
                            body: Box::new(Statement::Expression(Some(Expression::Assignment {
                                target: Box::new(Expression::Identifier {
                                    name: "x".to_string(),
                                    location: None,
                                }),
                                value: Box::new(Expression::Binary {
                                    op: crate::ast::BinaryOp::Add,
                                    lhs: Box::new(Expression::Identifier {
                                        name: "x".to_string(),
                                        location: None,
                                    }),
                                    rhs: Box::new(Expression::IntegerLiteral {
                                        value: 1,
                                        location: None,
                                    }),
                                    location: None,
                                }),
                                location: None,
                            }))),
                        }),
                        BlockItem::Statement(Statement::Return(Some(Expression::Identifier {
                            name: "x".to_string(),
                            location: None,
                        }))),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
        assert!(output.contains("Skipcond"));
        assert!(output.contains("Jump while_end_"));
    }

    /// Verifies for loop generates proper MARIE labels.
    /// Expected output contains: Jump for_cond_X, for_cond_X, Clear, for_end_X, Clear
    #[test]
    fn emits_for_loop_labels() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "i".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: None,
                            }],
                        }),
                        BlockItem::Statement(Statement::For {
                            init: Some(Expression::Assignment {
                                target: Box::new(Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                }),
                                value: Box::new(Expression::IntegerLiteral {
                                    value: 0,
                                    location: None,
                                }),
                                location: None,
                            }),
                            condition: Some(Expression::Binary {
                                op: crate::ast::BinaryOp::Less,
                                lhs: Box::new(Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                }),
                                rhs: Box::new(Expression::IntegerLiteral {
                                    value: 10,
                                    location: None,
                                }),
                                location: None,
                            }),
                            update: Some(Expression::Assignment {
                                target: Box::new(Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                }),
                                value: Box::new(Expression::Binary {
                                    op: crate::ast::BinaryOp::Add,
                                    lhs: Box::new(Expression::Identifier {
                                        name: "i".to_string(),
                                        location: None,
                                    }),
                                    rhs: Box::new(Expression::IntegerLiteral {
                                        value: 1,
                                        location: None,
                                    }),
                                    location: None,
                                }),
                                location: None,
                            }),
                            body: Box::new(Statement::Return(Some(Expression::IntegerLiteral {
                                value: 0,
                                location: None,
                            }))),
                        }),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
        assert!(output.contains("for_cond_"));
        assert!(output.contains("for_end_"));
        assert!(output.contains("Jump for_cond_"));
    }

    /// Verifies for loop without condition generates infinite loop (jumps to cond).
    #[test]
    fn emits_for_without_condition() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![BlockItem::Statement(Statement::For {
                        init: None,
                        condition: None,
                        update: None,
                        body: Box::new(Statement::Return(Some(Expression::IntegerLiteral {
                            value: 1,
                            location: None,
                        }))),
                    })],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
        assert!(output.contains("for_cond_"));
        assert!(output.contains("Jump for_cond_"));
    }

    /// Verifies nested for loops generate unique labels for each.
    #[test]
    fn emits_nested_for_loops() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "i".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: None,
                            }],
                        }),
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "j".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: None,
                            }],
                        }),
                        BlockItem::Statement(Statement::For {
                            init: Some(Expression::Assignment {
                                target: Box::new(Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                }),
                                value: Box::new(Expression::IntegerLiteral {
                                    value: 0,
                                    location: None,
                                }),
                                location: None,
                            }),
                            condition: Some(Expression::Binary {
                                op: crate::ast::BinaryOp::Less,
                                lhs: Box::new(Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                }),
                                rhs: Box::new(Expression::IntegerLiteral {
                                    value: 3,
                                    location: None,
                                }),
                                location: None,
                            }),
                            update: Some(Expression::Assignment {
                                target: Box::new(Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                }),
                                value: Box::new(Expression::Binary {
                                    op: crate::ast::BinaryOp::Add,
                                    lhs: Box::new(Expression::Identifier {
                                        name: "i".to_string(),
                                        location: None,
                                    }),
                                    rhs: Box::new(Expression::IntegerLiteral {
                                        value: 1,
                                        location: None,
                                    }),
                                    location: None,
                                }),
                                location: None,
                            }),
                            body: Box::new(Statement::For {
                                init: Some(Expression::Assignment {
                                    target: Box::new(Expression::Identifier {
                                        name: "j".to_string(),
                                        location: None,
                                    }),
                                    value: Box::new(Expression::IntegerLiteral {
                                        value: 0,
                                        location: None,
                                    }),
                                    location: None,
                                }),
                                condition: Some(Expression::Binary {
                                    op: crate::ast::BinaryOp::Less,
                                    lhs: Box::new(Expression::Identifier {
                                        name: "j".to_string(),
                                        location: None,
                                    }),
                                    rhs: Box::new(Expression::IntegerLiteral {
                                        value: 2,
                                        location: None,
                                    }),
                                    location: None,
                                }),
                                update: Some(Expression::Assignment {
                                    target: Box::new(Expression::Identifier {
                                        name: "j".to_string(),
                                        location: None,
                                    }),
                                    value: Box::new(Expression::Binary {
                                        op: crate::ast::BinaryOp::Add,
                                        lhs: Box::new(Expression::Identifier {
                                            name: "j".to_string(),
                                            location: None,
                                        }),
                                        rhs: Box::new(Expression::IntegerLiteral {
                                            value: 1,
                                            location: None,
                                        }),
                                        location: None,
                                    }),
                                    location: None,
                                }),
                                body: Box::new(Statement::Return(Some(
                                    Expression::IntegerLiteral {
                                        value: 0,
                                        location: None,
                                    },
                                ))),
                            }),
                        }),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
        assert!(output.contains("for_cond_"));
        assert!(output.contains("for_end_"));
    }

    #[test]
    fn emits_inline_asm_with_variable_substitution() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "x".to_string(),
                                ty: Type::Builtin(BuiltinType::Int),
                                initializer: None,
                            }],
                        }),
                        BlockItem::Statement(Statement::InlineAsm(vec![
                            "Load %x".to_string(),
                            "Output".to_string(),
                        ])),
                        BlockItem::Statement(Statement::Return(Some(Expression::IntegerLiteral {
                            value: 0,
                            location: None,
                        }))),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

        assert!(output.contains("Load v_main_"));
        assert!(output.contains("_x"));
        assert!(output.contains("Output"));
    }

    #[test]
    fn emits_inline_asm_with_newline_content() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Statement(Statement::InlineAsm(vec![
                            "Clear\nOutput".to_string(),
                        ])),
                        BlockItem::Statement(Statement::Return(Some(Expression::IntegerLiteral {
                            value: 0,
                            location: None,
                        }))),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

        assert!(output.contains("Clear"));
        assert!(output.contains("Output"));
    }

    #[test]
    fn emits_global_array_initializer_values() {
        let unit = TranslationUnit {
            top_level_items: vec![
                ExternalDeclaration::GlobalDeclaration(crate::ast::Declaration {
                    storage_class: None,
                    declarators: vec![crate::ast::Declarator {
                        name: "msg".to_string(),
                        ty: Type::Array {
                            element: Box::new(Type::Builtin(BuiltinType::Char)),
                            size: Some(crate::ast::ConstExpr { value: 6 }),
                        },
                        initializer: Some(Expression::ArrayInitializer {
                            elements: vec![
                                Expression::IntegerLiteral {
                                    value: 'h' as i64,
                                    location: None,
                                },
                                Expression::IntegerLiteral {
                                    value: 'e' as i64,
                                    location: None,
                                },
                                Expression::IntegerLiteral {
                                    value: 'l' as i64,
                                    location: None,
                                },
                                Expression::IntegerLiteral {
                                    value: 'l' as i64,
                                    location: None,
                                },
                                Expression::IntegerLiteral {
                                    value: 'o' as i64,
                                    location: None,
                                },
                                Expression::IntegerLiteral {
                                    value: 0,
                                    location: None,
                                },
                            ],
                            location: None,
                        }),
                    }],
                }),
                ExternalDeclaration::Function(FunctionDeclaration {
                    name: "main".to_string(),
                    return_type: Type::Builtin(BuiltinType::Int),
                    params: vec![Parameter {
                        name: None,
                        ty: Type::Builtin(BuiltinType::Void),
                        location: None,
                    }],
                    body: Block {
                        items: vec![BlockItem::Statement(Statement::Return(Some(
                            Expression::IntegerLiteral {
                                value: 0,
                                location: None,
                            },
                        )))],
                    },
                }),
            ],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

        assert!(output.contains("g_msg_elem_0, DEC 104"));
        assert!(output.contains("g_msg_elem_1, DEC 101"));
        assert!(output.contains("g_msg_elem_4, DEC 111"));
        assert!(output.contains("g_msg_elem_5, DEC 0"));
    }

    #[test]
    fn emits_local_array_initializer_values_in_data() {
        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(FunctionDeclaration {
                name: "main".to_string(),
                return_type: Type::Builtin(BuiltinType::Int),
                params: vec![Parameter {
                    name: None,
                    ty: Type::Builtin(BuiltinType::Void),
                    location: None,
                }],
                body: Block {
                    items: vec![
                        BlockItem::Declaration(crate::ast::Declaration {
                            storage_class: None,
                            declarators: vec![crate::ast::Declarator {
                                name: "msg".to_string(),
                                ty: Type::Array {
                                    element: Box::new(Type::Builtin(BuiltinType::Char)),
                                    size: Some(crate::ast::ConstExpr { value: 6 }),
                                },
                                initializer: Some(Expression::ArrayInitializer {
                                    elements: vec![
                                        Expression::IntegerLiteral {
                                            value: 'h' as i64,
                                            location: None,
                                        },
                                        Expression::IntegerLiteral {
                                            value: 'e' as i64,
                                            location: None,
                                        },
                                        Expression::IntegerLiteral {
                                            value: 'l' as i64,
                                            location: None,
                                        },
                                        Expression::IntegerLiteral {
                                            value: 'l' as i64,
                                            location: None,
                                        },
                                        Expression::IntegerLiteral {
                                            value: 'o' as i64,
                                            location: None,
                                        },
                                        Expression::IntegerLiteral {
                                            value: 0,
                                            location: None,
                                        },
                                    ],
                                    location: None,
                                }),
                            }],
                        }),
                        BlockItem::Statement(Statement::Return(Some(Expression::IntegerLiteral {
                            value: 0,
                            location: None,
                        }))),
                    ],
                },
            })],
        };

        let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

        assert!(output.contains("_msg_elem_0, DEC 104"));
        assert!(output.contains("_msg_elem_1, DEC 101"));
        assert!(output.contains("_msg_elem_4, DEC 111"));
        assert!(output.contains("_msg_elem_5, DEC 0"));
    }
}

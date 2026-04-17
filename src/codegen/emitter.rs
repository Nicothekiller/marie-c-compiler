use std::collections::HashMap;

use crate::ast::{Block, Expression, ExternalDeclaration, FunctionDeclaration, Statement, TranslationUnit, Type};
use crate::error::CompilerError;

#[derive(Debug, Default)]
pub(crate) struct MarieEmitter {
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
    pub(crate) fn emit_translation_unit(&mut self, ast: &TranslationUnit) -> Result<(), CompilerError> {
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
        function: &FunctionDeclaration,
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

    pub(crate) fn finish(mut self) -> String {
        let mut lines = Vec::new();
        self.emit_helpers();

        lines.push("/ marie-c-compiler output".to_string());
        lines.extend(self.instructions);
        lines.push("/ data".to_string());
        lines.extend(self.data);
        lines.join("\n")
    }
}

fn normalized_parameter_count(function: &FunctionDeclaration) -> usize {
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

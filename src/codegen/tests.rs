use crate::ast::{
    Block, BlockItem, BuiltinType, Expression, ExternalDeclaration, FunctionDeclaration, Parameter,
    Statement, TranslationUnit, Type,
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
fn emits_division_helper() {
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
                    Expression::Binary {
                        op: crate::ast::BinaryOp::Divide,
                        lhs: Box::new(Expression::IntegerLiteral {
                            value: 10,
                            location: None,
                        }),
                        rhs: Box::new(Expression::IntegerLiteral {
                            value: 3,
                            location: None,
                        }),
                        location: None,
                    },
                )))],
            },
        })],
    };

    let output = MarieCodegen
        .emit(&unit)
        .expect("codegen should emit division helper");

    assert!(output.contains("JnS helper_div"));
    assert!(output.contains("helper_div, HEX 000"));
    assert!(output.contains("helper_div_dividend"));
    assert!(output.contains("helper_div_rhs"));
    assert!(output.contains("helper_div_quotient"));
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

    assert!(output.contains("g_garr, ADR g_garr_elem_0"));
    assert!(output.contains("g_garr_elem_0, DEC 0"));
    assert!(output.contains("g_garr_elem_1, DEC 0"));
    assert!(output.contains("g_garr_elem_2, DEC 0"));
    assert!(output.contains("/ data"));
}

#[test]
fn emits_pointer_add_and_deref() {
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

    assert!(output.contains("const_one") || output.contains("const_int_1"));
    assert!(output.contains("Store helper_addr"));
    assert!(output.contains("LoadI helper_addr"));
}

#[test]
fn emits_pointer_subtraction_pattern() {
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

    assert!(output.contains("Subt ") || output.contains("Subt"));
}

#[test]
fn emits_pointer_add_exact_sequence() {
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
        add_idx.expect("add index") < store_helper_idx.expect("store index"),
        "Add tmp_ should occur before Store helper_addr"
    );
    assert!(
        store_helper_idx.expect("store index") < loadi_idx.expect("loadi index"),
        "Store helper_addr should occur before LoadI helper_addr"
    );
}

#[test]
fn emits_pointer_subtract_exact_sequence() {
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

    let count_store_tmp_before = lines[..subt_idx.expect("subt index")]
        .iter()
        .filter(|l| l.trim_start().starts_with("Store tmp_"))
        .count();

    assert!(
        count_store_tmp_before >= 1,
        "expected at least one Store tmp_ before Subt (lhs/rhs temps)"
    );
}

/// Verifies while loop generates proper MARIE labels.
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
                            body: Box::new(Statement::Return(Some(Expression::IntegerLiteral {
                                value: 0,
                                location: None,
                            }))),
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
                    BlockItem::Statement(Statement::InlineAsm(vec!["Clear\nOutput".to_string()])),
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

#[test]
fn emits_struct_member_load_and_store() {
    let source = "struct Point { int x; int y; } p; int main(void) { p.x = 9; return p.y + p.x; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");

    assert!(output.contains("g_p, ADR"));
    assert!(output.contains("StoreI helper_addr"));
    assert!(output.contains("LoadI helper_addr"));
}

#[test]
fn emits_struct_with_typedef_alias() {
    let source =
        "typedef struct Point { int x; int y; } Point; Point p; int main(void) { return p.x; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("g_p, ADR"));
    assert!(output.contains("LoadI helper_addr"));
}

#[test]
fn emits_struct_with_pointer_member() {
    let source = "struct Node { int value; struct Node *next; } n; int main(void) { n.next = 0; return n.value; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("g_n, ADR"));
    assert!(output.contains("LoadI helper_addr"));
}

#[test]
fn emits_typedef_pointer_to_struct() {
    let source =
        "typedef struct Point { int x; int y; } Point; Point *p; int main(void) { return p->x; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("LoadI helper_addr"));
}

#[test]
fn emits_typedef_array_of_struct() {
    let source = "typedef struct Point { int x; } Point; Point arr[2]; int main(void) { arr[0].x = 1; return arr[1].x; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("StoreI helper_addr"));
    assert!(output.contains("LoadI helper_addr"));
}

#[test]
fn emits_enum_constant_as_integer() {
    let source = "enum Color { RED, GREEN = 3, BLUE }; int main(void) { return BLUE; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("const_int_4") || output.contains("Load const_int_4"));
}

#[test]
fn emits_typedef_enum_alias_program() {
    let source =
        "typedef enum Color { RED, GREEN } Color; Color c; int main(void) { c = GREEN; return c; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("g_c, DEC 0"));
}

#[test]
fn emits_enum_with_variable_and_constant() {
    let source = "enum Color { RED, GREEN = 3 }; int main(void) { enum Color c; c = GREEN; return c + RED; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("const_int_3"));
}

#[test]
fn emits_division_with_variables() {
    let source = "int main(void) { int x = 10 / 3; return x / 2; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("helper_div"));
}

/// Verifies prefix increment generates correct MARIE code.
/// Output should load, add 1, store back, then load the new value.
#[test]
fn emits_prefix_increment() {
    let source = "int main(void) { int x; ++x; return x; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    // Should contain: Load x, Add const_one, Store x, Load x
    assert!(output.contains("Load v_main_0_x"));
    assert!(output.contains("Add const_one"));
    assert!(output.contains("Store v_main_0_x"));
    // Should contain another Load x to return the new value
    assert!(output.contains("Load v_main_0_x"));
}

/// Verifies prefix decrement generates correct MARIE code.
#[test]
fn emits_prefix_decrement() {
    let source = "int main(void) { int x; --x; return x; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("Load v_main_0_x"));
    assert!(output.contains("Subt const_one"));
    assert!(output.contains("Store v_main_0_x"));
    assert!(output.contains("Load v_main_0_x"));
}

/// Verifies postfix increment generates correct MARIE code.
/// Output should store original value, increment, then load original.
#[test]
fn emits_postfix_increment() {
    let source = "int main(void) { int x; int y = x++; return y; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    // Should store original value to temp, then load it for y
    assert!(output.contains("tmp_inc_"));
    assert!(output.contains("Load v_main_0_x"));
    assert!(output.contains("Store tmp_inc_"));
    assert!(output.contains("Add const_one"));
    assert!(output.contains("Store v_main_0_x"));
    // Final load should be from temp, not from x
    assert!(output.contains("Load tmp_inc_"));
    assert!(output.contains("Store v_main_1_y"));
}

/// Verifies postfix decrement generates correct MARIE code.
#[test]
fn emits_postfix_decrement() {
    let source = "int main(void) { int x; int y = x--; return y; }";
    let unit = crate::parser::CParser::new()
        .parse_translation_unit(source)
        .expect("source should parse");

    let output = MarieCodegen.emit(&unit).expect("codegen should succeed");
    assert!(output.contains("tmp_inc_"));
    assert!(output.contains("Load v_main_0_x"));
    assert!(output.contains("Store tmp_inc_"));
    assert!(output.contains("Subt const_one"));
    assert!(output.contains("Store v_main_0_x"));
    assert!(output.contains("Load tmp_inc_"));
}

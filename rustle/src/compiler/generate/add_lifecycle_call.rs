use swc_common::Span;
use swc_ecma_ast::{
    ArrayLit, ArrowExpr, BlockStmtOrExpr, CallExpr, Callee, Expr, ExprOrSpread, ExprStmt, Ident,
    Lit, MemberExpr, MemberProp, ParenExpr, Pat, PatOrExpr, Script, SeqExpr, Stmt, Str,
};

use crate::compiler::{
    expr_visitor::Visit,
    swc_ast_visitor::{ArrowInterpreter, VisitArrowExpr},
};

pub fn add_lifecycle_calls(mut script: Script) -> Script {
    let mut interpreter = ArrowInterpreter;

    let mut stmts = script.body.clone();

    for stmt in &mut stmts {
        if let Some(mut arrow) = ArrowInterpreter::visit_stmt(&mut interpreter, stmt) {
            update_body_ast(&mut arrow);
        }
    }

    script.body = stmts;
    script
}

fn update_body_ast(arrow_expr: &mut ArrowExpr) {
    match arrow_expr.body.clone() {
        BlockStmtOrExpr::BlockStmt(bs) => {
            let mut names = Vec::new();
            for stmt in bs.stmts {
                match stmt {
                    Stmt::Expr(expr) => match expr.expr.unwrap_parens() {
                        Expr::Assign(a) => match &a.left {
                            PatOrExpr::Expr(e) => match e.unwrap_parens() {
                                Expr::Ident(i) => names.push(i.sym.to_string()),
                                _ => println!(
                                    "Unsupported expression for adding lifecycle: {:#?}",
                                    expr
                                ),
                            },
                            PatOrExpr::Pat(p) => match &**p {
                                Pat::Ident(bi) => names.push(bi.id.sym.to_string()),
                                p => println!("Unsupported pattern for adding lifecycle: {:#?}", p),
                            },
                        },
                        e => println!("Unsupported expression for adding lifecycle: {:#?}", e),
                    },
                    s => println!("Unsupported statement for adding lifecycle: {:#?}", s),
                };
            }

            for name in names {
                arrow_expr
                    .body
                    .as_mut_block_stmt()
                    .unwrap()
                    .stmts
                    .push(Stmt::Expr(ExprStmt {
                        span: Span::default(),
                        expr: Box::new(Expr::Call(lifecycle_update_ast(&name))),
                    }));
            }
        }
        BlockStmtOrExpr::Expr(expr) => match expr.unwrap_parens() {
            Expr::Update(ue) => {
                let variable_names = ue.arg.extract_names();
                let mut lifecycle_updates = Vec::new();

                lifecycle_updates.push(Box::new(Expr::Update(ue.clone())));

                for var_name in variable_names {
                    lifecycle_updates.push(Box::new(Expr::Call(lifecycle_update_ast(&var_name))));
                }

                let new_body = ParenExpr {
                    span: Span::default(),
                    expr: Box::new(Expr::Seq(SeqExpr {
                        span: Span::default(),
                        exprs: lifecycle_updates,
                    })),
                };

                arrow_expr.body = BlockStmtOrExpr::Expr(Box::new(Expr::Paren(new_body)));
            }
            _ => (),
        },
    }
}

fn lifecycle_update_ast(variable_name: &str) -> CallExpr {
    CallExpr {
        span: Span::default(),
        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
            span: Span::default(),
            obj: Box::new(Expr::Ident(Ident {
                span: Span::default(),
                sym: "lifecycle".into(),
                optional: false,
            })),
            prop: MemberProp::Ident(Ident {
                span: Span::default(),
                sym: "update".into(),
                optional: false,
            }),
        }))),
        args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Array(ArrayLit {
                span: Span::default(),
                elems: vec![Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                        span: Span::default(),
                        value: variable_name.into(),
                        raw: Some(format!("\"{}\"", variable_name).into()),
                    }))),
                })],
            })),
        }],
        type_args: None,
    }
}

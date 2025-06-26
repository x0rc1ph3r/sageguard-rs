use colored::*;
use syn::{Expr, ExprField, ExprIndex, ExprPath, ItemFn, Member, Stmt};

pub fn check_remaining_accounts_usage(func: &ItemFn, file: &str) {
    for stmt in &func.block.stmts {
        detect_stmt(stmt, file, &func.sig.ident.to_string());
    }
}

fn detect_stmt(stmt: &Stmt, file: &str, fn_name: &str) {
    match stmt {
        Stmt::Expr(expr, _) => detect_expr(expr, file, fn_name),

        Stmt::Local(local) => {
            if let Some(init) = &local.init {
                detect_expr(&init.expr, file, fn_name);
            }
        }

        Stmt::Item(_) | Stmt::Macro(_) => {}
    }
}

fn warn(file: &str, fn_name: &str, line: usize) {
    println!(
        "{} Usage of `ctx.remaining_accounts` in `{}`. \
         Ensure you check length/order before indexing. ({}:{})\n",
        "[WARNING]".yellow().bold(),
        fn_name,
        file,
        line
    );
}

fn detect_expr(expr: &Expr, file: &str, fn_name: &str) {
    match expr {
        Expr::Field(ExprField { base, member, .. }) => {
            if let Member::Named(ident) = member {
                if ident == "remaining_accounts" {
                    if let Expr::Path(ExprPath { path, .. }) = &**base {
                        if path.is_ident("ctx") {
                            warn(file, fn_name, ident.span().start().line);
                        }
                    }
                }
            }
        }

        // indexing: ctx.remaining_accounts[...]
        Expr::Index(ExprIndex {
            expr: boxed, index, ..
        }) => {
            detect_expr(boxed, file, fn_name);
            detect_expr(index, file, fn_name);
        }

        // method call: ctx.remaining_accounts.len()
        Expr::MethodCall(mc) => {
            detect_expr(&mc.receiver, file, fn_name);
            for arg in &mc.args {
                detect_expr(arg, file, fn_name)
            }
        }

        Expr::Try(syn::ExprTry { expr: inner, .. }) => {
            // unwrap the inner expression (the call) and run again
            detect_expr(&*inner, file, fn_name);
        }

        // if you also want to catch `try { ... }?`, add:
        Expr::TryBlock(syn::ExprTryBlock { block, .. }) => {
            for stmt in &block.stmts {
                detect_stmt(stmt, file, fn_name);
            }
        }

        // recurse into sub-blocks
        Expr::Block(b) => {
            for stmt in &b.block.stmts {
                detect_stmt(stmt, file, fn_name);
            }
        }

        Expr::If(i) => {
            detect_expr(&i.cond, file, fn_name);
            for stmt in &i.then_branch.stmts {
                detect_stmt(stmt, file, fn_name);
            }
            if let Some((_, else_expr)) = &i.else_branch {
                detect_expr(else_expr, file, fn_name);
            }
        }

        Expr::Match(m) => {
            detect_expr(&m.expr, file, fn_name);
            for arm in &m.arms {
                detect_expr(&arm.body, file, fn_name);
            }
        }

        Expr::While(w) => {
            detect_expr(&w.cond, file, fn_name);
            for stmt in &w.body.stmts {
                detect_stmt(stmt, file, fn_name);
            }
        }

        Expr::ForLoop(f) => {
            detect_expr(&f.expr, file, fn_name);
            for stmt in &f.body.stmts {
                detect_stmt(stmt, file, fn_name);
            }
        }

        Expr::Loop(l) => {
            for stmt in &l.body.stmts {
                detect_stmt(stmt, file, fn_name);
            }
        }

        _ => {}
    }
}

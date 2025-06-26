use colored::*;
use syn::{Expr, ExprCall, ExprField, ExprMethodCall, ExprPath, ItemFn, Member, Stmt};

pub fn check_realloc_usage(func: &ItemFn, file: &str) {
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

fn detect_expr(expr: &Expr, file: &str, fn_name: &str) {
    match expr {
        Expr::MethodCall(ExprMethodCall {
            receiver,
            method,
            args,
            ..
        }) if method == "realloc" => {
            let account_name = if let Expr::Field(ExprField { member, .. }) = &**receiver {
                match member {
                    Member::Named(ident) => ident.to_string(),
                    _ => "<unknown>".into(),
                }
            } else {
                "<expr>".into()
            };

            let line = method.span().start().line;
            println!(
                "{} Call to `.realloc()` on `{}` in `{}`. \
                 Make sure to handle rent-exemption and re-serialization ({}:{})\n",
                "[WARNING]".yellow().bold(),
                account_name,
                fn_name,
                file,
                line
            );

            for arg in args {
                detect_expr(arg, file, fn_name);
            }
        }

        Expr::Call(ExprCall { func, args, .. }) => {
            if let Expr::Path(ExprPath { path, .. }) = &**func {
                if path.segments.last().unwrap().ident == "realloc" {
                    let line = path.segments.last().unwrap().ident.span().start().line;
                    println!(
                        "{} Free‐function `realloc()` called in `{}`. \
                         Make sure to handle rent-exemption and re-serialization ({}:{})\n",
                        "[WARNING]".yellow().bold(),
                        fn_name,
                        file,
                        line
                    );
                }
            }

            for arg in args {
                detect_expr(arg, file, fn_name);
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

        Expr::Block(b) => {
            for s in &b.block.stmts {
                detect_stmt(s, file, fn_name)
            }
        }
        Expr::If(i) => {
            detect_expr(&i.cond, file, fn_name);
            for s in &i.then_branch.stmts {
                detect_stmt(s, file, fn_name)
            }
            if let Some((_, else_e)) = &i.else_branch {
                detect_expr(else_e, file, fn_name)
            }
        }
        Expr::Match(m) => {
            detect_expr(&m.expr, file, fn_name);
            for arm in &m.arms {
                detect_expr(&arm.body, file, fn_name)
            }
        }
        Expr::While(w) => {
            detect_expr(&w.cond, file, fn_name);
            for s in &w.body.stmts {
                detect_stmt(s, file, fn_name)
            }
        }
        Expr::ForLoop(f) => {
            detect_expr(&f.expr, file, fn_name);
            for s in &f.body.stmts {
                detect_stmt(s, file, fn_name)
            }
        }
        Expr::Loop(l) => {
            for s in &l.body.stmts {
                detect_stmt(s, file, fn_name)
            }
        }

        Expr::MethodCall(mc) => {
            detect_expr(&mc.receiver, file, fn_name);
            for arg in &mc.args {
                detect_expr(arg, file, fn_name)
            }
        }

        _ => {}
    }
}

use colored::*;
use syn::{
    Expr, ExprBlock, ExprCall, ExprForLoop, ExprIf, ExprLoop, ExprMatch, ExprMethodCall, ExprPath,
    ExprReference, ExprWhile, ItemFn, Stmt, spanned::Spanned,
};

fn detect_invoke_signed_bump(call: &ExprCall, file: &str, fn_name: &str) {
    // match only `invoke_signed`
    if let Expr::Path(ExprPath { path, .. }) = &*call.func {
        if path.segments.last().unwrap().ident == "invoke_signed" {
            // third argument is the &[&[...], ...]
            if let Some(seeds_arg) = call.args.iter().nth(2) {
                // attempt to unwrap a leading `&`
                let outer_arr = if let Expr::Reference(ExprReference { expr, .. }) = seeds_arg {
                    if let Expr::Array(arr) = &**expr { Some(arr) } else { None }
                } else if let Expr::Array(arr) = seeds_arg {
                    Some(arr)
                } else {
                    None
                };

                if let Some(outer) = outer_arr {
                    for slice in &outer.elems {
                        // each slice should itself be an array `&[ seed1, seed2, ..., bump ]`
                        let inner_arr = if let Expr::Reference(ExprReference { expr, .. }) = slice {
                            if let Expr::Array(arr) = &**expr { Some(arr) } else { None }
                        } else if let Expr::Array(arr) = slice {
                            Some(arr)
                        } else {
                            None
                        };

                        if let Some(inner) = inner_arr {
                            // if there’s fewer than 2 seeds, bump is missing
                            if inner.elems.len() < 2 {
                                let line = call.func.span().start().line;
                                println!(
                                    "{} `invoke_signed` in `{}` is missing a bump in its signer seeds. \
Make sure each seed slice ends with the bump value. ({}:{})\n",
                                    "[ERROR]".red().bold(),
                                    fn_name,
                                    file,
                                    line
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}


pub fn detect_cpi_in_fn(func: &ItemFn, filename: &str) {
    let fn_name = func.sig.ident.to_string();
    for stmt in &func.block.stmts {
        detect_cpi_stmt(stmt, filename, &fn_name);
    }
}

fn detect_cpi_stmt(stmt: &Stmt, filename: &str, fn_name: &str) {
    match stmt {
        Stmt::Expr(expr, _) => detect_cpi_expr(expr, filename, fn_name),

        Stmt::Local(local) => {
            if let Some(init) = &local.init {
                detect_cpi_expr(&init.expr, filename, fn_name);
            }
        }

        Stmt::Item(_) => {}

        Stmt::Macro(_) => {}
    }
}

fn detect_cpi_expr(expr: &Expr, filename: &str, fn_name: &str) {
    match expr {
        // function call: anchor_spl::token::transfer(...)
        Expr::Call(call) => {
            if let Expr::Path(ExprPath { path, .. }) = call.func.as_ref() {
                if let Some(seg) = path.segments.last() {
                    let name = seg.ident.to_string();
                    if is_known_cpi(&name) {
                        let line = seg.ident.span().start().line;
                        println!(
                            "{} CPI `{}` in `{}`. Consider `.reload()?` on affected accounts. ({}:{})\n",
                            "[WARNING]".yellow().bold(),
                            name,
                            fn_name,
                            filename,
                            line,
                        );
                    }
                }
            }

            detect_invoke_signed_bump(call, filename, fn_name);

            // dive into arguments
            for arg in &call.args {
                detect_cpi_expr(arg, filename, fn_name);
            }
        }

        Expr::Try(syn::ExprTry { expr: inner, .. }) => {
            // unwrap the inner expression (the call) and run again
            detect_cpi_expr(&*inner, filename, fn_name);
        }

        // if you also want to catch `try { ... }?`, add:
        Expr::TryBlock(syn::ExprTryBlock { block, .. }) => {
            for stmt in &block.stmts {
                detect_cpi_stmt(stmt, filename, fn_name);
            }
        }

        // method call: some_struct.invoke(...)
        Expr::MethodCall(ExprMethodCall {
            method,
            receiver,
            args,
            ..
        }) => {
            let name = method.to_string();
            if is_known_cpi(&name) {
                let line = method.span().start().line;
                println!(
                    "{} CPI `{}` in `{}`. Consider `.reload()?` on affected accounts. ({}:{})\n",
                    "[WARNING]".yellow().bold(),
                    name,
                    fn_name,
                    filename,
                    line,
                );
            }
            // recurse
            detect_cpi_expr(receiver, filename, fn_name);
            for arg in args {
                detect_cpi_expr(arg, filename, fn_name);
            }
        }

        // blocks: { ... }
        Expr::Block(ExprBlock { block, .. }) => {
            for stmt in &block.stmts {
                detect_cpi_stmt(stmt, filename, fn_name);
            }
        }

        // if cond { ... } else { ... }
        Expr::If(ExprIf {
            cond,
            then_branch,
            else_branch,
            ..
        }) => {
            detect_cpi_expr(cond, filename, fn_name);
            for stmt in &then_branch.stmts {
                detect_cpi_stmt(stmt, filename, fn_name);
            }
            if let Some((_, else_expr)) = else_branch {
                detect_cpi_expr(else_expr, filename, fn_name);
            }
        }

        // match foo { ... }
        Expr::Match(ExprMatch {
            expr: match_expr,
            arms,
            ..
        }) => {
            detect_cpi_expr(match_expr, filename, fn_name);
            for arm in arms {
                detect_cpi_expr(&arm.body, filename, fn_name);
            }
        }

        // while cond { ... }
        Expr::While(ExprWhile { cond, body, .. }) => {
            detect_cpi_expr(cond, filename, fn_name);
            for stmt in &body.stmts {
                detect_cpi_stmt(stmt, filename, fn_name);
            }
        }

        // for pat in expr { ... }
        Expr::ForLoop(ExprForLoop { expr, body, .. }) => {
            detect_cpi_expr(expr, filename, fn_name);
            for stmt in &body.stmts {
                detect_cpi_stmt(stmt, filename, fn_name);
            }
        }

        // loop { ... }
        Expr::Loop(ExprLoop { body, .. }) => {
            for stmt in &body.stmts {
                detect_cpi_stmt(stmt, filename, fn_name);
            }
        }

        // other expressions - ignore
        _ => {}
    }
}

/// List of functions that perform CPIs in Anchor/SPL contexts
fn is_known_cpi(name: &str) -> bool {
    matches!(
        name,
        // Anchor direct CPIs
        "invoke" | "invoke_signed"
        // SPL token CPIs
        | "transfer"
        | "mint_to"
        | "burn"
    )
}

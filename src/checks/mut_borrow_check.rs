use colored::*;
use std::collections::{HashMap, HashSet};
use syn::{
    Attribute, Error, Expr, ExprAssign, ExprBlock, ExprField, ExprForLoop, ExprIf, ExprLoop,
    ExprMatch, ExprPath, ExprReference, ExprWhile, FnArg, GenericArgument, ItemFn,
    LocalInit, Member, PathArguments, Stmt, Type, spanned::Spanned, ExprBinary, BinOp
};

pub type AccountsMutMap = HashMap<String, HashSet<String>>;
pub type AccountsInitMap = HashMap<String, HashSet<String>>;

pub fn attr_contains_mut(attr: &Attribute) -> bool {
    if !attr.path().is_ident("account") {
        return false;
    }
    let mut found = false;
    let _ = attr.parse_nested_meta(|meta| {
        let name = meta
            .path
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_default();
        if name == "mut" {
            found = true;
            Err(Error::new_spanned(meta.path.clone(), "found mut"))
        } else {
            Ok(())
        }
    });
    found
}

pub fn check_mut_borrow(func: &ItemFn, file: &str, accounts_mut: &AccountsMutMap, accounts_init: &AccountsInitMap) {
    // Determine which Accounts struct the Context<> refers to
    let ctx_struct = func
        .sig
        .inputs
        .iter()
        .find_map(|arg| {
            if let FnArg::Typed(pat_ty) = arg {
                if let Type::Path(tp) = &*pat_ty.ty {
                    if tp.path.segments.last()?.ident == "Context" {
                        if let PathArguments::AngleBracketed(ab) =
                            &tp.path.segments.last()?.arguments
                        {
                            if let Some(GenericArgument::Type(Type::Path(inner))) = ab.args.first()
                            {
                                return Some(inner.path.segments.last()?.ident.to_string());
                            }
                        }
                    }
                }
            }
            None
        })
        .unwrap_or_default();

    let mut_set = accounts_mut.get(&ctx_struct).cloned().unwrap_or_default();
    let init_set = accounts_init.get(&ctx_struct).cloned().unwrap_or_default();

    let fn_name = func.sig.ident.to_string();
    for stmt in &func.block.stmts {
        detect_stmt(stmt, file, &fn_name, &ctx_struct, &mut_set, &init_set);
    }
}

fn detect_stmt(
    stmt: &Stmt,
    file: &str,
    fn_name: &str,
    ctx_struct: &str,
    mut_set: &HashSet<String>,
    init_set: &HashSet<String>,
) {
    match stmt {
        Stmt::Expr(expr, _) => detect_expr(expr, file, fn_name, ctx_struct, mut_set, init_set),

        Stmt::Local(local) => {
            if let Some(LocalInit { expr, .. }) = &local.init {
                detect_expr(expr, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Stmt::Item(_) | Stmt::Macro(_) => {}
    }
}

fn detect_expr(
    expr: &Expr,
    file: &str,
    fn_name: &str,
    ctx_struct: &str,
    mut_set: &HashSet<String>,
    init_set: &HashSet<String>,
) {
    match expr {
        // &mut ctx.accounts.foo
        Expr::Reference(ExprReference {
            mutability: Some(_),
            expr: inner,
            ..
        }) => {
            if let Expr::Field(f) = &**inner {
                if let Some(field_name) = extract_account_field(f) {
                    if !mut_set.contains(&field_name) {
                        let line = f.span().start().line;
                        println!(
                            "{} `{}` is mutably borrowed in `{}` but not declared `mut` in `{}`. \
Please add `#[account(mut)]` to `{}`. ({}:{})\n",
                            "[ERROR]".red().bold(),
                            field_name,
                            fn_name,
                            ctx_struct,
                            field_name,
                            file,
                            line
                        );
                    }
                }
            }
            detect_expr(&*inner, file, fn_name, ctx_struct, mut_set, init_set);
        }

        // assignment ctx.accounts.foo.<...> = ...
        Expr::Assign(ExprAssign { left, right, .. }) => {
            detect_field_mutation(left, file, fn_name, ctx_struct, mut_set, init_set);
            detect_expr(right, file, fn_name, ctx_struct, mut_set, init_set);
        }

        Expr::Binary(ExprBinary { left, op, right, .. }) => {
            match op {
                BinOp::AddAssign(_)
                | BinOp::SubAssign(_)
                | BinOp::MulAssign(_)
                | BinOp::DivAssign(_)
                | BinOp::RemAssign(_) => {
                    // treat it like an assignment to the left side
                    detect_field_mutation(left, file, fn_name, ctx_struct, mut_set, init_set);
                }
                _ => {}
            }
            // still recurse into both sides
            detect_expr(left,  file, fn_name, ctx_struct, mut_set, init_set);
            detect_expr(right, file, fn_name, ctx_struct, mut_set, init_set);
        }

        Expr::Try(syn::ExprTry { expr: inner, .. }) => {
            // unwrap the inner expression (the call) and run again
            detect_expr(&*inner, file, fn_name, ctx_struct, mut_set, init_set);
        }

        // if you also want to catch `try { ... }?`, add:
        Expr::TryBlock(syn::ExprTryBlock { block, .. }) => {
            for stmt in &block.stmts {
                detect_stmt(stmt, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Expr::MethodCall(mc) => {
            detect_expr(&mc.receiver, file, fn_name, ctx_struct, mut_set, init_set);
            for arg in &mc.args {
                detect_expr(arg, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Expr::Block(ExprBlock { block, .. }) => {
            for stmt in &block.stmts {
                detect_stmt(stmt, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Expr::If(ExprIf {
            cond,
            then_branch,
            else_branch,
            ..
        }) => {
            detect_expr(cond, file, fn_name, ctx_struct, mut_set, init_set);
            for stmt in &then_branch.stmts {
                detect_stmt(stmt, file, fn_name, ctx_struct, mut_set, init_set);
            }
            if let Some((_, else_expr)) = else_branch {
                detect_expr(else_expr, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Expr::While(ExprWhile { cond, body, .. }) => {
            detect_expr(cond, file, fn_name, ctx_struct, mut_set, init_set);
            for stmt in &body.stmts {
                detect_stmt(stmt, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Expr::ForLoop(ExprForLoop { expr, body, .. }) => {
            detect_expr(expr, file, fn_name, ctx_struct, mut_set, init_set);
            for stmt in &body.stmts {
                detect_stmt(stmt, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Expr::Loop(ExprLoop { body, .. }) => {
            for stmt in &body.stmts {
                detect_stmt(stmt, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Expr::Match(ExprMatch { expr, arms, .. }) => {
            detect_expr(expr, file, fn_name, ctx_struct, mut_set, init_set);
            for arm in arms {
                detect_expr(&arm.body, file, fn_name, ctx_struct, mut_set, init_set);
            }
        }

        Expr::Path(_) | Expr::Field(_) => {}

        _ => {}
    }
}

/// Handle assignments to `ctx.accounts.foo.* = ...`
fn detect_field_mutation(
    expr: &Expr,
    file: &str,
    fn_name: &str,
    ctx_struct: &str,
    mut_set: &HashSet<String>,
    init_set: &HashSet<String>
) {
    let mut idents = Vec::new();
    let mut current = expr;
    while let Expr::Field(ExprField { base, member, .. }) = current {
        if let Member::Named(ident) = member {
            idents.push(ident.to_string());
        } else {
            return;
        }
        current = &*base;
    }
    // now current should be `ctx` path
    if let Expr::Path(ExprPath { path, .. }) = current {
        if path.is_ident("ctx") {
            // idents is ["balance", "receiver", "accounts"] for `ctx.accounts.receiver.balance`
            // reverse it so [ "accounts", "receiver", "balance" ]
            idents.reverse();
            if idents.get(0) == Some(&"accounts".to_string()) {
                if let Some(acct_name) = idents.get(1) {
                    // this is the account you're mutating
                    if !mut_set.contains(acct_name) && !init_set.contains(acct_name) {
                        let line = expr.span().start().line;
                        println!(
                            "{} `{}` is mutated in `{}` but not declared `mut` in `{}`. \
Please add `#[account(mut)]` to `{}`. ({}:{})\n",
                            "[ERROR]".red().bold(),
                            acct_name,
                            fn_name,
                            ctx_struct,
                            acct_name,
                            file,
                            line
                        );
                    }
                }
            }
        }
    }
}

/// If f represents `ctx.accounts.foo`, return Some("foo").
fn extract_account_field(f: &ExprField) -> Option<String> {
    if let Expr::Field(inner) = &*f.base {
        if let Member::Named(accounts_ident) = &inner.member {
            if accounts_ident == "accounts" {
                if let Expr::Path(ExprPath { path, .. }) = &*inner.base {
                    if path.is_ident("ctx") {
                        if let Member::Named(field_ident) = &f.member {
                            return Some(field_ident.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

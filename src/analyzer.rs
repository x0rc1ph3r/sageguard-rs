use crate::checks;
use crate::checks::GlobalSeedUsage;
use crate::checks::mut_borrow_check::attr_contains_mut;
use crate::utils::is_anchor_account_struct;
use colored::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use syn::{Fields, File, Item, parse_file};
use walkdir::WalkDir;

pub fn analyze_path(path: &str) -> Result<(), String> {
    let path_obj = Path::new(path);

    if !path_obj.exists() {
        return Err(format!(
            "{} Path '{}' does not exist.",
            "[ERROR]".red().bold(),
            path
        ));
    }

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let path_str = entry.path().to_str().unwrap();
        if let Ok(content) = fs::read_to_string(entry.path()) {
            match parse_file(&content) {
                Ok(parsed) => analyze_file(&parsed, path_str),
                Err(e) => eprintln!(
                    "{} Failed to parse {}: {}",
                    "[ERROR]".red().bold(),
                    path_str,
                    e
                ),
            }
        }
    }

    Ok(())
}

fn analyze_file(file: &File, filename: &str) {
    let mut all_seeds = Vec::<GlobalSeedUsage>::new();
    let mut accounts_mut_fields: HashMap<String, HashSet<String>> = HashMap::new();
    let mut accounts_init_fields: HashMap<String, HashSet<String>> = HashMap::new();

    // ==== PASS 1: Struct‐level checks & collect maps / seeds ====
    for item in &file.items {
        if let Item::Struct(s) = item {
            if is_anchor_account_struct(&s.attrs) {
                // Info
                let line = s.ident.span().start().line;
                println!(
                    "{} Found #[derive(Accounts)] struct: {} ({}:{})\n",
                    "[INFO]".cyan().bold(),
                    s.ident,
                    filename,
                    line
                );

                // existing per‐struct checks
                checks::signer_check::check_missing_signer(s, filename);
                checks::state_overwrite_check::check_duplicate_account_types(s, filename);
                checks::init_if_needed_check::check_init_if_needed(s, filename);
                checks::seeds_reuse_check::collect_seeds(s, filename, &mut all_seeds);

                // build the mut‐map
                let mut set = HashSet::new();
                if let Fields::Named(fields) = &s.fields {
                    for f in &fields.named {
                        for attr in &f.attrs {
                            if attr_contains_mut(attr) {
                                set.insert(f.ident.as_ref().unwrap().to_string());
                            }
                        }
                    }
                }
                accounts_mut_fields.insert(s.ident.to_string(), set);

                // int-map
                let mut inited = HashSet::new();
                if let Fields::Named(fields) = &s.fields {
                    for f in &fields.named {
                        for attr in &f.attrs {
                            // look for either `init` or `init_if_needed`
                            if attr
                                .parse_nested_meta(|meta| {
                                    let name = meta.path.get_ident().unwrap().to_string();
                                    if name == "init" || name == "init_if_needed" {
                                        return Err(syn::Error::new_spanned(meta.path.clone(), ""));
                                    }
                                    Ok(())
                                })
                                .is_err()
                            {
                                inited.insert(f.ident.as_ref().unwrap().to_string());
                            }
                        }
                    }
                }
                accounts_init_fields.insert(s.ident.to_string(), inited);
            }
        }
    }

    // debug once after pass 1
    // eprintln!("[DEBUG accounts_mut_fields] = {:#?}", accounts_mut_fields);

    // ==== PASS 2: Program module & function‐level checks ====
    for item in &file.items {
        if let Item::Mod(m) = item {
            if m.attrs.iter().any(|attr| attr.path().is_ident("program")) {
                let line = m.ident.span().start().line;
                println!(
                    "{} Found program : {} ({}:{})\n",
                    "[INFO]".cyan().bold(),
                    m.ident,
                    filename,
                    line
                );
                if let Some((_, items)) = &m.content {
                    for inner_item in items {
                        if let Item::Fn(func) = inner_item {
                            let fl = func.sig.ident.span().start().line;
                            println!(
                                "  {} Function inside program: {} ({}:{})\n",
                                "[INFO]".cyan().bold(),
                                func.sig.ident,
                                filename,
                                fl
                            );
                            checks::cpi_check::detect_cpi_in_fn(func, filename);
                            checks::remaining_accounts_check::check_remaining_accounts_usage(
                                func, filename,
                            );
                            checks::realloc_check::check_realloc_usage(func, filename);
                            checks::mut_borrow_check::check_mut_borrow(
                                func,
                                filename,
                                &accounts_mut_fields,
                                &accounts_init_fields,
                            );
                        }
                    }
                }
            }
        }
    }

    // finally, global seed check
    checks::seeds_reuse_check::check_cross_struct_seeds(&all_seeds);
}

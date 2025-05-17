use crate::checks;
use crate::utils::{is_anchor_account_struct};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use syn::{parse_file, File, Item};
use colored::*;

pub fn analyze_path(path: &str) -> Result<(), String> {

    let path_obj = Path::new(path);

    if !path_obj.exists() {
        return Err(format!("{} Path '{}' does not exist.", "[ERROR]".red().bold(), path))
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
                Err(e) => eprintln!("{} Failed to parse {}: {}", "[ERROR]".red().bold(), path_str, e),
            }
        }
    }

    Ok(())
}

fn analyze_file(file: &File, filename: &str) {
    for item in &file.items {
        if let Item::Struct(s) = item {
            if is_anchor_account_struct(&s.attrs) {
                let line = s.ident.span().start().line;
                println!("{} Found #[derive(Accounts)] struct: {} ({}:{})", "[INFO]".cyan().bold(), s.ident, filename, line);
                checks::signer_check::check_missing_signer(s, filename);
            }
        }

        if let Item::Mod(m) = item {
            let line = m.ident.span().start().line;
            if m.attrs.iter().any(|attr| attr.path().is_ident("program")) {
                println!("{} Found program : {} ({}:{})", "[INFO]".cyan().bold(), m.ident, filename, line);
            }

            if let Some((_, items)) = &m.content {
                for inner_item in items {
                    if let Item::Fn(func) = inner_item {
                        let line = func.sig.ident.span().start().line;
                        println!("  {} Function inside program: {} ({}:{})", "[INFO]".cyan().bold(), func.sig.ident, filename, line);
                        // TODO: Add function-level checks here
                    }
                    // TODO: add more checks for structs/enums/etc inside the module here
                }
            } else {
                println!("{} Module '{}' has no inline content", "[WARN]".yellow().bold(), m.ident);
            }
        }

        if let Item::Fn(func) = item {
            let line = func.sig.ident.span().start().line;
            println!("{} Found function: {} ({}:{})", "[INFO]".cyan().bold(), func.sig.ident, filename, line);
            // TODO: Add function-level checks here
        }
    }
}

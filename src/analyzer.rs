use crate::checks;
use crate::utils::{is_anchor_account_struct, is_handler_fn};
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

        if let Item::Fn(func) = item {
            if is_handler_fn(&func.sig.ident) {
                println!("{} Found handler: {} ({})", "[INFO]".cyan().bold(), func.sig.ident, filename);
                // Add function-level checks here
            } else {
                println!("{} Found function: {} ({})", "[INFO]".cyan().bold(), func.sig.ident, filename);
            }
        }
    }
}

use crate::checks;
use crate::utils::{is_anchor_account_struct, is_handler_fn};
use std::fs;
use walkdir::WalkDir;
use syn::{parse_file, File, Item};

pub fn analyze_path(path: &str) {
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let path_str = entry.path().to_str().unwrap();
        if let Ok(content) = fs::read_to_string(entry.path()) {
            match parse_file(&content) {
                Ok(parsed) => analyze_file(&parsed, path_str),
                Err(e) => eprintln!("[ERROR] Failed to parse {}: {}", path_str, e),
            }
        }
    }
}

fn analyze_file(file: &File, filename: &str) {
    for item in &file.items {
        if let Item::Struct(s) = item {
            if is_anchor_account_struct(&s.attrs) {
                println!("[INFO] Found #[derive(Accounts)] struct: {} ({})", s.ident, filename);
                checks::signer_check::check_missing_signer(s, filename);
            }
        }

        if let Item::Fn(func) = item {
            if is_handler_fn(&func.sig.ident) {
                println!("[INFO] Found handler: {} ({})", func.sig.ident, filename);
                // Add function-level checks here
            }
        }
    }
}

use colored::*;
use syn::{Fields, ItemStruct, Type};

pub fn check_missing_signer(item_struct: &ItemStruct, file: &str) {
    if let Fields::Named(fields) = &item_struct.fields {
        // First, check if any field with #[account(...)] has type Signer
        let has_signer = fields.named.iter().any(|field| {
            field.attrs.iter().any(|attr| attr.path().is_ident("account")) &&
            matches!(&field.ty, Type::Path(tp) 
                if tp.path.segments.last().map(|s| s.ident == "Signer").unwrap_or(false))
        });

        if !has_signer {
            let line = item_struct.ident.span().start().line;
            println!(
                "{} Struct `{}` is missing a `Signer` type on one or more accounts. ({}:{})",
                "[WARMING]".yellow().bold(),
                item_struct.ident,
                file,
                line
            );
        }
    }
}
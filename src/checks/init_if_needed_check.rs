use colored::*;
use syn::{Error, Fields, ItemStruct, spanned::Spanned};

/// Warn on any `#[account(init_if_needed, ...)]` usage
pub fn check_init_if_needed(item_struct: &ItemStruct, file: &str) {
    if let Fields::Named(fields) = &item_struct.fields {
        for field in &fields.named {
            for attr in &field.attrs {
                if attr.path().is_ident("account") {
                    let mut found = false;
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("init_if_needed") {
                            found = true;
                            Err(Error::new_spanned(meta.path.clone(), "found"))
                        } else {
                            Ok(())
                        }
                    });

                    if found {
                        let field_name = field.ident.as_ref().unwrap();
                        let line = attr.span().start().line;
                        println!(
                            "{} `init_if_needed` on `{}` in struct `{}` may reinitialize an existing account. Use with caution! ({}:{})\n",
                            "[WARNING]".yellow().bold(),
                            field_name,
                            item_struct.ident,
                            file,
                            line
                        );
                    }
                }
            }
        }
    }
}

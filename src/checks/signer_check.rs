use syn::{ItemStruct, Fields, Error};
use colored::*;

/// Check if any account fields might be missing `signer` attribute
pub fn check_missing_signer(item_struct: &ItemStruct, file: &str) {
    if let Fields::Named(fields) = &item_struct.fields {
        for field in &fields.named {
            for attr in &field.attrs {
                if attr.path().is_ident("account") {
                    // parse nested meta arguments
                    let found_signer = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("signer") {
                            Err(Error::new_spanned(meta.path, "Found Signer")) // found signer, short-circuit
                        } else {
                            Ok(())
                        }
                    }).is_err();

                    if !found_signer {
                        let ident = field.ident.as_ref().unwrap();
                        println!(
                            "{} Account `{}` may be missing `signer` constraint. ({})",
                            "[WARNING]".yellow().bold(),
                            ident,
                            file
                        );
                    }
                }
            }
        }
    }
}


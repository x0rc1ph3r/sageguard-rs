use syn::{Attribute, Error, Ident};

/// Detects if struct is #[derive(Accounts)]

pub fn is_anchor_account_struct(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("derive") {
            // parse nested meta inside derive(...)
            let nested = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("Accounts") {
                    // return an error to short-circuit iteration
                    Err(Error::new_spanned(meta.path, "Found Accounts"))
                } else {
                    Ok(())
                }
            });
            // if Err(()) is returned, it means Accounts was found
            nested.is_err()
        } else {
            false
        }
    })
}


/// Detect handler function (basic heuristic)
pub fn is_handler_fn(ident: &Ident) -> bool {
    ident == "handler" || ident.to_string().starts_with("handle_")
}

use syn::{Fields, ItemStruct, Type, TypePath, PathArguments, GenericArgument};
use std::collections::HashMap;
use colored::*;

/// Recursively unwrap wrapper types like Box<Account<...>> to reach the inner type.
fn unwrap_type_path(ty: &Type) -> Option<&TypePath> {
    match ty {
        Type::Path(type_path) => Some(type_path),
        Type::Reference(inner) => unwrap_type_path(&inner.elem),
        Type::Paren(inner) => unwrap_type_path(&inner.elem),
        Type::Group(inner) => unwrap_type_path(&inner.elem),
        Type::Slice(inner) => unwrap_type_path(&inner.elem),
        Type::Array(inner) => unwrap_type_path(&inner.elem),
        _ => None,
    }
}

pub fn check_duplicate_account_types(item_struct: &ItemStruct, file: &str) {
    let mut type_lines: HashMap<String, Vec<usize>> = HashMap::new();

    if let Fields::Named(fields) = &item_struct.fields {
        for field in &fields.named {
            // Walk through wrapper types like Box<Account<T>>
            let mut current_type = &field.ty;

            loop {
                if let Some(tp) = unwrap_type_path(current_type) {
                    let seg = tp.path.segments.last().unwrap();
                    if seg.ident == "Account" {
                        // Extract generic parameter from Account<T>
                        if let PathArguments::AngleBracketed(args) = &seg.arguments {
                            for arg in &args.args {
                                if let GenericArgument::Type(Type::Path(inner_path)) = arg {
                                    if let Some(inner_seg) = inner_path.path.segments.last() {
                                        let inner_ty = inner_seg.ident.to_string();
                                        let line = field.ident.as_ref().map(|i| i.span().start().line).unwrap_or(0);
                                        type_lines.entry(inner_ty).or_default().push(line);
                                    }
                                }
                            }
                        }
                        break;
                    } else if seg.ident == "Box" {
                        // Recurse into Box<Account<T>>
                        if let PathArguments::AngleBracketed(args) = &seg.arguments {
                            if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                                current_type = inner_ty;
                                continue;
                            }
                        }
                    }
                }

                break;
            }
        }

        for (ty, lines) in type_lines {
            if lines.len() > 1 {
                let line_str = lines.iter().map(|l| format!("{}:{}", file, l)).collect::<Vec<_>>().join(", ");
                println!(
                    "{} Duplicate `Account<{}>` fields in struct `{}`. Consider using a separate `#[derive(Accounts)]` struct. ({})\nFor more details, see: https://hackmd.io/@S3v3ru5/Byia-fQHJe\n",
                    "[ERROR]".red().bold(),
                    ty,
                    item_struct.ident,
                    line_str
                );
            }
        }
    }
}

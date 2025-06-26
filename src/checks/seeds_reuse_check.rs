use colored::*;
use std::collections::HashMap;
use syn::{Fields, ItemStruct, Meta, spanned::Spanned};
// use syn::Error;

pub struct GlobalSeedUsage {
    struct_name: String,
    field_name: String,
    prefix: String,
    file: String,
    line: usize,
}

pub fn collect_seeds(item_struct: &ItemStruct, filename: &str, out: &mut Vec<GlobalSeedUsage>) {
    if let Fields::Named(fields) = &item_struct.fields {
        for field in &fields.named {
            for attr in &field.attrs {
                if !attr.path().is_ident("account") {
                    continue;
                }
                if let Meta::List(meta_list) = &attr.meta {
                    let raw = meta_list.tokens.to_string();
                    if let Some(pos) = raw.find("seeds") {
                        if let Some(bs) = raw[pos..].find('[') {
                            let start = pos + bs;
                            if let Some(re) = raw[start + 1..].find(']') {
                                let end = start + 1 + re;
                                let inner = &raw[start + 1..end]; // `b"pool", signer.key().as_ref()`
                                let first =
                                    inner.splitn(2, ',').next().unwrap_or("").trim().to_string();

                                let line = attr.span().start().line;
                                out.push(GlobalSeedUsage {
                                    struct_name: item_struct.ident.to_string(),
                                    field_name: field.ident.as_ref().unwrap().to_string(),
                                    prefix: first.clone(),
                                    file: filename.to_string(),
                                    line,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

// /// Warn when two accounts in the same struct use:
// /// 1) exactly the same `seeds = [...]` list, or
// /// 2) different lists that share the same first element (prefix collision risk)
// pub fn check_seeds_reuse(item_struct: &ItemStruct, file: &str) {
//     // Map full normalized seeds → list of (field, line)
//     let mut full_seen: HashMap<String, Vec<(String, usize)>> = HashMap::new();
//     // Map first-element → list of (field, line, full_list)
//     let mut first_seen: HashMap<String, Vec<(String, usize, String)>> = HashMap::new();

//     if let Fields::Named(fields) = &item_struct.fields {
//         for field in &fields.named {
//             for attr in &field.attrs {
//                 if attr.path().is_ident("account") {
//                     let mut found: Option<String> = None;
//                     // parse nested meta to find seeds = [...]
//                     let _ = attr.parse_nested_meta(|meta| {
//                         if meta.path.is_ident("seeds") {
//                             let raw = meta.input.to_string(); // e.g. "[b\"pool\", a.key().as_ref()]"
//                             let norm = raw.trim().to_string();
//                             found = Some(norm);
//                             Err(Error::new_spanned(meta.path.clone(), "got seeds"))
//                         } else {
//                             Ok(())
//                         }
//                     });

//                     if let Some(full_list) = found {
//                         let field_name = field.ident.as_ref().unwrap().to_string();
//                         let line = attr.span().start().line;
//                         // record full list
//                         full_seen
//                             .entry(full_list.clone())
//                             .or_default()
//                             .push((field_name.clone(), line));
//                         // extract first element
//                         // strip outer [ ] then split on commas
//                         let inner = full_list
//                             .trim()
//                             .trim_start_matches('[')
//                             .trim_end_matches(']')
//                             .to_string();
//                         if let Some(first) = inner.splitn(2, ',').next() {
//                             let first = first.trim().to_string();
//                             first_seen
//                                 .entry(first)
//                                 .or_default()
//                                 .push((field_name, line, full_list));
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     // 1) identical-list warnings
//     for (seeds, occ) in &full_seen {
//         if occ.len() > 1 {
//             let locations = occ
//                 .iter()
//                 .map(|(name, line)| format!("`{}` @ {}:{}", name, file, line))
//                 .collect::<Vec<_>>()
//                 .join(", ");
//             println!(
//                 "{} Duplicate `seeds = {}` in struct `{}` on: {}",
//                 "[WARNING]".yellow().bold(),
//                 seeds,
//                 item_struct.ident,
//                 locations
//             );
//         }
//     }

//     // 2) prefix-collision warnings (same first, but different full lists)
//     for (first, occ) in &first_seen {
//         // if more than one use of this first element
//         // and not *all* full lists are identical
//         if occ.len() > 1 {
//             let mut distinct_full: Vec<_> = occ.iter().map(|(_, _, full)| full.clone()).collect();
//             distinct_full.sort();
//             distinct_full.dedup();
//             if distinct_full.len() > 1 {
//                 // warn once per first‐element cluster
//                 let details = occ
//                     .iter()
//                     .map(|(name, line, full)| {
//                         format!("`{}` @ {}:{} (using {})", name, file, line, full)
//                     })
//                     .collect::<Vec<_>>()
//                     .join(", ");
//                 println!(
//                     "{} Seeds prefix collision on `{}` in struct `{}`: {}",
//                     "[WARNING]".yellow().bold(),
//                     first,
//                     item_struct.ident,
//                     details
//                 );
//             }
//         }
//     }
// }

pub fn check_cross_struct_seeds(usages: &[GlobalSeedUsage]) {
    let mut by_prefix: HashMap<&str, Vec<&GlobalSeedUsage>> = std::collections::HashMap::new();
    for u in usages {
        by_prefix.entry(u.prefix.as_str()).or_default().push(u);
    }

    for (&prefix, group) in &by_prefix {
        let mut structs = group
            .iter()
            .map(|u| u.struct_name.as_str())
            .collect::<Vec<_>>();
        structs.sort_unstable();
        structs.dedup();

        let mut fields = group
            .iter()
            .map(|u| u.field_name.as_str())
            .collect::<Vec<_>>();
        fields.sort_unstable();
        fields.dedup();

        if fields.len() <= 1 {
            continue;
        }

        if structs.len() > 1 {
            let details = group
                .iter()
                .map(|u| {
                    format!(
                        "{}::{} ({}:{})",
                        u.struct_name, u.field_name, u.file, u.line
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            println!(
                "{} Seed prefix `{}` reused across structs [{}]: {}\n",
                "[WARNING]".yellow().bold(),
                prefix,
                structs.join(", "),
                details
            );
        }
    }
}

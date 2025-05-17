mod analyzer;
mod utils;
mod checks;

use clap::Parser;

/// SageGuard-RS: A static analyzer for Anchor smart contracts
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to analyze
    path: String,
}

fn main() {
    let args = Args::parse();
    if let Err(e) = analyzer::analyze_path(&args.path) {
        eprintln!("{}", e);
    }
}

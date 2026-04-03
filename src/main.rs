use clap::Parser;

mod cli;
mod core;
mod crypto;
mod integrations;
mod storage;
mod tui;

fn main() {
    let cli = cli::commands::Cli::parse();
    if let Err(e) = cli::commands::run(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

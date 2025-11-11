mod commands;
mod errors;
mod output;

use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "quillmark")]
#[command(about = "Command-line interface for Quillmark", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render markdown file to output format
    Render(commands::render::RenderArgs),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Render(args) => commands::render::execute(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

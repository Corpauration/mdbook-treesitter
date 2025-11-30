use anyhow::Result;
use clap::{Parser, Subcommand};
use mdbook_preprocessor::{Preprocessor, errors::Error};
use mdbook_treesitter::MdbookTreesitter;
use std::{io, process};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check whether a renderer is supported by this preprocessor
    Supports { renderer: String },
}

fn main() {
    init_logger();

    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("Fatal error: {error}");
        for error in error.chain() {
            eprintln!("  - {error}");
        }
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        None => handle_preprocessing(),
        Some(Commands::Supports { renderer }) => {
            handle_supports(renderer);
        }
    }
}

fn handle_preprocessing() -> Result<(), Error> {
    let (ctx, book) = mdbook_preprocessor::parse_input(io::stdin())?;

    if ctx.mdbook_version != mdbook_preprocessor::MDBOOK_VERSION {
        eprintln!(
            "Warning: The mdbook-treesitter preprocessor was built against version \
             {} of mdbook, but we're being called from version {}",
            mdbook_preprocessor::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = MdbookTreesitter.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(renderer: String) -> ! {
    let supported = MdbookTreesitter.supports_renderer(&renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if let Ok(supported) = supported
        && supported
    {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn init_logger() {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing::Level::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .with_writer(io::stderr)
        .init();
}

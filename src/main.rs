use std::{io, process};

use anyhow::{Context, Result, bail};
use mdbook_preprocessor::Preprocessor;
use mdbook_treesitter::MdbookTreesitter;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let preprocessor = MdbookTreesitter::default();

    match args.next().as_deref() {
        None => {
            let (ctx, book) = mdbook_preprocessor::parse_input(io::stdin())?;

            if ctx.mdbook_version != mdbook_preprocessor::MDBOOK_VERSION {
                eprintln!(
                    "WARNING mdbook-treesitter: \
                    the proprocessor was built against version {} of mdbook, \
                    but we're being called from version {}",
                    mdbook_preprocessor::MDBOOK_VERSION,
                    ctx.mdbook_version
                );
            }

            let processed_book = preprocessor.run(&ctx, book)?;
            serde_json::to_writer(io::stdout(), &processed_book)?;

            Ok(())
        }
        Some("supports") => {
            let renderer = args
                .next()
                .context("`supports` subcommand needs a second argument")?;

            let supported = preprocessor.supports_renderer(&renderer).unwrap_or(false);
            // signal the renderer, 0 if supported
            process::exit(i32::from(!supported));
        }
        Some(subcommand) => bail!("unknown subcommand: {subcommand}"),
    }
}

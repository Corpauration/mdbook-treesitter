mod treesitter;

use crate::treesitter::MdbookTreesitterHighlighter;
use anyhow::anyhow;
use log::{debug, error};
use mdbook::BookItem;
use mdbook::book::Book;
use mdbook::errors::Result;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::CodeBlockKind::Fenced;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::process::exit;

pub struct MdbookTreesitter;

// Name used by `mdbook` to look for the treesitter preprocessor
const PREPROCESSOR: &str = "treesitter";

impl Preprocessor for MdbookTreesitter {
    fn name(&self) -> &str {
        PREPROCESSOR
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        book.for_each_mut(|item: &mut BookItem| {
            if let BookItem::Chapter(ref mut chapter) = *item {
                if Self::preprocess(ctx, &chapter.content)
                    .map(|md| {
                        chapter.content = md;
                    })
                    .map_err(|err| error!("Failed to preprocess chapter: {err}"))
                    .is_err()
                {
                    exit(1);
                }
            }
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer == "html"
    }
}

fn extract_code_body(content: &str) -> &str {
    const PRE_END: char = '\n';
    const POST: &str = "```";

    let start_index = content
        .find(PRE_END)
        .map(|index| index + 1)
        .unwrap_or_default();
    let end_index = content.len() - POST.len();

    let body = &content[start_index..end_index];
    body.trim()
}

impl MdbookTreesitter {
    fn get_ts_languages(ctx: &PreprocessorContext) -> Result<Vec<&str>> {
        let languages = ctx
            .config
            .get_preprocessor(PREPROCESSOR)
            .and_then(|t| t.get("languages"))
            .ok_or_else(|| {
                anyhow!(
                    "preprocessor.{PREPROCESSOR}.languages is missing from the project 'book.toml'"
                )
            })?;

        let ty_err = || anyhow!("preprocessor.{PREPROCESSOR}.languages must be a list of strings");
        let languages: Result<Vec<_>> = languages
            .as_array()
            .ok_or(ty_err())?
            .iter()
            .map(|v| v.as_str().ok_or(ty_err()))
            .collect();
        languages
    }
    fn parse_code(
        cfg_languages: &[&str],
        info_string: String,
        content: &str,
    ) -> Option<Result<String>> {
        // "```lang" info string must be declared in `book.toml`:
        // ```
        // [preprocessor.treesitter]
        // command = "mdbook-treesitter"
        // languages = [ "lang" ]
        // ```
        if !cfg_languages.contains(&info_string.as_str()) {
            return None;
        }

        debug!("Code block with `{info_string}` language detected");

        let mut highlighter = match MdbookTreesitterHighlighter::new(info_string.as_str()) {
            Ok(h) => h?,
            Err(e) => return Some(Err(e)),
        };

        let body = extract_code_body(content);
        highlighter.html(body).into()
    }

    fn preprocess(ctx: &PreprocessorContext, content: &str) -> Result<String> {
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_FOOTNOTES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);

        let mut code_blocks = vec![];

        let cfg_languages = Self::get_ts_languages(ctx)?;

        let events = Parser::new_ext(content, opts);
        for (e, span) in events.into_offset_iter() {
            if let Event::Start(Tag::CodeBlock(Fenced(info_string))) = e.clone() {
                let span_content = &content[span.start..span.end];
                let html =
                    match Self::parse_code(&cfg_languages, info_string.to_string(), span_content) {
                        Some(html) => html,
                        None => continue,
                    }?;
                code_blocks.push((span, html));
            }
        }

        let mut content = content.to_string();
        for (span, block) in code_blocks.iter().rev() {
            let pre_content = &content[..span.start];
            let post_content = &content[span.end..];
            content = format!("{pre_content}\n{block}{post_content}");
        }
        Ok(content)
    }
}

use std::{borrow::Cow, collections::BTreeMap, mem};

use anyhow::Context;
use mdbook_markdown::pulldown_cmark::{CodeBlockKind::Fenced, Event, Parser, Tag, TagEnd};
use mdbook_preprocessor::{Preprocessor, PreprocessorContext, book::Book};
use serde_json::Value;
use tree_sitter_highlight::HighlightConfiguration;

mod treesitter;

#[derive(Default)]
#[non_exhaustive]
pub struct MdbookTreesitter {}

// Name used by `mdbook` to look for the treesitter preprocessor
const PREPROCESSOR: &str = "treesitter";

impl Preprocessor for MdbookTreesitter {
    fn name(&self) -> &str {
        PREPROCESSOR
    }

    fn run(
        &self,
        ctx: &PreprocessorContext,
        mut book: Book,
    ) -> mdbook_preprocessor::errors::Result<Book> {
        let languages = read_languages_to_handle(ctx).context("could not read config")?;
        let highlighters =
            load_highlighters(&languages).context("could not load all highlighters")?;

        book.for_each_chapter_mut(|chapter| {
            preprocess(&mut chapter.content, &highlighters)
                .with_context(|| anyhow::anyhow!("failed to preprocess chapter"))
                .unwrap();
        });

        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> mdbook_preprocessor::errors::Result<bool> {
        Ok(renderer == "html")
    }
}

fn preprocess(
    content: &mut String,
    highlight_configs: &BTreeMap<String, HighlightConfiguration>,
) -> anyhow::Result<()> {
    let mut processed_output = String::new();
    let mut last_match = 0;

    let mut current_highlight_config = None;
    let mut codeblock_content = Cow::<str>::default();

    for (event, span) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(Fenced(language))) => {
                let Some(highlight_config) = highlight_configs.get(language.as_ref()) else {
                    // we ignore languages that were not registered
                    continue;
                };

                current_highlight_config = Some(highlight_config);

                processed_output.push_str(&content[last_match..span.start]);
                last_match = span.end;
            }
            Event::Text(str) if current_highlight_config.is_some() => {
                let mut next_content = codeblock_content.into_owned();
                next_content.push_str(&str);
                codeblock_content = Cow::Owned(next_content);
            }
            Event::End(TagEnd::CodeBlock) if current_highlight_config.is_some() => {
                let highlight_config = current_highlight_config.take().unwrap();
                let content = mem::take(&mut codeblock_content);

                let html = treesitter::highlight_to_html(highlight_config, &content)?;
                processed_output.push_str(&html);
            }
            _ => {}
        }
    }

    // only replace with copied content if a codeblock was highlighted
    if last_match != 0 {
        processed_output.push_str(&content[last_match..]);
        *content = processed_output;
    }

    Ok(())
}

fn read_languages_to_handle(ctx: &PreprocessorContext) -> anyhow::Result<Vec<String>> {
    let preprocessors = ctx.config.preprocessors::<Value>()?;
    let Some(preprocessor) = preprocessors.get(PREPROCESSOR) else {
        anyhow::bail!("`preprocessor.{PREPROCESSOR}` is missing from the project 'book.toml'")
    };
    let Some(languages) = preprocessor.get("languages") else {
        anyhow::bail!(
            "`preprocessor.{PREPROCESSOR}.languages` is missing from the project 'book.toml'"
        )
    };

    if let Value::Array(languages) = languages
        && let Some(languages) = languages
            .iter()
            .map(|v| v.as_str().map(ToOwned::to_owned))
            .collect::<Option<Vec<_>>>()
    {
        Ok(languages)
    } else {
        anyhow::bail!("`preprocessor.{PREPROCESSOR}.languages` must be a list of strings")
    }
}

fn load_highlighters(
    languages: &[String],
) -> anyhow::Result<BTreeMap<String, HighlightConfiguration>> {
    let mut highlighters = BTreeMap::default();

    for language in languages {
        let highlighter = treesitter::load_config_from_language(language)?;
        highlighters.insert(language.clone(), highlighter);
    }

    Ok(highlighters)
}

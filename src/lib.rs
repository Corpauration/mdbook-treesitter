mod treesitter;

use crate::treesitter::MdbookTreesitterHighlighter;
use anyhow::anyhow;
use log::{debug, error};
use mdbook::BookItem;
use mdbook::book::Book;
use mdbook::errors::Result;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use pulldown_cmark::CodeBlockKind::Fenced;
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};
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

fn filter_hidden_lines(code: &str, hide_prefix: Option<&str>) -> String {
    let prefix = match hide_prefix {
        Some(p) => p,
        None => return code.to_string(), // No hiding if no prefix configured
    };

    code.lines()
        .filter_map(|line| {
            let trimmed = line.trim();

            // Check if trimmed line starts with the hide prefix
            if trimmed.starts_with(prefix) {
                // Hide this line completely
                return None;
            }

            Some(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl MdbookTreesitter {
    fn get_hide_line_prefix(ctx: &PreprocessorContext, language: &str) -> Option<String> {
        ctx.config
            .get("output")
            .and_then(|output| output.get("html"))
            .and_then(|html| html.get("code"))
            .and_then(|code| code.get("hidelines"))
            .and_then(|hidelines| hidelines.get(language))
            .and_then(|prefix| prefix.as_str())
            .map(|s| s.to_string())
    }

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
        ctx: &PreprocessorContext,
        cfg_languages: &[&str],
        info_string: CowStr<'_>,
        content: &str,
    ) -> Option<Result<String>> {
        // "```lang" info string must be declared in `book.toml`:
        // ```
        // [preprocessor.treesitter]
        // command = "mdbook-treesitter"
        // languages = [ "lang" ]
        // ```
        if !cfg_languages.contains(&info_string.as_ref()) {
            return None;
        }

        debug!("Code block with `{info_string}` language detected");

        let mut highlighter = match MdbookTreesitterHighlighter::new(info_string.as_ref()) {
            Ok(h) => h?,
            Err(e) => return Some(Err(e)),
        };

        dbg!(&content);
        let body = extract_code_body(content);
        let hide_prefix = Self::get_hide_line_prefix(ctx, info_string.as_ref());
        let processed_body = filter_hidden_lines(body, hide_prefix.as_deref());
        highlighter.html(&processed_body).into()
    }

    fn preprocess(ctx: &PreprocessorContext, content: &str) -> Result<String> {
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_FOOTNOTES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);

        let cfg_languages = Self::get_ts_languages(ctx)?;
        let mut code_blocks = vec![];

        // Parse markdown to find code blocks
        let events = Parser::new_ext(content, opts);
        for (e, span) in events.into_offset_iter() {
            let Event::Start(Tag::CodeBlock(Fenced(info_string))) = e else {
                continue;
            };
            let span_content = &content[span.start..span.end];
            if let Some(html) =
                Self::parse_code(ctx, &cfg_languages, info_string, span_content).transpose()?
            {
                code_blocks.push((span, html));
            }
        }

        // Replace code blocks in reverse order to maintain correct indices
        let mut content = content.to_string();
        for (span, block) in code_blocks.iter().rev() {
            let pre_content = &content[..span.start];
            let post_content = &content[span.end..];
            content = format!("{}\n{}{}", pre_content, block, post_content);
        }
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_filter_hidden_lines_with_tilde_prefix() {
        let code = indoc! { r#"
        fn main() {
            println!("Hello"); // This line stays
            let x = 42;
        ~   let hidden_var = "secret";
            println!("World"); // This also stays
        ~
            println!("Goodbye"); // This stays too
        }"# };

        let expected = indoc! { r#"
        fn main() {
            println!("Hello"); // This line stays
            let x = 42;
            println!("World"); // This also stays
            println!("Goodbye"); // This stays too
        }"# };

        assert_eq!(filter_hidden_lines(code, Some("~")), expected);
    }

    #[test]
    fn test_filter_hidden_lines_with_hash_prefix() {
        let code = indoc! {
        r#"fn main() {
            println!("Hello"); // This line stays
            let x = 42;
        #   let hidden_var = "secret";
            println!("World"); // This also stays
        #
            println!("Goodbye"); // This stays too
        }"#};

        let expected = indoc! {r#"
        fn main() {
            println!("Hello"); // This line stays
            let x = 42;
            println!("World"); // This also stays
            println!("Goodbye"); // This stays too
        }"#};

        assert_eq!(filter_hidden_lines(code, Some("#")), expected);
    }

    #[test]
    fn test_filter_hidden_lines_with_double_slash_prefix() {
        let code = indoc! { r#"
        function test() {
            console.log("Hello"); // This line stays
            let x = 42;
        //  let hidden_var = "secret";
            console.log("World"); // This also stays
        //
            console.log("Goodbye"); // This stays too
        }"#};

        let expected = indoc! {r#"
        function test() {
            console.log("Hello"); // This line stays
            let x = 42;
            console.log("World"); // This also stays
            console.log("Goodbye"); // This stays too
        }"#};

        assert_eq!(filter_hidden_lines(code, Some("//")), expected);
    }

    #[test]
    fn test_filter_hidden_lines_no_prefix_configured() {
        let code = indoc! { r#"
        fn main() {
            println!("Hello");
        ~   let hidden_var = "secret";
            println!("World");
        }"#};

        // Should be unchanged when no prefix is configured
        assert_eq!(filter_hidden_lines(code, None), code);
    }

    #[test]
    fn test_filter_hidden_lines_only_hidden_lines() {
        let code = indoc! { r#"
        ~
        ~   let x = 42;
        ~   let y = 24;
        "#};

        let expected = "";

        assert_eq!(filter_hidden_lines(code, Some("~")), expected);
    }

    #[test]
    fn test_filter_hidden_lines_mixed_content() {
        let code = indoc! {r#"
        fn main() {
            println!("Hello");
        ~   // This line should be hidden
            let x = 42;
        ~
            println!("World");
        ~   let secret = "hidden";
            println!("Goodbye");
        }"#};

        let expected = indoc! {r#"
        fn main() {
            println!("Hello");
            let x = 42;
            println!("World");
            println!("Goodbye");
        }"# };

        assert_eq!(filter_hidden_lines(code, Some("~")), expected);
    }

    #[test]
    fn test_extract_code_body() {
        let content = indoc! {r#"
        ```rust
        fn main() {
            println!("Hello");
        }
        ```"#};
        let expected = indoc! { r#"
        fn main() {
            println!("Hello");
        }"# };

        assert_eq!(extract_code_body(content), expected);
    }

    #[test]
    fn test_extract_code_body_with_language_options() {
        let content = indoc! {r#"
        ```rust,no_run
        fn main() {
            println!("Hello");
        }```"#};
        let expected = indoc! { r#"
        fn main() {
            println!("Hello");
        }"#};

        assert_eq!(extract_code_body(content), expected);
    }

    #[test]
    fn test_extract_code_body_empty() {
        let content = "```rust\n```";
        let expected = "";

        assert_eq!(extract_code_body(content), expected);
    }

    #[test]
    fn test_filter_hidden_lines_preserve_indentation() {
        // don't use indoc here since it strips intentional indentation
        let code = r#"    fn helper() {
        let x = 42;
~       let hidden_var = "secret";
        println!("test");
    }"#;

        let expected = r#"    fn helper() {
        let x = 42;
        println!("test");
    }"#;

        assert_eq!(filter_hidden_lines(code, Some("~")), expected);
    }
}

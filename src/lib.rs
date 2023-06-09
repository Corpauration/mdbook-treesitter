mod treesitter;

use crate::treesitter::MdbookTreesitterHighlighter;
use mdbook::book::Book;
use mdbook::errors::Result;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use mdbook::BookItem;
use pulldown_cmark::CodeBlockKind::Fenced;
use pulldown_cmark::{Event, Options, Parser, Tag};

pub struct MdbookTreesitter;

impl Preprocessor for MdbookTreesitter {
    fn name(&self) -> &str {
        "treesitter"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let mut res = None;
        book.for_each_mut(|item: &mut BookItem| {
            if let Some(Err(_)) = res {
                return;
            }

            if let BookItem::Chapter(ref mut chapter) = *item {
                res = Some(preprocess(&chapter.content).map(|md| {
                    chapter.content = md;
                }));
            }
        });

        res.unwrap_or(Ok(())).map(|_| book)
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

fn parse_code(info_string: String, content: &str) -> Option<Result<String>> {
    let mut highlighter = match MdbookTreesitterHighlighter::new(info_string.as_str()) {
        Ok(h) => h?,
        Err(e) => return Some(Err(e)),
    };

    let body = extract_code_body(content);
    match highlighter.html(body) {
        Ok(html) => Some(Ok(html)),
        Err(e) => Some(Err(e)),
    }
}

fn preprocess(content: &str) -> Result<String> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let mut code_blocks = vec![];

    let events = Parser::new_ext(content, opts);
    for (e, span) in events.into_offset_iter() {
        if let Event::Start(Tag::CodeBlock(Fenced(info_string))) = e.clone() {
            let span_content = &content[span.start..span.end];
            let html = match parse_code(info_string.to_string(), span_content) {
                Some(html) => html,
                None => continue,
            };
            let html = html?;
            code_blocks.push((span, html));
        }
    }

    let mut content = content.to_string();
    for (span, block) in code_blocks.iter().rev() {
        let pre_content = &content[..span.start];
        let post_content = &content[span.end..];
        content = format!("{}\n{}{}", pre_content, block, post_content);
    }
    Ok(content)
}

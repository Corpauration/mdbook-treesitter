use anyhow::{anyhow, Context, Result};
use libloading::{Library, Symbol};
use map_macro::hash_map;
use regex::Regex;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tree_sitter::Language;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

pub struct MdbookTreesitterHighlighter {
    highlighter: Highlighter,
    config: HighlightConfiguration,
}

impl MdbookTreesitterHighlighter {
    pub fn new(codeblock_lang: &str) -> Result<Option<MdbookTreesitterHighlighter>> {
        let language = MdbookTreesitterHighlighter::get_language(codeblock_lang)?;

        let highlights_query = MdbookTreesitterHighlighter::load_scm(codeblock_lang, "highlights")?;
        let injection_query =
            MdbookTreesitterHighlighter::load_scm(codeblock_lang, "injections").unwrap_or_default();
        let locals_query =
            MdbookTreesitterHighlighter::load_scm(codeblock_lang, "locals").unwrap_or_default();

        let mut config = HighlightConfiguration::new(
            language,
            codeblock_lang,
            &highlights_query,
            &injection_query,
            &locals_query,
        )?;

        config.configure(MdbookTreesitterHighlighter::highlight_names());

        let highlighter = Highlighter::new();

        Ok(Some(MdbookTreesitterHighlighter {
            highlighter,
            config,
        }))
    }

    fn get_language(name: &str) -> Result<Language> {
        let mut library_path = Path::new("treesitter").join(name);
        library_path.set_extension("so");

        let library = unsafe { Library::new(&library_path) }
            .with_context(|| format!("Error opening dynamic library {:?}", library_path))?;
        let language_fn_name = format!("tree_sitter_{}", name.replace('-', "_"));
        let language = unsafe {
            let language_fn: Symbol<unsafe extern "C" fn() -> Language> = library
                .get(language_fn_name.as_bytes())
                .with_context(|| format!("Failed to load symbol {}", language_fn_name))?;
            language_fn()
        };
        std::mem::forget(library);
        Ok(language)
    }

    fn load_scm(language: &str, name: &str) -> Result<String> {
        let regex = Regex::new(r";+\s*inherits\s*:?\s*([a-z_,()-]+)\s*")?;
        let string = &mut String::new();
        File::open(
            Path::new("treesitter")
                .join(language)
                .join(name)
                .with_extension("scm"),
        )?
        .read_to_string(string)?;
        Ok(regex
            .replace_all(string, |captures: &regex::Captures| {
                captures[1]
                    .split(',')
                    .fold(String::new(), |mut output, language| {
                        write!(
                            output,
                            "\n{}\n",
                            MdbookTreesitterHighlighter::load_scm(language, name).unwrap()
                        )
                        .unwrap();
                        output
                    })
            })
            .to_string())
    }

    fn highlight_names<'a>() -> &'a [&'a str] {
        &[
            "type",
            "constructor",
            "constant",
            "constant.builtin",
            "constant.character",
            "constant.character.escape",
            "string",
            "string.regexp",
            "string.special",
            "string.escape",
            "escape",
            "comment",
            "variable",
            "variable.parameter",
            "variable.builtin",
            "variable.other.member",
            "label",
            "punctuation",
            "punctuation.special",
            "keyword",
            "keyword.storage.modifier.ref",
            "keyword.control.conditional",
            "operator",
            "function",
            "function.macro",
            "tag",
            "attribute",
            "namespace",
            "special",
            "markup.heading.marker",
            "markup.heading.1",
            "markup.heading.2",
            "markup.heading.3",
            "markup.heading.4",
            "markup.heading.5",
            "markup.heading.6",
            "markup.list",
            "markup.bold",
            "markup.italic",
            "markup.strikethrough",
            "markup.link.url",
            "markup.link.text",
            "markup.raw",
            "diff.plus",
            "diff.minus",
            "diff.delta",
            "number",
        ]
    }

    pub fn html(&mut self, s: &str) -> Result<String> {
        let map = hash_map! {
            "type" => "hljs-type",
            "constructor" => "hljs-title function_",
            "constant" => "hljs-variable constant_",
            "constant.builtin" => "hljs-built_in",
            "constant.character" => "hljs-symbol",
            "constant.character.escape" => "hljs-symbol",
            "string" => "hljs-string",
            "string.regexp" => "hljs-regexp",
            "string.special" => "hljs-string",
            "string.escape" => "hljs-char escape_",
            "escape" => "hljs-char escape_",
            "comment" => "hljs-comment",
            "variable" => "hljs-variable",
            "variable.parameter" => "hljs-params",
            "variable.builtin" => "hljs-built_in",
            "variable.other.member" => "hljs-variable",
            "label" => "hljs-symbol",
            "punctuation" => "hljs-punctuation",
            "punctuation.special" => "hljs-punctuation",
            "keyword" => "hljs-keyword",
            "keyword.storage.modifier.ref" => "hljs-keyword",
            "keyword.control.conditional" => "hljs-keyword",
            "operator" => "hljs-operator",
            "function" => "hljs-title function_",
            "function.macro" => "hljs-title function_",
            "tag" => "hljs-tag",
            "attribute" => "hljs-attribute",
            "namespace" => "hljs-title class_",
            "special" => "hljs-literal",
            "number" => "hljs-number",
        };

        let highlights = self
            .highlighter
            .highlight(&self.config, s.as_bytes(), None, |_| None)?;

        let mut result = "<pre><code class=\"hljs\">\n".to_string();

        for event in highlights {
            match event? {
                HighlightEvent::Source { start, end } => {
                    let code_span =
                        html_escape::encode_text(s.get(start..end).unwrap()).to_string();
                    result.push_str(&code_span);
                }
                HighlightEvent::HighlightStart(s) => {
                    let highlight = MdbookTreesitterHighlighter::highlight_names()
                        .get(s.0)
                        .ok_or(anyhow!(
                            "no highlight name found for highlight index {}",
                            s.0
                        ))?;
                    let name = map.get(highlight).ok_or(anyhow!(
                        "no highlightjs match found for highlight `{}`",
                        highlight
                    ))?;
                    result.push_str(&format!("<span class='{}'>", name));
                }
                HighlightEvent::HighlightEnd => {
                    result.push_str("</span>");
                }
            }
        }
        result.push_str("\n</code></pre>");

        Ok(result)
    }
}

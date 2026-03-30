use std::{fmt::Write, fs::File, io::Read, mem, path::Path, sync::LazyLock};

use anyhow::Context;
use libloading::{Library, Symbol};
use phf::phf_map;
use regex::Regex;
use tree_sitter::Language;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

pub fn load_config_from_language(lang_code: &str) -> anyhow::Result<HighlightConfiguration> {
    let language = load_language(lang_code)?;

    let highlights_query = load_scm_query(lang_code, "highlights")?;
    let injection_query = load_scm_query(lang_code, "injections").unwrap_or_default();
    let locals_query = load_scm_query(lang_code, "locals").unwrap_or_default();

    let mut config = HighlightConfiguration::new(
        language,
        lang_code,
        &highlights_query,
        &injection_query,
        &locals_query,
    )?;

    config.configure(HIGHLIGHT_NAMES);

    Ok(config)
}

fn load_language(name: &str) -> anyhow::Result<Language> {
    let library_path = Path::new("treesitter")
        .join(name)
        .with_added_extension("so");

    let library = unsafe { Library::new(&library_path) }
        .with_context(|| format!("error opening dynamic library {}", library_path.display()))?;

    let language_fn_name = format!("tree_sitter_{}", name.replace('-', "_"));

    // SAFETY: we ensure the library exists until the end by forgetting it
    let language = unsafe {
        let language_fn: Symbol<unsafe extern "C" fn() -> Language> = library
            .get(language_fn_name.as_bytes())
            .with_context(|| format!("failed to load symbol `{language_fn_name}`"))?;

        language_fn()
    };

    mem::forget(library);

    Ok(language)
}

fn load_scm_query(language: &str, name: &str) -> anyhow::Result<String> {
    static INHERIT_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r";+\s*inherits\s*:?\s*([a-z_,()-]+)\s*").unwrap());

    let path = Path::new("treesitter")
        .join(language)
        .join(name)
        .with_extension("scm");

    let mut query = String::new();
    File::open(path)?.read_to_string(&mut query)?;

    let mut all_captures = INHERIT_REGEX.captures_iter(&query).peekable();

    // avoid unnecessary allocation
    if all_captures.peek().is_none() {
        return Ok(query);
    }

    let mut expanded_query = String::with_capacity(query.len());
    let mut last_match = 0;

    for captures in all_captures {
        let capture = &captures.get(1).unwrap();
        let inherited_languages = capture.as_str().split(',');

        expanded_query.push_str(&query[last_match..capture.start()]);

        for language in inherited_languages {
            let subquery = load_scm_query(language, name).unwrap();
            expanded_query.push_str(&subquery);
        }

        last_match = capture.end();
    }
    expanded_query.push_str(&query[last_match..]);

    Ok(expanded_query)
}

pub fn highlight_to_html(config: &HighlightConfiguration, source: &str) -> anyhow::Result<String> {
    let mut highlighter = Highlighter::new();
    let highlights = highlighter.highlight(config, source.as_bytes(), None, |_| None)?;

    let mut result = String::new();

    result.push_str("<pre><code class=\"hljs\">");
    for event in highlights {
        match event? {
            HighlightEvent::HighlightStart(Highlight(s)) => {
                let highlight_name = HIGHLIGHT_NAMES.get(s).unwrap();
                let name = TS_TO_HIGHLIGHT_JS.get(highlight_name).ok_or_else(|| {
                    anyhow::anyhow!("no highlightjs match found for highlight `{highlight_name}`")
                })?;
                let _ = write!(result, "<span class=\"{name}\">");
            }
            HighlightEvent::Source { start, end } => {
                // newlines break html parsing in markdown when in an indented section, e.g. a list
                let no_newlines =
                    html_escape::encode_text(&source[start..end]).replace('\n', "<br>");
                result.push_str(&no_newlines);
            }
            HighlightEvent::HighlightEnd => {
                result.push_str("</span>");
            }
        }
    }
    result.push_str("</code></pre>");

    Ok(result)
}

const HIGHLIGHT_NAMES: &[&str; 47] = &[
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
];

static TS_TO_HIGHLIGHT_JS: phf::Map<&'static str, &'static str> = phf_map! {
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

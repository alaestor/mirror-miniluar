#![forbid(unsafe_code)]

mod assemble;
mod lexer;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Zero-based byte offset in the original source.
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub remove_comments: bool,
    pub remove_whitespace: bool,
    pub keep_newlines: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            remove_comments: true,
            remove_whitespace: true,
            keep_newlines: false,
        }
    }
}

/// Minify while preserving the requested trivia. Retained newline bytes are
/// copied exactly; `keep_newlines` does not infer statement boundaries.
pub fn minify_with_options(source: &[u8], options: Options) -> Result<Vec<u8>, Error> {
    let tokens = lexer::lex(source)?;
    Ok(assemble::assemble(&tokens, source.len(), options))
}

/// Strip comments and unnecessary whitespace without rewriting literal bytes.
/// This checks lexical completeness, not the full Lua or Luau grammar.
pub fn minify(source: &[u8]) -> Result<Vec<u8>, Error> {
    minify_with_options(source, Options::default())
}

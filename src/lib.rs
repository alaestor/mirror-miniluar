#![forbid(unsafe_code)]

mod assemble;
mod lexer;
mod literals;
mod rename;

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
pub struct Options<'a> {
    pub remove_comments: bool,
    pub remove_whitespace: bool,
    pub keep_newlines: bool,
    pub minify_strings: bool,
    pub minify_numbers: bool,
    pub rename_locals: bool,
    pub preserve_names: &'a [&'a str],
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            remove_comments: true,
            remove_whitespace: true,
            keep_newlines: false,
            minify_strings: true,
            minify_numbers: true,
            rename_locals: false,
            preserve_names: &[],
        }
    }
}

/// Minify while preserving the requested trivia. Retained newline bytes are
/// copied exactly; `keep_newlines` does not infer statement boundaries.
pub fn minify_with_options(source: &[u8], options: Options) -> Result<Vec<u8>, Error> {
    let mut tokens = lexer::lex(source)?;
    if options.rename_locals {
        rename::rename(&mut tokens, source.len(), options.preserve_names)?;
    }
    literals::shorten(&mut tokens, options);
    Ok(assemble::assemble(&tokens, source.len(), options))
}

/// Strip comments and unnecessary whitespace and conservatively shorten literals.
/// This checks lexical completeness, not the full Lua or Luau grammar.
pub fn minify(source: &[u8]) -> Result<Vec<u8>, Error> {
    minify_with_options(source, Options::default())
}

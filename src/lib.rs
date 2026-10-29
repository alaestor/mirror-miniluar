#![forbid(unsafe_code)]

mod lexer;

use std::fmt;

use lexer::{Kind, Token, lex, needs_separator};

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

/// Strip comments and unnecessary whitespace without rewriting literal bytes.
/// This checks lexical completeness, not the full Lua or Luau grammar.
pub fn minify(source: &[u8]) -> Result<Vec<u8>, Error> {
    let tokens = lex(source)?;
    let mut output = Vec::with_capacity(source.len());
    let mut previous: Option<Token<'_>> = None;
    for token in tokens {
        match token.kind {
            Kind::Space | Kind::Comment => continue,
            Kind::Shebang => {
                output.extend_from_slice(token.text);
                output.push(b'\n');
                continue;
            }
            _ => {}
        }
        if previous.is_some_and(|a| needs_separator(a, token)) {
            output.push(b' ');
        }
        output.extend_from_slice(token.text);
        previous = Some(token);
    }
    if !output.ends_with(b"\n") {
        output.push(b'\n');
    }
    Ok(output)
}

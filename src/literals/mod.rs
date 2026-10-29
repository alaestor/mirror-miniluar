mod number;
mod string;

use std::borrow::Cow;

use crate::Options;
use crate::lexer::{Kind, Token};

pub(crate) fn shorten(tokens: &mut [Token<'_>], options: Options) {
    for token in tokens {
        let replacement = match token.kind {
            Kind::Number if options.minify_numbers => number::shorten(&token.text),
            Kind::Literal if options.minify_strings => string::shorten(&token.text),
            _ => None,
        };
        if let Some(text) = replacement {
            token.text = Cow::Owned(text);
        }
    }
}

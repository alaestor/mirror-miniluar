use crate::Options;
use crate::lexer::{Kind, Token, needs_separator};

pub(crate) fn assemble(tokens: &[Token<'_>], capacity: usize, options: Options) -> Vec<u8> {
    let mut assembler = Assembler {
        output: Vec::with_capacity(capacity),
        previous: None,
        line_comment: false,
    };
    for &token in tokens {
        match token.kind {
            Kind::Space => assembler.whitespace(token.text, options),
            Kind::LineComment | Kind::LongComment if options.remove_comments => {}
            _ => assembler.token(token),
        }
    }
    if !assembler
        .output
        .last()
        .is_some_and(|b| matches!(b, b'\r' | b'\n'))
    {
        assembler.output.push(b'\n');
    }
    assembler.output
}

struct Assembler<'a> {
    output: Vec<u8>,
    previous: Option<Token<'a>>,
    line_comment: bool,
}

impl<'a> Assembler<'a> {
    fn whitespace(&mut self, text: &[u8], options: Options) {
        let start = self.output.len();
        if !options.remove_whitespace {
            self.output.extend_from_slice(text);
        } else if options.keep_newlines {
            self.output
                .extend(text.iter().filter(|b| matches!(b, b'\r' | b'\n')));
        }
        let appended = &self.output[start..];
        if !appended.is_empty() {
            self.previous = None;
        }
        if appended.iter().any(|b| matches!(b, b'\r' | b'\n')) {
            self.line_comment = false;
        }
    }

    fn token(&mut self, token: Token<'a>) {
        if self.line_comment {
            self.output.push(b'\n');
        }
        if self
            .previous
            .is_some_and(|previous| needs_separator(previous, token))
        {
            self.output.push(b' ');
        }
        self.output.extend_from_slice(token.text);
        self.line_comment = matches!(token.kind, Kind::LineComment | Kind::Shebang);
        // A comment is itself a separator. Keeping it as `previous` would
        // impose spurious boundaries based on the comment's last byte.
        self.previous = match token.kind {
            Kind::LineComment | Kind::LongComment | Kind::Shebang => None,
            _ => Some(token),
        };
    }
}

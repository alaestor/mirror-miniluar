//! Byte ranges borrow source unchanged, including non-UTF-8 literal bytes.
use crate::Error;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Word,
    Number,
    Literal,
    Operator,
    Space,
    LineComment,
    LongComment,
    Shebang,
}

#[derive(Clone, Debug)]
pub(crate) struct Token<'a> {
    pub offset: usize,
    pub kind: Kind,
    pub text: Cow<'a, [u8]>,
}

const OPERATORS: &[&[u8]] = &[
    b"...", b"//=", b"..=", b"..", b"//", b"==", b"~=", b"<=", b">=", b"<<", b">>", b"->", b"::",
    b"+=", b"-=", b"*=", b"/=", b"%=", b"^=", b"&=", b"|=",
];

pub(crate) fn word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

pub(crate) fn space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
}

pub(crate) fn lex(source: &[u8]) -> Result<Vec<Token<'_>>, Error> {
    let mut lexer = Lexer { source, pos: 0 };
    let mut tokens = Vec::new();
    while lexer.pos < source.len() {
        tokens.push(lexer.next(0)?);
    }
    Ok(tokens)
}

struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    fn starts(&self, text: &[u8]) -> bool {
        self.source[self.pos..].starts_with(text)
    }

    fn consume_while(&mut self, predicate: impl Fn(u8) -> bool) {
        while self.peek().is_some_and(&predicate) {
            self.pos += 1;
        }
    }

    fn error(&self, start: usize, message: &'static str) -> Error {
        Error {
            offset: start,
            message,
        }
    }

    fn next(&mut self, depth: usize) -> Result<Token<'a>, Error> {
        let start = self.pos;
        let byte = self.source[start];
        let kind = if start == 0 && self.starts(b"#!") {
            self.consume_while(|b| !matches!(b, b'\r' | b'\n'));
            Kind::Shebang
        } else if space(byte) {
            self.consume_while(space);
            Kind::Space
        } else if self.starts(b"--") {
            self.pos += 2;
            if let Some(level) = self.bracket_level() {
                self.long_bracket(level)?;
                Kind::LongComment
            } else {
                self.consume_while(|b| !matches!(b, b'\r' | b'\n'));
                Kind::LineComment
            }
        } else if let Some(level) = self.bracket_level() {
            self.long_bracket(level)?;
            Kind::Literal
        } else if matches!(byte, b'\'' | b'"') {
            self.quoted()?;
            Kind::Literal
        } else if byte == b'`' {
            if depth >= 128 {
                return Err(self.error(start, "interpolation nesting limit exceeded"));
            }
            self.interpolation(depth + 1)?;
            Kind::Literal
        } else if byte.is_ascii_digit()
            || (byte == b'.' && self.source.get(start + 1).is_some_and(u8::is_ascii_digit))
        {
            self.number();
            Kind::Number
        } else if word(byte) {
            self.consume_while(word);
            Kind::Word
        } else {
            self.pos += OPERATORS
                .iter()
                .find(|op| self.starts(op))
                .map_or(1, |op| op.len());
            Kind::Operator
        };
        Ok(Token {
            offset: start,
            kind,
            text: Cow::Borrowed(&self.source[start..self.pos]),
        })
    }

    fn bracket_level(&self) -> Option<usize> {
        if self.peek() != Some(b'[') {
            return None;
        }
        let mut end = self.pos + 1;
        while self.source.get(end) == Some(&b'=') {
            end += 1;
        }
        (self.source.get(end) == Some(&b'[')).then_some(end - self.pos - 1)
    }

    fn long_bracket(&mut self, level: usize) -> Result<(), Error> {
        let start = self.pos;
        self.pos += level + 2;
        while self.pos < self.source.len() {
            if self.peek() == Some(b']') {
                let mut end = self.pos + 1;
                while self.source.get(end) == Some(&b'=') {
                    end += 1;
                }
                if end - self.pos - 1 == level && self.source.get(end) == Some(&b']') {
                    self.pos = end + 1;
                    return Ok(());
                }
                self.pos = end;
            } else {
                self.pos += 1;
            }
        }
        Err(self.error(start, "unfinished long bracket"))
    }

    fn quoted(&mut self) -> Result<(), Error> {
        let start = self.pos;
        let quote = self.source[start];
        self.pos += 1;
        while let Some(byte) = self.peek() {
            self.pos += 1;
            match byte {
                b'\\' => {
                    if let Some(escaped) = self.peek() {
                        self.pos += 1;
                        if escaped == b'z' {
                            self.consume_while(space);
                        } else if matches!(escaped, b'\r' | b'\n')
                            && self
                                .peek()
                                .is_some_and(|b| matches!(b, b'\r' | b'\n') && b != escaped)
                        {
                            self.pos += 1;
                        }
                    }
                }
                b if b == quote => return Ok(()),
                b'\r' | b'\n' => return Err(self.error(start, "unescaped newline in string")),
                _ => {}
            }
        }
        Err(self.error(start, "unfinished string"))
    }

    fn interpolation(&mut self, depth: usize) -> Result<(), Error> {
        let start = self.pos;
        self.pos += 1;
        while let Some(byte) = self.peek() {
            self.pos += 1;
            match byte {
                b'\\' => self.pos += usize::from(self.peek().is_some()),
                b'`' => return Ok(()),
                b'{' => {
                    let mut braces = 1usize;
                    while braces > 0 && self.pos < self.source.len() {
                        let token = self.next(depth)?;
                        if token.kind == Kind::Operator {
                            match token.text.as_ref() {
                                b"{" => braces += 1,
                                b"}" => braces -= 1,
                                _ => {}
                            }
                        }
                    }
                    if braces > 0 {
                        return Err(self.error(start, "unfinished interpolation expression"));
                    }
                }
                _ => {}
            }
        }
        Err(self.error(start, "unfinished interpolated string"))
    }

    fn number(&mut self) {
        // Keep spelling intact. Validation of numeral grammar belongs to the interpreter.
        let hex = self.starts(b"0x") || self.starts(b"0X");
        if self.starts(b"0b") || self.starts(b"0B") {
            self.pos += 2;
            self.consume_while(|b| matches!(b, b'0' | b'1' | b'_'));
            return;
        }
        if hex {
            self.pos += 2;
        }
        let digit = |b: u8| {
            b == b'_'
                || if hex {
                    b.is_ascii_hexdigit()
                } else {
                    b.is_ascii_digit()
                }
        };
        self.consume_while(digit);
        if self.peek() == Some(b'.') {
            self.pos += 1;
            self.consume_while(digit);
        }
        if self.peek().is_some_and(|b| {
            if hex {
                matches!(b, b'p' | b'P')
            } else {
                matches!(b, b'e' | b'E')
            }
        }) {
            self.pos += 1;
            if self.peek().is_some_and(|b| matches!(b, b'+' | b'-')) {
                self.pos += 1;
            }
            self.consume_while(|b| b.is_ascii_digit() || b == b'_');
        }
    }
}

pub(crate) fn needs_separator(a: &Token<'_>, b: &Token<'_>) -> bool {
    let last = *a.text.last().expect("nonempty token");
    let first = b.text[0];
    (word(last) && word(first))
        || (a.kind == Kind::Number && (first == b'.' || b.kind == Kind::Word))
        || (last == b'-' && first == b'-')
        // Also guard multi-token openers such as [ = [.
        || (last == b'[' && matches!(first, b'[' | b'='))
        || (last == b'.' && (first == b'.' || b.kind == Kind::Number))
        || (a.kind == Kind::Operator && b.kind == Kind::Operator
            && OPERATORS.iter().any(|op| op.starts_with(&[last, first])))
}

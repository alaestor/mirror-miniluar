use super::{Parser, Result};
use crate::lexer::Kind;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Prefix {
    Variable,
    Call,
    Value,
}

impl Parser<'_, '_> {
    pub(super) fn expression_list(&mut self) -> Result<()> {
        self.expression(0)?;
        while self.accept(b",") {
            self.expression(0)?;
        }
        Ok(())
    }

    pub(super) fn expression(&mut self, minimum: u8) -> Result<()> {
        self.nested(|parser| parser.expression_inner(minimum))
    }

    fn expression_inner(&mut self, minimum: u8) -> Result<()> {
        if matches!(self.peek(), b"-" | b"not" | b"#" | b"~") {
            self.take()?;
            self.expression(11)?;
        } else {
            match self.peek() {
                b"nil" | b"true" | b"false" | b"..." => {
                    self.take()?;
                }
                b"function" => {
                    self.take()?;
                    self.function_body(false)?;
                }
                b"{" => self.table()?,
                _ => {
                    let index = *self.significant.get(self.pos).ok_or_else(|| self.error())?;
                    match self.tokens[index].kind {
                        Kind::Number => {
                            self.take()?;
                        }
                        Kind::Literal if !self.peek().starts_with(b"`") => {
                            self.take()?;
                        }
                        _ => {
                            self.prefix()?;
                        }
                    }
                }
            }
        }
        while let Some((left, right)) = precedence(self.peek()) {
            if left < minimum {
                break;
            }
            self.take()?;
            self.expression(right)?;
        }
        Ok(())
    }

    pub(super) fn prefix(&mut self) -> Result<Prefix> {
        let mut kind = if self.accept(b"(") {
            self.expression(0)?;
            self.expect(b")")?;
            Prefix::Value
        } else {
            let index = self.identifier()?;
            self.reference(index);
            Prefix::Variable
        };
        loop {
            match self.peek() {
                b"." => {
                    self.take()?;
                    self.identifier()?;
                    kind = Prefix::Variable;
                }
                b"[" => {
                    self.take()?;
                    self.expression(0)?;
                    self.expect(b"]")?;
                    kind = Prefix::Variable;
                }
                b":" => {
                    self.take()?;
                    self.identifier()?;
                    self.arguments()?;
                    kind = Prefix::Call;
                }
                _ if self.starts_arguments() => {
                    self.arguments()?;
                    kind = Prefix::Call;
                }
                _ => return Ok(kind),
            }
        }
    }

    fn starts_arguments(&self) -> bool {
        matches!(self.peek(), b"(" | b"{")
            || self.significant.get(self.pos).is_some_and(|&i| {
                self.tokens[i].kind == Kind::Literal && !self.peek().starts_with(b"`")
            })
    }

    fn arguments(&mut self) -> Result<()> {
        if self.accept(b"(") {
            if !self.accept(b")") {
                self.expression_list()?;
                self.expect(b")")?;
            }
        } else if self.peek() == b"{" {
            self.table()?;
        } else if self.starts_arguments() {
            self.take()?;
        } else {
            return Err(self.error());
        }
        Ok(())
    }

    fn table(&mut self) -> Result<()> {
        self.expect(b"{")?;
        while !self.accept(b"}") {
            if self.accept(b"[") {
                self.expression(0)?;
                self.expect(b"]")?;
                self.expect(b"=")?;
                self.expression(0)?;
            } else if self.at(1) == b"=" {
                self.identifier()?;
                self.expect(b"=")?;
                self.expression(0)?;
            } else {
                self.expression(0)?;
            }
            if !self.accept(b",") && !self.accept(b";") {
                self.expect(b"}")?;
                break;
            }
        }
        Ok(())
    }

    pub(super) fn function_body(&mut self, method: bool) -> Result<()> {
        self.expect(b"(")?;
        self.push();
        // A method's receiver exists before explicit parameters, which may shadow it.
        if method {
            self.bind(b"self".to_vec(), Vec::new());
        }
        if !self.accept(b")") {
            loop {
                if self.accept(b"...") {
                    break;
                }
                let index = self.identifier()?;
                self.define(index);
                if !self.accept(b",") {
                    break;
                }
            }
            self.expect(b")")?;
        }
        self.block()?;
        self.expect(b"end")?;
        self.pop();
        Ok(())
    }
}

fn precedence(operator: &[u8]) -> Option<(u8, u8)> {
    Some(match operator {
        b"or" => (1, 2),
        b"and" => (2, 3),
        b"<" | b">" | b"<=" | b">=" | b"==" | b"~=" => (3, 4),
        b"|" => (4, 5),
        b"~" => (5, 6),
        b"&" => (6, 7),
        b"<<" | b">>" => (7, 8),
        b".." => (8, 8),
        b"+" | b"-" => (9, 10),
        b"*" | b"/" | b"//" | b"%" => (10, 11),
        b"^" => (12, 12),
        _ => return None,
    })
}

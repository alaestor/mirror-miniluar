//! Resolve lexical bindings before changing any tokens. Generated names are unique
//! across the chunk and disjoint from all source identifiers, preventing capture.
mod expressions;
mod statements;

use crate::{
    Error,
    lexer::{Kind, Token},
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

type Result<T> = std::result::Result<T, Error>;

struct Binding {
    name: Vec<u8>,
    uses: Vec<usize>,
}

struct Parser<'t, 's> {
    tokens: &'t [Token<'s>],
    significant: Vec<usize>,
    pos: usize,
    end: usize,
    depth: usize,
    scopes: Vec<HashMap<Vec<u8>, usize>>,
    bindings: Vec<Binding>,
}

pub(super) fn rename(tokens: &mut [Token<'_>], end: usize, preserve: &[&str]) -> Result<()> {
    let significant = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            matches!(
                t.kind,
                Kind::Word | Kind::Number | Kind::Literal | Kind::Operator
            )
            .then_some(i)
        })
        .collect();
    let mut parser = Parser {
        tokens,
        significant,
        pos: 0,
        end,
        depth: 0,
        scopes: vec![HashMap::new()],
        bindings: Vec::new(),
    };
    parser.block()?;
    if !parser.peek().is_empty() {
        return Err(parser.error());
    }
    let bindings = parser.bindings;
    let mut reserved: HashSet<Vec<u8>> = tokens
        .iter()
        .filter(|t| t.kind == Kind::Word)
        .map(|t| t.text.to_vec())
        .collect();
    reserved.extend([b"self".to_vec(), b"_ENV".to_vec()]);
    reserved.extend(preserve.iter().map(|name| name.as_bytes().to_vec()));
    let mut next = 0usize;
    for binding in bindings {
        if binding.name == b"self"
            || binding.name == b"_ENV"
            || preserve.iter().any(|name| name.as_bytes() == binding.name)
        {
            continue;
        }
        let replacement = loop {
            let candidate = name(next);
            next += 1;
            if !keyword(&candidate) && reserved.insert(candidate.clone()) {
                break candidate;
            }
        };
        if replacement.len() > binding.name.len() {
            continue;
        }
        for index in binding.uses {
            tokens[index].text = Cow::Owned(replacement.clone());
        }
    }
    Ok(())
}

fn name(mut index: usize) -> Vec<u8> {
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut output = Vec::new();
    loop {
        output.push(LETTERS[index % LETTERS.len()]);
        index /= LETTERS.len();
        if index == 0 {
            break;
        }
        index -= 1;
    }
    output.reverse();
    output
}

fn keyword(name: &[u8]) -> bool {
    matches!(
        name,
        b"and"
            | b"break"
            | b"do"
            | b"else"
            | b"elseif"
            | b"end"
            | b"false"
            | b"for"
            | b"function"
            | b"if"
            | b"in"
            | b"local"
            | b"nil"
            | b"not"
            | b"or"
            | b"repeat"
            | b"return"
            | b"then"
            | b"true"
            | b"until"
            | b"while"
            | b"goto"
            | b"continue"
    )
}

impl Parser<'_, '_> {
    fn at(&self, ahead: usize) -> &[u8] {
        self.significant
            .get(self.pos + ahead)
            .map_or(b"", |&i| self.tokens[i].text.as_ref())
    }
    fn peek(&self) -> &[u8] {
        self.at(0)
    }
    fn error(&self) -> Error {
        Error {
            offset: self
                .significant
                .get(self.pos)
                .map_or(self.end, |&i| self.tokens[i].offset),
            message: "unsupported or invalid syntax for local renaming",
        }
    }
    fn take(&mut self) -> Result<usize> {
        let index = *self.significant.get(self.pos).ok_or_else(|| self.error())?;
        self.pos += 1;
        Ok(index)
    }
    fn accept(&mut self, text: &[u8]) -> bool {
        if self.peek() == text {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, text: &[u8]) -> Result<()> {
        if self.accept(text) {
            Ok(())
        } else {
            Err(self.error())
        }
    }
    fn identifier(&mut self) -> Result<usize> {
        let index = *self.significant.get(self.pos).ok_or_else(|| self.error())?;
        if self.tokens[index].kind != Kind::Word
            || (keyword(self.peek()) && !matches!(self.peek(), b"goto" | b"continue"))
        {
            return Err(self.error());
        }
        self.pos += 1;
        Ok(index)
    }
    fn define(&mut self, index: usize) {
        self.bind(self.tokens[index].text.to_vec(), vec![index]);
    }
    fn bind(&mut self, name: Vec<u8>, uses: Vec<usize>) {
        self.scopes
            .last_mut()
            .expect("root scope")
            .insert(name.clone(), self.bindings.len());
        self.bindings.push(Binding { name, uses });
    }
    fn reference(&mut self, index: usize) {
        let name = self.tokens[index].text.as_ref();
        if let Some(&binding) = self.scopes.iter().rev().find_map(|scope| scope.get(name)) {
            self.bindings[binding].uses.push(index);
        }
    }
    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop(&mut self) {
        self.scopes.pop();
    }
    fn nested<T>(&mut self, parse: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.depth >= 128 {
            return Err(Error {
                message: "renaming nesting limit exceeded",
                ..self.error()
            });
        }
        self.depth += 1;
        let result = parse(self);
        self.depth -= 1;
        result
    }
    fn scoped_block(&mut self) -> Result<()> {
        self.push();
        let result = self.block();
        self.pop();
        result
    }
}

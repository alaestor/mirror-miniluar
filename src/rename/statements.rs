use super::{Parser, Result, expressions::Prefix};

impl Parser<'_, '_> {
    pub(super) fn block(&mut self) -> Result<()> {
        self.nested(|parser| {
            while !matches!(parser.peek(), b"" | b"end" | b"else" | b"elseif" | b"until") {
                parser.statement()?;
            }
            Ok(())
        })
    }

    fn statement(&mut self) -> Result<()> {
        match self.peek() {
            b";" | b"break" => {
                self.take()?;
            }
            b"continue" if !self.contextual_variable() => {
                self.take()?;
            }
            b"local" => {
                self.take()?;
                self.local()?;
            }
            b"function" => {
                self.take()?;
                let base = self.identifier()?;
                self.reference(base);
                while self.accept(b".") {
                    self.identifier()?;
                }
                let method = self.accept(b":");
                if method {
                    self.identifier()?;
                }
                self.function_body(method)?;
            }
            b"do" => {
                self.take()?;
                self.scoped_block()?;
                self.expect(b"end")?;
            }
            b"while" => {
                self.take()?;
                self.expression(0)?;
                self.expect(b"do")?;
                self.scoped_block()?;
                self.expect(b"end")?;
            }
            b"repeat" => {
                self.take()?;
                self.push();
                self.block()?;
                self.expect(b"until")?;
                // Repeat-body locals are visible in the terminating condition.
                self.expression(0)?;
                self.pop();
            }
            b"if" => self.if_statement()?,
            b"for" => self.for_statement()?,
            b"return" => {
                self.take()?;
                if !matches!(
                    self.peek(),
                    b"" | b"end" | b"else" | b"elseif" | b"until" | b";"
                ) {
                    self.expression_list()?;
                }
                self.accept(b";");
            }
            b"::" => {
                self.take()?;
                self.identifier()?;
                self.expect(b"::")?;
            }
            b"goto" if !self.contextual_variable() => {
                self.take()?;
                self.identifier()?;
            }
            _ => self.assignment_or_call()?,
        }
        Ok(())
    }

    // `continue` is contextual in Luau; `goto` is an identifier in Lua 5.1
    // and Luau. Assignment/call suffixes distinguish these uses from jumps.
    fn contextual_variable(&self) -> bool {
        matches!(
            self.at(1),
            b"=" | b","
                | b"("
                | b"."
                | b"["
                | b":"
                | b"{"
                | b"+="
                | b"-="
                | b"*="
                | b"/="
                | b"//="
                | b"%="
                | b"^="
                | b"..="
        ) || self
            .significant
            .get(self.pos + 1)
            .is_some_and(|&i| self.tokens[i].kind == crate::lexer::Kind::Literal)
    }

    fn local(&mut self) -> Result<()> {
        if self.accept(b"function") {
            let index = self.identifier()?;
            self.define(index);
            return self.function_body(false);
        }
        let mut names = Vec::new();
        loop {
            names.push(self.identifier()?);
            if self.accept(b"<") {
                if !matches!(self.peek(), b"const" | b"close") {
                    return Err(self.error());
                }
                self.take()?;
                self.expect(b">")?;
            }
            if !self.accept(b",") {
                break;
            }
        }
        // Ordinary locals enter scope only after every initializer is resolved.
        if self.accept(b"=") {
            self.expression_list()?;
        }
        for index in names {
            self.define(index);
        }
        Ok(())
    }

    fn if_statement(&mut self) -> Result<()> {
        self.expect(b"if")?;
        loop {
            self.expression(0)?;
            self.expect(b"then")?;
            self.scoped_block()?;
            if !self.accept(b"elseif") {
                break;
            }
        }
        if self.accept(b"else") {
            self.scoped_block()?;
        }
        self.expect(b"end")
    }

    fn for_statement(&mut self) -> Result<()> {
        self.expect(b"for")?;
        let mut names = vec![self.identifier()?];
        if self.accept(b"=") {
            self.expression(0)?;
            self.expect(b",")?;
            self.expression(0)?;
            if self.accept(b",") {
                self.expression(0)?;
            }
        } else {
            while self.accept(b",") {
                names.push(self.identifier()?);
            }
            self.expect(b"in")?;
            self.expression_list()?;
        }
        self.expect(b"do")?;
        self.push();
        for index in names {
            self.define(index);
        }
        self.block()?;
        self.expect(b"end")?;
        self.pop();
        Ok(())
    }

    fn assignment_or_call(&mut self) -> Result<()> {
        let first = self.prefix()?;
        if first == Prefix::Call && self.peek() != b"," && self.peek() != b"=" {
            return Ok(());
        }
        if first != Prefix::Variable {
            return Err(self.error());
        }
        if matches!(
            self.peek(),
            b"+=" | b"-=" | b"*=" | b"/=" | b"//=" | b"%=" | b"^=" | b"..="
        ) {
            self.take()?;
            return self.expression(0);
        }
        while self.accept(b",") {
            if self.prefix()? != Prefix::Variable {
                return Err(self.error());
            }
        }
        self.expect(b"=")?;
        self.expression_list()
    }
}

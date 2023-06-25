use crate::scanner::{CompileError, Scanner, Token, TokenKind};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Precedence {
    None,
    Assignment,
    // Or,
    // And,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equality,
    Comparison,
    BitwiseShift,
    Term,
    Factor,
    Unary,
    Call,
    Construct,
    Delete,
    Path,
    #[allow(dead_code)]
    Primary,
}

impl Precedence {
    fn can_assign(&self) -> bool {
        *self <= Self::Assignment
    }

    fn is_construct(&self) -> bool {
        *self == Self::Construct
    }

    fn is_delete(&self) -> bool {
        *self == Self::Delete
    }
}

impl TokenKind {
    fn precedence(&self) -> Precedence {
        match self {
            Self::Dot | Self::LeftSquareBrace => Precedence::Path,
            Self::LeftParen => Precedence::Call,
            Self::Bang | Self::Delete | Self::Tilda | Self::Throw | Self::Typeof => {
                Precedence::Unary
            }
            Self::Star | Self::Slash | Self::Percent => Precedence::Factor,
            Self::Plus | Self::Minus => Precedence::Term,
            Self::DoubleGreater | Self::TripleGreater | Self::DoubleLess => {
                Precedence::BitwiseShift
            }
            Self::Greater
            | Self::GreaterEqual
            | Self::Less
            | Self::LessEqual
            | Self::InstanceOf => Precedence::Comparison,
            Self::BangEqual | Self::DoubleEqual | Self::TripleEqual => Precedence::Equality,
            Self::Ampersand => Precedence::BitwiseAnd,
            Self::Caret => Precedence::BitwiseXor,
            Self::Bar => Precedence::BitwiseOr,
            _ => Precedence::None,
        }
    }

    fn is_assign(&self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::PlusEqual
                | Self::MinusEqual
                | Self::StarEqual
                | Self::SlashEqual
                | Self::PercentEqual
                | Self::AmpersandEqual
                | Self::BarEqual
                | Self::CaretEqual
                | Self::DoubleGreaterEqual
                | Self::TripleGreaterEqual
                | Self::DoubleLessEqual
        )
    }
}

fn property_index(name: &str) -> Option<i32> {
    match name {
        "_x" => Some(0),
        "_y" => Some(1),
        "_xscale" => Some(2),
        "_yscale" => Some(3),
        "_currentframe" => Some(4),
        "_totalframes" => Some(5),
        "_alpha" => Some(6),
        "_visible" => Some(7),
        "_width" => Some(8),
        "_height" => Some(9),
        "_rotation" => Some(10),
        "_target" => Some(11),
        "_framesloaded" => Some(12),
        "_name" => Some(13),
        "_droptarget" => Some(14),
        "_url" => Some(15),
        "_highquality" => Some(16),
        "_focusrect" => Some(17),
        "_soundbuftime" => Some(18),
        "_quality" => Some(19),
        "_xmouse" => Some(20),
        "_ymouse" => Some(21),
        _ => None,
    }
}

fn register_index(name: &str) -> Option<u8> {
    name.strip_prefix("register").and_then(|r| r.parse().ok())
}

struct CompilerState<'a> {
    scanner: Scanner<'a>,
    current: Token<'a>,
}

impl<'a> CompilerState<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            scanner: Scanner::new(source),
            current: Token::INVALID,
        }
    }
}

struct Compiler<'a, 'b> {
    state: &'b mut CompilerState<'a>,
    action_data: Vec<u8>,
}

impl<'a, 'b> Compiler<'a, 'b> {
    fn new(state: &'b mut CompilerState<'a>) -> Self {
        Self {
            state,
            action_data: Vec::new(),
        }
    }

    fn nested(
        &mut self,
        f: impl FnOnce(&mut Compiler<'a, '_>) -> Result<(), CompileError>,
    ) -> Result<Vec<u8>, CompileError> {
        let mut compiler = Compiler::new(self.state);
        f(&mut compiler)?;
        Ok(compiler.action_data)
    }

    fn write_action(&mut self, action: swf::avm1::types::Action) {
        let mut writer = swf::avm1::write::Writer::new(&mut self.action_data, 0);
        writer.write_action(&action).unwrap();
    }

    fn read_token(&mut self) -> Result<Token<'a>, CompileError> {
        let next_token = self.state.scanner.read_token()?;
        let token = std::mem::replace(&mut self.state.current, next_token);
        Ok(token)
    }

    fn peek_token(&self) -> &Token<'a> {
        &self.state.current
    }

    fn consume(&mut self, kind: TokenKind) -> Result<bool, CompileError> {
        let token = self.peek_token();
        if token.kind == kind {
            self.read_token()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token<'a>, CompileError> {
        let token = self.peek_token();
        if token.kind == kind {
            self.read_token()
        } else {
            Err(CompileError {
                message: message.to_string(),
                line: token.line,
                column: token.column,
            })
        }
    }

    fn push(&mut self, value: swf::avm1::types::Value) {
        // TODO: Use constant pool.
        let push = swf::avm1::types::Push {
            values: vec![value],
        };
        self.write_action(swf::avm1::types::Action::Push(push));
    }

    fn grouping(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.expect(TokenKind::RightParen, "Expected ')' after expression")?;
        Ok(())
    }

    fn comma_separated(
        &mut self,
        f: impl Fn(&mut Compiler<'a, 'b>) -> Result<(), CompileError>,
        terminator: TokenKind,
        arity: Option<usize>,
    ) -> Result<usize, CompileError> {
        let mut count = 0;
        let token = loop {
            let token = self.peek_token();
            if token.kind == terminator {
                break self.read_token()?;
            }

            count += 1;
            if let Some(arity) = arity {
                if count > arity {
                    return Err(CompileError {
                        message: format!("Expected {} argument(s), got {}", arity, count),
                        line: token.line,
                        column: token.column,
                    });
                }
            }

            f(self)?;

            if !self.consume(TokenKind::Comma)? {
                // TODO: Print exact character.
                break self.expect(terminator, "Expected end of values")?;
            }
        };

        if let Some(arity) = arity {
            if count < arity {
                return Err(CompileError {
                    message: format!("Expected {} argument(s), got {}", arity, count),
                    line: token.line,
                    column: token.column,
                });
            }
        }

        Ok(count)
    }

    fn comma_separated_rev(
        &mut self,
        f: impl Fn(&mut Compiler<'a, '_>) -> Result<(), CompileError>,
        terminator: TokenKind,
    ) -> Result<usize, CompileError> {
        let mut values = Vec::new();
        loop {
            let token = self.peek_token();
            if token.kind == terminator {
                self.read_token()?;
                break;
            }

            values.push(self.nested(&f)?);

            if !self.consume(TokenKind::Comma)? {
                // TODO: Print exact character.
                self.expect(terminator, "Expected end of values")?;
                break;
            }
        }

        let count = values.len();
        for value_data in values.into_iter().rev() {
            self.action_data.extend(value_data);
        }
        Ok(count)
    }

    fn array(&mut self) -> Result<(), CompileError> {
        let count = self.comma_separated_rev(|c| c.expression(), TokenKind::RightSquareBrace)?;
        self.push(swf::avm1::types::Value::Int(count.try_into().unwrap()));
        self.write_action(swf::avm1::types::Action::InitArray);
        Ok(())
    }

    fn object(&mut self) -> Result<(), CompileError> {
        let count = self.comma_separated(
            |c| {
                let name = c.expect(TokenKind::Identifier, "Expected property name")?;
                c.push(swf::avm1::types::Value::Str(name.source.into()));
                c.expect(TokenKind::Colon, "Expected ':' after property name")?;
                c.expression()
            },
            TokenKind::RightBrace,
            None,
        )?;
        self.push(swf::avm1::types::Value::Int(count.try_into().unwrap()));
        self.write_action(swf::avm1::types::Action::InitObject);
        Ok(())
    }

    fn access(
        &mut self,
        push: impl Fn(&mut Self),
        duplicate: impl Fn(&mut Self),
        get: impl Fn(&mut Self),
        set: impl Fn(&mut Self),
        can_assign: bool,
    ) -> Result<(), CompileError> {
        if can_assign && self.peek_token().kind.is_assign() {
            let token = self.read_token()?;
            if token.kind == TokenKind::Equal {
                push(self);
            } else {
                duplicate(self);
                push(self);
                get(self);
            }
            self.expression()?;
            match token.kind {
                TokenKind::Equal => {}
                TokenKind::PlusEqual => self.write_action(swf::avm1::types::Action::Add2),
                TokenKind::MinusEqual => self.write_action(swf::avm1::types::Action::Subtract),
                TokenKind::StarEqual => self.write_action(swf::avm1::types::Action::Multiply),
                TokenKind::SlashEqual => self.write_action(swf::avm1::types::Action::Divide),
                TokenKind::PercentEqual => self.write_action(swf::avm1::types::Action::Modulo),
                TokenKind::AmpersandEqual => self.write_action(swf::avm1::types::Action::BitAnd),
                TokenKind::BarEqual => self.write_action(swf::avm1::types::Action::BitOr),
                TokenKind::CaretEqual => self.write_action(swf::avm1::types::Action::BitXor),
                TokenKind::DoubleGreaterEqual => {
                    self.write_action(swf::avm1::types::Action::BitRShift)
                }
                TokenKind::TripleGreaterEqual => {
                    self.write_action(swf::avm1::types::Action::BitURShift)
                }
                TokenKind::DoubleLessEqual => {
                    self.write_action(swf::avm1::types::Action::BitLShift)
                }
                _ => unreachable!(),
            }
            set(self);
        } else if self.consume(TokenKind::DoublePlus)? {
            duplicate(self);
            push(self);
            get(self);
            self.write_action(swf::avm1::types::Action::Increment);
            set(self);
        } else if self.consume(TokenKind::DoubleMinus)? {
            duplicate(self);
            push(self);
            get(self);
            self.write_action(swf::avm1::types::Action::Decrement);
            set(self);
        } else {
            push(self);
            get(self);
        }
        Ok(())
    }

    fn variable_access(&mut self, name: &str, precedence: Precedence) -> Result<(), CompileError> {
        let register = register_index(name);

        if self.consume(TokenKind::LeftParen)? {
            if register.is_some() {
                // TODO: Tell exact location.
                let token = self.peek_token();
                return Err(CompileError {
                    message: "Cannot call register".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }

            let count = self.comma_separated_rev(|c| c.expression(), TokenKind::RightParen)?;
            self.push(swf::avm1::types::Value::Int(count.try_into().unwrap()));

            self.push(swf::avm1::types::Value::Str(name.into()));

            if precedence.is_construct() {
                self.write_action(swf::avm1::types::Action::NewObject);
            } else {
                self.write_action(swf::avm1::types::Action::CallFunction);
            }
        } else if precedence.is_delete() && self.peek_token().kind.precedence() < Precedence::Call {
            if register.is_some() {
                // TODO: Tell exact location.
                let token = self.peek_token();
                return Err(CompileError {
                    message: "Cannot delete register".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }

            self.push(swf::avm1::types::Value::Str(name.into()));
            self.write_action(swf::avm1::types::Action::Delete2);
        } else {
            let push = |this: &mut Self| match register {
                Some(_) => {}
                None => this.push(swf::avm1::types::Value::Str(name.into())),
            };
            let duplicate = push;
            let get = |this: &mut Self| match register {
                Some(register) => this.push(swf::avm1::types::Value::Register(register)),
                None => this.write_action(swf::avm1::types::Action::GetVariable),
            };
            let set = |this: &mut Self| match register {
                Some(register) => this.write_action(swf::avm1::types::Action::StoreRegister(
                    swf::avm1::types::StoreRegister { register },
                )),
                None => this.write_action(swf::avm1::types::Action::SetVariable),
            };
            self.access(push, duplicate, get, set, precedence.can_assign())?;
        }

        Ok(())
    }

    fn dot(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        let name = self.expect(TokenKind::Identifier, "Expected name")?;

        if self.consume(TokenKind::LeftParen)? {
            // TODO: Error when calling a property?
            let count = self.comma_separated_rev(|c| c.expression(), TokenKind::RightParen)?;
            self.push(swf::avm1::types::Value::Int(count.try_into().unwrap()));
            self.write_action(swf::avm1::types::Action::StackSwap);

            self.push(swf::avm1::types::Value::Str(name.source.into()));

            if precedence.is_construct() {
                self.write_action(swf::avm1::types::Action::NewMethod);
            } else {
                self.write_action(swf::avm1::types::Action::CallMethod);
            }
        } else if precedence.is_delete() && self.peek_token().kind.precedence() < Precedence::Call {
            // TODO: Error when deleting a property?
            self.push(swf::avm1::types::Value::Str(name.source.into()));
            self.write_action(swf::avm1::types::Action::Delete);
        } else {
            let property = property_index(name.source);
            let push = |this: &mut Self| match property {
                Some(property) => this.push(swf::avm1::types::Value::Int(property)),
                None => this.push(swf::avm1::types::Value::Str(name.source.into())),
            };
            let duplicate = |this: &mut Self| {
                this.write_action(swf::avm1::types::Action::PushDuplicate);
                push(this);
                this.write_action(swf::avm1::types::Action::StackSwap);
            };
            let get = |this: &mut Self| match property {
                Some(_) => this.write_action(swf::avm1::types::Action::GetProperty),
                None => this.write_action(swf::avm1::types::Action::GetMember),
            };
            let set = |this: &mut Self| match property {
                Some(_) => this.write_action(swf::avm1::types::Action::SetProperty),
                None => this.write_action(swf::avm1::types::Action::SetMember),
            };
            self.access(push, duplicate, get, set, precedence.can_assign())?;
        }

        Ok(())
    }

    fn member_access(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        let name = self.nested(|c| c.expression())?;
        self.expect(TokenKind::RightSquareBrace, "Expected ']'")?;

        if self.consume(TokenKind::LeftParen)? {
            let count = self.comma_separated_rev(|c| c.expression(), TokenKind::RightParen)?;
            self.push(swf::avm1::types::Value::Int(count.try_into().unwrap()));

            self.write_action(swf::avm1::types::Action::StackSwap);
            self.action_data.extend(name);

            if precedence.is_construct() {
                self.write_action(swf::avm1::types::Action::NewMethod);
            } else {
                self.write_action(swf::avm1::types::Action::CallMethod);
            }
        } else if precedence.is_delete() && self.peek_token().kind.precedence() < Precedence::Call {
            self.action_data.extend(name);
            self.write_action(swf::avm1::types::Action::Delete);
        } else {
            // TODO: Fix.
            self.action_data.extend(name);
            let push = |_this: &mut Self| {};
            let duplicate = |this: &mut Self| {
                this.write_action(swf::avm1::types::Action::StackSwap);
                this.write_action(swf::avm1::types::Action::PushDuplicate);
                this.write_action(swf::avm1::types::Action::StackSwap);
            };
            let get = |this: &mut Self| this.write_action(swf::avm1::types::Action::GetMember);
            let set = |this: &mut Self| this.write_action(swf::avm1::types::Action::SetMember);
            self.access(push, duplicate, get, set, precedence.can_assign())?;
        }

        Ok(())
    }

    fn construct(&mut self) -> Result<(), CompileError> {
        self.expression_with_precedence(Precedence::Construct)
    }

    fn delete(&mut self) -> Result<(), CompileError> {
        self.expression_with_precedence(Precedence::Delete)
    }

    fn unary(&mut self, token_kind: TokenKind) -> Result<(), CompileError> {
        match token_kind {
            TokenKind::Minus => self.push(swf::avm1::types::Value::Int(0)),
            TokenKind::Tilda => self.push(swf::avm1::types::Value::Double(u32::MAX.into())),
            _ => {}
        }

        self.expression_with_precedence(Precedence::Unary)?;

        match token_kind {
            TokenKind::Plus => self.write_action(swf::avm1::types::Action::ToNumber),
            TokenKind::Minus => self.write_action(swf::avm1::types::Action::Subtract),
            TokenKind::Tilda => self.write_action(swf::avm1::types::Action::BitXor),
            TokenKind::Bang => self.write_action(swf::avm1::types::Action::Not),
            TokenKind::Throw => self.write_action(swf::avm1::types::Action::Throw),
            TokenKind::Typeof => self.write_action(swf::avm1::types::Action::TypeOf),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn prefix(&mut self, token_kind: TokenKind) -> Result<(), CompileError> {
        let variable = self.expect(TokenKind::Identifier, "Expected variable")?;
        let register = register_index(variable.source);

        if let Some(register) = register {
            self.push(swf::avm1::types::Value::Register(register));
        } else {
            self.push(swf::avm1::types::Value::Str(variable.source.into()));
            self.push(swf::avm1::types::Value::Str(variable.source.into()));
            self.write_action(swf::avm1::types::Action::GetVariable);
        }

        match token_kind {
            TokenKind::DoublePlus => self.write_action(swf::avm1::types::Action::Increment),
            TokenKind::DoubleMinus => self.write_action(swf::avm1::types::Action::Decrement),
            _ => unreachable!(),
        }

        if let Some(register) = register {
            self.write_action(swf::avm1::types::Action::StoreRegister(
                swf::avm1::types::StoreRegister { register },
            ));
        } else {
            self.write_action(swf::avm1::types::Action::SetVariable);
        }

        Ok(())
    }

    fn binary(&mut self, token: Token) -> Result<(), CompileError> {
        let next_precedence = match token.kind.precedence() {
            Precedence::None
            | Precedence::Construct
            | Precedence::Delete
            | Precedence::Path
            | Precedence::Primary => unreachable!(),
            Precedence::Assignment => Precedence::BitwiseOr,
            Precedence::BitwiseOr => Precedence::BitwiseXor,
            Precedence::BitwiseXor => Precedence::BitwiseAnd,
            Precedence::BitwiseAnd => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::BitwiseShift,
            Precedence::BitwiseShift => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary | Precedence::Call => {
                return Err(CompileError {
                    message: "Expected binary operator".to_string(),
                    line: token.line,
                    column: token.column,
                })
            }
        };
        self.expression_with_precedence(next_precedence)?;

        match token.kind {
            TokenKind::Ampersand => self.write_action(swf::avm1::types::Action::BitAnd),
            TokenKind::Bar => self.write_action(swf::avm1::types::Action::BitOr),
            TokenKind::Caret => self.write_action(swf::avm1::types::Action::BitXor),
            TokenKind::Percent => self.write_action(swf::avm1::types::Action::Modulo),
            TokenKind::Plus => self.write_action(swf::avm1::types::Action::Add2),
            TokenKind::Minus => self.write_action(swf::avm1::types::Action::Subtract),
            TokenKind::Slash => self.write_action(swf::avm1::types::Action::Divide),
            TokenKind::Star => self.write_action(swf::avm1::types::Action::Multiply),
            TokenKind::DoubleEqual => self.write_action(swf::avm1::types::Action::Equals2),
            TokenKind::TripleEqual => self.write_action(swf::avm1::types::Action::StrictEquals),
            TokenKind::Greater => self.write_action(swf::avm1::types::Action::Greater),
            TokenKind::DoubleGreater => self.write_action(swf::avm1::types::Action::BitRShift),
            TokenKind::TripleGreater => self.write_action(swf::avm1::types::Action::BitURShift),
            TokenKind::GreaterEqual => {
                self.write_action(swf::avm1::types::Action::Less);
                self.write_action(swf::avm1::types::Action::Not);
            }
            TokenKind::Less => self.write_action(swf::avm1::types::Action::Less),
            TokenKind::DoubleLess => self.write_action(swf::avm1::types::Action::BitLShift),
            TokenKind::LessEqual => {
                self.write_action(swf::avm1::types::Action::Greater);
                self.write_action(swf::avm1::types::Action::Not);
            }
            TokenKind::InstanceOf => self.write_action(swf::avm1::types::Action::InstanceOf),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn builtin(
        &mut self,
        action: swf::avm1::types::Action,
        arity: usize,
    ) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftParen, "Expected '('")?;
        self.comma_separated(|c| c.expression(), TokenKind::RightParen, Some(arity))?;
        self.write_action(action);
        Ok(())
    }

    fn expression_with_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        let token = self.read_token()?;
        match token.kind {
            TokenKind::LeftParen => self.grouping()?,
            TokenKind::LeftSquareBrace => self.array()?,
            TokenKind::LeftBrace => self.object()?,
            TokenKind::New => self.construct()?,
            TokenKind::Delete => self.delete()?,
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Tilda
            | TokenKind::Bang
            | TokenKind::Throw
            | TokenKind::Typeof => self.unary(token.kind)?,
            TokenKind::DoublePlus | TokenKind::DoubleMinus => self.prefix(token.kind)?,
            TokenKind::Number => {
                let integer = token.source.parse().unwrap();
                self.push(swf::avm1::types::Value::Int(integer));
            }
            TokenKind::String => {
                let string = &token.source[1..token.source.len() - 1];
                self.push(swf::avm1::types::Value::Str(string.into()));
            }
            TokenKind::False => self.push(swf::avm1::types::Value::Bool(false)),
            TokenKind::Null => self.push(swf::avm1::types::Value::Null),
            TokenKind::True => self.push(swf::avm1::types::Value::Bool(true)),
            TokenKind::Undefined => self.push(swf::avm1::types::Value::Undefined),
            TokenKind::Function => self.function_expression()?,
            TokenKind::Identifier => match token.source {
                "call" => self.builtin(swf::avm1::types::Action::Call, 1)?,
                "duplicateMovieClip" => self.builtin(swf::avm1::types::Action::CloneSprite, 3)?,
                "chr" => self.builtin(swf::avm1::types::Action::AsciiToChar, 1)?,
                "eval" => self.builtin(swf::avm1::types::Action::GetVariable, 1)?,
                "getTimer" => self.builtin(swf::avm1::types::Action::GetTime, 0)?,
                "int" => self.builtin(swf::avm1::types::Action::ToInteger, 1)?,
                "length" => self.builtin(swf::avm1::types::Action::StringLength, 1)?,
                "mbchr" => self.builtin(swf::avm1::types::Action::MBAsciiToChar, 1)?,
                "mblength" => self.builtin(swf::avm1::types::Action::MBStringLength, 1)?,
                "mbord" => self.builtin(swf::avm1::types::Action::MBCharToAscii, 1)?,
                "mbsubstring" => self.builtin(swf::avm1::types::Action::MBStringExtract, 3)?,
                "nextFrame" => self.builtin(swf::avm1::types::Action::NextFrame, 0)?,
                "ord" => self.builtin(swf::avm1::types::Action::CharToAscii, 1)?,
                "play" => self.builtin(swf::avm1::types::Action::Play, 0)?,
                "prevFrame" => self.builtin(swf::avm1::types::Action::PreviousFrame, 0)?,
                "random" => self.builtin(swf::avm1::types::Action::RandomNumber, 1)?,
                "stop" => self.builtin(swf::avm1::types::Action::Stop, 0)?,
                "stopAllSounds" => self.builtin(swf::avm1::types::Action::StopSounds, 0)?,
                "stopDrag" => self.builtin(swf::avm1::types::Action::EndDrag, 0)?,
                "targetPath" => self.builtin(swf::avm1::types::Action::TargetPath, 1)?,
                "toggleHighQuality" => self.builtin(swf::avm1::types::Action::ToggleQuality, 0)?,
                variable_name => self.variable_access(variable_name, precedence)?,
            },
            TokenKind::Eof => {
                return Err(CompileError {
                    message: "Unexpected end of file".to_string(),
                    line: token.line,
                    column: token.column,
                })
            }
            _ => {
                return Err(CompileError {
                    message: format!("Unexpected token: \"{}\"", token.source),
                    line: token.line,
                    column: token.column,
                })
            }
        }

        while self.peek_token().kind.precedence() >= precedence {
            let token = self.read_token()?;
            match token.kind {
                TokenKind::Dot => self.dot(precedence)?,
                TokenKind::LeftSquareBrace => self.member_access(precedence)?,
                _ => self.binary(token)?,
            }
        }

        if precedence.can_assign() {
            let token = self.peek_token();
            if token.kind == TokenKind::Equal {
                return Err(CompileError {
                    message: "Invalid assignment target".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }
        }

        if precedence.is_construct() {
            let token = self.peek_token();
            println!("{:?}", token);
            if token.kind.precedence() < Precedence::Construct
                && token.kind.precedence() != Precedence::None
            {
                return Err(CompileError {
                    message: "Invalid construct target".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }
        }

        if precedence.is_delete() {
            let token = self.peek_token();
            if token.kind.precedence() < Precedence::Call
                && token.kind.precedence() != Precedence::None
            {
                return Err(CompileError {
                    message: "Invalid delete target".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }
        }

        Ok(())
    }

    fn expression(&mut self) -> Result<(), CompileError> {
        self.expression_with_precedence(Precedence::Assignment)
    }

    fn trace_statement(&mut self) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftParen, "Expected '(' before expression")?;
        self.expression()?;
        self.expect(TokenKind::RightParen, "Expected ')' after expression")?;
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;
        self.write_action(swf::avm1::types::Action::Trace);
        Ok(())
    }

    fn variable_declaration(&mut self) -> Result<(), CompileError> {
        let variable = self.expect(TokenKind::Identifier, "Expected variable name")?;
        self.push(swf::avm1::types::Value::Str(variable.source.into()));
        if self.consume(TokenKind::Equal)? {
            self.expression()?;
            self.write_action(swf::avm1::types::Action::DefineLocal);
        } else {
            self.write_action(swf::avm1::types::Action::DefineLocal2);
        }
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;
        Ok(())
    }

    fn function_body(&mut self, name: &str) -> Result<(), CompileError> {
        let mut params = Vec::new();
        self.expect(TokenKind::LeftParen, "Expected '('")?;
        loop {
            if self.consume(TokenKind::RightParen)? {
                break;
            }
            let parameter = self.expect(TokenKind::Identifier, "Expected parameter name")?;
            params.push(parameter.source.into());
            if !self.consume(TokenKind::Comma)? {
                self.expect(TokenKind::RightParen, "Expected ')'")?;
                break;
            }
        }

        self.expect(TokenKind::LeftBrace, "Expected '{'")?;
        let actions = self.nested(|c| c.block_statement())?;
        self.write_action(swf::avm1::types::Action::DefineFunction(
            swf::avm1::types::DefineFunction {
                name: name.into(),
                params,
                actions: &actions,
            },
        ));
        Ok(())
    }

    fn function_declaration(&mut self) -> Result<(), CompileError> {
        let name = self.expect(TokenKind::Identifier, "Expected function name")?;
        self.function_body(name.source)
    }

    fn function_expression(&mut self) -> Result<(), CompileError> {
        let token = self.peek_token();
        if token.kind == TokenKind::Identifier {
            return Err(CompileError {
                message: "Function expression must be anonymous".to_string(),
                line: token.line,
                column: token.column,
            });
        }
        self.function_body("")
    }

    fn expression_statement(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;
        self.write_action(swf::avm1::types::Action::Pop);
        Ok(())
    }

    fn block_statement(&mut self) -> Result<(), CompileError> {
        while !matches!(
            self.peek_token().kind,
            TokenKind::RightBrace | TokenKind::Eof
        ) {
            self.declaration()?;
        }

        self.expect(TokenKind::RightBrace, "Expected '}' after block")?;
        Ok(())
    }

    fn if_statement(&mut self) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftParen, "Expected '(' after if")?;
        self.expression()?;
        self.expect(TokenKind::RightParen, "Expected ')' after condition")?;
        self.write_action(swf::avm1::types::Action::Not);

        let if_body = self.nested(|c| c.statement())?;
        self.write_action(swf::avm1::types::Action::If(swf::avm1::types::If {
            offset: if_body.len().try_into().unwrap(),
        }));
        self.action_data.extend(if_body);

        if self.consume(TokenKind::Else)? {
            let else_body = self.nested(|c| c.statement())?;
            self.write_action(swf::avm1::types::Action::Jump(swf::avm1::types::Jump {
                offset: else_body.len().try_into().unwrap(),
            }));
            self.action_data.extend(else_body);
        }

        Ok(())
    }

    fn while_statement(&mut self) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftParen, "Expected '(' after while")?;
        let condition = self.nested(|c| c.expression())?;
        self.expect(TokenKind::RightParen, "Expected ')' after condition")?;

        let body = self.nested(|c| c.statement())?;
        const JUMP_SIZE: usize = 5;
        let offset = body.len() + JUMP_SIZE * 2;

        self.write_action(swf::avm1::types::Action::Not);
        self.action_data.extend(&condition);
        self.write_action(swf::avm1::types::Action::If(swf::avm1::types::If {
            offset: offset.try_into().unwrap(),
        }));
        self.action_data.extend(body);
        self.write_action(swf::avm1::types::Action::Jump(swf::avm1::types::Jump {
            offset: -i16::try_from(condition.len() + offset).unwrap(),
        }));

        Ok(())
    }

    fn try_statement(&mut self) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftBrace, "Expected '{'")?;
        let try_body = self.nested(|c| c.block_statement())?;

        let catch_body = if self.consume(TokenKind::Catch)? {
            self.expect(TokenKind::LeftParen, "Expected '('")?;
            let catch_var = self.expect(TokenKind::Identifier, "Expected catch variable")?;
            self.expect(TokenKind::RightParen, "Expected ')'")?;

            self.expect(TokenKind::LeftBrace, "Expected '{'")?;
            let catch_body = self.nested(|c| c.block_statement())?;

            Some((catch_var, catch_body))
        } else {
            None
        };

        let finally_body = if self.consume(TokenKind::Finally)? {
            self.expect(TokenKind::LeftBrace, "Expected '{'")?;
            Some(self.nested(|c| c.block_statement())?)
        } else {
            None
        };

        // TODO: Validate existence of catch/finally?

        self.write_action(swf::avm1::types::Action::Try(swf::avm1::types::Try {
            try_body: &try_body,
            catch_body: catch_body.as_ref().map(|(catch_var, catch_body)| {
                let catch_var = if let Some(register) = register_index(catch_var.source) {
                    swf::avm1::types::CatchVar::Register(register)
                } else {
                    swf::avm1::types::CatchVar::Var(catch_var.source.into())
                };
                (catch_var, catch_body.as_ref())
            }),
            finally_body: finally_body.as_deref(),
        }));
        Ok(())
    }

    fn statement(&mut self) -> Result<(), CompileError> {
        if self.consume(TokenKind::LeftBrace)? {
            self.block_statement()
        } else if self.consume(TokenKind::If)? {
            self.if_statement()
        } else if self.consume(TokenKind::While)? {
            self.while_statement()
        } else if self.consume(TokenKind::Try)? {
            self.try_statement()
        } else if self.consume(TokenKind::Trace)? {
            self.trace_statement()
        } else {
            self.expression_statement()
        }
    }

    fn declaration(&mut self) -> Result<(), CompileError> {
        if self.consume(TokenKind::Var)? {
            self.variable_declaration()
        } else if self.consume(TokenKind::Function)? {
            self.function_declaration()
        } else {
            self.statement()
        }
    }

    fn compile(&mut self) -> Result<(), CompileError> {
        // Initialize `self.current`.
        self.read_token()?;

        while self.peek_token().kind != TokenKind::Eof {
            self.declaration()?;
        }

        Ok(())
    }
}

pub fn compile<W: std::io::Write>(source: &str, output: W) -> Result<(), CompileError> {
    let mut state = CompilerState::new(source);
    let mut compiler = Compiler::new(&mut state);
    compiler.compile()?;

    const SWF_VERSION: u8 = 32;
    let header = swf::Header {
        compression: swf::Compression::None,
        version: SWF_VERSION,
        stage_size: swf::Rectangle {
            x_min: swf::Twips::new(0),
            x_max: swf::Twips::new(100),
            y_min: swf::Twips::new(0),
            y_max: swf::Twips::new(100),
        },
        frame_rate: swf::Fixed8::ONE,
        num_frames: 0,
    };
    let tags = vec![
        swf::Tag::FileAttributes(swf::FileAttributes::empty()),
        swf::Tag::SetBackgroundColor(swf::Color::from_rgb(0xeeeeee, 255)),
        swf::Tag::DoAction(&compiler.action_data),
        swf::Tag::ShowFrame,
    ];
    swf::write_swf(&header, &tags, output).unwrap();
    Ok(())
}

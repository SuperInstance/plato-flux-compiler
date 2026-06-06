//! Parser for Plato alarm condition strings into the Condition AST.
//!
//! Supported syntax:
//! - Simple comparison: `sensor op value` (e.g., `temp > 95`)
//! - Logical: `cond AND cond`, `cond OR cond`, `NOT cond`
//! - Range: `sensor in [low, high]`
//! - Threshold: `sensor rising_by delta in N_ticks`
//! - Parenthesized grouping

use crate::ast::{CmpOp, Comparison, Condition, RangeCheck, ThresholdCheck};

/// Parse error with a description of what went wrong.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Token types for the condition lexer.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Number(i64),
    Op(String),    // comparison operators
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    And,
    Or,
    Not,
    In,
    RisingBy,
    UnderscoreTicks,
}

/// Simple tokenizer for condition strings.
struct Lexer {
    tokens: Vec<Token>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Result<Self, ParseError> {
        let mut tokens = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(&ch) = chars.peek() {
            match ch {
                ' ' | '\t' | '\n' | '\r' => {
                    chars.next();
                }
                '(' => { chars.next(); tokens.push(Token::LParen); }
                ')' => { chars.next(); tokens.push(Token::RParen); }
                '[' => { chars.next(); tokens.push(Token::LBracket); }
                ']' => { chars.next(); tokens.push(Token::RBracket); }
                ',' => { chars.next(); tokens.push(Token::Comma); }
                '=' | '!' | '<' | '>' => {
                    let mut op = String::new();
                    op.push(chars.next().unwrap());
                    if chars.peek() == Some(&'=') {
                        op.push(chars.next().unwrap());
                    }
                    tokens.push(Token::Op(op));
                }
                '_' => {
                    // Handle _ticks suffix
                    let mut word = String::new();
                    word.push(chars.next().unwrap());
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            word.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if word == "_ticks" {
                        tokens.push(Token::UnderscoreTicks);
                    } else {
                        return Err(ParseError { message: format!("unexpected token: {}", word) });
                    }
                }
                c if c.is_ascii_digit() || (c == '-' && tokens.last().map_or(true, |t| matches!(t, Token::Op(_) | Token::Comma | Token::LBracket))) => {
                    let mut num = String::new();
                    if c == '-' {
                        num.push(chars.next().unwrap());
                    }
                    while let Some(&d) = chars.peek() {
                        if d.is_ascii_digit() {
                            num.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let val: i64 = num.parse().map_err(|_| ParseError {
                        message: format!("invalid number: {}", num),
                    })?;
                    tokens.push(Token::Number(val));
                }
                c if c.is_alphabetic() => {
                    let mut word = String::new();
                    word.push(chars.next().unwrap());
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            word.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    match word.to_uppercase().as_str() {
                        "AND" => tokens.push(Token::And),
                        "OR" => tokens.push(Token::Or),
                        "NOT" => tokens.push(Token::Not),
                        "IN" => tokens.push(Token::In),
                        "RISING_BY" => tokens.push(Token::RisingBy),
                        _ => tokens.push(Token::Ident(word)), // preserve original case
                    }
                }
                _ => {
                    return Err(ParseError {
                        message: format!("unexpected character: '{}'", ch),
                    });
                }
            }
        }

        Ok(Lexer { tokens, pos: 0 })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        match self.next() {
            Some(tok) if tok == expected => Ok(()),
            Some(tok) => Err(ParseError {
                message: format!("expected {:?}, got {:?}", expected, tok),
            }),
            None => Err(ParseError {
                message: format!("expected {:?}, got end of input", expected),
            }),
        }
    }
}

/// Recursive-descent parser for alarm conditions.
///
/// Precedence (lowest to highest):
/// 1. OR
/// 2. AND
/// 3. NOT
/// 4. Comparison / Range / Threshold (atoms)
pub struct Parser<'a> {
    lexer: &'a mut Lexer,
}

impl<'a> Parser<'a> {
    #[allow(dead_code)]
    pub(crate) fn new(lexer: &'a mut Lexer) -> Self {
        Parser { lexer }
    }

    /// Parse a complete condition string.
    pub fn parse(input: &str) -> Result<Condition, ParseError> {
        let mut lexer = Lexer::new(input)?;
        let mut parser = Parser::new(&mut lexer);
        let cond = parser.parse_or()?;
        if parser.lexer.peek().is_some() {
            return Err(ParseError {
                message: format!("unexpected trailing token: {:?}", parser.lexer.peek()),
            });
        }
        Ok(cond)
    }

    fn parse_or(&mut self) -> Result<Condition, ParseError> {
        let mut left = self.parse_and()?;
        while self.lexer.peek() == Some(&Token::Or) {
            self.lexer.next();
            let right = self.parse_and()?;
            left = Condition::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Condition, ParseError> {
        let mut left = self.parse_not()?;
        while self.lexer.peek() == Some(&Token::And) {
            self.lexer.next();
            let right = self.parse_not()?;
            left = Condition::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Condition, ParseError> {
        if self.lexer.peek() == Some(&Token::Not) {
            self.lexer.next();
            let inner = self.parse_not()?;
            Ok(Condition::Not(Box::new(inner)))
        } else {
            self.parse_atom()
        }
    }

    fn parse_atom(&mut self) -> Result<Condition, ParseError> {
        // Parenthesized group
        if self.lexer.peek() == Some(&Token::LParen) {
            self.lexer.next();
            let inner = self.parse_or()?;
            self.lexer.expect(&Token::RParen)?;
            // Check for trailing AND/OR
            return self.parse_trailing_logical(inner);
        }

        // Must start with identifier (sensor name)
        let ident = match self.lexer.next() {
            Some(Token::Ident(s)) => s.clone(),
            other => return Err(ParseError {
                message: format!("expected identifier, got {:?}", other),
            }),
        };

        match self.lexer.peek() {
            // Range check: sensor in [low, high]
            Some(Token::In) => {
                self.lexer.next();
                self.lexer.expect(&Token::LBracket)?;
                let low = self.parse_number()?;
                self.lexer.expect(&Token::Comma)?;
                let high = self.parse_number()?;
                self.lexer.expect(&Token::RBracket)?;
                let cond = Condition::Range(RangeCheck {
                    sensor: ident,
                    low,
                    high,
                });
                self.parse_trailing_logical(cond)
            }
            // Threshold: sensor rising_by delta in N_ticks
            Some(Token::RisingBy) => {
                self.lexer.next();
                let delta = self.parse_number()?;
                // expect "in"
                match self.lexer.next() {
                    Some(Token::In) => {}
                    other => return Err(ParseError {
                        message: format!("expected 'in', got {:?}", other),
                    }),
                }
                let ticks = match self.lexer.next() {
                    Some(Token::Number(n)) => *n as u64,
                    other => return Err(ParseError {
                        message: format!("expected number for ticks, got {:?}", other),
                    }),
                };
                self.lexer.expect(&Token::UnderscoreTicks)?;
                let cond = Condition::Threshold(ThresholdCheck {
                    sensor: ident,
                    delta,
                    ticks,
                });
                self.parse_trailing_logical(cond)
            }
            // Comparison: sensor op value
            Some(Token::Op(_)) => {
                let op_str = match self.lexer.next() {
                    Some(Token::Op(s)) => s.clone(),
                    _ => unreachable!(),
                };
                let op = CmpOp::from_str(&op_str).ok_or_else(|| ParseError {
                    message: format!("unknown operator: {}", op_str),
                })?;
                let value = self.parse_number()?;
                let cond = Condition::Comparison(Comparison {
                    sensor: ident,
                    op,
                    value,
                });
                self.parse_trailing_logical(cond)
            }
            other => Err(ParseError {
                message: format!("expected operator, 'in', or 'rising_by', got {:?}", other),
            }),
        }
    }

    /// After parsing an atom, just return it. AND/OR are handled by
    /// parse_and/parse_or which call parse_atom.
    fn parse_trailing_logical(&mut self, left: Condition) -> Result<Condition, ParseError> {
        Ok(left)
    }

    fn parse_number(&mut self) -> Result<i64, ParseError> {
        match self.lexer.next() {
            Some(Token::Number(n)) => Ok(*n),
            other => Err(ParseError {
                message: format!("expected number, got {:?}", other),
            }),
        }
    }
}

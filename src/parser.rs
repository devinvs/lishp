use std::fmt::Display;
use rustyline::validate::{ValidationContext, ValidationResult};
use rustyline::{Completer, Helper, Highlighter, Hinter, Validator};

use std::fs::File;
use std::io::Read;

#[derive(Debug, PartialEq)]
enum Token {
    LParen,
    RParen,
    Quote,
    Ident(String)
}

fn lex(mut s: impl Iterator<Item = char>) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut stack = String::with_capacity(10);
    let mut in_quote = false;

    let push = |s: &mut String, toks: &mut Vec<Token>, in_quote: bool| {
        if s.is_empty() && !in_quote { return; }

        toks.push(Token::Ident(s.clone()));
        s.clear();
    };

    while let Some(c) = s.next() {
        match c {
            a if a.is_whitespace() && !in_quote => {
                push(&mut stack, &mut tokens, in_quote);
            }
            '\"' => {
                push(&mut stack, &mut tokens, in_quote);
                in_quote = !in_quote;
            }
            c if in_quote => stack.push(c),
            '\'' => {
                push(&mut stack, &mut tokens, in_quote);
                tokens.push(Token::Quote);
            }
            '(' => {
                push(&mut stack, &mut tokens, in_quote);
                tokens.push(Token::LParen);
            }
            ')' => {
                push(&mut stack, &mut tokens, in_quote);
                tokens.push(Token::RParen);
            }
            _ => stack.push(c)
        }
    }
    push(&mut stack, &mut tokens, in_quote);

    // In order to preserve somewhat normal behavior of the shell,
    // We automatically surround the input in a list if it is not alread a list

    if tokens[0] != Token::LParen || tokens[tokens.len()-1] != Token::RParen {
        tokens.insert(0, Token::LParen);
        tokens.push(Token::RParen);
    }

    tokens
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExpression {
    Call(Vec<SExpression>),
    List(Vec<SExpression>),
    Atom(String)
}

impl SExpression {
    pub fn parse(s: &str) -> Result<Self, String> {
        let toks = lex(s.chars());
        Self::parse_toks(&mut toks.into_iter().peekable())
    }

    fn parse_toks<I>(t: &mut std::iter::Peekable<I>) -> Result<Self, String>
    where I: Iterator<Item = Token> + std::fmt::Debug {
        match t.next() {
            None => Err("Unexpected EOF".into()),
            Some(Token::Quote) => {
                let next = Self::parse_toks(t)?;

                Ok(if let Self::Call(es) = next {
                    Self::List(es)
                } else {
                    next
                })
            }
            Some(Token::LParen) => {
                let mut es = Vec::new();

                loop {
                    match t.peek() {
                        None => return Err("Unexpected EOF".into()),
                        Some(tok) => {
                            if *tok == Token::RParen { break; }
                            es.push(Self::parse_toks(t)?);
                        }
                    }
                }

                t.next();

                Ok(Self::Call(es))
            }
            Some(Token::Ident(i)) => Ok(Self::Atom(i)),
            a => Err(format!("Unexpected token: {:?}", a))
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Atom(s) => s.len(),
            Self::List(es) => {
                es.iter().map(|e| e.len()).fold(0, |a, b| a+b)
            }
            _ => panic!("Ahh")
        }
    }
}

// Printing the parsed expression
impl Display for SExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::List(es) => {
                let l = self.len();

                f.write_str("(")?;
                if l > 100 {
                    f.write_str("\n\t")?;
                }

                for (i, e) in es.iter().enumerate() {
                    if i != 0 && i != es.len() {
                        if l > 100 {
                            f.write_str("\n\t")?;
                        } else {
                            f.write_str(" ")?;
                        }
                    }

                    e.fmt(f)?;
                }

                if l > 100 {
                    f.write_str("\n")?;
                }

                f.write_str(")")?;

            }
            Self::Call(es) => {
                f.write_str("(")?;

                for (i, e) in es.iter().enumerate() {
                    if i != 0 && i != es.len() {
                        f.write_str(" ")?;
                    }

                    e.fmt(f)?;
                }

                f.write_str(")")?;
            }
            Self::Atom(s) => {
                if s.contains(" ") {
                    f.write_str("\"")?;
                    f.write_str(s)?;
                    f.write_str("\"")?;
                } else if s.is_empty() {
                    f.write_str("\"\"")?;
                } else {
                    f.write_str(s)?
                }
            }
        }
        Ok(())
    }
}

pub fn parse_file(s: &str) -> Vec<SExpression> {
    let meta = std::fs::metadata(s);
    if meta.is_err() {
        return vec![];
    }

    let meta = meta.unwrap();
    let mut buf = String::with_capacity(meta.len() as usize);

    let mut f = File::open(s).unwrap();
    f.read_to_string(&mut buf).unwrap();

    let mut tokens = lex(buf.chars()).into_iter().peekable();
    let mut exprs = vec![];

    while let Ok(e) = SExpression::parse_toks(&mut tokens) {
        exprs.push(e)
    }

    exprs
}

#[derive(Default)]
struct InputValidator;

impl rustyline::validate::Validator for InputValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        match SExpression::parse(ctx.input()) {
            Ok(_) => Ok(ValidationResult::Valid(None)),
            Err(_) => Ok(ValidationResult::Incomplete)
        }
    }
}

#[derive(Completer, Helper, Highlighter, Hinter, Validator, Default)]
pub struct InputHelper {
    #[rustyline(Validator)]
    validator: InputValidator
}

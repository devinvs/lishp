use std::fs::File;
use std::io::Read;
use std::collections::LinkedList as List;

use crate::SExpression;
use crate::lexer::{lex, Token};

impl SExpression {
    pub fn parse(s: &str) -> Result<Self, String> {
        let toks = lex(s.chars());
        let mut iter = toks.into_iter().peekable();

        if let Some(Token::EOF) = iter.peek() {
            Ok(Self::Atom(List::new()))
        } else {
            Self::parse_toks(&mut iter)
        }
    }

    fn parse_toks<I>(t: &mut std::iter::Peekable<I>) -> Result<Self, String>
    where I: Iterator<Item = Token> + std::fmt::Debug {
        match t.next() {
            None | Some(Token::EOF) => Err("Unexpected EOF".into()),
            Some(Token::Quote) => {
                let next = Self::parse_toks(t)?;

                Ok(if let Self::Call(es) = next {
                    Self::List(es)
                } else {
                    next
                })
            }
            Some(Token::LParen) => {
                let mut es = List::new();

                loop {
                    match t.peek() {
                        None => return Err("Unexpected EOF".into()),
                        Some(tok) => {
                            if *tok == Token::RParen { break; }
                            es.push_back(Self::parse_toks(t)?);
                        }
                    }
                }

                t.next();

                Ok(Self::Call(es))
            }
            Some(Token::Ident(i)) => {
                Ok(Self::Atom(i))
            },
            a => Err(format!("Unexpected token: {:?}", a))
        }
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

    parse_str(&buf)
}

pub fn parse_str(s: &str) -> Vec<SExpression> {
    let mut tokens = lex(s.chars()).into_iter().peekable();
    let mut exprs = vec![];

    while tokens.peek() != Some(&Token::EOF) {
        match SExpression::parse_toks(&mut tokens) {
            Ok(e) => exprs.push(e),
            Err(e) => {
                eprintln!("Error parsing: {e}");
                break;
            }
        }
    }

    exprs
}

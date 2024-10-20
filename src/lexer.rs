use std::collections::HashMap;
use std::collections::LinkedList as List;

#[derive(Debug, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    Quote,
    Ident(List<char>),
    EOF,
}

// get the character defined by two hex chars
fn hex_to_c(a: char, b: char) -> char {
    let a = a.to_digit(16).unwrap();
    let b = b.to_digit(16).unwrap();
    let c = (a << 4) | b;
    let c = char::from_u32(c).unwrap();
    c
}

pub fn lex(
    mut s: impl Iterator<Item = char>,
    aliases: &HashMap<List<char>, List<List<char>>>,
) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut stack = List::new();

    let mut in_comment = false;
    let mut in_quote = false;
    let mut last_is_paren = false;

    let push = |s: &mut List<char>,
                toks: &mut Vec<Token>,
                in_quote: bool,
                last_is_paren: bool,
                aliases: &HashMap<List<char>, List<List<char>>>| {
        if s.is_empty() && !in_quote {
            return;
        }

        if last_is_paren {
            if let Some(ps) = aliases.get(&s) {
                for s in ps {
                    toks.push(Token::Ident(s.clone()))
                }
            } else {
                toks.push(Token::Ident(s.clone()))
            }
        } else {
            toks.push(Token::Ident(s.clone()))
        }

        s.clear();
    };

    while let Some(c) = s.next() {
        if c != '(' {}

        match c {
            '\n' if in_comment => {
                in_comment = false;
            }
            _ if in_comment => {}
            '\"' => {
                push(&mut stack, &mut tokens, in_quote, last_is_paren, aliases);
                last_is_paren = false;
                in_quote = !in_quote;
            }
            '\\' => {
                let next = s.next().unwrap();
                match next {
                    'n' => {
                        stack.push_back('\n');
                    }
                    'x' => {
                        let a = s.next().unwrap();
                        let b = s.next().unwrap();
                        stack.push_back(hex_to_c(a, b))
                    }
                    _ => stack.push_back(next),
                }
            }
            c if in_quote => stack.push_back(c),
            c if c.is_whitespace() => {
                push(&mut stack, &mut tokens, in_quote, last_is_paren, aliases);
                last_is_paren = false;
            }
            ';' => {
                push(&mut stack, &mut tokens, in_quote, last_is_paren, aliases);
                last_is_paren = false;
                in_comment = true;
            }
            '\'' => {
                push(&mut stack, &mut tokens, in_quote, last_is_paren, aliases);
                last_is_paren = false;
                tokens.push(Token::Quote);
            }
            '(' => {
                push(&mut stack, &mut tokens, in_quote, last_is_paren, aliases);
                tokens.push(Token::LParen);
                last_is_paren = true;
            }
            ')' => {
                push(&mut stack, &mut tokens, in_quote, last_is_paren, aliases);
                last_is_paren = false;
                tokens.push(Token::RParen);
            }
            _ => stack.push_back(c),
        }
    }
    push(&mut stack, &mut tokens, in_quote, last_is_paren, aliases);

    // In order to preserve somewhat normal behavior of the shell,
    // We automatically surround the input in a list if it is not alread a list

    if tokens.len() != 0
        && (tokens[0] != Token::LParen || tokens[tokens.len() - 1] != Token::RParen)
    {
        tokens.insert(0, Token::LParen);
        tokens.push(Token::RParen);
    }

    // push EOF token
    tokens.push(Token::EOF);

    tokens
}

#[derive(Debug, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    Quote,
    Ident(String),
    EOF
}

// get the character defined by two hex chars
fn hex_to_c(a: char, b: char) -> char {
    let a = a.to_digit(16).unwrap();
    let b = b.to_digit(16).unwrap();
    let c = (a << 4) | b;
    let c = char::from_u32(c).unwrap();
    c
}

pub fn lex(mut s: impl Iterator<Item = char>) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut stack = String::with_capacity(10);

    let mut in_comment = false;
    let mut in_quote = false;

    let push = |s: &mut String, toks: &mut Vec<Token>, in_quote: bool| {
        if s.is_empty() && !in_quote { return; }

        toks.push(Token::Ident(s.clone()));
        s.clear();
    };

    while let Some(c) = s.next() {
        match c {
            '\n' if in_comment => {
                in_comment = false;
            }
            _ if in_comment => {}
            '\"' => {
                push(&mut stack, &mut tokens, in_quote);
                in_quote = !in_quote;
            }
            '\\' => {
                let next = s.next().unwrap();
                match next {
                    'n' => {
                        stack.push('\n');
                    }
                    'x' => {
                        let a = s.next().unwrap();
                        let b = s.next().unwrap();
                        stack.push(hex_to_c(a, b))
                    }
                    _ => stack.push(next)
                }
            }
            c if in_quote => stack.push(c),
            c if c.is_whitespace() => {
                push(&mut stack, &mut tokens, in_quote);
            }
            ';' => {
                push(&mut stack, &mut tokens, in_quote);
                in_comment = true;
            }
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

    if tokens.len() != 0 && (tokens[0] != Token::LParen || tokens[tokens.len()-1] != Token::RParen) {
        tokens.insert(0, Token::LParen);
        tokens.push(Token::RParen);
    }

    // push EOF token
    tokens.push(Token::EOF);

    tokens
}

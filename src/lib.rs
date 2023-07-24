pub mod builtins;
pub mod input;
pub mod interpreter;
pub mod lexer;
pub mod parser;

pub use input::Input;
pub use interpreter::Interpreter;

pub use std::collections::LinkedList as List;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExpression {
    Call(List<SExpression>),
    List(List<SExpression>),
    Atom(List<char>),
}

// some basic utility methods
impl SExpression {
    pub fn len(&self) -> usize {
        match self {
            Self::Atom(s) => s.len(),
            Self::List(es) => es.iter().map(|e| e.len()).fold(0, |a, b| a + b),
            Self::Call(_) => panic!("Called len on call expression"),
        }
    }

    pub fn ident(self) -> List<char> {
        match self {
            SExpression::Atom(chars) => chars,
            SExpression::List(l) => {
                let mut i = l.into_iter().map(|e| e.ident());

                if let Some(first) = i.next() {
                    i.fold(first, |mut a, mut e| {
                        a.append(&mut e);
                        a
                    })
                } else {
                    List::new()
                }
            }
            _ => panic!("Called ident on call expression"),
        }
    }

    pub fn replace(self, from: &List<char>, to: SExpression) -> SExpression {
        match self {
            Self::Atom(s) => {
                if s == *from {
                    to
                } else {
                    Self::Atom(s)
                }
            }
            Self::Call(es) => Self::Call(
                es.into_iter()
                    .map(|e| e.replace(from, to.clone()))
                    .collect(),
            ),
            a => a,
        }
    }

    pub fn list(self) -> List<List<char>> {
        match self {
            Self::Call(es) | Self::List(es) => {
                let mut i = es.into_iter().map(|e| e.list());

                if let Some(first) = i.next() {
                    i.fold(first, |mut a, mut e| {
                        a.append(&mut e);
                        a
                    })
                } else {
                    List::new()
                }
            }
            Self::Atom(s) => {
                let mut l = List::new();
                l.push_front(s);
                l
            }
        }
    }
}

// Printing the parsed expression
impl std::fmt::Display for SExpression {
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
                let s: String = s.iter().collect();

                if s.contains(" ") {
                    f.write_str("\"")?;
                    f.write_str(&s)?;
                    f.write_str("\"")?;
                } else if s.is_empty() {
                    f.write_str("\"\"")?;
                } else {
                    f.write_str(&s)?
                }
            }
        }
        Ok(())
    }
}

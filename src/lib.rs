pub mod lexer;
pub mod parser;
pub mod builtins;
pub mod interpreter;
pub mod completer;

pub use interpreter::Interpreter;

pub use std::collections::LinkedList as List;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExpression {
    Call(List<SExpression>),
    List(List<SExpression>),
    Atom(String)
}

// some basic utility methods
impl SExpression {
    pub fn len(&self) -> usize {
        match self {
            Self::Atom(s) => s.len(),
            Self::List(es) => {
                es.iter().map(|e| e.len()).fold(0, |a, b| a+b)
            }
            Self::Call(_) => panic!("Called len on call expression")
        }
    }

    pub fn ident(&self) -> &str {
        match self {
            SExpression::Atom(s) => s,
            SExpression::List(l) => l.front().unwrap().ident(),
            _ => panic!("Called ident on call expression")
        }
    }

    pub fn replace(self, from: &str, to: SExpression) -> SExpression {
        match self {
            Self::Atom(s) => {
                if s==from { to } else { Self::Atom(s) }
            }
            Self::Call(es) => {
                Self::Call(es.into_iter()
                           .map(|e| e.replace(from, to.clone()))
                           .collect())
            }
            a => a
        }
    }

    pub fn list(&self) -> Vec<String> {
        let mut ls = Vec::new();

        match self {
            Self::Call(es) | Self::List(es) => {
                es.iter().for_each(|e| ls.push(e.ident().to_string()))
            }
            Self::Atom(s) => ls.push(s.clone())
        }

        ls
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

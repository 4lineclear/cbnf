use crate::lexer;
use crate::span::BSpan;

// TODO: consider having expected be a nested list of Lexemes

// TODO: add more values, etc, to errors
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    InvalidLit(InvalidLiteral, BSpan),
    Unterminated(BSpan),
    Expected(BSpan, Box<[lexer::LexKind]>),
}

impl Error {
    /// `Some(error)` means a non congregated error
    pub fn span(&self) -> BSpan {
        match self {
            Error::InvalidLit(_, span) | Error::Unterminated(span) | Error::Expected(span, _) => {
                *span
            }
        }
    }
    pub fn message(&self) -> String {
        match self {
            Error::InvalidLit(lit, _) => match lit {
                InvalidLiteral::Numeric => "Numbers not allowed",
                InvalidLiteral::Unterminated => "Unterminated terminal found",
            }
            .into(),
            Error::Unterminated(_) => "Group not terminated".into(),
            Error::Expected(_, acc) => {
                if acc.len() == 0 {
                    return "Token not expected".into();
                }
                let mut o = String::from("Token not expected, expected one of: [ ");
                o.push_str(acc[0].name());
                for l in acc[1..].iter() {
                    o.push_str(", ");
                    o.push_str(l.name());
                }
                o.push_str(" ]");
                o
            }
        }
    }
    /// `Some(error)` means a non congregated error
    pub fn congregate(&mut self, other: Self) -> Option<Self> {
        // NOTE: only Expected errors are congregated currently,
        // in the future when more errors are supported,
        // the system below must change to support it
        use Error::*;
        let Expected(span, exp) = self else {
            return Some(other);
        };
        let Expected(other_span, other_exp) = other else {
            return Some(other);
        };
        if span.to != other_span.from {
            return Some(Expected(other_span, other_exp));
        }
        if exp != &other_exp {
            return Some(Expected(other_span, other_exp));
        }
        span.to = other_span.to;
        None
    }
}

impl From<(InvalidLiteral, BSpan)> for Error {
    fn from(value: (InvalidLiteral, BSpan)) -> Self {
        Self::InvalidLit(value.0, value.1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvalidLiteral {
    /// Numeric Literal Found
    Numeric,
    /// Unterminated Literal Found
    Unterminated,
}

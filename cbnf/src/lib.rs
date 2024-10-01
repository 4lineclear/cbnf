// #![allow(clippy::wildcard_imports)]
use self::lexer::DocStyle;
use self::parser::error::Error;
use self::parser::Parser;
use self::span::BSpan;

pub mod lexer;
pub mod parser;
pub mod span;
pub mod util;

// TODO: resolve rule names.

#[derive(Clone, Debug)]
pub struct Comment(pub BSpan);

#[derive(Clone, Debug)]
pub struct DocComment(pub DocStyle, pub BSpan);

/// Complex Bachus-Naur Form
#[derive(Default, Clone, Debug)]
pub struct Cbnf {
    rules: Vec<Rule>,
    comments: Vec<Comment>,
    docs: Vec<DocComment>,
    errors: Vec<Error>,
}

impl From<Parser<'_>> for Cbnf {
    fn from(mut value: Parser<'_>) -> Self {
        let rules = core::iter::from_fn(|| value.next_rule()).collect();
        let (_, comments, docs, errors) = value.parts();
        Self {
            rules,
            comments,
            docs,
            errors,
        }
    }
}

impl Cbnf {
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
    #[must_use]
    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }
    #[must_use]
    pub fn docs(&self) -> &[DocComment] {
        &self.docs
    }
    #[must_use]
    pub fn errors(&self) -> &[Error] {
        &self.errors
    }
    #[must_use]
    pub fn parse(input: &str) -> Self {
        Self::from(Parser::new(input))
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: BSpan,
    pub expr: Option<Expression>,
    /// The span of the entire `Rule`
    ///
    /// This should extend either to the closing brace or semicolon
    pub span: BSpan,
}

/// An expression is a list seperated by delimiters
#[derive(Debug, Clone)]
pub struct Expression {
    pub span: BSpan,
    pub parts: Vec<(Delim, List)>,
}

/// Seperates different lists
#[derive(Debug, Clone)]
pub enum Delim {
    Or,
}

/// An list is a set of Terms seperated by whitespace
#[derive(Debug, Clone)]
pub struct List {
    pub span: BSpan,
    pub terms: Vec<Term>,
}

impl List {
    pub(crate) const fn new(span: BSpan) -> Self {
        Self {
            terms: Vec::new(),
            span,
        }
    }
    pub(crate) fn reset_span(&mut self, default: BSpan) {
        self.span = self.terms.first().map_or(default, |f| {
            f.span().to(self.terms.last().unwrap_or(f).span().to)
        });
    }
}

/// A single item within a list
#[derive(Debug, Clone)]
pub enum Term {
    /// ".."
    Literal(BSpan),
    /// ..
    Ident(BSpan),
    /// $ ..
    Meta(BSpan),
    /// ( .. )
    Group(Expression),
}

impl Term {
    #[must_use]
    pub const fn span(&self) -> BSpan {
        use Term::*;
        match self {
            Literal(span) | Ident(span) | Meta(span) => *span,
            Group(e) => e.span,
        }
    }
}

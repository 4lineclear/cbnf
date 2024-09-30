use self::lexer::DocStyle;
use self::parser::error::Error;
use self::parser::Parser;
use self::span::BSpan;

pub mod lexer;
pub mod parser;
pub mod span;
pub mod util;

// TODO: consider replacing all Vec's with TSpan
// having several large collections instead of multiple fragmented ones

#[derive(Debug)]
pub struct Comment {
    pub style: Option<DocStyle>,
    pub content: BSpan,
}

/// Complex Bachus-Naur Form
#[derive(Default)]
pub struct Cbnf {
    rules: Vec<Rule>,
    comments: Vec<Comment>,
    errors: Vec<Error>,
}

impl From<Parser<'_>> for Cbnf {
    fn from(mut value: Parser<'_>) -> Self {
        let rules = core::iter::from_fn(|| value.next_rule()).collect();
        let (_, comments, errors) = value.parts();
        Cbnf {
            rules,
            comments,
            errors,
        }
    }
}

impl Cbnf {
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }
    pub fn errors(&self) -> &[Error] {
        &self.errors
    }
    pub fn parse(input: &str) -> Self {
        Cbnf::from(Parser::new(input))
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
    pub(crate) fn new(span: BSpan) -> Self {
        Self {
            terms: Vec::new(),
            span,
        }
    }
    pub(crate) fn reset_span(&mut self, default: BSpan) {
        self.span = (&self.terms)
            .first()
            .map(|f| f.span().to(self.terms.last().unwrap_or(f).span().to))
            .unwrap_or(default);
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
    pub fn span(&self) -> BSpan {
        match self {
            Term::Literal(span) | Term::Ident(span) | Term::Meta(span) => *span,
            Term::Group(e) => e.span,
        }
    }
}

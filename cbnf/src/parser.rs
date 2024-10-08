#![allow(clippy::cast_possible_truncation)]
use crate::{
    lexer::{Base, Cursor, LexKind, LiteralKind, *},
    parser::error::{Error, InvalidLiteral},
    span::{BSpan, TSpan},
    util::*,
    Comment, DocComment, List, Rule, Term,
};

pub mod error;

#[cfg(test)]
mod test;

pub struct Parser<'a> {
    pub(crate) cursor: Cursor<'a>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) docs: Vec<DocComment>,
    pub(crate) errors: Vec<Error>,
    /// a central list of terms
    pub(crate) terms: Vec<Term>,
}

impl<'a> Parser<'a> {
    #[allow(dead_code)]
    fn lex_non_wc(&mut self) -> Option<Lexeme> {
        let token = self.cursor.advance();
        (!self.handle_wc(token)).then_some(token)
    }
    fn lex_until_non_wc(&mut self) -> Lexeme {
        loop {
            let token = self.cursor.advance();
            if !self.handle_wc(token) {
                break token;
            }
        }
    }
    fn push_comment(&mut self, style: Option<DocStyle>, content: impl Into<AsBSpan>) {
        let span = self.span(content);
        match style {
            Some(style) => self.docs.push(DocComment(style, span)),
            None => self.comments.push(Comment(span)),
        }
    }
    fn handle_wc(&mut self, token: Lexeme) -> bool {
        if let LineComment { doc_style } = token.kind {
            self.push_comment(doc_style, token);
        }
        matches!(token.kind, LineComment { .. } | Whitespace)
    }
    fn err_unterminated(&mut self, span: impl Into<AsBSpan>) {
        self.push_err(Error::Unterminated(self.span(span)));
    }
    fn err_expected(&mut self, span: impl Into<AsBSpan>, expected: impl Into<Box<[LexKind]>>) {
        self.push_err(Error::Expected(self.span(span), expected.into()));
    }
    fn push_err(&mut self, err: impl Into<Error>) {
        let err: Error = err.into();
        let Some(prev) = self.errors.last_mut() else {
            self.errors.push(err);
            return;
        };
        if let Some(err) = prev.congregate(err) {
            self.errors.push(err);
        }
    }
    #[must_use]
    const fn src(&self) -> &str {
        self.cursor.src()
    }
    #[must_use]
    pub fn slice(&self, span: impl Into<AsBSpan>) -> &str {
        self.span(span).slice(self.src())
    }
    pub fn span(&self, span: impl Into<AsBSpan>) -> BSpan {
        match span.into() {
            AsBSpan::Len(len) => self.token_span(len),
            AsBSpan::Lex(token) => self.token_span(token.len),
            AsBSpan::Span(span) => span,
        }
    }
    #[must_use]
    const fn token_pos(&self) -> u32 {
        self.cursor.lex_pos()
    }
    #[must_use]
    const fn token_span(&self, len: u32) -> BSpan {
        BSpan::new(self.token_pos(), self.token_pos() + len)
    }
}

pub const LIST_EXPECTED: [LexKind; 5] = [OpenParen, Ident, Or, LITERAL, CloseBrace];
pub const RULE_EXPECTED: [LexKind; 2] = [Dollar, Ident];

impl<'a> Parser<'a> {
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::new(input),
            comments: Vec::new(),
            docs: Vec::new(),
            errors: Vec::new(),
            terms: Vec::new(),
        }
    }

    #[must_use]
    pub fn next_rule(&mut self) -> Option<Rule> {
        let (name, open) = if let Some(span) = self.until_dollar_or_ident()? {
            (span, self.open_brace()?)
        } else {
            let dollar = self.token_pos();
            let ident = self.ident().ok()?;
            let semi = self.semi_or_open_brace()?;
            let span = ident.from(dollar);
            if semi {
                return Some(Rule {
                    name: span,
                    expr: None,
                    span,
                });
            }
            (span, self.token_pos())
        };
        let (close, terms) = self.list();
        let span = name.to(self.cursor.pos());
        Some(Rule {
            name,
            expr: Some(List {
                span: (open, close).into(),
                terms,
            }),
            span,
        })
    }

    fn list(&mut self) -> (u32, TSpan) {
        let first = self.terms.len() as u32;
        let mut token = self.lex_until_non_wc();
        let mut groups = Vec::new();
        let mut ors = Vec::new();
        let (span, eof) = loop {
            let span = self.span(token);
            match token.kind {
                CloseBrace => break (span, false),
                Eof => break (span, true),
                OpenParen => {
                    groups.push(self.terms.len() as u32);
                    self.terms.push(Term::Group(List::new(
                        span,
                        TSpan::empty(self.terms.len() as u32),
                    )));
                }
                CloseParen if !groups.is_empty() => {
                    self.pop_group(&mut ors, &mut groups, span.to);
                }
                Or => {
                    self.handle_or(&mut ors, &groups);
                    ors.push(self.terms.len() as u32);
                    self.terms.push(Term::Or(List::new(
                        span,
                        TSpan::empty(self.terms.len() as u32),
                    )));
                }
                Ident => self.terms.push(Term::Ident(span)),
                Dollar => match self.ident().map(|s| s.to) {
                    Ok(to) => self.terms.push(Term::Ident(span.to(to))),
                    Err(other) => {
                        token = other;
                        continue;
                    }
                },
                Literal { kind, .. } if kind.is_string() => {
                    if !kind.terminated() {
                        self.push_err((InvalidLiteral::Unterminated, span));
                    }
                    self.terms.push(Term::Literal(span));
                }
                Literal { .. } => self.push_err((InvalidLiteral::Numeric, span)),
                // TODO: also add a CloseParen item to the EXPECTED when there
                // are unclosed groups
                _ => self.err_expected(span, LIST_EXPECTED),
            }
            token = self.lex_until_non_wc();
        };
        self.handle_or(&mut ors, &groups);
        self.handle_unclosed(groups, span);
        if eof {
            self.err_expected(span, [CloseBrace]);
        }
        assert!(ors.is_empty(), "or backlog not empty: {ors:#?}");
        (span.to, (first, self.terms.len() as u32).into())
    }

    // TODO: bspan.to may not be going to the last byte(close paren) of a group.
    fn handle_or(&mut self, ors: &mut Vec<u32>, groups: &[u32]) -> bool {
        let or = match (ors.last(), groups.last()) {
            (Some(&o), Some(&g)) if o > g => o,
            (Some(&o), None) => o,
            _ => return false,
        };
        let len = self.terms.len();
        let to = self.terms[len - 1].span().to;
        let Term::Or(list) = &mut self.terms[or as usize] else {
            unreachable!("non 'or' found at index {or}")
        };
        list.terms.to = len as u32;
        list.span.to = to;
        ors.pop();
        true
    }

    fn pop_group(&mut self, ors: &mut Vec<u32>, groups: &mut Vec<u32>, to: u32) {
        let Some(&group) = groups.last() else { return };
        let len = self.terms.len();
        if let Term::Group(list) = &mut self.terms[group as usize] {
            list.terms.to = len as u32;
            list.span.to = to;
        } else {
            unreachable!("group 'or' found at index {group}")
        };
        groups.pop();
        while self.handle_or(ors, groups) {}
        if let Some(Term::Or(or)) = self.terms.get_mut(group.saturating_sub(1) as usize) {
            or.span.to = to;
        }
    }

    fn handle_unclosed(&mut self, groups: Vec<u32>, span: BSpan) {
        for group in groups {
            let err_span;
            let len = self.terms.len();
            if let Term::Group(group) = &mut self.terms[group as usize] {
                group.span.to = span.to;
                group.terms.to = len as u32;
                err_span = group.span;
            } else {
                unreachable!("group 'or' found at index {group}")
            };
            self.err_unterminated(err_span);
        }
    }

    fn ident(&mut self) -> Result<BSpan, Lexeme> {
        let token = self.lex_until_non_wc();
        if let Ident = token.kind {
            Ok(self.span(token))
        } else {
            self.err_expected(self.span(token), [Ident]);
            Err(token)
        }
    }

    /// `X` = Eof, `Y` = Dollar, `Z` = Ident
    ///
    /// Runs until one of the above is found
    fn until_dollar_or_ident(&mut self) -> Option<Option<BSpan>> {
        loop {
            let token = self.lex_until_non_wc();
            match token.kind {
                Dollar => break Some(None),
                Ident => break Some(Some(self.span(token))),
                Eof => return None,
                _ => self.err_expected(token, [Dollar, Ident]),
            };
        }
    }

    /// `A` = Semi, `B` = Brace
    fn open_brace(&mut self) -> Option<u32> {
        let token = self.lex_until_non_wc();
        if OpenBrace == token.kind {
            Some(self.token_pos())
        } else {
            self.err_expected(token, [OpenBrace]);
            None
        }
    }

    /// `true` if semi else `false`
    fn semi_or_open_brace(&mut self) -> Option<bool> {
        let token = self.lex_until_non_wc();
        match token.kind {
            Semi => true.into(),
            OpenBrace => false.into(),
            _ => {
                self.err_expected(token, [Semi, OpenBrace]);
                None
            }
        }
    }
}

const LITERAL: LexKind = LexKind::Literal {
    kind: LiteralKind::Int {
        base: Base::Binary,
        empty_int: false,
    },
    suffix_start: 0,
};

#[allow(dead_code)]
const LINE_COMMENT: LexKind = LexKind::LineComment { doc_style: None };

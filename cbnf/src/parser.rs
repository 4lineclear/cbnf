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
    pub const fn token_pos(&self) -> usize {
        self.cursor.lex_pos()
    }
    #[must_use]
    const fn token_span(&self, len: usize) -> BSpan {
        BSpan::new(self.token_pos(), self.token_pos() + len)
    }
    fn handle_other<T>(
        &mut self,
        token: Lexeme,
        expected: impl Into<Box<[LexKind]>>,
    ) -> Filtered<T> {
        self.err_expected(token, expected);
        Other(token)
    }
}

pub const LIST_EXPECTED: [LexKind; 5] = [OpenParen, Ident, RawIdent, LITERAL, CloseBrace];
pub const RULE_EXPECTED: [LexKind; 3] = [Dollar, Ident, RawIdent];

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
        let (name, open) = match self.until_dollar_or_ident() {
            X(()) => return None,
            Y(pos) => {
                let ident = self.ident().ok()?;
                let semi = self.semi_or_open_brace().opt()?;
                let span = ident.from(pos);
                if semi {
                    return Some(Rule {
                        name: span,
                        expr: None,
                        span,
                    });
                }
                (span, self.token_pos())
            }
            Z(span) => (span, self.open_brace().opt().map(|()| self.token_pos())?),
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

    fn handle_or(&mut self, ors: &mut Vec<usize>, groups: &mut Vec<usize>) -> bool {
        let (or, _) = match ors.last().zip(groups.last()) {
            Some((&o, &g)) if o < g => (o, g),
            _ => return false,
        };
        let len = self.terms.len();
        let span_to = self.terms[len - 1].span().to;
        match &mut self.terms[or] {
            Term::Or(list) => {
                list.terms.to = len;
                list.span.to = span_to;
            }
            _ => unreachable!("non 'or' found at index {or}"),
        }
        ors.pop();
        true
    }
    fn pop_group(&mut self, ors: &mut Vec<usize>, groups: &mut Vec<usize>, to: usize) {
        let Some(&group) = groups.last() else { return };
        let len = self.terms.len();
        match &mut self.terms[group] {
            Term::Group(list) => {
                list.terms.to = len;
                list.span.to = to;
            }
            _ => unreachable!("group 'or' found at index {group}"),
        }
        while self.handle_or(ors, groups) {}
        groups.pop();
    }
    fn handle_unclosed(&mut self, groups: Vec<usize>, span: BSpan) {
        groups.into_iter().for_each(|group| {
            let err_span;
            let len = self.terms.len();
            if let Term::Group(group) = &mut self.terms[group] {
                group.span.to = span.to;
                group.terms.to = len;
                err_span = group.span;
            } else {
                unreachable!("group 'or' found at index {group}")
            };
            self.err_unterminated(err_span);
        });
    }
    fn list(&mut self) -> (usize, TSpan) {
        // TODO: consider moving groups & ors to self
        let first = self.terms.len();
        let mut token = self.lex_until_non_wc();
        let mut groups = Vec::new();
        let mut ors = Vec::new();
        let (span, eof) = loop {
            let span = self.span(token);
            match token.kind {
                // TODO: consider adding error/diagnostic for open braces
                CloseBrace => break (span, false),
                Eof => break (span, true),
                OpenParen => {
                    groups.push(self.terms.len());
                    self.terms
                        .push(Term::Group(List::new(span, TSpan::empty(self.terms.len()))));
                }
                CloseParen if !groups.is_empty() => {
                    self.pop_group(&mut ors, &mut groups, span.to);
                }
                Ident if self.slice(span) == "or" => {
                    self.handle_or(&mut ors, &mut groups);
                    ors.push(self.terms.len());
                    self.terms
                        .push(Term::Or(List::new(span, TSpan::empty(self.terms.len()))));
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
                        self.push_err((InvalidLiteral::Unterminated, span))
                    }
                    self.terms.push(Term::Literal(span));
                }
                Literal { .. } => self.push_err((InvalidLiteral::Numeric, span)),
                _ => self.err_expected(span, LIST_EXPECTED),
            }
            token = self.lex_until_non_wc();
        };
        // self.pop_group(&mut ors, &mut groups);
        self.handle_or(&mut ors, &mut groups);
        self.handle_unclosed(groups, span);
        if eof {
            self.err_expected(span, [CloseBrace]);
        }
        (span.to, (first, self.terms.len()).into())
    }
    fn ident(&mut self) -> Result<BSpan, Lexeme> {
        let token = self.lex_until_non_wc();
        if let Ident | RawIdent = token.kind {
            Ok(self.span(token))
        } else {
            self.err_expected(self.span(token), [Ident]);
            Err(token)
        }
    }
    /// `X` = Eof, `Y` = Dollar, `Z` = Ident
    ///
    /// Runs until one of the above is found
    fn until_dollar_or_ident(&mut self) -> Either3<(), usize, BSpan> {
        loop {
            match look_for!(match (self, token, [Dollar, Ident, RawIdent]) {
                Dollar => break Y(self.token_pos()).into(),
                Ident | RawIdent => break Z(self.span(token)).into(),
                Eof => break InputEnd,
            }) {
                InputEnd => return X(()),
                Correct(span) => break span,
                Other(_) => (),
            }
        }
    }
    /// `A` = Semi, `B` = Brace
    fn open_brace(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [OpenBrace]) {
            OpenBrace => break ().into(),
        })
    }

    /// `true` if semi else `false`
    fn semi_or_open_brace(&mut self) -> Filtered<bool> {
        look_for!(match (self, token, [Semi, OpenBrace]) {
            Semi => break true.into(),
            OpenBrace => break false.into(),
        })
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

#[allow(unused)]
use Either::*;
use Either3::*;
use Filtered::*;

// TODO: retire the below

macro_rules! look_for {
    (match ($this:ident, $lex:ident, $expected: expr $(, $span:ident)?) {
        $($matcher:pat $(if $pred:expr)? => $result:expr $(,)?)*
    }) => {{
        use LexKind::*;
        loop {
            let Some($lex) = $this.lex_non_wc() else {
                continue;
            };
            #[allow(unused)]
            $(let $span = $this.span($lex);)?
            match $lex.kind {
                $($matcher $(if $pred)? => $result,)*
                #[allow(unreachable_patterns)]
                _ => break $this.handle_other($lex, $expected)
            }
        }
    }};
}

use look_for;

// use self::token::LiteralGroup;

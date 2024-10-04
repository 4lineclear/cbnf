//! byte spans, tokens spans

use std::ops::Range;

// TODO: consider renaping BSpan & TSpan

/// A byte span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BSpan {
    /// inclusive
    pub from: usize,
    /// exclusive
    pub to: usize,
}

/// A token span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TSpan {
    /// inclusive
    pub from: usize,
    /// exclusive
    pub to: usize,
}

impl From<(usize, usize)> for BSpan {
    fn from(value: (usize, usize)) -> Self {
        Self {
            from: value.0,
            to: value.1,
        }
    }
}
impl From<(usize, usize)> for TSpan {
    fn from(value: (usize, usize)) -> Self {
        Self {
            from: value.0,
            to: value.1,
        }
    }
}

impl BSpan {
    #[must_use]
    pub const fn empty(i: usize) -> Self {
        Self::new(i, i)
    }
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.from == self.to
    }
    #[must_use]
    pub const fn new(from: usize, to: usize) -> Self {
        Self { from, to }
    }
    #[must_use]
    pub const fn from_len(from: usize, len: usize) -> Self {
        Self::new(from, from + len)
    }
    #[must_use]
    pub const fn from(mut self, from: usize) -> Self {
        self.from = from;
        self
    }

    #[must_use]
    pub const fn to(mut self, to: usize) -> Self {
        self.to = to;
        self
    }

    #[must_use]
    pub fn slice<'a>(&self, item: &'a str) -> &'a str {
        &item[self.from..self.to]
    }
}

impl TSpan {
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.from == self.to
    }
    #[must_use]
    pub const fn empty(i: usize) -> Self {
        Self::new(i, i)
    }
    #[must_use]
    pub const fn from(mut self, from: usize) -> Self {
        self.from = from;
        self
    }
    #[must_use]
    pub const fn to(mut self, to: usize) -> Self {
        self.to = to;
        self
    }
    #[must_use]
    pub const fn new(from: usize, to: usize) -> Self {
        Self { from, to }
    }
    pub fn range(&self) -> Range<usize> {
        self.from..self.to
    }
}

//! byte spans, tokens spans

use std::ops::Range;

/// A byte span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BSpan {
    /// inclusive
    pub from: u32,
    /// exclusive
    pub to: u32,
}

/// A token span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TSpan {
    /// inclusive
    pub from: u32,
    /// exclusive
    pub to: u32,
}

impl From<(u32, u32)> for BSpan {
    fn from(value: (u32, u32)) -> Self {
        Self {
            from: value.0,
            to: value.1,
        }
    }
}
impl From<(u32, u32)> for TSpan {
    fn from(value: (u32, u32)) -> Self {
        Self {
            from: value.0,
            to: value.1,
        }
    }
}

impl BSpan {
    #[must_use]
    pub const fn empty(i: u32) -> Self {
        Self::new(i, i)
    }
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.from == self.to
    }
    #[must_use]
    pub const fn new(from: u32, to: u32) -> Self {
        Self { from, to }
    }
    #[must_use]
    pub const fn from_len(from: u32, len: u32) -> Self {
        Self::new(from, from + len)
    }
    #[must_use]
    pub const fn from(mut self, from: u32) -> Self {
        self.from = from;
        self
    }

    #[must_use]
    pub const fn to(mut self, to: u32) -> Self {
        self.to = to;
        self
    }

    #[must_use]
    pub fn slice<'a>(&self, item: &'a str) -> &'a str {
        &item[self.from as usize..self.to as usize]
    }
}

impl TSpan {
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.from == self.to
    }
    #[must_use]
    pub const fn empty(i: u32) -> Self {
        Self::new(i, i)
    }
    #[must_use]
    pub const fn from(mut self, from: u32) -> Self {
        self.from = from;
        self
    }
    #[must_use]
    pub const fn to(mut self, to: u32) -> Self {
        self.to = to;
        self
    }
    #[must_use]
    pub const fn new(from: u32, to: u32) -> Self {
        Self { from, to }
    }
    #[must_use]
    pub const fn range(&self) -> Range<usize> {
        self.from as usize..self.to as usize
    }
}

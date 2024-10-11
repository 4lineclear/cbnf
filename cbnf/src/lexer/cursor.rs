#![allow(clippy::cast_possible_truncation)]
use std::str::Chars;

use super::{token::LexKind, Lexeme};

// PERF: Chars is slightly faster than &str
// Also: may want to simplify closer to the og for more perf

/// Peekable iterator over a char sequence.
///
/// Next characters can be peeked via `first` method,
/// and position can be shifted forward via `bump` method.
#[derive(Debug)]
pub struct Cursor<'a> {
    pub(super) token_pos: u32,
    len_remaining: u32,
    src: &'a str,
    chars: Chars<'a>,
    #[cfg(debug_assertions)]
    prev: char,
    #[cfg(debug_assertions)]
    prev_token: Lexeme,
}

impl Default for Cursor<'_> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<'a> Cursor<'a> {
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            token_pos: 0,
            len_remaining: input.len() as u32,
            src: input,
            chars: input.chars(),
            #[cfg(debug_assertions)]
            prev: EOF_CHAR,
            #[cfg(debug_assertions)]
            prev_token: Lexeme::new(LexKind::Eof, 0),
        }
    }
}

pub const EOF_CHAR: char = '\0';

impl Cursor<'_> {
    /// The current subslice as a string

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.chars.as_str()
    }

    #[must_use]
    pub fn pos(&self) -> u32 {
        (self.src.len() - self.chars.as_str().len()) as u32
    }

    /// the position of the start of the previous lexeme
    #[must_use]
    pub const fn lex_pos(&self) -> u32 {
        self.token_pos
    }

    #[inline]
    #[must_use]
    pub const fn src(&self) -> &str {
        self.src
    }

    /// Returns the last eaten symbol (or `'\0'` in release builds).
    /// (For debug assertions only.)
    #[cfg(debug_assertions)]
    #[must_use]
    pub const fn prev(&self) -> char {
        self.prev
    }

    /// Returns the last eaten token
    /// (For debug assertions only.)
    #[cfg(debug_assertions)]
    #[must_use]
    pub const fn prev_token(&self) -> Lexeme {
        self.prev_token
    }

    /// Peeks the next symbol from the input stream without consuming it.
    /// If requested position doesn't exist, `EOF_CHAR` is returned.
    /// However, getting `EOF_CHAR` doesn't always mean actual end of file,
    /// it should be checked with `is_eof` method.
    #[must_use]
    pub fn first(&self) -> char {
        // PERF: `.next()` optimizes better than `.nth(0)`
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    #[must_use]
    pub fn second(&self) -> char {
        // PERF: `.next()` optimizes better than `.nth(1)`
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the third symbol from the input stream without consuming it.
    #[must_use]
    pub fn third(&self) -> char {
        // PERF: `.next()` optimizes better than `.nth(2)`
        let mut iter = self.chars.clone();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Checks if there is nothing more to consume.
    #[must_use]
    pub fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Returns amount of already consumed symbols.
    #[must_use]
    pub fn pos_within_token(&self) -> u32 {
        self.len_remaining - self.chars.as_str().len() as u32
    }

    /// Resets the number of bytes consumed to 0.
    pub fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len() as u32;
    }

    /// Moves to the next character.
    pub fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }
        Some(c)
    }

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // PERF: It was tried making optimized version of this for eg. line comments, but
        // LLVM can inline all of this and compile it down to fast iteration over bytes.
        while predicate(self.first()) && !self.is_eof() {
            self.bump();
        }
    }
}

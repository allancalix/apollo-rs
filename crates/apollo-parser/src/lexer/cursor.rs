use std::str::{CharIndices, Chars};

use crate::Error;
/// Peekable iterator over a char sequence.
pub(crate) struct Cursor<'a> {
    index: usize,
    offset: usize,
    prev: usize,
    source: &'a str,
    chars: CharIndices<'a>,
    pending: Option<char>,
    pub(crate) err: Option<Error>,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(input: &'a str) -> Cursor<'a> {
        Cursor {
            index: 0,
            offset: 0,
            prev: 0,
            pending: None,
            source: input,
            chars: input.char_indices(),
            err: None,
        }
    }
}

pub(crate) const EOF_CHAR: char = '\0';

impl<'a> Cursor<'a> {
    /// Returns nth character relative to the current cursor position.
    fn nth_char(&self, n: usize) -> char {
        self.chars().nth(n).unwrap_or(EOF_CHAR)
    }

    /// Peeks the next char in input without consuming.
    pub(crate) fn first(&self) -> char {
        self.nth_char(0)
    }

    /// Peeks the second char in input without consuming.
    pub(crate) fn second(&self) -> char {
        self.nth_char(1)
    }

    /// Checks if there are chars to consume.
    pub(crate) fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    pub(crate) fn index(&self) -> usize {
        self.index
    }

    pub(crate) fn pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn pending_len(&self) -> usize {
        self.offset - self.index
    }

    /// Moves to the next character.
    pub(crate) fn prev_str(&mut self) -> &'a str {
        let slice = &self.source[self.index..self.offset];

        self.index = self.offset;
        self.pending = self.source[self.offset..].chars().next();

        slice
    }

    /// Moves to the next character.
    pub(crate) fn current_str(&mut self) -> &'a str {
        self.pending = None;
        let slice = &self.source[self.index..=self.offset];

        self.index = self.offset;
        self.offset = self.offset;
        if let Some((pos, next)) = self.chars.next() {
            self.index = pos;
            self.offset = pos;
            self.pending = Some(next);
        }

        slice
    }

    /// Moves to the next character.
    pub(crate) fn bump(&mut self) -> Option<char> {
        if let Some(c) = self.pending {
            self.pending = None;

            return Some(c);
        }

        if self.offset == self.source.len() {
            return None;
        }

        let (pos, c) = self.chars.next()?;
        self.prev = self.offset;
        self.offset = pos;

        Some(c)
    }

    /// Moves to the next character.
    pub(crate) fn eatc(&mut self, c: char) -> bool {
        if self.pending.is_some() {
            panic!("dont call eatc when a character is pending");
        }

        if let Some((pos, c_in)) = self.chars.next() {
            self.prev = self.offset;
            self.offset = pos;

            if c_in == c {
                return true;
            }

            self.pending = Some(c_in);
        }

        false
    }

    pub(crate) fn drain(&mut self) {
        while let Some((pos, _c)) = self.chars.next() {
            self.prev = self.offset;
            self.offset = pos;
        }
    }

    /// Get current error object in the cursor.
    pub(crate) fn err(&mut self) -> Option<Error> {
        self.err.clone()
    }

    /// Add error object to the cursor.
    pub(crate) fn add_err(&mut self, err: Error) {
        self.err = Some(err)
    }

    /// Returns a `Chars` iterator over the remaining characters.
    pub fn chars(&self) -> Chars<'_> {
        self.source[self.offset..self.source.len()].chars()
    }
}

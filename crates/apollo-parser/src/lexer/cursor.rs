use std::str::CharIndices;

use crate::Error;

/// Peekable iterator over a char sequence.
#[derive(Debug, Clone)]
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

impl<'a> Cursor<'a> {
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    fn eof(&self) -> bool {
        self.offset == self.source.len()
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
        if self.eof() {
            return &self.source[self.index..];
        }
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
        if self.pending.is_some() {
            return self.pending.take();
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

    /// Get current error object in the cursor.
    pub(crate) fn err(&mut self) -> Option<Error> {
        self.err.clone()
    }

    pub(crate) fn drain(&mut self) -> &'a str{
        self.pending = None;
        let start = self.index;
        self.index = self.source.len() - 1;

        &self.source[start..=self.index]
    }

    /// Add error object to the cursor.
    pub(crate) fn add_err(&mut self, err: Error) {
        self.err = Some(err)
    }
}

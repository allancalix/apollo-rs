mod cursor;
mod token;
mod token_kind;

use std::slice::Iter;

use crate::{lexer::cursor::Cursor, Error};

pub use token::Token;
pub use token_kind::TokenKind;
/// Parses tokens into text.
/// ```rust
/// use apollo_parser::Lexer;
///
/// let query = "
/// {
///     animal
///     ...snackSelection
///     ... on Pet {
///       playmates {
///         count
///       }
///     }
/// }
/// ";
/// let lexer = Lexer::new(query);
/// assert_eq!(lexer.errors().len(), 0);
///
/// let tokens = lexer.tokens();
/// ```
pub struct Lexer<'a> {
    tokens: Vec<Token<'a>>,
    errors: Vec<Error>,
}

impl<'a> Lexer<'a> {
    /// Create a new instance of `Lexer`.
    pub fn new(input: &'a str) -> Self {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        let mut token_stream = LexerIterator::new(input);
        while let Some(result) = token_stream.next() {
            match result {
                Ok(t) => tokens.push(t),
                Err(e) => errors.push(e),
            }
        }

        Self { tokens, errors }
    }

    /// Get a reference to the lexer's tokens.
    pub fn tokens(&self) -> &[Token] {
        self.tokens.as_slice()
    }

    /// Get a reference to the lexer's errors.
    pub fn errors(&self) -> Iter<'_, Error> {
        self.errors.iter()
    }
}

#[derive(Clone, Debug)]
pub struct LexerIterator<'a> {
    cursor: Cursor<'a>,
    finished: bool,
}

impl<'a> LexerIterator<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::new(input),
            finished: false,
        }
    }
}

impl<'a> Iterator for LexerIterator<'a> {
    type Item = Result<Token<'a>, Error>;

    fn next<'b>(&'b mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.cursor.advance() {
            Ok(token) => {
                if matches!(token.kind(), TokenKind::Eof) {
                    self.finished = true;

                    return Some(Ok(token));
                }

                Some(Ok(token))
            }
            Err(err) => {
                Some(Err(err))
            }
        }
    }
}

impl<'a> Cursor<'a> {
    fn advance(&mut self) -> Result<Token<'a>, Error> {
        #[derive(Debug)]
        enum State {
            Start,
            Done,
            Ident,
            StringLiteral,
            StringLiteralStart,
            BlockStringLiteral,
            StringLiteralBackslash,
            IntLiteral,
            FloatLiteral,
            ExponentLiteral,
            Whitespace,
            Comment,
            SpreadOperator,
            PlusMinus,
        }

        let mut state = State::Start;
        let mut token = Token {
            kind: TokenKind::Eof,
            data: "EOF",
            index: self.index(),
        };

        while let Some(c) = self.bump() {
            match state {
                State::Start => {
                    match c {
                        '"' => {
                            token.kind = TokenKind::StringValue;
                            state = State::StringLiteralStart;
                        }
                        '#' => {
                            token.kind = TokenKind::Comment;
                            state = State::Comment;
                        }
                        '.' => {
                            token.kind = TokenKind::Spread;
                            state = State::SpreadOperator;
                        }
                        c if is_whitespace(c) => {
                            token.kind = TokenKind::Whitespace;
                            state = State::Whitespace;
                        }
                        c if is_ident_char(c) => {
                            token.kind = TokenKind::Name;
                            state = State::Ident;
                        }
                        '+' | '-' => {
                            token.kind = TokenKind::Int;
                            state = State::PlusMinus;
                        }
                        c if is_digit_char(c) => {
                            token.kind = TokenKind::Int;
                            state = State::IntLiteral;
                        }
                        '!' => {
                            token.kind = TokenKind::Bang;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '$' => {
                            token.kind = TokenKind::Dollar;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '&' => {
                            token.kind = TokenKind::Amp;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '(' => {
                            token.kind = TokenKind::LParen;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        ')' => {
                            token.kind = TokenKind::RParen;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        ':' => {
                            token.kind = TokenKind::Colon;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        ',' => {
                            token.kind = TokenKind::Comma;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '=' => {
                            token.kind = TokenKind::Eq;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '@' => {
                            token.kind = TokenKind::At;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '[' => {
                            token.kind = TokenKind::LBracket;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        ']' => {
                            token.kind = TokenKind::RBracket;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '{' => {
                            token.kind = TokenKind::LCurly;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '|' => {
                            token.kind = TokenKind::Pipe;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        '}' => {
                            token.kind = TokenKind::RCurly;
                            token.data = self.current_str();
                            return Ok(token);
                        }
                        c => {
                            return Err(Error::new(
                                    format!("Unexpected character \"{}\"", c),
                                    c.to_string(),
                                    ))
                        }
                    };
                }
                State::Ident => match c {
                    curr if is_ident_char(curr) || is_digit_char(curr) => {}
                    _ => {
                        token.data = self.prev_str();

                        state = State::Done;
                        break;
                    }
                },
                State::Whitespace => match c {
                    curr if is_whitespace(curr) => {}
                    _ => {
                        token.data = self.prev_str();

                        state = State::Done;
                        break;
                    }
                },
                State::BlockStringLiteral => match c {
                    '"' => {
                        if self.eatc('"') {
                            if self.eatc('"') {
                                token.data = self.current_str();

                                state = State::Done;
                                break;
                            }
                        }
                    }
                    curr if is_source_char(curr) => {}
                    _ => {
                        state = State::Done;
                        break;
                    },
                },
                State::StringLiteralStart => match c {
                    '"' => {
                        if self.eatc('"') {
                            state = State::BlockStringLiteral;

                            continue;
                        }

                        if self.pending() {
                            token.data = self.prev_str();
                        } else {
                            token.data = self.current_str();
                        }

                        state = State::Done;
                        break;
                    }
                    '\\' => {
                        state = State::StringLiteralBackslash;
                    }
                    _ => {
                        state = State::StringLiteral;

                        continue;
                    }
                },
                State::StringLiteral => match c {
                    '"' => {
                        token.data = self.current_str();

                        state = State::Done;
                        break;
                    }
                    curr if is_line_terminator(curr) => {
                        self.add_err(Error::new("unexpected line terminator", "".to_string()));
                    }
                    '\\' => {
                        state = State::StringLiteralBackslash;
                    }
                    curr if is_source_char(curr) => {}
                    _ => {
                        token.data = self.current_str();

                        state = State::Done;
                        break;
                    }
                },
                State::StringLiteralBackslash => match c {
                    curr if is_escaped_char(curr) => {
                        state = State::StringLiteral;
                    }
                    'u' => {
                        state = State::StringLiteral;
                    }
                    _ => {
                        self.add_err(Error::new("unexpected escaped character", c.to_string()));

                        state = State::StringLiteral;
                    }
                },
                State::IntLiteral => match c {
                    curr if is_digit_char(curr) => {}
                    '.' => {
                        token.kind = TokenKind::Float;
                        state = State::FloatLiteral;
                    }
                    'e' | 'E' => {
                        token.kind = TokenKind::Float;
                        state = State::ExponentLiteral;
                    }
                    _ => {
                        token.data = self.prev_str();

                        state = State::Done;
                        break;
                    }
                },
                State::FloatLiteral => match c {
                    curr if is_digit_char(curr) => {}
                    '.' => {
                        self.add_err(Error::new(
                                format!("Unexpected character `{}`", c),
                                c.to_string(),
                                ));

                        continue;
                    }
                    'e' | 'E' => {
                        state = State::ExponentLiteral;
                    }
                    _ => {
                        token.data = self.prev_str();

                        state = State::Done;
                        break;
                    }
                },
                State::ExponentLiteral => match c {
                    curr if is_digit_char(curr) => {
                        state = State::FloatLiteral;
                    }
                    '+' | '-' => {
                        state = State::FloatLiteral;
                    }
                    _ => {
                        let err = self.current_str();
                        return Err(Error::new(
                                format!("Unexpected character `{}`", err),
                                err.to_string(),
                                ));
                    }
                },
                State::SpreadOperator => match c {
                    '.' => {
                        if self.pending_len() == 2 {
                            token.data = self.current_str();
                            return Ok(token);
                        }
                    }
                    _ => {
                        let curr = self.current_str();
                        self.add_err(Error::new(
                                "Unterminated spread operator",
                                format!("{}", curr),
                                ))
                    }
                },
                State::PlusMinus => match c {
                    curr if is_digit_char(curr) => {
                        state = State::IntLiteral;
                    }
                    _ => {
                        let curr = self.current_str();
                        return Err(Error::new(
                                format!("Unexpected character `{}`", curr),
                                curr.to_string(),
                                ));
                    }
                },
                State::Comment => match c {
                    curr if is_line_terminator(curr) => {
                        token.data = self.prev_str();

                        state = State::Done;
                        break;
                    }
                    _ => {}
                },
                State::Done => unreachable!("must finalize loop when State::Done"),
            }
        }

        match state {
            State::Done => {
                if let Some(mut err) = self.err() {
                    err.data = token.data.to_string();
                    err.index = token.index;
                    self.err = None;

                    return Err(err);
                }

                Ok(token)
            }
            State::Start => {
                token.index += 1;
                return Ok(token);
            }
            State::StringLiteralStart => {
                let curr = self.current_str();

                return Err(Error::new(
                        "unexpected end of data while lexing string value",
                        curr.to_string(),
                ));
            }
            State::StringLiteral => {
                let curr = self.drain();

                return Err(Error::with_loc(
                        "unterminated string value",
                        curr.to_string(),
                        token.index,
                        ));
            }
            State::SpreadOperator => {
                let curr = self.current_str();
                return Err(Error::new(
                        "Unterminated spread operator",
                        format!("{}", curr),
                        ));
            }
            _ => {
                if let Some(mut err) = self.err() {
                    err.data = self.current_str().to_string();
                    return Err(err);
                }

                token.data = self.current_str();

                return Ok(token);
            }
        }
    }
}

fn is_whitespace(c: char) -> bool {
    // from rust's lexer:
    matches!(
        c,
        // ASCII
        '\u{0009}'   // \t
        | '\u{000A}' // \n
        | '\u{000B}' // vertical tab
        | '\u{000C}' // form feed
        | '\u{000D}' // \r
        | '\u{0020}' // space

        // Unicode BOM (Byte Order Mark)
        | '\u{FEFF}'

        // NEXT LINE from latin1
        | '\u{0085}'

        // Bidi markers
        | '\u{200E}' // LEFT-TO-RIGHT MARK
        | '\u{200F}' // RIGHT-TO-LEFT MARK

        // Dedicated whitespace characters from Unicode
        | '\u{2028}' // LINE SEPARATOR
        | '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

fn is_ident_char(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '_')
}

fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r')
}

fn is_digit_char(c: char) -> bool {
    matches!(c, '0'..='9')
}

// EscapedCharacter
//     "  \  /  b  f  n  r  t
fn is_escaped_char(c: char) -> bool {
    matches!(c, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't')
}

// SourceCharacter
//     /[\u0009\u000A\u000D\u0020-\uFFFF]/
fn is_source_char(c: char) -> bool {
    matches!(c, '\t' | '\r' | '\n' | '\u{0020}'..='\u{FFFF}')
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tests() {
        let gql_1 = "\"\nhello";
        let lexer_1 = Lexer::new(gql_1);
        dbg!(lexer_1.tokens);
        dbg!(lexer_1.errors);
    }
}

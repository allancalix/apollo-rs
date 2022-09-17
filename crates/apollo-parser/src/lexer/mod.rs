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

        let mut c = Cursor::new(input);
        loop {
            let r = c.new_advance();

            match r {
                Ok(token) => match token.kind() {
                    TokenKind::Eof => {
                        tokens.push(token);
                        break;
                    }
                    _ => tokens.push(token),
                },
                Err(e) => errors.push(e),
            }
        }

        Self { tokens, errors }
    }

    /// Take ownership of lexer parts.
    pub fn into_parts(self) -> (Vec<Token<'a>>, Vec<Error>) {
        (self.tokens, self.errors)
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

impl<'a> Cursor<'a> {
    fn new_advance(&mut self) -> Result<Token<'a>, Error> {
        #[derive(Debug)]
        enum State {
            Start,
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
        let mut token = Token::new(TokenKind::Eof, "EOF");

        token.index = self.index();
        loop {
            let c = match self.bump() {
                Some(c) => c,
                None => match state {
                    State::Start => {
                        token.index += 1;
                        return Ok(token);
                    }
                    State::StringLiteral | State::StringLiteralStart => {
                        return Err(Error::new(
                            "unexpected end of data while lexing string value",
                            "\"".to_string(),
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
                },
            };

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
                        c => return Err(Error::new(format!("Unexpected character \"{}\"", c), c.to_string())),
                    };
                }
                State::Ident => match c {
                    curr if is_ident_char(curr) || is_digit_char(curr) => {}
                    _ => {
                        token.data = self.prev_str();

                        break;
                    }
                },
                State::Whitespace => match c {
                    curr if is_whitespace(curr) => {}
                    _ => {
                        token.data = self.prev_str();

                        break;
                    }
                },
                State::BlockStringLiteral => match c {
                    '"' => {
                        if self.eatc('"') {
                            if self.eatc('"') {
                                token.data = self.current_str();

                                break;
                            }

                        }
                    }
                    curr if is_source_char(curr) => {},
                    _ => break,
                }
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

                        break;
                    },
                    '\\' => {
                        state = State::StringLiteralBackslash;
                    }
                    _ => {
                        state = State::StringLiteral;

                        continue;
                    }
                }
                State::StringLiteral => match c {
                    '"' => {
                        token.data = self.current_str();

                        break;
                    }
                    curr if is_line_terminator(curr) => {
                        self.drain();

                        token.data = self.prev_str();
                        self.add_err(Error::new("unterminated string value", "".to_string()));

                        break;
                    }
                    '\\' => {
                        state = State::StringLiteralBackslash;
                    }
                    curr if is_source_char(curr) => {}
                    _ => {
                        token.data = self.current_str();

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
                        token.data = self.current_str();

                        break;
                    }
                    _ => {}
                },
            }
        }

        if let Some(mut err) = self.err() {
            err.data = token.data.to_string();
            err.index = token.index;
            self.err = None;

            return Err(err);
        }

        Ok(token)
    }
}

impl Cursor<'_> {
    // fn string_value(&mut self) -> Result<Token, Error> {
    //     let c = match self.bump() {
    //         None => {
    //             return Err(Error::new(
    //                 "unexpected end of data while lexing string value",
    //                 "\"".to_string(),
    //             ));
    //         }
    //         Some(c) => c,
    //     };

    //     match c {
    //         '"' => self.block_string_value(c),
    //         t => {
    //             let mut was_backslash = t == '\\';

    //             while !self.is_eof() {
    //                 let c = self.bump().unwrap();

    //                 if was_backslash && !is_escaped_char(c) && c != 'u' {
    //                     self.add_err(Error::new("unexpected escaped character", c.to_string()));
    //                 }

    //                 if c == '"' {
    //                     if !was_backslash {
    //                         break;
    //                     }
    //                 } else if is_escaped_char(c)
    //                     || is_source_char(c) && c != '\\' && c != '"' && !is_line_terminator(c)
    //                 {
    //                     // buf.push(c);
    //                     // TODO @lrlna: this should error if c == \ or has a line terminator
    //                 } else {
    //                     break;
    //                 }
    //                 was_backslash = c == '\\';
    //             }

    //             if !self.current_str().ends_with('"') {
    //                 // If it's an unclosed string then take all remaining tokens into this string value
    //                 while !self.is_eof() {
    //                     self.bump().unwrap();
    //                 }
    //                 self.add_err(Error::new(
    //                     "unterminated string value",
    //                     self.current_str().to_string(),
    //                 ));
    //             }

    //             if let Some(mut err) = self.err() {
    //                 err.data = self.current_str().to_string();
    //                 return Err(err);
    //             }

    //             Ok(Token::new(
    //                 TokenKind::StringValue,
    //                 self.current_str().to_string(),
    //             ))
    //         }
    //     }
    // }

    // fn block_string_value(&mut self, char: char) -> Result<Token, Error> {
    //     let c = match self.bump() {
    //         None => {
    //             return Ok(Token::new(
    //                 TokenKind::StringValue,
    //                 self.current_str().to_string(),
    //             ));
    //         }
    //         Some(c) => c,
    //     };

    //     if let first_char @ '"' = c {
    //         while !self.is_eof() {
    //             let c = self.bump().unwrap();
    //             if c == '"' {
    //                 if ('"', '"') == (self.first(), self.second()) {
    //                     self.bump();
    //                     self.bump();
    //                     break;
    //                 }
    //             } else if is_source_char(c) {
    //                 // buf.push(c);
    //             } else {
    //                 break;
    //             }
    //         }
    //     }

    //     Ok(Token::new(
    //         TokenKind::StringValue,
    //         self.current_str().to_string(),
    //     ))
    // }

    fn comment(&mut self) -> Result<Token, Error> {
        while !self.is_eof() {
            let first = self.bump().unwrap();
            if !is_line_terminator(first) {
                continue;
            } else {
                break;
            }
        }

        Ok(Token::new(TokenKind::Comment, self.current_str()))
    }

    // fn spread_operator(&mut self, first_char: char) -> Result<Token, Error> {
    //     let mut buf = String::new();
    //     buf.push(first_char);

    //     match (self.first(), self.second()) {
    //         ('.', '.') => {
    //             buf.push('.');
    //             buf.push('.');
    //             self.bump();
    //             self.bump();
    //         }
    //         (a, b) => self.add_err(Error::new(
    //             "Unterminated spread operator",
    //             format!(".{}{}", a, b),
    //         )),
    //     }

    //     if let Some(mut err) = self.err() {
    //         err.data = buf;
    //         return Err(err);
    //     }

    //     Ok(Token::new(TokenKind::Spread, buf))
    // }

    fn whitespace(&mut self) -> Result<Token, Error> {
        while !self.is_eof() {
            let first = self.first();
            if is_whitespace(first) {
                self.bump().unwrap();
                continue;
            } else {
                break;
            }
        }

        Ok(Token::new(TokenKind::Whitespace, self.current_str()))
    }

    fn ident(&mut self) -> Result<Token, Error> {
        while !self.is_eof() {
            let first = self.first();
            if is_ident_char(first) || is_digit_char(first) {
                self.bump().unwrap();
            } else {
                break;
            }
        }

        Ok(Token::new(TokenKind::Name, self.current_str()))
    }

    fn number(&mut self) -> Result<Token, Error> {
        let mut has_exponent = false;
        let mut has_fractional = false;
        let mut has_digit = is_digit_char(self.first());

        while !self.is_eof() {
            let first = self.first();
            match first {
                'e' | 'E' => {
                    self.bump();
                    if !has_digit {
                        self.add_err(Error::new(
                            format!("Unexpected character `{}` in exponent", first),
                            first.to_string(),
                        ));
                    }
                    if has_exponent {
                        self.add_err(Error::new(
                            format!("Unexpected character `{}`", first),
                            first.to_string(),
                        ));
                    }
                    has_exponent = true;
                    if matches!(self.first(), '+' | '-') {
                        self.bump();
                    }
                }
                '.' => {
                    self.bump();

                    if !has_digit {
                        self.add_err(Error::new(
                            format!("Unexpected character `{}` before a digit", first),
                            first.to_string(),
                        ));
                    }

                    if has_fractional {
                        self.add_err(Error::new(
                            format!("Unexpected character `{}`", first),
                            first.to_string(),
                        ));
                    }

                    if has_exponent {
                        self.add_err(Error::new(
                            format!("Unexpected character `{}`", first),
                            first.to_string(),
                        ));
                    }

                    has_fractional = true;
                }
                first if is_digit_char(first) => {
                    self.bump();
                    has_digit = true;
                }
                _ => break,
            }
        }

        if let Some(mut err) = self.err() {
            err.data = self.current_str().to_string();
            return Err(err);
        }

        if has_exponent || has_fractional {
            Ok(Token::new(TokenKind::Float, self.current_str()))
        } else {
            Ok(Token::new(TokenKind::Int, self.current_str()))
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
        let gql_1 = r#"
        """
        **Example**: "Saturn5"
        """
        name: String @join__field(graph: PRODUCTS)
        "#;
        let lexer_1 = Lexer::new(gql_1);
        dbg!(lexer_1.tokens);
        dbg!(lexer_1.errors);
    }
}

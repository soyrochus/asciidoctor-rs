/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Return the tokens from an asciidoctor text.

use std::char;
use std::io::Read;

use error::ErrorKind::{Eof, UnexpectedChar};
use error::Result;
use position::Pos;
use token::Token;
use token::Token::*;

const BUFFER_SIZE: usize = 4096;

pub struct Lexer<R: Read> {
    buffer: [u8; BUFFER_SIZE],
    buffer_index: usize,
    buffer_size: usize,
    column: usize,
    line: usize,
    reader: R,
}

impl<R: Read> Lexer<R> {
    /// Create a new parser from a `Reader`.
    /// This is an iterator over the tokens.
    pub fn new(reader: R) -> Self {
        Lexer {
            buffer: [0; BUFFER_SIZE],
            buffer_index: BUFFER_SIZE,
            buffer_size: 0,
            column: 1,
            line: 1,
            reader,
        }
    }

    /// Advance the internal position cursor.
    fn advance(&mut self, actual: u8) {
        self.buffer_index += 1;
        if actual == b'\n' {
            self.line += 1;
            self.column = 1;
        }
        else {
            self.column += 1;
        }
    }

    /// Advance until the end of the line.
    fn advance_to_eol(&mut self) -> Result<()> {
        self.advance_while(|c| c != b'\n')
    }

    /// Advance while the predicate is true.
    fn advance_while<F: Fn(u8) -> bool>(&mut self, predicate: F) -> Result<()> {
        loop {
            self.read_if_needed()?;
            let actual = self.current_char()?;
            if !predicate(actual) {
                break;
            }
            self.advance(actual);
        }
        Ok(())
    }

    /// Parse (and ignore) a comment.
    fn comment(&mut self) -> Result<()> {
        self.eat(b'/')?;
        self.eat(b'/')?;

        // Try to parse a multiline comment.
        if self.current_char()? == b'/' {
            self.eat(b'/');
            self.eat(b'/');

            let comment_delim = b"////";
            while &self.buffer[self.buffer_index..self.buffer_index + comment_delim.len()] != comment_delim {
                self.eat(b'\n');
                self.advance_to_eol()?;
            }
        }
        else {
            // Single comment.
            self.advance_to_eol()?;
        }
        Ok(())
    }

    /// Get the current character (filling the buffer if needed).
    fn current_char(&mut self) -> Result<u8> {
        self.read_if_needed()?;
        Ok(self.buffer[self.buffer_index])
    }

    /// Eat the next character if it is the one specified in the parameter.
    fn eat(&mut self, expected: u8) -> Result<()> {
        self.read_if_needed()?;
        let actual = self.current_char()?;
        if actual == expected {
            self.advance(actual);
            Ok(())
        }
        else {
            bail!(UnexpectedChar {
                actual,
                expected: vec![expected],
                pos: self.pos(),
            })
        }
    }

    /// Parse a new line.
    fn newline(&mut self) -> Result<Token> {
        self.eat(b'\n')?;
        Ok(NewLine)
    }

    /// Parse a number sign.
    fn number_sign(&mut self) -> Result<Token> {
        self.eat(b'#')?;
        Ok(NumberSign)
    }

    /// Get the current position in the file.
    fn pos(&self) -> Pos {
        Pos::new(self.line, self.column)
    }

    /// Read from the buffer if needed.
    fn read_if_needed(&mut self) -> Result<()> {
        if self.buffer_index >= self.buffer_size {
            self.buffer_size = self.reader.read(&mut self.buffer)?;
            if self.buffer_size == 0 {
                bail!(Eof);
            }
            self.buffer_index = 0;
        }
        Ok(())
    }

    /// Parse a space.
    fn space(&mut self) -> Result<Token> {
        self.eat(b' ')?;
        Ok(Space)
    }

    /// Get the next token from the file.
    fn token(&mut self) -> Result<Token> {
        self.read_if_needed()?;
        let actual = self.current_char()?;
        match actual {
            b'/' => {
                self.comment()?;
                self.token()
            },
            b'<' => self.triple_lt(),
            b'\'' => self.triple_apos(),
            b'\n' => self.newline(),
            b'\r' => {
                self.advance(actual);
                self.token()
            },
            b'#' => self.number_sign(),
            b' ' => self.space(),
            _ => self.word(),
        }
    }

    /// Parse three '.
    fn triple_apos(&mut self) -> Result<Token> {
        self.eat(b'\'')?;
        self.eat(b'\'')?;
        self.eat(b'\'')?;
        Ok(TripleApos)
    }

    /// Parse three <.
    fn triple_lt(&mut self) -> Result<Token> {
        self.eat(b'<')?;
        self.eat(b'<')?;
        self.eat(b'<')?;
        Ok(TripleLt)
    }

    /// Parse a word.
    fn word(&mut self) -> Result<Token> {
        let start_index = self.buffer_index;
        self.advance_while(|c| !b" *_`#[^~:\n\r\t".contains(&c))?;
        if self.buffer_index == start_index {
            bail!("bug in the parser, next character `{}` is not part of a word token",
                  char::from_u32(self.current_char()? as u32)
                      .ok_or("byte is not a character")?)
        }
        Ok(Word(self.buffer[start_index..self.buffer_index].to_vec()))
    }
}

impl<R: Read> Iterator for Lexer<R> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.token())
    }
}

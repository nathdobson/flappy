use crate::lookahead::Lookahead;
use crate::token::{
    IdentToken, Keyword, KeywordToken, Location, NumberToken, Symbol, SymbolToken, Token,
};
use core::fmt::{Debug, Formatter};
use core::iter::Peekable;
use core::str::CharIndices;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum LexerError<'src> {
    UnexpectedEof,
    UnexpectedChar(char),
    BadToken(&'src str),
}

pub struct Lexer<'src> {
    src: &'src str,
    iter: Lookahead<2, CharIndices<'src>>,
    loc: Location,
}

pub struct TokenReader<'lexer, 'src> {
    lexer: &'lexer mut Lexer<'src>,
    origin_loc: Location,
    origin_cursor: usize,
}

fn char_is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl<'lexer, 'src> TokenReader<'lexer, 'src> {
    pub fn peek(&mut self, index: usize) -> Result<char, LexerError<'src>> {
        Ok(self
            .lexer
            .iter
            .peek(index)
            .ok_or(LexerError::UnexpectedEof)?
            .1)
    }
    pub fn try_peek(&mut self, index: usize) -> Option<char> {
        Some(self.lexer.iter.peek(index)?.1)
    }
    pub fn next(&mut self) -> Result<char, LexerError<'src>> {
        let c = self.lexer.iter.next().ok_or(LexerError::UnexpectedEof)?.1;
        if c == '\n' {
            self.lexer.loc.column = 1;
            self.lexer.loc.line += 1;
        } else {
            self.lexer.loc.column += 1;
        }
        Ok(c)
    }
    fn into_str(self) -> &'src str {
        &self.lexer.src[self.origin_cursor..self.lexer.cursor()]
    }
    fn read_numeric(mut self) -> Result<Token<'src>, LexerError<'src>> {
        loop {
            let c = self.peek(0)?;
            if c == '.' {
                if self.try_peek(1) == Some('.') {
                    break;
                } else {
                    self.next()?;
                }
            } else if c.is_ascii_alphanumeric() || c == '_' {
                self.next()?;
            } else {
                break;
            }
        }
        let loc = self.origin_loc;
        Ok(Token::Number(NumberToken {
            number: self.into_str(),
            loc,
        }))
    }
    fn read_ident(mut self) -> Result<Token<'src>, LexerError<'src>> {
        loop {
            let c = self.peek(0)?;
            if char_is_ident(c) {
                self.next()?;
            } else {
                let loc = self.origin_loc;
                let str = self.into_str();
                let keyword = match str {
                    "let" => Keyword::Let,
                    "fn" => Keyword::Fn,
                    "for" => Keyword::For,
                    "in" => Keyword::In,
                    _ => {
                        return Ok(Token::Ident(IdentToken { ident: str, loc }));
                    }
                };
                return Ok(Token::Keyword(KeywordToken { keyword, loc }));
            }
        }
    }
    fn read_symbol(mut self) -> Result<Token<'src>, LexerError<'src>> {
        let symbol = match self.peek(0)? {
            '.' => {
                self.next()?;
                match self.peek(0)? {
                    '.' => {
                        self.next()?;
                        Symbol::DotDot
                    }
                    c => return Err(LexerError::UnexpectedChar(c)),
                }
            }
            '+' => {
                self.next()?;
                match self.peek(0)? {
                    '+' => {
                        self.next()?;
                        Symbol::PlusPlus
                    }
                    '=' => {
                        self.next()?;
                        Symbol::PlusEquals
                    }
                    _ => Symbol::Plus,
                }
            }
            '=' => {
                self.next()?;
                match self.peek(0)? {
                    '=' => {
                        self.next()?;
                        Symbol::EqualsEquals
                    }
                    _ => Symbol::Equals,
                }
            }
            ';' => {
                self.next()?;
                Symbol::Semi
            }
            '(' => {
                self.next()?;
                Symbol::LParen
            }
            ')' => {
                self.next()?;
                Symbol::RParen
            }
            '{' => {
                self.next()?;
                Symbol::LBrace
            }
            '}' => {
                self.next()?;
                Symbol::RBrace
            }
            c => return Err(LexerError::UnexpectedChar(c)),
        };
        Ok(Token::Symbol(SymbolToken {
            symbol,
            loc: self.origin_loc,
        }))
    }
    fn read_token(mut self) -> Result<Option<Token<'src>>, LexerError<'src>> {
        let c = self.peek(0)?;
        let token = if c.is_ascii_whitespace() {
            self.next()?;
            return Ok(None);
        } else if c.is_ascii_digit() {
            self.read_numeric()?
        } else if char_is_ident(c) {
            self.read_ident()?
        } else {
            self.read_symbol()?
        };
        Ok(Some(token))
    }
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Lexer<'src> {
        Lexer {
            src,
            iter: Lookahead::new(src.char_indices()),
            loc: Location { line: 1, column: 1 },
        }
    }
    fn cursor(&mut self) -> usize {
        if let Some((index, _)) = self.iter.peek(0) {
            *index
        } else {
            self.src.len()
        }
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Result<Token<'src>, LexerError<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.iter.peek(0).is_none() {
                return None;
            }
            let loc = self.loc;
            let cursor = self.cursor();
            let token = TokenReader {
                lexer: self,
                origin_loc: loc,
                origin_cursor: cursor,
            }
            .read_token();
            match token {
                Ok(Some(token)) => return Some(Ok(token)),
                Ok(None) => continue,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

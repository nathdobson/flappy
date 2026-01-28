use core::fmt::{Debug, Formatter};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Symbol {
    Plus,
    Minus,
    Times,
    Divide,
    PlusEquals,
    PlusPlus,
    EqualsEquals,
    Equals,
    Semi,
    LParen,
    RParen,
    Comma,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Keyword {
    Let,
    Fn,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct SymbolToken {
    pub symbol: Symbol,
    pub loc: Location,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct KeywordToken {
    pub keyword: Keyword,
    pub loc: Location,
}

#[derive(Eq, PartialEq)]
pub struct IdentToken<'src> {
    pub ident: &'src str,
    pub loc: Location,
}

#[derive(Eq, PartialEq)]
pub struct NumberToken<'src> {
    pub number: &'src str,
    pub loc: Location,
}

#[derive(Eq, PartialEq)]
pub enum Token<'src> {
    Ident(IdentToken<'src>),
    Number(NumberToken<'src>),
    Symbol(SymbolToken),
    Keyword(KeywordToken),
}

impl Debug for Symbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Symbol::Plus => "+",
                Symbol::Minus => "-",
                Symbol::Times => "*",
                Symbol::Divide => "/",
                Symbol::PlusEquals => "+=",
                Symbol::PlusPlus => "++",
                Symbol::EqualsEquals => "==",
                Symbol::Equals => "=",
                Symbol::Semi => ";",
                Symbol::LParen => "(",
                Symbol::RParen => ")",
                Symbol::Comma => ",",
            }
        )
    }
}

impl Debug for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl Debug for KeywordToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}({:?})", self.keyword, self.loc)
    }
}

impl<'src> Debug for IdentToken<'src> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}({:?})", self.ident, self.loc)
    }
}

impl<'src> Debug for NumberToken<'src> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}({:?})", self.number, self.loc)
    }
}

impl<'src> Debug for SymbolToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}({:?})", self.symbol, self.loc)
    }
}

impl<'src> Debug for Token<'src> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Token::Ident(t) => write!(f, "{:?}", t),
            Token::Number(t) => write!(f, "{:?}", t),
            Token::Symbol(t) => write!(f, "{:?}", t),
            Token::Keyword(t) => write!(f, "{:?}", t),
        }
    }
}

impl Debug for Keyword {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Keyword::Let => write!(f, "let"),
            Keyword::Fn => write!(f, "fn"),
        }
    }
}

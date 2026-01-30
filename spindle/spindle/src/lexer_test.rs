use crate::lexer::{Lexer, LexerError};
use crate::token::{IdentToken, Keyword, KeywordToken, NumberToken, Symbol, SymbolToken, Token};
use itertools::Itertools;
use std::assert_matches;

#[test]
fn test_lexer() {
    let code = r#"
        let foo = 2 + 2;
        print(foo);
    "#;
    let tokens = Lexer::new(code)
        .collect::<Result<Vec<Token>, LexerError>>()
        .unwrap();
    assert_matches!(
        &*tokens,
        [
            Token::Keyword(KeywordToken {
                keyword: Keyword::Let,
                ..
            }),
            Token::Ident(IdentToken { ident: "foo", .. }),
            Token::Symbol(SymbolToken {
                symbol: Symbol::Equals,
                ..
            }),
            Token::Number(NumberToken { number: "2", .. }),
            Token::Symbol(SymbolToken {
                symbol: Symbol::Plus,
                ..
            }),
            Token::Number(NumberToken { number: "2", .. }),
            Token::Symbol(SymbolToken {
                symbol: Symbol::Semi,
                ..
            }),
            Token::Ident(IdentToken { ident: "print", .. }),
            Token::Symbol(SymbolToken {
                symbol: Symbol::LParen,
                ..
            }),
            Token::Ident(IdentToken { ident: "foo", .. }),
            Token::Symbol(SymbolToken {
                symbol: Symbol::RParen,
                ..
            }),
            Token::Symbol(SymbolToken {
                symbol: Symbol::Semi,
                ..
            }),
        ]
    );
}

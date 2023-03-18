use logos::{Lexer, Logos};

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexicalError {
    #[default]
    NoMatchingRule,
    IDK((usize, char), String),
    UnterminatedStringLiteral(usize),
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(error = LexicalError, skip r"[ \t\r\n\f]+")]
pub enum Tok<'a> {
    /// The token `+`
    #[token("+")]
    Plus,
    /// The token `-`
    #[token("-")]
    Minus,
    /// The token `|`
    #[token("|")]
    Pipe,
    /// The token `&`
    #[token("&")]
    Ampersand,
    /// The token `(`
    #[token("(")]
    LeftParen,
    /// The token `)`
    #[token(")")]
    RightParen,

    /// String literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice())]
    #[regex(r#""([^"\\]|\\.)*"#, |lex| {
        Err(LexicalError::UnterminatedStringLiteral(lex.span().start))
    })]
    StringLiteral(&'a str),

    /// ID literals
    #[regex(r"[0-9]+", |lex| lex.slice())]
    IDLiteral(&'a str),

    /// User mentions
    #[regex(r"<@!?[0-9]+>", |lex| lex.slice())]
    UserMention(&'a str),

    /// Role mentions
    #[regex(r"<@&[0-9]+>", |lex| lex.slice())]
    RoleMention(&'a str),

    /// Raw names
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice())]
    RawName(&'a str),
}

pub struct DrqlLexer<'input> {
    lex: Lexer<'input, Tok<'input>>,
}

impl<'input> DrqlLexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            lex: Tok::lexer(input),
        }
    }
}

impl<'input> Iterator for DrqlLexer<'input> {
    type Item = Spanned<Tok<'input>, usize, LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.lex.next()?;
        let span = self.lex.span();
        let slice = self.lex.slice().to_string();
        match token {
            Err(LexicalError::NoMatchingRule) => {
                let char = slice.chars().next().unwrap();
                Some(Err(LexicalError::IDK(
                    (span.start, char),
                    format!("Internal error: Unknown token '{char}'"),
                )))
            }
            Err(e) => Some(Err(e)),
            Ok(token) => Some(Ok((span.start, token, span.end))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_works_as_expected() {
        let lexer = DrqlLexer::new("t e s t");
        let tokens: Vec<_> = lexer.map(|x| x.unwrap().1).collect();
        assert_eq!(
            tokens,
            vec![
                Tok::RawName("t"),
                Tok::RawName("e"),
                Tok::RawName("s"),
                Tok::RawName("t"),
            ]
        );
    }

    #[test]
    fn lexer_unknown_token() {
        let lexer = DrqlLexer::new("a #");
        let results: Vec<_> = lexer.collect();
        assert_eq!(
            results,
            vec![
                Ok((0, Tok::RawName("a"), 1)),
                Err(LexicalError::IDK(
                    (2, '#'),
                    "Internal error: Unknown token '#'".to_string()
                )),
            ]
        );
    }
}

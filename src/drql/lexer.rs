use logos::{Lexer, Logos};
use std::num::ParseIntError;

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexicalError {
    #[default]
    NoMatchingRule,
    UnknownToken((usize, char)),
    UnterminatedStringLiteral(usize),
    ParseIntError(ParseIntError),
}
impl From<ParseIntError> for LexicalError {
    fn from(value: ParseIntError) -> Self {
        LexicalError::ParseIntError(value)
    }
}
impl std::fmt::Display for LexicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexicalError::NoMatchingRule => write!(f, "No matching rule."),
            LexicalError::UnknownToken((index, ch)) => {
                write!(f, "Unknown token at index {index}: `{ch}`")
            }
            LexicalError::UnterminatedStringLiteral(index) => {
                write!(f, "Unterminated string literal at index {index}")
            }
            LexicalError::ParseIntError(e) => write!(f, "ParseIntError: {e}"),
        }
    }
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(error = LexicalError, skip r"[ \t\r\n\f]+")]
pub enum Tok {
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

    /// String literals: `"abc def"`, `abc`, `everyone`, `here`, etc
    /// From issue #25, `@everyone` and `@here` (the exact strings, which are the mentions)
    /// are treated as `everyone` and `here`.
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice()[1..(lex.slice().len()-1)].to_string())]
    #[regex(r#""([^"\\]|\\.)*"#, |lex| {
        Err(LexicalError::UnterminatedStringLiteral(lex.span().start))
    })]
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    #[token("@everyone", |lex| lex.slice()[1..].to_string())]
    #[token("@here", |lex| lex.slice()[1..].to_string())]
    StringLiteral(String),

    /// ID literals
    #[regex(r"[0-9]+", |lex| lex.slice().to_string())]
    IDLiteral(String),

    /// User mentions
    #[regex(r"<@!?([0-9]+)>", |lex| {
        let s = lex.slice();

        if s[2..3] == *"!" {
            s[3..(s.len()-1)].to_string()
        } else {
            s[2..(s.len()-1)].to_string()
        }
    })]
    UserMention(String),

    /// Role mentions
    #[regex(r"<@&[0-9]+>", |lex| lex.slice()[3..(lex.slice().len()-1)].to_string())]
    RoleMention(String),
}

impl std::fmt::Display for Tok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tok::Plus => write!(f, "+"),
            Tok::Minus => write!(f, "-"),
            Tok::Pipe => write!(f, "|"),
            Tok::Ampersand => write!(f, "&"),
            Tok::LeftParen => write!(f, "("),
            Tok::RightParen => write!(f, ")"),
            Tok::StringLiteral(s) => write!(f, "\"{s}\""),
            Tok::IDLiteral(s) => write!(f, "{s}"),
            Tok::UserMention(s) => write!(f, "<@{s}>"),
            Tok::RoleMention(s) => write!(f, "<@&{s}>"),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct DrqlLexer<'input> {
    lex: Lexer<'input, Tok>,
}

impl<'input> DrqlLexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            lex: Tok::lexer(input),
        }
    }
}

impl<'input> Iterator for DrqlLexer<'input> {
    type Item = Spanned<Tok, usize, LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.lex.next()?;
        let span = self.lex.span();
        let slice = self.lex.slice().to_string();
        match token {
            Err(LexicalError::NoMatchingRule) => {
                let char = slice.chars().next().unwrap();
                Some(Err(LexicalError::UnknownToken((span.start, char))))
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
                Tok::StringLiteral("t".to_string()),
                Tok::StringLiteral("e".to_string()),
                Tok::StringLiteral("s".to_string()),
                Tok::StringLiteral("t".to_string()),
            ]
        );
    }

    // Issue #25
    #[test]
    fn lexer_mention_everyone_works_as_expected() {
        let lexer = DrqlLexer::new("everyone here @everyone @here");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok((0, Tok::StringLiteral("everyone".to_string()), 8)),
                Ok((9, Tok::StringLiteral("here".to_string()), 13)),
                Ok((14, Tok::StringLiteral("everyone".to_string()), 23)),
                Ok((24, Tok::StringLiteral("here".to_string()), 29))
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
                Ok((0, Tok::StringLiteral("a".to_string()), 1)),
                Err(LexicalError::UnknownToken((2, '#'))),
            ]
        );
    }

    #[test]
    fn lexer_token_slices() {
        let lexer = DrqlLexer::new("abc + \"def\" + <@123> + 456 + <@&789> + <@!111>");
        let tokens: Vec<_> = lexer.map(|x| x.unwrap().1).collect();
        assert_eq!(
            tokens,
            vec![
                Tok::StringLiteral("abc".to_string()),
                Tok::Plus,
                Tok::StringLiteral("def".to_string()),
                Tok::Plus,
                Tok::UserMention("123".to_string()),
                Tok::Plus,
                Tok::IDLiteral("456".to_string()),
                Tok::Plus,
                Tok::RoleMention("789".to_string()),
                Tok::Plus,
                Tok::UserMention("111".to_string()),
            ]
        );
    }
}

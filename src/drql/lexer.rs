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

    /// String literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice()[1..(lex.slice().len()-1)].to_string())]
    #[regex(r#""([^"\\]|\\.)*"#, |lex| {
        Err(LexicalError::UnterminatedStringLiteral(lex.span().start))
    })]
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

    /// Special mentions
    #[token("@everyone")]
    Everyone,
    #[token("@here")]
    Here,

    /// Raw names
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    RawName(String),
}

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
                Tok::RawName("t".to_string()),
                Tok::RawName("e".to_string()),
                Tok::RawName("s".to_string()),
                Tok::RawName("t".to_string()),
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
                Ok((0, Tok::RawName("a".to_string()), 1)),
                Err(LexicalError::IDK(
                    (2, '#'),
                    "Internal error: Unknown token '#'".to_string()
                )),
            ]
        );
    }

    #[test]
    fn lexer_token_slices() {
        let lexer = DrqlLexer::new("abc + \"def\" + <@123> + 456 + <@&789> + <@!111> + @here");
        let tokens: Vec<_> = lexer.map(|x| x.unwrap().1).collect();
        assert_eq!(
            tokens,
            vec![
                Tok::RawName("abc".to_string()),
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
                Tok::Plus,
                Tok::Here,
            ]
        );
    }
}

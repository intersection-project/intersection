use std::fmt::Debug;
use std::fmt::Display;

use super::ast;
use super::lexer;
use crate::parser;
use lalrpop_util::{ErrorRecovery, ParseError};

#[derive(Debug, PartialEq)]
pub enum DrqlParserError<T> {
    /// An error we were able to recover from
    Recoverable {
        errors: Vec<ErrorRecovery<usize, lexer::Tok, lexer::LexicalError>>,
        partial: T,
    },
    /// An error that stopped the parser.
    Fatal(ParseError<usize, lexer::Tok, lexer::LexicalError>),
}

impl<T> Display for DrqlParserError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrqlParserError::Recoverable { errors, .. } => {
                for error in errors {
                    writeln!(f, "{:?}", error)?;
                }
                Ok(())
            }
            DrqlParserError::Fatal(e) => write!(f, "{:?}", e),
        }
    }
}

impl<T> std::error::Error for DrqlParserError<T> where T: Debug {}

/// Parse a DRQL expression with the DRQL parser.
pub fn parse_drql(input: &str) -> Result<ast::Expr, DrqlParserError<ast::Expr>> {
    let mut errors = Vec::new();
    let result = parser::ExprParser::new().parse(&mut errors, lexer::DrqlLexer::new(input));
    match result {
        Err(e) => Err(DrqlParserError::Fatal(e)),
        Ok(v) if errors.is_empty() => Ok(v),
        Ok(v) => Err(DrqlParserError::Recoverable { errors, partial: v }),
    }
}

#[cfg(test)]
mod tests {
    use crate::drql::ast::Expr;

    use super::*;

    #[test]
    fn many_token_types() {
        assert_eq!(
            parse_drql(concat!(
                "raw_name\n",
                "  + \"string literal\"\n",
                "  + <@1>\n",
                "  + <@!2>\n",
                "  + <@&3>\n",
                "  + 4\n",
            )),
            Ok(Expr::Union(
                Box::new(Expr::Union(
                    Box::new(Expr::Union(
                        Box::new(Expr::Union(
                            Box::new(Expr::Union(
                                Box::new(Expr::StringLiteral("raw_name".to_string())),
                                Box::new(Expr::StringLiteral("string literal".to_string()))
                            )),
                            Box::new(Expr::UserID("1".to_string()))
                        )),
                        Box::new(Expr::UserID("2".to_string()))
                    )),
                    Box::new(Expr::RoleID("3".to_string()))
                )),
                Box::new(Expr::UnknownID("4".to_string()))
            ))
        );
    }
}

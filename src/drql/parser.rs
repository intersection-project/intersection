//! Wrapper for the DRQL parser

use lalrpop_util::ParseError;
use tap::Tap;
use tracing::{debug, instrument};

use super::{ast, lexer};
use crate::parser;

/// Parse a DRQL expression with the DRQL parser.
#[instrument]
pub fn parse_drql(
    input: &str,
) -> Result<ast::Expr, ParseError<usize, lexer::Tok, lexer::LexicalError>> {
    parser::ExprParser::new()
        .parse(lexer::DrqlLexer::new(input))
        .tap(|ast| debug!("Parser result: {ast:?}"))
}

#[cfg(test)]
mod tests {
    use poise::serenity_prelude::model::prelude::{RoleId, UserId};

    use super::*;
    use crate::drql::ast::Expr;

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
                            Box::new(Expr::UserID(UserId(1)))
                        )),
                        Box::new(Expr::UserID(UserId(2)))
                    )),
                    Box::new(Expr::RoleID(RoleId(3)))
                )),
                Box::new(Expr::UnknownID("4".to_string()))
            ))
        );
    }
}

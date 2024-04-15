//! Utilities and functions for interpreting DRQL queries

use std::collections::HashSet;

use async_recursion::async_recursion;
use poise::{
    async_trait,
    serenity_prelude::{RoleId, UserId},
};
use tracing::instrument;

use super::ast::Expr;

/// Describes a set of functions used to resolve values in [interpret].
#[allow(clippy::module_name_repetitions)]
#[async_trait]
pub trait InterpreterResolver<E> {
    /// Resolve a role name to the HashSet of its members
    async fn resolve_string_literal(&mut self, literal: String) -> Result<HashSet<UserId>, E>;
    /// Resolve an ID to the HashSet of its members
    async fn resolve_unknown_id(&mut self, id: String) -> Result<HashSet<UserId>, E>;
    /// Resolve a user ID to the HashSet of just its ID
    async fn resolve_user_id(&mut self, id: UserId) -> Result<HashSet<UserId>, E>;
    /// Resolve a role ID to the HashSet of its members
    async fn resolve_role_id(&mut self, id: RoleId) -> Result<HashSet<UserId>, E>;
}

/// Interpret a DRQL AST, deferring to the Resolver to resolve string literals, user IDs, and role IDs.
#[async_recursion]
#[instrument(skip_all, fields(node = %node))]
#[allow(clippy::multiple_bound_locations)]
pub async fn interpret<E: Send>(
    node: Expr,
    resolver: &mut (impl InterpreterResolver<E> + Send),
) -> Result<HashSet<UserId>, E> {
    Ok(match node {
        Expr::Difference(lhs, rhs) => interpret(*lhs, resolver)
            .await?
            .difference(&interpret(*rhs, resolver).await?)
            .copied()
            .collect::<HashSet<_>>(),
        Expr::Intersection(lhs, rhs) => interpret(*lhs, resolver)
            .await?
            .intersection(&interpret(*rhs, resolver).await?)
            .copied()
            .collect::<HashSet<_>>(),
        Expr::Union(lhs, rhs) => interpret(*lhs, resolver)
            .await?
            .union(&interpret(*rhs, resolver).await?)
            .copied()
            .collect::<HashSet<_>>(),

        Expr::StringLiteral(contents) => resolver.resolve_string_literal(contents).await?,
        Expr::UnknownID(id) => resolver.resolve_unknown_id(id).await?,
        Expr::UserID(id) => resolver.resolve_user_id(id).await?,
        Expr::RoleID(id) => resolver.resolve_role_id(id).await?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod basic_cases {
        use anyhow::anyhow;

        use super::*;

        // In this case, the resolver uses some basic predefined values.
        struct Resolver;
        #[async_trait]
        impl InterpreterResolver<anyhow::Error> for Resolver {
            async fn resolve_string_literal(
                &mut self,
                contents: String,
            ) -> Result<HashSet<UserId>, anyhow::Error> {
                if contents == "test_ok_case" {
                    Ok(HashSet::from([UserId(1)]))
                } else {
                    Err(anyhow!("error case 1"))
                }
            }

            async fn resolve_unknown_id(
                &mut self,
                id: String,
            ) -> Result<HashSet<UserId>, anyhow::Error> {
                if id == "0" {
                    Ok(HashSet::from([UserId(2)]))
                } else {
                    Err(anyhow!("error case 2"))
                }
            }

            async fn resolve_user_id(
                &mut self,
                id: UserId,
            ) -> Result<HashSet<UserId>, anyhow::Error> {
                if id.0 == 0 {
                    Ok(HashSet::from([UserId(3)]))
                } else {
                    Err(anyhow!("error case 3"))
                }
            }

            async fn resolve_role_id(
                &mut self,
                id: RoleId,
            ) -> Result<HashSet<UserId>, anyhow::Error> {
                if id.0 == 0 {
                    Ok(HashSet::from([UserId(4)]))
                } else {
                    Err(anyhow!("error case 4"))
                }
            }
        }

        #[tokio::test]
        async fn union_ok_case() {
            assert_eq!(
                interpret(
                    Expr::Union(
                        Box::new(Expr::StringLiteral("test_ok_case".to_string())),
                        Box::new(Expr::Union(
                            Box::new(Expr::UnknownID("0".to_string())),
                            Box::new(Expr::Union(
                                Box::new(Expr::UserID(UserId(0))),
                                Box::new(Expr::RoleID(RoleId(0)))
                            ))
                        ))
                    ),
                    &mut Resolver {}
                )
                .await
                .expect("interpret should not fail"),
                HashSet::from([UserId(1), UserId(2), UserId(3), UserId(4)])
            );
        }

        #[tokio::test]
        async fn errors_bubble() {
            assert!(interpret(
                Expr::Union(
                    Box::new(Expr::StringLiteral("7".to_string())),
                    Box::new(Expr::StringLiteral("test_ok_case".to_string())),
                ),
                &mut Resolver {},
            )
            .await
            .is_err());
        }
    }
}

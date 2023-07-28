//! DRQL's Abstract Syntax Tree

use std::fmt::{Display, Formatter};

use poise::serenity_prelude::model::prelude::{RoleId, UserId};

/// Represents a single DRQL query, or a view into that query
#[derive(Debug, PartialEq)]
pub enum Expr {
    /// Represents the union of two expressions, `a + b` or `a | b`
    Union(Box<Expr>, Box<Expr>),
    /// Represents the intersection of two expressions, `a & b`
    Intersection(Box<Expr>, Box<Expr>),
    /// Represents the difference between two expressions, `a - b`
    Difference(Box<Expr>, Box<Expr>),

    /// The name of a role itself, like `everyone`
    StringLiteral(String),
    /// Some ID. It could belong to a user or role.
    UnknownID(String),
    /// An ID that's guaranteed to belong to a role.
    ///
    /// This is generated when a role is mentioned directly in a query.
    UserID(UserId),
    /// An ID that's guaranteed to belong to a user.
    ///
    /// This is generated when a user is mentioned directly in a query.
    RoleID(RoleId),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Union(l, r) => write!(f, "({l} | {r})"),
            Self::Intersection(l, r) => write!(f, "({l} & {r})"),
            Self::Difference(l, r) => write!(f, "({l} - {r})"),

            Self::StringLiteral(l) => {
                if l.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    write!(f, "{l}")
                } else {
                    write!(f, "\"{l}\"")
                }
            }
            Self::UnknownID(id) => write!(f, "{id}"),
            Self::UserID(id) => write!(f, "<@{id}>"),
            Self::RoleID(id) => write!(f, "<@&{id}>"),
        }
    }
}

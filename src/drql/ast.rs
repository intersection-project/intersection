//! Abstract Syntax Tree enum for DRQL

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

use serenity::model::prelude::{RoleId, UserId};

#[derive(Debug, PartialEq)]
pub enum Expr {
    Union(Box<Expr>, Box<Expr>),
    Intersection(Box<Expr>, Box<Expr>),
    Difference(Box<Expr>, Box<Expr>),

    StringLiteral(String),
    UnknownID(String),
    UserID(UserId),
    RoleID(RoleId),
}

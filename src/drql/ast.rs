#[derive(Debug, PartialEq)]
pub enum Expr {
    Union(Box<Expr>, Box<Expr>),
    Intersection(Box<Expr>, Box<Expr>),
    Difference(Box<Expr>, Box<Expr>),

    StringLiteral(String),
    UnknownID(String),
    UserID(String),
    RoleID(String),
}

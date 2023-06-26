//! Structures representing Discord @-mentions

/// A mention like <@123> or @everyone
#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum Mention {
    /// Specifically mentioning a user: `<@ID>` or <@!ID>`
    User(poise::serenity_prelude::UserId),
    /// Mentioning in the form @everyone, @here, or `<&ID>`
    Role(RoleType),
}

impl std::fmt::Display for Mention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mention::User(id) => write!(f, "<@{id}>"),
            Mention::Role(mention) => write!(f, "{mention}"),
        }
    }
}

/// A subset of Mention representing @everyone, @here, or `<@&ID>`.
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum RoleType {
    /// @everyone
    Everyone,
    /// @here
    Here,
    /// `<@&ID>`
    Role(poise::serenity_prelude::RoleId),
}

impl std::fmt::Display for RoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleType::Everyone => write!(f, "@everyone"),
            RoleType::Here => write!(f, "@here"),
            RoleType::Role(id) => write!(f, "<@&{id}>"),
        }
    }
}

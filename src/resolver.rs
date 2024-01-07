//! The instance of the DRQL interpreter resolver used for Intersection

use std::collections::HashSet;

use anyhow::{bail, Context as _};
use poise::{async_trait, serenity_prelude as serenity};
use tap::Tap;
use tracing::{debug, error, instrument, trace};

use crate::{
    drql::interpreter::InterpreterResolver,
    extensions::{CustomGuildImpl, CustomMemberImpl, CustomRoleImpl},
};

/// The custom instance of the DRQL [`InterpreterResolver`] used for Intersection.
pub struct Resolver<'a> {
    /// The guild that the query was originally sent in
    pub guild: &'a serenity::Guild,
    /// The member who originally sent the query
    pub member: &'a serenity::Member,
    /// The Context made available to the command
    pub ctx: &'a serenity::Context,
    /// THe channel the query was originally sent in
    pub channel: &'a serenity::GuildChannel,
}
#[async_trait]
impl<'a> InterpreterResolver<anyhow::Error> for Resolver<'a> {
    #[instrument(skip(self))]
    async fn resolve_string_literal(
        &mut self,
        literal: String,
    ) -> Result<HashSet<serenity::UserId>, anyhow::Error> {
        if literal == "everyone" || literal == "here" {
            if !self.member.permissions(self.ctx)?.mention_everyone() {
                debug!("Member does not have permission to mention everyone or here, bailing!");
                bail!(
                    concat!(
                        "You do not have the \"Mention everyone, here, and ",
                        "All Roles\" permission required to use the role {}."
                    ),
                    literal
                );
            }

            Ok(match literal.as_str() {
                "everyone" => self.guild.get_everyone(),
                "here" => self.guild.get_here(),
                _ => unreachable!(),
            }
            .tap(|x| {
                debug!(
                    "Resolved everyone/here literal to {:?}",
                    &x.iter().map(|x| x.0).collect::<Vec<_>>()
                );
            }))
        } else {
            trace!("Finding possible members/roles for string literal");

            let possible_members = self
                .guild
                .search_members(self.ctx, literal.as_str(), None)
                .await?;

            let possible_roles = self
                .guild
                .roles
                .iter()
                .filter(|(_, role)| role.name == literal)
                .collect::<Vec<_>>();

            debug!(
                "Found possible members: {:?}",
                possible_members
                    .iter()
                    .map(|x| x.user.id.0)
                    .collect::<Vec<_>>()
            );
            debug!(
                "Found possible roles: {:?}",
                possible_roles
                    .iter()
                    .map(|(_, x)| x.id.0)
                    .collect::<Vec<_>>()
            );

            match (possible_members.len(), possible_roles.len()) {
                (members_matched, roles_matched) if members_matched >= 1 && roles_matched >= 1 => {
                    debug!("Found both members and roles that matched the query, bailing!");
                    bail!(
                        concat!(
                            "Found {} member(s) and {} role(s) that matched your query for \"{}\".",
                            " Please narrow your query or use the ID of the object you are referring",
                            " to instead."
                        ),
                        members_matched,
                        roles_matched,
                        literal
                    );
                }
                (members_matched, _) if members_matched > 1 => {
                    debug!("Found multiple members that matched the query, bailing!");
                    bail!(
                        concat!(
                            "Found {} members that matched your query for \"{}\". Please narrow your",
                            " query: it may help to use the user's ID, or add their discriminator,",
                            " like \"luna..♡#9082\" instead of \"luna..♡\"."
                        ),
                        members_matched,
                        literal
                    );
                }
                (_, roles_matched) if roles_matched > 1 => {
                    debug!("Found multiple roles that matched the query, bailing!");
                    bail!(
                        concat!(
                            "Found {} roles that matched your query for \"{}\". Please narrow your",
                            " query: it may help to use a role ID instead."
                        ),
                        roles_matched,
                        literal
                    );
                }
                // At this point, we KNOW that members_matched and roles_matched are <= 1, and
                // only ONE of them is 1. Let's make sure that they aren't both 0:
                (members_matched, roles_matched) if members_matched == 0 && roles_matched == 0 => {
                    debug!("Found no members or roles that matched the query, bailing!");
                    bail!(
                        concat!(
                            "Unable to find a role or member with the name {}. Searches for roles",
                            " are case sensitive! Try using the ID instead?"
                        ),
                        literal
                    );
                }
                // Continue, members_matched + roles_matched == 1.
                _ => {}
            }

            assert!(possible_members.len() + possible_roles.len() == 1);

            // .first() more like .only() (len asserted == 1) and only one will be Some
            // TODO: use custom enum or perhaps Either
            let member = possible_members.first();
            let role = possible_roles.first().map(|(_, x)| x);

            match (member, role) {
                (Some(member), None) => {
                    debug!("Chose to use member {}", member.user.id.0);
                    self.resolve_user_id(member.user.id).await
                }

                (None, Some(role))
                    if !self.member.can_mention_role(self.ctx, role, self.channel)? =>
                {
                    debug!(
                        "Chose to use role {}, but user cannot mention it!",
                        role.id.0
                    );
                    bail!(
                        concat!(
                            "The role {} is not mentionable and you do not have",
                            " the \"Mention everyone, here, and All",
                            " Roles\" permission."
                        ),
                        role.name
                    );
                }

                (None, Some(role)) => {
                    debug!("Chose to use role {}", role.id.0);
                    self.resolve_role_id(role.id).await
                }

                // All other cases have been eliminated above.
                _ => unreachable!(),
            }
        }
    }

    #[instrument(skip(self))]
    async fn resolve_unknown_id(
        &mut self,
        id: String,
    ) -> Result<HashSet<serenity::UserId>, anyhow::Error> {
        if id == self.guild.id.to_string() {
            debug!("Unknown ID is the guild's ID, treating it as everyone");
            self.resolve_string_literal("everyone".to_string()).await
        } else {
            let id = id.parse::<u64>()?;
            debug!("Finding possible member/role for unknown ID");
            let possible_member = self.guild.member(self.ctx, id).await;
            let possible_role = self.guild.roles.get(&serenity::RoleId::from(id));

            debug!(
                "Possible member: {:?}",
                possible_member.as_ref().map(|member| member.user.id.0)
            );

            match (possible_member, possible_role) {
                (Ok(_), Some(_)) => {
                    error!("Somehow both a member and a role had the same ID, bailing!");
                    bail!(
                        "Somehow there was both a member and a role with the ID {}??",
                        id
                    )
                }

                (Ok(member), None) => {
                    debug!("Treating ID as a user ID.");
                    self.resolve_user_id(member.user.id).await
                }

                (Err(_), Some(role))
                    if !self.member.can_mention_role(self.ctx, role, self.channel)? =>
                {
                    debug!("Treating ID as a role ID, but user cannot mention role! Bailing.");
                    bail!(
                        concat!(
                            "The role {} is not mentionable and you do not have",
                            " the \"Mention everyone, here, and All Roles\"",
                            " permission."
                        ),
                        role.name
                    )
                }

                (Err(_), Some(role)) => {
                    debug!("Treating ID as a role ID.");
                    self.resolve_role_id(role.id).await
                }

                (Err(_), None) => {
                    debug!("Nothing found!");
                    bail!("Unable to resolve role or member ID: {}", id)
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn resolve_user_id(
        &mut self,
        id: serenity::UserId,
    ) -> Result<HashSet<serenity::UserId>, anyhow::Error> {
        debug!("Resolving User ID to itself: {}", id);
        Ok(HashSet::from([id]))
    }

    #[instrument(skip(self))]
    async fn resolve_role_id(
        &mut self,
        id: serenity::RoleId,
    ) -> Result<HashSet<serenity::UserId>, anyhow::Error> {
        if id.to_string() == self.guild.id.to_string() {
            debug!("Role ID is the guild's ID, treating it as everyone");
            self.resolve_string_literal("everyone".to_string()).await
        } else {
            Ok(self
                .guild
                .roles
                .get(&id)
                .context(format!("Unable to resolve role with ID {id}"))?
                .members(self.guild)
                .tap(|x| debug!("Resolved role ID to {x:?}")))
        }
    }
}

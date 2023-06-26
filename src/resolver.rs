use crate::drql::interpreter::InterpreterResolver;
use crate::extensions::{CustomGuildImpl, CustomMemberImpl, CustomRoleImpl};
use anyhow::{bail, Context as _};
use poise::{async_trait, serenity_prelude as serenity};
use std::collections::HashSet;

/// The custom instance of the DRQL [`InterpreterResolver`] used for Intersection.
pub struct Resolver<'a> {
    pub guild: &'a serenity::Guild,
    pub member: &'a serenity::Member,
    pub ctx: &'a serenity::Context,
}
#[async_trait]
impl<'a> InterpreterResolver<anyhow::Error> for Resolver<'a> {
    async fn resolve_string_literal(
        &mut self,
        literal: String,
    ) -> Result<HashSet<serenity::UserId>, anyhow::Error> {
        if literal == "everyone" || literal == "here" {
            if !self.member.permissions(self.ctx)?.mention_everyone() {
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
            })
        } else {
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

            match (possible_members.len(), possible_roles.len()) {
                (members_matched, roles_matched) if members_matched >= 1 && roles_matched >= 1 => {
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

            let member = possible_members.get(0);
            let role = possible_roles.get(0).map(|(_, x)| x);

            match (member, role) {
                (Some(member), None) => self.resolve_user_id(member.user.id).await,

                (None, Some(role)) if !self.member.can_mention_role(self.ctx, role)? => {
                    bail!(
                        concat!(
                            "The role {} is not mentionable and you do not have",
                            " the \"Mention everyone, here, and All",
                            " Roles\" permission."
                        ),
                        role.name
                    );
                }

                (None, Some(role)) => self.resolve_role_id(role.id).await,

                // All other cases have been eliminated above.
                _ => unreachable!(),
            }
        }
    }

    async fn resolve_unknown_id(
        &mut self,
        id: String,
    ) -> Result<HashSet<serenity::UserId>, anyhow::Error> {
        if id == self.guild.id.to_string() {
            self.resolve_string_literal("everyone".to_string()).await
        } else {
            let id = id.parse::<u64>()?;
            let possible_member = self.guild.member(self.ctx, id).await;
            let possible_role = self.guild.roles.get(&serenity::RoleId::from(id));

            match (possible_member, possible_role) {
                (Ok(_), Some(_)) => bail!(
                    "Somehow there was both a member and a role with the ID {}??",
                    id
                ),

                (Ok(member), None) => self.resolve_user_id(member.user.id).await,

                (Err(_), Some(role)) if !self.member.can_mention_role(self.ctx, role)? => {
                    bail!(
                        concat!(
                            "The role {} is not mentionable and you do not have",
                            " the \"Mention everyone, here, and All Roles\"",
                            " permission."
                        ),
                        role.name
                    )
                }

                (Err(_), Some(role)) => self.resolve_role_id(role.id).await,

                (Err(_), None) => {
                    bail!("Unable to resolve role or member ID: {}", id)
                }
            }
        }
    }

    async fn resolve_user_id(
        &mut self,
        id: serenity::UserId,
    ) -> Result<HashSet<serenity::UserId>, anyhow::Error> {
        Ok(HashSet::from([id]))
    }

    async fn resolve_role_id(
        &mut self,
        id: serenity::RoleId,
    ) -> Result<HashSet<serenity::UserId>, anyhow::Error> {
        if id.to_string() == self.guild.id.to_string() {
            self.resolve_string_literal("everyone".to_string()).await
        } else {
            Ok(self
                .guild
                .roles
                .get(&id)
                .context(format!("Unable to resolve role with ID {id}"))?
                .members(self.guild))
        }
    }
}

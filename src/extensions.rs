//! Intersection's extension traits
//!
//! This module defines extensions on Serenity types.

use std::collections::{HashMap, HashSet};

use anyhow::Context as _;
use poise::serenity_prelude as serenity;
use tracing::debug;

use crate::models;

/// Custom trait implemented on all [`serenity::Member`]s
pub trait CustomMemberImpl {
    /// Determine if this member can mention the given role
    fn can_mention_role(
        &self,
        ctx: &serenity::Context,
        role: &serenity::Role,
        channel: &serenity::GuildChannel,
    ) -> anyhow::Result<bool>;
}
impl CustomMemberImpl for serenity::Member {
    #[allow(clippy::cognitive_complexity)]
    fn can_mention_role(
        &self,
        ctx: &serenity::Context,
        role: &serenity::Role,
        channel: &serenity::GuildChannel,
    ) -> anyhow::Result<bool> {
        let guild_permissions = self.permissions(ctx)?;
        let channel_permissions = channel.permissions_for_user(ctx, self)?;

        if guild_permissions.administrator() {
            debug!(
                "{} can mention role {} because the user is an administrator",
                self.user.id, role.id
            );
            Ok(true)
        } else if role.mentionable {
            debug!(
                "{} can mention role {} because the role is mentionable by all",
                self.user.id, role.id
            );
            Ok(true)
        } else if guild_permissions.mention_everyone() {
            debug!(
                "{} can mention role {} because the user can mention everyone",
                self.user.id, role.id
            );
            Ok(true)
        } else if channel_permissions.mention_everyone() {
            debug!(
                "{} can mention role {} because the user can mention everyone in this channel",
                self.user.id, role.id
            );
            Ok(true)
        } else {
            debug!("{} cannot mention role {}", self.user.id, role.id);
            Ok(false)
        }
    }
}

/// Custom trait implemented on all [`serenity::Guild`]s
pub trait CustomGuildImpl {
    /// Obtain a [`HashSet`] of every member in this guild's user ID
    fn get_everyone(&self) -> HashSet<serenity::UserId>;
    /// Obtain a [`HashSet`] of every online member in this guild's user ID
    fn get_here(&self) -> HashSet<serenity::UserId>;
    /// Obtain a [`HashMap`] mapping every role in this guild to its members
    fn all_roles_and_members(
        &self,
        ctx: &serenity::Context,
    ) -> anyhow::Result<HashMap<models::mention::RoleType, HashSet<serenity::UserId>>>;
}
impl CustomGuildImpl for serenity::Guild {
    fn get_everyone(&self) -> HashSet<serenity::UserId> {
        self.members
            .values()
            .map(|member| member.user.id)
            .collect::<HashSet<_>>()
    }
    fn get_here(&self) -> HashSet<serenity::UserId> {
        self.get_everyone()
            .into_iter()
            .filter(|id| {
                self.presences.get(id).map_or(false, |presence| {
                    presence.status != serenity::OnlineStatus::Offline
                })
            })
            .collect::<HashSet<_>>()
    }
    fn all_roles_and_members(
        &self,
        ctx: &serenity::Context,
    ) -> anyhow::Result<HashMap<models::mention::RoleType, HashSet<serenity::UserId>>> {
        let mut map = HashMap::from([
            (models::mention::RoleType::Everyone, self.get_everyone()),
            (models::mention::RoleType::Here, self.get_here()),
        ]);

        for member in self.members.values() {
            for role in member.roles(ctx).context(format!(
                "Failed to get user role data for {}",
                member.user.id
            ))? {
                map.entry(models::mention::RoleType::Role(role.id))
                    .or_insert_with(HashSet::new)
                    .insert(member.user.id);
            }
        }

        Ok(map)
    }
}

/// Custom trait implemented on all [`serenity::Role`]s
pub trait CustomRoleImpl {
    /// Determine the members of this role
    fn members(&self, guild: &serenity::Guild) -> HashSet<serenity::UserId>;
}
impl CustomRoleImpl for serenity::Role {
    fn members(&self, guild: &serenity::Guild) -> HashSet<serenity::UserId> {
        guild
            .members
            .values()
            .filter(|member| member.roles.contains(&self.id))
            .map(|member| member.user.id)
            .collect::<HashSet<_>>()
    }
}

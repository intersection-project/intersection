use anyhow::Context as _;
use std::collections::{HashMap, HashSet};

use poise::serenity_prelude as serenity;

use crate::models;

pub trait CustomMemberImpl {
    fn can_mention_role(
        &self,
        ctx: &serenity::Context,
        role: &serenity::Role,
    ) -> anyhow::Result<bool>;
}
impl CustomMemberImpl for serenity::Member {
    fn can_mention_role(
        &self,
        ctx: &serenity::Context,
        role: &serenity::Role,
    ) -> anyhow::Result<bool> {
        Ok(role.mentionable
            || (self.permissions(ctx)?.mention_everyone())
            || (self.permissions(ctx)?.administrator()))
    }
}

pub trait CustomGuildImpl {
    fn get_everyone(&self) -> HashSet<serenity::UserId>;
    fn get_here(&self) -> HashSet<serenity::UserId>;
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

pub trait CustomRoleImpl {
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

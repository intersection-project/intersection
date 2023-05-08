mod commands;
mod drql;
mod util;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(
    /// Direct access to the DRQL LALRPOP parser. Prefer to use the functions exported by drql::parser instead.
    #[allow(clippy::all)]
    parser
);

use anyhow::{anyhow, bail};
use dotenvy::dotenv;
use drql::{ast, interpreter::InterpreterResolver};
use poise::{async_trait, serenity_prelude as serenity};
use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Display,
    hash::Hash,
    sync::Arc,
};

pub struct Data {
    /// The framework.shard_manager, used to get the latency of the current shard in the ping command
    shard_manager: Arc<serenity::Mutex<serenity::ShardManager>>,
}
type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum RoleType {
    Everyone,
    Here,
    Id(serenity::RoleId),
}
impl Display for RoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleType::Everyone => write!(f, "@everyone"),
            RoleType::Here => write!(f, "@here"),
            RoleType::Id(id) => write!(f, "<@&{}>", id),
        }
    }
}

/// Function to fold an iterator of ASTs into one large union expression
fn reduce_ast_chunks(iter: impl Iterator<Item = ast::Expr>) -> Option<ast::Expr> {
    iter.reduce(|acc, chunk| ast::Expr::Union(Box::new(acc), Box::new(chunk)))
}

trait CustomMemberImpl {
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

trait CustomGuildImpl {
    fn get_everyone(&self) -> HashSet<serenity::UserId>;
    fn get_here(&self) -> HashSet<serenity::UserId>;
    fn all_roles_and_members(
        &self,
        ctx: &serenity::Context,
    ) -> anyhow::Result<HashMap<RoleType, HashSet<serenity::UserId>>>;
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
                if let Some(presence) = self.presences.get(id) {
                    presence.status != serenity::OnlineStatus::Offline
                } else {
                    false
                }
            })
            .collect::<HashSet<_>>()
    }
    fn all_roles_and_members(
        &self,
        ctx: &serenity::Context,
    ) -> anyhow::Result<HashMap<RoleType, HashSet<serenity::UserId>>> {
        let mut map = HashMap::from([
            (RoleType::Everyone, self.get_everyone()),
            (RoleType::Here, self.get_here()),
        ]);

        for member in self.members.values() {
            for role in member.roles(ctx).ok_or(anyhow!(
                "Failed to get user role data for {}",
                member.user.id
            ))? {
                map.entry(RoleType::Id(role.id))
                    .or_insert_with(HashSet::new)
                    .insert(member.user.id);
            }
        }

        Ok(map)
    }
}

trait CustomRoleImpl {
    fn members(&self, guild: &serenity::Guild) -> HashSet<serenity::UserId>;
}
impl CustomRoleImpl for serenity::Role {
    fn members(&self, guild: &serenity::Guild) -> HashSet<serenity::UserId> {
        HashSet::from_iter(
            guild
                .members
                .values()
                .filter(|member| member.roles.contains(&self.id))
                .map(|member| member.user.id),
        )
    }
}

/// The custom instance of the DRQL [InterpreterResolver] used for Intersection.
struct Resolver<'a> {
    guild: &'a serenity::Guild,
    member: &'a serenity::Member,
    ctx: &'a serenity::Context,
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
                .ok_or(anyhow!("Unable to resolve role with ID {}", id))?
                .members(self.guild))
        }
    }
}

/// Find the application command `/name` and return the string mentioning that application command.
///
/// If the name contains spaces, the first word is the command name and the rest is the subcommand name.
///
/// If the command is not found, it returns a code block containing the command name and prints
/// a warning.
async fn mention_application_command(ctx: &serenity::Context, command_string: &str) -> Result<String, anyhow::Error> {
    let command_name = command_string.split_once(' ').map(|(name, _)| name).unwrap_or(command_string);

    let command = serenity::model::application::command::Command::get_global_application_commands(ctx)
        .await
        .map_err(|_| anyhow::anyhow!("Error looking up global application commands!"))?
        .into_iter()
        .find(|command| command.name == command_name);

    match command {
        Some(command) => Ok(format!("</{}:{}>", command_string, command.id.0)),
        None => {
            println!("WARN: Attempt to mention the command \"{}\" (root command {}) which was not found!", command_string, command_name);
            Ok(format!("`/{}`", command_string))
        }
    }
}

async fn on_message(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    _framework: poise::FrameworkContext<'_, Data, anyhow::Error>,
    _data: &Data,
) -> Result<(), anyhow::Error> {
    if msg.author.bot {
        return Ok(());
    }

    if msg.guild(ctx).is_none() {
        if drql::scanner::scan(msg.content.as_str()).count() > 0 {
            msg.reply(
                ctx,
                "DRQL queries are only supported in guilds, not in DMs.",
            )
            .await?;
        }
        return Ok(());
    }

    let Some(ast) = reduce_ast_chunks(
        drql::scanner::scan(msg.content.as_str())
            .map(drql::parser::parse_drql)
            // TODO: Report errors as 'error in chunk X'?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter(),
    ) else {
        return Ok(());
    };

    let guild = msg.guild(ctx).ok_or(anyhow!("Unable to resolve guild"))?;

    let members_to_ping = match drql::interpreter::interpret(
        ast,
        &mut Resolver {
            guild: &guild,
            member: &msg.member(ctx).await?,
            ctx,
        },
    )
    .await
    {
        Ok(ast) => ast,
        Err(e) => {
            msg.reply(
                ctx,
                format!("An error occurred while calculating the result: {}", e),
            )
            .await?;
            return Ok(());
        }
    };

    // Now that we know which members we have to notify, we can do some specialized calculations
    // to try to replace members in that set with existing roles in the server. First, we choose our
    // "qualifiers" -- any role in this server that is a **subset** of our members_to_ping.

    // A hashmap of every role in the guild and its members.
    let roles_and_their_members = guild.all_roles_and_members(ctx)?;

    // next, we represent the list of users as a bunch of roles containing them and one outliers set.
    let util::unionize_set::UnionizeSetResult { sets, outliers } =
        util::unionize_set::unionize_set(&members_to_ping, &roles_and_their_members);

    // if members_to_ping.len() > 50 {
    //     // TODO: Ask the user to confirm they wish to do this action
    // }

    // Now we need to split the output message into individual pings. First, stringify each user mention...
    // TODO: Once message splitting is complete this could result in a user being
    // pinged multiple times if they are present in a role that is split into multiple
    // messages.
    // e.g.
    // user is in @A and @C
    // message 1: @A @B ...
    // message 2: @C @D ...
    // double ping!
    let stringified_mentions = sets
        .into_keys()
        .map(|role| role.to_string())
        .chain(outliers.into_iter().map(|id| format!("<@{}>", id)))
        .collect::<Vec<_>>();

    if stringified_mentions.is_empty() {
        msg.reply(ctx, "No users matched.").await?;
        return Ok(());
    }

    let notification_string = format!(
        concat!(
            "Notification triggered by Intersection.\n",
            ":question: **What is this?** Run {} for more information.\n"
        ),
        mention_application_command(ctx, "about").await
    );

    if stringified_mentions.join(" ").len() <= (2000 - notification_string.len()) {
        msg.reply(
            ctx,
            format!("{}{}", notification_string, stringified_mentions.join(" ")),
        )
        .await?;
    } else {
        let messages = util::wrap_string_vec(stringified_mentions, " ", 2000)?;
        msg.reply(
            ctx,
            format!(
                "Notification triggered by Intersection. Please wait, sending {} messages...",
                messages.len()
            ),
        )
        .await?;
        for message in messages {
            msg.reply(ctx, message).await?;
        }
        msg.reply(
            ctx,
            format!(
                concat!(
                    "Notification triggered successfully.\n",
                    ":question: **What is this?** Run {} for more information."
                ),
                mention_application_command(ctx, "about").await
            ),
        )
        .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // We ignore the error because environment variables may be passed
    // in directly, and .env might not exist (e.g. in Docker with --env-file)
    let _ = dotenv();

    let framework: poise::FrameworkBuilder<Data, anyhow::Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::ping(), commands::about(), commands::debug()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(async move {
                    if let poise::Event::Message { new_message } = event {
                        on_message(ctx, new_message, framework, data).await
                    } else {
                        Ok(())
                    }
                })
            },

            ..Default::default()
        })
        .token(env::var("TOKEN").expect("Expected a token in the environment"))
        .intents(serenity::GatewayIntents::all())
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                println!(
                    "Logged in as {}#{}!",
                    ready.user.name, ready.user.discriminator
                );

                println!("Registering global application (/) commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                println!("Finished registering global application (/) commands.");

                Ok(Data {
                    shard_manager: Arc::clone(framework.shard_manager()),
                })
            })
        });

    Ok(framework.run().await?)
}

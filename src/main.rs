#![feature(is_some_and)]

mod drql;
mod models;
mod schema;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(
    /// Direct access to the DRQL LALRPOP parser. Prefer to use the functions exported by drql::parser instead.
    #[allow(clippy::all)]
    parser
);

use crate::{drql::ast::Expr, models::NewGuild};
use anyhow::{anyhow, bail};
use async_recursion::async_recursion;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    result::Error::NotFound,
    ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection,
};
use dotenvy::dotenv;
use drql::ast;
use models::GuildDBData;
use serenity::{async_trait, model::prelude::*, prelude::*};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    env,
    fmt::Display,
};

struct DB;

impl TypeMapKey for DB {
    type Value = Pool<ConnectionManager<SqliteConnection>>;
}

struct Handler;

struct CommandExecution<'a> {
    ctx: &'a Context,
    msg: &'a Message,
    guild: GuildDBData,
    command: &'a str,
    args: VecDeque<&'a str>,
}

/// Function to obtain all members in a role
fn members_of_role(guild: &Guild, role: &Role) -> HashSet<UserId> {
    HashSet::from_iter(
        guild
            .members
            .values()
            .filter(|member| member.roles.contains(&role.id))
            .map(|member| member.user.id),
    )
}

/// Function to fold an iterator of ASTs into one large union expression
fn reduce_ast_chunks(iter: impl Iterator<Item = ast::Expr>) -> Option<ast::Expr> {
    iter.reduce(|acc, chunk| ast::Expr::Union(Box::new(acc), Box::new(chunk)))
}

/// Determine if a user can mention a given role
fn can_mention_role(ctx: &Context, role: &Role, member: &Member) -> anyhow::Result<bool> {
    Ok(role.mentionable || (member.permissions(ctx)?.mention_everyone()))
}

/// Function called whenever a **message-based command** is triggered.
async fn handle_command(data: CommandExecution<'_>) -> anyhow::Result<()> {
    let CommandExecution {
        ctx,
        msg,
        guild,
        command,
        mut args,
    } = data;

    if command == "config" {
        if !msg.member(ctx).await?.permissions(ctx)?.manage_guild() {
            msg.reply(
                ctx,
                "You need the Manage Server permission to run this command!",
            )
            .await?;
            return Ok(());
        }

        let subcommand = match args.pop_front() {
            Some(subcommand) => subcommand.to_lowercase(),
            None => {
                msg.reply(
                    ctx,
                    format!(
                        "You need to specify a subcommand. Try `{}config help`",
                        guild.prefix
                    ),
                )
                .await?;
                return Ok(());
            }
        };

        if subcommand == "help" {
            msg.reply(ctx, "Available subcommands: `prefix`, `help`")
                .await?;
        } else if subcommand == "prefix" {
            let action = match args.pop_front() {
                Some(action) => action.to_lowercase(),
                None => {
                    msg.reply(ctx, "Specify an action verb, `get` or `set`.")
                        .await?;
                    return Ok(());
                }
            };

            if action == "set" {
                if args.is_empty() {
                    msg.reply(
                        ctx,
                        format!(
                            "You need to specify a prefix. Try `{}config prefix set <prefix>`",
                            guild.prefix
                        ),
                    )
                    .await?;
                    return Ok(());
                }

                let new_prefix = args.make_contiguous().join(" ");

                // Obtain a connection to the database
                let mut conn = ctx
                    .data
                    .read()
                    .await
                    .get::<DB>()
                    .ok_or(anyhow!("DB was None"))?
                    .get()?;

                diesel::update(schema::guilds::table)
                    .filter(
                        schema::guilds::id.eq(msg
                            .guild_id
                            .ok_or(anyhow!("msg.guild_id was None"))?
                            .to_string()),
                    )
                    .set(schema::guilds::prefix.eq(new_prefix.as_str()))
                    .execute(&mut conn)?;

                msg.reply(
                    ctx,
                    format!("This server's prefix has been set to `{}`.", new_prefix),
                )
                .await?;
            } else if action == "get" {
                msg.reply(
                    ctx,
                    format!("This server's prefix is set to `{}`.", guild.prefix),
                )
                .await?;
            } else {
                msg.reply(
                    ctx,
                    format!(
                        "Unknown action verb. Try `{}config prefix get` or `{}config prefix set`.",
                        guild.prefix, guild.prefix
                    ),
                )
                .await?;
            }
        } else {
            msg.reply(
                ctx,
                format!("Unknown subcommand. Try `{}config help`", guild.prefix),
            )
            .await?;
        }
    } else if command == "run" {
        let Some(ast) = reduce_ast_chunks(
            drql::scanner::scan(args.make_contiguous().join(" ").as_str())
                .map(drql::parser::parse_drql)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter(),
        ) else {
            msg.reply(ctx, "Your message does not contain any DRQL queries to attempt to resolve").await?;
            return Ok(());
        };

        /// Walk over the [Expr] type and reduce it into a set of user IDs that
        /// need to be mentioned
        #[async_recursion]
        async fn walk_and_reduce_ast(
            msg: &Message,
            ctx: &Context,
            node: Expr,
        ) -> anyhow::Result<HashSet<UserId>> {
            let discord_guild = msg.guild(ctx).ok_or(anyhow!("Unable to resolve guild"))?;

            Ok(match node {
                Expr::Difference(left, right) => walk_and_reduce_ast(msg, ctx, *left)
                    .await?
                    .difference(&walk_and_reduce_ast(msg, ctx, *right).await?)
                    .copied()
                    .collect::<HashSet<_>>(),
                Expr::Intersection(left, right) => walk_and_reduce_ast(msg, ctx, *left)
                    .await?
                    .intersection(&walk_and_reduce_ast(msg, ctx, *right).await?)
                    .copied()
                    .collect::<HashSet<_>>(),
                Expr::Union(left, right) => walk_and_reduce_ast(msg, ctx, *left)
                    .await?
                    .union(&walk_and_reduce_ast(msg, ctx, *right).await?)
                    .copied()
                    .collect::<HashSet<_>>(),
                Expr::UserID(id) => HashSet::from([id]),
                Expr::RoleID(id) => {
                    if id.to_string() == discord_guild.id.to_string() {
                        walk_and_reduce_ast(msg, ctx, Expr::StringLiteral("everyone".to_string()))
                            .await?
                    } else {
                        let role = discord_guild
                            .roles
                            .get(&id)
                            .ok_or(anyhow!("Unable to resolve role"))?;

                        members_of_role(&discord_guild, role)
                    }
                }
                Expr::UnknownID(id) => {
                    if id == discord_guild.id.to_string() {
                        walk_and_reduce_ast(msg, ctx, Expr::StringLiteral("everyone".to_string()))
                            .await?
                    } else {
                        let possible_member = discord_guild.member(ctx, id.parse::<u64>()?).await;
                        let possible_role =
                            discord_guild.roles.get(&RoleId::from(id.parse::<u64>()?));

                        match (possible_member, possible_role) {
                            (Ok(_), Some(_)) => bail!(
                                "Somehow there was both a member and a role with the ID {}??",
                                id
                            ),

                            (Ok(member), None) => {
                                walk_and_reduce_ast(msg, ctx, Expr::UserID(member.user.id)).await?
                            }

                            (Err(_), Some(role))
                                if !can_mention_role(ctx, role, &msg.member(ctx).await?)? =>
                            {
                                bail!("The role {} is not mentionable and you do not have the \"Mention everyone, here, and All Roles\" permission.", role.name)
                            }

                            (Err(_), Some(role)) => {
                                walk_and_reduce_ast(msg, ctx, Expr::RoleID(role.id)).await?
                            }

                            (Err(_), None) => bail!("Unable to resolve role or member ID: {}", id),
                        }
                    }
                }
                Expr::StringLiteral(s) => {
                    if s == "everyone" || s == "here" {
                        if !msg.member(ctx).await?.permissions(ctx)?.mention_everyone() {
                            bail!("You do not have the \"Mention everyone, here, and All Roles\" permission required to use the role {}.", s);
                        }

                        let everyone = discord_guild.members.values().map(|member| member.user.id);

                        match s.as_str() {
                            "everyone" => HashSet::from_iter(everyone),
                            "here" => HashSet::from_iter(everyone.filter(|id| {
                                discord_guild.presences.get(id).is_some_and(|presence| {
                                    presence.status != OnlineStatus::Offline
                                })
                            })),
                            _ => panic!("This will never happen"),
                        }
                    } else {
                        let possible_members = discord_guild
                            .members // FIXME: what if the members aren't cached?
                            .iter()
                            .filter(|(_, member)| {
                                member.user.tag().to_lowercase() == s.to_lowercase()
                            })
                            .collect::<Vec<_>>();
                        let possible_roles = discord_guild
                            .roles
                            .iter()
                            .filter(|(_, role)| role.name.to_lowercase() == s.to_lowercase())
                            .collect::<Vec<_>>();

                        if possible_members.len() > 1 {
                            bail!(
                                "Multiple members matched your query for {}. Use their ID instead.",
                                s
                            );
                        }
                        if possible_members.len() > 1 {
                            bail!(
                                "Multiple roles matched your query for {}. Use their ID instead.",
                                s
                            );
                        }

                        match (possible_members.get(0), possible_roles.get(0)) {
                            (Some(_), Some(_)) => bail!(
                                "Found a member and role with the same name. Use their ID instead."
                            ),

                            (Some((_, member)), None) => {
                                walk_and_reduce_ast(msg, ctx, Expr::UserID(member.user.id)).await?
                            }

                            (None, Some((_, role)))
                                if !can_mention_role(ctx, role, &msg.member(ctx).await?)? =>
                            {
                                bail!("The role {} is not mentionable and you do not have the \"Mention everyone, here, and All Roles\" permission.", role.name);
                            }

                            (None, Some((_, role))) => {
                                walk_and_reduce_ast(msg, ctx, Expr::RoleID(role.id)).await?
                            }

                            (None, None) => {
                                bail!("Unable to resolve role or member **username** (use a tag like \"User#1234\" and no nickname!): {}", s);
                            }
                        }
                    }
                }
            })
        }

        let members_to_ping = match walk_and_reduce_ast(msg, ctx, ast).await {
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

        let discord_guild = msg.guild(ctx).ok_or(anyhow!("Unable to resolve guild"))?;

        // Now that we know which members we have to notify, we can do some specialized calculations
        // to try to replace members in that set with existing roles in the server. First, we choose our
        // "qualifiers" -- any role in this server that is a **subset** of our members_to_ping.

        #[derive(Debug, PartialEq, Eq, Hash, Clone)]
        enum RoleThing {
            Everyone,
            Here,
            Id(RoleId),
        }
        impl Display for RoleThing {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    RoleThing::Everyone => write!(f, "@everyone"),
                    RoleThing::Here => write!(f, "@here"),
                    RoleThing::Id(id) => write!(f, "<@&{}>", id),
                }
            }
        }

        // A hashmap of every role in the guild and its members.
        let mut roles_and_their_members: HashMap<RoleThing, HashSet<UserId>> = HashMap::from([
            (
                RoleThing::Everyone,
                discord_guild
                    .members
                    .values()
                    .map(|v| v.user.id)
                    .collect::<HashSet<_>>(),
            ),
            (
                RoleThing::Here,
                discord_guild
                    .members
                    .values()
                    .filter(|v| {
                        discord_guild
                            .presences
                            .get(&v.user.id)
                            .is_some_and(|p| p.status != OnlineStatus::Offline)
                    })
                    .map(|v| v.user.id)
                    .collect::<HashSet<_>>(),
            ),
        ]);

        for member in discord_guild.members.values() {
            for role in member.roles(ctx).ok_or(anyhow!("No role data??"))? {
                if let Some(entry) = roles_and_their_members.get_mut(&RoleThing::Id(role.id)) {
                    entry.insert(member.user.id);
                } else {
                    roles_and_their_members
                        .insert(RoleThing::Id(role.id), HashSet::from([member.user.id]));
                }
            }
        }

        // determine which of the available roles in the guild is a subset of our target notification
        // and qualify it
        let qualifiers: HashMap<&RoleThing, &HashSet<UserId>> = roles_and_their_members
            .iter()
            .filter(|(_, members)| members.is_subset(&members_to_ping))
            .collect::<HashMap<_, _>>();

        // Now we remove redundant qualifiers. This is done by iterating over each one and determining
        // if one of the other values in it is a superset of itself, if so, it's redundant and can be
        // removed.
        let new_qualifiers: HashMap<&RoleThing, &HashSet<UserId>> = qualifiers
            .iter()
            .map(|(&a, &b)| (a, b)) // TODO: Is there a way to do this without copying?
            .filter(|(key, value)| {
                // Filter out any values in qualifiers with a superset also within qualifiers.
                !(qualifiers.iter().any(|(other_key, other_value)| {
                    // But don't count ourself
                    key != other_key && other_value.is_superset(value)
                }))
            })
            .collect::<HashMap<_, _>>();

        // Now that new_qualifiers holds the roles that we plan on pinging, we determine our outliers.
        let included_members: HashSet<UserId> = qualifiers
            .into_values()
            .flatten()
            .copied() // TODO: is there a way to not copy here? the problem is .difference won't work with iterator over T vs &T,
            .collect::<HashSet<_>>();

        let outliers = members_to_ping.difference(&included_members);

        msg.reply(
            ctx,
            format!(
                "{} {}",
                new_qualifiers
                    .keys()
                    .map(|k| k.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                outliers
                    .map(|id| format!("<@{}>", id))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        )
        .await?;
    } else {
        msg.reply(ctx, "Unknown command.").await?;
    }

    Ok(())
}

/// Obtain a [Guild] instance
async fn obtain_guild(ctx: &Context, guild_id: &str) -> anyhow::Result<GuildDBData> {
    use schema::guilds::dsl::*;

    let mut conn = ctx
        .data
        .read()
        .await
        .get::<DB>()
        .ok_or(anyhow!("DB was None"))?
        .get()?;

    Ok(
        match guilds
            .filter(id.eq(guild_id))
            .first::<GuildDBData>(&mut conn)
        {
            Ok(guild) => guild,
            Err(NotFound) => {
                let new_guild = NewGuild {
                    id: guild_id,
                    prefix: None,
                };

                diesel::insert_into(guilds)
                    .values(&new_guild)
                    .execute(&mut conn)?;

                // Re-do the query now that we have inserted
                guilds
                    .filter(id.eq(guild_id))
                    .first::<GuildDBData>(&mut conn)?
            }
            Err(e) => return Err(e.into()),
        },
    )
}

/// Function called on every message.
async fn handle_message(ctx: &Context, msg: &Message) -> anyhow::Result<()> {
    if msg.author.bot {
        return Ok(());
    }

    if msg.channel(ctx).await?.guild().is_none() {
        msg.reply(ctx, "This bot only works in servers.").await?;
        return Ok(());
    }

    // Get this Guild from the database
    let guild = obtain_guild(
        ctx,
        msg.guild_id
            .ok_or(anyhow!("msg.guild_id was None"))?
            .to_string()
            .as_str(),
    )
    .await?;

    // TODO: Guide the user if they mention the bot instead of a prefix

    if !msg.content.starts_with(&guild.prefix) {
        return Ok(());
    }

    let mut args = msg.content[guild.prefix.len()..]
        .split_whitespace()
        .collect::<VecDeque<_>>();

    let command = match args.pop_front() {
        Some(command) => command,
        None => return Ok(()),
    };

    println!(
        "Command {} run by {} ({}) with args \"{}\"",
        command,
        msg.author.tag(),
        msg.author.id,
        args.make_contiguous().join(" ")
    );

    handle_command(CommandExecution {
        ctx,
        msg,
        guild,
        command,
        args,
    })
    .await
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let result = handle_message(&ctx, &msg).await;

        if let Err(e) = result {
            if let Err(e2) = msg
                .reply(
                    ctx,
                    format!("An error occurred while processing your command: {}", e),
                )
                .await
            {
                println!("An error occurred while handling an error. {:?}", e2);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Logged in as {}!", ready.user.tag());
        ctx.set_activity(Activity::watching("for custom mentions"))
            .await;
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv()?;

    let database_url = env::var("DATABASE_URL").expect("Expected DATABASE_URL in the environment");
    let pool = Pool::builder()
        .test_on_check_out(true)
        .build(ConnectionManager::<SqliteConnection>::new(database_url))?;

    let intents = GatewayIntents::all();

    let mut client = Client::builder(
        env::var("TOKEN").expect("Expected a token in the environment"),
        intents,
    )
    .event_handler(Handler)
    .await?;

    {
        let mut data = client.data.write().await;
        data.insert::<DB>(pool);
    }

    client.start().await?;

    Ok(())
}

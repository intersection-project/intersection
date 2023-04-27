#![feature(never_type)]
#![feature(async_closure)]

mod drql;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(
    /// Direct access to the DRQL LALRPOP parser. Prefer to use the functions exported by drql::parser instead.
    #[allow(clippy::all)]
    parser
);

use crate::drql::{ast::Expr, interpreter::ReducerOp};
use anyhow::{anyhow, bail};
use async_recursion::async_recursion;
use dotenvy::dotenv;
use drql::ast;
use poise::serenity_prelude as serenity;
use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Display,
    hash::Hash,
};

struct Data {}
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

// Create a function chunk_str_vec_into_max_size that takes 3 parameters. The first parameter, 'input'
// is a vector of strings. The second parameter, sep, is a separator. The third parameter, 'size' is
// the maximum size to create. The function should return a vector of strings, where each element in
// the result vector is as many elements from the input vector as possible, without going over the
// size limit. For example, given an input of ["abc", "def", "ghi", "jkl", "mno"] and a limit of 7,
// return ["abc def", "ghi jkl", "mno"].
// Errors when a chunk len()>size.
fn chunk_str_vec_into_max_size(
    mut input: Vec<String>,
    sep: &str,
    size: usize,
) -> anyhow::Result<Vec<String>> {
    input.reverse();
    let mut result = Vec::new();
    let mut current = String::new();
    while let Some(next) = input.pop() {
        if next.len() > size {
            bail!("Chunk of length {} too large for size {}", next.len(), size);
        }
        if current.len() + next.len() + sep.len() > size {
            result.push(current);
            current = next;
        } else {
            if !current.is_empty() {
                current.push_str(sep);
            }
            current.push_str(&next);
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    Ok(result)
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
                self.presences
                    .get(id)
                    .is_some_and(|presence| presence.status != serenity::OnlineStatus::Offline)
            })
            .collect::<HashSet<_>>()
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

// THE FIRST LINE IS THE COMMAND DESCRIPTION!
/// Check if Intersection is online
#[poise::command(slash_command)]
async fn ping(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    // TODO: get current bot ping
    ctx.say("I'm alive!").await?;
    Ok(())
}

/// Learn about the Intersection Project
#[poise::command(slash_command)]
async fn about(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    ctx.say(concat!(
        "The Intersection Project is a Discord bot with the purpose of supercharging Discord",
        " mentions. Intersection empowers Discord with a language called DRQL -- the Discord",
        " Role Query Language. DRQL is a very simple set-theory based query language that",
        " allows you to make advanced operations on roles.\n",
        "\n",
        "An example of a DRQL query might be `@{ admins + (mods & here) }`. This query represents",
        " some very basic set theory concepts, namely the union and **intersection** operations.",
        " The following operators are available in DRQL:\n",
        "- `A + B` or `A | B`: Union -- Members with either role A or role B (equivalent to if you",
        " were to ping @A and @B)\n",
        "- `A & B`: Intersection -- Members with *both* roles A and B (something Discord does not",
        " provide a native facility for\n",
        "- `A - B`: Difference -- Members with role A but *not* role B\n",
        "\n",
        "Intersection functions by searching for messages that contain DRQL queries wrapped in @{...}",
        " and then calculating and replying to your message by pinging every user that that query matched.",
        " The bot will also attempt to ping *roles* that match the query (to help keep the resulting",
        " message short) but this is not always possible. The availability of so-called \"optimized",
        " result messages\" depends on the query and the roles in your server.\n",
        "\n",
        "Intersection is still a project in-development. If you have any questions, comments, or",
        " suggestions, please feel free to reach out!"
    ))
    .await?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum DRQLValue {
    UserID(serenity::UserId),
    RoleID(serenity::RoleId),
    UnknownID(String),
    StringLiteral(String),
}
impl From<Expr> for ReducerOp<DRQLValue> {
    fn from(value: Expr) -> Self {
        match value {
            Expr::Difference(l, r) => {
                ReducerOp::Difference(Box::new((*l).into()), Box::new((*r).into()))
            }
            Expr::Intersection(l, r) => {
                ReducerOp::Intersection(Box::new((*l).into()), Box::new((*r).into()))
            }
            Expr::Union(l, r) => ReducerOp::Union(Box::new((*l).into()), Box::new((*r).into())),
            Expr::RoleID(id) => ReducerOp::User(DRQLValue::RoleID(id)),
            Expr::UserID(id) => ReducerOp::User(DRQLValue::UserID(id)),
            Expr::UnknownID(id) => ReducerOp::User(DRQLValue::UnknownID(id)),
            Expr::StringLiteral(id) => ReducerOp::User(DRQLValue::StringLiteral(id)),
        }
    }
}

/// Walk over the [Expr] type and reduce it into a set of user IDs that
/// need to be mentioned
#[derive(Clone)]
struct UserData<'a> {
    msg: &'a serenity::Message,
    ctx: &'a serenity::Context,
}

#[async_recursion]
async fn resolver<'a>(
    value: DRQLValue,
    data: &UserData<'a>,
) -> anyhow::Result<HashSet<serenity::UserId>> {
    let UserData { msg, ctx } = data;

    let discord_guild = msg.guild(ctx).ok_or(anyhow!("Unable to resolve guild"))?;

    Ok(match value {
        DRQLValue::UserID(id) => HashSet::from([id]),
        DRQLValue::RoleID(id) => {
            if id.to_string() == discord_guild.id.to_string() {
                resolver(DRQLValue::StringLiteral("everyone".to_string()), data).await?
            } else {
                let role = discord_guild
                    .roles
                    .get(&id)
                    .ok_or(anyhow!("Unable to resolve role"))?;

                role.members(&discord_guild)
            }
        }
        DRQLValue::UnknownID(id) => {
            if id == discord_guild.id.to_string() {
                resolver(DRQLValue::StringLiteral("everyone".to_string()), data).await?
            } else {
                let possible_member = discord_guild.member(ctx, id.parse::<u64>()?).await;
                let possible_role = discord_guild
                    .roles
                    .get(&serenity::RoleId::from(id.parse::<u64>()?));

                match (possible_member, possible_role) {
                    (Ok(_), Some(_)) => bail!(
                        "Somehow there was both a member and a role with the ID {}??",
                        id
                    ),

                    (Ok(member), None) => resolver(DRQLValue::UserID(member.user.id), data).await?,

                    (Err(_), Some(role))
                        if !msg.member(ctx).await?.can_mention_role(ctx, role)? =>
                    {
                        bail!(
                            concat!(
                                "The role {} is not mentionable and you do not have",
                                " the \"Mention everyone, here, and All Roles\"",
                                " permission."
                            ),
                            role.name
                        )
                    }

                    (Err(_), Some(role)) => resolver(DRQLValue::RoleID(role.id), data).await?,

                    (Err(_), None) => {
                        bail!("Unable to resolve role or member ID: {}", id)
                    }
                }
            }
        }
        DRQLValue::StringLiteral(s) => {
            if s == "everyone" || s == "here" {
                if !msg.member(ctx).await?.permissions(ctx)?.mention_everyone() {
                    bail!(
                        concat!(
                            "You do not have the \"Mention everyone, here, and ",
                            "All Roles\" permission required to use the role {}."
                        ),
                        s
                    );
                }

                match s.as_str() {
                    "everyone" => discord_guild.get_everyone(),
                    "here" => discord_guild.get_here(),
                    _ => unreachable!(),
                }
            } else {
                let possible_members = discord_guild
                    .members // FIXME: what if the members aren't cached?
                    .iter()
                    .filter(|(_, member)| member.user.tag().to_lowercase() == s.to_lowercase())
                    .collect::<Vec<_>>();
                let possible_roles = discord_guild
                    .roles
                    .iter()
                    .filter(|(_, role)| role.name.to_lowercase() == s.to_lowercase())
                    .collect::<Vec<_>>();

                if possible_members.len() > 1 {
                    bail!(
                        concat!(
                            "Multiple members matched your query for {}.",
                            " Use their ID instead."
                        ),
                        s
                    );
                }
                if possible_roles.len() > 1 {
                    bail!(
                        "Multiple roles matched your query for {}. Use their ID instead.",
                        s
                    );
                }

                match (possible_members.get(0), possible_roles.get(0)) {
                    (Some(_), Some(_)) => bail!(
                        concat!(
                            "Found a member and role with the same name",
                            " in your query for {}. Use their ID instead."
                        ),
                        s
                    ),

                    (Some((_, member)), None) => {
                        resolver(DRQLValue::UserID(member.user.id), data).await?
                    }

                    (None, Some((_, role)))
                        if !msg.member(ctx).await?.can_mention_role(ctx, role)? =>
                    {
                        bail!(
                            concat!(
                                "The role {} is not mentionable and you do not have",
                                " the \"Mention everyone, here, and All",
                                " Roles\" permission."
                            ),
                            role.name
                        );
                    }

                    (None, Some((_, role))) => resolver(DRQLValue::RoleID(role.id), data).await?,

                    (None, None) => {
                        bail!(
                            concat!(
                                "Unable to resolve role or member **username**",
                                " (use a tag like \"User#1234\" and no nickname!): {}"
                            ),
                            s
                        );
                    }
                }
            }
        }
    })
}

/// Find the application command `/{name}` and return `</{name}:{id of /name}>`, or `/name` if
/// it could not be found.
async fn mention_application_command(ctx: &serenity::Context, name: &str) -> String {
    match serenity::model::application::command::Command::get_global_application_commands(ctx).await
    {
        Ok(commands) => match commands.iter().find(|command| command.name == name) {
            Some(command) => format!("</{}:{}>", name, command.id.0),
            None => {
                println!("WARN (mention_application_command): Attempt to mention a slash command {} that was not found!", name);
                format!("/{}", name)
            }
        },
        Err(_) => {
            println!(
                "WARN (mention_application_command): Error looking up global application commands!"
            );
            format!("/{}", name)
        }
    }
}

/// Results from [unionize_set].
#[derive(Debug)]
struct UnionizeSetResult<'a, Key, Value> {
    sets: HashMap<&'a Key, &'a HashSet<Value>>,
    outliers: HashSet<Value>,
}

/// "Optimize" a set, representing it as the union of pre-existing sets and one "outliers" set.
///
/// Given the following:
/// - A target set to represent
/// - A HashMap of pre-existing HashSets
///
/// This function will return two HashSets:
/// - A set of keys within the HashMap
/// - The "outliers" set
///
/// The purpose of this function is to take the input set and calculate which pre-existing sets
/// which when the union of all of them along with the outliers set is calculated, will exactly
/// equal the target set. This is better described using an example from this project: Intersection
/// calculates a set of user IDs that must be mentioned, but the long message that might be a result
/// of a long DRQL query would not be very user-friendly. This function is passed the target users,
/// and a HashMap of every role in the server, and it outputs the roles that can be mentioned to
/// closely represent the target users in as few mentions as possible. In some cases, there will not
/// be a perfect representation, and the outliers set represents those that were not included.
///
/// This function is a best-effort optimization, and speed is a CRITICAL priority, meaning some
/// small details may not be accounted for in the interest of rare cases. The output from this
/// function may change without notice.
///
/// TODO: Example
fn unionize_set<'a, Key, Value>(
    target: &'a HashSet<Value>,
    preexisting_sets: &'a HashMap<Key, HashSet<Value>>,
) -> UnionizeSetResult<'a, Key, Value>
where
    Key: PartialEq + Eq + Hash + Copy,
    Value: PartialEq + Eq + Hash + Copy,
{
    // First, we will determine which sets in preexisting_sets even *qualify* to be used in the output.
    // The condition here is simple: A qualifying set must be a subset of the target.
    let qualifying_sets = preexisting_sets
        .iter()
        .filter(|(_, set)| set.is_subset(target))
        .collect::<HashMap<_, _>>();

    // Next, we remove so-called "redundant qualifiers." A qualifier is considered redundant if
    // another qualifying set is a superset of it -- because if we take the union, all of the values
    // in this set will be included in one of the other sets, making this set unnecessary.
    let optimized_qualifiers = qualifying_sets
        .iter()
        .filter(|(key, value)| {
            // Filter out any values where this predicate is true for any value within qualifying_sets...
            !(qualifying_sets.iter().any(|(other_key, other_value)| {
                // ...except for the value itself.
                // FIXME: In the case that two qualifiers have identical member lists,
                //        (e.g.: @{everyone} where @everyone are all online so @everyone==@here),
                //        this will remove *both* sets. this is not catastrophic, as optimization
                //        is sorta "best-effort" and the outliers will be caught, but it's nice
                //        if we can fix this
                *key != other_key && value.is_subset(other_value)
            }))
        })
        .map(|(&a, &b)| (a, b))
        .collect::<HashMap<_, _>>();

    // Next, we calculate the union of all of the sets we qualified! This is used in calculating
    // outliers, later.
    let union_of_qualifiers = optimized_qualifiers
        .values()
        .copied() // TODO: remove copy
        .flatten()
        .copied() // TODO: remove copy
        .collect::<HashSet<_>>();

    let outliers = target
        .difference(&union_of_qualifiers)
        .copied() // TODO: remove copy
        .collect::<HashSet<_>>();

    UnionizeSetResult {
        sets: optimized_qualifiers,
        outliers,
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
            .collect::<Result<Vec<_>, _>>()?
            .into_iter(),
    ) else {
        return Ok(());
    };

    let members_to_ping =
        match drql::interpreter::interpret(ast.into(), &resolver, &UserData { msg, ctx }).await {
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

    // A hashmap of every role in the guild and its members.
    let mut roles_and_their_members: HashMap<RoleType, HashSet<serenity::UserId>> =
        HashMap::from([
            (RoleType::Everyone, discord_guild.get_everyone()),
            (RoleType::Here, discord_guild.get_here()),
        ]);

    for member in discord_guild.members.values() {
        for role in member.roles(ctx).ok_or(anyhow!("No role data??"))? {
            if let Some(entry) = roles_and_their_members.get_mut(&RoleType::Id(role.id)) {
                entry.insert(member.user.id);
            } else {
                roles_and_their_members
                    .insert(RoleType::Id(role.id), HashSet::from([member.user.id]));
            }
        }
    }

    // next, we represent the list of users as a bunch of roles containing them and one outliers set.
    let UnionizeSetResult { sets, outliers } =
        unionize_set(&members_to_ping, &roles_and_their_members);

    // if members_to_ping.len() > 50 {
    //     // TODO: Ask the user to confirm they wish to do this action
    // }

    // Now we need to split the output message into individual pings. First, stringify each user mention...
    enum MentionType {
        User(serenity::UserId),
        Role(RoleType),
    }
    impl Display for MentionType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MentionType::User(id) => write!(f, "<@{}>", id),
                MentionType::Role(role) => write!(f, "{}", role),
            }
        }
    }

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
        .map(|x| MentionType::Role(*x))
        .chain(outliers.into_iter().map(MentionType::User))
        .map(|x| x.to_string())
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
        let messages = chunk_str_vec_into_max_size(stringified_mentions, " ", 2000)?;
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
            commands: vec![ping(), about()],
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

                Ok(Data {})
            })
        });

    Ok(framework.run().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_str_vec_into_max_size_works() {
        let result = chunk_str_vec_into_max_size(
            vec![
                "abc".to_string(),
                "def".to_string(),
                "ghi".to_string(),
                "jkl".to_string(),
                "mno".to_string(),
            ],
            " ",
            7,
        )
        .unwrap();
        assert_eq!(
            result,
            vec![
                "abc def".to_string(),
                "ghi jkl".to_string(),
                "mno".to_string(),
            ]
        );

        let result = chunk_str_vec_into_max_size(
            ('A'..='Z').map(|l| l.to_string()).collect::<Vec<_>>(),
            " ",
            10,
        )
        .unwrap();
        assert_eq!(
            result,
            vec![
                "A B C D E".to_string(),
                "F G H I J".to_string(),
                "K L M N O".to_string(),
                "P Q R S T".to_string(),
                "U V W X Y".to_string(),
                "Z".to_string()
            ]
        );
    }

    #[test]
    fn chunk_str_vec_into_max_size_has_overflow() {
        println!(
            "{:?}",
            chunk_str_vec_into_max_size(vec!["ABCDEF".to_string()], " ", 5)
        );
        assert!(matches!(
            chunk_str_vec_into_max_size(vec!["ABCDEF".to_string()], " ", 5),
            Err(_)
        ));
    }

    #[test]
    fn unionize_set_works_with_superset_of_all() {
        let target = HashSet::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let preexisting_sets = HashMap::from([
            ("empty set", HashSet::from([])),
            ("not a subset", HashSet::from([1, 32, 5, 2, 6])),
            (
                "exact match",
                HashSet::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            ),
        ]);

        let UnionizeSetResult { sets, outliers } = unionize_set(&target, &preexisting_sets);
        assert_eq!(outliers.len(), 0);
        assert_eq!(
            sets,
            HashMap::from([(
                &"exact match",
                &HashSet::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
            )])
        );
    }

    #[test]
    fn unionize_set_works() {
        let target = HashSet::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let preexisting_sets = HashMap::from([
            ("empty set", HashSet::from([])), // gets optimized away
            ("not a subset", HashSet::from([1, 32, 5, 2, 6])), // gets disqualified
            ("subset of A", HashSet::from([1, 2, 3, 4])), // gets optimized away
            ("A", HashSet::from([1, 2, 3, 4, 5, 8])), // kept
            ("other subset of A", HashSet::from([4, 1, 8])), // gets optimized away
            ("B", HashSet::from([5, 9])),     // kept
        ]);

        let UnionizeSetResult { sets, outliers } = unionize_set(&target, &preexisting_sets);
        assert_eq!(outliers, HashSet::from([6, 7, 10]));
        assert_eq!(
            sets,
            HashMap::from([
                (&"A", &HashSet::from([1, 2, 3, 4, 5, 8])),
                (&"B", &HashSet::from([5, 9]))
            ])
        );
    }
}

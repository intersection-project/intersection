#![doc = include_str!("../README.md")]
#![warn(
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic,
    clippy::missing_docs_in_private_items,
    missing_docs
)]
#![allow(clippy::multiple_crate_versions, clippy::unused_async)]

mod commands;
mod drql;
mod extensions;
mod models;
mod resolver;
mod util;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(
    /// Direct access to the DRQL LALRPOP parser. Prefer to use the functions exported by drql::parser instead.
    #[allow(clippy::all)]
    #[allow(clippy::nursery)]
    #[allow(clippy::pedantic)]
    #[allow(missing_docs)]
    #[allow(clippy::missing_docs_in_private_items)]
    parser
);

use std::{collections::HashSet, env, ops::ControlFlow, sync::Arc};

use anyhow::{bail, Context as _};
use dotenvy::dotenv;
use poise::serenity_prelude as serenity;
use tracing::{debug, error, info, instrument, trace, warn};
use tracing_subscriber::prelude::*;

use crate::extensions::CustomGuildImpl;

/// Information collected when compiled, by crate `built`
pub mod build_info {
    // File is inserted by build.rs
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Global data passed around throughout the bot via the Context instance
pub struct Data {
    /// The framework.shard_manager, used to get the latency of the current shard in the ping command
    shard_manager: Arc<serenity::Mutex<serenity::ShardManager>>,
}
/// Type alias for the poise Context given our `Data` type
type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

/// Confirms the user is aware of the number of mentions that will be created by a query
///
/// Called usually when there is more than 50 in a query.
///
/// Will return Ok(Continue) if the user accepted, Ok(Break) if the user cancelled or timed out,
/// and Err if there was an error.
#[instrument(skip_all, fields(count = members_to_ping.len()))]
async fn confirm_mention_count(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    stringified_mentions: &Vec<String>,
    members_to_ping: &HashSet<serenity::UserId>,
) -> anyhow::Result<ControlFlow<(), ()>> {
    let serenity::Channel::Guild(channel) = msg.channel(ctx).await? else {
        // DMs would have been prevented already.
        // Messages can't be sent in categories
        bail!("unreachable");
    };

    trace!("sending confirmation message");

    let mut m = channel
        .send_message(ctx, |m| {
            m.content(format!(
                concat!(
                    "**Hold up!** By running this query, you are about to",
                    " mention {} people.{} Are you sure?"
                ),
                members_to_ping.len(),
                {
                    let len = util::wrap_string_vec(stringified_mentions, " ", 2000)
                        .unwrap() // TODO: Remove unwrap?
                        .len();
                    if len > 2 {
                        format!(" This will require the sending of {len} messages.")
                    } else {
                        String::new()
                    }
                }
            ))
            .reference_message(msg) // basically makes it a reply
            .components(|components| {
                components.create_action_row(|action_row| {
                    action_row
                        .create_button(|button| {
                            button
                                .custom_id("large_ping_confirm_no")
                                .emoji(serenity::ReactionType::Unicode("❌".to_string()))
                                .label("Cancel")
                                .style(serenity::ButtonStyle::Secondary)
                        })
                        .create_button(|button| {
                            button
                                .custom_id("large_ping_confirm_yes")
                                .emoji(serenity::ReactionType::Unicode("✅".to_string()))
                                .label("Yes")
                                .style(serenity::ButtonStyle::Primary)
                        })
                })
            })
        })
        .await?;

    trace!("waiting for confirmation");

    let Some(interaction) = m
        .await_component_interaction(ctx)
        .collect_limit(1)
        .author_id(msg.author.id)
        .timeout(std::time::Duration::from_secs(30))
        .await
    else {
        debug!("timed out waiting for confirmation");
        m.edit(ctx, |m| {
            m.content("Timed out waiting for confirmation.")
                .components(|components| components)
        })
        .await?;
        return Ok(ControlFlow::Break(()));
    };

    if interaction.data.custom_id == "large_ping_confirm_no" {
        debug!("User cancelled operation");
        m.edit(ctx, |m| {
            m.content("Cancelled.").components(|components| components)
        })
        .await?;

        return Ok(ControlFlow::Break(()));
    } else if interaction.data.custom_id == "large_ping_confirm_yes" {
        debug!("User confirmed operation");
        m.edit(ctx, |m| {
            m.content("Confirmed.").components(|components| components)
        })
        .await?;

        // continue normally!
        return Ok(ControlFlow::Continue(()));
    }

    bail!("unreachable");
}

/// Handle a DRQL query from a message, sending the response message(s) to the channel.
#[instrument(skip_all)]
async fn handle_drql_query(ctx: &serenity::Context, msg: &serenity::Message) -> anyhow::Result<()> {
    if msg.guild(ctx).is_none() {
        debug!("Ignoring DRQL query sent in DMs.");
        bail!("DRQL queries are not available in DMs.");
    }

    trace!("Parsing queries in message...");

    let ast = drql::scanner::scan(msg.content.as_str())
        .enumerate()
        .map(|(n, chunk)| {
            drql::parser::parse_drql(chunk).context(format!("Error parsing chunk {n}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .reduce(|acc, chunk| crate::drql::ast::Expr::Union(Box::new(acc), Box::new(chunk)))
        .context("There is no DRQL query in your message to handle.")?; // This should never happen, as we already checked that there was at least one chunk in the input

    debug!("Fully parsed and reduced AST: {ast:?}");

    trace!("Fetching guild, channel, and member information");
    let guild = msg.guild(ctx).context("Unable to resolve guild")?;
    let member = msg.member(ctx).await?;
    let serenity::Channel::Guild(channel) = msg.channel(ctx).await? else {
        // DMs would have been prevented already.
        // Messages can't be sent in categories
        bail!("unreachable");
    };

    trace!("Running DRQL interpreter on AST");
    let members_to_ping = drql::interpreter::interpret(
        ast,
        &mut resolver::Resolver {
            guild: &guild,
            member: &member,
            ctx,
            channel: &channel,
        },
    )
    .await
    .context("Error calculating result")?;

    debug!(
        "We need to mention these members: {:?}",
        members_to_ping.iter().map(|id| id.0).collect::<Vec<_>>()
    );

    // A hashmap of every role in the guild and its members.
    let roles_and_their_members = guild.all_roles_and_members(ctx)?;

    // next, we represent the list of users as a bunch of roles containing them and one outliers set.
    let util::unionize_set::UnionizeSetResult { sets, outliers } =
        util::unionize_set::unionize_set(&members_to_ping, &roles_and_their_members);

    debug!(
        "unionize_set result sets: {sets:?}, outliers: {outliers:?}",
        sets = sets,
        outliers = outliers
    );

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
        .into_iter()
        .copied()
        .map(models::mention::Mention::Role)
        .chain(
            outliers
                .into_iter()
                .map(|&id| models::mention::Mention::User(id)),
        )
        .map(|x| x.to_string())
        .collect::<Vec<_>>();

    debug!(
        "stringified_mentions: {stringified_mentions:?}",
        stringified_mentions = stringified_mentions
    );

    if stringified_mentions.is_empty() {
        debug!("Nobody to mention!");
        msg.reply(ctx, "No users matched.").await?;
        return Ok(());
    }

    if members_to_ping.len() > 50 {
        debug!("need to wait for user to confirm large mention");
        if let ControlFlow::Break(_) =
            confirm_mention_count(ctx, msg, &stringified_mentions, &members_to_ping).await?
        {
            debug!("User cancelled or timed out");
            // The user declined or the operation timed out. The message has already been edited for us.
            return Ok(());
        }
        debug!("User confirmed!");
    }

    let notification_string = format!(
        concat!(
            "Notification triggered by Intersection.\n",
            ":question: **What is this?** Run {} for more information.\n"
        ),
        util::mention_application_command(ctx, "about landing").await?
    );

    if stringified_mentions.join(" ").len() <= (2000 - notification_string.len()) {
        trace!("Sending single message for mentions");
        msg.reply(
            ctx,
            format!("{}{}", notification_string, stringified_mentions.join(" ")),
        )
        .await?;
    } else {
        let messages = util::wrap_string_vec(&stringified_mentions, " ", 2000)?;
        trace!("Need to send {} messages.", messages.len());
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
                util::mention_application_command(ctx, "about landing").await?
            ),
        )
        .await?;
    }

    trace!("Query handling completed!");

    Ok(())
}

/// Event handler for Intersection
struct Handler;
#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    #[instrument(skip_all, fields(author = msg.author.id.0, content = msg.content))]
    async fn message(&self, ctx: serenity::Context, msg: serenity::Message) {
        debug!("Received new message event");

        if msg.author.bot {
            debug!("Ignoring message from bot.");
            return;
        }

        if drql::scanner::scan(msg.content.as_str()).count() > 0 {
            debug!("Found DRQL queries in message! Handling queries.");
            match handle_drql_query(&ctx, &msg)
                .await
                .context("Error handling DRQL query")
            {
                Ok(_) => debug!("Finished handling queries."),

                Err(query_err) => {
                    // THIS IS NOT OUR FAULT -- This most likely means the USER made a mistake
                    debug!(
                        "An error occurred handling the DRQL query, notifying user: {query_err:#}"
                    );

                    if let Err(message_send_err) = msg.reply(&ctx, format!("{query_err:#}")).await {
                        warn!("An error occurred while notifying the user of a query error: {message_send_err:#}");
                        warn!("Initial query error: {query_err:#}");
                        debug!("Trying again...");

                        if let Err(double_message_send_err) = msg
                            .reply(
                                &ctx,
                                format!(
                                    concat!(
                                        "{query_err:#}\n",
                                        "Additionally, we attempted to send this error to you but this failed:",
                                        " {message_send_err:#}"
                                    ),
                                    query_err = query_err,
                                    message_send_err = message_send_err
                                ),
                            )
                            .await
                        {
                            // Oh god the error message.
                            error!("Failed to notify a user of an error notifying them of an error notifying them of a query error: {double_message_send_err:#}");
                            error!("We were attempting to notify them of this error: {message_send_err:#}");
                            error!("That error occurred while notifying them of this error: {query_err:#}");
                            error!("Message sending failed twice! Giving up.");
                        } else {
                            debug!("Alright, it worked that time.");
                        }
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // We ignore the error because environment variables may be passed
    // in directly, and .env might not exist (e.g. in Docker with --env-file)
    let _: Result<_, _> = dotenv();

    let filter = tracing_subscriber::EnvFilter::builder().parse(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "warn,intersection=info".to_string()),
    )?;

    // Note: We do not log spans by default, as they are very verbose.
    // To enable these, add the .with_span_events() call to the stdout_log layer.

    // Make sure to keep the _log_file_guard, if it goes out of scope the log file will be flushed and closed!
    // It's kept to be able to flush quickly in the case of abrupt process termination while stack is unwinding.
    let (non_blocking_log_file, _log_file_guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily("./logs", "intersection.log"),
    );

    let rolling_appender = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking_log_file);

    let stdout_log = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_log)
        .with(rolling_appender)
        .init();

    let framework: poise::FrameworkBuilder<Data, anyhow::Error> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::ping(),
                commands::about(),
                commands::debug(),
                commands::version(),
            ],

            ..Default::default()
        })
        .client_settings(|client| client.event_handler(Handler))
        .token(env::var("TOKEN").expect("Expected a token in the environment"))
        .intents(serenity::GatewayIntents::all())
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                info!(
                    "Logged in as {}#{}!",
                    ready.user.name, ready.user.discriminator
                );

                ctx.set_activity(serenity::Activity::watching("for queries".to_string())).await;

                info!("Registering global application (/) commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                info!("Finished registering global application (/) commands.");

                Ok(Data {
                    shard_manager: Arc::clone(framework.shard_manager()),
                })
            })
        });

    Ok(framework.run().await?)
}

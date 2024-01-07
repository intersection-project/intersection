use std::{borrow::Cow, fmt::Write as _};

use anyhow::{bail, Context as _};
use poise::serenity_prelude::{self as serenity};
use tracing::{debug, trace};

use super::super::Context;
use crate::{extensions::CustomGuildImpl, models, parse_and_evaluate_query, util};

/// Run a DRQL query and test what it would do
#[poise::command(slash_command, ephemeral)]
#[allow(clippy::too_many_lines)]
pub async fn dry_run(
    ctx: Context<'_>,
    #[description = "The query you would like to test"] query: String,
) -> Result<(), anyhow::Error> {
    if ctx.guild().is_none() {
        debug!("Ignoring DRQL query sent in DMs.");
        bail!("DRQL queries are not available in DMs.");
    }

    trace!("Fetching guild, channel, and member information");
    let guild = ctx.guild().context("Unable to resolve guild")?;
    let member = ctx.author_member().await.context("Error fetching member")?;
    let channel = ctx
        .guild_channel()
        .await
        .context("Error fetching channel")?;

    trace!("Running DRQL parser/interpreter on message");
    let members_to_ping =
        parse_and_evaluate_query(ctx.serenity_context(), &[&query], &guild, &member, &channel)
            .await?;

    // A hashmap of every role in the guild and its members.
    let roles_and_their_members = guild.all_roles_and_members(ctx.serenity_context())?;

    // next, we represent the list of users as a bunch of roles containing them and one outliers set.
    let util::unionize_set::UnionizeSetResult { sets, outliers } =
        util::unionize_set::unionize_set(&members_to_ping, &roles_and_their_members);

    debug!(
        "unionize_set result sets: {sets:?}, outliers: {outliers:?}",
        sets = sets,
        outliers = outliers
    );

    // Now stringify solely the USERS we want to ping...
    let stringified_mentions = members_to_ping
        .iter()
        .map(|id| models::mention::Mention::User(*id).to_string())
        .collect::<Vec<_>>();

    debug!("dry run result: {stringified_mentions:?}");

    if stringified_mentions.is_empty() {
        debug!("Nobody to mention!");
        ctx.say("Your query matches 0 users.").await?;
        return Ok(());
    }

    let message_count_if_optimized = util::wrap_string_vec(
        &sets
            .iter()
            .copied()
            .map(|id| models::mention::Mention::Role(*id))
            .chain(
                outliers
                    .iter()
                    .map(|&id| models::mention::Mention::User(*id)),
            )
            .map(|x| x.to_string())
            .collect::<Vec<_>>(),
        " ",
        2000,
    )?
    .len();

    debug!(
        "stringified_mentions: {stringified_mentions:?}",
        stringified_mentions = stringified_mentions
    );

    let message_header = format!(
        "Your query matches the following {} users:\n",
        stringified_mentions.len()
    );
    let message_footer = format!(
        concat!(
            "\n\nThis will require sending {} messages.",
            " (optimized by pinging {} roles, saving you {} mentions)."
        ),
        message_count_if_optimized,
        sets.len(),
        stringified_mentions.len() - (sets.len() + outliers.len())
    );

    if stringified_mentions.join(" ").len() <= (2000 - message_header.len() - message_footer.len())
    {
        debug!("All mentions fit in one message!");
        ctx.say(format!(
            "{}{}{}",
            message_header,
            stringified_mentions.join(" "),
            message_footer
        ))
        .await?;
        return Ok(());
    }

    debug!("Mentions do not fit in one message, using text file");

    let mut file_contents = String::new();
    for id in &members_to_ping {
        let member = guild.member(ctx.serenity_context(), *id).await?;
        writeln!(
            &mut file_contents,
            "{}#{} ({})",
            member.user.name, member.user.discriminator, member.user.id
        )?;
    }

    ctx.send(|builder| {
        builder
            .content(format!(
                concat!(
                    "Your query matches the attached {} users.",
                    " This will require sending {} messages",
                    " (optimized by pinging {} roles, saving you {} mentions)."
                ),
                stringified_mentions.len(),
                message_count_if_optimized,
                sets.len(),
                stringified_mentions.len() - (sets.len() + outliers.len())
            ))
            .attachment(serenity::AttachmentType::Bytes {
                data: Cow::Borrowed(file_contents.as_bytes()),
                filename: "dry_run.txt".to_string(),
            })
    })
    .await?;

    Ok(())
}

use crate::util;

use super::super::super::Context;

/// Learn some of the DRQL syntax and how to use it!
#[poise::command(slash_command)]
pub async fn drql(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    let serenity_ctx = ctx.serenity_context();

    // Way too big. We'll send two messages.

    let reply_handle = ctx
        .say(format!(
            include_str!("./drql_1.md"),
            cmd_about_set_theory =
                util::mention_application_command(serenity_ctx, "about set_theory").await?,
            bot_user_id = serenity_ctx.cache.current_user_id(),
        ))
        .await?;

    // I think this is the closest thing to Discord.js Interaction#followUp...
    reply_handle
        .into_message()
        .await?
        .reply(
            ctx,
            format!(
                include_str!("./drql_2.md"),
                cmd_about_how_it_works =
                    util::mention_application_command(serenity_ctx, "about how_it_works").await?
            ),
        )
        .await?;

    Ok(())
}

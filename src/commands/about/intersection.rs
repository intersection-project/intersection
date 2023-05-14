use super::super::super::Context;
use crate::util;

/// Learn about what the Intersection Project is
#[poise::command(slash_command)]
pub async fn intersection(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    let serenity_ctx = ctx.serenity_context();

    // Way too big. Two messages!

    let reply_handle = ctx
        .say(format!(
            include_str!("./intersection_1.md"),
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
                include_str!("./intersection_2.md"),
                cmd_about_set_theory =
                    util::mention_application_command(serenity_ctx, "about set_theory").await?,
                cmd_about_drql =
                    util::mention_application_command(serenity_ctx, "about drql").await?,
                cmd_about_how_it_works =
                    util::mention_application_command(serenity_ctx, "about how_it_works").await?,
            ),
        )
        .await?;

    Ok(())
}

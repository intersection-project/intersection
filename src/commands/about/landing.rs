use super::super::super::Context;
use crate::util;

/// I just got pinged by Intersection, what does this mean?
#[poise::command(slash_command)]
pub async fn landing(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    ctx.say(format!(
        include_str!("./landing.md"),
        cmd_about_intersection =
            util::mention_application_command(ctx.serenity_context(), "about intersection").await?
    ))
    .await?;
    Ok(())
}

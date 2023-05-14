use super::super::super::Context;
use crate::util;

/// Learn some basic set theory and how it applies to Intersection and Discord
#[poise::command(slash_command)]
pub async fn set_theory(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    ctx.say(format!(
        include_str!("./set_theory.md"),
        cmd_about_drql =
            util::mention_application_command(ctx.serenity_context(), "about drql").await?
    ))
    .await?;
    Ok(())
}

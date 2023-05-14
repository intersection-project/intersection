use super::super::Context;

/// See what version of Intersection and our dependencies we're running
#[poise::command(slash_command)]
pub async fn version(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    ctx.say("I'm currently running Intersection version 1.0.0. [google](<https://google.com>)")
        .await?;
    Ok(())
}

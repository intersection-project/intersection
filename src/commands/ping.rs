use super::super::Context;

/// Check if Intersection is online
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    // TODO: get current bot ping
    ctx.say("I'm alive!").await?;
    Ok(())
}

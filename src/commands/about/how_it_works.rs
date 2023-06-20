use crate::util;

use super::super::super::Context;

/// Learn about how Intersection works, and how we could use your help!
#[poise::command(slash_command)]
pub async fn how_it_works(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    let serenity_ctx = ctx.serenity_context();

    ctx.say(format!(
        include_str!("./how_it_works.md"),
        cmd_version = util::mention_application_command(serenity_ctx, "version").await?,
        cmd_debug_parse_one =
            util::mention_application_command(serenity_ctx, "debug parse_one").await?,
        repo_url = super::super::super::build_info::PKG_REPOSITORY,
    ))
    .await?;
    Ok(())
}

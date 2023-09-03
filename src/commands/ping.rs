use anyhow::Context as _;
use poise::serenity_prelude::ShardId;

use super::super::Context;

/// Check if Intersection is online
#[poise::command(slash_command)]
#[allow(clippy::significant_drop_tightening)] // faulty rule in this case i think -- needs investigation
pub async fn ping(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    let m = ctx.say("Ping?").await?;

    let poise::Context::Application(new_ctx) = ctx else {
        panic!();
    };

    let diff_ms = m.message().await?.timestamp.timestamp_millis()
        - new_ctx.interaction.id().created_at().timestamp_millis();

    // See https://github.com/serenity-rs/serenity/blob/6e2e70766e1afbce9cd1d4b43e3c6ee3b474f0bf/examples/e05_command_framework/src/main.rs#L468
    let shard_latency = {
        let shard_manager = ctx.data().shard_manager.lock().await;
        let runners = shard_manager.runners.lock().await;
        runners
            .get(&ShardId(ctx.serenity_context().shard_id))
            .context("Failed to obtain current shard")?
            .latency
    };

    m.edit(ctx, |r| {
        r.content(format!(
            "Pong :ping_pong:! (Round trip: {}ms. Heartbeat: {}.)",
            diff_ms,
            shard_latency.map_or_else(|| "unknown".to_string(), |l| format!("{}ms", l.as_millis()))
        ))
    })
    .await?;

    Ok(())
}

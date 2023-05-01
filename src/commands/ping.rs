use poise::serenity_prelude::ShardId;

use super::super::Context;

/// Check if Intersection is online
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    let m = ctx.say("Ping?").await?;

    let poise::Context::Application(new_ctx) = ctx else {
        panic!();
    };

    let diff_ms = m.message().await?.timestamp.timestamp_millis()
        - new_ctx.interaction.id().created_at().timestamp_millis();

    // See https://github.com/serenity-rs/serenity/blob/6e2e70766e1afbce9cd1d4b43e3c6ee3b474f0bf/examples/e05_command_framework/src/main.rs#L468
    // TODO: A lot of locking here. Could this block and be dangerous?
    let shard_manager = ctx.data().shard_manager.lock().await;
    let runners = shard_manager.runners.lock().await;
    let current_shard = runners.get(&ShardId(ctx.serenity_context().shard_id));

    m.edit(ctx, |r| {
        r.content(format!(
            "Pong :ping_pong:! (Round trip: {}ms. {})",
            diff_ms,
            if let Some(info) = current_shard {
                if let Some(latency) = info.latency {
                    format!("Heartbeat: {}ms.", latency.as_millis())
                } else {
                    "No heartbeat available.".to_string()
                }
            } else {
                "Failed to obtain current shard.".to_string()
            }
        ))
    })
    .await?;

    Ok(())
}

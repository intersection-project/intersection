use std::{env, fs};

use dotenv::dotenv;
use serenity::{
    async_trait,
    model::prelude::{Activity, Message, Ready},
    prelude::*,
};
use sqlx::SqlitePool;

struct DB;

impl TypeMapKey for DB {
    type Value = SqlitePool;
}

struct Handler;

macro_rules! message_channel_send {
    ($ctx:ident, $msg:ident, $content:expr) => {
        if let Err(why) = $msg.channel_id.say(&$ctx.http, $content).await {
            println!("Error sending message: {:?}", why);
        }
    };
}

macro_rules! message_reply {
    ($ctx:ident, $msg:ident, $content:expr) => {
        if let Err(why) = $msg.reply(&$ctx.http, $content).await {
            println!("Error sending message: {:?}", why);
        }
    };
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if msg.channel(&ctx.http).await.unwrap().guild().is_none() {
            message_reply!(ctx, msg, "This bot only works in servers.");
            return;
        }

        if msg.content == "!ping" {
            message_channel_send!(ctx, msg, "Pong!");
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Logged in as {}!", ready.user.tag());
        ctx.set_activity(Activity::watching("for custom mentions"))
            .await;
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv()?;

    let conn = SqlitePool::connect("sqlite:./data.sqlite?mode=rwc").await?;
    sqlx::query!("CREATE TABLE IF NOT EXISTS guilds (prefix TEXT);")
        .execute()
        .await?;

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(
        env::var("TOKEN").expect("Expected a token in the environment"),
        intents,
    )
    .event_handler(Handler)
    .await?;

    {
        let mut data = client.data.write().await;
        data.insert::<DB>(conn);
    }

    client.start().await?;

    Ok(())
}

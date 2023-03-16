mod models;
mod schema;

use std::env;

use anyhow::anyhow;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection,
};
use dotenvy::dotenv;
use models::Guild;
use serenity::{
    async_trait,
    model::prelude::{Activity, Message, Ready},
    prelude::*,
};

struct DB;

impl TypeMapKey for DB {
    type Value = Pool<ConnectionManager<SqliteConnection>>;
}

struct Handler;

/// Function called whenever a message event is triggered. This can return an Anyhow Error
/// which is displayed to the user.
async fn handle_message(ctx: &Context, msg: &Message) -> anyhow::Result<()> {
    if msg.author.bot {
        return Ok(());
    }

    let _guild = schema::guilds::table
        .filter(
            schema::guilds::id.eq(msg
                .guild_id
                .ok_or(anyhow!("msg.guild_id was None"))?
                .to_string()),
        )
        .first::<Guild>(
            &mut ctx
                .data
                .read()
                .await
                .get::<DB>()
                .ok_or(anyhow!("DB was None"))?
                .get()?,
        )?;

    if msg.channel(&ctx.http).await?.guild().is_none() {
        msg.reply(&ctx.http, "This bot only works in servers.")
            .await?;
        return Ok(());
    }

    if msg.content == "!ping" {
        msg.channel_id.say(&ctx.http, "Pong!").await?;
    }

    Ok(())
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let result = handle_message(&ctx, &msg).await;

        if let Err(e) = result {
            if let Err(e2) = msg
                .reply(
                    &ctx.http,
                    format!(
                        "An internal error occurred while processing your command: {}",
                        e
                    ),
                )
                .await
            {
                println!("An error occurred while handling an error. {:?}", e2);
            }
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

    let database_url = env::var("DATABASE_URL").expect("Expected DATABASE_URL in the environment");
    let pool = Pool::builder()
        .test_on_check_out(true)
        .build(ConnectionManager::<SqliteConnection>::new(database_url))?;

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
        data.insert::<DB>(pool);
    }

    client.start().await?;

    Ok(())
}

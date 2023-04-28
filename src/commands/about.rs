use super::super::Context;

/// Learn about the Intersection Project
#[poise::command(slash_command)]
pub async fn about(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    ctx.say(concat!(
        "The Intersection Project is a Discord bot with the purpose of supercharging Discord",
        " mentions. Intersection empowers Discord with a language called DRQL -- the Discord",
        " Role Query Language. DRQL is a very simple set-theory based query language that",
        " allows you to make advanced operations on roles.\n",
        "\n",
        "An example of a DRQL query might be `@{ admins + (mods & here) }`. This query represents",
        " some very basic set theory concepts, namely the union and **intersection** operations.",
        " The following operators are available in DRQL:\n",
        "- `A + B` or `A | B`: Union -- Members with either role A or role B (equivalent to if you",
        " were to ping @A and @B)\n",
        "- `A & B`: Intersection -- Members with *both* roles A and B (something Discord does not",
        " provide a native facility for\n",
        "- `A - B`: Difference -- Members with role A but *not* role B\n",
        "\n",
        "Intersection functions by searching for messages that contain DRQL queries wrapped in @{...}",
        " and then calculating and replying to your message by pinging every user that that query matched.",
        " The bot will also attempt to ping *roles* that match the query (to help keep the resulting",
        " message short) but this is not always possible. The availability of so-called \"optimized",
        " result messages\" depends on the query and the roles in your server.\n",
        "\n",
        "Intersection is still a project in-development. If you have any questions, comments, or",
        " suggestions, please feel free to reach out!"
    ))
    .await?;
    Ok(())
}

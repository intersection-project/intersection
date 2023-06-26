use super::super::drql;
use super::super::Context;
use anyhow::bail;

/// Debug DRQL queries or the DRQL facilities itself
#[poise::command(slash_command, subcommands("scan", "parse_one", "reduce"))]
pub async fn debug(_ctx: Context<'_>) -> Result<(), anyhow::Error> {
    bail!("unreachable");
}

/// Scan input text for DRQL queries
#[poise::command(slash_command)]
pub async fn scan(
    ctx: Context<'_>,
    #[description = "The message to scan for queries in"] msg: String,
) -> Result<(), anyhow::Error> {
    let chunks = drql::scanner::scan(msg.as_str()).collect::<Vec<_>>();

    if chunks.is_empty() {
        ctx.say("No chunks were scanned.").await?;
    } else {
        ctx.say(format!(
            "Found {} chunks:\n\n{}",
            chunks.len(),
            chunks
                .iter()
                .map(|x| format!("`{x}`"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
        .await?;
    }

    Ok(())
}

/// Parse a single DRQL query
#[poise::command(slash_command)]
pub async fn parse_one(
    ctx: Context<'_>,
    #[description = "The DRQL query to parse (DO NOT include @{})"] query: String,
) -> Result<(), anyhow::Error> {
    ctx.say(match drql::parser::parse_drql(query.as_str()) {
        Err(e) => format!("Encountered an error while parsing:\n\n```{e:?}```"),
        Ok(ast) => format!("Successfully parsed:\n\n```{ast:?}```"),
    })
    .await?;

    Ok(())
}

/// Scan the input, parse each query, and finally reduce into one tree
#[poise::command(slash_command)]
pub async fn reduce(
    ctx: Context<'_>,
    #[description = "The message to scan"] msg: String,
) -> Result<(), anyhow::Error> {
    ctx.say(
        match drql::scanner::scan(msg.as_str())
            .map(drql::parser::parse_drql)
            // TODO: Report errors as 'error in chunk X'?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .reduce(|acc, chunk| crate::drql::ast::Expr::Union(Box::new(acc), Box::new(chunk)))
        {
            None => "No chunks found.".to_string(),
            Some(ast) => format!("Success! Resulting AST:\n\n```{ast:?}```"),
        },
    )
    .await?;

    Ok(())
}

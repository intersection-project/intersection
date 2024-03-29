use anyhow::{bail, Context as _};

use super::super::{drql, Context};
use crate::drql::ast::Expr;

/// Debug DRQL queries or the DRQL facilities itself
#[poise::command(slash_command, subcommands("scan", "parse_one", "reduce"))]
pub async fn debug(_ctx: Context<'_>) -> Result<(), anyhow::Error> {
    bail!("unreachable");
}

/// Scan input text for DRQL queries
#[poise::command(slash_command)]
async fn scan(
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
async fn parse_one(
    ctx: Context<'_>,
    #[description = "The DRQL query to parse (DO NOT include @{})"] query: String,
) -> Result<(), anyhow::Error> {
    ctx.say(match drql::parser::parse_drql(query.as_str()) {
        Err(err) => format!("Encountered an error while parsing:\n\n```{err:?}```"),
        Ok(ast) => format!("Successfully parsed:\n\n```{ast:?}```"),
    })
    .await?;

    Ok(())
}

/// Scan the input, parse each query, and finally reduce into one tree
#[poise::command(slash_command)]
async fn reduce(
    ctx: Context<'_>,
    #[description = "The message to scan"] msg: String,
) -> Result<(), anyhow::Error> {
    ctx.say(
        match drql::scanner::scan(msg.as_str())
            .enumerate()
            .map(|(n, chunk)| {
                drql::parser::parse_drql(chunk).context(format!("Error parsing chunk {n}"))
            })
            .collect::<Result<Vec<_>, _>>()
        {
            Err(err) => format!("Encountered an error while parsing:\n\n```{err:#}```"),
            Ok(ast) => ast
                .into_iter()
                .reduce(|acc, chunk| Expr::Union(Box::new(acc), Box::new(chunk)))
                .map_or_else(
                    || "No chunks found.".to_string(),
                    |ast| format!("Success! Resulting AST:\n\n```{ast:?}```"),
                ),
        },
    )
    .await?;

    Ok(())
}

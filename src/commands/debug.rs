use anyhow::{bail, Context as _};
use poise::AutocompleteChoice;
use tracing::{debug, instrument};

use super::super::{drql, Context};

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

#[instrument(skip_all, fields(query = query))]
async fn parse_one_autocomplete(_ctx: Context<'_>, query: &str) -> Vec<AutocompleteChoice<String>> {
    match drql::parser::parse_drql(query) {
        // When we encounter an error, we want to split it across multiple lines because Discord only
        // gives us 100 characters for the 'name' field in AutocompleteChoice. We split as follows:
        // 1. We always split on newlines within the error message.
        // 2. For a single line of the error message, we split on whitespace.

        // TODO: Move this to a function? It's sorta repetitive
        Err(e) => {
            debug!("Returning parse error response to autocomplete: {e:#}");
            format!("Encountered an error while parsing:\n{e:#}")
                .split('\n')
                .flat_map(|part| {
                    crate::util::wrap_string_vec(
                        &part
                            .split_whitespace()
                            .map(std::string::ToString::to_string)
                            .collect::<Vec<_>>(),
                        " ",
                        100,
                    )
                    .unwrap()
                })
                .map(|option| AutocompleteChoice {
                    name: option,
                    value: query.to_string(),
                })
                .collect::<Vec<_>>()
        }
        Ok(_) => {
            debug!("Returning \"Parsed successfully\" response to autocomplete");
            vec![AutocompleteChoice {
                name: "Parsed successfully. Send command to view AST.".to_string(),
                value: query.to_string(),
            }]
        }
    }
}

/// Parse a single DRQL query
#[instrument(skip_all, fields(query = query))]
#[poise::command(slash_command)]
async fn parse_one(
    ctx: Context<'_>,

    #[description = "The DRQL query to parse (DO NOT include @{})"]
    #[autocomplete = "parse_one_autocomplete"]
    query: String,
) -> Result<(), anyhow::Error> {
    ctx.say(match drql::parser::parse_drql(query.as_str()) {
        Err(e) => format!("Encountered an error while parsing:\n\n```{e:?}```"),
        Ok(ast) => format!("Successfully parsed:\n\n```{ast:?}```"),
    })
    .await?;

    Ok(())
}

#[instrument(skip_all, fields(query = query))]
async fn reduce_autocomplete(_ctx: Context<'_>, query: &str) -> Vec<AutocompleteChoice<String>> {
    match drql::scanner::scan(query)
        .enumerate()
        .map(|(n, chunk)| {
            drql::parser::parse_drql(chunk).context(format!("Error parsing chunk {n}"))
        })
        .collect::<Result<Vec<_>, _>>()
    {
        // The same whitespace printing is done here. First split on newlines, then split on spaces.
        Err(e) => {
            debug!("Returning parse error response to autocomplete: {e:#}");
            format!("Encountered an error while parsing:\n{e:#}")
                .split('\n')
                .flat_map(|part| {
                    crate::util::wrap_string_vec(
                        &part
                            .split_whitespace()
                            .map(std::string::ToString::to_string)
                            .collect::<Vec<_>>(),
                        " ",
                        100,
                    )
                    .unwrap()
                })
                .map(|option| AutocompleteChoice {
                    name: option,
                    value: query.to_string(),
                })
                .collect::<Vec<_>>()
        }

        Ok(exprs) if exprs.is_empty() => {
            debug!("Returning \"No chunks found\" response to autocomplete");
            vec![AutocompleteChoice {
                name: "No chunks found.".to_string(),
                value: query.to_string(),
            }]
        }
        Ok(_) => {
            debug!("Returning \"Parsed successfully\" response to autocomplete");
            vec![AutocompleteChoice {
                name: "Parsed successfully. Send command to view reduced AST.".to_string(),
                value: query.to_string(),
            }]
        }
    }
}

/// Scan the input, parse each query, and finally reduce into one tree
#[instrument(skip_all, fields(msg = msg))]
#[poise::command(slash_command)]
async fn reduce(
    ctx: Context<'_>,

    #[description = "The message to scan"]
    #[autocomplete = "reduce_autocomplete"]
    msg: String,
) -> Result<(), anyhow::Error> {
    ctx.say(
        match drql::scanner::scan(msg.as_str())
            .enumerate()
            .map(|(n, chunk)| {
                drql::parser::parse_drql(chunk).context(format!("Error parsing chunk {n}"))
            })
            .collect::<Result<Vec<_>, _>>()
        {
            Err(e) => format!("Encountered an error while parsing:\n\n```{e:#}```"),
            Ok(ast) => ast
                .into_iter()
                .reduce(|acc, chunk| crate::drql::ast::Expr::Union(Box::new(acc), Box::new(chunk)))
                .map_or_else(
                    || "No chunks found.".to_string(),
                    |ast| format!("Success! Resulting AST:\n\n```{ast:?}```"),
                ),
        },
    )
    .await?;

    Ok(())
}

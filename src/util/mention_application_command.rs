use anyhow::Context as _;
use log::warn;
use poise::serenity_prelude as serenity;

/// Find the application command `/name` and return the string mentioning that application command.
///
/// If the name contains spaces, the first word is the command name and the rest is the subcommand name.
///
/// If the command is not found, it returns a code block containing the command name and prints
/// a warning.
pub async fn mention_application_command(
    ctx: &serenity::Context,
    command_string: &str,
) -> Result<String, anyhow::Error> {
    let command_name = command_string
        .split_once(' ')
        .map_or(command_string, |(name, _)| name);

    let command =
        serenity::model::application::command::Command::get_global_application_commands(ctx)
            .await
            .context("Error looking up global application commands!")?
            .into_iter()
            .find(|command| command.name == command_name);

    if let Some(command) = command {
        Ok(format!("</{}:{}>", command_string, command.id.0))
    } else {
        warn!("Attempt to mention the command \"{command_string}\" (root command {command_name}) which was not found!");
        Ok(format!("`/{command_string}`"))
    }
}

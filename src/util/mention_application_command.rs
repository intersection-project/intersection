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
        .map(|(name, _)| name)
        .unwrap_or(command_string);

    let command =
        serenity::model::application::command::Command::get_global_application_commands(ctx)
            .await
            .map_err(|_| anyhow::anyhow!("Error looking up global application commands!"))?
            .into_iter()
            .find(|command| command.name == command_name);

    match command {
        Some(command) => Ok(format!("</{}:{}>", command_string, command.id.0)),
        None => {
            println!("WARN: Attempt to mention the command \"{}\" (root command {}) which was not found!", command_string, command_name);
            Ok(format!("`/{}`", command_string))
        }
    }
}

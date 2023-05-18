use super::super::build_info;
use super::super::Context;
use anyhow::Context as _;
use chrono::DateTime;

/// Returns [Display Name](crate url) vCRATE_VERSION
fn crate_version(display_name: &str, crate_name: &str) -> String {
    let version = build_info::DEPENDENCIES
        .iter()
        .find(|(name, _)| *name == crate_name)
        .map(|(_, version)| format!("v{version}"));

    format!(
        "[{display_name}](https://crates.io/crates/{crate_name}{crate_version_suffix}) {crate_version_string}",
        crate_version_suffix = match &version {
            None => "".to_string(),
            Some(version) => format!("/{}", version),
        },
        crate_version_string = version
            .unwrap_or("(unknown version)".to_string())
    )
}

/// See what version of Intersection and our dependencies we're running
#[poise::command(slash_command)]
pub async fn version(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    ctx.say(format!(
        concat!(
            "Intersection v{version}{git_str}, compiled by {rustc_version} for {target} ({profile} build) on <t:{epoch}:F> (<t:{epoch}:R>)\n",
            "\n",
            "Powered by:\n",
            "{lalrpop_version}\n",
            "{logos_version}\n",
            "{poise_version}\n",
            "{serenity_version}\n",
        ),
        version = build_info::PKG_VERSION,
        git_str = match build_info::GIT_VERSION {
            None => "".to_string(),
            Some(tag) => format!(
                " (git {tag}{dirty_str})",
                dirty_str = match build_info::GIT_DIRTY {
                    None => "",
                    Some(false) => "",
                    Some(true) => ", dirty source tree"
                }
            )
        },
        rustc_version = build_info::RUSTC_VERSION,
        target = build_info::TARGET,
        profile = build_info::PROFILE,

        lalrpop_version = crate_version("LALRPOP", "lalrpop"),
        logos_version = crate_version("Logos", "logos"),
        poise_version = crate_version("Poise", "poise"),
        serenity_version = crate_version("Serenity", "serenity"),

        epoch = DateTime::parse_from_rfc2822(build_info::BUILT_TIME_UTC).context("Invalid build time string in build info")?.timestamp(),
    ))
    .await?;
    Ok(())
}

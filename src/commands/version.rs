use anyhow::Context as _;
use chrono::DateTime;

use super::super::{build_info, Context};

/// Returns `[Display Name](crate url) vCRATE_VERSION`
fn crate_version(display_name: &str, crate_name: &str) -> String {
    let version = build_info::DEPENDENCIES
        .iter()
        .find(|(name, _)| *name == crate_name)
        .map(|(_, version)| format!("v{version}"));

    format!(
        "[{display_name}](https://crates.io/crates/{crate_name}{crate_version_suffix}) {crate_version_string}",
        crate_version_suffix = version.clone().map_or_else(String::new, |version| format!("/{}", &version[1..])),
        crate_version_string = version
            .unwrap_or_else(|| "(unknown version)".to_string())
    )
}

/// See what version of Intersection and our dependencies we're running
#[poise::command(slash_command)]
#[allow(clippy::const_is_empty)]
pub async fn version(ctx: Context<'_>) -> Result<(), anyhow::Error> {
    let dirty_str = if build_info::GIT_DIRTY.unwrap_or(false) {
        ", dirty source tree"
    } else {
        ""
    };
    let git_str = build_info::GIT_COMMIT_HASH_SHORT
        .zip(build_info::GIT_COMMIT_HASH)
        .map_or(String::new(), |(short, long)| {
            format!(
                " (git {commit_hash_or_link}{dirty_str})",
                commit_hash_or_link = if build_info::PKG_REPOSITORY.is_empty() {
                    format!("`{short}`")
                } else {
                    format!(
                        "[`{short}`]({repo}/tree/{long})",
                        repo = build_info::PKG_REPOSITORY
                    )
                }
            )
        });

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
        rustc_version = build_info::RUSTC_VERSION,
        target = build_info::TARGET,
        profile = build_info::PROFILE,
        git_str = git_str,

        lalrpop_version = crate_version("LALRPOP", "lalrpop"),
        logos_version = crate_version("Logos", "logos"),
        poise_version = crate_version("Poise", "poise"),
        serenity_version = crate_version("Serenity", "serenity"),

        epoch = DateTime::parse_from_rfc2822(build_info::BUILT_TIME_UTC).context("Invalid build time string in build info")?.timestamp(),
    ))
    .await?;
    Ok(())
}

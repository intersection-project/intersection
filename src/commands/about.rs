mod drql;
mod how_it_works;
mod intersection;
mod landing;
mod set_theory;

use anyhow::bail;

use super::super::Context;

/// Learn about Intersection, how it works, and how to use it!
#[poise::command(
    slash_command,
    subcommands(
        "landing::landing",
        "intersection::intersection",
        "set_theory::set_theory",
        "drql::drql",
        "how_it_works::how_it_works"
    )
)]
pub async fn about(_ctx: Context<'_>) -> Result<(), anyhow::Error> {
    bail!("unreachable");
}

//! Intersection's front-facing commands

#![allow(
    clippy::missing_docs_in_private_items, // because we don't expect all of these small modules to have docs
    clippy::unused_async // all commands must be async fn
)]

mod about;
mod debug;
mod ping;
mod version;

pub use about::about;
pub use debug::debug;
pub use ping::ping;
pub use version::version;

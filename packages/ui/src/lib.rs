//! This crate contains all shared UI for the workspace.

// Dioxus `rsx!` macro expands to unwraps internally; allow to avoid false positives.
#![allow(clippy::disallowed_methods)]

mod hero;
pub use hero::Hero;

mod navbar;
pub use navbar::Navbar;

mod echo;
pub use echo::Echo;

pub mod admin;

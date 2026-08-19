//! Individual command handlers pulled out of `main.rs` for SRP (2026-08-19) -- each submodule
//! owns one cohesive group of commands' full request-to-response handling plus its own tests,
//! matching the pattern already established by `compare.rs`/`context_menu.rs`/
//! `frame_analysis.rs`/etc. (which sit one level up, in `src/`, not under `commands/` -- they
//! predate this pass and own real domain logic beyond thin request/response wiring, so they
//! weren't moved here).

pub mod data_plane;
pub mod selection;
pub mod stream;
pub mod stream_query;

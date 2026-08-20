//! Flow's local task store, extracted from the GPUI app's own `src/db.rs`
//! so it's reusable by any frontend, not just GPUI — a real, verified
//! extraction (`db.rs` never imported `gpui` in the first place; every
//! `crate::db::…` reference throughout the GPUI app still resolves
//! unchanged, via `src/lib.rs`'s `pub use flow_data::db;`), done as the
//! first concrete step of
//! [`wayfinder/tickets/migrate-to-tauri.md`](../../wayfinder/tickets/migrate-to-tauri.md)'s
//! open "how does the Tauri frontend reach Flow's real data" question —
//! `eventkit.rs`/`platform.rs`'s calendar functions and `parse.rs` are
//! equally GPUI-free and equally movable here, just not moved yet.

pub mod db;
pub mod parse;

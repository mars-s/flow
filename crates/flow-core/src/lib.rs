//! Generic, reusable infrastructure kept from Flow's original daemon core.
//!
//! Everything agent/daemon-specific (session drivers, persistence, Git and
//! workspace automation, the daemon RPC server, usage tracking, and so on)
//! was deleted as part of stripping Flow down from the Flow coding-agent
//! backend. Only locale plumbing and application identity survived as
//! genuinely generic and worth keeping.

rust_i18n::i18n!("../../locales", fallback = "en");

pub mod i18n;
pub mod identity;

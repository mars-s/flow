//! Shared application identity.

#[cfg(debug_assertions)]
pub const APP_NAME: &str = "Flow Dev";
#[cfg(not(debug_assertions))]
pub const APP_NAME: &str = "Flow";

#[cfg(debug_assertions)]
pub const APP_ID: &str = "sh.flow.dev";
#[cfg(not(debug_assertions))]
pub const APP_ID: &str = "sh.flow";

#[cfg(debug_assertions)]
pub const DATA_DIRECTORY_NAME: &str = "Flow Dev";
#[cfg(not(debug_assertions))]
pub const DATA_DIRECTORY_NAME: &str = "Flow";

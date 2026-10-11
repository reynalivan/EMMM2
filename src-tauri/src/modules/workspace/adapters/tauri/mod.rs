pub mod folder_entries_cmds;
pub mod scanner_conflict_cmds;
pub(crate) mod watcher_activation;
pub(crate) mod watcher_runtime;
#[cfg(feature = "watcher-verification")]
mod watcher_verification;
pub mod workspace_cmds;

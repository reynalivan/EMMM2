pub(crate) use super::adapters::tauri::watcher_activation::{
    publish_pending_activation_when_recovered, try_reuse_active_watcher_for_activation,
};
pub(crate) use super::adapters::tauri::watcher_runtime::{
    inactive_activation_is_ready, start_inactive_watcher_with_activation_guard, start_watcher,
    start_watcher_for_activation,
};
pub(crate) use super::application::scanner::watcher::lifecycle::{
    WatcherActivation, WatcherInstallScope,
};
pub use super::application::*;
pub(crate) use super::domain::normalizer::is_effectively_enabled_path;

#[cfg(debug_assertions)]
pub mod testing {
    pub mod adapters {
        pub use super::super::super::adapters::*;
    }
    pub mod domain {
        pub use super::super::super::domain::*;
    }
    pub mod application {
        pub use super::super::super::application::*;
    }
}

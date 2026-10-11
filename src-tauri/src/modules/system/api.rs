pub use super::application::file_lock::{
    bind_file_lock_scope, file_lock_retry_identities, rebind_file_lock_error,
    register_file_lock_with_identity, validate_file_lock_recovery,
    validate_file_lock_recovery_for_game, validate_file_lock_recovery_for_paths,
    validate_file_lock_replay_scope,
};
pub use super::application::game_detector::{focused_game_window, GameWindowGeometry};
pub use super::application::*;
pub use super::domain::file_lock::{FileLockDiscoveryStatus, FileLockProcess, FileLockReport};

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

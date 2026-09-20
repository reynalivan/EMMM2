pub use super::application::*;

// Workspace resolves an immutable explorer snapshot before delegating the
// mutation. Keep that cross-slice contract behind Library's facade so callers
// never reach into the Tauri adapter implementation directly.
pub(crate) use super::adapters::tauri::mod_bulk_cmds::{
    bulk_delete_mods_from_snapshot, bulk_pin_mods_from_snapshot, bulk_set_mod_safety_from_snapshot,
    bulk_toggle_favorite_from_snapshot, bulk_toggle_mods_from_snapshot,
    bulk_update_info_from_snapshot, BulkCancelState,
};
pub(crate) use super::adapters::tauri::mod_meta_cmds::{
    move_mods_to_object_from_snapshot, MoveModsToObjectInput,
};

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

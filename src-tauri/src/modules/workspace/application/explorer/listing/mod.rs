mod builder;
mod grid;
mod owners;
mod paged;
mod scan;

// Named rather than globbed so the module's public surface stays legible:
// everything else in `listing` is internal to the pass, or test-only.
pub use builder::{build_mod_folder_from_fs_entry, build_mod_folder_from_path};
pub use grid::{list_mod_folders_inner, list_mod_folders_inner_shallow};
pub use owners::{
    enrich_mod_folder_for_game, list_mod_folders_for_game, list_mod_folders_for_game_shallow,
    list_mod_folders_for_game_shallow_with_enrichment, list_mod_folders_for_game_with_enrichment,
    load_listing_enrichment,
};
pub use paged::{
    list_workspace_explorer_page, load_workspace_explorer_context,
    resolve_workspace_explorer_selection, validate_workspace_explorer_selection_identities,
};
pub use scan::{find_disabled_ancestor, scan_fs_folders, scan_fs_folders_shallow};

#[cfg(test)]
#[path = "../tests/listing_tests.rs"]
mod tests;

//! What `mods.folder_path` actually holds.

use std::path::{Path, PathBuf};

/// A mod folder path as the database stores it: **relative to the game's mods
/// root**, never a complete filesystem path.
///
/// Deliberately not `Deref<Target = Path>`, not `AsRef<Path>`, and not
/// `Into<PathBuf>`. Six readers treated the stored string as something they
/// could hand to `Path::new` and test, and every one of them failed the same
/// way: a relative path resolves against the process working directory, the
/// check says "not there", and the code takes its nothing-found branch. None
/// of them raised an error, because "no conflicts", "no duplicates" and "no
/// keybinds" are all ordinary answers. One concluded the folder was gone and
/// deleted the row.
///
/// The convention was known — one caller documented it correctly while four
/// neighbours got it wrong — but a comment cannot be checked. Reaching the
/// filesystem now costs a [`resolve`](Self::resolve) call that has to name the
/// root it resolves against, so the mistake is a missing argument rather than
/// a silent empty result.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModFolderPath(String);

impl ModFolderPath {
    /// Wrap a value read from `mods.folder_path`.
    ///
    /// Named for where it comes from: the only correct source is the database.
    /// A path assembled from a filesystem walk is already absolute and belongs
    /// in a `PathBuf`.
    pub fn from_stored(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The absolute path this row refers to.
    ///
    /// `join` also accepts an already-absolute value by replacing the base,
    /// which keeps rows written before the column was normalized working.
    pub fn resolve(&self, mods_root: &Path) -> PathBuf {
        mods_root.join(&self.0)
    }

    /// The stored form, for comparing against other stored values and for
    /// writing back. This is not a filesystem path — use [`resolve`] for that.
    ///
    /// [`resolve`]: Self::resolve
    pub fn as_stored(&self) -> &str {
        &self.0
    }

    /// The folder's own name, without its parent segments.
    pub fn folder_name(&self) -> &str {
        self.0
            .rsplit(['/', '\\'])
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or(&self.0)
    }

    pub fn into_stored(self) -> String {
        self.0
    }
}

impl std::fmt::Display for ModFolderPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolving_prefixes_the_mods_root() {
        let stored = ModFolderPath::from_stored("Amber/Blue Dress");
        assert_eq!(
            stored.resolve(Path::new("C:/Games/Mods")),
            PathBuf::from("C:/Games/Mods/Amber/Blue Dress")
        );
    }

    #[test]
    fn an_absolute_leftover_still_resolves_to_itself() {
        // Rows written before the column was normalized. `join` replaces the
        // base when given an absolute path, so they keep working.
        let stored = ModFolderPath::from_stored("C:/Elsewhere/Amber");
        assert_eq!(
            stored.resolve(Path::new("C:/Games/Mods")),
            PathBuf::from("C:/Elsewhere/Amber")
        );
    }

    #[test]
    fn the_folder_name_ignores_parent_segments_and_separator_style() {
        assert_eq!(
            ModFolderPath::from_stored("Amber/Blue Dress").folder_name(),
            "Blue Dress"
        );
        assert_eq!(
            ModFolderPath::from_stored("Amber\\Blue Dress").folder_name(),
            "Blue Dress"
        );
        assert_eq!(ModFolderPath::from_stored("Amber").folder_name(), "Amber");
    }
}

#[cfg(not(windows))]
pub use notify_debouncer_full::RecommendedCache as WatcherIdentityCache;

#[cfg(windows)]
pub use directory::WatcherIdentityCache;

#[cfg(windows)]
mod directory {
    use file_id::{get_file_id, FileId};
    use notify::RecursiveMode;
    use notify_debouncer_full::FileIdCache;
    use rayon::prelude::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    pub struct CacheDiagnostics {
        pub directory_id_reads: AtomicUsize,
        pub entries_visited: AtomicUsize,
        pub cached_directories: AtomicUsize,
        failure: Mutex<Option<io::ErrorKind>>,
        pending_failure: AtomicBool,
        validity: Arc<AtomicBool>,
    }

    impl Default for CacheDiagnostics {
        fn default() -> Self {
            Self {
                directory_id_reads: AtomicUsize::new(0),
                entries_visited: AtomicUsize::new(0),
                cached_directories: AtomicUsize::new(0),
                failure: Mutex::new(None),
                pending_failure: AtomicBool::new(false),
                validity: Arc::new(AtomicBool::new(true)),
            }
        }
    }

    impl CacheDiagnostics {
        fn failed(&self, operation: &str, path: &Path, error: &io::Error) {
            self.validity.store(false, Ordering::Release);
            let mut failure = crate::shared::sync::lock(&self.failure);
            if failure.is_none() {
                log::warn!(
                    "[Watcher] directory identity failure: operation={} path={} kind={:?} os_code={:?}",
                    operation,
                    path.display(),
                    error.kind(),
                    error.raw_os_error()
                );
                *failure = Some(error.kind());
            }
            self.pending_failure.store(true, Ordering::Release);
        }

        pub fn take_failure(&self) -> Option<io::ErrorKind> {
            if self.pending_failure.swap(false, Ordering::AcqRel) {
                *crate::shared::sync::lock(&self.failure)
            } else {
                None
            }
        }

        pub fn failure(&self) -> Option<io::ErrorKind> {
            *crate::shared::sync::lock(&self.failure)
        }
    }

    #[derive(Debug, Default)]
    pub struct WatcherIdentityCache {
        directories: BTreeMap<PathBuf, FileId>,
        explicit_roots: Vec<(PathBuf, RecursiveMode)>,
        diagnostics: Arc<CacheDiagnostics>,
    }

    impl WatcherIdentityCache {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn for_roots(roots: Vec<(PathBuf, RecursiveMode)>, validity: Arc<AtomicBool>) -> Self {
            Self {
                explicit_roots: roots,
                diagnostics: Arc::new(CacheDiagnostics {
                    validity,
                    ..CacheDiagnostics::default()
                }),
                ..Self::default()
            }
        }

        pub fn diagnostics(&self) -> Arc<CacheDiagnostics> {
            Arc::clone(&self.diagnostics)
        }

        fn cache_key(path: &Path) -> io::Result<PathBuf> {
            crate::shared::path_key::physical_namespace_path(path)
        }

        fn seed(
            &mut self,
            path: &Path,
            mode: RecursiveMode,
            read_children: &(impl Fn(&Path) -> io::Result<(Vec<PathBuf>, usize)> + Sync),
            metadata: &fs::Metadata,
            root_only: bool,
        ) -> io::Result<()> {
            if !metadata.is_dir() {
                return Ok(());
            }

            let mut directories = vec![path.to_path_buf()];
            let (children, mut entries_visited) = if root_only {
                (Vec::new(), 0)
            } else {
                read_children(path)?
            };
            if mode == RecursiveMode::Recursive {
                let subtrees: Vec<_> = children
                    .into_par_iter()
                    .map(|root| Self::directory_tree(root, read_children))
                    .collect();
                for subtree in subtrees {
                    let (subtree, visited) = subtree?;
                    directories.extend(subtree);
                    entries_visited += visited;
                }
            } else {
                directories.extend(children);
            }
            self.diagnostics
                .entries_visited
                .fetch_add(entries_visited, Ordering::Relaxed);
            self.diagnostics
                .directory_id_reads
                .fetch_add(directories.len(), Ordering::Relaxed);
            let identities: Vec<_> = directories
                .into_par_iter()
                .map(|directory| {
                    let identity = Self::cache_key(&directory)
                        .and_then(|key| get_file_id(&directory).map(|id| (key, id)));
                    identity.map_err(|error| (directory, error))
                })
                .collect();
            for identity in identities {
                match identity {
                    Ok((key, id)) => {
                        self.directories.insert(key, id);
                    }
                    Err((directory, error)) => {
                        self.diagnostics.failed("read_identity", &directory, &error)
                    }
                }
            }
            Ok(())
        }

        fn child_directories(directory: &Path) -> io::Result<(Vec<PathBuf>, usize)> {
            let mut children = Vec::new();
            let mut visited = 0;
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                visited += 1;
                if entry.file_type()?.is_dir()
                    && !crate::shared::path_key::has_hidden_path_component(Path::new(
                        &entry.file_name(),
                    ))
                {
                    children.push(entry.path());
                }
            }
            Ok((children, visited))
        }

        fn directory_tree(
            root: PathBuf,
            read_children: &(impl Fn(&Path) -> io::Result<(Vec<PathBuf>, usize)> + Sync),
        ) -> io::Result<(Vec<PathBuf>, usize)> {
            let mut directories = vec![root.clone()];
            let mut pending = vec![root];
            let mut visited = 0;
            while let Some(directory) = pending.pop() {
                let (children, count) = read_children(&directory)?;
                visited += count;
                directories.extend(children.iter().cloned());
                pending.extend(children);
            }
            Ok((directories, visited))
        }

        fn add_path_with_reader(
            &mut self,
            path: &Path,
            recursive_mode: RecursiveMode,
            read_children: &(impl Fn(&Path) -> io::Result<(Vec<PathBuf>, usize)> + Sync),
        ) {
            let root_only = match self.coverage_mode(path) {
                Ok(None) => return,
                Ok(Some(mode)) => mode == RecursiveMode::NonRecursive,
                Err(error) => {
                    self.diagnostics.failed("observe_path", path, &error);
                    return;
                }
            };
            let explicit_root = self.explicit_roots.iter().any(|(root, _)| root == path);
            let metadata = if explicit_root {
                fs::metadata(path)
            } else {
                fs::symlink_metadata(path)
            };
            let metadata = match metadata {
                Ok(metadata) => metadata,
                // A queued Create/To can outlive a moved subtree. Keep its
                // historical IDs until notify explicitly removes the old path.
                Err(error) if !explicit_root && error.kind() == io::ErrorKind::NotFound => return,
                Err(error) => {
                    self.diagnostics.failed("metadata", path, &error);
                    return;
                }
            };
            // A runtime-parent refresh must not erase a separately watched Mods subtree.
            if !root_only {
                self.remove_path(path);
            }
            if let Err(error) = self.seed(path, recursive_mode, read_children, &metadata, root_only)
            {
                self.diagnostics.failed("seed_subtree", path, &error);
            }
            self.diagnostics
                .cached_directories
                .store(self.directories.len(), Ordering::Relaxed);
        }

        fn coverage_mode(&self, path: &Path) -> io::Result<Option<RecursiveMode>> {
            if self.explicit_roots.is_empty() {
                return Ok(Some(RecursiveMode::Recursive));
            }
            let path = Self::cache_key(path)?;
            let mut coverage = None;
            for (root, mode) in &self.explicit_roots {
                let root = Self::cache_key(root)?;
                if let Ok(relative) = path.strip_prefix(root) {
                    if *mode == RecursiveMode::Recursive
                        && !crate::shared::path_key::has_hidden_path_component(relative)
                    {
                        return Ok(Some(RecursiveMode::Recursive));
                    }
                    if relative.as_os_str().is_empty() {
                        coverage = Some(*mode);
                    }
                }
            }
            Ok(coverage)
        }
    }

    impl FileIdCache for WatcherIdentityCache {
        fn cached_file_id(&self, path: &Path) -> Option<impl AsRef<FileId>> {
            match Self::cache_key(path) {
                Ok(key) => self.directories.get(&key).copied(),
                Err(error) => {
                    self.diagnostics.failed("cache_lookup", path, &error);
                    None
                }
            }
        }

        fn add_path(&mut self, path: &Path, recursive_mode: RecursiveMode) {
            self.add_path_with_reader(path, recursive_mode, &Self::child_directories);
        }

        fn remove_path(&mut self, path: &Path) {
            match Self::cache_key(path) {
                Ok(key) => {
                    let descendants: Vec<_> = self
                        .directories
                        .range(key.clone()..)
                        .take_while(|(directory, _)| directory.starts_with(&key))
                        .map(|(directory, _)| directory.clone())
                        .collect();
                    for directory in descendants {
                        self.directories.remove(&directory);
                    }
                }
                Err(error) => self.diagnostics.failed("cache_remove", path, &error),
            }
            self.diagnostics
                .cached_directories
                .store(self.directories.len(), Ordering::Relaxed);
        }

        fn rescan(&mut self, roots: &[(PathBuf, RecursiveMode)]) {
            self.diagnostics.validity.store(false, Ordering::Release);
            *crate::shared::sync::lock(&self.diagnostics.failure) = None;
            self.diagnostics
                .pending_failure
                .store(false, Ordering::Release);
            self.directories.clear();
            for (root, mode) in roots {
                self.add_path(root, *mode);
            }
            if self.diagnostics.failure().is_none() {
                self.diagnostics.validity.store(true, Ordering::Release);
            }
        }
    }

    #[cfg(test)]
    mod traversal_tests {
        use super::*;

        #[test]
        fn runtime_parent_refresh_preserves_nested_mods_without_enumerating_siblings() {
            let fixture = tempfile::tempdir().unwrap();
            let mods = fixture.path().join("Mods");
            let module = mods.join("Object/Mod");
            fs::create_dir_all(&module).unwrap();
            let unrelated = fixture.path().join("Tools");
            fs::create_dir(&unrelated).unwrap();
            let roots = vec![
                (mods.clone(), RecursiveMode::Recursive),
                (fixture.path().to_path_buf(), RecursiveMode::NonRecursive),
            ];
            let validity = Arc::new(AtomicBool::new(true));
            let mut cache = WatcherIdentityCache::for_roots(roots.clone(), Arc::clone(&validity));
            cache.add_path(&mods, RecursiveMode::Recursive);
            let identity = *cache.cached_file_id(&module).unwrap().as_ref();
            cache.add_path_with_reader(fixture.path(), RecursiveMode::NonRecursive, &|_| {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            });
            assert!(validity.load(Ordering::Acquire));
            assert_eq!(
                cache.cached_file_id(&module).map(|id| *id.as_ref()),
                Some(identity)
            );
            assert!(cache.cached_file_id(fixture.path()).is_some());
            assert!(cache.cached_file_id(&unrelated).is_none());
            cache.rescan(&roots);
            assert!(validity.load(Ordering::Acquire));
            assert_eq!(
                cache.cached_file_id(&module).map(|id| *id.as_ref()),
                Some(identity)
            );
            assert!(cache.cached_file_id(&unrelated).is_none());
        }

        #[test]
        fn runtime_sibling_failure_is_excluded_but_mods_failure_invalidates() {
            let fixture = tempfile::tempdir().unwrap();
            let mods = fixture.path().join("Mods");
            let module = mods.join("Object/Mod");
            let unrelated = fixture.path().join("Unrelated");
            fs::create_dir_all(&module).unwrap();
            fs::create_dir(&unrelated).unwrap();
            let validity = Arc::new(AtomicBool::new(true));
            let mut cache = WatcherIdentityCache::for_roots(
                vec![
                    (mods, RecursiveMode::Recursive),
                    (fixture.path().to_path_buf(), RecursiveMode::NonRecursive),
                ],
                Arc::clone(&validity),
            );
            let denied = |_: &Path| Err(io::Error::from(io::ErrorKind::PermissionDenied));
            cache.add_path_with_reader(&unrelated, RecursiveMode::Recursive, &denied);
            assert!(validity.load(Ordering::Acquire));
            assert_eq!(cache.diagnostics.failure(), None);
            cache.add_path_with_reader(&module, RecursiveMode::Recursive, &denied);
            assert!(!validity.load(Ordering::Acquire));
            assert_eq!(
                cache.diagnostics.failure(),
                Some(io::ErrorKind::PermissionDenied)
            );
        }

        #[test]
        fn disappearing_child_taints_present_uncached_parent() {
            let fixture = tempfile::tempdir().expect("fixture");
            let parent = fixture.path().join("New Mod");
            let child = parent.join("Transient");
            fs::create_dir_all(&child).expect("new subtree");
            let mut cache = WatcherIdentityCache::new();
            cache.add_path_with_reader(&parent, RecursiveMode::Recursive, &|directory| {
                if directory == child {
                    fs::remove_dir(&child).expect("child disappeared during enumeration");
                }
                WatcherIdentityCache::child_directories(directory)
            });
            assert!(parent.is_dir());
            assert!(
                !cache.diagnostics.validity.load(Ordering::Acquire),
                "failed subtree enumeration must invalidate coverage"
            );
            assert_eq!(
                cache.diagnostics.take_failure(),
                Some(io::ErrorKind::NotFound)
            );
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::WatcherIdentityCache;
    use notify::RecursiveMode;
    use notify_debouncer_full::FileIdCache;
    use std::fs;

    #[test]
    fn locked_regular_file_does_not_invalidate_directory_identity_coverage() {
        use std::os::windows::fs::OpenOptionsExt;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let root = tempfile::tempdir().unwrap();
        let asset = root.path().join("status.json");
        fs::write(&asset, b"{}").unwrap();
        let validity = Arc::new(AtomicBool::new(true));
        let mut cache = WatcherIdentityCache::for_roots(
            vec![(root.path().to_path_buf(), RecursiveMode::Recursive)],
            Arc::clone(&validity),
        );
        cache.add_path(root.path(), RecursiveMode::Recursive);
        let _writer = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&asset)
            .unwrap();
        cache.add_path(&asset, RecursiveMode::NonRecursive);
        assert!(
            validity.load(Ordering::Acquire),
            "Locked regular file poisoned directory cache: {:?}",
            cache.diagnostics().failure()
        );
        assert!(cache.cached_file_id(root.path()).is_some());
        assert!(cache.cached_file_id(&asset).is_none());
    }

    #[test]
    fn directory_seed_does_not_read_asset_file_ids() {
        let root = tempfile::tempdir().expect("watch root");
        let dotted = root.path().join("Object.v2/Mod.ini/Assets.bin");
        fs::create_dir_all(&dotted).expect("dotted directories");
        let asset = dotted.join("mesh.buf");
        let ini = dotted.join("mod.ini");
        fs::write(&asset, b"asset").expect("asset file");
        fs::write(&ini, b"[TextureOverride]").expect("ini file");

        let mut cache = WatcherIdentityCache::new();
        cache.add_path(root.path(), RecursiveMode::Recursive);
        assert!(cache.cached_file_id(root.path()).is_some());
        assert!(cache.cached_file_id(&dotted).is_some());
        assert!(
            cache.cached_file_id(&asset).is_none(),
            "asset file IDs must not be prewarmed"
        );
        assert!(
            cache.cached_file_id(&ini).is_none(),
            "file changes remain observations without file-ID stitching"
        );
    }

    #[test]
    fn runtime_artifact_subtrees_are_not_mod_identity_coverage() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let root = tempfile::tempdir().expect("watch root");
        let mod_path = root.path().join("Object/Mod");
        let runtime = root.path().join(".emmm_data");
        let staging = runtime.join("generations/publish.staging");
        fs::create_dir_all(&mod_path).expect("mod directory");
        fs::create_dir_all(&staging).expect("runtime staging");
        let validity = Arc::new(AtomicBool::new(true));
        let mut cache = WatcherIdentityCache::for_roots(
            vec![(root.path().to_path_buf(), RecursiveMode::Recursive)],
            Arc::clone(&validity),
        );
        cache.add_path(root.path(), RecursiveMode::Recursive);
        assert!(cache.cached_file_id(&mod_path).is_some());
        assert!(cache.cached_file_id(&runtime).is_none());
        assert!(cache.cached_file_id(&staging).is_none());
        let reads = cache
            .diagnostics()
            .directory_id_reads
            .load(Ordering::Relaxed);
        cache.add_path(&staging, RecursiveMode::Recursive);
        fs::rename(&staging, runtime.join("generations/published"))
            .expect("publish runtime generation");
        cache.add_path(&staging, RecursiveMode::Recursive);
        assert_eq!(
            cache
                .diagnostics()
                .directory_id_reads
                .load(Ordering::Relaxed),
            reads
        );
        assert!(validity.load(Ordering::Acquire));
        assert_eq!(cache.diagnostics().failure(), None);
    }

    #[test]
    fn explicit_hidden_named_roots_still_track_their_visible_mods() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let fixture = tempfile::tempdir().expect("runtime parent");
        let mods = fixture.path().join(".configured-mods");
        let module = mods.join("Object/Mod");
        fs::create_dir_all(&module).expect("visible mod below configured root");
        let validity = Arc::new(AtomicBool::new(true));
        let mut cache = WatcherIdentityCache::for_roots(
            vec![
                (fixture.path().to_path_buf(), RecursiveMode::NonRecursive),
                (mods.clone(), RecursiveMode::Recursive),
            ],
            Arc::clone(&validity),
        );
        cache.add_path(fixture.path(), RecursiveMode::NonRecursive);
        cache.add_path(&mods, RecursiveMode::Recursive);
        cache.add_path(&module, RecursiveMode::Recursive);
        assert!(cache.cached_file_id(&mods).is_some());
        assert!(cache.cached_file_id(&module).is_some());
        assert!(validity.load(Ordering::Acquire));
    }

    #[test]
    fn rename_source_identity_survives_until_explicit_removal() {
        let root = tempfile::tempdir().expect("watch root");
        let old = root.path().join("Mod");
        let new = root.path().join("DISABLED Mod");
        fs::create_dir_all(old.join("Variant")).expect("mod subtree");
        let mut cache = WatcherIdentityCache::new();
        cache.add_path(root.path(), RecursiveMode::Recursive);
        let before = cache
            .cached_file_id(&old)
            .map(|id| *id.as_ref())
            .expect("source ID");

        fs::rename(&old, &new).expect("physical prefix rename");
        assert_eq!(
            cache.cached_file_id(&old).map(|id| *id.as_ref()),
            Some(before)
        );
        cache.remove_path(&old);
        cache.add_path(&new, RecursiveMode::Recursive);
        assert!(cache.cached_file_id(&old).is_none());
        assert!(cache.cached_file_id(&old.join("Variant")).is_none());
        assert_eq!(
            cache.cached_file_id(&new).map(|id| *id.as_ref()),
            Some(before)
        );
        assert!(cache.cached_file_id(&new.join("Variant")).is_some());
    }

    #[test]
    fn late_refresh_of_a_moved_cached_directory_preserves_rename_identity_and_coverage() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let root = tempfile::tempdir().expect("watch root");
        let old = root.path().join("Mod");
        let new = root.path().join("DISABLED Mod");
        fs::create_dir_all(old.join("Variant")).expect("mod subtree");
        let validity = Arc::new(AtomicBool::new(true));
        let mut cache = WatcherIdentityCache::for_roots(
            vec![(root.path().to_path_buf(), RecursiveMode::Recursive)],
            Arc::clone(&validity),
        );
        cache.add_path(root.path(), RecursiveMode::Recursive);
        let before = cache
            .cached_file_id(&old)
            .map(|id| *id.as_ref())
            .expect("source ID");
        let child_before = cache
            .cached_file_id(&old.join("Variant"))
            .map(|id| *id.as_ref())
            .expect("child ID");

        fs::rename(&old, &new).expect("physical prefix rename");
        cache.add_path(&old, RecursiveMode::Recursive);
        assert!(validity.load(Ordering::Acquire));
        assert_eq!(cache.diagnostics().failure(), None);
        assert_eq!(
            cache.cached_file_id(&old).map(|id| *id.as_ref()),
            Some(before)
        );
        assert_eq!(
            cache
                .cached_file_id(&old.join("Variant"))
                .map(|id| *id.as_ref()),
            Some(child_before)
        );
        cache.remove_path(&old);
        cache.add_path(&new, RecursiveMode::Recursive);
        assert!(cache.cached_file_id(&old).is_none());
        assert!(cache.cached_file_id(&old.join("Variant")).is_none());
        assert_eq!(
            cache.cached_file_id(&new).map(|id| *id.as_ref()),
            Some(before)
        );
        assert!(validity.load(Ordering::Acquire));
    }

    #[test]
    fn nonrecursive_runtime_parent_does_not_seed_nested_trees() {
        let root = tempfile::tempdir().expect("runtime parent");
        let child = root.path().join("Tools");
        let deep = child.join("Nested");
        fs::create_dir_all(&deep).expect("nested tools");
        fs::write(root.path().join("d3dx.ini"), b"[Constants]").expect("runtime config");
        let mut cache = WatcherIdentityCache::new();
        cache.add_path(root.path(), RecursiveMode::NonRecursive);
        assert!(cache.cached_file_id(root.path()).is_some());
        assert!(cache.cached_file_id(&child).is_some());
        assert!(cache.cached_file_id(&deep).is_none());
    }

    #[test]
    fn reseeding_replacement_subtree_discards_vanished_descendant_ids() {
        let root = tempfile::tempdir().expect("watch root");
        let original = root.path().join("Object");
        let old_child = original.join("Old Mod");
        fs::create_dir_all(&old_child).expect("original subtree");
        let mut cache = WatcherIdentityCache::new();
        cache.add_path(root.path(), RecursiveMode::Recursive);
        fs::rename(&original, root.path().join("Elsewhere")).expect("old subtree moved");
        let new_child = original.join("New Mod");
        fs::create_dir_all(&new_child).expect("replacement subtree");

        cache.add_path(&original, RecursiveMode::Recursive);
        assert!(
            cache.cached_file_id(&old_child).is_none(),
            "vanished descendants must not retain old ownership"
        );
        assert!(cache.cached_file_id(&new_child).is_some());
    }

    #[test]
    fn failed_seed_remains_visible_after_its_error_is_reported() {
        let root = tempfile::tempdir().expect("watch root");
        for mode in [RecursiveMode::Recursive, RecursiveMode::NonRecursive] {
            let mut cache = WatcherIdentityCache::for_roots(
                vec![(root.path().join("Missing"), mode)],
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
            );
            let diagnostics = cache.diagnostics();
            cache.add_path(&root.path().join("Missing"), mode);
            assert_eq!(
                diagnostics.take_failure(),
                Some(std::io::ErrorKind::NotFound)
            );
            assert_eq!(diagnostics.take_failure(), None);
            assert_eq!(diagnostics.failure(), Some(std::io::ErrorKind::NotFound));
        }
    }

    #[test]
    fn asset_burst_preserves_unrelated_directory_identities() {
        let root = tempfile::tempdir().expect("watch root");
        for name in ["Mod", "Mod.v2", "Mod2", "Mod/Sub"] {
            fs::create_dir_all(root.path().join(name)).expect("directory fixture");
        }
        let asset = root.path().join("Mod/mesh.buf");
        fs::write(&asset, b"asset").expect("asset fixture");
        let mut cache = WatcherIdentityCache::new();
        cache.add_path(root.path(), RecursiveMode::Recursive);
        for _ in 0..1000 {
            cache.add_path(&asset, RecursiveMode::Recursive);
        }
        cache.remove_path(&root.path().join("Mod"));
        assert!(cache.cached_file_id(&root.path().join("Mod/Sub")).is_none());
        assert!(cache.cached_file_id(&root.path().join("Mod.v2")).is_some());
        assert!(cache.cached_file_id(&root.path().join("Mod2")).is_some());
    }

    #[test]
    fn only_complete_rescan_restores_failed_directory_coverage() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let root = tempfile::tempdir().expect("watch root");
        let missing = root.path().join("Missing");
        let validity = Arc::new(AtomicBool::new(true));
        let mut cache = WatcherIdentityCache::for_roots(
            vec![(missing.clone(), RecursiveMode::Recursive)],
            Arc::clone(&validity),
        );
        cache.add_path(&missing, RecursiveMode::Recursive);
        assert!(!validity.load(Ordering::Acquire));
        fs::create_dir(&missing).expect("restore directory");
        cache.add_path(&missing, RecursiveMode::Recursive);
        assert!(
            !validity.load(Ordering::Acquire),
            "partial repair cannot certify complete coverage"
        );
        cache.rescan(&[(missing, RecursiveMode::Recursive)]);
        assert!(validity.load(Ordering::Acquire));
        assert_eq!(cache.diagnostics().failure(), None);
    }
}

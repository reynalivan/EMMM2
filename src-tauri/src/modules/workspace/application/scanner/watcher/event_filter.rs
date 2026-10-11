use notify::event::ModifyKind;
use notify::{Event, EventKind};
use std::path::{Path, PathBuf};

const RELEVANT_EXTENSIONS: &[&str] = &["ini", "json", "png", "jpg", "jpeg", "webp"];

fn runtime_path_matches(path: &Path, runtime: &Path) -> bool {
    path.to_string_lossy()
        .eq_ignore_ascii_case(&runtime.to_string_lossy())
}

#[cfg(test)]
pub(super) fn is_runtime_config_path(path: &Path, runtime: Option<&Path>) -> bool {
    let normalized = crate::shared::path_key::physical_namespace_path(path).ok();
    let runtime =
        runtime.and_then(|path| crate::shared::path_key::physical_namespace_path(path).ok());
    normalized
        .as_deref()
        .zip(runtime.as_deref())
        .is_some_and(|(path, runtime)| runtime_path_matches(path, runtime))
}

fn is_relevant_path(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("info.json"))
    {
        return true;
    }

    match path.extension().and_then(|e| e.to_str()) {
        None => true,
        Some(ext) => {
            let lower = ext.to_ascii_lowercase();
            RELEVANT_EXTENSIONS.contains(&lower.as_str())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DirectoryFact {
    Directory,
    NotDirectory,
    Missing,
    Unavailable,
}

impl DirectoryFact {
    pub(super) fn capture(path: &Path) -> Self {
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => Self::Directory,
            Ok(_) => Self::NotDirectory,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::Missing,
            Err(_) => Self::Unavailable,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct EndpointPolicy {
    pub(super) structural: bool,
    content: bool,
    pub(super) needs_directory_fact: bool,
    ancestor: bool,
}

impl EndpointPolicy {
    pub(super) fn keeps_content(self, fact: Option<DirectoryFact>) -> bool {
        self.content || (self.needs_directory_fact && fact == Some(DirectoryFact::Directory))
    }
}

/// Lexical coverage of historical event paths; never canonicalize or stat them.
pub(super) struct WatcherCoverage {
    root: Option<PathBuf>,
    runtime: Option<PathBuf>,
}

impl WatcherCoverage {
    pub(super) fn new(root: &Path, runtime: Option<&Path>) -> Self {
        Self {
            root: crate::shared::path_key::physical_namespace_path(root).ok(),
            runtime: runtime
                .and_then(|path| crate::shared::path_key::physical_namespace_path(path).ok()),
        }
    }

    fn endpoint(&self, path: &Path) -> EndpointPolicy {
        let normalized = crate::shared::path_key::physical_namespace_path(path).ok();
        let relative = normalized.as_deref().and_then(|path| {
            path.strip_prefix(self.root.as_deref()?)
                .ok()
                .filter(|relative| !crate::shared::path_key::has_hidden_path_component(relative))
        });
        let runtime = normalized
            .as_ref()
            .zip(self.runtime.as_ref())
            .is_some_and(|(path, runtime)| runtime_path_matches(path, runtime));
        let structural = relative.is_some() || runtime;
        let content = runtime
            || relative.is_some_and(|relative| {
                relative.components().nth(2).is_none() || is_relevant_path(path)
            });
        let ancestor = normalized.as_ref().is_none_or(|path| {
            self.root.as_ref().is_none_or(|root| root.starts_with(path))
                || self
                    .runtime
                    .as_deref()
                    .and_then(Path::parent)
                    .is_some_and(|parent| parent.starts_with(path))
        });
        EndpointPolicy {
            structural,
            content,
            needs_directory_fact: structural && !content,
            ancestor,
        }
    }

    pub(super) fn policy(&self, event: &Event) -> EventPolicy {
        let endpoints: Vec<_> = event.paths.iter().map(|path| self.endpoint(path)).collect();
        let structural = matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
        );
        let authority_relevant = event.need_rescan()
            || (!matches!(event.kind, EventKind::Access(_))
                && endpoints
                    .iter()
                    .any(|endpoint| endpoint.structural || (structural && endpoint.ancestor)));
        EventPolicy {
            endpoints,
            authority_relevant,
        }
    }
}

/// One ordered endpoint view is shared by authority observation and delivery.
pub(super) struct EventPolicy {
    pub(super) endpoints: Vec<EndpointPolicy>,
    pub(super) authority_relevant: bool,
}

#[cfg(test)]
pub(crate) fn should_keep_event_path(path: &Path, watcher_path: &Path) -> bool {
    let policy = WatcherCoverage::new(watcher_path, None).endpoint(path);
    policy.keeps_content(
        policy
            .needs_directory_fact
            .then(|| DirectoryFact::capture(path)),
    )
}

#[cfg(test)]
pub(crate) fn should_keep_structural_event_path(path: &Path, watcher_path: &Path) -> bool {
    WatcherCoverage::new(watcher_path, None)
        .endpoint(path)
        .structural
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_policy_is_independent_of_directory_fact() {
        let root = Path::new(r"E:\Mods");
        let path = root.join("Alice/Nested/Blue.v2");
        let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path);
        let policy = WatcherCoverage::new(root, None).policy(&event);
        assert!(policy.authority_relevant);
        let endpoint = policy.endpoints[0];
        assert!(endpoint.structural);
        assert!(endpoint.needs_directory_fact);
        assert!(endpoint.keeps_content(Some(DirectoryFact::Directory)));
        for fact in [
            DirectoryFact::NotDirectory,
            DirectoryFact::Missing,
            DirectoryFact::Unavailable,
        ] {
            assert!(!endpoint.keeps_content(Some(fact)));
        }
    }

    #[test]
    fn root_loss_and_rescan_are_authority_relevant_but_access_is_not() {
        let root = Path::new(r"E:\Importer\Mods");
        let coverage = WatcherCoverage::new(root, Some(Path::new(r"E:\Importer\d3dx.ini")));
        let parent = Event::new(EventKind::Remove(notify::event::RemoveKind::Folder))
            .add_path(PathBuf::from(r"E:\Importer"));
        let policy = coverage.policy(&parent);
        assert!(policy.authority_relevant);
        assert!(!policy.endpoints[0].structural);
        assert!(
            coverage
                .policy(&Event::new(EventKind::Other).set_flag(notify::event::Flag::Rescan))
                .authority_relevant
        );
        assert!(
            !coverage
                .policy(
                    &Event::new(EventKind::Access(notify::event::AccessKind::Any))
                        .add_path(root.to_path_buf())
                )
                .authority_relevant
        );
    }
}

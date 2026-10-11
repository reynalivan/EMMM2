use crate::modules::system::domain::file_lock::{
    FileLockDiscoveryStatus, FileLockProcess, FileLockReport,
};
use crate::platform::fs::{
    file_utils::FilesystemIdentityProof,
    locking::{self, LockDiscovery, LockProcess},
};
use crate::shared::errors::AppError;
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

const CONTEXT_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_BULK_TARGETS: usize = 10_000;
const MAX_CONTEXTS: usize = MAX_BULK_TARGETS + 64;

#[derive(Clone)]
pub struct FileLockGameSnapshot {
    pub game_id: String,
    pub root: PathBuf,
}

type SnapshotProvider = dyn Fn() -> Option<FileLockGameSnapshot> + Send + Sync;
static SNAPSHOT_PROVIDER: OnceLock<Box<SnapshotProvider>> = OnceLock::new();
static CONTEXTS: OnceLock<Mutex<ContextStore>> = OnceLock::new();

type ContextKey = (String, PathBuf, String, PathBuf, String);

struct ContextEntry {
    key: ContextKey,
    context: Arc<Mutex<RecoveryContext>>,
}

#[derive(Default)]
struct ContextStore {
    entries: HashMap<String, ContextEntry>,
    by_target: HashMap<ContextKey, String>,
}

impl ContextStore {
    fn remove_revoked(&mut self, id: &str) {
        if let Some(entry) = self.entries.remove(id) {
            self.by_target.remove(&entry.key);
        }
    }

    fn insert(&mut self, context: RecoveryContext) -> Option<String> {
        let key = (
            context.game.game_id.clone(),
            context.game.root.clone(),
            context.root_proof.identity().to_string(),
            context.path.clone(),
            context.path_proof.identity().to_string(),
        );
        if let Some(id) = self.by_target.get(&key).cloned() {
            let entry = self.entries.get(&id)?;
            let mut existing = entry.context.try_lock().ok()?;
            if existing.scope == context.scope {
                existing.issued = Instant::now();
                existing.process_refs.clear();
                return Some(id);
            }
            existing.revoked = true;
            drop(existing);
            self.remove_revoked(&id);
        }
        if self.entries.len() >= MAX_CONTEXTS {
            let expired: Vec<String> = self
                .entries
                .iter()
                .filter_map(|(id, entry)| {
                    entry
                        .context
                        .try_lock()
                        .ok()
                        .filter(|context| context.issued.elapsed() >= CONTEXT_TTL)
                        .map(|_| id.clone())
                })
                .collect();
            for id in expired {
                let revoke = self.entries.get(&id).is_some_and(|entry| {
                    entry.context.try_lock().is_ok_and(|mut context| {
                        if context.issued.elapsed() < CONTEXT_TTL {
                            return false;
                        }
                        context.revoked = true;
                        true
                    })
                });
                if revoke {
                    self.remove_revoked(&id);
                }
            }
        }
        if self.entries.len() >= MAX_CONTEXTS {
            return None;
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.by_target.insert(key.clone(), id.clone());
        self.entries.insert(
            id.clone(),
            ContextEntry {
                key,
                context: Arc::new(Mutex::new(context)),
            },
        );
        Some(id)
    }
}

#[derive(Clone)]
struct RecoveryContext {
    revoked: bool,
    issued: Instant,
    game: FileLockGameSnapshot,
    root_proof: FilesystemIdentityProof,
    path: PathBuf,
    path_proof: FilesystemIdentityProof,
    process_refs: BTreeMap<String, LockProcess>,
    scope: Vec<(PathBuf, String)>,
}

pub fn initialize_file_lock_contexts(
    provider: impl Fn() -> Option<FileLockGameSnapshot> + Send + Sync + 'static,
) {
    let _ = SNAPSHOT_PROVIDER.set(Box::new(provider));
}

fn contexts() -> &'static Mutex<ContextStore> {
    CONTEXTS.get_or_init(|| Mutex::new(ContextStore::default()))
}

fn state_error() -> AppError {
    AppError::Validation("File-lock recovery expired or its game/root/target changed. Refresh the selection before retrying.".into())
}

fn current_game() -> Option<FileLockGameSnapshot> {
    SNAPSHOT_PROVIDER.get()?.as_ref()()
}

fn create_context(path: &Path, game: FileLockGameSnapshot) -> Option<RecoveryContext> {
    let root = game.root.canonicalize().ok()?;
    let path = path.canonicalize().ok()?;
    if path == root || !path.starts_with(&root) {
        return None;
    }
    let root_proof = FilesystemIdentityProof::capture(&root).ok()?;
    let path_proof = FilesystemIdentityProof::capture(&path).ok()?;
    Some(RecoveryContext {
        revoked: false,
        issued: Instant::now(),
        game: FileLockGameSnapshot { root, ..game },
        root_proof,
        path,
        path_proof,
        process_refs: BTreeMap::new(),
        scope: Vec::new(),
    })
}

/// Only issued after a blocked native step, using its frozen pre-attempt identity.
pub fn register_file_lock_with_identity(path: &Path, expected_identity: &str) -> Option<String> {
    let context = create_context(path, current_game()?)?;
    if context.path_proof.identity() != expected_identity {
        return None;
    }
    store_context(context)
}

fn store_context(context: RecoveryContext) -> Option<String> {
    contexts().lock().ok()?.insert(context)
}

fn lookup(id: &str) -> Result<Arc<Mutex<RecoveryContext>>, AppError> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(state_error());
    }
    contexts()
        .lock()
        .map_err(|_| AppError::Internal("File-lock context store is unavailable".into()))?
        .entries
        .get(id)
        .map(|entry| entry.context.clone())
        .ok_or_else(state_error)
}

fn validate(context: &RecoveryContext, game: Option<FileLockGameSnapshot>) -> Result<(), AppError> {
    if context.issued.elapsed() >= CONTEXT_TTL {
        return Err(state_error());
    }
    validate_original(context, game)
}

fn validate_root(
    context: &RecoveryContext,
    game: Option<FileLockGameSnapshot>,
) -> Result<(), AppError> {
    let game = game.ok_or_else(state_error)?;
    if context.revoked
        || game.game_id != context.game.game_id
        || game.root.canonicalize().ok().as_deref() != Some(context.game.root.as_path())
    {
        return Err(state_error());
    }
    context
        .root_proof
        .validate(&context.game.root)
        .map_err(|_| state_error())
}

fn validate_scope(context: &RecoveryContext) -> Result<(), AppError> {
    for (path, identity) in &context.scope {
        if path == &context.game.root
            || !path.starts_with(&context.game.root)
            || path.canonicalize().ok().as_deref() != Some(path.as_path())
            || crate::platform::fs::file_utils::filesystem_identity(path).as_deref()
                != Some(identity)
        {
            return Err(state_error());
        }
    }
    Ok(())
}

fn validate_original(
    context: &RecoveryContext,
    game: Option<FileLockGameSnapshot>,
) -> Result<(), AppError> {
    validate_root(context, game)?;
    if context.path.canonicalize().ok().as_deref() != Some(context.path.as_path()) {
        return Err(state_error());
    }
    validate_scope(context)?;
    context
        .path_proof
        .validate(&context.path)
        .map_err(|_| state_error())
}

fn renew_context(
    context: &mut RecoveryContext,
    game: Option<FileLockGameSnapshot>,
) -> Result<(), AppError> {
    validate_original(context, game)?;
    if context.issued.elapsed() >= CONTEXT_TTL {
        context.process_refs.clear();
    }
    context.issued = Instant::now();
    Ok(())
}

pub fn validate_file_lock_recovery(id: &str, target: &Path) -> Result<(), AppError> {
    let context = lookup(id)?;
    let context = context.lock().map_err(|_| state_error())?;
    validate(&context, current_game())?;
    if target.canonicalize().ok().as_deref() != Some(context.path.as_path()) {
        return Err(state_error());
    }
    Ok(())
}

pub fn validate_file_lock_recovery_for_paths(id: &str, paths: &[PathBuf]) -> Result<(), AppError> {
    let context = lookup(id)?;
    let context = context.lock().map_err(|_| state_error())?;
    validate(&context, current_game())?;
    if !paths
        .iter()
        .any(|path| path.canonicalize().ok().as_deref() == Some(context.path.as_path()))
    {
        return Err(state_error());
    }
    Ok(())
}

pub fn validate_file_lock_recovery_for_game(id: &str, game_id: &str) -> Result<(), AppError> {
    let context = lookup(id)?;
    let context = context.lock().map_err(|_| state_error())?;
    if context.game.game_id != game_id {
        return Err(state_error());
    }
    validate(&context, current_game())
}

pub fn validate_file_lock_replay_scope(
    id: &str,
    proposed_scope: &[(PathBuf, String)],
) -> Result<(), AppError> {
    let context = lookup(id)?;
    let context = context.lock().map_err(|_| state_error())?;
    validate(&context, current_game())?;
    validate_replay_scope(&context, proposed_scope)
}

fn validate_replay_scope(
    context: &RecoveryContext,
    proposed_scope: &[(PathBuf, String)],
) -> Result<(), AppError> {
    if proposed_scope.len() > MAX_BULK_TARGETS {
        return Err(state_error());
    }
    let mut known = HashMap::with_capacity(context.scope.len().max(1));
    if context.scope.is_empty() {
        known.insert(
            crate::shared::path_key::exact_location_key_for_path(&context.path),
            context.path_proof.identity(),
        );
    } else {
        for (path, identity) in &context.scope {
            let key = crate::shared::path_key::exact_location_key_for_path(path);
            if known
                .insert(key, identity.as_str())
                .is_some_and(|previous| previous != identity)
            {
                return Err(state_error());
            }
        }
    }
    for (path, identity) in proposed_scope {
        let canonical = path.canonicalize().map_err(|_| state_error())?;
        let key = crate::shared::path_key::exact_location_key_for_path(&canonical);
        if known.get(&key).copied() != Some(identity.as_str())
            || crate::platform::fs::file_utils::filesystem_identity(&canonical).as_deref()
                != Some(identity.as_str())
        {
            return Err(state_error());
        }
    }
    Ok(())
}

pub fn file_lock_retry_identities(
    ids: &[String],
    paths: &[String],
) -> Result<Vec<(String, String)>, AppError> {
    if ids.is_empty() || paths.is_empty() || paths.len() > MAX_BULK_TARGETS {
        return Err(state_error());
    }
    let game = current_game();
    let mut by_path = HashMap::with_capacity(ids.len());
    let mut seen_tokens = std::collections::HashSet::new();
    for id in ids {
        if !seen_tokens.insert(id) {
            return Err(state_error());
        }
        let context = lookup(id)?;
        let context = context.lock().map_err(|_| state_error())?;
        validate(&context, game.clone())?;
        let scope = if context.scope.is_empty() {
            vec![(
                context.path.clone(),
                context.path_proof.identity().to_string(),
            )]
        } else {
            context.scope.clone()
        };
        for (path, identity) in scope {
            if by_path.insert(path, (identity, id)).is_some() {
                return Err(state_error());
            }
        }
    }
    let mut matched_tokens = std::collections::HashSet::new();
    let result: Result<Vec<(String, String)>, AppError> = paths
        .iter()
        .map(|path| {
            let canonical = Path::new(path).canonicalize().map_err(|_| state_error())?;
            let (identity, token) = by_path.remove(&canonical).ok_or_else(state_error)?;
            matched_tokens.insert(token);
            Ok((path.clone(), identity))
        })
        .collect();
    let result = result?;
    if matched_tokens.len() != ids.len() {
        return Err(state_error());
    }
    Ok(result)
}

pub fn dismiss_file_lock_recovery(id: &str) -> Result<(), AppError> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(state_error());
    }
    let mut entries = contexts().lock().map_err(|_| state_error())?;
    // Removal cannot invalidate an in-progress process action halfway through.
    if let Some(entry) = entries.entries.get(id) {
        let mut context = entry
            .context
            .try_lock()
            .map_err(|_| AppError::Validation("A recovery action is still running".into()))?;
        context.revoked = true;
        context.issued = Instant::now() - CONTEXT_TTL;
    }
    entries.remove_revoked(id);
    Ok(())
}

/// Native callers use this only after verified complete transaction compensation.
pub fn bind_file_lock_scope(error: AppError, original_scope: &[(PathBuf, String)]) -> AppError {
    let AppError::FileLock {
        path: _,
        recovery_id,
    } = error
    else {
        return error;
    };
    match bind_scope(&recovery_id, original_scope, current_game()) {
        Ok((path, recovery_id)) => AppError::FileLock { path, recovery_id },
        Err(error) => error,
    }
}

fn bind_scope(
    id: &str,
    original_scope: &[(PathBuf, String)],
    game: Option<FileLockGameSnapshot>,
) -> Result<(String, String), AppError> {
    let original = lookup(id)?;
    let mut original_context = original.try_lock().map_err(|_| state_error())?;
    if original_context.issued.elapsed() >= CONTEXT_TTL
        || original_scope.is_empty()
        || original_scope.len() > MAX_BULK_TARGETS
    {
        return Err(state_error());
    }
    validate_root(&original_context, game.clone())?;
    let normalized = verified_scope(&original_context, original_scope)?;
    let restored = normalized
        .iter()
        .find(|(_, identity)| identity == original_context.path_proof.identity())
        .map(|(path, _)| path.clone())
        .ok_or_else(state_error)?;
    let mut rebound =
        create_context(&restored, game.ok_or_else(state_error)?).ok_or_else(state_error)?;
    if rebound.root_proof.identity() != original_context.root_proof.identity()
        || rebound.path_proof.identity() != original_context.path_proof.identity()
    {
        return Err(state_error());
    }
    rebound.scope = normalized;
    validate_original(&rebound, current_game())?;
    let path = rebound.path.to_string_lossy().into_owned();
    let mut store = contexts().lock().map_err(|_| state_error())?;
    if !store
        .entries
        .get(id)
        .is_some_and(|entry| Arc::ptr_eq(&entry.context, &original))
    {
        return Err(state_error());
    }
    original_context.revoked = true;
    original_context.process_refs.clear();
    store.remove_revoked(id);
    let new_id = store.insert(rebound).ok_or_else(state_error)?;
    Ok((path, new_id))
}

fn verified_scope(
    context: &RecoveryContext,
    original_scope: &[(PathBuf, String)],
) -> Result<Vec<(PathBuf, String)>, AppError> {
    let mut normalized = BTreeMap::new();
    let mut leaf_count = 0;
    for (path, identity) in original_scope {
        let path = path.canonicalize().map_err(|_| state_error())?;
        if path == context.game.root
            || !path.starts_with(&context.game.root)
            || crate::platform::fs::file_utils::filesystem_identity(&path).as_deref()
                != Some(identity)
        {
            return Err(state_error());
        }
        if let Some(previous) = normalized.insert(path, identity.clone()) {
            if previous != *identity {
                return Err(state_error());
            }
        }
    }
    for identity in normalized.values() {
        if identity == context.path_proof.identity() {
            leaf_count += 1;
        }
    }
    if leaf_count != 1 {
        return Err(state_error());
    }
    Ok(normalized.into_iter().collect())
}

pub fn rebind_file_lock_error(error: AppError, restored_paths: &[PathBuf]) -> AppError {
    let AppError::FileLock {
        ref recovery_id, ..
    } = error
    else {
        return error;
    };
    let expected = match original_identity(recovery_id) {
        Ok(identity) => identity,
        Err(error) => return error,
    };
    let matches: Vec<(PathBuf, String)> = restored_paths
        .iter()
        .filter(|path| {
            crate::platform::fs::file_utils::filesystem_identity(path).as_deref() == Some(&expected)
        })
        .map(|path| (path.clone(), expected.clone()))
        .collect();
    bind_file_lock_scope(error, &matches)
}

fn original_identity(id: &str) -> Result<String, AppError> {
    let context = lookup(id)?;
    let context = context.try_lock().map_err(|_| state_error())?;
    Ok(context.path_proof.identity().to_string())
}

fn report(
    id: &str,
    context: &mut RecoveryContext,
    discovery: LockDiscovery,
    outcome: Option<String>,
) -> FileLockReport {
    let mut new_refs = BTreeMap::new();
    let processes = discovery
        .processes
        .into_iter()
        .map(|process| {
            let process_ref = context
                .process_refs
                .iter()
                .find(|(_, known)| {
                    known.pid == process.pid && known.started_at == process.started_at
                })
                .map(|(reference, _)| reference.clone())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let dto = FileLockProcess {
                process_ref: process_ref.clone(),
                pid: process.pid,
                name: process.name.clone(),
                executable: process.executable.clone(),
                started_at: process.started_at.clone(),
                application_type: process.application_type,
                session_id: process.session_id,
                service: (!process.service.is_empty()).then(|| process.service.clone()),
                can_close: process.can_close,
                can_terminate: process.can_terminate,
                unavailable_reason: process.unavailable_reason.clone(),
            };
            new_refs.insert(process_ref, process);
            dto
        })
        .collect();
    context.process_refs = new_refs;
    FileLockReport {
        recovery_id: id.to_string(),
        path: context.path.to_string_lossy().into_owned(),
        status: match discovery.status {
            "complete" => FileLockDiscoveryStatus::Complete,
            "partial" => FileLockDiscoveryStatus::Partial,
            "unknown" => FileLockDiscoveryStatus::Unknown,
            "unavailable" => FileLockDiscoveryStatus::Unavailable,
            _ => FileLockDiscoveryStatus::Failed,
        },
        message: discovery.message,
        processes,
        action_outcome: outcome,
    }
}

pub fn get_file_lock_info(id: &str) -> Result<FileLockReport, AppError> {
    let context = lookup(id)?;
    let mut context = context
        .try_lock()
        .map_err(|_| AppError::Validation("A recovery action is still running".into()))?;
    renew_context(&mut context, current_game())?;
    let _pins = locking::pin_lock_context(&context.game.root, &context.path).map_err(|error| {
        AppError::Io(format!(
            "Cannot safely inspect the locked target: {error}. Release its handles and retry."
        ))
    })?;
    validate(&context, current_game())?;
    let discovery = locking::discover_lock_processes(&context.path);
    validate(&context, current_game())?;
    Ok(report(id, &mut context, discovery, None))
}

pub fn control_file_lock_process(
    id: &str,
    process_ref: &str,
    force: bool,
) -> Result<FileLockReport, AppError> {
    let context = lookup(id)?;
    let mut context = context
        .try_lock()
        .map_err(|_| AppError::Validation("A recovery action is still running".into()))?;
    validate(&context, current_game())?;
    let selected = context
        .process_refs
        .get(process_ref)
        .cloned()
        .ok_or_else(|| {
            AppError::Validation(
                "The selected process is stale or was not reported by this recovery context".into(),
            )
        })?;
    let _pins = locking::pin_lock_context(&context.game.root, &context.path).map_err(|error| {
        AppError::Io(format!(
            "Cannot safely inspect the locked target: {error}. Release its handles and retry."
        ))
    })?;
    validate(&context, current_game())?;
    let discovery = locking::discover_lock_processes(&context.path);
    validate(&context, current_game())?;
    let Some(process) = discovery
        .processes
        .iter()
        .find(|process| process.pid == selected.pid && process.started_at == selected.started_at)
    else {
        return Ok(report(id, &mut context, discovery, Some("The selected process is no longer reported using these files. Retry the operation to verify the lock.".into())));
    };
    if force && !process.can_terminate || !force && !process.can_close {
        return Err(AppError::Validation(
            process
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "This process action is unavailable".into()),
        ));
    }
    let outcome = locking::control_lock_process(process, force).map_err(AppError::Validation)?;
    validate(&context, current_game())?;
    let discovery = locking::discover_lock_processes(&context.path);
    validate(&context, current_game())?;
    Ok(report(id, &mut context, discovery, Some(outcome)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_target_and_game_switch_expire_context() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("mod");
        std::fs::create_dir(&target).unwrap();
        let game = FileLockGameSnapshot {
            game_id: "fixture".into(),
            root: root.path().into(),
        };
        let context = create_context(&target, game.clone()).unwrap();
        assert!(validate(&context, Some(game.clone())).is_ok());
        assert!(validate(
            &context,
            Some(FileLockGameSnapshot {
                game_id: "other".into(),
                ..game.clone()
            })
        )
        .is_err());
        std::fs::rename(&target, root.path().join("original")).unwrap();
        std::fs::create_dir(&target).unwrap();
        assert!(validate(&context, Some(game)).is_err());
    }
    #[test]
    fn contexts_reject_roots_outside_paths_and_expiration() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let game = FileLockGameSnapshot {
            game_id: "fixture".into(),
            root: root.path().into(),
        };
        assert!(create_context(root.path(), game.clone()).is_none());
        assert!(create_context(outside.path(), game.clone()).is_none());
        let target = root.path().join("mod");
        std::fs::create_dir(&target).unwrap();
        let mut context = create_context(&target, game.clone()).unwrap();
        context.issued = Instant::now() - CONTEXT_TTL;
        assert!(validate(&context, Some(game)).is_err());
        assert!(lookup("forged").is_err());
    }

    #[test]
    fn retained_expired_context_renews_only_original_owned_target() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("mod");
        std::fs::create_dir(&target).unwrap();
        let game = FileLockGameSnapshot {
            game_id: "renewal".into(),
            root: root.path().into(),
        };
        let mut context = create_context(&target, game.clone()).unwrap();
        context.issued = Instant::now() - CONTEXT_TTL;
        context.process_refs.insert(
            "old-process-ref".into(),
            LockProcess {
                pid: 123,
                started_at: "456".into(),
                name: "owned mock".into(),
                executable: None,
                application_type: 1,
                session_id: 1,
                service: String::new(),
                can_close: false,
                can_terminate: false,
                unavailable_reason: None,
            },
        );
        assert!(validate(&context, Some(game.clone())).is_err());
        renew_context(&mut context, Some(game.clone())).unwrap();
        assert!(validate(&context, Some(game.clone())).is_ok());
        assert!(context.process_refs.is_empty());
        context.issued = Instant::now() - CONTEXT_TTL;
        std::fs::rename(&target, root.path().join("original-mod")).unwrap();
        std::fs::create_dir(&target).unwrap();
        assert!(renew_context(&mut context, Some(game)).is_err());
    }

    #[test]
    fn changed_root_or_revoked_context_cannot_be_renewed() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("mods");
        std::fs::create_dir(&root).unwrap();
        let target = root.join("mod");
        std::fs::create_dir(&target).unwrap();
        let game = FileLockGameSnapshot {
            game_id: "renewal-root".into(),
            root: root.clone(),
        };
        let mut context = create_context(&target, game.clone()).unwrap();
        context.revoked = true;
        assert!(renew_context(&mut context, Some(game.clone())).is_err());
        context.revoked = false;
        std::fs::rename(&root, parent.path().join("original-root")).unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&target).unwrap();
        assert!(renew_context(&mut context, Some(game)).is_err());
    }

    #[test]
    fn scope_blocks_replaced_rolled_back_sibling_even_when_locked_leaf_survives() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("A");
        let blocked = root.path().join("B");
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&blocked).unwrap();
        let game = FileLockGameSnapshot {
            game_id: "scope".into(),
            root: root.path().into(),
        };
        let mut context = create_context(&blocked, game.clone()).unwrap();
        let original_scope = vec![
            (
                first.clone(),
                crate::platform::fs::file_utils::filesystem_identity(&first).unwrap(),
            ),
            (blocked.clone(), context.path_proof.identity().to_string()),
        ];
        context.scope = verified_scope(&context, &original_scope).unwrap();
        assert!(validate_original(&context, Some(game.clone())).is_ok());
        std::fs::rename(&first, root.path().join("old-A")).unwrap();
        std::fs::create_dir(&first).unwrap();
        context.path_proof.validate(&blocked).unwrap();
        assert!(validate_original(&context, Some(game.clone())).is_err());
        context.issued = Instant::now() - CONTEXT_TTL;
        assert!(renew_context(&mut context, Some(game)).is_err());
    }

    #[test]
    fn verified_original_scope_recovers_blocked_leaf_after_ancestor_rollback() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("A");
        let leaf = parent.join("B");
        std::fs::create_dir_all(&leaf).unwrap();
        let originals = vec![
            (
                parent.clone(),
                crate::platform::fs::file_utils::filesystem_identity(&parent).unwrap(),
            ),
            (
                leaf.clone(),
                crate::platform::fs::file_utils::filesystem_identity(&leaf).unwrap(),
            ),
        ];
        let moved_parent = root.path().join("DISABLED A");
        std::fs::rename(&parent, &moved_parent).unwrap();
        let game = FileLockGameSnapshot {
            game_id: "rollback".into(),
            root: root.path().into(),
        };
        let context = create_context(&moved_parent.join("B"), game.clone()).unwrap();
        std::fs::rename(&moved_parent, &parent).unwrap();
        assert!(validate_original(&context, Some(game.clone())).is_err());
        let scope = verified_scope(&context, &originals).unwrap();
        let restored = scope
            .iter()
            .find(|(_, identity)| identity == context.path_proof.identity())
            .unwrap();
        let mut rebound = create_context(&restored.0, game.clone()).unwrap();
        assert_eq!(rebound.path_proof.identity(), context.path_proof.identity());
        rebound.scope = scope;
        validate_original(&rebound, Some(game)).unwrap();
    }

    #[test]
    fn registry_preserves_large_bulk_deduplicates_and_does_not_evict_fresh_contexts() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("mod");
        std::fs::create_dir(&target).unwrap();
        let game = FileLockGameSnapshot {
            game_id: "capacity".into(),
            root: root.path().into(),
        };
        let template = create_context(&target, game).unwrap();
        let mut store = ContextStore::default();
        let first = store.insert(template.clone()).unwrap();
        assert_eq!(store.insert(template.clone()).unwrap(), first);
        for index in 1..MAX_CONTEXTS {
            let mut context = template.clone();
            context.path = root.path().join(format!("synthetic-capacity-key-{index}"));
            assert!(store.insert(context).is_some());
        }
        assert_eq!(store.entries.len(), MAX_CONTEXTS);
        assert!(store.entries.contains_key(&first));
        let mut additional = template.clone();
        additional.path = root.path().join("overflow");
        assert!(store.insert(additional).is_none());
        assert!(store.entries.contains_key(&first));
        let retained = store.entries.get(&first).unwrap().context.clone();
        retained.lock().unwrap().revoked = true;
        store.remove_revoked(&first);
        assert!(!store.entries.contains_key(&first));
        assert!(renew_context(&mut retained.lock().unwrap(), Some(template.game)).is_err());
    }

    #[test]
    fn replay_scope_allows_original_subset_but_rejects_newly_planned_targets() {
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("B");
        let newly_active = root.path().join("C");
        let initially_noop = root.path().join("A");
        for path in [&original, &newly_active, &initially_noop] {
            std::fs::create_dir(path).unwrap();
        }
        let game = FileLockGameSnapshot {
            game_id: "replay-membership".into(),
            root: root.path().into(),
        };
        let mut context = create_context(&original, game).unwrap();
        let original_b = (original.clone(), context.path_proof.identity().to_string());
        context.scope = vec![(original.canonicalize().unwrap(), original_b.1.clone())];
        assert!(validate_replay_scope(&context, std::slice::from_ref(&original_b)).is_ok());
        let c = (
            newly_active.clone(),
            crate::platform::fs::file_utils::filesystem_identity(&newly_active).unwrap(),
        );
        assert!(validate_replay_scope(&context, &[original_b.clone(), c]).is_err());
        std::fs::rename(&initially_noop, root.path().join("old-A")).unwrap();
        std::fs::create_dir(&initially_noop).unwrap();
        let replaced_a = (
            initially_noop.clone(),
            crate::platform::fs::file_utils::filesystem_identity(&initially_noop).unwrap(),
        );
        assert!(validate_replay_scope(&context, &[original_b.clone(), replaced_a]).is_err());
        assert!(
            validate_replay_scope(&context, &[(original, "different-identity".into())]).is_err()
        );
    }

    #[test]
    fn replay_scope_accepts_original_nested_child_and_leaf_only_legacy_context() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("A");
        let child = parent.join("B");
        std::fs::create_dir_all(&child).unwrap();
        let game = FileLockGameSnapshot {
            game_id: "nested-replay".into(),
            root: root.path().into(),
        };
        let mut context = create_context(&child, game).unwrap();
        let proposed_child = (child.clone(), context.path_proof.identity().to_string());
        assert!(validate_replay_scope(&context, std::slice::from_ref(&proposed_child)).is_ok());
        let proposed_parent = (
            parent.clone(),
            crate::platform::fs::file_utils::filesystem_identity(&parent).unwrap(),
        );
        assert!(validate_replay_scope(&context, std::slice::from_ref(&proposed_parent)).is_err());
        context.scope =
            verified_scope(&context, &[proposed_parent.clone(), proposed_child.clone()]).unwrap();
        assert!(validate_replay_scope(&context, &[proposed_parent, proposed_child]).is_ok());
        assert!(validate_replay_scope(&context, &[]).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn replay_scope_compares_canonical_physical_case_without_disabled_aliases() {
        let root = tempfile::tempdir().unwrap();
        let child = root.path().join("OriginalCase");
        let disabled = root.path().join("DISABLED OriginalCase");
        std::fs::create_dir(&child).unwrap();
        std::fs::create_dir(&disabled).unwrap();
        let game = FileLockGameSnapshot {
            game_id: "case-replay".into(),
            root: root.path().into(),
        };
        let context = create_context(&child, game).unwrap();
        let changed_case = root.path().join("originalcase");
        assert!(validate_replay_scope(
            &context,
            &[(changed_case, context.path_proof.identity().to_string())]
        )
        .is_ok());
        let disabled_identity =
            crate::platform::fs::file_utils::filesystem_identity(&disabled).unwrap();
        assert!(validate_replay_scope(&context, &[(disabled, disabled_identity)]).is_err());
    }
}

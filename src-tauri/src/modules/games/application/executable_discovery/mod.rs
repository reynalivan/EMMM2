mod providers;
#[cfg(test)]
mod tests;

use super::executable_profiles::{
    executable_names, inspect_game_executable, validate_local_path, ExecutableProfileError,
};
use crate::modules::games::domain::{executable::*, models::GameType};
use crate::shared::errors::AppError;
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

pub const MAX_CONFIG_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_PROVIDER_ENTRIES: usize = 256;
pub const MAX_CONFIG_DEPTH: usize = 3;
pub const MAX_CANDIDATES: usize = 128;
pub const MAX_XXMI_ROOTS: usize = 16;
pub const WORK_BUDGET: Duration = Duration::from_secs(10);

pub struct DiscoveryLocations {
    pub roaming: Option<PathBuf>,
    pub xxmi_roots: Vec<PathBuf>,
}

pub struct DiscoveryContext {
    pub(crate) result: GameExecutableDiscoveryResult,
    requested: Vec<GameType>,
    identities: HashMap<(u8, String, String), usize>,
    locations_seen: HashMap<GameExecutableSource, usize>,
    deadline: Instant,
}

impl DiscoveryContext {
    pub fn new(requested: Vec<GameType>, started: Instant) -> Self {
        Self {
            result: GameExecutableDiscoveryResult {
                candidates: Vec::new(),
                issues: Vec::new(),
                complete: true,
            },
            requested,
            identities: HashMap::new(),
            locations_seen: HashMap::new(),
            deadline: started + WORK_BUDGET,
        }
    }

    pub fn issue(&mut self, source: GameExecutableSource, code: GameExecutableIssueCode) {
        self.result.complete = false;
        if !self
            .result
            .issues
            .iter()
            .any(|issue| issue.source == source && issue.code == code)
        {
            self.result
                .issues
                .push(GameExecutableDiscoveryIssue { source, code });
        }
    }

    pub fn within_budget(&mut self, source: GameExecutableSource) -> bool {
        if Instant::now() >= self.deadline {
            self.issue(source, GameExecutableIssueCode::DeadlineExceeded);
            false
        } else {
            true
        }
    }

    pub fn requested(&self, game_type: GameType) -> bool {
        self.requested.contains(&game_type)
    }

    pub fn location(
        &mut self,
        game_type: GameType,
        path: &str,
        source: GameExecutableSource,
        region: Option<String>,
        channel: Option<String>,
    ) {
        if !self.requested(game_type) || !self.within_budget(source) {
            return;
        }
        let seen = self.locations_seen.entry(source).or_default();
        *seen += 1;
        if *seen > MAX_PROVIDER_ENTRIES {
            self.issue(source, GameExecutableIssueCode::ResourceLimit);
            return;
        }
        let Ok(path) = validate_local_path(path) else {
            self.issue(source, GameExecutableIssueCode::InvalidLocation);
            return;
        };
        let paths = if path
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
        {
            vec![path]
        } else {
            executable_names(game_type)
                .iter()
                .map(|name| path.join(name))
                .collect()
        };
        for path in paths {
            if !self.within_budget(source) {
                break;
            }
            // A stale launcher location is ordinary absence. A valid result still
            // requires the complete profile; basename alone is never sufficient.
            let inspected = inspect_game_executable(game_type, &path.to_string_lossy());
            let Some(mut candidate) = self.profile_result(source, inspected) else {
                continue;
            };
            if source == GameExecutableSource::HoyoplayRegistry
                && !self.registry_marker(
                    PathBuf::from(&candidate.install_root)
                        .join("config.ini")
                        .metadata(),
                )
            {
                continue;
            }
            let key = (
                game_type as u8,
                physical_location_key(&candidate.install_root),
                physical_location_key(&candidate.executable_path),
            );
            if let Some(index) = self.identities.get(&key) {
                let existing = &mut self.result.candidates[*index];
                if !existing.sources.contains(&source) {
                    existing.sources.push(source);
                }
                if existing.region.is_none() {
                    existing.region = region.clone();
                }
                if existing.channel.is_none() {
                    existing.channel = channel.clone();
                }
                continue;
            }
            if self.result.candidates.len() >= MAX_CANDIDATES {
                self.issue(source, GameExecutableIssueCode::ResourceLimit);
                break;
            }
            candidate.sources = vec![source];
            candidate.region = region.clone();
            if channel.is_some() {
                candidate.channel = channel.clone();
            }
            self.identities.insert(key, self.result.candidates.len());
            self.result.candidates.push(candidate);
        }
    }

    fn profile_result(
        &mut self,
        source: GameExecutableSource,
        result: Result<GameExecutableCandidate, ExecutableProfileError>,
    ) -> Option<GameExecutableCandidate> {
        match result {
            Ok(candidate) => Some(candidate),
            Err(error) => {
                if error.code != GameExecutableIssueCode::InvalidLocation {
                    self.issue(source, error.code);
                }
                None
            }
        }
    }

    fn registry_marker(&mut self, result: std::io::Result<std::fs::Metadata>) -> bool {
        match result {
            Ok(metadata) => metadata.is_file(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => {
                self.issue(
                    GameExecutableSource::HoyoplayRegistry,
                    ExecutableProfileError::from(error).code,
                );
                false
            }
        }
    }

    pub fn finish(mut self) -> GameExecutableDiscoveryResult {
        self.result.candidates.sort_by_key(|candidate| {
            (
                candidate.game_type as u8,
                candidate.executable_path.to_lowercase(),
            )
        });
        self.result
    }
}

fn physical_location_key(path: &str) -> String {
    crate::platform::fs::file_utils::filesystem_identity(std::path::Path::new(path))
        .unwrap_or_else(|| crate::shared::path_key::folder_path_key(path, None))
}

pub fn validate_request(request: &DiscoverGameExecutablesRequest) -> Result<(), AppError> {
    if request.game_types.is_empty()
        || request.game_types.len() > GameType::ALL.len()
        || request.xxmi_roots.len() > MAX_XXMI_ROOTS
    {
        return Err(AppError::Validation(
            "Choose one to five game types and at most sixteen known XXMI roots.".into(),
        ));
    }
    for root in &request.xxmi_roots {
        validate_local_path(root)?;
    }
    Ok(())
}

pub fn discover_configuration(context: &mut DiscoveryContext, locations: DiscoveryLocations) {
    let mut xxmi_roots = locations.xxmi_roots;
    if let Some(roaming) = &locations.roaming {
        let default_root = roaming.join("XXMI Launcher");
        if !xxmi_roots.contains(&default_root) {
            xxmi_roots.push(default_root);
        }
    }
    providers::xxmi(context, &xxmi_roots);
    if let Some(roaming) = locations.roaming {
        providers::kuro(context, &roaming.join("KRLauncher"));
        providers::hoyoplay(context, &roaming.join("Cognosphere/HYP"));
    }
}

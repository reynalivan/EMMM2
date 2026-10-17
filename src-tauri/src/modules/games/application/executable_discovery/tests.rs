use super::*;
use crate::modules::games::{
    adapters::executable_registry::{collect_hoyoplay, RegistryReader},
    application::executable_profiles::*,
};
use std::{fs, path::Path};

fn installation(base: &Path, game: GameType) -> PathBuf {
    let root = base.join(game.to_string());
    let marker = match game {
        GameType::Gimi => "GenshinImpact_Data",
        GameType::Srmi => "StarRail_Data",
        GameType::Zzmi => "ZenlessZoneZero_Data",
        GameType::Efmi => "Endfield_Data",
        GameType::Wwmi => "Engine",
    };
    fs::create_dir_all(root.join(marker)).unwrap();
    let exe = root.join(executable_names(game)[0]);
    fs::write(&exe, []).unwrap();
    fs::write(root.join("config.ini"), "[General]\n").unwrap();
    if game == GameType::Wwmi {
        fs::create_dir_all(root.join("Client/Binaries/Win64")).unwrap();
        fs::write(root.join(WWMI_PROCESS), []).unwrap();
    }
    exe
}

fn write_json(path: &Path, value: serde_json::Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

#[test]
fn all_five_profiles_validate_exact_layout_and_reject_other_games() {
    let temp = tempfile::tempdir().unwrap();
    for game in GameType::ALL {
        let exe = installation(temp.path(), game);
        let candidate = validate_game_executable(game, &exe.to_string_lossy()).unwrap();
        assert_eq!(
            PathBuf::from(&candidate.executable_path),
            exe.canonicalize().unwrap()
        );
        let another = GameType::ALL
            .into_iter()
            .find(|other| *other != game)
            .unwrap();
        assert!(validate_game_executable(another, &exe.to_string_lossy()).is_err());
        fs::remove_file(&exe).unwrap();
        assert!(validate_game_executable(game, &candidate.executable_path).is_err());
    }
}

#[test]
fn wwmi_wrapper_resolves_shipping_only_inside_validated_installation() {
    let temp = tempfile::tempdir().unwrap();
    let wrapper = installation(temp.path(), GameType::Wwmi);
    let shipping = wrapper
        .parent()
        .unwrap()
        .join(WWMI_PROCESS)
        .canonicalize()
        .unwrap();
    assert_eq!(
        resolve_process_executable(GameType::Wwmi, Some(&wrapper)),
        Some(shipping.clone())
    );
    assert_eq!(
        resolve_process_executable(GameType::Wwmi, Some(&shipping)),
        Some(shipping.clone())
    );
    assert!(validate_game_executable(GameType::Wwmi, &shipping.to_string_lossy()).is_ok());
    fs::remove_file(&shipping).unwrap();
    assert!(resolve_process_executable(GameType::Wwmi, Some(&wrapper)).is_none());
    let manual = temp.path().join("custom.exe");
    assert_eq!(
        resolve_process_executable(GameType::Wwmi, Some(&manual)),
        Some(manual)
    );
    assert_eq!(resolve_process_executable(GameType::Gimi, None), None);
}

#[test]
fn rejects_commands_remote_paths_and_missing_resources() {
    for value in [
        "",
        "GenshinImpact.exe",
        "https://site/game.exe",
        "\\\\server\\game.exe",
        "\\\\?\\UNC\\server\\game.exe",
        "C:/game.exe\" --flag",
        "C:/game.exe\n",
    ] {
        assert!(validate_local_path(value).is_err(), "{value}");
    }
    let temp = tempfile::tempdir().unwrap();
    let exe = installation(temp.path(), GameType::Efmi);
    fs::remove_dir(exe.parent().unwrap().join("Endfield_Data")).unwrap();
    assert!(validate_game_executable(GameType::Efmi, &exe.to_string_lossy()).is_err());
}

#[test]
fn xxmi_reads_only_allowlisted_game_folder_fields_and_merges_sources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("XXMI Unicode 空格");
    let mut importers = serde_json::Map::new();
    for game in GameType::ALL {
        let exe = installation(temp.path(), game);
        importers.insert(game.to_string(),serde_json::json!({"Importer":{"game_folder":exe.parent().unwrap(),"custom_launch":"do not execute"}}));
    }
    write_json(
        &root.join("XXMI Launcher Config.json"),
        serde_json::json!({"Importers":importers}),
    );
    let mut context = DiscoveryContext::new(GameType::ALL.to_vec(), Instant::now());
    discover_configuration(
        &mut context,
        DiscoveryLocations {
            roaming: None,
            xxmi_roots: vec![root.clone(), root],
        },
    );
    let result = context.finish();
    assert!(result.complete);
    assert_eq!(result.candidates.len(), 5);
    assert!(result
        .candidates
        .iter()
        .all(|candidate| candidate.sources == vec![GameExecutableSource::XxmiConfig]));
}

#[test]
fn malformed_oversized_and_missing_config_keep_other_provider_results() {
    let temp = tempfile::tempdir().unwrap();
    let exe = installation(temp.path(), GameType::Wwmi);
    let roaming = temp.path().join("Roaming");
    write_json(
        &roaming.join("KRLauncher/one/kr_starter_game.json"),
        serde_json::json!({"path":exe.parent().unwrap()}),
    );
    let bad = temp.path().join("bad");
    fs::create_dir(&bad).unwrap();
    fs::write(bad.join("XXMI Launcher Config.json"), "{bad").unwrap();
    let large = temp.path().join("large");
    fs::create_dir(&large).unwrap();
    let file = fs::File::create(large.join("XXMI Launcher Config.json")).unwrap();
    file.set_len(MAX_CONFIG_BYTES + 1).unwrap();
    let mut context = DiscoveryContext::new(GameType::ALL.to_vec(), Instant::now());
    discover_configuration(
        &mut context,
        DiscoveryLocations {
            roaming: Some(roaming),
            xxmi_roots: vec![bad, large, temp.path().join("missing")],
        },
    );
    let result = context.finish();
    assert!(!result.complete);
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(
        result.candidates[0].sources,
        vec![GameExecutableSource::KuroConfig]
    );
    assert!(result
        .issues
        .iter()
        .any(|issue| issue.code == GameExecutableIssueCode::MalformedConfig));
    assert!(result
        .issues
        .iter()
        .any(|issue| issue.code == GameExecutableIssueCode::ResourceLimit));
}

struct FixtureRegistry {
    path: String,
    denied: bool,
}
impl RegistryReader for FixtureRegistry {
    fn children(&self, path: &str, _view: u32) -> std::io::Result<Vec<String>> {
        if self.denied && path.contains("miHoYo") {
            return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        }
        if path.ends_with("HYP") {
            Ok(vec!["1.0".into()])
        } else {
            Ok(vec!["hk4e_global".into(), "unknown_global".into()])
        }
    }
    fn string(&self, _path: &str, _view: u32, name: &str) -> std::io::Result<Option<String>> {
        assert_eq!(name, "GameInstallPath");
        Ok(Some(self.path.clone()))
    }
}

#[test]
fn registry_both_views_deduplicate_and_preserve_access_denial_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let exe = installation(temp.path(), GameType::Gimi);
    let registry = FixtureRegistry {
        path: exe.parent().unwrap().to_string_lossy().into_owned(),
        denied: true,
    };
    let mut context = DiscoveryContext::new(vec![GameType::Gimi], Instant::now());
    collect_hoyoplay(&mut context, &registry);
    context.location(
        GameType::Gimi,
        &registry.path,
        GameExecutableSource::XxmiConfig,
        None,
        None,
    );
    let result = context.finish();
    assert!(!result.complete);
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(
        result.candidates[0].sources,
        vec![
            GameExecutableSource::HoyoplayRegistry,
            GameExecutableSource::XxmiConfig
        ]
    );
    assert_eq!(result.candidates[0].region.as_deref(), Some("global"));
    assert_eq!(result.issues[0].code, GameExecutableIssueCode::AccessDenied);
}

#[test]
fn expired_budget_and_provider_traversal_limits_are_partial() {
    let temp = tempfile::tempdir().unwrap();
    let mut context = DiscoveryContext::new(GameType::ALL.to_vec(), Instant::now() - WORK_BUDGET);
    discover_configuration(
        &mut context,
        DiscoveryLocations {
            roaming: None,
            xxmi_roots: vec![temp.path().to_path_buf()],
        },
    );
    assert_eq!(
        context.finish().issues[0].code,
        GameExecutableIssueCode::DeadlineExceeded
    );
    let root = temp.path().join("KRLauncher");
    fs::create_dir(&root).unwrap();
    for index in 0..=MAX_PROVIDER_ENTRIES {
        fs::write(root.join(format!("{index}.txt")), []).unwrap();
    }
    let mut context = DiscoveryContext::new(vec![GameType::Wwmi], Instant::now());
    discover_configuration(
        &mut context,
        DiscoveryLocations {
            roaming: Some(temp.path().to_path_buf()),
            xxmi_roots: vec![],
        },
    );
    assert_eq!(
        context.finish().issues[0].code,
        GameExecutableIssueCode::ResourceLimit
    );
}

#[test]
fn request_limits_reject_unbounded_roots_and_empty_roster() {
    assert!(validate_request(&DiscoverGameExecutablesRequest {
        game_types: vec![],
        xxmi_roots: vec![]
    })
    .is_err());
    assert!(validate_request(&DiscoverGameExecutablesRequest {
        game_types: vec![GameType::Efmi],
        xxmi_roots: vec!["C:/".into(); MAX_XXMI_ROOTS + 1]
    })
    .is_err());
}

#[test]
fn inaccessible_profile_is_partial_but_missing_or_mismatched_install_is_ordinary_absence() {
    for (kind, expected) in [
        (
            std::io::ErrorKind::PermissionDenied,
            Some(GameExecutableIssueCode::AccessDenied),
        ),
        (
            std::io::ErrorKind::Other,
            Some(GameExecutableIssueCode::ReadFailed),
        ),
        (std::io::ErrorKind::NotFound, None),
    ] {
        let mut context = DiscoveryContext::new(vec![GameType::Efmi], Instant::now());
        assert!(context
            .profile_result(
                GameExecutableSource::XxmiConfig,
                Err(std::io::Error::from(kind).into())
            )
            .is_none());
        let result = context.finish();
        assert_eq!(result.complete, expected.is_none());
        assert_eq!(result.issues.first().map(|issue| issue.code), expected);
    }
}

#[test]
fn registry_configuration_marker_reports_denied_reads_but_ignores_absence() {
    for (kind, expected) in [
        (
            std::io::ErrorKind::PermissionDenied,
            Some(GameExecutableIssueCode::AccessDenied),
        ),
        (
            std::io::ErrorKind::Other,
            Some(GameExecutableIssueCode::ReadFailed),
        ),
        (std::io::ErrorKind::NotFound, None),
    ] {
        let mut context = DiscoveryContext::new(vec![GameType::Gimi], Instant::now());
        assert!(!context.registry_marker(Err(std::io::Error::from(kind))));
        let result = context.finish();
        assert_eq!(result.complete, expected.is_none());
        assert_eq!(result.issues.first().map(|issue| issue.code), expected);
    }
}

#[test]
fn unrelated_registry_is_not_read_for_endfield_or_wuthering_waves() {
    struct UnexpectedRegistry;
    impl RegistryReader for UnexpectedRegistry {
        fn children(&self, _path: &str, _view: u32) -> std::io::Result<Vec<String>> {
            panic!("Unrequested provider read");
        }
        fn string(&self, _path: &str, _view: u32, _name: &str) -> std::io::Result<Option<String>> {
            panic!("Unrequested provider read");
        }
    }
    let mut context = DiscoveryContext::new(vec![GameType::Efmi, GameType::Wwmi], Instant::now());
    collect_hoyoplay(&mut context, &UnexpectedRegistry);
    assert!(context.finish().complete);
    let mut expired = DiscoveryContext::new(vec![GameType::Gimi], Instant::now() - WORK_BUDGET);
    collect_hoyoplay(&mut expired, &UnexpectedRegistry);
    assert_eq!(
        expired.finish().issues[0].code,
        GameExecutableIssueCode::DeadlineExceeded
    );
}

#[test]
fn known_regions_beta_and_launcher_hints_remain_distinct_candidates() {
    let temp = tempfile::tempdir().unwrap();
    let cn_root = temp.path().join("CN");
    fs::create_dir_all(cn_root.join("GenshinImpact_Data")).unwrap();
    fs::write(cn_root.join("YuanShen.exe"), []).unwrap();
    let beta = temp.path().join("Beta");
    fs::create_dir_all(beta.join("ZenlessZoneZero_Data")).unwrap();
    fs::write(beta.join("ZenlessZoneZeroBeta.exe"), []).unwrap();
    assert!(validate_game_executable(
        GameType::Gimi,
        &cn_root.join("YuanShen.exe").to_string_lossy()
    )
    .is_ok());
    assert_eq!(
        validate_game_executable(
            GameType::Zzmi,
            &beta.join("ZenlessZoneZeroBeta.exe").to_string_lossy()
        )
        .unwrap()
        .channel
        .as_deref(),
        Some("beta")
    );
    let exe = installation(temp.path(), GameType::Efmi);
    let xxmi = temp.path().join("XXMI");
    write_json(
        &xxmi.join("XXMI Launcher Config.json"),
        serde_json::json!({"Importers":{"EFMI":{"Importer":{"game_folder":exe.parent().unwrap()}}}}),
    );
    let mut context = DiscoveryContext::new(vec![GameType::Efmi], Instant::now());
    discover_configuration(
        &mut context,
        DiscoveryLocations {
            roaming: None,
            xxmi_roots: vec![xxmi.join("XXMI Launcher.exe"), xxmi.join("EFMI")],
        },
    );
    assert_eq!(context.finish().candidates.len(), 1);
}

#[test]
fn hoyoplay_text_candidates_are_validated_not_treated_as_commands() {
    let temp = tempfile::tempdir().unwrap();
    let exe = installation(temp.path(), GameType::Srmi);
    let root = temp.path().join("Roaming");
    write_json(
        &root.join("Cognosphere/HYP/1/gamedata.dat"),
        serde_json::json!({"installPath":exe.parent().unwrap(),"persistentInstallPath":"https://invalid/game.exe","command":"C:/fake/game.exe --run"}),
    );
    let mut context = DiscoveryContext::new(vec![GameType::Srmi], Instant::now());
    discover_configuration(
        &mut context,
        DiscoveryLocations {
            roaming: Some(root),
            xxmi_roots: vec![],
        },
    );
    let result = context.finish();
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(
        result.candidates[0].sources,
        vec![GameExecutableSource::HoyoplayConfig]
    );
}

#[test]
fn candidate_limit_preserves_first_valid_results_and_reports_partial() {
    let temp = tempfile::tempdir().unwrap();
    let mut context = DiscoveryContext::new(vec![GameType::Efmi], Instant::now());
    for index in 0..=MAX_CANDIDATES {
        let exe = installation(&temp.path().join(index.to_string()), GameType::Efmi);
        context.location(
            GameType::Efmi,
            &exe.to_string_lossy(),
            GameExecutableSource::XxmiConfig,
            None,
            None,
        );
    }
    let result = context.finish();
    assert_eq!(result.candidates.len(), MAX_CANDIDATES);
    assert!(!result.complete);
    assert_eq!(
        result.issues[0].code,
        GameExecutableIssueCode::ResourceLimit
    );
}

#[test]
fn hardlinked_executables_in_distinct_installations_remain_separate_candidates() {
    let temp = tempfile::tempdir().unwrap();
    let first = installation(&temp.path().join("first"), GameType::Efmi);
    let second_root = temp.path().join("second");
    fs::create_dir_all(second_root.join("Endfield_Data")).unwrap();
    let second = second_root.join("Endfield.exe");
    fs::hard_link(&first, &second).unwrap();
    assert_eq!(
        crate::platform::fs::file_utils::filesystem_identity(&first),
        crate::platform::fs::file_utils::filesystem_identity(&second)
    );

    let mut context = DiscoveryContext::new(vec![GameType::Efmi], Instant::now());
    for executable in [&first, &second] {
        context.location(
            GameType::Efmi,
            &executable.to_string_lossy(),
            GameExecutableSource::XxmiConfig,
            None,
            None,
        );
    }
    assert_eq!(context.result.candidates.len(), 2);

    let first_alias = first.parent().unwrap().join(".").join("Endfield.exe");
    context.location(
        GameType::Efmi,
        &first_alias.to_string_lossy(),
        GameExecutableSource::XxmiConfig,
        None,
        None,
    );
    let result = context.finish();
    assert!(result.complete);
    assert_eq!(result.candidates.len(), 2);
    for root in [first.parent().unwrap(), second_root.as_path()] {
        assert!(result.candidates.iter().any(|candidate| {
            Path::new(&candidate.install_root) == root.canonicalize().unwrap()
        }));
    }
}

#[test]
fn default_xxmi_location_discovers_all_profiles_without_explicit_roots() {
    let temp = tempfile::tempdir().unwrap();
    let roaming = temp.path().join("Roaming 空格");
    let mut importers = serde_json::Map::new();
    for game in GameType::ALL {
        let executable = installation(temp.path(), game);
        importers.insert(
            game.to_string(),
            serde_json::json!({"Importer":{"game_folder":executable.parent().unwrap()}}),
        );
    }
    write_json(
        &roaming.join("XXMI Launcher/XXMI Launcher Config.json"),
        serde_json::json!({"Importers":importers}),
    );
    let mut context = DiscoveryContext::new(GameType::ALL.to_vec(), Instant::now());
    discover_configuration(
        &mut context,
        DiscoveryLocations {
            roaming: Some(roaming),
            xxmi_roots: Vec::new(),
        },
    );
    let result = context.finish();
    assert!(result.complete);
    assert_eq!(result.candidates.len(), GameType::ALL.len());
    assert!(result
        .candidates
        .iter()
        .all(|candidate| candidate.sources == vec![GameExecutableSource::XxmiConfig]));
}

#[test]
fn default_xxmi_location_preserves_explicit_candidates_and_partial_errors() {
    let temp = tempfile::tempdir().unwrap();
    let executable = installation(&temp.path().join("custom game"), GameType::Efmi);
    let other_executable = installation(&temp.path().join("other game"), GameType::Efmi);
    let custom = temp.path().join("portable XXMI");
    let config = serde_json::json!({"Importers":{"EFMI":{"Importer":{"game_folder":executable.parent().unwrap()}}}});
    write_json(&custom.join("XXMI Launcher Config.json"), config.clone());

    for scenario in ["missing", "malformed", "duplicate", "distinct"] {
        let roaming = temp.path().join(scenario);
        let default_config = roaming.join("XXMI Launcher/XXMI Launcher Config.json");
        match scenario {
            "missing" => {}
            "malformed" => {
                fs::create_dir_all(default_config.parent().unwrap()).unwrap();
                fs::write(&default_config, "{invalid").unwrap();
            }
            "duplicate" => write_json(&default_config, config.clone()),
            "distinct" => write_json(
                &default_config,
                serde_json::json!({"Importers":{"EFMI":{"Importer":{"game_folder":other_executable.parent().unwrap()}}}}),
            ),
            _ => unreachable!(),
        }
        let mut context = DiscoveryContext::new(vec![GameType::Efmi], Instant::now());
        discover_configuration(
            &mut context,
            DiscoveryLocations {
                roaming: Some(roaming),
                xxmi_roots: vec![custom.clone()],
            },
        );
        let result = context.finish();
        assert_eq!(
            result.candidates.len(),
            if scenario == "distinct" { 2 } else { 1 }
        );
        assert!(result.candidates.iter().any(|candidate| {
            Path::new(&candidate.executable_path) == executable.canonicalize().unwrap()
        }));
        assert_eq!(result.complete, scenario != "malformed", "{scenario}");
        if scenario == "malformed" {
            assert_eq!(result.issues.len(), 1);
            assert_eq!(result.issues[0].source, GameExecutableSource::XxmiConfig);
            assert_eq!(
                result.issues[0].code,
                GameExecutableIssueCode::MalformedConfig
            );
        } else {
            assert!(result.issues.is_empty(), "{scenario}");
        }
    }
}

#[test]
fn xxmi_root_inputs_resolve_repository_supported_launcher_and_importer_layouts() {
    let temp = tempfile::tempdir().unwrap();
    let executable = installation(temp.path(), GameType::Efmi);
    let root = temp.path().join("XXMI");
    let importer = root.join("EFMI");
    let mods = importer.join("Mods");
    fs::create_dir_all(&mods).unwrap();
    let launcher = root.join("Resources/Bin/XXMI Launcher.exe");
    fs::create_dir_all(launcher.parent().unwrap()).unwrap();
    fs::write(&launcher, []).unwrap();
    write_json(
        &root.join("XXMI Launcher Config.json"),
        serde_json::json!({"Importers":{"EFMI":{"Importer":{"game_folder":executable.parent().unwrap()}}}}),
    );
    for input in [root, importer, mods, launcher] {
        let mut context = DiscoveryContext::new(vec![GameType::Efmi], Instant::now());
        discover_configuration(
            &mut context,
            DiscoveryLocations {
                roaming: None,
                xxmi_roots: vec![input.clone()],
            },
        );
        let result = context.finish();
        assert!(result.complete, "{input:?}");
        assert_eq!(result.candidates.len(), 1, "{input:?}");
    }
}

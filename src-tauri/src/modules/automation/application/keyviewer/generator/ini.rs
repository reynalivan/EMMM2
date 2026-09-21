use std::path::Path;

use crate::modules::automation::application::keyviewer::matcher::MatchResult;
use crate::modules::games::domain::models::GameType;
use crate::shared::errors::AppError;

use super::atomic::atomic_write;
use super::keybind_text::keybind_file_name;

fn text_api_namespace(game_type: GameType) -> Result<&'static str, AppError> {
    match game_type {
        GameType::GIMI => Ok("GIMIv8"),
        GameType::SRMI => Ok("SRMIv1"),
        GameType::WWMI => Ok("WWMIv1"),
        GameType::ZZMI => Ok("ZZMIv1"),
        GameType::EFMI => Err(AppError::Validation(
            "KeyViewer text overlay is not supported by the installed EFMI profile".to_string(),
        )),
    }
}

fn section_component(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn validate_sentinel(hash: &str) -> Result<(), AppError> {
    if hash.len() == 8 && hash.chars().all(|character| character.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(AppError::Validation(format!(
        "KeyViewer sentinel must be an 8-digit resource hash: {hash}"
    )))
}

fn detection_variable(index: usize) -> String {
    format!("$emmm_kv_detect_{index:03}")
}

fn keyviewer_resource(index: usize) -> String {
    format!("ResourceEMMM_KeyViewer_{index:03}")
}

fn keyviewer_box_resource(index: usize) -> String {
    format!("ResourceEMMM_KeyViewerBox_{index}")
}

const OVERLAY_STATUS_LEFT: f32 = -0.96;
const OVERLAY_STATUS_RIGHT: f32 = -0.30;
const OVERLAY_PANEL_LEFT: f32 = -0.96;
const OVERLAY_PANEL_RIGHT: f32 = -0.56;
const OVERLAY_STATUS_TOP: f32 = 0.36;
const OVERLAY_STATUS_BOTTOM: f32 = 0.24;
const OVERLAY_PANEL_TOP: f32 = -0.24;
const OVERLAY_PANEL_BOTTOM: f32 = -0.92;
const OVERLAY_TEXT_ALIGNMENT_LEFT: u8 = 0;
const OVERLAY_VERTICAL_ANCHOR_BOTTOM: u8 = 3;
const OVERLAY_STATUS_SCALE: f32 = 1.00;
const OVERLAY_CHARACTER_SCALE: f32 = 0.92;
// Keep the incomplete preset status banner out of the game viewport. KeyViewer panels remain
// enabled and are rendered separately below when a matching character is detected.
const PRESET_STATUS_OVERLAY_ENABLED: bool = false;

/// Bumped when generated overlay output changes so existing overlays are republished.
pub const KEYVIEWER_LAYOUT_REVISION: u8 = 5;

fn text_box_data(left: f32, top: f32, right: f32, bottom: f32, scale: f32) -> String {
    format!(
        "{left:.2} {top:.2} {right:.2} {bottom:.2}  1 1 1 1  0 0 0 0.92  0.02 0.02  {OVERLAY_TEXT_ALIGNMENT_LEFT} {OVERLAY_VERTICAL_ANCHOR_BOTTOM}  0  {scale:.2}"
    )
}

fn status_box_data() -> String {
    text_box_data(
        OVERLAY_STATUS_LEFT,
        OVERLAY_STATUS_TOP,
        OVERLAY_STATUS_RIGHT,
        OVERLAY_STATUS_BOTTOM,
        OVERLAY_STATUS_SCALE,
    )
}

/// Keep every character panel anchored to the same normalized viewport box.
/// Character match order must not affect where the active panel is rendered.
fn character_box_data() -> String {
    text_box_data(
        OVERLAY_PANEL_LEFT,
        OVERLAY_PANEL_TOP,
        OVERLAY_PANEL_RIGHT,
        OVERLAY_PANEL_BOTTOM,
        OVERLAY_CHARACTER_SCALE,
    )
}

fn resource_path(resource_root: &str, relative: &str) -> Result<String, AppError> {
    let resource_root = resource_root.trim_matches(['/', '\\']);
    if resource_root.is_empty() {
        return Ok(relative.to_string());
    }
    if resource_root
        .split(['/', '\\'])
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(AppError::Validation(
            "KeyViewer resource root is not a safe relative path".to_string(),
        ));
    }
    Ok(format!("{resource_root}/{relative}"))
}

/// Generate one additive, package-portable KeyViewer entrypoint.
///
/// The observer uses resource-hash TextureOverrides only to set its own
/// per-frame flags. Rendering happens before the `post` resets, so a detected
/// character disappears on the next frame it is no longer rendered.
pub fn generate_keyviewer_ini(
    matches: &[MatchResult],
    toggle_key: &str,
    game_type: GameType,
) -> Result<String, AppError> {
    generate_keyviewer_ini_for_resources(matches, toggle_key, game_type, "")
}

/// Generate an entrypoint that reads text resources from the single stable
/// `generations/` directory. The caller publishes that directory from a
/// complete staging tree before refreshing the entrypoint.
pub fn generate_keyviewer_ini_for_resources(
    matches: &[MatchResult],
    toggle_key: &str,
    game_type: GameType,
    resource_root: &str,
) -> Result<String, AppError> {
    let text_namespace = text_api_namespace(game_type)?;
    let toggle_key =
        crate::modules::automation::application::hotkeys::validate_3dmigoto_binding(toggle_key)?;
    for sentinel in matches.iter().flat_map(|result| result.sentinels.iter()) {
        validate_sentinel(&sentinel.hash)?;
    }

    let mut lines = vec![
        "; =================================================================".to_string(),
        "; KeyViewer.ini — generated by EMMM. Do not edit manually.".to_string(),
        "; Requires the installed package text renderer and active".to_string(),
        "; checktextureoverride callbacks for the selected targets.".to_string(),
        "; =================================================================".to_string(),
        "; EMMM-Artifact: KeyViewer v1".to_string(),
        "namespace = EMMMv1".to_string(),
        String::new(),
        "[Constants]".to_string(),
        "global persist $emmm_kv_active = 0".to_string(),
    ];

    for index in 0..matches.len() {
        lines.push(format!("global {} = 0", detection_variable(index)));
    }

    lines.extend([
        String::new(),
        "[KeyEMMMv1_ToggleOverlay]".to_string(),
        format!(
            "key = {}",
            toggle_key.to_ascii_uppercase().replace('+', " ")
        ),
        "type = cycle".to_string(),
        "$emmm_kv_active = 0, 1".to_string(),
        String::new(),
    ]);

    for (match_index, result) in matches.iter().enumerate() {
        for (sentinel_index, sentinel) in result.sentinels.iter().enumerate() {
            lines.push(format!(
                "[TextureOverride_EMMMv1_{}_{}_S{sentinel_index}]",
                section_component(&result.object_name),
                match_index
            ));
            lines.push(format!("hash = {}", sentinel.hash));
            if let Some(match_first_index) = sentinel.match_first_index {
                lines.push(format!("match_first_index = {match_first_index}"));
            }
            lines.push("match_priority = -100".to_string());
            lines.push(format!("{} = 1", detection_variable(match_index)));
            lines.push(String::new());
        }
    }

    lines.extend([
        "[Present]".to_string(),
        "run = CommandList_EMMMv1_Render".to_string(),
    ]);
    for index in 0..matches.len() {
        lines.push(format!("post {} = 0", detection_variable(index)));
    }
    lines.push(String::new());

    lines.extend([
        "[CommandList_EMMMv1_Render]".to_string(),
        "if $emmm_kv_active == 1".to_string(),
    ]);

    if PRESET_STATUS_OVERLAY_ENABLED {
        lines.extend([
            format!("    Resource\\{text_namespace}\\Text = ref ResourceEMMM_Status"),
            format!("    Resource\\{text_namespace}\\TextParams = ref ResourceEMMM_StatusBox"),
            format!("    run = CommandList\\{text_namespace}\\PrintText"),
        ]);
    }

    for (match_index, _) in matches.iter().enumerate() {
        lines.push(format!("    if {} == 1", detection_variable(match_index)));
        lines.push(format!(
            "        Resource\\{text_namespace}\\Text = ref {}",
            keyviewer_resource(match_index)
        ));
        lines.push(format!(
            "        Resource\\{text_namespace}\\TextParams = ref {}",
            keyviewer_box_resource(match_index)
        ));
        lines.push(format!(
            "        run = CommandList\\{text_namespace}\\PrintText"
        ));
        lines.push("    endif".to_string());
    }
    lines.extend(["endif".to_string(), String::new()]);

    lines.extend([
        "[ResourceEMMM_StatusBox]".to_string(),
        "type = StructuredBuffer".to_string(),
        "array = 1".to_string(),
        format!("data = R32_FLOAT  {}", status_box_data()),
        String::new(),
    ]);

    for box_index in 0..matches.len() {
        lines.push(format!("[{}]", keyviewer_box_resource(box_index)));
        lines.push("type = StructuredBuffer".to_string());
        lines.push("array = 1".to_string());
        lines.push(format!("data = R32_FLOAT  {}", character_box_data()));
        lines.push(String::new());
    }

    lines.extend([
        "[ResourceEMMM_Status]".to_string(),
        "type = buffer".to_string(),
        "format = R8_UINT".to_string(),
        format!(
            "filename = {}",
            resource_path(resource_root, "status/runtime_status.txt")?
        ),
        String::new(),
    ]);

    for (index, _) in matches.iter().enumerate() {
        lines.push(format!("[{}]", keyviewer_resource(index)));
        lines.push("type = buffer".to_string());
        lines.push("format = R8_UINT".to_string());
        lines.push(format!(
            "filename = {}",
            resource_path(
                resource_root,
                &format!("keybinds/active/{}", keybind_file_name(index))
            )?
        ));
        lines.push(String::new());
    }

    Ok(lines.join("\n"))
}

pub fn write_keyviewer_ini(
    output_path: &Path,
    matches: &[MatchResult],
    toggle_key: &str,
    game_type: GameType,
) -> Result<(), AppError> {
    let content = generate_keyviewer_ini(matches, toggle_key, game_type)?;
    atomic_write(output_path, &content)
}

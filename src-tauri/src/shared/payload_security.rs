use std::path::Path;

const BLOCKED_MOD_PAYLOAD_EXTENSIONS: &[&str] = &[
    "bat",
    "cmd",
    "com",
    "dll",
    "exe",
    "hta",
    "jar",
    "js",
    "jse",
    "lnk",
    "msi",
    "msix",
    "msixbundle",
    "ps1",
    "scr",
    "vbe",
    "vbs",
    "wsf",
];

pub fn blocked_mod_payload_extension(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?;
    BLOCKED_MOD_PAYLOAD_EXTENSIONS
        .iter()
        .copied()
        .find(|blocked| extension.eq_ignore_ascii_case(blocked))
}

#[cfg(test)]
mod tests {
    use super::blocked_mod_payload_extension;
    use std::path::Path;

    #[test]
    fn detects_executable_and_script_extensions_case_insensitively() {
        assert_eq!(
            blocked_mod_payload_extension(Path::new("mod-installer.EXE")),
            Some("exe")
        );
        assert_eq!(
            blocked_mod_payload_extension(Path::new("setup.ps1")),
            Some("ps1")
        );
        assert_eq!(
            blocked_mod_payload_extension(Path::new("mod-pack.zip")),
            None
        );
    }
}

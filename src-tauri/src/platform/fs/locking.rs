use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
mod windows;

const MAX_FILES: usize = 1_024;
const MAX_DIRECTORIES: usize = 4_096;
const DISCOVERY_BUDGET: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub(crate) struct LockProcess {
    pub pid: u32,
    pub started_at: String,
    pub name: String,
    pub executable: Option<String>,
    pub application_type: i32,
    pub session_id: u32,
    pub service: String,
    pub can_close: bool,
    pub can_terminate: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug)]
pub(crate) struct LockDiscovery {
    pub status: &'static str,
    pub message: Option<String>,
    pub processes: Vec<LockProcess>,
}

#[derive(Debug)]
struct LockResources {
    files: Vec<PathBuf>,
    partial: bool,
    _pins: Vec<std::fs::File>,
}

fn pin_resource(path: &Path) -> std::io::Result<std::fs::File> {
    #[cfg(target_os = "windows")]
    let file = {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_DATA,
            FILE_SHARE_READ, FILE_SHARE_WRITE,
        };
        std::fs::OpenOptions::new()
            .access_mode(FILE_READ_DATA)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?
    };
    #[cfg(not(target_os = "windows"))]
    let file = std::fs::File::open(path)?;
    if is_reparse(&file.metadata()?) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "File-lock recovery cannot follow a reparse point",
        ));
    }
    Ok(file)
}

pub(crate) fn pin_lock_context(root: &Path, target: &Path) -> std::io::Result<Vec<std::fs::File>> {
    let relative = target.strip_prefix(root).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Recovery target is outside its configured root",
        )
    })?;
    let mut path = root.to_path_buf();
    let mut handles = vec![pin_resource(&path)?];
    for component in relative.components() {
        path.push(component);
        handles.push(pin_resource(&path)?);
    }
    Ok(handles)
}

fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        metadata.file_type().is_symlink()
    }
}

fn collect_resources(path: &Path) -> std::io::Result<LockResources> {
    let started = Instant::now();
    let metadata = std::fs::symlink_metadata(path)?;
    if is_reparse(&metadata) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "File-lock discovery cannot follow a reparse point",
        ));
    }
    if metadata.is_file() {
        return Ok(LockResources {
            files: vec![path.to_path_buf()],
            partial: false,
            _pins: vec![pin_resource(path)?],
        });
    }
    let mut result = LockResources {
        files: Vec::new(),
        partial: false,
        _pins: Vec::new(),
    };
    let mut directories = vec![path.to_path_buf()];
    let mut directory_count = 0;
    while let Some(directory) = directories.pop() {
        if started.elapsed() >= DISCOVERY_BUDGET || directory_count >= MAX_DIRECTORIES {
            result.partial = true;
            break;
        }
        directory_count += 1;
        let pin = match pin_resource(&directory) {
            Ok(pin) => pin,
            Err(_) => {
                result.partial = true;
                continue;
            }
        };
        result._pins.push(pin);
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                result.partial = true;
                continue;
            }
        };
        for entry in entries {
            if result.files.len() >= MAX_FILES || started.elapsed() >= DISCOVERY_BUDGET {
                result.partial = true;
                return Ok(result);
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    result.partial = true;
                    continue;
                }
            };
            let metadata = match std::fs::symlink_metadata(entry.path()) {
                Ok(metadata) => metadata,
                Err(_) => {
                    result.partial = true;
                    continue;
                }
            };
            if is_reparse(&metadata) {
                result.partial = true;
                continue;
            }
            if metadata.is_dir() {
                if directories.len() >= MAX_DIRECTORIES {
                    result.partial = true;
                } else {
                    directories.push(entry.path());
                }
            } else if metadata.is_file() {
                match pin_resource(&entry.path()) {
                    Ok(pin) => {
                        result._pins.push(pin);
                        result.files.push(entry.path());
                    }
                    Err(_) => {
                        result.partial = true;
                    }
                }
            }
        }
    }
    Ok(result)
}

pub(crate) fn discover_lock_processes(path: &Path) -> LockDiscovery {
    #[cfg(target_os = "windows")]
    {
        match collect_resources(path) {
            Ok(resources) => windows::discover(resources),
            Err(error) => LockDiscovery {
                status: "failed",
                message: Some(error.to_string()),
                processes: Vec::new(),
            },
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        LockDiscovery {
            status: "unavailable",
            message: Some("Process discovery is available on Windows only".into()),
            processes: Vec::new(),
        }
    }
}

pub(crate) fn control_lock_process(process: &LockProcess, force: bool) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        windows::control(process, force)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (process, force);
        Err("Process control is available on Windows only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn recovery_pins_prevent_target_and_root_namespace_changes() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("root");
        let target = root.join("mod");
        std::fs::create_dir_all(&target).unwrap();
        let pins = pin_lock_context(&root, &target).unwrap();
        assert_eq!(
            std::fs::rename(&target, root.join("replacement"))
                .unwrap_err()
                .raw_os_error(),
            Some(32)
        );
        assert_eq!(
            std::fs::rename(&root, fixture.path().join("replacement-root"))
                .unwrap_err()
                .raw_os_error(),
            Some(32)
        );
        drop(pins);
        std::fs::rename(&target, root.join("replacement")).unwrap();
        std::fs::rename(&root, fixture.path().join("replacement-root")).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn recovery_pins_reject_existing_delete_access_and_allow_shared_readers() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY, FILE_SHARE_DELETE,
            FILE_SHARE_READ, FILE_SHARE_WRITE,
        };
        let fixture = tempfile::tempdir().unwrap();
        let target = fixture.path().join("mod");
        std::fs::create_dir(&target).unwrap();
        let holder = std::fs::OpenOptions::new()
            .access_mode(DELETE | FILE_LIST_DIRECTORY)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(&target)
            .unwrap();
        assert_eq!(
            pin_lock_context(fixture.path(), &target)
                .unwrap_err()
                .raw_os_error(),
            Some(32)
        );
        drop(holder);
        let reader = std::fs::OpenOptions::new()
            .access_mode(FILE_LIST_DIRECTORY)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(&target)
            .unwrap();
        let pins = pin_lock_context(fixture.path(), &target).unwrap();
        drop(pins);
        drop(reader);
    }

    #[test]
    fn discovery_registers_files_never_directories() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("child")).unwrap();
        std::fs::write(temp.path().join("child/mod.ini"), b"fixture").unwrap();
        let resources = collect_resources(temp.path()).unwrap();
        assert_eq!(resources.files.len(), 1);
        assert!(resources.files[0].is_file());
        assert!(!resources.partial);
    }

    #[test]
    fn discovery_caps_large_folders_and_marks_partial() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..=MAX_FILES {
            std::fs::write(temp.path().join(index.to_string()), b"").unwrap();
        }
        let resources = collect_resources(temp.path()).unwrap();
        assert!(resources.files.len() <= MAX_FILES);
        assert!(resources.partial);
    }
}

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// An open destination directory used for race-free relative publication.
///
/// On Windows the handle denies delete sharing, which prevents the directory
/// name from being replaced while the handle is retained. On Linux the file
/// descriptor is passed directly to `renameat2`, so publication remains bound
/// to the opened directory even if another process renames its path.
#[derive(Debug)]
pub(crate) struct PinnedDirectory {
    path: PathBuf,
    handle: fs::File,
    identity: String,
}

impl PinnedDirectory {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        Self::open_with_access(path, false)
    }

    pub(crate) fn open_for_rename(path: &Path) -> io::Result<Self> {
        Self::open_with_access(path, true)
    }

    fn open_with_access(path: &Path, delete_access: bool) -> io::Result<Self> {
        let handle = open_directory(path, delete_access)?;
        if !handle.metadata()?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Pinned destination is not a directory: {}", path.display()),
            ));
        }
        let identity = directory_handle_identity(&handle, path)?;
        Ok(Self {
            path: path.to_path_buf(),
            handle,
            identity,
        })
    }

    pub(crate) fn rename_into(&self, parent: &Self, name: &std::ffi::OsStr) -> io::Result<()> {
        rename_open_directory_into(&self.handle, &self.path, parent, name)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn identity(&self) -> &str {
        &self.identity
    }

    pub(crate) fn stable_path(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            PathBuf::from(format!("/proc/self/fd/{}", self.handle.as_raw_fd()))
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.path.clone()
        }
    }

    pub(crate) fn current_path(&self) -> io::Result<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            fs::read_link(self.stable_path())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(self.path.clone())
        }
    }

    pub(crate) fn create_child(&self, name: &std::ffi::OsStr) -> io::Result<Self> {
        validate_child_name(name)?;
        let temporary = tempfile::Builder::new()
            .prefix(".emmm_import_parent_")
            .tempdir_in(self.stable_path())?;
        let handle = open_directory(temporary.path(), true)?;
        let identity = directory_handle_identity(&handle, temporary.path())?;
        rename_open_directory_into(&handle, temporary.path(), self, name)?;
        let _retained_path = temporary.keep();
        Ok(Self {
            path: self.path.join(name),
            handle,
            identity,
        })
    }

    #[cfg(windows)]
    pub(crate) fn open_for_mutation(path: &Path) -> io::Result<Self> {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY,
            FILE_SHARE_READ, FILE_SHARE_WRITE,
        };
        let handle = fs::OpenOptions::new()
            .access_mode(DELETE | FILE_LIST_DIRECTORY)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = handle.metadata()?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Owned source must be a non-linked directory",
            ));
        }
        let identity = directory_handle_identity(&handle, path)?;
        Ok(Self {
            path: path.to_path_buf(),
            handle,
            identity,
        })
    }

    #[cfg(not(windows))]
    pub(crate) fn open_for_mutation(_path: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Ownership-bound directory mutation is unavailable on this platform",
        ))
    }

    #[cfg(windows)]
    pub(crate) fn rename_to(&self, parent: &Self, name: &std::ffi::OsStr) -> io::Result<()> {
        validate_child_name(name)?;
        rename_open_directory_into(&self.handle, &self.path, parent, name)
    }

    #[cfg(windows)]
    pub(crate) fn remove_empty(self, parent: &Self) -> io::Result<()> {
        self.remove_empty_with_before_disposition(parent, || {})
    }

    #[cfg(windows)]
    fn remove_empty_with_before_disposition(
        self,
        parent: &Self,
        before_disposition: impl FnOnce(),
    ) -> io::Result<()> {
        use std::mem;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::{HANDLE, TRUE};
        use windows_sys::Win32::Storage::FileSystem::{
            FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
        };
        if self.path.parent() != Some(parent.path()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Owned directory is not below its pinned parent",
            ));
        }
        if fs::read_dir(&self.path)?.next().transpose()?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::DirectoryNotEmpty,
                "Owned directory contains added files",
            ));
        }
        let disposition = FILE_DISPOSITION_INFO {
            DeleteFile: TRUE as u8,
        };
        before_disposition();
        // The handle denies delete sharing, and Windows rechecks emptiness at
        // disposition. Neither replacement nor a raced child can be deleted.
        let result = unsafe {
            SetFileInformationByHandle(
                self.handle.as_raw_handle() as HANDLE,
                FileDispositionInfo,
                &disposition as *const FILE_DISPOSITION_INFO as _,
                mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        };
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(not(windows))]
    pub(crate) fn remove_empty(self, _parent: &Self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Ownership-bound empty-directory removal is unavailable on this platform",
        ))
    }
}

#[cfg(all(test, windows))]
mod owned_directory_tests {
    use super::*;

    #[test]
    fn pinned_creation_source_cannot_be_replaced_before_disposition() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("Owned");
        fs::create_dir(&path).unwrap();
        let parent = PinnedDirectory::open(fixture.path()).unwrap();
        let source = PinnedDirectory::open_for_mutation(&path).unwrap();
        source
            .remove_empty_with_before_disposition(&parent, || {
                assert!(fs::rename(&path, fixture.path().join("Moved")).is_err());
                assert!(path.is_dir());
            })
            .unwrap();
        assert!(!path.exists());
        assert!(!fixture.path().join("Moved").exists());
    }

    #[test]
    fn a_child_added_after_empty_check_prevents_handle_deletion() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("Owned");
        fs::create_dir(&path).unwrap();
        let parent = PinnedDirectory::open(fixture.path()).unwrap();
        let source = PinnedDirectory::open_for_mutation(&path).unwrap();
        let asset = path.join("added.txt");
        assert!(source
            .remove_empty_with_before_disposition(&parent, || {
                fs::write(&asset, b"raced asset").unwrap();
            })
            .is_err());
        assert_eq!(fs::read(asset).unwrap(), b"raced asset");
    }
}

fn validate_child_name(name: &std::ffi::OsStr) -> io::Result<()> {
    let path = Path::new(name);
    if path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination leaf must be one normal path component",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn open_directory(path: &Path, delete_access: bool) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    let access = FILE_LIST_DIRECTORY | if delete_access { DELETE } else { 0 };
    fs::OpenOptions::new()
        .access_mode(access)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(
            FILE_FLAG_BACKUP_SEMANTICS
                | if delete_access {
                    FILE_FLAG_OPEN_REPARSE_POINT
                } else {
                    0
                },
        )
        .open(path)
}

#[cfg(not(windows))]
fn open_directory(path: &Path, _delete_access: bool) -> io::Result<fs::File> {
    fs::File::open(path)
}

#[cfg(windows)]
fn directory_handle_identity(handle: &fs::File, _path: &Path) -> io::Result<String> {
    use std::mem;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
        BY_HANDLE_FILE_INFORMATION, FILE_ID_INFO,
    };

    let mut high: FILE_ID_INFO = unsafe { mem::zeroed() };
    let high_result = unsafe {
        GetFileInformationByHandleEx(
            handle.as_raw_handle() as HANDLE,
            FileIdInfo,
            &mut high as *mut FILE_ID_INFO as _,
            mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if high_result != 0 {
        return Ok(format!(
            "win-high:{}:{}",
            high.VolumeSerialNumber,
            u128::from_le_bytes(high.FileId.Identifier)
        ));
    }
    let mut low: BY_HANDLE_FILE_INFORMATION = unsafe { mem::zeroed() };
    let low_result =
        unsafe { GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, &mut low) };
    if low_result == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(format!(
        "win-low:{}:{}",
        low.dwVolumeSerialNumber,
        ((low.nFileIndexHigh as u64) << 32) | u64::from(low.nFileIndexLow)
    ))
}

#[cfg(target_family = "unix")]
fn directory_handle_identity(handle: &fs::File, _path: &Path) -> io::Result<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = handle.metadata()?;
    Ok(format!("inode:{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(not(any(windows, target_family = "unix")))]
fn directory_handle_identity(_handle: &fs::File, path: &Path) -> io::Result<String> {
    crate::platform::fs::file_utils::filesystem_identity(path).ok_or_else(|| {
        io::Error::other(format!(
            "Cannot identify pinned destination directory: {}",
            path.display()
        ))
    })
}

#[cfg(windows)]
fn rename_open_directory_into(
    source_handle: &fs::File,
    _source_path: &Path,
    destination_parent: &PinnedDirectory,
    destination_name: &std::ffi::OsStr,
) -> io::Result<()> {
    rename_handle_into(source_handle, destination_parent, destination_name)
}

#[cfg(not(windows))]
fn rename_open_directory_into(
    _source_handle: &fs::File,
    source_path: &Path,
    destination_parent: &PinnedDirectory,
    destination_name: &std::ffi::OsStr,
) -> io::Result<()> {
    rename_no_replace_into(source_path, destination_parent, destination_name)
}

#[cfg(windows)]
pub(crate) fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_uint};
    use std::os::unix::ffi::OsStrExt;

    unsafe extern "C" {
        fn renameat2(
            old_dir_fd: c_int,
            old_path: *const c_char,
            new_dir_fd: c_int,
            new_path: *const c_char,
            flags: c_uint,
        ) -> c_int;
    }

    const AT_FDCWD: c_int = -100;
    const RENAME_NOREPLACE: c_uint = 1;
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in source path"))?;
    let destination = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in destination path"))?;
    let result = unsafe {
        renameat2(
            AT_FDCWD,
            source.as_ptr(),
            AT_FDCWD,
            destination.as_ptr(),
            RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) fn rename_no_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Atomic no-replace rename is unavailable on this platform",
    ))
}

#[cfg(windows)]
pub(crate) fn rename_no_replace_into(
    source: &Path,
    destination_parent: &PinnedDirectory,
    destination_name: &std::ffi::OsStr,
) -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    validate_child_name(destination_name)?;
    let source_handle = fs::OpenOptions::new()
        .access_mode(DELETE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(source)?;
    rename_handle_into(&source_handle, destination_parent, destination_name)
}

#[cfg(windows)]
pub(crate) fn rename_handle_into(
    source_handle: &fs::File,
    destination_parent: &PinnedDirectory,
    destination_name: &std::ffi::OsStr,
) -> io::Result<()> {
    use std::ffi::c_void;
    use std::mem::{offset_of, size_of};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;

    #[repr(C)]
    union IoStatus {
        status: i32,
        pointer: *mut c_void,
    }

    #[repr(C)]
    struct IoStatusBlock {
        status: IoStatus,
        information: usize,
    }

    #[repr(C)]
    union RenameOptions {
        replace_if_exists: u8,
        flags: u32,
    }

    #[repr(C)]
    struct FileRenameInformation {
        options: RenameOptions,
        root_directory: HANDLE,
        file_name_length: u32,
        file_name: [u16; 1],
    }

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtSetInformationFile(
            file_handle: HANDLE,
            io_status_block: *mut IoStatusBlock,
            file_information: *mut c_void,
            length: u32,
            file_information_class: i32,
        ) -> i32;
        fn RtlNtStatusToDosError(status: i32) -> u32;
    }

    validate_child_name(destination_name)?;
    let wide = destination_name.encode_wide().collect::<Vec<_>>();
    let bytes = (offset_of!(FileRenameInformation, file_name) + wide.len() * size_of::<u16>())
        .max(size_of::<FileRenameInformation>());
    let words = bytes.div_ceil(size_of::<usize>());
    let mut buffer = vec![0_usize; words];
    let info = buffer.as_mut_ptr().cast::<FileRenameInformation>();
    let mut io_status = IoStatusBlock {
        status: IoStatus {
            pointer: std::ptr::null_mut(),
        },
        information: 0,
    };
    unsafe {
        (*info).options.flags = 0;
        (*info).root_directory = destination_parent.handle.as_raw_handle() as HANDLE;
        (*info).file_name_length = (wide.len() * size_of::<u16>()) as u32;
        std::ptr::copy_nonoverlapping(wide.as_ptr(), (*info).file_name.as_mut_ptr(), wide.len());
        const FILE_RENAME_INFORMATION_CLASS: i32 = 10;
        let status = NtSetInformationFile(
            source_handle.as_raw_handle() as HANDLE,
            &mut io_status,
            info.cast(),
            bytes as u32,
            FILE_RENAME_INFORMATION_CLASS,
        );
        if status < 0 {
            return Err(io::Error::from_raw_os_error(
                RtlNtStatusToDosError(status) as i32
            ));
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(crate) fn rename_no_replace_into(
    source: &Path,
    destination_parent: &PinnedDirectory,
    destination_name: &std::ffi::OsStr,
) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::fd::AsRawFd;
    use std::os::raw::{c_char, c_int, c_uint};
    use std::os::unix::ffi::OsStrExt;

    unsafe extern "C" {
        fn renameat2(
            old_dir_fd: c_int,
            old_path: *const c_char,
            new_dir_fd: c_int,
            new_path: *const c_char,
            flags: c_uint,
        ) -> c_int;
    }

    validate_child_name(destination_name)?;
    const AT_FDCWD: c_int = -100;
    const RENAME_NOREPLACE: c_uint = 1;
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in source path"))?;
    let destination = CString::new(destination_name.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in destination name"))?;
    let result = unsafe {
        renameat2(
            AT_FDCWD,
            source.as_ptr(),
            destination_parent.handle.as_raw_fd(),
            destination.as_ptr(),
            RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) fn rename_no_replace_into(
    _source: &Path,
    _destination_parent: &PinnedDirectory,
    _destination_name: &std::ffi::OsStr,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Handle-relative no-replace rename is unavailable on this platform",
    ))
}

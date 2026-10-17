use crate::modules::games::{
    application::executable_discovery::{DiscoveryContext, MAX_PROVIDER_ENTRIES},
    domain::{
        executable::{GameExecutableIssueCode, GameExecutableSource},
        models::GameType,
    },
};

pub trait RegistryReader {
    fn children(&self, path: &str, view: u32) -> std::io::Result<Vec<String>>;
    fn string(&self, path: &str, view: u32, name: &str) -> std::io::Result<Option<String>>;
}

const REGISTRY_VIEW_32BIT: u32 = 0x0200;
const REGISTRY_VIEW_64BIT: u32 = 0x0100;

pub fn collect_hoyoplay(context: &mut DiscoveryContext, registry: &impl RegistryReader) {
    if ![GameType::Gimi, GameType::Srmi, GameType::Zzmi]
        .iter()
        .any(|game| context.requested(*game))
    {
        return;
    }
    let source = GameExecutableSource::HoyoplayRegistry;
    let mut visited = 0;
    for view in [REGISTRY_VIEW_32BIT, REGISTRY_VIEW_64BIT] {
        for vendor in ["miHoYo", "Cognosphere"] {
            if !context.within_budget(source) {
                return;
            }
            let root = format!("Software\\{vendor}\\HYP");
            let versions = match registry.children(&root, view) {
                Ok(versions) => versions,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    context.issue(source, io_code(&error));
                    continue;
                }
            };
            for version in versions {
                if !context.within_budget(source) {
                    return;
                }
                visited += 1;
                if visited > MAX_PROVIDER_ENTRIES {
                    context.issue(source, GameExecutableIssueCode::ResourceLimit);
                    return;
                }
                let path = format!("{root}\\{version}");
                let games = match registry.children(&path, view) {
                    Ok(games) => games,
                    Err(error) => {
                        context.issue(source, io_code(&error));
                        continue;
                    }
                };
                for business in games {
                    if !context.within_budget(source) {
                        return;
                    }
                    visited += 1;
                    if visited > MAX_PROVIDER_ENTRIES {
                        context.issue(source, GameExecutableIssueCode::ResourceLimit);
                        return;
                    }
                    let Some((game, region, channel)) = business_profile(&business) else {
                        continue;
                    };
                    if !context.requested(game) {
                        continue;
                    }
                    match registry.string(&format!("{path}\\{business}"), view, "GameInstallPath") {
                        Ok(Some(path)) => context.location(
                            game,
                            &path,
                            source,
                            Some(region.into()),
                            Some(channel.into()),
                        ),
                        Ok(None) => {}
                        Err(error) => context.issue(source, io_code(&error)),
                    }
                }
            }
        }
    }
}

fn io_code(error: &std::io::Error) -> GameExecutableIssueCode {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => GameExecutableIssueCode::AccessDenied,
        std::io::ErrorKind::InvalidData => GameExecutableIssueCode::MalformedConfig,
        _ => GameExecutableIssueCode::ReadFailed,
    }
}

fn business_profile(business: &str) -> Option<(GameType, &'static str, &'static str)> {
    let (prefix, suffix) = business.split_once('_')?;
    let game = match prefix.to_ascii_lowercase().as_str() {
        "hk4e" => GameType::Gimi,
        "hkrpg" => GameType::Srmi,
        "nap" => GameType::Zzmi,
        _ => return None,
    };
    let (region, channel) = match suffix.to_ascii_lowercase().as_str() {
        "global" => ("global", "official"),
        "cn" => ("cn", "official"),
        "bilibili" => ("cn", "bilibili"),
        _ => return None,
    };
    Some((game, region, channel))
}

#[cfg(windows)]
pub struct WindowsRegistry;

#[cfg(windows)]
mod native {
    use super::*;
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS},
        System::Registry::*,
    };
    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn open(path: &str, view: u32) -> std::io::Result<Key> {
        let mut key = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                wide(path).as_ptr(),
                0,
                KEY_READ | view,
                &mut key,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(std::io::Error::from_raw_os_error(status as i32));
        }
        Ok(Key(key))
    }
    impl RegistryReader for WindowsRegistry {
        fn children(&self, path: &str, view: u32) -> std::io::Result<Vec<String>> {
            let key = open(path, view)?;
            let mut names = Vec::new();
            for index in 0..=MAX_PROVIDER_ENTRIES {
                let mut buffer = [0u16; 256];
                let mut length = buffer.len() as u32;
                let status = unsafe {
                    RegEnumKeyExW(
                        key.0,
                        index as u32,
                        buffer.as_mut_ptr(),
                        &mut length,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                if status == ERROR_NO_MORE_ITEMS {
                    break;
                }
                if status != ERROR_SUCCESS {
                    return Err(std::io::Error::from_raw_os_error(status as i32));
                }
                names.push(String::from_utf16(&buffer[..length as usize]).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid registry key")
                })?);
            }
            Ok(names)
        }
        fn string(&self, path: &str, view: u32, name: &str) -> std::io::Result<Option<String>> {
            let key = open(path, view)?;
            let mut buffer = vec![
                0u16;
                crate::modules::games::application::executable_profiles::MAX_EXECUTABLE_PATH
                    + 1
            ];
            let mut size = (buffer.len() * 2) as u32;
            let status = unsafe {
                RegGetValueW(
                    key.0,
                    std::ptr::null(),
                    wide(name).as_ptr(),
                    RRF_RT_REG_SZ | RRF_NOEXPAND,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr().cast(),
                    &mut size,
                )
            };
            if status == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            if status != ERROR_SUCCESS {
                return Err(std::io::Error::from_raw_os_error(status as i32));
            }
            buffer.truncate(size as usize / 2);
            while buffer.last() == Some(&0) {
                buffer.pop();
            }
            String::from_utf16(&buffer).map(Some).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid registry location")
            })
        }
    }
}

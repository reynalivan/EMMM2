pub use super::application::*;

#[cfg(test)]
pub(crate) use super::adapters::tauri::settings_cmds::persist_telemetry_consent;

#[cfg(debug_assertions)]
pub mod testing {
    pub mod adapters {
        pub use super::super::super::adapters::*;
    }
    pub mod application {
        pub use super::super::super::application::*;
    }
}

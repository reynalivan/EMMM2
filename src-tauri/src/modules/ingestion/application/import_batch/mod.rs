pub mod analyze;
pub mod coordinator;
pub mod extraction_state;
pub mod mod_inbox;
pub mod mod_inbox_watcher;
pub mod payload_manifest;
pub mod preview;
pub mod ready_to_move;
pub mod relocation;
pub mod staging;
pub mod target_manifest_index;
pub mod types;

#[cfg(test)]
mod tests;

fn is_cancelled(cancel_token: &Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) -> bool {
    cancel_token
        .as_ref()
        .is_some_and(|token| token.load(std::sync::atomic::Ordering::SeqCst))
}

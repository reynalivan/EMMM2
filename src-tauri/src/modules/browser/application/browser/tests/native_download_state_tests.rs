use super::*;
use std::time::Duration;

#[test]
fn native_publication_preserves_a_destination_created_after_admission() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("pack.zip");
    let transfer = NativeDownloadFile::new(&destination).unwrap();
    std::fs::write(transfer.staging_path(), b"download payload").unwrap();
    std::fs::write(&destination, b"intervening file").unwrap();

    assert!(transfer.publish().is_err());
    assert_eq!(std::fs::read(&destination).unwrap(), b"intervening file");
    assert_eq!(
        std::fs::read(transfer.staging_path()).unwrap(),
        b"download payload"
    );
}

#[test]
fn native_publication_moves_the_owned_staging_file_only_when_target_is_vacant() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("pack.zip");
    let transfer = NativeDownloadFile::new(&destination).unwrap();
    let staging = transfer.staging_path().to_path_buf();
    assert_ne!(staging, destination);
    std::fs::write(&staging, b"download payload").unwrap();

    transfer.publish().unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), b"download payload");
    assert!(!staging.exists());
    drop(transfer);
    assert!(destination.exists());
}

#[test]
fn released_native_destinations_are_available_for_the_next_request() {
    let mut admission = NativeDownloadAdmission::default();
    let path = PathBuf::from("C:/Downloads/pack.zip");
    admission
        .reserve("first", path.clone(), Instant::now())
        .unwrap();
    assert!(!admission.is_available(&path));
    admission.confirm("first", Instant::now()).unwrap();
    admission.release("first");
    assert!(admission.is_available(&path));
    admission.reserve("second", path, Instant::now()).unwrap();
}

#[test]
fn pending_capacity_is_bounded_without_losing_existing_reservations() {
    let mut admission = NativeDownloadAdmission::default();
    for index in 0..MAX_PENDING_CONFIRMATIONS {
        admission
            .reserve(
                &format!("request-{index}"),
                PathBuf::from(format!("pack-{index}.zip")),
                Instant::now(),
            )
            .unwrap();
    }
    assert!(matches!(
        admission.reserve("overflow", PathBuf::from("overflow.zip"), Instant::now()),
        Err(BrowserError::QueueFull)
    ));
    admission.release("request-0");
    admission
        .reserve("replacement", PathBuf::from("pack-0.zip"), Instant::now())
        .unwrap();
}

#[test]
fn active_native_downloads_are_bounded_and_release_capacity_on_completion() {
    let mut admission = NativeDownloadAdmission::default();
    let now = Instant::now();
    for index in 0..(MAX_CONCURRENT_DOWNLOADS + MAX_QUEUED_DOWNLOADS) {
        let id = format!("active-{index}");
        admission
            .reserve(&id, PathBuf::from(format!("active-{index}.zip")), now)
            .unwrap();
        admission.confirm(&id, now).unwrap();
    }
    assert!(matches!(
        admission.reserve("overflow", PathBuf::from("overflow.zip"), now),
        Err(BrowserError::QueueFull)
    ));
    admission.release("active-0");
    admission
        .reserve("replacement", PathBuf::from("active-0.zip"), now)
        .unwrap();
}

#[test]
fn expired_confirmations_cannot_be_activated_but_live_downloads_do_not_expire() {
    let mut admission = NativeDownloadAdmission::default();
    let now = Instant::now();
    admission
        .reserve("waiting", PathBuf::from("waiting.zip"), now)
        .unwrap();
    admission
        .reserve("active", PathBuf::from("active.zip"), now)
        .unwrap();
    admission.confirm("active", now).unwrap();
    let later = now + PENDING_CONFIRMATION_TTL + Duration::from_millis(1);
    assert_eq!(admission.expired_ids(later), vec!["waiting".to_string()]);
    assert!(matches!(
        admission.confirm("waiting", later),
        Err(BrowserError::DownloadConfirmationUnavailable)
    ));
    admission.release("waiting");
    assert!(admission.is_available(Path::new("waiting.zip")));
    assert!(!admission.is_available(Path::new("active.zip")));
}

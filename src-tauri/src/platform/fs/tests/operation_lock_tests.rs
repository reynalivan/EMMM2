use super::*;

// Covers: NC-5.1-04 (Operation Lock Active)
#[tokio::test]
async fn test_lock_acquisition() {
    let lock = OperationLock::new();
    let guard = lock.acquire().await;
    assert!(guard.is_ok(), "First acquisition should succeed");
}

// Covers: NC-5.1-04 (Operation Lock blocks concurrent)
#[tokio::test]
async fn test_lock_contention() {
    let lock = Arc::new(OperationLock::new());
    // Hold the lock
    let _guard = lock.acquire().await.unwrap();

    // Second acquisition should fail (timeout)
    let lock2 = lock.clone();
    let result = lock2.acquire().await;
    assert!(result.is_err(), "Second acquisition should fail");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Operation in progress"),
        "Error should mention operation in progress"
    );
}

// Covers: EC-5.01 (Lock released after drop)
#[tokio::test]
async fn test_lock_release_on_drop() {
    let lock = OperationLock::new();
    {
        let _guard = lock.acquire().await.unwrap();
        // Guard dropped here
    }
    // Should succeed after release
    let result = lock.acquire().await;
    assert!(result.is_ok(), "Should succeed after guard is dropped");
}

#[tokio::test]
async fn cloned_lock_handles_share_contention() {
    let lock = OperationLock::new();
    let shared = lock.clone();
    let _guard = lock.acquire().await.unwrap();

    let result = shared.acquire().await;

    assert!(result.is_err(), "cloned handles must guard the same mutex");
}

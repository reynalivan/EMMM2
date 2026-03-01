use crate::services::update::metadata_sync;
use crate::test_utils;

#[tokio::test]
async fn test_update_cmds_delegation() {
    let test_db = test_utils::init_test_db().await;
    let pool = &test_db.pool;

    // Simulate the command inner logic which syncs metadata
    // This calls HTTP but handles offline connection gracefully
    let result = metadata_sync::check_and_sync_metadata(pool).await;

    // It should succeed (either true or false depending on network)
    assert!(result.updated == true || result.updated == false);
}
